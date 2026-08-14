//! Story 7.2: 의존성 없는 JPEG 구조 probe.
//!
//! 왜 필요한가: 기존 `render::has_jpeg_signature`는 앞 3바이트만 본다. 절반만 쓰인 파일도
//! 통과하므로 partial file이 활성 display truth가 될 수 있다.
//!
//! 이 probe는 새 crate 없이 다음을 확인한다.
//! - SOI (`FFD8`) 시작
//! - SOF 마커에서 실제 width/height 파싱 (thumbnail 오인 방지)
//! - **EOI (`FFD9`) trailer 존재** — partial file 방어의 실질적 근거
//! - EXIF orientation이 있으면 `1`만 허용 (geometry 흔들림 차단)
//!
//! 픽셀 단위 완전 decode는 viewer의 `img.decode()`가 담당한다. 두 단계 모두 필수다.

/// EOI를 찾을 때 파일 끝에서 되짚어볼 최대 바이트 수.
/// 일부 인코더가 뒤에 패딩을 붙이므로 정확히 마지막 2바이트만 보지는 않는다.
const EOI_SEARCH_TAIL_BYTES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JpegProbe {
    pub width_px: u32,
    pub height_px: u32,
    pub orientation: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeError {
    /// EOI trailer가 없다. 아직 다 쓰이지 않았거나 잘린 파일이다.
    PartialFile,
    /// SOI/SOF 구조를 해석할 수 없다.
    Undecodable,
    /// EXIF orientation이 1이 아니다.
    OrientationUnsupported,
}

fn has_eoi_trailer(bytes: &[u8]) -> bool {
    let tail_start = bytes.len().saturating_sub(EOI_SEARCH_TAIL_BYTES);
    let tail = &bytes[tail_start..];

    tail.windows(2)
        .any(|window| window[0] == 0xFF && window[1] == 0xD9)
}

fn read_u16_be(bytes: &[u8], offset: usize) -> Option<u16> {
    let slice = bytes.get(offset..offset + 2)?;

    Some(u16::from_be_bytes([slice[0], slice[1]]))
}

/// APP1 payload에서 EXIF orientation(tag `0x0112`)을 읽는다.
/// 구조가 예상과 다르면 `None`을 돌려주고 상위에서 "orientation 없음"으로 취급한다.
fn parse_exif_orientation(payload: &[u8]) -> Option<u16> {
    if !payload.starts_with(b"Exif\0\0") {
        return None;
    }

    let tiff = payload.get(6..)?;
    let is_little_endian = match tiff.get(0..2)? {
        b"II" => true,
        b"MM" => false,
        _ => return None,
    };

    let read_u16 = |offset: usize| -> Option<u16> {
        let slice = tiff.get(offset..offset + 2)?;
        Some(if is_little_endian {
            u16::from_le_bytes([slice[0], slice[1]])
        } else {
            u16::from_be_bytes([slice[0], slice[1]])
        })
    };
    let read_u32 = |offset: usize| -> Option<u32> {
        let slice = tiff.get(offset..offset + 4)?;
        Some(if is_little_endian {
            u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]])
        } else {
            u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]])
        })
    };

    if read_u16(2)? != 0x002A {
        return None;
    }

    let ifd0_offset = read_u32(4)? as usize;
    let entry_count = read_u16(ifd0_offset)? as usize;

    for index in 0..entry_count {
        let entry_offset = ifd0_offset + 2 + index * 12;

        if read_u16(entry_offset)? == 0x0112 {
            return read_u16(entry_offset + 8);
        }
    }

    None
}

/// 마커를 순회하며 SOF 크기와 EXIF orientation을 읽는 **구조 판정**이다.
///
/// orientation 값 자체는 그대로 돌려준다. "orientation 1만 허용"은 display 승인 정책이지
/// 구조 결함이 아니므로, 그 정책은 `probe_jpeg`가 얹는다. Story 7.3의 source 비교 lane은
/// 실측 orientation을 기록해야 하므로 이 구조 판정을 공유한다 — 규칙은 여전히 한 벌이다.
pub fn probe_jpeg_structure(bytes: &[u8]) -> Result<JpegProbe, ProbeError> {
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return Err(ProbeError::Undecodable);
    }

    if !has_eoi_trailer(bytes) {
        return Err(ProbeError::PartialFile);
    }

    let mut cursor = 2usize;
    let mut dimensions: Option<(u32, u32)> = None;
    let mut orientation: Option<u16> = None;

    while cursor + 1 < bytes.len() {
        if bytes[cursor] != 0xFF {
            cursor += 1;
            continue;
        }

        let marker = bytes[cursor + 1];

        // fill byte
        if marker == 0xFF {
            cursor += 1;
            continue;
        }

        // payload 없는 마커
        if marker == 0xD8 || marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            cursor += 2;
            continue;
        }

        // EOI 또는 SOS(엔트로피 데이터 시작) 이후로는 마커를 신뢰할 수 없다.
        if marker == 0xD9 || marker == 0xDA {
            break;
        }

        let segment_length =
            read_u16_be(bytes, cursor + 2).ok_or(ProbeError::Undecodable)? as usize;

        if segment_length < 2 || cursor + 2 + segment_length > bytes.len() {
            return Err(ProbeError::Undecodable);
        }

        let payload = &bytes[cursor + 4..cursor + 2 + segment_length];

        match marker {
            // SOF0/1/2/3, 5/6/7, 9/10/11, 13/14/15 — DHT(C4)/JPG(C8)/DAC(CC)는 제외한다.
            0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF => {
                if payload.len() < 6 {
                    return Err(ProbeError::Undecodable);
                }

                let height = u16::from_be_bytes([payload[1], payload[2]]) as u32;
                let width = u16::from_be_bytes([payload[3], payload[4]]) as u32;

                if width == 0 || height == 0 {
                    return Err(ProbeError::Undecodable);
                }

                if dimensions.is_none() {
                    dimensions = Some((width, height));
                }
            }
            0xE1 => {
                if orientation.is_none() {
                    orientation = parse_exif_orientation(payload);
                }
            }
            _ => {}
        }

        cursor += 2 + segment_length;
    }

    let (width_px, height_px) = dimensions.ok_or(ProbeError::Undecodable)?;

    Ok(JpegProbe {
        width_px,
        height_px,
        orientation,
    })
}

