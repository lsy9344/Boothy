//! Story 7.3 Route A: RAW 컨테이너에 내장된 full-size JPEG 추출.
//!
//! **새 의존성 없이 구현한다.** CR2는 TIFF 기반 컨테이너이고, 내장 full-size JPEG의 위치는
//! IFD#0의 `StripOffsets`/`StripByteCounts`가 그대로 가리킨다. 이 저장소는 이미
//! `display::image_probe`에서 같은 종류의 구조 파싱을 crate 없이 하고 있으므로,
//! 여기서도 같은 방식을 쓴다.
//!
//! 그 결과 LGPL-2.1/CDDL license gate와 Story 7.7 offline clean-machine 인벤토리 영향이
//! **둘 다 발생하지 않는다.**
//!
//! # 경계
//!
//! - 이 모듈은 **읽기만 한다.** RAW 파일을 열어 바이트를 훑을 뿐 어떤 경로에도 쓰지 않는다.
//!   추출이 실패해도 이미 저장된 RAW truth는 영향을 받지 않는다.
//! - 구조 검증은 **Story 7.2의 `image_probe`를 그대로 재사용한다.** 규칙을 두 벌 만들면
//!   Story 7.4에서 통과 기준이 갈라진다.
//! - CR3(ISO BMFF 기반)는 이 파서의 대상이 아니다. 승인 하드웨어 EOS 700D는 CR2를 만든다.
//!   CR3가 들어오면 조용히 실패하지 않고 고유 사유로 거부한다.

use crate::contracts::dto::{
    SOURCE_REJECT_ABSENT, SOURCE_REJECT_CORRUPT, SOURCE_REJECT_ORIENTATION_UNSUPPORTED,
    SOURCE_REJECT_UNDECODABLE,
};
use crate::display::image_probe::{probe_jpeg_structure, JpegProbe, ProbeError};

/// TIFF IFD entry 하나의 크기 (tag 2 + type 2 + count 4 + value/offset 4).
const IFD_ENTRY_BYTES: usize = 12;

const TAG_ORIENTATION: u16 = 0x0112;
const TAG_STRIP_OFFSETS: u16 = 0x0111;
const TAG_STRIP_BYTE_COUNTS: u16 = 0x0117;

const TIFF_TYPE_SHORT: u16 = 3;
const TIFF_TYPE_LONG: u16 = 4;

/// 추출된 내장 JPEG의 위치와 검증 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedJpeg {
    /// RAW 파일 안에서 JPEG이 시작하는 바이트 위치.
    pub offset: usize,
    pub byte_size: usize,
    pub probe: JpegProbe,
    /// 최종 orientation. JPEG 자체 EXIF가 있으면 그것을, 없으면 TIFF IFD#0의 값을 쓴다.
    pub orientation: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ByteOrder {
    Little,
    Big,
}

impl ByteOrder {
    fn u16(self, bytes: &[u8], offset: usize) -> Option<u16> {
        let slice = bytes.get(offset..offset + 2)?;
        let array = [slice[0], slice[1]];

        Some(match self {
            ByteOrder::Little => u16::from_le_bytes(array),
            ByteOrder::Big => u16::from_be_bytes(array),
        })
    }

    fn u32(self, bytes: &[u8], offset: usize) -> Option<u32> {
        let slice = bytes.get(offset..offset + 4)?;
        let array = [slice[0], slice[1], slice[2], slice[3]];

        Some(match self {
            ByteOrder::Little => u32::from_le_bytes(array),
            ByteOrder::Big => u32::from_be_bytes(array),
        })
    }
}

/// IFD entry 하나에서 첫 번째 값을 읽는다.
///
/// `SHORT`/`LONG`만 다룬다. CR2의 strip 관련 태그는 전부 이 두 타입이다.
/// 값이 4바이트에 들어가면 inline이고, 그렇지 않으면 offset이 가리키는 곳에 있다.
fn read_first_value(
    bytes: &[u8],
    order: ByteOrder,
    entry_offset: usize,
    value_type: u16,
    count: u32,
) -> Option<u32> {
    if count == 0 {
        return None;
    }

    let value_field = entry_offset + 8;

    match value_type {
        TIFF_TYPE_SHORT => {
            // SHORT 2개까지는 value field 안에 들어간다.
            if count <= 2 {
                order.u16(bytes, value_field).map(u32::from)
            } else {
                let pointer = order.u32(bytes, value_field)? as usize;
                order.u16(bytes, pointer).map(u32::from)
            }
        }
        TIFF_TYPE_LONG => {
            if count == 1 {
                order.u32(bytes, value_field)
            } else {
                let pointer = order.u32(bytes, value_field)? as usize;
                order.u32(bytes, pointer)
            }
        }
        _ => None,
    }
}

