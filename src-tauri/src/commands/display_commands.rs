//! Story 7.2: display pointer 조회/게시와 actual-present 계측 진입점.
//!
//! 이 파일은 얇은 경계다. 승격 판정은 `display::display_artifact`, 파일 커밋은
//! `display::generation_repository`, 게시 순서는 `display::sample_publisher`가 소유한다.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use tauri::path::BaseDirectory;
use tauri::{Emitter, Manager};

use crate::{
    contracts::dto::{
        validate_display_present_report, validate_display_sample_publish_input,
        validate_trusted_input_report, ClockProbeResultDto, DisplayPointerSnapshotDto,
        DisplayPresentReportDto, DisplaySamplePublishInputDto, DisplayUpdateDto, HostErrorEnvelope,
        TrustedInputReportDto, DISPLAY_REJECT_PRESENT_UNREPORTED, DISPLAY_REJECT_STALE_EPOCH,
        DISPLAY_REJECT_UNKNOWN_GENERATION,
    },
    display::{
        current_display_lane_flags,
        generation_publisher::{publish_generation_in_dir, PublishRequest},
        generation_repository::{remove_request_generations, write_reconciled_pointer},
        proxy_publisher::{
            current_proxy_lane_mode, evaluate_proxy_eligibility, publish_rendered_proxy_in_dir,
            render_display_proxy, DisplayProxyJob, ProxyRenderError, ProxySkipReason,
            ProxySourceRoute,
        },
        raw_refined_publisher::{
            current_raw_refined_lane_mode, evaluate_raw_refined_eligibility,
            publish_rendered_raw_refined_in_dir, render_raw_refined_display,
            InheritedProxyGeometry, RawRefinedJob, RawRefinedRenderError, RawRefinedSkipReason,
            RAW_REFINED_TIER_JUSTIFICATION,
        },
        sample_publisher::{
            current_sample_gap_ms, current_sample_lane_mode, sample_fixture_path, SampleLaneMode,
        },
        DisplayStateHandle,
    },
    preset::preset_catalog::{
        find_published_preset_runtime_bundle, resolve_published_preset_catalog_dir,
    },
    render::{scheduler_for, CancelScope},
    session::session_repository::resolve_app_session_base_dir,
    viewer::{
        current_epoch_micros, current_monotonic_micros,
        present_telemetry::{
            append_present_sample, classify_confidence, conservative_latency_micros,
            viewer_window_event_count_between, PresentSampleRecord, PRESENT_CONFIDENCE_UNREPORTED,
            PRESENT_TELEMETRY_SCHEMA_VERSION,
        },
        ViewerStateHandle,
    },
};

pub const VIEWER_DISPLAY_UPDATE_EVENT: &str = "viewer-display-update";
pub const VIEWER_WINDOW_LABEL: &str = "viewer-window";

/// viewer만 표시 결과를 보고할 수 있다. 다른 창의 보고는 계측을 오염시킨다.
pub fn is_viewer_window_label(label: &str) -> bool {
    label == VIEWER_WINDOW_LABEL
}

/// client `performance.now()` micros를 host monotonic micros로 옮긴다.
/// `offset = host - client`이므로 `host = client + offset`이다.
pub fn to_host_micros(client_micros: u64, clock_offset_micros: i64) -> u64 {
    let converted = client_micros as i128 + clock_offset_micros as i128;

    converted.clamp(0, u64::MAX as i128) as u64
}

/// commit·notify가 끝난 generation의 terminal 보고를 기다리는 유예 시간.
///
/// 실측(HV-13B)에서 event emit → actual-present는 200~400ms였고 viewer의 after-paint fallback은
/// 250ms다. 그보다 훨씬 넉넉하게 잡아, 이 시간을 넘겨서야 도착하는 보고는 정상 경로가 아니라
/// 이상 현상으로 다룬다.
pub const PRESENT_REPORT_GRACE_MICROS: u64 = 2_000_000;

/// Story 7.4. proxy 표본을 fixture 표본과 구분하고 출처를 남기는 요약.
#[derive(Debug, Default, Clone)]
struct GenerationProvenanceSummary {
    tier: Option<String>,
    preset_id: Option<String>,
    preset_version: Option<String>,
    source_route: Option<String>,
    target_width_px: Option<u32>,
    target_height_px: Option<u32>,
    /// Story 7.6. `fast` / `high`. `tier`와 어긋난 행을 evidence에서 잡아낸다.
    render_quality: Option<String>,
}

impl GenerationProvenanceSummary {
    fn from_generation(generation: &crate::contracts::dto::DisplayGenerationDto) -> Self {
        let provenance = generation.proxy_provenance.as_ref();

        Self {
            tier: Some(generation.tier.clone()),
            preset_id: provenance.map(|value| value.preset_id.clone()),
            preset_version: provenance.map(|value| value.preset_version.clone()),
            source_route: provenance.map(|value| value.source_route.clone()),
            target_width_px: provenance.map(|value| value.target_width_px),
            target_height_px: provenance.map(|value| value.target_height_px),
            render_quality: provenance.map(|value| value.render_quality.clone()),
        }
    }

    fn is_raw_refined(&self) -> bool {
        self.tier.as_deref() == Some(crate::contracts::dto::DISPLAY_TIER_RAW_REFINED_DISPLAY)
    }
}

#[derive(Debug, Default, Clone)]
struct GenerationSpans {
    sample_write_start_at_micros: Option<u64>,
    file_ready_at_micros: Option<u64>,
    probe_ok_at_micros: Option<u64>,
    pointer_committed_at_micros: Option<u64>,
    event_emitted_at_micros: Option<u64>,
    sample_variant: Option<String>,
    /// Story 7.4. proxy 구간 진단 span.
    proxy_source_ready_at_micros: Option<u64>,
    proxy_queue_wait_micros: Option<u64>,
    proxy_render_start_at_micros: Option<u64>,
    proxy_process_exited_at_micros: Option<u64>,
    /// Story 7.6. 정밀본 구간과 스케줄러 구간.
    raw_refined_enqueued_at_micros: Option<u64>,
    raw_refined_queue_wait_micros: Option<u64>,
    raw_refined_render_start_at_micros: Option<u64>,
    raw_refined_process_exited_at_micros: Option<u64>,
    raw_refined_committed_at_micros: Option<u64>,
    scheduler_priority: Option<String>,
    scheduler_deadline_micros: Option<u64>,
    scheduler_deadline_missed: Option<bool>,
    scheduler_coalesced_count: Option<u32>,
    scheduler_preempted_by: Option<String>,
    provenance: GenerationProvenanceSummary,
    /// generation당 terminal 행은 **정확히 하나**다. 먼저 도착한 경로가 그 행을 쓴다.
    terminal_recorded: bool,
}

#[derive(Debug, Default)]
struct PendingRequest {
    session_id: String,
    trusted_input_at_micros: Option<u64>,
    input_uncertainty_micros: u64,
    is_trusted_input: bool,
    host_accepted_at_micros: Option<u64>,
    generations: HashMap<String, GenerationSpans>,
    pending_present_reports: HashMap<String, DisplayPresentReportDto>,
}

#[derive(Debug, Clone)]
struct GenerationContext {
    session_id: String,
    request_id: String,
    sample_variant: Option<String>,
    provenance: GenerationProvenanceSummary,
    /// 이 generation이 검증된 viewer 세대.
    ///
    /// 늦게 도착한 보고가 **다른 세대의 generation과 짝지어지는 것**을 막는다.
    /// 한 촬영에 generation이 둘(proxy·정밀본)이 되면서 잘못된 짝짓기 위험이 커졌다.
    viewer_epoch: u64,
    /// 등록 순서. 오래된 tombstone부터 버리기 위해서만 쓴다.
    registered_at_micros: u64,
}

/// correlation tombstone 보관 상한.
///
/// 이 map은 capture 삭제나 세션 교체 **뒤에** 도착한 보고도 어느 세션의 거부인지 적을 수 있게
/// 유지된다. 그래서 terminal 행이 남았다고 바로 지울 수 없다. 대신 상한을 두고 오래된 것부터
/// 버린다 — Story 7.6이 촬영당 generation을 2개로 늘렸으므로 누수량이 두 배가 됐다
/// (`deferred-work.md` 미결 항목).
const MAX_RETAINED_GENERATION_CONTEXTS: usize = 512;

/// commit됐지만 아직 terminal 행이 없는 generation.
#[derive(Debug, Clone)]
struct OpenGeneration {
    generation_id: String,
    request_id: String,
    session_id: String,
    /// 이 시각을 넘기면 viewer 보고가 오지 않은 것으로 확정한다.
    deadline_at_micros: u64,
}

/// terminal 행이 없는 generation을 닫는 범위.
enum CloseOutScope {
    /// 유예 시간이 지난 것만 (정상 경로).
    Due(u64),
    /// 이 세션에 속하지 않는 전부. 세션 교체 시 이전 세션의 증거를 완결시킨다.
    OtherSessions(String),
    /// 이 request의 전부. capture 삭제 시 그 request의 증거를 완결시킨다.
    Request(String),
}

#[derive(Debug, Default)]
struct TelemetryState {
    requests: HashMap<String, PendingRequest>,
    // capture 삭제나 세션 교체 뒤의 늦은 viewer 보고도 어느 세션의 거부인지 기록할 수 있게
    // 프로세스 수명 동안 작은 correlation tombstone을 유지한다. 계측 lane에서만 채워진다.
    generation_contexts: HashMap<String, GenerationContext>,
    /// terminal 행을 기다리는 generation들. 여기가 비어야 계측이 완결된다.
    open_generations: Vec<OpenGeneration>,
}

fn telemetry_state() -> &'static Mutex<TelemetryState> {
    static STATE: OnceLock<Mutex<TelemetryState>> = OnceLock::new();

    STATE.get_or_init(|| Mutex::new(TelemetryState::default()))
}

/// correlation tombstone을 상한 안으로 줄인다. **오래된 것부터** 버린다.
///
/// 늦게 도착하는 보고는 최근 generation의 것이므로, 오래된 tombstone을 버려도
/// 실제로 필요한 correlation은 남는다. 아직 terminal 행이 없는(미결) generation은
/// 절대 버리지 않는다 — 버리면 계측 완결성이 깨진다.
fn prune_generation_contexts(telemetry: &mut TelemetryState) {
    if telemetry.generation_contexts.len() <= MAX_RETAINED_GENERATION_CONTEXTS {
        return;
    }

    let open: std::collections::HashSet<String> = telemetry
        .open_generations
        .iter()
        .map(|open| open.generation_id.clone())
        .collect();

    let mut evictable: Vec<(String, u64)> = telemetry
        .generation_contexts
        .iter()
        .filter(|(generation_id, _)| !open.contains(*generation_id))
        .map(|(generation_id, context)| (generation_id.clone(), context.registered_at_micros))
        .collect();
    evictable.sort_by_key(|(_, registered_at_micros)| *registered_at_micros);

    let excess = telemetry
        .generation_contexts
        .len()
        .saturating_sub(MAX_RETAINED_GENERATION_CONTEXTS);

    for (generation_id, _) in evictable.into_iter().take(excess) {
        telemetry.generation_contexts.remove(&generation_id);
    }
}

