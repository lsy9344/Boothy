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
        TrustedInputReportDto, DISPLAY_REJECT_PRESENT_UNREPORTED,
        DISPLAY_REJECT_UNKNOWN_GENERATION,
    },
    display::{
        generation_repository::{remove_request_generations, write_reconciled_pointer},
        sample_publisher::{
            current_sample_gap_ms, current_sample_lane_mode, publish_generation_in_dir,
            sample_fixture_path, PublishRequest, SampleLaneMode,
        },
        DisplayStateHandle,
    },
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

#[derive(Debug, Default, Clone)]
struct GenerationSpans {
    sample_write_start_at_micros: Option<u64>,
    file_ready_at_micros: Option<u64>,
    probe_ok_at_micros: Option<u64>,
    pointer_committed_at_micros: Option<u64>,
    event_emitted_at_micros: Option<u64>,
    sample_variant: Option<String>,
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
    sample_variant: String,
}

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

fn session_base_dir(app: &tauri::AppHandle) -> Result<PathBuf, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;

    Ok(resolve_app_session_base_dir(app_local_data_dir))
}

/// 현재 viewer truth(세션 binding, epoch, photo rect)를 읽는다.
fn read_viewer_context(app: &tauri::AppHandle) -> (Option<String>, u64, u32, u32) {
    let handle = app.state::<ViewerStateHandle>();
    let state = handle.0.lock().expect("viewer state lock poisoned");
    let snapshot = state.snapshot_with_clock(
        crate::viewer::current_epoch_ms(),
        crate::viewer::current_monotonic_ms(),
    );
    let (required_width, required_height) = snapshot
        .photo_rect
        .as_ref()
        .map(|rect| {
            (
                rect.required_source_width_px,
                rect.required_source_height_px,
            )
        })
        .unwrap_or((0, 0));

    (
        snapshot.session_id,
        snapshot.viewer_epoch,
        required_width,
        required_height,
    )
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
    let (bound_session_id, _viewer_epoch, required_width, required_height) =
        read_viewer_context(app);

    let (snapshot, changed, previous_session_id) = {
        let handle = app.state::<DisplayStateHandle>();
        let mut state = handle.0.lock().expect("display state lock poisoned");
        let previous_session_id = state.session_id().map(str::to_string);
        let changed = state.reconcile(bound_session_id.as_deref(), required_width, required_height);

        (
            state.snapshot(
                required_width,
                required_height,
                current_sample_lane_mode().is_enabled(),
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
    // 이전 세션의 미결 generation을 **버리기 전에** 닫는다. 세션이 끝났다는 이유로
    // 계측 행이 사라지면 그 세션의 evidence 분모가 조용히 줄어든다.
    close_out_open_generations(app, CloseOutScope::OtherSessions(session_id.to_string()));

    telemetry_state()
        .lock()
        .expect("telemetry state lock poisoned")
        .requests
        .retain(|_, pending| pending.session_id == session_id);

    read_display_snapshot(app)?;
    Ok(())
}

/// capture 삭제 시 해당 request의 표시 자산과 pointer를 함께 정리한다.
pub fn forget_display_request(
    app: &tauri::AppHandle,
    session_id: &str,
    request_id: &str,
) -> Result<(), HostErrorEnvelope> {
    // 삭제 전에 이 request의 미결 generation을 닫는다.
    close_out_open_generations(app, CloseOutScope::Request(request_id.to_string()));

    {
        let mut telemetry = telemetry_state()
            .lock()
            .expect("telemetry state lock poisoned");
        telemetry.requests.remove(request_id);
    }

    let base_dir = session_base_dir(app)?;
    let (_, _, required_width, required_height) = read_viewer_context(app);
    let (changed, snapshot) = {
        let handle = app.state::<DisplayStateHandle>();
        let mut state = handle.0.lock().expect("display state lock poisoned");
        let changed = state.forget_request(request_id);
        let snapshot = state.snapshot(
            required_width,
            required_height,
            current_sample_lane_mode().is_enabled(),
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
    let (bound_session_id, current_viewer_epoch, required_width, required_height) =
        read_viewer_context(app);

    let source_bytes =
        resolve_sample_fixture_bytes(app, &input.sample_variant).ok_or_else(|| {
            HostErrorEnvelope::persistence("표시 샘플 자산을 찾지 못했어요.".to_string())
        })?;

    let write_start_at_micros = current_monotonic_micros();
    let request = PublishRequest {
        session_id: &input.session_id,
        request_id: &input.request_id,
        capture_id: input.capture_id.as_deref(),
        sample_variant: &input.sample_variant,
        source_bytes: &source_bytes,
        bound_session_id: bound_session_id.as_deref(),
        request_viewer_epoch: request_viewer_epoch.unwrap_or(current_viewer_epoch),
        current_viewer_epoch,
        required_source_width_px: required_width,
        required_source_height_px: required_height,
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

    {
        let mut telemetry = telemetry_state()
            .lock()
            .expect("telemetry state lock poisoned");
        telemetry.generation_contexts.insert(
            generation.generation_id.clone(),
            GenerationContext {
                session_id: generation.session_id.clone(),
                request_id: generation.request_id.clone(),
                sample_variant: generation.sample_variant.clone(),
            },
        );

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
                sample_write_start_at_micros: Some(write_start_at_micros),
                file_ready_at_micros: outcome.file_ready_at_micros,
                probe_ok_at_micros: outcome.probe_ok_at_micros,
                pointer_committed_at_micros: outcome.pointer_committed_at_micros,
                event_emitted_at_micros: Some(event_emitted_at_micros),
                sample_variant: Some(generation.sample_variant.clone()),
                terminal_recorded: false,
            },
        );

        // 이 generation은 terminal 행이 남을 때까지 미결이다.
        telemetry.open_generations.push(OpenGeneration {
            generation_id: generation.generation_id.clone(),
            request_id: generation.request_id.clone(),
            session_id: generation.session_id.clone(),
            deadline_at_micros: event_emitted_at_micros.saturating_add(PRESENT_REPORT_GRACE_MICROS),
        });
    }

    // 유예 시간이 지난 이전 generation을 여기서 닫는다. 계측 lane 안에서만 도는 경로다.
    close_out_open_generations(app, CloseOutScope::Due(current_monotonic_micros()));

    log::info!(
        "display_sample_committed session={} request={} generation={} variant={} size={}x{}",
        generation.session_id,
        generation.request_id,
        generation.generation_id,
        generation.sample_variant,
        generation.source_width_px,
        generation.source_height_px
    );

    Ok(outcome.pointer)
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

    let gap_ms = current_sample_gap_ms();
    let lane_app = app.clone();
    // request가 수락된 시점의 viewer 세대. 게시 사이에 창이 재생성되면 stale-epoch로 거부된다.
    let (_, request_viewer_epoch, _, _) = read_viewer_context(app);
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
    let (
        sample_variant,
        sample_write_start_at_micros,
        file_ready_at_micros,
        probe_ok_at_micros,
        pointer_committed_at_micros,
        event_emitted_at_micros,
    ) = {
        let spans = pending
            .generations
            .entry(input.generation_id.clone())
            .or_default();
        // 이 generation의 terminal 행은 여기서 확정된다. 이후 close-out은 행을 더 만들지 않는다.
        spans.terminal_recorded = true;
        (
            spans.sample_variant.clone(),
            spans.sample_write_start_at_micros,
            spans.file_ready_at_micros,
            spans.probe_ok_at_micros,
            spans.pointer_committed_at_micros,
            spans.event_emitted_at_micros,
        )
    };

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
    let qualifying_latency_micros = match (
        pending.trusted_input_at_micros,
        actual_present_at_micros,
        outcome.as_str(),
        pending.is_trusted_input,
    ) {
        (Some(input_at), Some(present_at), "presented", true) => Some(conservative_latency_micros(
            input_at,
            present_at,
            pending.input_uncertainty_micros,
            input.clock_uncertainty_micros,
        )),
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
        lane_mode: current_sample_lane_mode().as_str().into(),
        outcome,
        reject_reason,
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
        sample_variant: Some(context.sample_variant.clone()),
        lane_mode: current_sample_lane_mode().as_str().into(),
        outcome: "rejected".into(),
        reject_reason: Some(DISPLAY_REJECT_UNKNOWN_GENERATION.into()),
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
        lane_mode: current_sample_lane_mode().as_str().into(),
        outcome: "rejected".into(),
        reject_reason: Some(DISPLAY_REJECT_PRESENT_UNREPORTED.into()),
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
        viewer_window_events_after_input,
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

fn persist_present_record(
    app: &tauri::AppHandle,
    record: &PresentSampleRecord,
) -> Result<(), HostErrorEnvelope> {
    let base_dir = session_base_dir(app)?;

    if let Err(error) = append_present_sample(&base_dir, &record.session_id, record) {
        log::warn!("display_present_telemetry_failed error={}", error.message);
    }

    if record.viewer_window_events_after_input > 0 {
        log::warn!(
            "display_viewer_window_event_after_trusted_input request={} count={} — AC4 실패 표본",
            record.request_id,
            record.viewer_window_events_after_input
        );
    }

    Ok(())
}

/// viewer → host. 표시 결과와 viewer 측 span을 기록한다.
#[tauri::command]
pub fn report_display_present(
    app: tauri::AppHandle,
    webview_window: tauri::WebviewWindow,
    input: DisplayPresentReportDto,
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
            let (bound_session_id, _, required_width, required_height) = read_viewer_context(&app);
            let snapshot = {
                let handle = app.state::<DisplayStateHandle>();
                let state = handle.0.lock().expect("display state lock poisoned");
                state.snapshot(
                    required_width,
                    required_height,
                    current_sample_lane_mode().is_enabled(),
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

    let base_dir = session_base_dir(&app)?;

    if let Err(error) = append_present_sample(&base_dir, &record.session_id, &record) {
        log::warn!("display_present_telemetry_failed error={}", error.message);
    }

    if record.viewer_window_events_after_input > 0 {
        log::warn!(
            "display_viewer_window_event_after_trusted_input request={} count={} — AC4 실패 표본",
            record.request_id,
            record.viewer_window_events_after_input
        );
    }

    Ok(())
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
            sample_variant: "a".into(),
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