/// Story 7.2 display 승인 판정: 구조 판정 + orientation 1만 허용.
pub fn probe_jpeg(bytes: &[u8]) -> Result<JpegProbe, ProbeError> {
    let probe = probe_jpeg_structure(bytes)?;

    if let Some(value) = probe.orientation {
        if value != 1 {
            return Err(ProbeError::OrientationUnsupported);
        }
    }

    Ok(probe)
}

/// provenance/상관관계 확인용 해시. 보안 해시가 아니므로 알고리즘을 접두사로 드러낸다.
pub fn content_hash(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;

    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }

    format!("fnv1a64:{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 최소 구조의 baseline JPEG을 합성한다. SOI + (선택) APP1 + SOF0 + SOS + EOI.
    fn build_jpeg(width: u16, height: u16, orientation: Option<u16>) -> Vec<u8> {
        let mut bytes = vec![0xFF, 0xD8];

        if let Some(orientation) = orientation {
            let mut app1 = Vec::new();
            app1.extend_from_slice(b"Exif\0\0");
            app1.extend_from_slice(b"MM");
            app1.extend_from_slice(&0x002Au16.to_be_bytes());
            app1.extend_from_slice(&8u32.to_be_bytes());
            app1.extend_from_slice(&1u16.to_be_bytes());
            app1.extend_from_slice(&0x0112u16.to_be_bytes());
            app1.extend_from_slice(&3u16.to_be_bytes());
            app1.extend_from_slice(&1u32.to_be_bytes());
            app1.extend_from_slice(&orientation.to_be_bytes());
            app1.extend_from_slice(&[0x00, 0x00]);

            bytes.extend_from_slice(&[0xFF, 0xE1]);
            bytes.extend_from_slice(&((app1.len() + 2) as u16).to_be_bytes());
            bytes.extend_from_slice(&app1);
        }

        let mut sof = Vec::new();
        sof.push(8u8);
        sof.extend_from_slice(&height.to_be_bytes());
        sof.extend_from_slice(&width.to_be_bytes());
        sof.push(3u8);
        sof.extend_from_slice(&[0x01, 0x11, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01]);

        bytes.extend_from_slice(&[0xFF, 0xC0]);
        bytes.extend_from_slice(&((sof.len() + 2) as u16).to_be_bytes());
        bytes.extend_from_slice(&sof);

        bytes.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00]);
        bytes.extend_from_slice(&[0x12, 0x34, 0x56, 0x78]);
        bytes.extend_from_slice(&[0xFF, 0xD9]);

        bytes
    }

    #[test]
    fn reads_sof_dimensions() {
        let probe = probe_jpeg(&build_jpeg(3840, 2560, None)).expect("probe should succeed");

        assert_eq!(probe.width_px, 3840);
        assert_eq!(probe.height_px, 2560);
        assert_eq!(probe.orientation, None);
    }

    #[test]
    fn rejects_file_without_eoi_trailer() {
        let full = build_jpeg(3840, 2560, None);
        let truncated = &full[..full.len() - 2];

        assert_eq!(probe_jpeg(truncated), Err(ProbeError::PartialFile));
    }

    #[test]
    fn rejects_half_written_file() {
        let full = build_jpeg(3840, 2560, None);
        let half = &full[..full.len() / 2];

        assert_eq!(probe_jpeg(half), Err(ProbeError::PartialFile));
    }

    #[test]
    fn rejects_non_jpeg_bytes() {
        assert_eq!(
            probe_jpeg(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]),
            Err(ProbeError::Undecodable)
        );
    }

    #[test]
    fn accepts_orientation_one() {
        let probe = probe_jpeg(&build_jpeg(3840, 2560, Some(1))).expect("probe should succeed");

        assert_eq!(probe.orientation, Some(1));
    }

    #[test]
    fn rejects_rotated_orientation() {
        assert_eq!(
            probe_jpeg(&build_jpeg(3840, 2560, Some(6))),
            Err(ProbeError::OrientationUnsupported)
        );
    }

    /// 구조 판정은 회전된 orientation을 값 그대로 돌려준다. display 정책(`probe_jpeg`)이
    /// 그 값을 거부하는 것과 별개다 — Story 7.3 source lane이 실측값을 기록할 때 쓴다.
    #[test]
    fn structure_probe_reports_rotated_orientation_without_display_policy() {
        let probe = probe_jpeg_structure(&build_jpeg(3840, 2560, Some(6)))
            .expect("structure probe should succeed");

        assert_eq!(probe.orientation, Some(6));
        assert_eq!((probe.width_px, probe.height_px), (3840, 2560));
    }

    #[test]
    fn content_hash_is_stable_and_distinct() {
        let first = build_jpeg(3840, 2560, None);
        let second = build_jpeg(1920, 1280, None);

        assert_eq!(content_hash(&first), content_hash(&first));
        assert_ne!(content_hash(&first), content_hash(&second));
        assert!(content_hash(&first).starts_with("fnv1a64:"));
    }
}
