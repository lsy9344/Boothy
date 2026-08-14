//! Story 7.3: source 후보의 승격 판정과 거부 사유.
//!
//! `display::display_artifact::evaluate_admission`과 같은 형태의 **순수 판정 로직**이다.
//! I/O는 호출자가 하고, 여기서는 결정만 내린다.
//!
//! 두 가지가 이 모듈의 존재 이유다.
//!
//! 1. **모든 거부가 고유 사유를 가진다.** 조용한 무시를 허용하면 성공률 분모가 소리 없이
//!    줄어 비교 결과가 실제보다 좋아 보인다. Story 7.2가 이 결함으로 한 회차를 잃었다.
//! 2. **RAW truth는 어떤 실험에도 종속되지 않는다.** 여기서 무엇을 거부하든 이미 저장된
//!    RAW의 성공 판정은 바뀌지 않는다. 이 함수는 `Err`를 돌려줄 뿐 아무것도 지우지 않는다.
//!
//! JPEG 구조 판정은 **Story 7.2의 `display::image_probe`를 그대로 재사용한다.** 규칙을
//! 두 벌 만들면 Story 7.4에서 통과 기준이 갈라진다.

use crate::contracts::dto::{
    SourceCandidateDto, SOURCE_REJECT_ABSENT, SOURCE_REJECT_CANCELLED, SOURCE_REJECT_CORRUPT,
    SOURCE_REJECT_EXTRACTION_FAILED, SOURCE_REJECT_ORIENTATION_UNSUPPORTED, SOURCE_REJECT_PARTIAL,
    SOURCE_REJECT_STALE, SOURCE_REJECT_UNDECODABLE, SOURCE_REJECT_UNSUPPORTED_COMBINATION,
    SOURCE_REJECT_WRONG_CAPTURE, SOURCE_REJECT_WRONG_REQUEST, SOURCE_REJECT_WRONG_SESSION,
};
use crate::display::image_probe::{content_hash, probe_jpeg_structure, JpegProbe, ProbeError};

/// 후보를 만들어 낸 쪽이 보고한 결과. 파일이 생기지 않은 이유를 host가 추측하지 않는다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceProducerOutcome {
    /// 후보 파일이 만들어졌다. 구조 검증 대상이다.
    Produced,
    /// 후보 자체가 존재하지 않는다 (embedded JPEG 없음, JPEG object 미도착 등).
    Absent,
    /// 추출을 시도했으나 실패했다.
    ExtractionFailed,
    /// 진행 중이던 전송이 취소됐다.
    Cancelled,
    /// 카메라 capability descriptor에 없는 조합이라 **시도하지 않았다.**
    ///
    /// 시도해서 실패한 것(`ExtractionFailed`)과는 다른 결과다. 섞으면 "카메라가 지원하지
    /// 않는다"와 "우리 구현이 실패했다"를 evidence에서 구분할 수 없다.
    UnsupportedCombination,
}

/// 승격 판정에 필요한 host 측 문맥. 전부 host가 소유한 truth다.
#[derive(Debug, Clone, Copy)]
pub struct SourceAdmissionContext<'a> {
    pub bound_session_id: Option<&'a str>,
    pub active_request_id: Option<&'a str>,
    /// RAW handoff로 확정된 capture id. 아직 없으면 `None`이고 capture 검사를 건너뛴다.
    pub expected_capture_id: Option<&'a str>,
    /// 이 request가 시작된 host monotonic 시각. 이보다 이른 산출물은 이전 촬영의 잔재다.
    pub request_started_at_host_micros: u64,
    /// 현재 lane 설정에서 측정하기로 한 route 목록.
    pub enabled_routes: &'a [&'a str],
    /// groupID가 없을 때 helper가 파일명 stem + 도착 시각 창으로 correlation을 완료했는가.
    pub used_fallback_correlation: bool,
    /// CR2 컨테이너(TIFF IFD#0)가 선언한 orientation. JPEG 자체 EXIF가 없을 때만 쓰는
    /// fallback truth이며, host가 RAW를 직접 읽어 얻은 값이다 (embedded route 한정).
    pub container_orientation: Option<u16>,
}