/// 다른 command 모듈이 같은 세션 root를 쓰게 하는 좁은 재노출.
///
/// 경로 해석 규칙을 모듈마다 다시 적으면 두 규칙이 갈라지고, 그 순간
/// evidence와 제품이 서로 다른 디렉터리를 본다.
pub fn session_base_dir_for(app: &tauri::AppHandle) -> Result<PathBuf, HostErrorEnvelope> {
    session_base_dir(app)
}

fn session_base_dir(app: &tauri::AppHandle) -> Result<PathBuf, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;

    Ok(resolve_app_session_base_dir(app_local_data_dir))
}

/// 현재 viewer truth(세션 binding, epoch, photo rect, 승인 display profile).
#[derive(Debug, Clone, Default)]
struct ViewerContext {
    session_id: Option<String>,
    viewer_epoch: u64,
    required_source_width_px: u32,
    required_source_height_px: u32,
    /// Story 7.4. proxy provenance에 실린다. 미측정 상태에서는 명시적으로 비운다.
    display_profile_id: Option<String>,
    device_pixel_ratio: Option<f64>,
}

fn read_viewer_context(app: &tauri::AppHandle) -> ViewerContext {
    let handle = app.state::<ViewerStateHandle>();
    let state = handle.0.lock().expect("viewer state lock poisoned");
    let snapshot = state.snapshot_with_clock(
        crate::viewer::current_epoch_ms(),
        crate::viewer::current_monotonic_ms(),
    );
    let (required_width, required_height, device_pixel_ratio) = snapshot
        .photo_rect
        .as_ref()
        .map(|rect| {
            (
                rect.required_source_width_px,
                rect.required_source_height_px,
                Some(rect.device_pixel_ratio),
            )
        })
        .unwrap_or((0, 0, None));

    ViewerContext {
        session_id: snapshot.session_id,
        viewer_epoch: snapshot.viewer_epoch,
        required_source_width_px: required_width,
        required_source_height_px: required_height,
        display_profile_id: snapshot
            .display_profile
            .as_ref()
            .map(|profile| profile.profile_id.clone()),
        device_pixel_ratio,
    }
}

fn emit_display_update(app: &tauri::AppHandle, pointer: DisplayPointerSnapshotDto) {
    let _ = app.emit(VIEWER_DISPLAY_UPDATE_EVENT, DisplayUpdateDto::new(pointer));
}

/// pointer를 현재 viewer truth와 맞추고 snapshot을 돌려준다.
///
/// 세션 교체와 photo rect 확대로 인한 무효화가 여기서 일어난다. revision이 실제로 오를 때만 emit한다.
pub fn read_display_snapshot(
    app: &tauri::AppHandle,
) -> Result<DisplayPointerSnapshotDto, HostErrorEnvelope> {
    let viewer = read_viewer_context(app);
    let bound_session_id = viewer.session_id.clone();
    let required_width = viewer.required_source_width_px;
    let required_height = viewer.required_source_height_px;

    let (snapshot, changed, previous_session_id) = {
        let handle = app.state::<DisplayStateHandle>();
        let mut state = handle.0.lock().expect("display state lock poisoned");
        let previous_session_id = state.session_id().map(str::to_string);
        let changed = state.reconcile(bound_session_id.as_deref(), required_width, required_height);

        (
            state.snapshot(
                required_width,
                required_height,
                current_display_lane_flags(),
                current_monotonic_micros(),
            ),
            changed,
            previous_session_id,
        )
    };

    if changed {
        let base_dir = session_base_dir(app)?;
        write_reconciled_pointer(&base_dir, previous_session_id.as_deref(), &snapshot)?;
        emit_display_update(app, snapshot.clone());
    }

    Ok(snapshot)
}

/// 세션 binding 변경 시 표시 상태를 즉시 정리한다. NFR-004는 0 tolerance다.
pub fn bind_display_session(
    app: &tauri::AppHandle,
    session_id: &str,
) -> Result<(), HostErrorEnvelope> {
    // Story 7.6. 이전 세션의 렌더를 **모든 우선순위에서** 취소한다.
    // 세션이 끝났으면 그 세션의 final도 더 이상 만들 이유가 없다.
    cancel_display_lane_renders(app, CancelScope::OtherSessions(session_id.to_string()));

    // 이전 세션의 미결 generation을 **버리기 전에** 닫는다. 세션이 끝났다는 이유로
    // 계측 행이 사라지면 그 세션의 evidence 분모가 조용히 줄어든다.
    close_out_open_generations(app, CloseOutScope::OtherSessions(session_id.to_string()));

    {
        let mut telemetry = telemetry_state()
            .lock()
            .expect("telemetry state lock poisoned");
        telemetry
            .requests
            .retain(|_, pending| pending.session_id == session_id);
        // 세션이 바뀌면 이전 세션의 correlation tombstone도 더 이상 쓰이지 않는다.
        // 미결 generation은 위의 close-out이 이미 닫았다.
        telemetry
            .generation_contexts
            .retain(|_, context| context.session_id == session_id);
    }

    read_display_snapshot(app)?;
    Ok(())
}

/// capture 삭제 시 해당 request의 표시 자산과 pointer를 함께 정리한다.
pub fn forget_display_request(
    app: &tauri::AppHandle,
    session_id: &str,
    request_id: &str,
) -> Result<(), HostErrorEnvelope> {
    // Story 7.6. 삭제된 촬영의 렌더를 **모든 우선순위에서** 취소한다.
    // 지워질 사진의 final을 계속 만드는 것은 순수 낭비다.
    cancel_display_lane_renders(app, CancelScope::Request(request_id.to_string()));

    // 삭제 전에 이 request의 미결 generation을 닫는다.
    close_out_open_generations(app, CloseOutScope::Request(request_id.to_string()));

    {
        let mut telemetry = telemetry_state()
            .lock()
            .expect("telemetry state lock poisoned");
        telemetry.requests.remove(request_id);
    }

    let base_dir = session_base_dir(app)?;
    let viewer = read_viewer_context(app);
    let required_width = viewer.required_source_width_px;
    let required_height = viewer.required_source_height_px;
    let (changed, snapshot) = {
        let handle = app.state::<DisplayStateHandle>();
        let mut state = handle.0.lock().expect("display state lock poisoned");
        let changed = state.forget_request(request_id);
        let snapshot = state.snapshot(
            required_width,
            required_height,
            current_display_lane_flags(),
            current_monotonic_micros(),
        );
        (changed, snapshot)
    };

    if changed {
        write_reconciled_pointer(&base_dir, Some(session_id), &snapshot)?;
        emit_display_update(app, snapshot.clone());
    }

    remove_request_generations(
        &base_dir,
        session_id,
        request_id,
        current_monotonic_micros(),
    )?;

    Ok(())
}

fn resolve_sample_fixture_bytes(app: &tauri::AppHandle, variant: &str) -> Option<Vec<u8>> {
    let relative = format!("fixtures/display-sample/sample-{variant}.jpg");

    if let Ok(resource_path) = app.path().resolve(&relative, BaseDirectory::Resource) {
        if let Ok(bytes) = std::fs::read(&resource_path) {
            return Some(bytes);
        }
    }

    // dev 루프에서는 resource 디렉터리가 없을 수 있다. 저장소 fixture로 대체한다.
    let repo_fallback = sample_fixture_path(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("storage"),
        variant,
    );

    std::fs::read(repo_fallback).ok()
}

/// `request_viewer_epoch`가 `None`이면 지금 관측한 epoch를 request 세대로 삼는다.
/// sample lane은 request 수락 시점의 epoch를 넘겨, 그 사이 창이 재생성되면 게시가 거부되게 한다.
fn publish_sample_generation(
    app: &tauri::AppHandle,
    input: &DisplaySamplePublishInputDto,
    request_viewer_epoch: Option<u64>,
) -> Result<DisplayPointerSnapshotDto, HostErrorEnvelope> {
    let base_dir = session_base_dir(app)?;
    let viewer = read_viewer_context(app);
    let bound_session_id = viewer.session_id.clone();
    let current_viewer_epoch = viewer.viewer_epoch;
    let required_width = viewer.required_source_width_px;
    let required_height = viewer.required_source_height_px;

    let source_bytes =
        resolve_sample_fixture_bytes(app, &input.sample_variant).ok_or_else(|| {
            HostErrorEnvelope::persistence("표시 샘플 자산을 찾지 못했어요.".to_string())
        })?;

    let write_start_at_micros = current_monotonic_micros();
    let request = PublishRequest {
        session_id: &input.session_id,
        request_id: &input.request_id,
        capture_id: input.capture_id.as_deref(),
        tier: crate::contracts::dto::DISPLAY_TIER_SAMPLE,
        sample_variant: Some(&input.sample_variant),
        proxy_provenance: None,
        source_bytes: &source_bytes,
        bound_session_id: bound_session_id.as_deref(),
        request_viewer_epoch: request_viewer_epoch.unwrap_or(current_viewer_epoch),
        current_viewer_epoch,
        // fixture lane은 촬영과 무관하므로 capture 순서 좌표를 갖지 않는다.
        capture_order: None,
        required_source_width_px: required_width,
        required_source_height_px: required_height,
        lanes: current_display_lane_flags(),
        // fixture lane은 정밀본 tier를 게시하지 않는다. AC 6 gate와 무관하다.
        refined_tier_justified: false,
    };

    let outcome = {
        let handle = app.state::<DisplayStateHandle>();
        let mut state = handle.0.lock().expect("display state lock poisoned");

        publish_generation_in_dir(&base_dir, &mut state, &request, &current_monotonic_micros)?
    };

    if let Some(reason) = outcome.reject_reason.as_deref() {
        log::warn!(
            "display_sample_rejected session={} request={} variant={} reason={}",
            input.session_id,
            input.request_id,
            input.sample_variant,
            reason
        );

        return Ok(outcome.pointer);
    }

    let Some(generation) = outcome.committed.as_ref() else {
        return Ok(outcome.pointer);
    };

    // hidden prewarm 변형은 첫 qualifying generation에서 창을 드러낸다.
    // 그 비용이 present 계측에 그대로 포함되어야 A/B가 정직하다.
    crate::commands::viewer_commands::reveal_prewarmed_viewer_window(app);

    // pointer commit이 끝난 뒤에야 notify한다.
    emit_display_update(app, outcome.pointer.clone());
    let event_emitted_at_micros = current_monotonic_micros();

    register_committed_generation(
        generation,
        &outcome,
        CommittedGenerationSpans {
            write_start_at_micros: Some(write_start_at_micros),
            event_emitted_at_micros,
            ..CommittedGenerationSpans::default()
        },
    );

    // 유예 시간이 지난 이전 generation을 여기서 닫는다. 계측 lane 안에서만 도는 경로다.
    close_out_open_generations(app, CloseOutScope::Due(current_monotonic_micros()));

    log::info!(
        "display_sample_committed session={} request={} generation={} variant={} size={}x{}",
        generation.session_id,
        generation.request_id,
        generation.generation_id,
        generation.sample_variant.as_deref().unwrap_or("none"),
        generation.source_width_px,
        generation.source_height_px
    );

    Ok(outcome.pointer)
}