/// TIFF 헤더를 읽어 byte order와 IFD#0 위치를 돌려준다.
fn read_tiff_header(bytes: &[u8]) -> Result<(ByteOrder, usize), &'static str> {
    let order = match bytes.get(0..2) {
        Some(b"II") => ByteOrder::Little,
        Some(b"MM") => ByteOrder::Big,
        // CR3는 ISO BMFF다. `ftyp` box로 시작하므로 TIFF가 아니라는 것이 바로 드러난다.
        _ => return Err(SOURCE_REJECT_UNDECODABLE),
    };

    if order.u16(bytes, 2) != Some(0x002A) {
        return Err(SOURCE_REJECT_UNDECODABLE);
    }

    // Route A is deliberately CR2-only. Accepting arbitrary TIFF files would make
    // route evidence claim that a non-RAW container was a camera RAW source.
    if bytes.get(8..12) != Some(b"CR\x02\x00") {
        return Err(SOURCE_REJECT_UNDECODABLE);
    }

    let ifd0_offset = order.u32(bytes, 4).ok_or(SOURCE_REJECT_UNDECODABLE)? as usize;

    if ifd0_offset < 8 || ifd0_offset >= bytes.len() {
        return Err(SOURCE_REJECT_CORRUPT);
    }

    Ok((order, ifd0_offset))
}