/// 후보를 승격할 수 있는지 판정한다.
///
/// `bytes`는 호출자가 읽어 온 후보 파일의 내용이다. `producer`가 `Produced`가 아니면
/// 구조 검증까지 가지 않으므로 `None`이어도 된다.
///
/// 반환값이 `Err`여도 **이미 저장된 RAW truth는 영향을 받지 않는다.**
pub fn evaluate_source_candidate(
    candidate: &SourceCandidateDto,
    producer: SourceProducerOutcome,
    bytes: Option<&[u8]>,
    context: &SourceAdmissionContext<'_>,
) -> Result<(), &'static str> {
    // 1. correlation 먼저. 다른 세션의 산출물은 어떤 이유보다 먼저 잘라낸다 (NFR-004).
    if context.bound_session_id != Some(candidate.session_id.as_str()) {
        return Err(SOURCE_REJECT_WRONG_SESSION);
    }

    if context.active_request_id != Some(candidate.request_id.as_str()) {
        return Err(SOURCE_REJECT_WRONG_REQUEST);
    }

    if let Some(expected) = context.expected_capture_id {
        if candidate.capture_id.as_deref() != Some(expected) {
            return Err(SOURCE_REJECT_WRONG_CAPTURE);
        }
    }

    // 2. 이 lane에서 재기로 한 route인가.
    if !context.enabled_routes.contains(&candidate.route.as_str()) {
        return Err(SOURCE_REJECT_UNSUPPORTED_COMBINATION);
    }

    if candidate.route == crate::contracts::dto::SOURCE_ROUTE_CAMERA_PAIRED_JPEG
        && (candidate.object_index.is_none()
            || (candidate.group_id.is_none() && !context.used_fallback_correlation))
    {
        return Err(SOURCE_REJECT_WRONG_CAPTURE);
    }

    // 3. 생산자가 보고한 결과. host가 이유를 추측하지 않는다.
    match producer {
        SourceProducerOutcome::Produced => {}
        SourceProducerOutcome::Absent => return Err(SOURCE_REJECT_ABSENT),
        SourceProducerOutcome::ExtractionFailed => return Err(SOURCE_REJECT_EXTRACTION_FAILED),
        SourceProducerOutcome::Cancelled => return Err(SOURCE_REJECT_CANCELLED),
        SourceProducerOutcome::UnsupportedCombination => {
            return Err(SOURCE_REJECT_UNSUPPORTED_COMBINATION)
        }
    }

    // 4. 산출물이 있다고 보고했으면 실제로 있어야 한다.
    if candidate.asset_path.is_none() || candidate.byte_size.unwrap_or(0) == 0 {
        return Err(SOURCE_REJECT_ABSENT);
    }

    // 5. 이전 촬영의 잔재가 이번 request의 후보로 올라오지 못하게 한다.
    let Some(ready_at) = candidate.ready_at_host_micros else {
        return Err(SOURCE_REJECT_STALE);
    };
    if ready_at < context.request_started_at_host_micros {
        return Err(SOURCE_REJECT_STALE);
    }

    // 6. 구조 검증 — Story 7.2와 **같은 구조 규칙**을 쓴다.
    //
    // orientation 정책은 lane마다 다르다: display 승인(7.2/7.4)은 여전히 1만 통과시키지만,
    // 이 비교 lane은 실장비가 실제로 만드는 값(1~8)을 **기록**해야 한다. HV-14 첫 회차에서
    // EOS 700D의 Route A/B 표본 34건 전부가 orientation!=1로 거부되어 비교 자체가 무산됐다.
    // 유효 범위(1~8) 밖의 값만 orientation-unsupported로 거부한다.
    let Some(bytes) = bytes else {
        return Err(SOURCE_REJECT_EXTRACTION_FAILED);
    };

    let probe = probe_source_bytes(bytes)?;

    // 7. 기록된 메타데이터가 파일과 어긋나면 그 표본은 신뢰할 수 없다.
    if !candidate.decode_valid
        || candidate.width_px != Some(probe.width_px)
        || candidate.height_px != Some(probe.height_px)
        || candidate.byte_size != Some(bytes.len() as u64)
    {
        return Err(SOURCE_REJECT_CORRUPT);
    }

    // orientation truth는 JPEG 자체 EXIF가 우선이고, 없으면 CR2 컨테이너 값이다 (T3 규칙).
    let effective_orientation = probe.orientation.or(context.container_orientation);
    if let Some(value) = effective_orientation {
        if !(1..=8).contains(&value) {
            return Err(SOURCE_REJECT_ORIENTATION_UNSUPPORTED);
        }
    }

    if candidate.exif_orientation != effective_orientation {
        return Err(SOURCE_REJECT_CORRUPT);
    }

    let Some(recorded_hash) = candidate.source_hash.as_deref() else {
        return Err(SOURCE_REJECT_CORRUPT);
    };
    if recorded_hash != content_hash(bytes) {
        return Err(SOURCE_REJECT_CORRUPT);
    }

    Ok(())
}