/// 게시 경로가 채워 넣는 lane 고유 span.
#[derive(Debug, Default, Clone)]
struct CommittedGenerationSpans {
    write_start_at_micros: Option<u64>,
    event_emitted_at_micros: u64,
    proxy_source_ready_at_micros: Option<u64>,
    proxy_queue_wait_micros: Option<u64>,
    proxy_render_start_at_micros: Option<u64>,
    proxy_process_exited_at_micros: Option<u64>,
    /// Story 7.6. 정밀본 lane 전용 span. proxy lane에서는 전부 `None`이다.
    raw_refined_enqueued_at_micros: Option<u64>,
    raw_refined_queue_wait_micros: Option<u64>,
    raw_refined_render_start_at_micros: Option<u64>,
    raw_refined_process_exited_at_micros: Option<u64>,
    raw_refined_committed_at_micros: Option<u64>,
    /// Story 7.6. 두 display lane 모두 채운다. **큐 대기 비율 계산의 근거다.**
    scheduler_priority: Option<String>,
    scheduler_deadline_micros: Option<u64>,
    scheduler_deadline_missed: Option<bool>,
    scheduler_coalesced_count: Option<u32>,
    scheduler_preempted_by: Option<String>,
}

/// commit된 generation을 계측에 등록한다. **sample lane과 proxy lane이 같은 함수를 쓴다.**
///
/// 두 lane이 서로 다른 등록 경로를 가지면 한쪽에서만 terminal 행이 새고,
/// 그것이 Story 7.2가 한 회차를 잃은 방식이다.
fn register_committed_generation(
    generation: &crate::contracts::dto::DisplayGenerationDto,
    outcome: &crate::display::generation_publisher::PublishOutcome,
    spans: CommittedGenerationSpans,
) {
    let provenance = GenerationProvenanceSummary::from_generation(generation);
    let mut telemetry = telemetry_state()
        .lock()
        .expect("telemetry state lock poisoned");

    telemetry.generation_contexts.insert(
        generation.generation_id.clone(),
        GenerationContext {
            session_id: generation.session_id.clone(),
            request_id: generation.request_id.clone(),
            sample_variant: generation.sample_variant.clone(),
            provenance: provenance.clone(),
            viewer_epoch: generation.viewer_epoch,
            registered_at_micros: spans.event_emitted_at_micros,
        },
    );
    prune_generation_contexts(&mut telemetry);

    let pending = telemetry
        .requests
        .entry(generation.request_id.clone())
        .or_insert_with(|| PendingRequest {
            session_id: generation.session_id.clone(),
            ..PendingRequest::default()
        });

    pending.generations.insert(
        generation.generation_id.clone(),
        GenerationSpans {
            sample_write_start_at_micros: spans.write_start_at_micros,
            file_ready_at_micros: outcome.file_ready_at_micros,
            probe_ok_at_micros: outcome.probe_ok_at_micros,
            pointer_committed_at_micros: outcome.pointer_committed_at_micros,
            event_emitted_at_micros: Some(spans.event_emitted_at_micros),
            sample_variant: generation.sample_variant.clone(),
            proxy_source_ready_at_micros: spans.proxy_source_ready_at_micros,
            proxy_queue_wait_micros: spans.proxy_queue_wait_micros,
            proxy_render_start_at_micros: spans.proxy_render_start_at_micros,
            proxy_process_exited_at_micros: spans.proxy_process_exited_at_micros,
            raw_refined_enqueued_at_micros: spans.raw_refined_enqueued_at_micros,
            raw_refined_queue_wait_micros: spans.raw_refined_queue_wait_micros,
            raw_refined_render_start_at_micros: spans.raw_refined_render_start_at_micros,
            raw_refined_process_exited_at_micros: spans.raw_refined_process_exited_at_micros,
            raw_refined_committed_at_micros: spans.raw_refined_committed_at_micros,
            scheduler_priority: spans.scheduler_priority,
            scheduler_deadline_micros: spans.scheduler_deadline_micros,
            scheduler_deadline_missed: spans.scheduler_deadline_missed,
            scheduler_coalesced_count: spans.scheduler_coalesced_count,
            scheduler_preempted_by: spans.scheduler_preempted_by,
            provenance,
            terminal_recorded: false,
        },
    );

    // 이 generation은 terminal 행이 남을 때까지 미결이다.
    telemetry.open_generations.push(OpenGeneration {
        generation_id: generation.generation_id.clone(),
        request_id: generation.request_id.clone(),
        session_id: generation.session_id.clone(),
        deadline_at_micros: spans
            .event_emitted_at_micros
            .saturating_add(PRESENT_REPORT_GRACE_MICROS),
    });
}

/// 계측 lane이 켜져 있을 때만, 한 request에 sample A와 B를 순서대로 게시한다.
///
/// **`request_capture`를 지연시키지 않는다.** 전부 background 스레드에서 수행한다.
pub fn spawn_sample_lane_publication(
    app: &tauri::AppHandle,
    session_id: String,
    request_id: String,
    capture_id: Option<String>,
) {
    let mode = current_sample_lane_mode();

    if !mode.is_enabled() {
        return;
    }

    if current_proxy_lane_mode().is_enabled() {
        log::warn!(
            "display_lane_conflict session={session_id} request={request_id} resolution=proxy-priority"
        );
        return;
    }

    let gap_ms = current_sample_gap_ms();
    let lane_app = app.clone();
    // request가 수락된 시점의 viewer 세대. 게시 사이에 창이 재생성되면 stale-epoch로 거부된다.
    let request_viewer_epoch = read_viewer_context(app).viewer_epoch;
    {
        let mut telemetry = telemetry_state()
            .lock()
            .expect("telemetry state lock poisoned");
        let pending = telemetry
            .requests
            .entry(request_id.clone())
            .or_insert_with(|| PendingRequest {
                session_id: session_id.clone(),
                ..PendingRequest::default()
            });
        pending.session_id = session_id.clone();
        pending
            .host_accepted_at_micros
            .get_or_insert_with(current_monotonic_micros);
    }

    thread::spawn(move || {
        for (index, variant) in ["a", "b"].into_iter().enumerate() {
            if index > 0 {
                thread::sleep(Duration::from_millis(gap_ms));
            }

            let input = DisplaySamplePublishInputDto {
                session_id: session_id.clone(),
                request_id: request_id.clone(),
                capture_id: capture_id.clone(),
                sample_variant: variant.into(),
            };

            if let Err(error) =
                publish_sample_generation(&lane_app, &input, Some(request_viewer_epoch))
            {
                log::warn!(
                    "display_sample_publish_failed session={session_id} request={request_id} variant={variant} error={}",
                    error.message
                );
            }
        }

        // 마지막 generation도 반드시 닫힌다. 다음 촬영이 없어도 계측이 완결되도록,
        // 유예 시간이 지난 뒤 이 lane 스레드가 직접 sweep한다.
        thread::sleep(Duration::from_micros(
            PRESENT_REPORT_GRACE_MICROS.saturating_add(250_000),
        ));
        close_out_open_generations(&lane_app, CloseOutScope::Due(current_monotonic_micros()));
    });
}

/// **촬영 수락 시점의** viewer 세대. 게시 시점이 아니라 이때 읽어야 한다.
///
/// 게시는 수 초 뒤에 일어나고, 그 사이 관람 창이 재생성되면 그 자산은 죽은 세대에 속한다.
pub fn current_viewer_epoch_for_request(app: &tauri::AppHandle) -> u64 {
    read_viewer_context(app).viewer_epoch
}

/// Story 7.4. 촬영이 확정된 시점에 이 capture의 순서 좌표를 고정한다.
///
/// **게시 시점이 아니라 촬영 시점이어야 한다.** proxy 렌더가 3.5초 걸리므로 완료 순서가
/// 촬영 순서와 갈린다. 이 좌표가 없으면 먼저 찍고 늦게 끝난 사진이 나중 사진을 덮는다.
pub fn observe_capture_order(app: &tauri::AppHandle, capture_id: &str) -> u64 {
    let handle = app.state::<DisplayStateHandle>();
    let mut state = handle.0.lock().expect("display state lock poisoned");

    state.observe_capture(capture_id)
}

/// Story 7.4. 한 촬영의 display-fit preset proxy를 만들어 관람 화면에 게시한다.
///
/// **촬영 경로의 반환을 지연시키지 않는다.** 전부 background 스레드에서 수행한다.
/// 렌더는 수 초가 걸리므로 `DisplayState` mutex를 잡은 채로 돌지 않는다 —
/// 잡으면 그 시간 동안 관람 화면의 pointer 조회가 통째로 막힌다.
#[allow(clippy::too_many_arguments)]
pub fn spawn_display_proxy_publication(
    app: &tauri::AppHandle,
    session_id: String,
    request_id: String,
    capture_id: String,
    preset_id: Option<String>,
    preset_version: String,
    source_path: PathBuf,
    request_viewer_epoch: u64,
) {
    if !current_proxy_lane_mode().is_enabled() {
        return;
    }

    // 촬영 시점 좌표를 **스레드를 띄우기 전에** 고정한다.
    let capture_order = observe_capture_order(app, &capture_id);
    let source_ready_at_micros = current_monotonic_micros();
    let lane_app = app.clone();

    thread::spawn(move || {
        if let Err(error) = run_display_proxy_publication(
            &lane_app,
            &session_id,
            &request_id,
            &capture_id,
            preset_id.as_deref(),
            &preset_version,
            &source_path,
            request_viewer_epoch,
            capture_order,
            source_ready_at_micros,
        ) {
            // 표시 실패는 고객 화면에 노출되지 않는다. 저장된 RAW와 기존 자산은 그대로다.
            log::warn!(
                "display_proxy_publish_failed session={session_id} request={request_id} capture={capture_id} error={}",
                error.message
            );
        }

        // 마지막 generation도 반드시 닫힌다. 다음 촬영이 없어도 계측이 완결되도록,
        // 유예 시간이 지난 뒤 이 lane 스레드가 직접 sweep한다.
        thread::sleep(Duration::from_micros(
            PRESENT_REPORT_GRACE_MICROS.saturating_add(250_000),
        ));
        close_out_open_generations(&lane_app, CloseOutScope::Due(current_monotonic_micros()));
    });
}