/// RAW 컨테이너에서 내장 full-size JPEG을 꺼낸다.
///
/// 실패는 전부 고유한 거부 사유로 돌아온다. **조용한 무시는 없다.**
/// 이 함수는 아무것도 쓰지 않으므로 어떤 실패도 RAW truth를 건드리지 않는다.
pub fn extract_embedded_jpeg(raw_bytes: &[u8]) -> Result<EmbeddedJpeg, &'static str> {
    let (order, ifd0_offset) = read_tiff_header(raw_bytes)?;

    let entry_count = order
        .u16(raw_bytes, ifd0_offset)
        .ok_or(SOURCE_REJECT_CORRUPT)? as usize;

    let mut strip_offset: Option<u32> = None;
    let mut strip_byte_count: Option<u32> = None;
    let mut tiff_orientation: Option<u16> = None;

    for index in 0..entry_count {
        let entry_offset = ifd0_offset + 2 + index * IFD_ENTRY_BYTES;

        // entry가 파일 밖으로 나가면 손상된 것이다.
        if entry_offset + IFD_ENTRY_BYTES > raw_bytes.len() {
            return Err(SOURCE_REJECT_CORRUPT);
        }

        let tag = order
            .u16(raw_bytes, entry_offset)
            .ok_or(SOURCE_REJECT_CORRUPT)?;
        let value_type = order
            .u16(raw_bytes, entry_offset + 2)
            .ok_or(SOURCE_REJECT_CORRUPT)?;
        let count = order
            .u32(raw_bytes, entry_offset + 4)
            .ok_or(SOURCE_REJECT_CORRUPT)?;

        match tag {
            TAG_STRIP_OFFSETS => {
                if strip_offset.is_some()
                    || count != 1
                    || !matches!(value_type, TIFF_TYPE_SHORT | TIFF_TYPE_LONG)
                {
                    return Err(SOURCE_REJECT_CORRUPT);
                }
                strip_offset = Some(
                    read_first_value(raw_bytes, order, entry_offset, value_type, count)
                        .ok_or(SOURCE_REJECT_CORRUPT)?,
                );
            }
            TAG_STRIP_BYTE_COUNTS => {
                if strip_byte_count.is_some()
                    || count != 1
                    || !matches!(value_type, TIFF_TYPE_SHORT | TIFF_TYPE_LONG)
                {
                    return Err(SOURCE_REJECT_CORRUPT);
                }
                strip_byte_count = Some(
                    read_first_value(raw_bytes, order, entry_offset, value_type, count)
                        .ok_or(SOURCE_REJECT_CORRUPT)?,
                );
            }
            TAG_ORIENTATION => {
                if tiff_orientation.is_some() || count != 1 || value_type != TIFF_TYPE_SHORT {
                    return Err(SOURCE_REJECT_CORRUPT);
                }
                let value = read_first_value(raw_bytes, order, entry_offset, value_type, count)
                    .ok_or(SOURCE_REJECT_CORRUPT)?;
                tiff_orientation = Some(u16::try_from(value).map_err(|_| SOURCE_REJECT_CORRUPT)?);
            }
            _ => {}
        }
    }

    // 태그가 없으면 이 컨테이너에는 내장 JPEG이 없다. 손상이 아니라 부재다.
    let (Some(offset), Some(byte_size)) = (strip_offset, strip_byte_count) else {
        return Err(SOURCE_REJECT_ABSENT);
    };

    let offset = offset as usize;
    let byte_size = byte_size as usize;

    if byte_size == 0 {
        return Err(SOURCE_REJECT_CORRUPT);
    }

    let end = offset.checked_add(byte_size).ok_or(SOURCE_REJECT_CORRUPT)?;

    // 파일이 끝나기 전에 strip이 끝나야 한다. 잘린 파일을 통과시키면 7.4가 깨진 자산을 올린다.
    let jpeg_bytes = raw_bytes.get(offset..end).ok_or(SOURCE_REJECT_CORRUPT)?;

    let probe = probe_jpeg_structure(jpeg_bytes).map_err(|error| match error {
        ProbeError::PartialFile => crate::contracts::dto::SOURCE_REJECT_PARTIAL,
        ProbeError::Undecodable => SOURCE_REJECT_UNDECODABLE,
        ProbeError::OrientationUnsupported => SOURCE_REJECT_ORIENTATION_UNSUPPORTED,
    })?;

    // orientation은 JPEG 자체 EXIF가 우선이고, 없으면 TIFF IFD#0의 값을 쓴다.
    // CR2는 orientation을 컨테이너 쪽에 두는 경우가 많아 이 fallback이 없으면
    // 회전된 촬영이 orientation 1로 잘못 통과한다.
    //
    // 회전된 값(2~8)은 거부하지 않고 **그대로 돌려준다.** HV-14 첫 회차에서 부스 rig의
    // EOS 700D가 만드는 모든 촬영이 orientation!=1로 거부되어 Route A 비교가 무산됐다.
    // 회전 지원 여부의 판정은 승격 로직(`source_probe`)이, 표시 정규화는 Story 7.4가 소유한다.
    // EXIF 유효 범위(1~8) 밖의 값만 손상된 메타데이터로 거부한다.
    let orientation = probe.orientation.or(tiff_orientation);

    if let Some(value) = orientation {
        if !(1..=8).contains(&value) {
            return Err(SOURCE_REJECT_CORRUPT);
        }
    }

    Ok(EmbeddedJpeg {
        offset,
        byte_size,
        probe,
        orientation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::dto::SOURCE_REJECT_PARTIAL;

    /// 최소 구조의 유효 JPEG. `image_probe` 테스트와 같은 합성 방식이다.
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

        let mut sof = vec![8u8];
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

    struct Cr2Options {
        include_strip_tags: bool,
        tiff_orientation: Option<u16>,
        malformed_orientation: bool,
        /// strip byte count를 실제보다 크게 적어 잘린 파일을 흉내 낸다.
        overstate_byte_count: bool,
        zero_byte_count: bool,
        duplicate_strip_offset: bool,
        duplicate_strip_byte_count: bool,
        truncate_jpeg_tail: bool,
    }

    impl Default for Cr2Options {
        fn default() -> Self {
            Self {
                include_strip_tags: true,
                tiff_orientation: None,
                malformed_orientation: false,
                overstate_byte_count: false,
                zero_byte_count: false,
                duplicate_strip_offset: false,
                duplicate_strip_byte_count: false,
                truncate_jpeg_tail: false,
            }
        }
    }

    /// EOS 700D가 만드는 CR2의 관련 구조만 합성한다.
    /// little-endian TIFF 헤더 + CR2 매직 + IFD#0(내장 JPEG을 가리키는 strip 태그).
    fn build_cr2(jpeg: &[u8], options: Cr2Options) -> Vec<u8> {
        build_cr2_with_order(jpeg, options, ByteOrder::Little)
    }

    fn push_u16(bytes: &mut Vec<u8>, value: u16, order: ByteOrder) {
        match order {
            ByteOrder::Little => bytes.extend_from_slice(&value.to_le_bytes()),
            ByteOrder::Big => bytes.extend_from_slice(&value.to_be_bytes()),
        }
    }

    fn push_u32(bytes: &mut Vec<u8>, value: u32, order: ByteOrder) {
        match order {
            ByteOrder::Little => bytes.extend_from_slice(&value.to_le_bytes()),
            ByteOrder::Big => bytes.extend_from_slice(&value.to_be_bytes()),
        }
    }

    fn build_cr2_with_order(jpeg: &[u8], options: Cr2Options, order: ByteOrder) -> Vec<u8> {
        let mut entries: Vec<(u16, u16, u32, u32)> = Vec::new();

        // IFD#0은 offset 16에 두고, JPEG은 IFD 뒤에 붙인다.
        let ifd0_offset = 16usize;
        let entry_count = if options.include_strip_tags {
            3 + usize::from(options.duplicate_strip_offset)
                + usize::from(options.duplicate_strip_byte_count)
        } else {
            1
        } + usize::from(options.tiff_orientation.is_some());
        let jpeg_offset = ifd0_offset + 2 + entry_count * IFD_ENTRY_BYTES + 4;

        // Compression = 6 (JPEG). 파서가 무시하지만 실물과 같은 모양을 유지한다.
        entries.push((0x0103, TIFF_TYPE_SHORT, 1, 6));

        if let Some(orientation) = options.tiff_orientation {
            entries.push((
                TAG_ORIENTATION,
                if options.malformed_orientation {
                    TIFF_TYPE_LONG
                } else {
                    TIFF_TYPE_SHORT
                },
                1,
                u32::from(orientation),
            ));
        }

        if options.include_strip_tags {
            let declared = if options.zero_byte_count {
                0
            } else if options.overstate_byte_count {
                jpeg.len() as u32 + 4_096
            } else {
                jpeg.len() as u32
            };

            entries.push((TAG_STRIP_OFFSETS, TIFF_TYPE_LONG, 1, jpeg_offset as u32));
            if options.duplicate_strip_offset {
                entries.push((TAG_STRIP_OFFSETS, TIFF_TYPE_LONG, 1, jpeg_offset as u32));
            }
            entries.push((TAG_STRIP_BYTE_COUNTS, TIFF_TYPE_LONG, 1, declared));
            if options.duplicate_strip_byte_count {
                entries.push((TAG_STRIP_BYTE_COUNTS, TIFF_TYPE_LONG, 1, declared));
            }
        }

        entries.sort_by_key(|entry| entry.0);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(match order {
            ByteOrder::Little => b"II",
            ByteOrder::Big => b"MM",
        });
        push_u16(&mut bytes, 0x002A, order);
        push_u32(&mut bytes, ifd0_offset as u32, order);
        bytes.extend_from_slice(b"CR");
        bytes.extend_from_slice(&[0x02, 0x00]);
        push_u32(&mut bytes, 0, order);

        debug_assert_eq!(bytes.len(), ifd0_offset);

        push_u16(&mut bytes, entries.len() as u16, order);
        for (tag, value_type, count, value) in &entries {
            push_u16(&mut bytes, *tag, order);
            push_u16(&mut bytes, *value_type, order);
            push_u32(&mut bytes, *count, order);

            if *value_type == TIFF_TYPE_SHORT {
                push_u16(&mut bytes, *value as u16, order);
                bytes.extend_from_slice(&[0x00, 0x00]);
            } else {
                push_u32(&mut bytes, *value, order);
            }
        }
        push_u32(&mut bytes, 0, order); // next IFD 없음

        debug_assert_eq!(bytes.len(), jpeg_offset);

        if options.truncate_jpeg_tail {
            bytes.extend_from_slice(&jpeg[..jpeg.len() - 2]);
        } else {
            bytes.extend_from_slice(jpeg);
        }

        bytes
    }

    #[test]
    fn extracts_the_full_size_embedded_jpeg() {
        let jpeg = build_jpeg(5184, 3456, None);
        let cr2 = build_cr2(&jpeg, Cr2Options::default());

        let extracted = extract_embedded_jpeg(&cr2).expect("extraction should succeed");

        assert_eq!(extracted.byte_size, jpeg.len());
        assert_eq!(extracted.probe.width_px, 5184);
        assert_eq!(extracted.probe.height_px, 3456);
        assert_eq!(
            &cr2[extracted.offset..extracted.offset + extracted.byte_size],
            jpeg.as_slice()
        );
    }

    #[test]
    fn accepts_tiff_orientation_one() {
        let jpeg = build_jpeg(5184, 3456, None);
        let cr2 = build_cr2(
            &jpeg,
            Cr2Options {
                tiff_orientation: Some(1),
                ..Default::default()
            },
        );

        let extracted = extract_embedded_jpeg(&cr2).expect("extraction should succeed");

        assert_eq!(extracted.orientation, Some(1));
    }

    /// CR2는 orientation을 컨테이너 쪽에 두는 경우가 많다. 이 fallback이 없으면
    /// 회전된 촬영이 orientation 없음으로 잘못 통과한다. 실장비(HV-14)의 회전 값은
    /// 거부 대상이 아니라 **기록 대상**이다.
    #[test]
    fn reports_rotated_orientation_declared_only_in_the_container() {
        let jpeg = build_jpeg(5184, 3456, None);
        let cr2 = build_cr2(
            &jpeg,
            Cr2Options {
                tiff_orientation: Some(6),
                ..Default::default()
            },
        );

        let extracted = extract_embedded_jpeg(&cr2).expect("extraction should succeed");

        assert_eq!(extracted.orientation, Some(6));
        assert_eq!(extracted.probe.orientation, None);
    }

    #[test]
    fn reports_rotated_orientation_declared_in_the_embedded_jpeg() {
        let jpeg = build_jpeg(5184, 3456, Some(8));
        let cr2 = build_cr2(&jpeg, Cr2Options::default());

        let extracted = extract_embedded_jpeg(&cr2).expect("extraction should succeed");

        assert_eq!(extracted.orientation, Some(8));
    }

    /// EXIF 유효 범위(1~8) 밖의 orientation은 손상된 메타데이터다.
    #[test]
    fn rejects_orientation_outside_exif_range_as_corrupt() {
        let jpeg = build_jpeg(5184, 3456, None);
        let cr2 = build_cr2(
            &jpeg,
            Cr2Options {
                tiff_orientation: Some(9),
                ..Default::default()
            },
        );

        assert_eq!(extract_embedded_jpeg(&cr2), Err(SOURCE_REJECT_CORRUPT));
    }

    #[test]
    fn reports_absent_when_the_container_has_no_embedded_jpeg() {
        let jpeg = build_jpeg(5184, 3456, None);
        let cr2 = build_cr2(
            &jpeg,
            Cr2Options {
                include_strip_tags: false,
                ..Default::default()
            },
        );

        assert_eq!(extract_embedded_jpeg(&cr2), Err(SOURCE_REJECT_ABSENT));
    }

    /// strip이 파일 끝을 넘어가면 손상이다. 조용히 잘라 쓰지 않는다.
    #[test]
    fn rejects_a_strip_that_runs_past_the_end_of_the_file() {
        let jpeg = build_jpeg(5184, 3456, None);
        let cr2 = build_cr2(
            &jpeg,
            Cr2Options {
                overstate_byte_count: true,
                ..Default::default()
            },
        );

        assert_eq!(extract_embedded_jpeg(&cr2), Err(SOURCE_REJECT_CORRUPT));
    }

    #[test]
    fn rejects_duplicate_strip_metadata() {
        let jpeg = build_jpeg(5184, 3456, None);
        let duplicate_offset = build_cr2(
            &jpeg,
            Cr2Options {
                duplicate_strip_offset: true,
                ..Default::default()
            },
        );
        let duplicate_byte_count = build_cr2(
            &jpeg,
            Cr2Options {
                duplicate_strip_byte_count: true,
                ..Default::default()
            },
        );

        assert_eq!(
            extract_embedded_jpeg(&duplicate_offset),
            Err(SOURCE_REJECT_CORRUPT)
        );
        assert_eq!(
            extract_embedded_jpeg(&duplicate_byte_count),
            Err(SOURCE_REJECT_CORRUPT)
        );
    }

    #[test]
    fn rejects_malformed_orientation_metadata() {
        let jpeg = build_jpeg(5184, 3456, None);
        let cr2 = build_cr2(
            &jpeg,
            Cr2Options {
                tiff_orientation: Some(1),
                malformed_orientation: true,
                ..Default::default()
            },
        );

        assert_eq!(extract_embedded_jpeg(&cr2), Err(SOURCE_REJECT_CORRUPT));
    }

    #[test]
    fn rejects_zero_length_strip_metadata_as_corrupt() {
        let jpeg = build_jpeg(5184, 3456, None);
        let cr2 = build_cr2(
            &jpeg,
            Cr2Options {
                zero_byte_count: true,
                ..Default::default()
            },
        );

        assert_eq!(extract_embedded_jpeg(&cr2), Err(SOURCE_REJECT_CORRUPT));
    }

    /// 선언된 길이는 맞지만 JPEG 자체에 EOI가 없는 경우 — Story 7.2의 partial 판정이 잡는다.
    #[test]
    fn rejects_an_embedded_jpeg_without_an_eoi_trailer() {
        let jpeg = build_jpeg(5184, 3456, None);
        let mut cr2 = build_cr2(
            &jpeg,
            Cr2Options {
                truncate_jpeg_tail: true,
                ..Default::default()
            },
        );
        // 잘린 만큼 채워 strip 범위 자체는 파일 안에 있게 만든다.
        cr2.extend_from_slice(&[0x00, 0x00]);

        assert_eq!(extract_embedded_jpeg(&cr2), Err(SOURCE_REJECT_PARTIAL));
    }

    /// CR3는 ISO BMFF다. 이 파서의 대상이 아니며 조용히 실패하지 않는다.
    #[test]
    fn rejects_a_cr3_style_container_with_a_distinct_reason() {
        let mut cr3 = vec![0x00, 0x00, 0x00, 0x18];
        cr3.extend_from_slice(b"ftypcrx ");
        cr3.extend_from_slice(&[0x00; 32]);

        assert_eq!(extract_embedded_jpeg(&cr3), Err(SOURCE_REJECT_UNDECODABLE));
    }

    #[test]
    fn rejects_non_tiff_input() {
        assert_eq!(
            extract_embedded_jpeg(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]),
            Err(SOURCE_REJECT_UNDECODABLE)
        );
        assert_eq!(extract_embedded_jpeg(&[]), Err(SOURCE_REJECT_UNDECODABLE));
    }

    #[test]
    fn rejects_a_header_pointing_outside_the_file() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"II");
        bytes.extend_from_slice(&0x002Au16.to_le_bytes());
        bytes.extend_from_slice(&99_999u32.to_le_bytes());
        bytes.extend_from_slice(b"CR\x02\x00");
        bytes.extend_from_slice(&[0x00; 12]);

        assert_eq!(extract_embedded_jpeg(&bytes), Err(SOURCE_REJECT_CORRUPT));
    }

    /// big-endian TIFF도 읽는다. Canon은 II를 쓰지만 파서가 byte order를 가정하지 않는다.
    #[test]
    fn reads_big_endian_containers_too() {
        let jpeg = build_jpeg(1920, 1280, None);
        let little = build_cr2(&jpeg, Cr2Options::default());
        let big = build_cr2_with_order(&jpeg, Cr2Options::default(), ByteOrder::Big);

        assert!(extract_embedded_jpeg(&little).is_ok());
        let extracted = extract_embedded_jpeg(&big).expect("big-endian CR2 should decode");
        assert_eq!(extracted.probe.width_px, 1920);
        assert_eq!(extracted.probe.height_px, 1280);
    }

    #[test]
    fn rejects_generic_tiff_without_the_cr2_signature() {
        let jpeg = build_jpeg(1920, 1280, None);
        let mut tiff = build_cr2(&jpeg, Cr2Options::default());
        tiff[8..12].copy_from_slice(&[0, 0, 0, 0]);

        assert_eq!(extract_embedded_jpeg(&tiff), Err(SOURCE_REJECT_UNDECODABLE));
    }

    #[test]
    fn rejects_multi_strip_metadata_instead_of_using_only_the_first_value() {
        let jpeg = build_jpeg(1920, 1280, None);
        let mut cr2 = build_cr2(&jpeg, Cr2Options::default());
        let ifd0 = 16usize;
        let entries = u16::from_le_bytes([cr2[ifd0], cr2[ifd0 + 1]]) as usize;

        for index in 0..entries {
            let entry = ifd0 + 2 + index * IFD_ENTRY_BYTES;
            let tag = u16::from_le_bytes([cr2[entry], cr2[entry + 1]]);
            if tag == TAG_STRIP_OFFSETS {
                cr2[entry + 4..entry + 8].copy_from_slice(&2u32.to_le_bytes());
            }
        }

        assert_eq!(extract_embedded_jpeg(&cr2), Err(SOURCE_REJECT_CORRUPT));
    }

    /// 추출은 읽기 전용이다. 입력 버퍼를 바꾸지 않는다.
    #[test]
    fn extraction_does_not_mutate_the_source_bytes() {
        let jpeg = build_jpeg(5184, 3456, None);
        let cr2 = build_cr2(&jpeg, Cr2Options::default());
        let before = cr2.clone();

        let _ = extract_embedded_jpeg(&cr2);

        assert_eq!(cr2, before);
    }
}