/// Story 7.2의 probe 실패를 Story 7.3의 거부 사유로 옮긴다.
fn map_probe_error(error: ProbeError) -> &'static str {
    match error {
        ProbeError::PartialFile => SOURCE_REJECT_PARTIAL,
        ProbeError::Undecodable => SOURCE_REJECT_UNDECODABLE,
        ProbeError::OrientationUnsupported => SOURCE_REJECT_ORIENTATION_UNSUPPORTED,
    }
}

/// 후보 파일을 구조 검증하고 계측에 필요한 값을 뽑는다.
///
/// 호출자가 후보를 기록하기 전에 쓰는 헬퍼다. **구조 규칙은 하나뿐**이라는 것을 코드 구조로
/// 보장하기 위해 `image_probe`의 구조 판정을 직접 감싼다. display의 "orientation 1만 허용"
/// 정책은 여기 얹지 않는다 — 이 lane은 실장비의 실제 orientation(1~8)을 기록하는 것이
/// 목적이고, EXIF 유효 범위 밖의 값만 거부한다.
pub fn probe_source_bytes(bytes: &[u8]) -> Result<JpegProbe, &'static str> {
    let probe = probe_jpeg_structure(bytes).map_err(map_probe_error)?;

    if let Some(value) = probe.orientation {
        if !(1..=8).contains(&value) {
            return Err(SOURCE_REJECT_ORIENTATION_UNSUPPORTED);
        }
    }

    Ok(probe)
}