#[allow(clippy::too_many_arguments)]
fn run_display_proxy_publication(
    app: &tauri::AppHandle,
    session_id: &str,
    request_id: &str,
    capture_id: &str,
    preset_id: Option<&str>,
    preset_version: &str,
    source_path: &std::path::Path,
    request_viewer_epoch: u64,
    capture_order: u64,
    source_ready_at_micros: u64,
) -> Result<(), HostErrorEnvelope> {
    let base_dir = session_base_dir(app)?;
    let viewer = read_viewer_context(app);
    let lanes = current_display_lane_flags();

    if viewer.session_id.as_deref() != Some(session_id)
        || viewer.viewer_epoch != request_viewer_epoch
    {
        log_proxy_skip(
            session_id,
            capture_id,
            ProxySkipReason::ViewerContextChanged,
            "viewer session or epoch changed before render",
        );
        return Ok(());
    }

    let Some(preset_id) = preset_id else {
        log_proxy_skip(session_id, capture_id, ProxySkipReason::PresetUnbound, "");
        return Ok(());
    };

    // capture record에 고정된 version으로만 번들을 찾는다. live catalog pointer가 아니다.
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);
    let bundle = find_published_preset_runtime_bundle(&catalog_root, preset_id, preset_version);

    // HV-14 route 결정 전 기본값. 승인되면 이 값 하나만 바뀐다.
    let route = ProxySourceRoute::RawOriginal;
    let publication = match evaluate_proxy_eligibility(
        current_proxy_lane_mode(),
        lanes.measurement_lane_enabled,
        bundle.as_ref(),
        route,
        crate::render::PINNED_DARKTABLE_VERSION,
        viewer.required_source_width_px,
        viewer.required_source_height_px,
    ) {
        Ok(publication) => publication,
        Err(reason) => {
            log_proxy_skip(session_id, capture_id, reason, "");
            return Ok(());
        }
    };
    // `evaluate_proxy_eligibility`가 통과했다면 번들은 반드시 존재한다.
    let Some(bundle) = bundle else {
        return Ok(());
    };

    let job = DisplayProxyJob {
        session_id,
        request_id,
        capture_id,
        source_path,
        route,
        bound_session_id: viewer.session_id.as_deref(),
        request_viewer_epoch,
        current_viewer_epoch: viewer.viewer_epoch,
        capture_order,
        required_source_width_px: viewer.required_source_width_px,
        required_source_height_px: viewer.required_source_height_px,
        display_profile_id: viewer.display_profile_id.as_deref().unwrap_or("unknown"),
        device_pixel_ratio: viewer.device_pixel_ratio.unwrap_or(1.0),
        lanes,
        deadline_micros: display_lane_deadline_micros(request_id, source_ready_at_micros),
    };

    let rendered = match render_display_proxy(
        &base_dir,
        &job,
        &bundle,
        &publication,
        &current_monotonic_micros,
    ) {
        Ok(rendered) => rendered,
        Err(ProxyRenderError::Skipped { reason, detail }) => {
            log_proxy_skip(session_id, capture_id, reason, &detail);
            return Ok(());
        }
        Err(ProxyRenderError::Render(error)) => {
            // 렌더 실패는 RAW·preview·final truth를 건드리지 않는다.
            log::warn!(
                "display_proxy_render_failed session={session_id} capture={capture_id} reason={} detail={}",
                error.reason_code,
                error.operator_detail
            );
            return Ok(());
        }
    };

    let fresh_viewer = read_viewer_context(app);
    let viewer_context_unchanged = fresh_viewer.session_id == viewer.session_id
        && fresh_viewer.viewer_epoch == viewer.viewer_epoch
        && fresh_viewer.required_source_width_px == viewer.required_source_width_px
        && fresh_viewer.required_source_height_px == viewer.required_source_height_px
        && fresh_viewer.display_profile_id == viewer.display_profile_id
        && fresh_viewer.device_pixel_ratio == viewer.device_pixel_ratio;

    if !viewer_context_unchanged {
        let _ = std::fs::remove_file(&rendered.staging_path);
        log_proxy_skip(
            session_id,
            capture_id,
            ProxySkipReason::ViewerContextChanged,
            "viewer context changed during render",
        );
        return Ok(());
    }

    let outcome = {
        let handle = app.state::<DisplayStateHandle>();
        let mut state = handle.0.lock().expect("display state lock poisoned");

        if state.session_id().is_some() && state.session_id() != Some(session_id) {
            let _ = std::fs::remove_file(&rendered.staging_path);
            log_proxy_skip(
                session_id,
                capture_id,
                ProxySkipReason::ViewerContextChanged,
                "display state is already bound to another session",
            );
            return Ok(());
        }

        publish_rendered_proxy_in_dir(
            &base_dir,
            &mut state,
            &job,
            &rendered,
            // 렌더 시작 시점이 아니라 **게시 시점에 읽은** epoch다.
            fresh_viewer.viewer_epoch,
            &current_monotonic_micros,
        )?
    };

    if let Some(reason) = outcome.reject_reason.as_deref() {
        log::warn!(
            "display_proxy_rejected session={session_id} request={request_id} capture={capture_id} reason={reason}"
        );
        return Ok(());
    }

    let Some(generation) = outcome.committed.as_ref() else {
        return Ok(());
    };

    crate::commands::viewer_commands::reveal_prewarmed_viewer_window(app);

    // pointer commit이 끝난 뒤에야 notify한다.
    emit_display_update(app, outcome.pointer.clone());
    let event_emitted_at_micros = current_monotonic_micros();

    register_committed_generation(
        generation,
        &outcome,
        CommittedGenerationSpans {
            write_start_at_micros: Some(rendered.render.render_started_at_micros),
            event_emitted_at_micros,
            proxy_source_ready_at_micros: Some(source_ready_at_micros),
            proxy_queue_wait_micros: Some(rendered.render.queue_wait_micros),
            proxy_render_start_at_micros: Some(rendered.render.render_started_at_micros),
            proxy_process_exited_at_micros: Some(rendered.render.process_exited_at_micros),
            scheduler_priority: Some(rendered.render.scheduler.priority.to_string()),
            scheduler_deadline_micros: Some(rendered.render.scheduler.deadline_micros),
            scheduler_deadline_missed: Some(rendered.render.scheduler.deadline_missed),
            scheduler_coalesced_count: Some(rendered.render.scheduler.coalesced_count),
            ..CommittedGenerationSpans::default()
        },
    );

    close_out_open_generations(app, CloseOutScope::Due(current_monotonic_micros()));

    // Story 7.6. 이 촬영이 화면에 올라갔다. 더 오래된 촬영의 정밀본은 이제
    // `older-capture`로 어차피 거부되므로 계속 돌릴 이유가 없다.
    cancel_superseded_refined_renders(app, session_id, generation.capture_order);

    // Story 7.6. **proxy가 commit된 직후에만** 정밀본을 시작한다.
    // 크기·preset·source 해시를 전부 이 generation에서 상속하므로, 여기가 유일한 진입점이다.
    match inherit_proxy_geometry(generation) {
        Some(_) if current_monotonic_micros() > job.deadline_micros => log_raw_refined_skip(
            session_id,
            capture_id,
            RawRefinedSkipReason::DeadlineMissed,
            "proxy committed after the display-lane deadline",
        ),
        Some(inherited) => spawn_raw_refined_publication(
            app,
            session_id.to_string(),
            request_id.to_string(),
            capture_id.to_string(),
            bundle.preset_id.clone(),
            bundle.published_version.clone(),
            source_path.to_path_buf(),
            request_viewer_epoch,
            capture_order,
            inherited,
            job.deadline_micros,
        ),
        None => log_raw_refined_skip(
            session_id,
            capture_id,
            RawRefinedSkipReason::ProxyNotCommitted,
            "committed proxy generation carries no provenance",
        ),
    }

    log::info!(
        "display_proxy_committed session={} request={} capture={} generation={} preset={}@{} size={}x{} queueWaitMicros={} detail={}",
        generation.session_id,
        generation.request_id,
        capture_id,
        generation.generation_id,
        bundle.preset_id,
        bundle.published_version,
        generation.source_width_px,
        generation.source_height_px,
        rendered.render.queue_wait_micros,
        rendered.render.detail
    );

    Ok(())
}

/// Story 7.6. 렌더 스케줄러에 취소 신호를 보낸다.
///
/// **어느 범위가 어느 우선순위를 건드리는지는 `CancelScope`가 소유한다.** 여기서 다시 판정하면
/// 규칙이 두 벌이 되고, 그중 하나는 언젠가 final 렌더를 취소하게 된다.
///
/// 취소는 **로그와 span에만 남는다.** RAW·preview·final·현재 화면 truth를 건드리지 않으며
/// 고객 화면에 아무것도 나타나지 않는다.
fn cancel_display_lane_renders(app: &tauri::AppHandle, scope: CancelScope) {
    let Ok(base_dir) = session_base_dir(app) else {
        log::warn!("render_cancel_skipped reason=session-base-dir-unavailable");
        return;
    };

    let summary = scheduler_for(&base_dir).cancel(&scope);

    if summary.cancelled_waiting > 0 || summary.cancelled_running > 0 {
        log::info!(
            "render_cancel_requested scope={:?} waiting={} running={}",
            scope,
            summary.cancelled_waiting,
            summary.cancelled_running
        );
    }
}

/// Story 7.6. viewer window가 재생성됐다. **그 세대에 묶인 display lane 작업만** 취소한다.
///
/// P2는 관람 창 세대와 무관한 산출물(final, 384px 레일)을 만들므로 건드리지 않는다.
pub fn cancel_stale_epoch_display_renders(app: &tauri::AppHandle, current_epoch: u64) {
    let Some(session_id) = read_viewer_context(app).session_id else {
        return;
    };

    cancel_display_lane_renders(
        app,
        CancelScope::StaleViewerEpoch {
            session_id,
            current_epoch,
        },
    );
}

/// Story 7.6. 더 새 촬영의 generation이 pointer에 commit됐다. **이전 촬영의 P1만** 취소한다.
///
/// 그 뒤에는 `older-capture` guard로 어차피 거부되므로 계속 돌리는 것은 순수 낭비다.
/// **P0는 취소하지 않는다** — 이 사진도 고객이 실제로 찍은 사진이다.
fn cancel_superseded_refined_renders(
    app: &tauri::AppHandle,
    session_id: &str,
    capture_order: Option<u64>,
) {
    let Some(capture_order) = capture_order else {
        return;
    };

    cancel_display_lane_renders(
        app,
        CancelScope::SupersededRefined {
            session_id: session_id.to_string(),
            capture_order,
        },
    );
}

/// Story 7.6. 이 촬영의 display lane deadline.
///
/// **trusted capture input 시각 + NFR-003 hard max(5초)**다. 공식 KPI 시작점과 같은 값을 쓰므로
/// deadline 관측이 KPI와 같은 시계 위에 선다. trusted input 보고가 아직 도착하지 않았다면
/// (booth가 계측 IPC를 하지 않는 구성) 렌더 요청 시각을 대신 쓴다 — 그 경우 deadline은
/// 순서 결정에만 쓰이고 `deadline-missed` 관측값은 낙관적으로 잡힌다.
///
/// **이 값은 어떤 작업도 취소하지 않는다.** 순서 결정과 관측이 전부다.
fn display_lane_deadline_micros(request_id: &str, fallback_at_micros: u64) -> u64 {
    let started_at_micros = telemetry_state()
        .lock()
        .expect("telemetry state lock poisoned")
        .requests
        .get(request_id)
        .and_then(|pending| pending.trusted_input_at_micros)
        .unwrap_or(fallback_at_micros);

    started_at_micros.saturating_add(crate::render::DISPLAY_LANE_DEADLINE_BUDGET_MICROS)
}

/// proxy lane이 시작되지 않은 이유를 남긴다. **조용한 무시를 만들지 않는다.**
fn log_proxy_skip(session_id: &str, capture_id: &str, reason: ProxySkipReason, detail: &str) {
    log::info!(
        "display_proxy_skipped session={session_id} capture={capture_id} reason={} detail={}",
        reason.as_str(),
        if detail.is_empty() { "none" } else { detail }
    );
}

/// 정밀본 lane이 시작되지 않은 이유를 남긴다. **조용한 무시를 만들지 않는다.**
fn log_raw_refined_skip(
    session_id: &str,
    capture_id: &str,
    reason: RawRefinedSkipReason,
    detail: &str,
) {
    log::info!(
        "display_raw_refined_skipped session={session_id} capture={capture_id} reason={} detail={}",
        reason.as_str(),
        if detail.is_empty() { "none" } else { detail }
    );
}

/// Story 7.6. commit된 proxy generation에서 정밀본이 상속할 값을 뽑는다.
///
/// **렌더 시점에 viewer를 다시 읽지 않는다.** 그 사이 viewer 문맥이 바뀌었다면 정밀본을
/// 버리고 화면은 proxy 그대로 남긴다 — 새로 읽은 크기로 렌더하면 교체 순간 사진이 튄다.
fn inherit_proxy_geometry(
    generation: &crate::contracts::dto::DisplayGenerationDto,
) -> Option<InheritedProxyGeometry> {
    let provenance = generation.proxy_provenance.as_ref()?;

    Some(InheritedProxyGeometry {
        proxy_generation_id: generation.generation_id.clone(),
        target_width_px: provenance.target_width_px,
        target_height_px: provenance.target_height_px,
        display_profile_id: provenance.display_profile_id.clone(),
        device_pixel_ratio: provenance.device_pixel_ratio,
        // **실측 크기다.** 정밀본은 이 값과 정확히 같아야 승급한다.
        proxy_source_width_px: generation.source_width_px,
        proxy_source_height_px: generation.source_height_px,
        preset_id: provenance.preset_id.clone(),
        preset_version: provenance.preset_version.clone(),
        source_asset_hash: provenance.source_asset_hash.clone(),
        source_route: provenance.source_route.clone(),
    })
}

/// Story 7.6. proxy generation이 **commit된 직후** 정밀본을 만든다.
///
/// **proxy가 없으면 정밀본도 없다.** tier 하락 없이 승급만 존재해야 하고,
/// proxy 없이 정밀본만 뜨면 고객의 첫 화면이 3배 느려진다 (UX-DR19).
#[allow(clippy::too_many_arguments)]
fn spawn_raw_refined_publication(
    app: &tauri::AppHandle,
    session_id: String,
    request_id: String,
    capture_id: String,
    preset_id: String,
    preset_version: String,
    source_path: PathBuf,
    request_viewer_epoch: u64,
    capture_order: u64,
    inherited: InheritedProxyGeometry,
    deadline_micros: u64,
) {
    let lane_app = app.clone();

    thread::spawn(move || {
        if let Err(error) = run_raw_refined_publication(
            &lane_app,
            &session_id,
            &request_id,
            &capture_id,
            &preset_id,
            &preset_version,
            &source_path,
            request_viewer_epoch,
            capture_order,
            &inherited,
            deadline_micros,
        ) {
            // 정밀본 실패는 관람 화면에 노출되지 않는다. 현재 proxy가 그대로 남는다.
            log::warn!(
                "display_raw_refined_publish_failed session={session_id} request={request_id} capture={capture_id} error={}",
                error.message
            );
        }

        // 정밀본 generation도 반드시 terminal 행을 갖는다. 유예 시간이 지난 뒤 sweep한다.
        thread::sleep(Duration::from_micros(
            PRESENT_REPORT_GRACE_MICROS.saturating_add(250_000),
        ));
        close_out_open_generations(&lane_app, CloseOutScope::Due(current_monotonic_micros()));
    });
}

#[allow(clippy::too_many_arguments)]
fn run_raw_refined_publication(
    app: &tauri::AppHandle,
    session_id: &str,
    request_id: &str,
    capture_id: &str,
    preset_id: &str,
    preset_version: &str,
    source_path: &std::path::Path,
    request_viewer_epoch: u64,
    capture_order: u64,
    inherited: &InheritedProxyGeometry,
    deadline_micros: u64,
) -> Result<(), HostErrorEnvelope> {
    let base_dir = session_base_dir(app)?;
    let viewer = read_viewer_context(app);
    let lanes = current_display_lane_flags();

    if viewer.session_id.as_deref() != Some(session_id)
        || viewer.viewer_epoch != request_viewer_epoch
    {
        log_raw_refined_skip(
            session_id,
            capture_id,
            RawRefinedSkipReason::ViewerContextChanged,
            "viewer session or epoch changed before render",
        );
        return Ok(());
    }

    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);
    let bundle = find_published_preset_runtime_bundle(&catalog_root, preset_id, preset_version);

    let publication = match evaluate_raw_refined_eligibility(
        current_raw_refined_lane_mode(),
        RAW_REFINED_TIER_JUSTIFICATION,
        // 이 경로는 proxy commit 직후에만 도달한다.
        true,
        bundle.as_ref(),
        crate::render::PINNED_DARKTABLE_VERSION,
    ) {
        Ok(publication) => publication,
        Err(reason) => {
            log_raw_refined_skip(session_id, capture_id, reason, "");
            return Ok(());
        }
    };
    let Some(bundle) = bundle else {
        return Ok(());
    };

    let job = RawRefinedJob {
        session_id,
        request_id,
        capture_id,
        source_path,
        bound_session_id: viewer.session_id.as_deref(),
        request_viewer_epoch,
        current_viewer_epoch: viewer.viewer_epoch,
        capture_order,
        inherited: inherited.clone(),
        lanes,
        deadline_micros,
    };

    let rendered = match render_raw_refined_display(
        &base_dir,
        &job,
        &bundle,
        &publication,
        &current_monotonic_micros,
    ) {
        Ok(rendered) => rendered,
        Err(RawRefinedRenderError::Skipped { reason, detail }) => {
            log_raw_refined_skip(session_id, capture_id, reason, &detail);
            return Ok(());
        }
        Err(RawRefinedRenderError::Render(error)) => {
            // 렌더 실패는 RAW·preview·final·현재 화면 truth를 건드리지 않는다.
            log::warn!(
                "display_raw_refined_render_failed session={session_id} capture={capture_id} reason={} detail={}",
                error.reason_code,
                error.operator_detail
            );
            return Ok(());
        }
    };

    // 게시 **직전에** viewer 문맥을 다시 확인한다. 렌더 시작 시점의 판정을 상속하지 않는다.
    let fresh_viewer = read_viewer_context(app);
    let viewer_context_unchanged = fresh_viewer.session_id == viewer.session_id
        && fresh_viewer.viewer_epoch == viewer.viewer_epoch
        && fresh_viewer.required_source_width_px == viewer.required_source_width_px
        && fresh_viewer.required_source_height_px == viewer.required_source_height_px
        && fresh_viewer.display_profile_id == viewer.display_profile_id
        && fresh_viewer.device_pixel_ratio == viewer.device_pixel_ratio;

    if !viewer_context_unchanged {
        let _ = std::fs::remove_file(&rendered.staging_path);
        log_raw_refined_skip(
            session_id,
            capture_id,
            RawRefinedSkipReason::ViewerContextChanged,
            "viewer context changed during render",
        );
        return Ok(());
    }

    let outcome = {
        let handle = app.state::<DisplayStateHandle>();
        let mut state = handle.0.lock().expect("display state lock poisoned");

        if state.session_id().is_some() && state.session_id() != Some(session_id) {
            let _ = std::fs::remove_file(&rendered.staging_path);
            log_raw_refined_skip(
                session_id,
                capture_id,
                RawRefinedSkipReason::ViewerContextChanged,
                "display state is already bound to another session",
            );
            return Ok(());
        }

        publish_rendered_raw_refined_in_dir(
            &base_dir,
            &mut state,
            &job,
            &rendered,
            RAW_REFINED_TIER_JUSTIFICATION,
            // 렌더 시작 시점이 아니라 **게시 시점에 읽은** epoch다.
            fresh_viewer.viewer_epoch,
            &current_monotonic_micros,
        )?
    };

    if let Some(reason) = outcome.reject_reason.as_deref() {
        log::warn!(
            "display_raw_refined_rejected session={session_id} request={request_id} capture={capture_id} reason={reason}"
        );
        return Ok(());
    }

    let Some(generation) = outcome.committed.as_ref() else {
        return Ok(());
    };

    // pointer commit이 끝난 뒤에야 notify한다. **확정 파일을 in-place overwrite하지 않는다.**
    emit_display_update(app, outcome.pointer.clone());
    let event_emitted_at_micros = current_monotonic_micros();

    register_committed_generation(
        generation,
        &outcome,
        CommittedGenerationSpans {
            write_start_at_micros: Some(rendered.render.render_started_at_micros),
            event_emitted_at_micros,
            raw_refined_enqueued_at_micros: Some(rendered.render.scheduler.enqueued_at_micros),
            raw_refined_queue_wait_micros: Some(rendered.render.scheduler.queue_wait_micros),
            raw_refined_render_start_at_micros: Some(rendered.render.render_started_at_micros),
            raw_refined_process_exited_at_micros: Some(rendered.render.process_exited_at_micros),
            raw_refined_committed_at_micros: outcome.pointer_committed_at_micros,
            scheduler_priority: Some(rendered.render.scheduler.priority.to_string()),
            scheduler_deadline_micros: Some(rendered.render.scheduler.deadline_micros),
            scheduler_deadline_missed: Some(rendered.render.scheduler.deadline_missed),
            scheduler_coalesced_count: Some(rendered.render.scheduler.coalesced_count),
            ..CommittedGenerationSpans::default()
        },
    );

    close_out_open_generations(app, CloseOutScope::Due(current_monotonic_micros()));

    log::info!(
        "display_raw_refined_committed session={} request={} capture={} generation={} promotedFrom={} preset={}@{} size={}x{} queueWaitMicros={} detail={}",
        generation.session_id,
        generation.request_id,
        capture_id,
        generation.generation_id,
        inherited.proxy_generation_id,
        bundle.preset_id,
        bundle.published_version,
        generation.source_width_px,
        generation.source_height_px,
        rendered.render.scheduler.queue_wait_micros,
        rendered.render.detail
    );

    Ok(())
}

/// 계측 lane 모드를 booth 로그에 남긴다. 켜져 있으면 제품 표시가 아님을 분명히 한다.
pub fn log_sample_lane_mode() {
    let mode = current_sample_lane_mode();

    if mode.is_enabled() {
        log::warn!(
            "display_sample_lane_enabled mode={} — 표시되는 이미지는 계측용 fixture이며 제품 결과가 아니다",
            mode.as_str()
        );
    }
}

#[tauri::command]
pub fn get_viewer_display_state(
    app: tauri::AppHandle,
) -> Result<DisplayPointerSnapshotDto, HostErrorEnvelope> {
    read_display_snapshot(&app)
}

/// `async` 필수. 파일 write/fsync/rename을 event loop 스레드에서 돌리면 창이 멈춘다
/// (HV-13A No-Go의 실제 원인과 같은 종류의 실수다).
#[tauri::command(async)]
pub fn publish_display_sample(
    app: tauri::AppHandle,
    input: DisplaySamplePublishInputDto,
) -> Result<DisplayPointerSnapshotDto, HostErrorEnvelope> {
    validate_display_sample_publish_input(&input)?;

    if !current_sample_lane_mode().is_enabled() {
        return Err(HostErrorEnvelope::validation_message(
            "표시 샘플 계측 모드가 꺼져 있어요.",
        ));
    }

    publish_sample_generation(&app, &input, None)
}

/// clock 보정 왕복. 수신 즉시 host monotonic clock을 찍어 돌려준다.
#[tauri::command]
pub fn stamp_clock_probe() -> Result<ClockProbeResultDto, HostErrorEnvelope> {
    Ok(ClockProbeResultDto {
        host_monotonic_micros: current_monotonic_micros(),
        host_epoch_micros: current_epoch_micros(),
    })
}