/// AB/BA block 순서를 host가 결정한다.
///
/// 사람이 순서를 고르면 편향이 들어간다. seed에서 유도하고 표본에 seed를 남겨 재현 가능하게 한다.
/// 홀짝을 그대로 쓰지 않고 섞는 이유는 연속된 block index가 같은 순서로 몰리지 않게 하기 위해서다.
pub fn block_order_for(seed: u64, block_index: u32) -> &'static str {
    let mut mixed = seed ^ ((block_index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    mixed ^= mixed >> 33;
    mixed = mixed.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    mixed ^= mixed >> 33;

    if mixed & 1 == 0 {
        crate::contracts::dto::SOURCE_BLOCK_ORDER_AB
    } else {
        crate::contracts::dto::SOURCE_BLOCK_ORDER_BA
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::dto::{
        SOURCE_ROUTE_CAMERA_PAIRED_JPEG, SOURCE_ROUTE_EMBEDDED_JPEG,
        SOURCE_ROUTE_WINDOWS_SHELL_THUMBNAIL,
    };

    const SESSION: &str = "session_01hzzzzzzzzzzzzzzzzzzzzzzz";
    const OTHER_SESSION: &str = "session_01haaaaaaaaaaaaaaaaaaaaaaa";
    const REQUEST: &str = "capture_req_0001";
    const CAPTURE: &str = "capture_0001";
    const REQUEST_STARTED: u64 = 10_000;

    const ALL_ROUTES: [&str; 3] = [
        SOURCE_ROUTE_EMBEDDED_JPEG,
        SOURCE_ROUTE_CAMERA_PAIRED_JPEG,
        SOURCE_ROUTE_WINDOWS_SHELL_THUMBNAIL,
    ];

    /// 최소 구조의 유효 JPEG. `image_probe` 테스트의 합성 방식을 그대로 쓴다.
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

    fn candidate_for(bytes: &[u8], width: u32, height: u32) -> SourceCandidateDto {
        SourceCandidateDto {
            capture_id: Some(CAPTURE.into()),
            request_id: REQUEST.into(),
            session_id: SESSION.into(),
            route: SOURCE_ROUTE_EMBEDDED_JPEG.into(),
            asset_path: Some("C:/tmp/sources/capture_0001-embedded.jpg".into()),
            width_px: Some(width),
            height_px: Some(height),
            byte_size: Some(bytes.len() as u64),
            exif_orientation: None,
            decode_valid: true,
            source_hash: Some(content_hash(bytes)),
            object_index: Some(0),
            group_id: Some(7),
            ready_at_host_micros: Some(REQUEST_STARTED + 5_000),
            extraction_cost_micros: Some(12_000),
        }
    }

    fn context() -> SourceAdmissionContext<'static> {
        SourceAdmissionContext {
            bound_session_id: Some(SESSION),
            active_request_id: Some(REQUEST),
            expected_capture_id: Some(CAPTURE),
            request_started_at_host_micros: REQUEST_STARTED,
            enabled_routes: &ALL_ROUTES,
            used_fallback_correlation: false,
            container_orientation: None,
        }
    }

    fn evaluate(
        candidate: &SourceCandidateDto,
        producer: SourceProducerOutcome,
        bytes: Option<&[u8]>,
    ) -> Result<(), &'static str> {
        evaluate_source_candidate(candidate, producer, bytes, &context())
    }

    #[test]
    fn admits_a_well_formed_candidate() {
        let bytes = build_jpeg(5184, 3456, None);
        let candidate = candidate_for(&bytes, 5184, 3456);

        assert_eq!(
            evaluate(&candidate, SourceProducerOutcome::Produced, Some(&bytes)),
            Ok(())
        );
    }

    #[test]
    fn rejects_wrong_session() {
        let bytes = build_jpeg(5184, 3456, None);
        let mut candidate = candidate_for(&bytes, 5184, 3456);
        candidate.session_id = OTHER_SESSION.into();

        assert_eq!(
            evaluate(&candidate, SourceProducerOutcome::Produced, Some(&bytes)),
            Err(SOURCE_REJECT_WRONG_SESSION)
        );
    }

    #[test]
    fn rejects_wrong_request() {
        let bytes = build_jpeg(5184, 3456, None);
        let mut candidate = candidate_for(&bytes, 5184, 3456);
        candidate.request_id = "capture_req_0002".into();

        assert_eq!(
            evaluate(&candidate, SourceProducerOutcome::Produced, Some(&bytes)),
            Err(SOURCE_REJECT_WRONG_REQUEST)
        );
    }

    #[test]
    fn rejects_wrong_capture() {
        let bytes = build_jpeg(5184, 3456, None);
        let mut candidate = candidate_for(&bytes, 5184, 3456);
        candidate.capture_id = Some("capture_0002".into());

        assert_eq!(
            evaluate(&candidate, SourceProducerOutcome::Produced, Some(&bytes)),
            Err(SOURCE_REJECT_WRONG_CAPTURE)
        );
    }

    #[test]
    fn rejects_missing_capture_when_host_has_already_bound_one() {
        let bytes = build_jpeg(5184, 3456, None);
        let mut candidate = candidate_for(&bytes, 5184, 3456);
        candidate.capture_id = None;

        assert_eq!(
            evaluate(&candidate, SourceProducerOutcome::Produced, Some(&bytes)),
            Err(SOURCE_REJECT_WRONG_CAPTURE)
        );
    }

    #[test]
    fn rejects_candidate_without_a_freshness_timestamp() {
        let bytes = build_jpeg(5184, 3456, None);
        let mut candidate = candidate_for(&bytes, 5184, 3456);
        candidate.ready_at_host_micros = None;

        assert_eq!(
            evaluate(&candidate, SourceProducerOutcome::Produced, Some(&bytes)),
            Err(SOURCE_REJECT_STALE)
        );
    }

    #[test]
    fn paired_candidate_requires_group_or_explicit_fallback_correlation() {
        let bytes = build_jpeg(5184, 3456, None);
        let mut candidate = candidate_for(&bytes, 5184, 3456);
        candidate.route = SOURCE_ROUTE_CAMERA_PAIRED_JPEG.into();
        candidate.group_id = None;

        assert_eq!(
            evaluate(&candidate, SourceProducerOutcome::Produced, Some(&bytes)),
            Err(SOURCE_REJECT_WRONG_CAPTURE)
        );

        let mut fallback = context();
        fallback.used_fallback_correlation = true;
        assert_eq!(
            evaluate_source_candidate(
                &candidate,
                SourceProducerOutcome::Produced,
                Some(&bytes),
                &fallback,
            ),
            Ok(())
        );
    }

    #[test]
    fn rejects_absent_candidate() {
        let bytes = build_jpeg(5184, 3456, None);
        let candidate = candidate_for(&bytes, 5184, 3456);

        assert_eq!(
            evaluate(&candidate, SourceProducerOutcome::Absent, None),
            Err(SOURCE_REJECT_ABSENT)
        );
    }

    #[test]
    fn rejects_candidate_reported_present_but_missing_asset_path() {
        let bytes = build_jpeg(5184, 3456, None);
        let mut candidate = candidate_for(&bytes, 5184, 3456);
        candidate.asset_path = None;

        assert_eq!(
            evaluate(&candidate, SourceProducerOutcome::Produced, Some(&bytes)),
            Err(SOURCE_REJECT_ABSENT)
        );
    }

    #[test]
    fn rejects_partial_file() {
        let bytes = build_jpeg(5184, 3456, None);
        let truncated = &bytes[..bytes.len() - 2];
        let mut candidate = candidate_for(truncated, 5184, 3456);
        candidate.source_hash = Some(content_hash(truncated));

        assert_eq!(
            evaluate(&candidate, SourceProducerOutcome::Produced, Some(truncated)),
            Err(SOURCE_REJECT_PARTIAL)
        );
    }

    #[test]
    fn rejects_undecodable_bytes() {
        let png = [0x89u8, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0xFF, 0xD9];
        let candidate = candidate_for(&png, 5184, 3456);

        assert_eq!(
            evaluate(&candidate, SourceProducerOutcome::Produced, Some(&png)),
            Err(SOURCE_REJECT_UNDECODABLE)
        );
    }

    /// HV-14 첫 회차의 교훈: 부스 rig의 EOS 700D는 orientation 6/8을 실제로 만든다.
    /// 비교 lane은 그 값을 거부하지 않고 **기록**해야 한다. display 승인 정책(1만 허용)은
    /// `image_probe::probe_jpeg`에 그대로 남아 있다.
    #[test]
    fn admits_real_camera_orientation_and_records_it() {
        let bytes = build_jpeg(5184, 3456, Some(6));
        let mut candidate = candidate_for(&bytes, 5184, 3456);
        candidate.exif_orientation = Some(6);

        assert_eq!(
            evaluate(&candidate, SourceProducerOutcome::Produced, Some(&bytes)),
            Ok(())
        );
    }

    /// EXIF 유효 범위(1~8) 밖의 orientation은 여전히 지원하지 않는다.
    #[test]
    fn rejects_orientation_outside_exif_range() {
        let bytes = build_jpeg(5184, 3456, Some(9));
        let candidate = candidate_for(&bytes, 5184, 3456);

        assert_eq!(
            evaluate(&candidate, SourceProducerOutcome::Produced, Some(&bytes)),
            Err(SOURCE_REJECT_ORIENTATION_UNSUPPORTED)
        );
    }

    /// CR2는 orientation을 컨테이너 쪽에 두는 경우가 많다. JPEG 자체 EXIF가 없으면
    /// host가 읽은 컨테이너 값이 truth이고, 기록도 그 값과 일치해야 한다.
    #[test]
    fn admits_container_declared_orientation_via_context() {
        let bytes = build_jpeg(5184, 3456, None);
        let mut candidate = candidate_for(&bytes, 5184, 3456);
        candidate.exif_orientation = Some(8);

        let mut with_container = context();
        with_container.container_orientation = Some(8);

        assert_eq!(
            evaluate_source_candidate(
                &candidate,
                SourceProducerOutcome::Produced,
                Some(&bytes),
                &with_container
            ),
            Ok(())
        );

        // 컨테이너 값이 있는데 기록이 비어 있으면 그 표본은 신뢰할 수 없다.
        candidate.exif_orientation = None;
        assert_eq!(
            evaluate_source_candidate(
                &candidate,
                SourceProducerOutcome::Produced,
                Some(&bytes),
                &with_container
            ),
            Err(SOURCE_REJECT_CORRUPT)
        );
    }

    #[test]
    fn rejects_corrupt_metadata_mismatch() {
        let bytes = build_jpeg(5184, 3456, None);
        // 파일은 5184×3456인데 표본은 1600×1600이라고 주장한다.
        let candidate = candidate_for(&bytes, 1600, 1600);

        assert_eq!(
            evaluate(&candidate, SourceProducerOutcome::Produced, Some(&bytes)),
            Err(SOURCE_REJECT_CORRUPT)
        );
    }

    #[test]
    fn rejects_corrupt_hash_mismatch() {
        let bytes = build_jpeg(5184, 3456, None);
        let mut candidate = candidate_for(&bytes, 5184, 3456);
        candidate.source_hash = Some("fnv1a64:0000000000000000".into());

        assert_eq!(
            evaluate(&candidate, SourceProducerOutcome::Produced, Some(&bytes)),
            Err(SOURCE_REJECT_CORRUPT)
        );
    }

    #[test]
    fn rejects_an_accepted_candidate_without_a_provenance_hash() {
        let bytes = build_jpeg(5184, 3456, None);
        let mut candidate = candidate_for(&bytes, 5184, 3456);
        candidate.source_hash = None;

        assert_eq!(
            evaluate(&candidate, SourceProducerOutcome::Produced, Some(&bytes)),
            Err(SOURCE_REJECT_CORRUPT)
        );
    }

    #[test]
    fn rejects_recorded_orientation_that_disagrees_with_the_jpeg() {
        let bytes = build_jpeg(5184, 3456, Some(1));
        let candidate = candidate_for(&bytes, 5184, 3456);

        assert_eq!(
            evaluate(&candidate, SourceProducerOutcome::Produced, Some(&bytes)),
            Err(SOURCE_REJECT_CORRUPT)
        );
    }

    #[test]
    fn rejects_a_produced_candidate_when_the_asset_cannot_be_read() {
        let bytes = build_jpeg(5184, 3456, None);
        let candidate = candidate_for(&bytes, 5184, 3456);

        assert_eq!(
            evaluate(&candidate, SourceProducerOutcome::Produced, None),
            Err(SOURCE_REJECT_EXTRACTION_FAILED)
        );
    }

    #[test]
    fn rejects_stale_artifact_from_an_earlier_capture() {
        let bytes = build_jpeg(5184, 3456, None);
        let mut candidate = candidate_for(&bytes, 5184, 3456);
        candidate.ready_at_host_micros = Some(REQUEST_STARTED - 1);

        assert_eq!(
            evaluate(&candidate, SourceProducerOutcome::Produced, Some(&bytes)),
            Err(SOURCE_REJECT_STALE)
        );
    }

    #[test]
    fn rejects_unsupported_combination_without_attempting() {
        let bytes = build_jpeg(5184, 3456, None);
        let candidate = candidate_for(&bytes, 5184, 3456);

        assert_eq!(
            evaluate(
                &candidate,
                SourceProducerOutcome::UnsupportedCombination,
                None
            ),
            Err(SOURCE_REJECT_UNSUPPORTED_COMBINATION)
        );
    }

    #[test]
    fn rejects_route_that_is_not_enabled_in_this_lane() {
        let bytes = build_jpeg(5184, 3456, None);
        let candidate = candidate_for(&bytes, 5184, 3456);
        let only_shell = [SOURCE_ROUTE_WINDOWS_SHELL_THUMBNAIL];
        let mut narrowed = context();
        narrowed.enabled_routes = &only_shell;

        assert_eq!(
            evaluate_source_candidate(
                &candidate,
                SourceProducerOutcome::Produced,
                Some(&bytes),
                &narrowed
            ),
            Err(SOURCE_REJECT_UNSUPPORTED_COMBINATION)
        );
    }

    #[test]
    fn rejects_extraction_failure() {
        let bytes = build_jpeg(5184, 3456, None);
        let candidate = candidate_for(&bytes, 5184, 3456);

        assert_eq!(
            evaluate(&candidate, SourceProducerOutcome::ExtractionFailed, None),
            Err(SOURCE_REJECT_EXTRACTION_FAILED)
        );
    }

    #[test]
    fn rejects_cancelled_transfer() {
        let bytes = build_jpeg(5184, 3456, None);
        let candidate = candidate_for(&bytes, 5184, 3456);

        assert_eq!(
            evaluate(&candidate, SourceProducerOutcome::Cancelled, None),
            Err(SOURCE_REJECT_CANCELLED)
        );
    }

    /// 세션 불일치는 다른 어떤 결함보다 먼저 잘려야 한다 (NFR-004).
    #[test]
    fn session_mismatch_wins_over_every_other_defect() {
        let png = [0x89u8, b'P', b'N', b'G'];
        let mut candidate = candidate_for(&png, 1, 1);
        candidate.session_id = OTHER_SESSION.into();
        candidate.asset_path = None;

        assert_eq!(
            evaluate(&candidate, SourceProducerOutcome::Cancelled, Some(&png)),
            Err(SOURCE_REJECT_WRONG_SESSION)
        );
    }

    /// capture id가 아직 확정되지 않은 시점의 후보는 capture 검사를 건너뛴다.
    #[test]
    fn skips_capture_check_before_raw_handoff_assigns_one() {
        let bytes = build_jpeg(5184, 3456, None);
        let mut candidate = candidate_for(&bytes, 5184, 3456);
        candidate.capture_id = None;

        let mut unassigned = context();
        unassigned.expected_capture_id = None;

        assert_eq!(
            evaluate_source_candidate(
                &candidate,
                SourceProducerOutcome::Produced,
                Some(&bytes),
                &unassigned
            ),
            Ok(())
        );
    }

    #[test]
    fn probe_source_bytes_reuses_story_72_rules() {
        let bytes = build_jpeg(5184, 3456, None);
        let probe = probe_source_bytes(&bytes).expect("probe should succeed");

        assert_eq!(probe.width_px, 5184);
        assert_eq!(probe.height_px, 3456);
        assert_eq!(
            probe_source_bytes(&bytes[..bytes.len() - 2]),
            Err(SOURCE_REJECT_PARTIAL)
        );
    }

    #[test]
    fn block_order_is_deterministic_and_uses_both_orders() {
        assert_eq!(block_order_for(42, 0), block_order_for(42, 0));

        let orders: Vec<&str> = (0..32).map(|index| block_order_for(42, index)).collect();

        assert!(orders.contains(&"AB"));
        assert!(orders.contains(&"BA"));
    }
}