/// booth → host. 공식 KPI 시작점을 기록한다.
#[tauri::command]
pub fn report_trusted_capture_input(
    app: tauri::AppHandle,
    input: TrustedInputReportDto,
) -> Result<(), HostErrorEnvelope> {
    validate_trusted_input_report(&input)?;

    let host_input_micros = to_host_micros(input.client_input_micros, input.clock_offset_micros);
    let records = {
        let mut telemetry = telemetry_state()
            .lock()
            .expect("telemetry state lock poisoned");

        let pending = telemetry
            .requests
            .entry(input.request_id.clone())
            .or_insert_with(|| PendingRequest {
                session_id: input.session_id.clone(),
                ..PendingRequest::default()
            });

        pending.session_id = input.session_id.clone();
        pending.trusted_input_at_micros = Some(host_input_micros);
        pending.input_uncertainty_micros = input.clock_uncertainty_micros;
        pending.is_trusted_input = input.is_trusted;
        pending
            .host_accepted_at_micros
            .get_or_insert_with(current_monotonic_micros);
        std::mem::take(&mut pending.pending_present_reports)
            .into_values()
            .map(|report| {
                let window_events = window_events_for_report(pending, &report);
                build_present_record(&input.request_id, pending, &report, window_events)
            })
            .collect::<Vec<_>>()
    };

    for record in records {
        persist_present_record(&app, &record)?;
    }

    Ok(())
}

const DISPLAY_REJECT_VIEWER_WINDOW_EVENT_AFTER_INPUT: &str = "viewer-window-event-after-input";

fn should_defer_present_report(pending: &PendingRequest, input: &DisplayPresentReportDto) -> bool {
    input.outcome == "presented" && pending.trusted_input_at_micros.is_none()
}

fn window_events_for_report(pending: &PendingRequest, input: &DisplayPresentReportDto) -> u32 {
    let Some(input_at_micros) = pending.trusted_input_at_micros else {
        return 0;
    };
    let Some(present_at_micros) = input
        .spans
        .actual_present_at_micros
        .map(|value| to_host_micros(value, input.clock_offset_micros))
    else {
        return 0;
    };

    viewer_window_event_count_between(
        input_at_micros.saturating_sub(pending.input_uncertainty_micros),
        present_at_micros.saturating_add(input.clock_uncertainty_micros),
    )
}

fn build_present_record(
    request_id: &str,
    pending: &mut PendingRequest,
    input: &DisplayPresentReportDto,
    viewer_window_events_after_input: u32,
) -> PresentSampleRecord {
    let recorded_spans = {
        let spans = pending
            .generations
            .entry(input.generation_id.clone())
            .or_default();
        // 이 generation의 terminal 행은 여기서 확정된다. 이후 close-out은 행을 더 만들지 않는다.
        spans.terminal_recorded = true;
        spans.clone()
    };
    let sample_variant = recorded_spans.sample_variant.clone();
    let sample_write_start_at_micros = recorded_spans.sample_write_start_at_micros;
    let file_ready_at_micros = recorded_spans.file_ready_at_micros;
    let probe_ok_at_micros = recorded_spans.probe_ok_at_micros;
    let pointer_committed_at_micros = recorded_spans.pointer_committed_at_micros;
    let event_emitted_at_micros = recorded_spans.event_emitted_at_micros;

    let total_uncertainty_micros = pending
        .input_uncertainty_micros
        .saturating_add(input.clock_uncertainty_micros);
    let actual_present_at_micros = input
        .spans
        .actual_present_at_micros
        .map(|value| to_host_micros(value, input.clock_offset_micros));
    let ac4_failed =
        pending.trusted_input_at_micros.is_some() && viewer_window_events_after_input > 0;
    let outcome = if ac4_failed {
        "rejected".to_string()
    } else {
        input.outcome.clone()
    };
    let reject_reason = if ac4_failed {
        Some(DISPLAY_REJECT_VIEWER_WINDOW_EVENT_AFTER_INPUT.to_string())
    } else {
        input.reject_reason.clone()
    };
    // **공식 KPI는 `trusted input → 첫 자격 frame actual present` 하나다.**
    //
    // Story 7.6의 정밀본 present는 **두 번째 terminal 행**이지 KPI 종료점이 아니다.
    // 여기에 qualifying latency를 채우면 evidence 도구가 첫 화면 지연과 승급 지연을
    // 같은 분포에 넣게 되고, NFR-003 판정이 조용히 거짓이 된다.
    // 정밀본의 종료점은 `raw_refined_presented_at_micros`에 그대로 남는다 — 5초를 넘어도.
    let is_kpi_endpoint = !recorded_spans.provenance.is_raw_refined();
    let qualifying_latency_micros = match (
        pending.trusted_input_at_micros,
        actual_present_at_micros,
        outcome.as_str(),
        pending.is_trusted_input,
        is_kpi_endpoint,
    ) {
        (Some(input_at), Some(present_at), "presented", true, true) => {
            Some(conservative_latency_micros(
                input_at,
                present_at,
                pending.input_uncertainty_micros,
                input.clock_uncertainty_micros,
            ))
        }
        _ => None,
    };
    let convert =
        |value: Option<u64>| value.map(|raw| to_host_micros(raw, input.clock_offset_micros));

    PresentSampleRecord {
        schema_version: PRESENT_TELEMETRY_SCHEMA_VERSION.into(),
        session_id: pending.session_id.clone(),
        request_id: request_id.to_string(),
        generation_id: Some(input.generation_id.clone()),
        sample_variant,
        lane_mode: current_lane_mode_label(),
        outcome,
        reject_reason,
        tier: recorded_spans.provenance.tier.clone(),
        preset_id: recorded_spans.provenance.preset_id.clone(),
        preset_version: recorded_spans.provenance.preset_version.clone(),
        source_route: recorded_spans.provenance.source_route.clone(),
        target_width_px: recorded_spans.provenance.target_width_px,
        target_height_px: recorded_spans.provenance.target_height_px,
        trusted_input_at_micros: pending.trusted_input_at_micros,
        actual_present_at_micros,
        qualifying_latency_micros,
        total_uncertainty_micros,
        confidence: classify_confidence(total_uncertainty_micros).into(),
        is_trusted_input: pending.is_trusted_input,
        host_accepted_at_micros: pending.host_accepted_at_micros,
        sample_write_start_at_micros,
        file_ready_at_micros,
        probe_ok_at_micros,
        pointer_committed_at_micros,
        event_emitted_at_micros,
        viewer_receipt_at_micros: convert(input.spans.viewer_receipt_at_micros),
        decode_start_at_micros: convert(input.spans.decode_start_at_micros),
        decode_end_at_micros: convert(input.spans.decode_end_at_micros),
        swap_committed_at_micros: convert(input.spans.swap_committed_at_micros),
        img_on_load_at_micros: convert(input.spans.img_on_load_at_micros),
        element_timing_render_at_micros: convert(input.spans.element_timing_render_at_micros),
        is_element_render_time: input.spans.is_element_render_time,
        proxy_source_ready_at_micros: recorded_spans.proxy_source_ready_at_micros,
        proxy_queue_wait_micros: recorded_spans.proxy_queue_wait_micros,
        proxy_render_start_at_micros: recorded_spans.proxy_render_start_at_micros,
        proxy_process_exited_at_micros: recorded_spans.proxy_process_exited_at_micros,
        render_quality: recorded_spans.provenance.render_quality.clone(),
        raw_refined_enqueued_at_micros: recorded_spans.raw_refined_enqueued_at_micros,
        raw_refined_queue_wait_micros: recorded_spans.raw_refined_queue_wait_micros,
        raw_refined_render_start_at_micros: recorded_spans.raw_refined_render_start_at_micros,
        raw_refined_process_exited_at_micros: recorded_spans.raw_refined_process_exited_at_micros,
        raw_refined_committed_at_micros: recorded_spans.raw_refined_committed_at_micros,
        // **정밀본 present는 두 번째 terminal 행이지 KPI 종료점이 아니다.**
        // 5초를 넘어도 그대로 기록한다. 공식 KPI는 첫 자격 frame 하나뿐이다.
        raw_refined_presented_at_micros: recorded_spans
            .provenance
            .is_raw_refined()
            .then_some(actual_present_at_micros)
            .flatten(),
        scheduler_priority: recorded_spans.scheduler_priority.clone(),
        scheduler_deadline_micros: recorded_spans.scheduler_deadline_micros,
        scheduler_deadline_missed: recorded_spans.scheduler_deadline_missed,
        scheduler_coalesced_count: recorded_spans.scheduler_coalesced_count,
        scheduler_preempted_by: recorded_spans.scheduler_preempted_by.clone(),
        viewer_window_events_after_input,
    }
}

fn build_unknown_generation_record(
    context: &GenerationContext,
    input: &DisplayPresentReportDto,
) -> PresentSampleRecord {
    let convert =
        |value: Option<u64>| value.map(|raw| to_host_micros(raw, input.clock_offset_micros));

    PresentSampleRecord {
        schema_version: PRESENT_TELEMETRY_SCHEMA_VERSION.into(),
        session_id: context.session_id.clone(),
        request_id: context.request_id.clone(),
        generation_id: Some(input.generation_id.clone()),
        sample_variant: context.sample_variant.clone(),
        lane_mode: current_lane_mode_label(),
        outcome: "rejected".into(),
        reject_reason: Some(DISPLAY_REJECT_UNKNOWN_GENERATION.into()),
        tier: context.provenance.tier.clone(),
        preset_id: context.provenance.preset_id.clone(),
        preset_version: context.provenance.preset_version.clone(),
        source_route: context.provenance.source_route.clone(),
        target_width_px: context.provenance.target_width_px,
        target_height_px: context.provenance.target_height_px,
        trusted_input_at_micros: None,
        actual_present_at_micros: convert(input.spans.actual_present_at_micros),
        qualifying_latency_micros: None,
        total_uncertainty_micros: input.clock_uncertainty_micros,
        confidence: classify_confidence(input.clock_uncertainty_micros).into(),
        is_trusted_input: false,
        host_accepted_at_micros: None,
        sample_write_start_at_micros: None,
        file_ready_at_micros: None,
        probe_ok_at_micros: None,
        pointer_committed_at_micros: None,
        event_emitted_at_micros: None,
        viewer_receipt_at_micros: convert(input.spans.viewer_receipt_at_micros),
        decode_start_at_micros: convert(input.spans.decode_start_at_micros),
        decode_end_at_micros: convert(input.spans.decode_end_at_micros),
        swap_committed_at_micros: convert(input.spans.swap_committed_at_micros),
        img_on_load_at_micros: convert(input.spans.img_on_load_at_micros),
        element_timing_render_at_micros: convert(input.spans.element_timing_render_at_micros),
        is_element_render_time: input.spans.is_element_render_time,
        proxy_source_ready_at_micros: None,
        proxy_queue_wait_micros: None,
        proxy_render_start_at_micros: None,
        proxy_process_exited_at_micros: None,
        render_quality: context.provenance.render_quality.clone(),
        raw_refined_enqueued_at_micros: None,
        raw_refined_queue_wait_micros: None,
        raw_refined_render_start_at_micros: None,
        raw_refined_process_exited_at_micros: None,
        raw_refined_committed_at_micros: None,
        raw_refined_presented_at_micros: None,
        scheduler_priority: None,
        scheduler_deadline_micros: None,
        scheduler_deadline_missed: None,
        scheduler_coalesced_count: None,
        scheduler_preempted_by: None,
        viewer_window_events_after_input: 0,
    }
}

/// commit·notify까지 끝났는데 terminal 보고가 오지 않은 generation의 행.
///
/// **행을 만들지 않는 선택지는 없다.** 아무 행도 남기지 않으면 evidence에서
/// "표시되지 않았다"와 "보고가 유실됐다"를 구분할 수 없고, 분모가 조용히 줄어
/// 성공률이 실제보다 좋아 보인다.
fn build_unreported_record(
    open: &OpenGeneration,
    pending: &PendingRequest,
    spans: &GenerationSpans,
    closed_at_micros: u64,
) -> PresentSampleRecord {
    // 보고가 오지 않은 이유가 창 이벤트일 수 있다. 구간을 그대로 세어 남긴다.
    let viewer_window_events_after_input = pending
        .trusted_input_at_micros
        .map(|input_at_micros| {
            viewer_window_event_count_between(
                input_at_micros.saturating_sub(pending.input_uncertainty_micros),
                closed_at_micros,
            )
        })
        .unwrap_or(0);

    PresentSampleRecord {
        schema_version: PRESENT_TELEMETRY_SCHEMA_VERSION.into(),
        session_id: open.session_id.clone(),
        request_id: open.request_id.clone(),
        generation_id: Some(open.generation_id.clone()),
        sample_variant: spans.sample_variant.clone(),
        lane_mode: current_lane_mode_label(),
        outcome: "rejected".into(),
        reject_reason: Some(DISPLAY_REJECT_PRESENT_UNREPORTED.into()),
        tier: spans.provenance.tier.clone(),
        preset_id: spans.provenance.preset_id.clone(),
        preset_version: spans.provenance.preset_version.clone(),
        source_route: spans.provenance.source_route.clone(),
        target_width_px: spans.provenance.target_width_px,
        target_height_px: spans.provenance.target_height_px,
        trusted_input_at_micros: pending.trusted_input_at_micros,
        // 종료점을 관측하지 못했다. 추정값을 넣으면 KPI가 거짓이 된다.
        actual_present_at_micros: None,
        qualifying_latency_micros: None,
        total_uncertainty_micros: pending.input_uncertainty_micros,
        confidence: PRESENT_CONFIDENCE_UNREPORTED.into(),
        is_trusted_input: pending.is_trusted_input,
        host_accepted_at_micros: pending.host_accepted_at_micros,
        sample_write_start_at_micros: spans.sample_write_start_at_micros,
        file_ready_at_micros: spans.file_ready_at_micros,
        probe_ok_at_micros: spans.probe_ok_at_micros,
        pointer_committed_at_micros: spans.pointer_committed_at_micros,
        event_emitted_at_micros: spans.event_emitted_at_micros,
        viewer_receipt_at_micros: None,
        decode_start_at_micros: None,
        decode_end_at_micros: None,
        swap_committed_at_micros: None,
        img_on_load_at_micros: None,
        element_timing_render_at_micros: None,
        is_element_render_time: false,
        proxy_source_ready_at_micros: spans.proxy_source_ready_at_micros,
        proxy_queue_wait_micros: spans.proxy_queue_wait_micros,
        proxy_render_start_at_micros: spans.proxy_render_start_at_micros,
        proxy_process_exited_at_micros: spans.proxy_process_exited_at_micros,
        render_quality: spans.provenance.render_quality.clone(),
        raw_refined_enqueued_at_micros: spans.raw_refined_enqueued_at_micros,
        raw_refined_queue_wait_micros: spans.raw_refined_queue_wait_micros,
        raw_refined_render_start_at_micros: spans.raw_refined_render_start_at_micros,
        raw_refined_process_exited_at_micros: spans.raw_refined_process_exited_at_micros,
        raw_refined_committed_at_micros: spans.raw_refined_committed_at_micros,
        // 종료점을 관측하지 못했다. 정밀본이라도 추정값을 넣지 않는다.
        raw_refined_presented_at_micros: None,
        scheduler_priority: spans.scheduler_priority.clone(),
        scheduler_deadline_micros: spans.scheduler_deadline_micros,
        scheduler_deadline_missed: spans.scheduler_deadline_missed,
        scheduler_coalesced_count: spans.scheduler_coalesced_count,
        scheduler_preempted_by: spans.scheduler_preempted_by.clone(),
        viewer_window_events_after_input,
    }
}

/// 표본이 어느 lane에서 나왔는지. **두 lane을 한 문자열로 뭉개지 않는다.**
///
/// 이 값이 없으면 evidence에서 fixture 표본과 실제 preset 표본을 구분할 수 없다.
fn current_lane_mode_label() -> String {
    let sample = current_sample_lane_mode();
    let proxy = current_proxy_lane_mode();

    if proxy.is_enabled() {
        return format!("proxy:{}", proxy.as_str());
    }

    match sample.is_enabled() {
        true => sample.as_str().into(),
        false => "off".into(),
    }
}

fn is_in_close_out_scope(open: &OpenGeneration, scope: &CloseOutScope) -> bool {
    match scope {
        CloseOutScope::Due(now_micros) => open.deadline_at_micros <= *now_micros,
        CloseOutScope::OtherSessions(session_id) => &open.session_id != session_id,
        CloseOutScope::Request(request_id) => &open.request_id == request_id,
    }
}

/// **계측 완결성 계약:** commit된 generation 하나당 terminal 행이 정확히 하나 남는다.
///
/// viewer 보고가 먼저 도착하면 그 행이 terminal이고, 유예 시간이 먼저 지나면 host가
/// `present-unreported` 행으로 닫는다. trusted input을 기다리며 보류된 보고가 있다면
/// 그 보고를 그대로 기록한다 — 시작점만 없을 뿐 종료점은 실제로 관측했기 때문이다.
fn close_out_open_generations(app: &tauri::AppHandle, scope: CloseOutScope) {
    let closed_at_micros = current_monotonic_micros();
    let records = {
        let mut telemetry = telemetry_state()
            .lock()
            .expect("telemetry state lock poisoned");

        let mut selected = Vec::new();
        let mut retained = Vec::new();

        for open in std::mem::take(&mut telemetry.open_generations) {
            if is_in_close_out_scope(&open, &scope) {
                selected.push(open);
            } else {
                retained.push(open);
            }
        }

        telemetry.open_generations = retained;

        selected
            .into_iter()
            .filter_map(|open| {
                // request 기록이 이미 정리됐어도 행은 반드시 남는다. 여기서 조용히 빠져나가면
                // 고치려던 바로 그 결함(행 없는 generation)을 다시 만든다.
                let mut orphaned = PendingRequest {
                    session_id: open.session_id.clone(),
                    ..PendingRequest::default()
                };
                let pending = telemetry
                    .requests
                    .get_mut(&open.request_id)
                    .unwrap_or(&mut orphaned);
                let deferred = pending.pending_present_reports.remove(&open.generation_id);

                if pending
                    .generations
                    .get(&open.generation_id)
                    .is_some_and(|spans| spans.terminal_recorded)
                {
                    return None;
                }

                if let Some(report) = deferred {
                    // 종료점은 관측됐고 시작점만 없다. `build_present_record`가 latency를
                    // 계산하지 않으므로 KPI를 오염시키지 않는다.
                    let window_events = window_events_for_report(pending, &report);

                    return Some(build_present_record(
                        &open.request_id,
                        pending,
                        &report,
                        window_events,
                    ));
                }

                let spans = {
                    let spans = pending
                        .generations
                        .entry(open.generation_id.clone())
                        .or_default();
                    spans.terminal_recorded = true;
                    spans.clone()
                };

                Some(build_unreported_record(
                    &open,
                    pending,
                    &spans,
                    closed_at_micros,
                ))
            })
            .collect::<Vec<_>>()
    };

    for record in records {
        if record.reject_reason.as_deref() == Some(DISPLAY_REJECT_PRESENT_UNREPORTED) {
            log::warn!(
                "display_present_unreported generation={} request={} window_events={} — 계측 완결성 실패 표본",
                record.generation_id.as_deref().unwrap_or("unknown"),
                record.request_id,
                record.viewer_window_events_after_input
            );
        }

        let _ = persist_present_record(app, &record);
    }
}

/// terminal 행을 durable evidence에 남긴다.
///
/// **쓰기 실패를 성공으로 바꾸지 않는다.** 이 행이 없으면 그 generation은 evidence에서
/// 사라지고, 완결성 gate(`check-telemetry-completeness.ps1`)는 그것을 "표시되지 않았다"와
/// 구분하지 못한다.
fn persist_present_record(
    app: &tauri::AppHandle,
    record: &PresentSampleRecord,
) -> Result<(), HostErrorEnvelope> {
    let base_dir = session_base_dir(app)?;

    if record.viewer_window_events_after_input > 0 {
        log::warn!(
            "display_viewer_window_event_after_trusted_input request={} count={} — AC4 실패 표본",
            record.request_id,
            record.viewer_window_events_after_input
        );
    }

    if let Err(error) = append_present_sample(&base_dir, &record.session_id, record) {
        log::error!(
            "display_present_telemetry_failed generation={} request={} error={} — 계측 완결성 실패",
            record.generation_id.as_deref().unwrap_or("unknown"),
            record.request_id,
            error.message
        );

        return Err(error);
    }

    Ok(())
}

/// viewer → host. 표시 결과와 viewer 측 span을 기록한다.
#[tauri::command]
pub fn report_display_present(
    app: tauri::AppHandle,
    webview_window: tauri::WebviewWindow,
    mut input: DisplayPresentReportDto,
) -> Result<(), HostErrorEnvelope> {
    if !is_viewer_window_label(webview_window.label()) {
        return Err(HostErrorEnvelope::capability_denied(
            "관람 창만 표시 결과를 보고할 수 있어요.",
        ));
    }

    // 검증 실패도 유실 경로다. 호출자는 오류를 삼키므로 여기서 남기지 않으면 흔적이 사라진다.
    if let Err(error) = validate_display_present_report(&input) {
        log::warn!(
            "display_present_report_invalid generation={} outcome={} reject_reason={} message={}",
            input.generation_id,
            input.outcome,
            input.reject_reason.as_deref().unwrap_or("none"),
            error.message
        );

        return Err(error);
    }

    // 픽셀 decode 실패는 활성 truth로 남길 수 없다.
    if input.outcome == "decode-failed" {
        let changed = {
            let handle = app.state::<DisplayStateHandle>();
            let mut state = handle.0.lock().expect("display state lock poisoned");
            state.mark_poisoned(&input.generation_id)
        };

        log::warn!(
            "display_generation_decode_failed generation={}",
            input.generation_id
        );

        if changed {
            let base_dir = session_base_dir(&app)?;
            let viewer = read_viewer_context(&app);
            let bound_session_id = viewer.session_id.clone();
            let required_width = viewer.required_source_width_px;
            let required_height = viewer.required_source_height_px;
            let snapshot = {
                let handle = app.state::<DisplayStateHandle>();
                let state = handle.0.lock().expect("display state lock poisoned");
                state.snapshot(
                    required_width,
                    required_height,
                    current_display_lane_flags(),
                    current_monotonic_micros(),
                )
            };
            write_reconciled_pointer(&base_dir, bound_session_id.as_deref(), &snapshot)?;
            emit_display_update(&app, snapshot);
        }
    }

    let record = {
        let mut telemetry = telemetry_state()
            .lock()
            .expect("telemetry state lock poisoned");
        let Some(context) = telemetry
            .generation_contexts
            .get(&input.generation_id)
            .cloned()
        else {
            log::warn!(
                "display_present_unknown_generation generation={}",
                input.generation_id
            );
            return Err(HostErrorEnvelope::validation_message(
                "등록되지 않은 표시 세대의 결과는 받을 수 없어요.",
            ));
        };

        // **보고한 viewer 세대와 generation의 세대를 대조한다.**
        //
        // 이 대조가 없으면 재생성된 창의 늦은 보고가 이전 세대의 generation과 짝지어져
        // "표시됐다"로 집계된다. 한 촬영에 generation이 둘(proxy·정밀본)이 되면서
        // 잘못된 짝짓기 위험이 커졌다 (`deferred-work.md` 미결 항목).
        //
        // **행을 없애지는 않는다** — 조용히 버리면 계측 완결성이 깨진다.
        // 대신 결과를 `stale-epoch` 거부로 확정한다.
        if input.viewer_epoch != context.viewer_epoch {
            log::warn!(
                "display_present_epoch_mismatch generation={} reported_epoch={} generation_epoch={}",
                input.generation_id,
                input.viewer_epoch,
                context.viewer_epoch
            );
            input.outcome = "rejected".into();
            input.reject_reason = Some(DISPLAY_REJECT_STALE_EPOCH.into());
        }

        if let Some(pending) = telemetry.requests.get_mut(&context.request_id) {
            if pending
                .generations
                .get(&input.generation_id)
                .is_some_and(|spans| spans.terminal_recorded)
            {
                // generation당 terminal 행은 하나다. 유예 시간을 넘겨 도착한 보고는
                // 행을 하나 더 만드는 대신 로그로 남긴다 — 조용히 버리지도, 중복 집계하지도 않는다.
                log::warn!(
                    "display_present_after_terminal_record generation={} outcome={} swap_at={:?} present_at={:?}",
                    input.generation_id,
                    input.outcome,
                    input.spans.swap_committed_at_micros,
                    input.spans.actual_present_at_micros
                );
                None
            } else if should_defer_present_report(pending, &input) {
                pending
                    .pending_present_reports
                    .insert(input.generation_id.clone(), input);
                None
            } else {
                let window_events = window_events_for_report(pending, &input);
                Some(build_present_record(
                    &context.request_id,
                    pending,
                    &input,
                    window_events,
                ))
            }
        } else {
            log::warn!(
                "display_present_retired_generation generation={} request={}",
                input.generation_id,
                context.request_id
            );
            Some(build_unknown_generation_record(&context, &input))
        }
    };

    let Some(record) = record else {
        return Ok(());
    };

    // **terminal evidence 쓰기 실패를 성공으로 반환하지 않는다.**
    //
    // 이 generation은 이미 `terminal_recorded`로 표시됐으므로, 쓰기가 실패하면 그 행은
    // 영영 남지 않는다. 성공을 반환하면 호출자도 evidence 도구도 그 사실을 알 수 없고,
    // HV-17B의 완결성 판정이 그 반환값 위에 선다 (`deferred-work.md` 미결 항목).
    persist_present_record(&app, &record)
}

/// 계측 lane이 hidden 변형일 때만 사용한다. 기본 lane에서는 호출되지 않는다.
pub fn sample_lane_mode_is_hidden_prewarm() -> bool {
    matches!(current_sample_lane_mode(), SampleLaneMode::HiddenPrewarm)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::dto::{DisplayPresentSpansDto, DISPLAY_REJECT_DECODE_FAILED};

    fn present_report(outcome: &str, reject_reason: Option<&str>) -> DisplayPresentReportDto {
        DisplayPresentReportDto {
            generation_id: "generation-1".into(),
            viewer_epoch: 1,
            outcome: outcome.into(),
            reject_reason: reject_reason.map(str::to_string),
            natural_width_px: 1920,
            natural_height_px: 1080,
            spans: DisplayPresentSpansDto {
                viewer_receipt_at_micros: Some(1_100),
                decode_start_at_micros: Some(1_200),
                decode_end_at_micros: Some(1_300),
                swap_committed_at_micros: Some(1_400),
                img_on_load_at_micros: Some(1_350),
                actual_present_at_micros: Some(1_500),
                element_timing_render_at_micros: None,
                is_element_render_time: false,
            },
            clock_offset_micros: 0,
            clock_uncertainty_micros: 10,
        }
    }

    #[test]
    fn only_the_viewer_window_may_report_present() {
        assert!(is_viewer_window_label("viewer-window"));
        assert!(!is_viewer_window_label("booth-window"));
        assert!(!is_viewer_window_label("operator-window"));
    }

    #[test]
    fn client_micros_convert_with_signed_offset() {
        assert_eq!(to_host_micros(1_000, 500), 1_500);
        assert_eq!(to_host_micros(1_000, -500), 500);
        // 음수로 내려가면 0으로 고정한다. 시작점이 뒤로 밀려 KPI가 짧아지지 않는다.
        assert_eq!(to_host_micros(100, -1_000), 0);
    }

    #[test]
    fn presented_report_waits_for_trusted_input_join() {
        let pending = PendingRequest::default();

        assert!(should_defer_present_report(
            &pending,
            &present_report("presented", None)
        ));
        assert!(!should_defer_present_report(
            &pending,
            &present_report("decode-failed", Some(DISPLAY_REJECT_DECODE_FAILED))
        ));
    }

    #[test]
    fn window_event_after_input_rejects_the_sample() {
        let mut pending = PendingRequest {
            session_id: "session-1".into(),
            trusted_input_at_micros: Some(1_000),
            input_uncertainty_micros: 10,
            is_trusted_input: true,
            ..PendingRequest::default()
        };

        let record = build_present_record(
            "request-1",
            &mut pending,
            &present_report("presented", None),
            1,
        );

        assert_eq!(record.outcome, "rejected");
        assert_eq!(
            record.reject_reason.as_deref(),
            Some(DISPLAY_REJECT_VIEWER_WINDOW_EVENT_AFTER_INPUT)
        );
        assert_eq!(record.viewer_window_events_after_input, 1);
        assert_eq!(record.qualifying_latency_micros, None);
    }

    #[test]
    fn contradictory_present_outcomes_are_rejected() {
        assert!(validate_display_present_report(&present_report("presented", None)).is_ok());
        assert!(validate_display_present_report(&present_report(
            "presented",
            Some(DISPLAY_REJECT_DECODE_FAILED)
        ))
        .is_err());
        assert!(validate_display_present_report(&present_report("rejected", None)).is_err());
        assert!(validate_display_present_report(&present_report(
            "rejected",
            Some(DISPLAY_REJECT_DECODE_FAILED)
        ))
        .is_err());
        assert!(validate_display_present_report(&present_report(
            "decode-failed",
            Some(DISPLAY_REJECT_DECODE_FAILED)
        ))
        .is_ok());

        let mut missing_present_timestamp = present_report("presented", None);
        missing_present_timestamp.spans.actual_present_at_micros = None;
        assert!(validate_display_present_report(&missing_present_timestamp).is_err());
    }

    fn open_generation(deadline_at_micros: u64) -> OpenGeneration {
        OpenGeneration {
            generation_id: "request-1-000008".into(),
            request_id: "request-1".into(),
            session_id: "session-1".into(),
            deadline_at_micros,
        }
    }

    #[test]
    fn only_generations_past_the_grace_deadline_are_closed_out() {
        let open = open_generation(2_000);

        assert!(!is_in_close_out_scope(&open, &CloseOutScope::Due(1_999)));
        assert!(is_in_close_out_scope(&open, &CloseOutScope::Due(2_000)));
        assert!(is_in_close_out_scope(&open, &CloseOutScope::Due(2_001)));
    }

    #[test]
    fn session_and_request_boundaries_close_out_their_own_generations() {
        let open = open_generation(u64::MAX);

        // 세션이 바뀌면 이전 세션 것만 닫는다. 현재 세션 표본을 조기 종료시키면 안 된다.
        assert!(is_in_close_out_scope(
            &open,
            &CloseOutScope::OtherSessions("session-2".into())
        ));
        assert!(!is_in_close_out_scope(
            &open,
            &CloseOutScope::OtherSessions("session-1".into())
        ));

        assert!(is_in_close_out_scope(
            &open,
            &CloseOutScope::Request("request-1".into())
        ));
        assert!(!is_in_close_out_scope(
            &open,
            &CloseOutScope::Request("request-2".into())
        ));
    }

    #[test]
    fn an_unreported_generation_still_produces_a_terminal_row() {
        let pending = PendingRequest {
            session_id: "session-1".into(),
            trusted_input_at_micros: Some(1_000),
            input_uncertainty_micros: 10,
            is_trusted_input: true,
            host_accepted_at_micros: Some(1_100),
            ..PendingRequest::default()
        };
        let spans = GenerationSpans {
            sample_write_start_at_micros: Some(1_200),
            file_ready_at_micros: Some(1_300),
            probe_ok_at_micros: Some(1_400),
            pointer_committed_at_micros: Some(1_500),
            event_emitted_at_micros: Some(1_600),
            sample_variant: Some("b".into()),
            terminal_recorded: true,
            ..GenerationSpans::default()
        };

        let record = build_unreported_record(&open_generation(3_600), &pending, &spans, 3_700);

        assert_eq!(record.generation_id.as_deref(), Some("request-1-000008"));
        assert_eq!(record.outcome, "rejected");
        assert_eq!(
            record.reject_reason.as_deref(),
            Some(DISPLAY_REJECT_PRESENT_UNREPORTED)
        );
        assert_eq!(record.sample_variant.as_deref(), Some("b"));
        // host가 관측한 구간은 그대로 남고, 관측하지 못한 종료점은 비어 있어야 한다.
        assert_eq!(record.pointer_committed_at_micros, Some(1_500));
        assert_eq!(record.event_emitted_at_micros, Some(1_600));
        assert_eq!(record.actual_present_at_micros, None);
        assert_eq!(record.qualifying_latency_micros, None);
        assert_eq!(record.confidence, PRESENT_CONFIDENCE_UNREPORTED);
        assert_eq!(record.trusted_input_at_micros, Some(1_000));
    }

    #[test]
    fn a_deferred_report_is_recorded_without_a_qualifying_latency() {
        // trusted input이 끝내 도착하지 않아도 종료점은 실제로 관측됐다.
        // 그 표본은 기록하되 KPI에는 넣지 않는다.
        let mut pending = PendingRequest {
            session_id: "session-1".into(),
            ..PendingRequest::default()
        };

        let record = build_present_record(
            "request-1",
            &mut pending,
            &present_report("presented", None),
            0,
        );

        assert_eq!(record.outcome, "presented");
        assert_eq!(record.actual_present_at_micros, Some(1_500));
        assert_eq!(record.qualifying_latency_micros, None);
        assert!(!record.is_trusted_input);
        assert!(
            pending
                .generations
                .get("generation-1")
                .is_some_and(|spans| spans.terminal_recorded),
            "기록된 generation은 close-out이 다시 행을 만들지 않도록 표시되어야 한다"
        );
    }

    #[test]
    fn retired_generation_report_becomes_an_explicit_rejection() {
        let context = GenerationContext {
            session_id: "session-1".into(),
            request_id: "request-1".into(),
            sample_variant: Some("a".into()),
            provenance: GenerationProvenanceSummary::default(),
            viewer_epoch: 3,
            registered_at_micros: 1_000,
        };

        let record = build_unknown_generation_record(&context, &present_report("presented", None));

        assert_eq!(record.outcome, "rejected");
        assert_eq!(
            record.reject_reason.as_deref(),
            Some(DISPLAY_REJECT_UNKNOWN_GENERATION)
        );
        assert_eq!(record.session_id, "session-1");
        assert_eq!(record.request_id, "request-1");
    }
}
