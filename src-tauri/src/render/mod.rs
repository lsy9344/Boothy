pub mod scheduler;

use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::Write,
    panic::{catch_unwind, resume_unwind, AssertUnwindSafe},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Condvar, LazyLock, Mutex},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crate::{
    preset::preset_bundle::PublishedPresetRuntimeBundle,
    preset::preset_catalog::{
        find_published_preset_runtime_bundle, resolve_published_preset_catalog_dir,
    },
    session::{session_manifest::SessionCaptureRecord, session_paths::SessionPaths},
    viewer::current_monotonic_micros,
};

pub use scheduler::{
    background_job_key, display_job_key, display_job_key_for_context, scheduler_for, CancelScope,
    CancellationToken, JobCoordinates, JobPriority, RenderJobRequest, RenderLease,
    SchedulerAdmission, SchedulerSpans, MAX_IN_FLIGHT_CURRENT_CAPTURE_RENDER_JOBS,
    MAX_IN_FLIGHT_RENDER_JOBS,
};

pub const PINNED_DARKTABLE_VERSION: &str = "5.4.1";
const DEFAULT_RENDER_TIMEOUT: Duration = Duration::from_secs(45);
/// Story 7.6. NFR-003 hard max. deadline은 **순서 결정과 관측에만** 쓴다.
pub const DISPLAY_LANE_DEADLINE_BUDGET_MICROS: u64 = 5_000_000;
/// 취소 신호를 확인하는 간격. 렌더 loop의 polling 주기와 같다.
const RENDER_CANCEL_POLL: Duration = Duration::from_millis(100);
const DARKTABLE_CLI_BIN_ENV: &str = "BOOTHY_DARKTABLE_CLI_BIN";
/// 명시한 회차에서만 taskkill 원문과 process-tree 결과를 보존한다.
const HV17_EVIDENCE_ROOT_ENV: &str = "BOOTHY_HV17_EVIDENCE_ROOT";
const RAW_PREVIEW_MAX_WIDTH_PX: u32 = 384;
const RAW_PREVIEW_MAX_HEIGHT_PX: u32 = 384;
const FAST_PREVIEW_RENDER_MAX_WIDTH_PX: u32 = 384;
const FAST_PREVIEW_RENDER_MAX_HEIGHT_PX: u32 = 384;
const DARKTABLE_APPLY_CUSTOM_PRESETS_DISABLED: &str = "false";
/// Story 7.4. `darktable-cli`에는 JPEG 품질 전용 플래그가 없다. core 설정으로만 전달된다.
/// **실측으로 확인한 키다** — quality 40과 95가 서로 다른 산출물을 만든다.
const DARKTABLE_JPEG_QUALITY_CONF_KEY: &str = "plugins/imageio/format/jpeg/quality";
const PREVIEW_RENDER_WARMUP_INPUT_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

static PREVIEW_RENDER_WARMUP_IN_FLIGHT: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));
/// 출력 경로별 P2 owner와 follower가 공유하는 in-process flight.
/// 동일 operation은 결과를 공유하고, 다른 operation은 현재 owner의 commit 뒤에 이어서 실행한다.
static BACKGROUND_RENDER_FLIGHTS: LazyLock<Mutex<HashMap<PathBuf, Arc<BackgroundRenderFlight>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// P2 작업 하나를 큐에 넣고 기다린다. **가득 차도 즉시 실패하지 않는다.**
///
/// 오늘의 `render-queue-saturated` 즉시 실패가 여기서 사라진다. 대신 취소(삭제·세션 교체)만
/// 이 함수를 오류로 끝낼 수 있다.
fn admit_background_render(
    base_dir: &Path,
    session_id: &str,
    request_id: Option<&str>,
    capture_id: &str,
    stage: &str,
    stage_context: RenderStage,
    output_identity: &Path,
) -> Result<RenderLease, RenderWorkerError> {
    let scheduler = scheduler_for(base_dir);
    let request = RenderJobRequest {
        priority: JobPriority::P2Background,
        job_key: format!(
            "{}#output={}",
            background_job_key(session_id, capture_id, stage),
            output_identity.to_string_lossy()
        ),
        // P2는 촬영 예산을 갖지 않는다. 순서 안에서는 FIFO로만 정렬된다.
        deadline_micros: u64::MAX,
        coordinates: JobCoordinates {
            session_id: session_id.to_string(),
            // 삭제는 request 단위로 취소한다. 알 수 있는 호출자는 반드시 넘긴다.
            request_id: request_id.unwrap_or_default().to_string(),
            capture_id: Some(capture_id.to_string()),
            capture_order: None,
            viewer_epoch: 0,
        },
    };

    match scheduler.admit(&request, &current_monotonic_micros) {
        SchedulerAdmission::Granted(lease) => Ok(*lease),
        SchedulerAdmission::Cancelled { spans } => Err(RenderWorkerError {
            reason_code: "render-cancelled",
            customer_message: stage_context.customer_message.into(),
            operator_detail: format!(
                "렌더가 시작 전에 취소됐어요: reason={} queueWaitMicros={}",
                spans.preempted_by.as_deref().unwrap_or("unknown"),
                spans.queue_wait_micros
            ),
        }),
        // 동일 출력의 follower는 이 함수 위의 flight에서 owner 결과를 기다린다.
        SchedulerAdmission::Coalesced { job_key, .. } => Err(RenderWorkerError {
            reason_code: "render-coordination-failed",
            customer_message: stage_context.customer_message.into(),
            operator_detail: format!(
                "flight owner가 scheduler에서 예기치 않게 병합됐어요: jobKey={job_key}"
            ),
        }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderIntent {
    Preview,
    Final,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedCaptureAsset {
    pub asset_path: String,
    pub ready_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderWorkerError {
    pub reason_code: &'static str,
    pub customer_message: String,
    pub operator_detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreviewRenderSourceKind {
    RawOriginal,
    FastPreviewRaster,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPreviewRender {
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BackgroundRenderOperation {
    Capture {
        session_id: String,
        request_id: String,
        capture_id: String,
        intent: RenderIntent,
        preset_id: Option<String>,
        preset_version: String,
        forced_source_kind: Option<PreviewRenderSourceKind>,
    },
    SpeculativePreview {
        session_id: String,
        request_id: String,
        capture_id: String,
        preset_id: String,
        preset_version: String,
        source_asset_path: PathBuf,
    },
}

impl BackgroundRenderOperation {
    fn customer_message(&self) -> String {
        match self {
            Self::Capture { intent, .. } => safe_render_failure_message(*intent),
            Self::SpeculativePreview { .. } => safe_render_failure_message(RenderIntent::Preview),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BackgroundRenderOutcome {
    Capture(RenderedCaptureAsset),
    SpeculativePreview(PreparedPreviewRender),
}

struct BackgroundRenderFlight {
    operation: BackgroundRenderOperation,
    outcome: Mutex<Option<Result<BackgroundRenderOutcome, RenderWorkerError>>>,
    ready: Condvar,
}

impl BackgroundRenderFlight {
    fn new(operation: BackgroundRenderOperation) -> Self {
        Self {
            operation,
            outcome: Mutex::new(None),
            ready: Condvar::new(),
        }
    }

    fn publish(&self, outcome: Result<BackgroundRenderOutcome, RenderWorkerError>) {
        let mut published = self
            .outcome
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *published = Some(outcome);
        self.ready.notify_all();
    }

    fn wait(&self) -> Result<BackgroundRenderOutcome, RenderWorkerError> {
        let mut published = self
            .outcome
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        loop {
            if let Some(outcome) = published.as_ref() {
                return outcome.clone();
            }
            published = self
                .ready
                .wait(published)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }
}

fn run_or_join_background_render(
    output_identity: PathBuf,
    operation: BackgroundRenderOperation,
    render: impl FnOnce() -> Result<BackgroundRenderOutcome, RenderWorkerError>,
) -> Result<BackgroundRenderOutcome, RenderWorkerError> {
    let mut render = Some(render);

    loop {
        let (flight, is_owner, is_same_operation) = {
            let mut flights = BACKGROUND_RENDER_FLIGHTS
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(flight) = flights.get(&output_identity) {
                (Arc::clone(flight), false, flight.operation == operation)
            } else {
                let flight = Arc::new(BackgroundRenderFlight::new(operation.clone()));
                flights.insert(output_identity.clone(), Arc::clone(&flight));
                (flight, true, true)
            }
        };

        if !is_owner {
            let outcome = flight.wait();
            if is_same_operation {
                return outcome;
            }
            // A different recipe targeting the same path waits for the current commit, then retries.
            continue;
        }

        let owner_result = catch_unwind(AssertUnwindSafe(
            render
                .take()
                .expect("background render closure must run exactly once"),
        ));
        match owner_result {
            Ok(outcome) => {
                flight.publish(outcome.clone());
                remove_background_render_flight(&output_identity, &flight);
                return outcome;
            }
            Err(payload) => {
                flight.publish(Err(RenderWorkerError {
                    reason_code: "render-owner-abandoned",
                    customer_message: operation.customer_message(),
                    operator_detail: "background render owner stopped before publishing a result"
                        .into(),
                }));
                remove_background_render_flight(&output_identity, &flight);
                resume_unwind(payload);
            }
        }
    }
}

fn remove_background_render_flight(output_identity: &Path, flight: &Arc<BackgroundRenderFlight>) {
    let mut flights = BACKGROUND_RENDER_FLIGHTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if flights
        .get(output_identity)
        .is_some_and(|active| Arc::ptr_eq(active, flight))
    {
        flights.remove(output_identity);
    }
}

pub fn render_capture_asset_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture: &SessionCaptureRecord,
    intent: RenderIntent,
) -> Result<RenderedCaptureAsset, RenderWorkerError> {
    render_capture_asset_with_forced_source_in_dir(base_dir, session_id, capture, intent, None)
}

pub fn render_capture_asset_from_raw_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture: &SessionCaptureRecord,
    intent: RenderIntent,
) -> Result<RenderedCaptureAsset, RenderWorkerError> {
    render_capture_asset_with_forced_source_in_dir(
        base_dir,
        session_id,
        capture,
        intent,
        Some(PreviewRenderSourceKind::RawOriginal),
    )
}

fn render_capture_asset_with_forced_source_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture: &SessionCaptureRecord,
    intent: RenderIntent,
    forced_source_kind: Option<PreviewRenderSourceKind>,
) -> Result<RenderedCaptureAsset, RenderWorkerError> {
    let paths = SessionPaths::new(base_dir, session_id);
    let output_path = canonical_render_output_path(&paths, &capture.capture_id, intent);
    let operation = BackgroundRenderOperation::Capture {
        session_id: session_id.to_string(),
        request_id: capture.request_id.clone(),
        capture_id: capture.capture_id.clone(),
        intent,
        preset_id: capture.active_preset_id.clone(),
        preset_version: capture.active_preset_version.clone(),
        forced_source_kind,
    };

    match run_or_join_background_render(output_path, operation, || {
        render_capture_asset_as_owner(base_dir, session_id, capture, intent, forced_source_kind)
            .map(BackgroundRenderOutcome::Capture)
    })? {
        BackgroundRenderOutcome::Capture(asset) => Ok(asset),
        BackgroundRenderOutcome::SpeculativePreview(_) => Err(RenderWorkerError {
            reason_code: "render-coordination-failed",
            customer_message: safe_render_failure_message(intent),
            operator_detail: "capture render joined an incompatible flight outcome".into(),
        }),
    }
}

fn render_capture_asset_as_owner(
    base_dir: &Path,
    session_id: &str,
    capture: &SessionCaptureRecord,
    intent: RenderIntent,
    forced_source_kind: Option<PreviewRenderSourceKind>,
) -> Result<RenderedCaptureAsset, RenderWorkerError> {
    let paths = SessionPaths::new(base_dir, session_id);
    let output_path = canonical_render_output_path(&paths, &capture.capture_id, intent);
    // Story 7.6. P2다. **display 이벤트로 취소되지 않는다** — Story 3.2의 완료 진실이
    // final 산출물에 달려 있다. 삭제와 세션 교체만 이 작업을 취소한다.
    let queue_lease = admit_background_render(
        base_dir,
        session_id,
        Some(&capture.request_id),
        &capture.capture_id,
        render_stage_label(intent),
        stage_for_intent(intent),
        &output_path,
    )?;
    let preset_id =
        capture
            .active_preset_id
            .as_deref()
            .ok_or_else(|| RenderWorkerError {
                reason_code: "missing-preset-binding",
                customer_message: safe_render_failure_message(intent),
                operator_detail:
                    "capture record에 activePresetId가 없어 published bundle을 고정할 수 없어요."
                        .into(),
            })?;
    let bundle =
        resolve_runtime_bundle_in_dir(base_dir, preset_id, &capture.active_preset_version, intent)?;

    let output_root = match intent {
        RenderIntent::Preview => &paths.renders_previews_dir,
        RenderIntent::Final => &paths.renders_finals_dir,
    };
    let staging_output_path =
        build_staging_render_output_path(output_root, &capture.capture_id, intent);

    fs::create_dir_all(output_root).map_err(|error| RenderWorkerError {
        reason_code: "render-output-dir-unavailable",
        customer_message: safe_render_failure_message(intent),
        operator_detail: format!("render output directory를 준비하지 못했어요: {error}"),
    })?;

    if !output_path.starts_with(output_root) {
        return Err(RenderWorkerError {
            reason_code: "invalid-output-path",
            customer_message: safe_render_failure_message(intent),
            operator_detail: "render output path가 현재 세션 범위를 벗어났어요.".into(),
        });
    }

    let _ = fs::remove_file(&staging_output_path);

    let invocation = build_darktable_invocation(
        base_dir,
        &bundle.darktable_version,
        &bundle.xmp_template_path,
        capture,
        &paths,
        &staging_output_path,
        intent,
        forced_source_kind,
    );
    log::info!(
        "render_job_started session={} capture_id={} stage={} binary={} source={} detail={}",
        session_id,
        capture.capture_id,
        render_stage_label(intent),
        invocation.binary,
        invocation.binary_source,
        render_invocation_detail_with_source(intent, Some(invocation.render_source_kind))
    );
    let render_started = Instant::now();
    let invocation_result = match run_darktable_invocation(
        &invocation,
        stage_for_intent(intent),
        Some(queue_lease.token()),
    ) {
        Ok(result) => result,
        Err(error) => {
            let _ = fs::remove_file(&staging_output_path);
            return Err(error);
        }
    };
    if let Err(error) = validate_render_output(&staging_output_path, stage_for_intent(intent)) {
        let _ = fs::remove_file(&staging_output_path);
        return Err(error);
    }
    if let Err(error) = promote_render_output(&staging_output_path, &output_path, intent) {
        let _ = fs::remove_file(&staging_output_path);
        return Err(error);
    }
    let render_elapsed_ms = render_started.elapsed().as_millis();

    let ready_at_ms = current_time_ms().map_err(|error| RenderWorkerError {
        reason_code: "render-clock-unavailable",
        customer_message: safe_render_failure_message(intent),
        operator_detail: error,
    })?;

    append_render_event(
        &paths,
        &capture.capture_id,
        Some(&capture.request_id),
        intent,
        render_ready_event_name(intent),
        Some(match intent {
            RenderIntent::Preview => "preview-ready",
            RenderIntent::Final => "final-ready",
        }),
        Some(&format!(
            "presetId={};publishedVersion={};binary={};source={};elapsedMs={};detail={};args={};status={}",
            bundle.preset_id,
            bundle.published_version,
            invocation.binary,
            invocation.binary_source,
            render_elapsed_ms,
            render_invocation_detail_with_source(intent, Some(invocation.render_source_kind)),
            invocation.arguments.join(" "),
            invocation_result.exit_code
        )),
    );

    Ok(RenderedCaptureAsset {
        asset_path: output_path.to_string_lossy().into_owned(),
        ready_at_ms,
    })
}

pub fn render_preview_asset_to_path_in_dir(
    base_dir: &Path,
    session_id: &str,
    request_id: &str,
    capture_id: &str,
    preset_id: &str,
    preset_version: &str,
    source_asset_path: &Path,
    output_path: &Path,
) -> Result<PreparedPreviewRender, RenderWorkerError> {
    let operation = BackgroundRenderOperation::SpeculativePreview {
        session_id: session_id.to_string(),
        request_id: request_id.to_string(),
        capture_id: capture_id.to_string(),
        preset_id: preset_id.to_string(),
        preset_version: preset_version.to_string(),
        source_asset_path: source_asset_path.to_path_buf(),
    };

    match run_or_join_background_render(output_path.to_path_buf(), operation, || {
        render_preview_asset_to_path_as_owner(
            base_dir,
            session_id,
            request_id,
            capture_id,
            preset_id,
            preset_version,
            source_asset_path,
            output_path,
        )
        .map(BackgroundRenderOutcome::SpeculativePreview)
    })? {
        BackgroundRenderOutcome::SpeculativePreview(prepared) => Ok(prepared),
        BackgroundRenderOutcome::Capture(_) => Err(RenderWorkerError {
            reason_code: "render-coordination-failed",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: "speculative preview joined an incompatible flight outcome".into(),
        }),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_preview_asset_to_path_as_owner(
    base_dir: &Path,
    session_id: &str,
    request_id: &str,
    capture_id: &str,
    preset_id: &str,
    preset_version: &str,
    source_asset_path: &Path,
    output_path: &Path,
) -> Result<PreparedPreviewRender, RenderWorkerError> {
    // Story 7.6. 384px 레일의 speculative preview도 P2다. 순서만 뒤로 밀린다.
    let queue_lease = admit_background_render(
        base_dir,
        session_id,
        Some(request_id),
        capture_id,
        "speculative-preview",
        PREVIEW_STAGE,
        output_path,
    )?;
    let bundle =
        resolve_runtime_bundle_in_dir(base_dir, preset_id, preset_version, RenderIntent::Preview)?;

    if !is_valid_render_preview_asset(source_asset_path) {
        return Err(RenderWorkerError {
            reason_code: "invalid-preview-source",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!(
                "speculative preview source가 displayable raster가 아니에요: {}",
                source_asset_path.to_string_lossy()
            ),
        });
    }

    let output_root = output_path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(output_root).map_err(|error| RenderWorkerError {
        reason_code: "render-output-dir-unavailable",
        customer_message: safe_render_failure_message(RenderIntent::Preview),
        operator_detail: format!(
            "speculative render output directory를 준비하지 못했어요: {error}"
        ),
    })?;
    let _ = fs::remove_file(output_path);

    let invocation = build_darktable_invocation_from_source(
        base_dir,
        &bundle.darktable_version,
        &bundle.xmp_template_path,
        source_asset_path,
        output_path,
        RenderIntent::Preview,
        PreviewRenderSourceKind::FastPreviewRaster,
    );
    let render_detail = render_invocation_detail_with_source(
        RenderIntent::Preview,
        Some(invocation.render_source_kind),
    );
    log::info!(
        "speculative_preview_render_started session={} capture_id={} request_id={} binary={} source={} detail={}",
        session_id,
        capture_id,
        request_id,
        invocation.binary,
        invocation.binary_source,
        render_detail
    );

    let render_started = Instant::now();
    let invocation_result =
        match run_darktable_invocation(&invocation, PREVIEW_STAGE, Some(queue_lease.token())) {
            Ok(result) => result,
            Err(error) => {
                let _ = fs::remove_file(output_path);
                return Err(error);
            }
        };
    validate_render_output(output_path, PREVIEW_STAGE)?;
    let render_elapsed_ms = render_started.elapsed().as_millis();

    Ok(PreparedPreviewRender {
        detail: format!(
            "presetId={};publishedVersion={};binary={};source={};elapsedMs={};detail={};args={};status={}",
            bundle.preset_id,
            bundle.published_version,
            invocation.binary,
            invocation.binary_source,
            render_elapsed_ms,
            render_detail,
            invocation.arguments.join(" "),
            invocation_result.exit_code
        ),
    })
}

/// Story 7.4. 관람 화면 크기에 맞춘 preset proxy 한 장을 만든다.
/// Story 7.6. 같은 명세로 `--hq true` 정밀본도 만든다.
///
/// **목표 크기를 호출자가 준다.** 이 함수 안에 크기 상수를 만들지 않는다 —
/// 크기는 Story 7.1의 viewer photoRect × DPR에서만 나오고, 정밀본은 그 값을
/// **commit된 proxy generation에서 상속**한다.
pub struct DisplayProxyRenderSpec<'a> {
    pub source_asset_path: &'a Path,
    /// capture-bound published bundle의 XMP template. live catalog가 아니다.
    pub xmp_template_path: &'a Path,
    pub output_path: &'a Path,
    /// viewer photoRect × DPR. 0이면 viewer가 아직 크기를 보고하지 않은 것이다.
    pub target_width_px: u32,
    pub target_height_px: u32,
    /// 승인된 proxy recipe의 출력 색공간 (`sRGB`).
    pub output_color_space: &'a str,
    pub jpeg_quality: u32,
    pub icc_intent: &'a str,
    /// 이 source가 RAW 원본인지 이미 raster인지. 로그·진단용이다.
    pub source_is_raw_original: bool,
    /// Story 7.6. 이 렌더가 큐에서 갖는 자리.
    pub schedule: DisplayRenderSchedule,
}

/// Story 7.6. display lane 렌더 한 건의 큐 좌표.
#[derive(Debug, Clone)]
pub struct DisplayRenderSchedule {
    /// `P0CurrentProxy` 또는 `P1CurrentRawRefined`.
    pub priority: JobPriority,
    /// `session/request/capture/tier/presetId@version`. 같은 키는 두 번 렌더하지 않는다.
    pub job_key: String,
    /// 촬영 시점 + NFR-003 hard max. **순서 결정과 관측에만 쓴다.**
    pub deadline_micros: u64,
    pub coordinates: JobCoordinates,
}

/// darktable pixelpipe 품질. `--hq` 인자 하나로 표현된다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayRenderQuality {
    /// 먼저 축소하고 처리한다. 빠르고 덜 선명하다 (Story 7.4 proxy lane).
    Fast,
    /// 전체 해상도로 처리한 뒤 축소한다 (Story 7.6 정밀본 lane).
    High,
}

impl DisplayRenderQuality {
    fn hq_flag(self) -> &'static str {
        match self {
            DisplayRenderQuality::Fast => "false",
            DisplayRenderQuality::High => "true",
        }
    }

    fn stage(self) -> RenderStage {
        match self {
            DisplayRenderQuality::Fast => DISPLAY_PROXY_STAGE,
            DisplayRenderQuality::High => RAW_REFINED_STAGE,
        }
    }
}

/// display lane 렌더 한 건의 진단 span. **어느 것도 KPI 종료점이 아니다.**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayProxyRenderOutcome {
    pub detail: String,
    /// **이제 실제 큐 대기 시간이다.** Story 7.6 이전 값은 승인 게이트의 mutex 시간이며
    /// 대기 시간이 아니었다 — 그 시절 큐는 가득 차면 기다리지 않고 즉시 실패했다.
    pub queue_wait_micros: u64,
    pub render_started_at_micros: u64,
    /// darktable 프로세스가 **종료한** 시각. AC 3의 "프로세스가 종료했거나"가 이 경계다.
    pub process_exited_at_micros: u64,
    /// 스케줄러 구간 전체. 우선순위·deadline·병합·선점 사유를 그대로 싣는다.
    pub scheduler: SchedulerSpans,
}

/// display lane 렌더 요청의 결말. **병합과 취소는 오류가 아니라 정상 결과다.**
#[derive(Debug)]
pub enum DisplayRenderDisposition {
    Rendered(Box<DisplayProxyRenderOutcome>),
    /// 같은 좌표의 렌더가 이미 진행 중이다. 이 호출은 아무것도 만들지 않았다.
    Coalesced {
        job_key: String,
        spans: SchedulerSpans,
    },
    /// 대기 중 또는 실행 중 취소됐다. **staging 산출물은 남지 않는다.**
    Cancelled {
        reason: String,
        spans: SchedulerSpans,
    },
}

/// Story 7.4. proxy lane (`--hq false`).
pub fn render_display_proxy_to_path_in_dir(
    base_dir: &Path,
    spec: &DisplayProxyRenderSpec<'_>,
    now: &dyn Fn() -> u64,
) -> Result<DisplayRenderDisposition, RenderWorkerError> {
    render_display_lane_to_path_in_dir(base_dir, spec, DisplayRenderQuality::Fast, now)
}

/// Story 7.6. RAW 정밀본 lane (`--hq true`).
///
/// **proxy lane과 같은 진입 구조를 쓴다.** 두 번째 렌더 경로를 만들면 한쪽이 조용히 어긋나고,
/// AC 6이 비교하려는 "`--hq` 하나만 다른 두 결과"라는 전제가 무너진다.
pub fn render_raw_refined_display_to_path_in_dir(
    base_dir: &Path,
    spec: &DisplayProxyRenderSpec<'_>,
    now: &dyn Fn() -> u64,
) -> Result<DisplayRenderDisposition, RenderWorkerError> {
    render_display_lane_to_path_in_dir(base_dir, spec, DisplayRenderQuality::High, now)
}

fn render_display_lane_to_path_in_dir(
    base_dir: &Path,
    spec: &DisplayProxyRenderSpec<'_>,
    quality: DisplayRenderQuality,
    now: &dyn Fn() -> u64,
) -> Result<DisplayRenderDisposition, RenderWorkerError> {
    let stage = quality.stage();

    if spec.target_width_px == 0 || spec.target_height_px == 0 {
        return Err(RenderWorkerError {
            reason_code: "display-proxy-target-unmeasured",
            customer_message: stage.customer_message.into(),
            operator_detail:
                "관람 화면이 아직 크기를 보고하지 않아 proxy 목표 크기를 정할 수 없어요.".into(),
        });
    }

    let icc_type = resolve_darktable_icc_type(spec.output_color_space)?;
    let icc_intent = resolve_darktable_icc_intent(spec.icc_intent)?;

    if !(1..=100).contains(&spec.jpeg_quality) {
        return Err(RenderWorkerError {
            reason_code: "display-proxy-quality-out-of-range",
            customer_message: stage.customer_message.into(),
            operator_detail: format!(
                "승인된 proxy JPEG 품질 범위를 벗어났어요: quality={}",
                spec.jpeg_quality
            ),
        });
    }

    let scheduler = scheduler_for(base_dir);
    let lease = match scheduler.admit(
        &RenderJobRequest {
            priority: spec.schedule.priority,
            job_key: spec.schedule.job_key.clone(),
            deadline_micros: spec.schedule.deadline_micros,
            coordinates: spec.schedule.coordinates.clone(),
        },
        now,
    ) {
        SchedulerAdmission::Granted(lease) => *lease,
        SchedulerAdmission::Coalesced { job_key, spans } => {
            return Ok(DisplayRenderDisposition::Coalesced { job_key, spans })
        }
        SchedulerAdmission::Cancelled { spans } => {
            let reason = spans
                .preempted_by
                .clone()
                .unwrap_or_else(|| "unknown".into());

            return Ok(DisplayRenderDisposition::Cancelled { reason, spans });
        }
    };

    if let Some(parent) = spec.output_path.parent() {
        fs::create_dir_all(parent).map_err(|error| RenderWorkerError {
            reason_code: "render-output-dir-unavailable",
            customer_message: stage.customer_message.into(),
            operator_detail: format!("display proxy output dir를 준비하지 못했어요: {error}"),
        })?;
    }

    // 이전 시도의 잔여물이 남아 있으면 그것이 "성공"으로 오인된다.
    let _ = fs::remove_file(spec.output_path);

    let invocation = build_display_proxy_invocation(base_dir, spec, quality, icc_type, icc_intent);
    let render_started_at_micros = now();
    let invocation_result = match run_darktable_invocation(&invocation, stage, Some(lease.token()))
    {
        Ok(result) => result,
        Err(error) if error.reason_code == "render-cancelled" => {
            // 취소된 작업은 중간 산출물을 남기지 않는다. 중간에 죽은 파일이
            // 다음 시도의 "성공"으로 오인되면 안 된다.
            let _ = fs::remove_file(spec.output_path);
            let reason = lease
                .token()
                .reason()
                .unwrap_or_else(|| "cancelled".to_string());
            let mut spans = lease.spans().clone();
            spans.preempted_by = Some(reason.clone());

            return Ok(DisplayRenderDisposition::Cancelled { reason, spans });
        }
        Err(error) => {
            let _ = fs::remove_file(spec.output_path);
            return Err(error);
        }
    };
    // `run_darktable_invocation`은 프로세스가 종료한 뒤에만 반환한다. 여기가 AC 3의 첫 관문이다.
    let process_exited_at_micros = now();

    if let Err(error) = validate_render_output(spec.output_path, stage) {
        let _ = fs::remove_file(spec.output_path);
        return Err(error);
    }

    Ok(DisplayRenderDisposition::Rendered(Box::new(
        DisplayProxyRenderOutcome {
            detail: format!(
                "binary={};source={};widthCap={};heightCap={};upscale=false;hq={};iccType={};iccIntent={};jpegQuality={};priority={};queueWaitMicros={};deadlineMissed={};args={};status={}",
                invocation.binary,
                invocation.binary_source,
                spec.target_width_px,
                spec.target_height_px,
                quality.hq_flag(),
                icc_type,
                icc_intent,
                spec.jpeg_quality,
                lease.spans().priority,
                lease.spans().queue_wait_micros,
                lease.spans().deadline_missed,
                invocation.arguments.join(" "),
                invocation_result.exit_code
            ),
            queue_wait_micros: lease.spans().queue_wait_micros,
            render_started_at_micros,
            process_exited_at_micros,
            scheduler: lease.spans().clone(),
        },
    )))
}

/// 승인된 출력 색공간 → `darktable-cli --icc-type` 토큰.
///
/// **추측하지 않는다.** 알 수 없는 값은 오류이며, 조용히 기본값으로 떨어지지 않는다.
/// 잘못된 토큰을 넘기면 darktable-cli는 렌더 대신 도움말을 출력하고 종료 코드 0으로 끝난다
/// (실측 확인). 그러면 "성공했는데 파일이 없는" 상태가 되어 진단이 어려워진다.
fn resolve_darktable_icc_type(output_color_space: &str) -> Result<&'static str, RenderWorkerError> {
    match output_color_space.trim().to_ascii_lowercase().as_str() {
        "srgb" => Ok("SRGB"),
        "adobergb" | "adobe rgb" => Ok("ADOBERGB"),
        "lin_rec709" | "linear rec709" => Ok("LIN_REC709"),
        "lin_rec2020" | "linear rec2020" => Ok("LIN_REC2020"),
        _ => Err(RenderWorkerError {
            reason_code: "display-proxy-color-space-unsupported",
            customer_message: DISPLAY_PROXY_STAGE.customer_message.into(),
            operator_detail: format!(
                "승인되지 않은 proxy 출력 색공간이에요: outputColorSpace={output_color_space}"
            ),
        }),
    }
}

/// 승인된 렌더링 intent → `darktable-cli --icc-intent` 토큰.
///
/// 실측 확인: `PERCEPTUAL`은 받아들여지고 `INTENT_PERCEPTUAL`은 거부된다.
fn resolve_darktable_icc_intent(icc_intent: &str) -> Result<&'static str, RenderWorkerError> {
    match icc_intent.trim().to_ascii_lowercase().as_str() {
        "perceptual" => Ok("PERCEPTUAL"),
        "relative_colorimetric" | "relative colorimetric" => Ok("RELATIVE_COLORIMETRIC"),
        "saturation" => Ok("SATURATION"),
        "absolute_colorimetric" | "absolute colorimetric" => Ok("ABSOLUTE_COLORIMETRIC"),
        _ => Err(RenderWorkerError {
            reason_code: "display-proxy-icc-intent-unsupported",
            customer_message: DISPLAY_PROXY_STAGE.customer_message.into(),
            operator_detail: format!("승인되지 않은 proxy ICC intent예요: iccIntent={icc_intent}"),
        }),
    }
}

fn build_display_proxy_invocation(
    base_dir: &Path,
    spec: &DisplayProxyRenderSpec<'_>,
    quality: DisplayRenderQuality,
    icc_type: &str,
    icc_intent: &str,
) -> DarktableInvocation {
    // Story 7.6. 정밀본은 **네 번째 worker root**를 쓴다. display-proxy와 `configdir`/`library.db`를
    // 공유하면 P0와 P1이 서로를 막는다.
    let worker_root = base_dir
        .join(".boothy-darktable")
        .join(quality.stage().label);
    let configdir = worker_root.join("config");
    let library = worker_root.join("library.db");
    let binary_resolution = resolve_darktable_cli_binary();

    let arguments = vec![
        spec.source_asset_path.to_string_lossy().replace('\\', "/"),
        spec.xmp_template_path.to_string_lossy().replace('\\', "/"),
        spec.output_path.to_string_lossy().replace('\\', "/"),
        "--width".into(),
        spec.target_width_px.to_string(),
        "--height".into(),
        spec.target_height_px.to_string(),
        // upscaled frame은 PRD NFR-003의 zero 항목이다. source가 작으면 작게 나오고,
        // 그 결과는 display fit gate에서 정직하게 거부된다.
        "--upscale".into(),
        "false".into(),
        // **proxy lane의 `false`는 Story 7.4의 승인 인자다. 바꾸면 HV-15 `Go`가 흔들린다.**
        // 정밀본 lane만 `true`를 쓴다 — 그것이 두 tier의 유일한 차이다.
        "--hq".into(),
        quality.hq_flag().into(),
        "--apply-custom-presets".into(),
        DARKTABLE_APPLY_CUSTOM_PRESETS_DISABLED.into(),
        "--icc-type".into(),
        icc_type.into(),
        "--icc-intent".into(),
        icc_intent.into(),
        "--out-ext".into(),
        "jpg".into(),
        // `--core` 뒤는 전부 darktable core로 넘어간다. 반드시 마지막이다.
        "--core".into(),
        "--configdir".into(),
        configdir.to_string_lossy().replace('\\', "/"),
        "--library".into(),
        library.to_string_lossy().replace('\\', "/"),
        "--conf".into(),
        format!("{DARKTABLE_JPEG_QUALITY_CONF_KEY}={}", spec.jpeg_quality),
    ];

    DarktableInvocation {
        binary: binary_resolution.binary,
        binary_source: binary_resolution.source,
        render_source_kind: if spec.source_is_raw_original {
            PreviewRenderSourceKind::RawOriginal
        } else {
            PreviewRenderSourceKind::FastPreviewRaster
        },
        arguments,
        working_directory: base_dir.to_path_buf(),
    }
}

pub fn schedule_preview_renderer_warmup_in_dir(
    base_dir: &Path,
    session_id: &str,
    preset_id: &str,
    preset_version: &str,
) {
    let warmup_key = build_preview_render_warmup_key(session_id, preset_id, preset_version);
    if !try_mark_preview_render_warmup_in_flight(&warmup_key) {
        return;
    }

    let base_dir = base_dir.to_path_buf();
    let session_id = session_id.to_string();
    let preset_id = preset_id.to_string();
    let preset_version = preset_version.to_string();

    thread::spawn(move || {
        let result =
            run_preview_renderer_warmup_in_dir(&base_dir, &session_id, &preset_id, &preset_version);
        if let Err(error) = result {
            log::warn!(
                "preview_renderer_warmup_failed session={} preset_id={} published_version={} code={} detail={}",
                session_id,
                preset_id,
                preset_version,
                error.reason_code,
                error.operator_detail
            );
        }
        clear_preview_render_warmup_in_flight(&warmup_key);
    });
}

pub fn log_render_failure_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
    request_id: Option<&str>,
    intent: RenderIntent,
    reason_code: &str,
) {
    let paths = SessionPaths::new(base_dir, session_id);
    let event_name = if reason_code == "render-queue-saturated" {
        render_queue_saturated_event_name(intent)
    } else {
        render_failed_event_name(intent)
    };
    append_render_event(
        &paths,
        capture_id,
        request_id,
        intent,
        event_name,
        Some(reason_code),
        None,
    );
}

pub fn log_render_start_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
    request_id: &str,
    intent: RenderIntent,
) {
    let paths = SessionPaths::new(base_dir, session_id);
    append_render_event(
        &paths,
        capture_id,
        Some(request_id),
        intent,
        render_start_event_name(intent),
        Some("render-start"),
        None,
    );
}

pub fn log_render_ready_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
    request_id: &str,
    intent: RenderIntent,
    detail: &str,
) {
    let paths = SessionPaths::new(base_dir, session_id);
    append_render_event(
        &paths,
        capture_id,
        Some(request_id),
        intent,
        render_ready_event_name(intent),
        Some(match intent {
            RenderIntent::Preview => "preview-ready",
            RenderIntent::Final => "final-ready",
        }),
        Some(detail),
    );
}

pub fn is_valid_render_preview_asset(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("jpg") | Some("jpeg") => has_jpeg_signature(path),
        Some("png") => has_png_signature(path),
        Some("webp") | Some("gif") | Some("bmp") => true,
        _ => false,
    }
}

fn safe_render_failure_message(intent: RenderIntent) -> String {
    stage_for_intent(intent).customer_message.into()
}

fn render_stage_label(intent: RenderIntent) -> &'static str {
    stage_for_intent(intent).label
}

/// darktable 실행 한 건의 격리 경계와 고객 안전 문구.
///
/// Story 7.4가 도입했다. display proxy는 `renders/previews` / `renders/finals` 어느 쪽도
/// 만들지 않으므로 `RenderIntent`로 표현되지 않는다. 그렇다고 프로세스 실행·stderr 로그·
/// 산출물 검증 코드를 두 벌 만들 이유는 없어서, 실행에 필요한 것만 이 타입으로 뽑았다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RenderStage {
    /// `.boothy-darktable/<label>/` 아래로 config·library·로그가 격리된다.
    label: &'static str,
    /// 고객 화면에 보여도 안전한 문구. 경로·엔진 이름을 담지 않는다.
    customer_message: &'static str,
}

const PREVIEW_STAGE: RenderStage = RenderStage {
    label: "preview",
    customer_message: "확인용 사진을 아직 준비하지 못했어요. 가까운 직원에게 알려 주세요.",
};
const FINAL_STAGE: RenderStage = RenderStage {
    label: "final",
    customer_message: "결과 사진을 아직 마무리하지 못했어요. 가까운 직원에게 알려 주세요.",
};
/// Story 7.4. **preview/final과 `configdir`·`library.db`를 공유하지 않는다.**
/// 공유하면 동시 실행이 서로를 막아 고객의 첫 화면이 썸네일 렌더 뒤에서 기다린다.
const DISPLAY_PROXY_STAGE: RenderStage = RenderStage {
    label: "display-proxy",
    customer_message: "확인용 사진을 아직 준비하지 못했어요. 가까운 직원에게 알려 주세요.",
};
/// Story 7.6. **네 번째 worker root.** display-proxy와 `configdir`/`library.db`를 공유하면
/// P0와 P1이 서로를 막는다 — 정밀본을 만드는 동안 고객의 첫 화면이 기다리게 된다.
///
/// 고객 문구는 proxy lane과 같다. **정밀본 실패는 관람 화면에 노출되지 않으므로**
/// 이 문구는 운영자 화면으로만 투영된다. NFR-001의 copy budget에 새 문구를 더하지 않는다.
const RAW_REFINED_STAGE: RenderStage = RenderStage {
    label: "raw-refined",
    customer_message: "확인용 사진을 아직 준비하지 못했어요. 가까운 직원에게 알려 주세요.",
};

fn stage_for_intent(intent: RenderIntent) -> RenderStage {
    match intent {
        RenderIntent::Preview => PREVIEW_STAGE,
        RenderIntent::Final => FINAL_STAGE,
    }
}

fn render_invocation_detail_with_source(
    intent: RenderIntent,
    source_kind: Option<PreviewRenderSourceKind>,
) -> String {
    match intent {
        RenderIntent::Preview => {
            let source_kind = source_kind.unwrap_or(PreviewRenderSourceKind::RawOriginal);
            let source_asset = match source_kind {
                PreviewRenderSourceKind::RawOriginal => "raw-original",
                PreviewRenderSourceKind::FastPreviewRaster => "fast-preview-raster",
            };
            let (width_cap, height_cap) = preview_render_dimensions(source_kind);

            format!(
                "widthCap={width_cap};heightCap={height_cap};hq=false;sourceAsset={source_asset}"
            )
        }
        RenderIntent::Final => {
            "widthCap=full;heightCap=full;hq=true;sourceAsset=raw-original".into()
        }
    }
}

fn build_preview_render_warmup_key(
    session_id: &str,
    preset_id: &str,
    preset_version: &str,
) -> String {
    format!("{session_id}:{preset_id}:{preset_version}")
}

fn try_mark_preview_render_warmup_in_flight(key: &str) -> bool {
    let Ok(mut in_flight) = PREVIEW_RENDER_WARMUP_IN_FLIGHT.lock() else {
        return false;
    };

    in_flight.insert(key.to_string())
}

fn clear_preview_render_warmup_in_flight(key: &str) {
    if let Ok(mut in_flight) = PREVIEW_RENDER_WARMUP_IN_FLIGHT.lock() {
        in_flight.remove(key);
    }
}

fn run_preview_renderer_warmup_in_dir(
    base_dir: &Path,
    session_id: &str,
    preset_id: &str,
    preset_version: &str,
) -> Result<(), RenderWorkerError> {
    // Story 7.6. **warm-up만은 줄을 서지 않는다.** 실패해도 제품 진실에 영향이 없는 작업이
    // 고객의 첫 화면 뒤에 darktable 실행을 하나 더 붙이면 순손해다. 오늘의 동작을 유지한다.
    let scheduler = scheduler_for(base_dir);
    let Some(queue_lease) = scheduler.try_admit_idle_background(
        &RenderJobRequest {
            priority: JobPriority::P2Background,
            job_key: background_job_key(session_id, "warmup", "preview-renderer-warmup"),
            deadline_micros: u64::MAX,
            coordinates: JobCoordinates {
                session_id: session_id.to_string(),
                request_id: String::new(),
                capture_id: None,
                capture_order: None,
                viewer_epoch: 0,
            },
        },
        &current_monotonic_micros,
    ) else {
        log::info!(
            "preview_renderer_warmup_skipped session={} preset_id={} published_version={} reason=render-queue-busy",
            session_id,
            preset_id,
            preset_version
        );
        return Ok(());
    };

    let bundle =
        resolve_runtime_bundle_in_dir(base_dir, preset_id, preset_version, RenderIntent::Preview)?;
    let warmup_source_path = ensure_preview_renderer_warmup_source(base_dir)?;
    let warmup_output_path = base_dir
        .join(".boothy-darktable")
        .join("preview")
        .join("warmup")
        .join("preview-renderer-warmup.jpg");

    if let Some(parent) = warmup_output_path.parent() {
        fs::create_dir_all(parent).map_err(|error| RenderWorkerError {
            reason_code: "render-warmup-dir-unavailable",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!(
                "preview renderer warm-up output dir를 준비하지 못했어요: {error}"
            ),
        })?;
    }

    let _ = fs::remove_file(&warmup_output_path);
    let invocation = build_darktable_invocation_from_source(
        base_dir,
        &bundle.darktable_version,
        &bundle.xmp_template_path,
        &warmup_source_path,
        &warmup_output_path,
        RenderIntent::Preview,
        PreviewRenderSourceKind::FastPreviewRaster,
    );
    log::info!(
        "preview_renderer_warmup_started session={} preset_id={} published_version={} binary={} source={}",
        session_id,
        preset_id,
        preset_version,
        invocation.binary,
        invocation.binary_source
    );

    let result = run_darktable_invocation(&invocation, PREVIEW_STAGE, Some(queue_lease.token()));
    match result {
        Ok(_) => {
            let _ = validate_render_output(&warmup_output_path, PREVIEW_STAGE);
            let _ = fs::remove_file(&warmup_output_path);
            log::info!(
                "preview_renderer_warmup_completed session={} preset_id={} published_version={}",
                session_id,
                preset_id,
                preset_version
            );
            Ok(())
        }
        Err(error) => {
            let _ = fs::remove_file(&warmup_output_path);
            Err(RenderWorkerError {
                reason_code: error.reason_code,
                customer_message: error.customer_message,
                operator_detail: format!(
                    "preview renderer warm-up failed: {}",
                    error.operator_detail
                ),
            })
        }
    }
}

fn ensure_preview_renderer_warmup_source(base_dir: &Path) -> Result<PathBuf, RenderWorkerError> {
    let warmup_source_path = base_dir
        .join(".boothy-darktable")
        .join("preview")
        .join("warmup")
        .join("preview-renderer-warmup-source.png");
    if warmup_source_path.is_file() {
        return Ok(warmup_source_path);
    }

    if let Some(parent) = warmup_source_path.parent() {
        fs::create_dir_all(parent).map_err(|error| RenderWorkerError {
            reason_code: "render-warmup-dir-unavailable",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!(
                "preview renderer warm-up source dir를 준비하지 못했어요: {error}"
            ),
        })?;
    }

    fs::write(&warmup_source_path, PREVIEW_RENDER_WARMUP_INPUT_PNG).map_err(|error| {
        RenderWorkerError {
            reason_code: "render-warmup-source-write-failed",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!("preview renderer warm-up source를 쓰지 못했어요: {error}"),
        }
    })?;

    Ok(warmup_source_path)
}

fn render_start_event_name(intent: RenderIntent) -> &'static str {
    match intent {
        RenderIntent::Preview => "preview-render-start",
        RenderIntent::Final => "final-render-start",
    }
}

fn render_ready_event_name(intent: RenderIntent) -> &'static str {
    match intent {
        RenderIntent::Preview => "preview-render-ready",
        RenderIntent::Final => "final-render-ready",
    }
}

fn render_failed_event_name(intent: RenderIntent) -> &'static str {
    match intent {
        RenderIntent::Preview => "preview-render-failed",
        RenderIntent::Final => "final-render-failed",
    }
}

fn render_queue_saturated_event_name(intent: RenderIntent) -> &'static str {
    match intent {
        RenderIntent::Preview => "preview-render-queue-saturated",
        RenderIntent::Final => "final-render-queue-saturated",
    }
}

fn canonical_render_output_path(
    paths: &SessionPaths,
    capture_id: &str,
    intent: RenderIntent,
) -> PathBuf {
    match intent {
        RenderIntent::Preview => paths.renders_previews_dir.join(format!("{capture_id}.jpg")),
        RenderIntent::Final => paths.renders_finals_dir.join(format!("{capture_id}.jpg")),
    }
}

fn build_staging_render_output_path(
    output_root: &Path,
    capture_id: &str,
    intent: RenderIntent,
) -> PathBuf {
    let stage_label = match intent {
        RenderIntent::Preview => "preview-rendering",
        RenderIntent::Final => "final-rendering",
    };

    output_root.join(format!("{capture_id}.{stage_label}.jpg"))
}

fn promote_render_output(
    staging_output_path: &Path,
    output_path: &Path,
    intent: RenderIntent,
) -> Result<(), RenderWorkerError> {
    let backup_path = if output_path.exists() {
        let backup_path = build_replacement_backup_path(output_path, intent);
        fs::rename(output_path, &backup_path).map_err(|error| RenderWorkerError {
            reason_code: "render-output-overwrite-failed",
            customer_message: safe_render_failure_message(intent),
            operator_detail: format!(
                "기존 render output을 백업하지 못했어요: path={} backup={} error={error}",
                output_path.to_string_lossy(),
                backup_path.to_string_lossy()
            ),
        })?;
        Some(backup_path)
    } else {
        None
    };

    if let Err(error) = fs::rename(staging_output_path, output_path) {
        if let Some(backup_path) = backup_path.as_ref() {
            let _ = restore_replaced_output(backup_path, output_path);
        }

        return Err(RenderWorkerError {
            reason_code: "render-output-promote-failed",
            customer_message: safe_render_failure_message(intent),
            operator_detail: format!(
                "staging render output을 최종 경로로 승격하지 못했어요: from={} to={} error={error}",
                staging_output_path.to_string_lossy(),
                output_path.to_string_lossy()
            ),
        });
    }

    if let Some(backup_path) = backup_path.as_ref() {
        let _ = fs::remove_file(backup_path);
    }

    Ok(())
}

pub fn promote_preview_render_output(
    staging_output_path: &Path,
    output_path: &Path,
) -> Result<(), RenderWorkerError> {
    promote_render_output(staging_output_path, output_path, RenderIntent::Preview)
}

fn build_replacement_backup_path(output_path: &Path, intent: RenderIntent) -> PathBuf {
    let stage_label = match intent {
        RenderIntent::Preview => "preview-backup",
        RenderIntent::Final => "final-backup",
    };
    let parent = output_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = output_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("render-output");
    let extension = output_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("jpg");

    parent.join(format!("{stem}.{stage_label}.{extension}"))
}

fn restore_replaced_output(backup_path: &Path, output_path: &Path) -> Result<(), std::io::Error> {
    if output_path.exists() {
        fs::remove_file(output_path)?;
    }

    fs::rename(backup_path, output_path)
}

fn current_time_ms() -> Result<u64, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "render worker가 시스템 시계를 읽지 못했어요.".to_string())?
        .as_millis() as u64)
}

fn append_render_event(
    paths: &SessionPaths,
    capture_id: &str,
    request_id: Option<&str>,
    intent: RenderIntent,
    event: &str,
    reason_code: Option<&str>,
    detail: Option<&str>,
) {
    let occurred_at = match crate::session::session_manifest::current_timestamp(SystemTime::now()) {
        Ok(value) => value,
        Err(_) => return,
    };
    let _ = fs::create_dir_all(&paths.diagnostics_dir);
    let log_path = paths.diagnostics_dir.join("timing-events.log");
    let mut file = match OpenOptions::new().create(true).append(true).open(log_path) {
        Ok(file) => file,
        Err(_) => return,
    };
    let stage = match intent {
        RenderIntent::Preview => "preview",
        RenderIntent::Final => "final",
    };
    let request_id = request_id.unwrap_or("none");
    let reason_code = reason_code.unwrap_or("none");
    let detail = detail.unwrap_or("none");
    let _ = writeln!(
        file,
        "{occurred_at}\tsession={}\tcapture={capture_id}\trequest={request_id}\tevent={event}\tstage={stage}\treason={reason_code}\tdetail={detail}",
        paths
            .session_root
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default()
    );
}

struct DarktableInvocation {
    binary: String,
    binary_source: &'static str,
    render_source_kind: PreviewRenderSourceKind,
    arguments: Vec<String>,
    working_directory: PathBuf,
}

struct DarktableInvocationResult {
    exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DarktableBinaryResolution {
    pub binary: String,
    pub source: &'static str,
}

fn resolve_runtime_bundle_in_dir(
    base_dir: &Path,
    preset_id: &str,
    preset_version: &str,
    intent: RenderIntent,
) -> Result<PublishedPresetRuntimeBundle, RenderWorkerError> {
    let catalog_root = resolve_published_preset_catalog_dir(base_dir);
    let bundle = find_published_preset_runtime_bundle(&catalog_root, preset_id, preset_version)
        .ok_or_else(|| RenderWorkerError {
            reason_code: "bundle-resolution-failed",
            customer_message: safe_render_failure_message(intent),
            operator_detail: format!(
                "capture-bound bundle을 찾지 못했어요: presetId={preset_id}, publishedVersion={preset_version}"
            ),
        })?;

    if bundle.darktable_version != PINNED_DARKTABLE_VERSION {
        return Err(RenderWorkerError {
            reason_code: "darktable-version-mismatch",
            customer_message: safe_render_failure_message(intent),
            operator_detail: format!(
                "pinned darktable version과 bundle metadata가 다릅니다: expected={PINNED_DARKTABLE_VERSION}, actual={}",
                bundle.darktable_version
            ),
        });
    }

    Ok(bundle)
}

fn build_darktable_invocation(
    base_dir: &Path,
    _darktable_version: &str,
    xmp_template_path: &Path,
    capture: &SessionCaptureRecord,
    paths: &SessionPaths,
    output_path: &Path,
    intent: RenderIntent,
    forced_source_kind: Option<PreviewRenderSourceKind>,
) -> DarktableInvocation {
    let render_source = resolve_preview_render_source(capture, paths, intent, forced_source_kind);
    build_darktable_invocation_from_source(
        base_dir,
        _darktable_version,
        xmp_template_path,
        Path::new(&render_source.asset_path),
        output_path,
        intent,
        render_source.kind,
    )
}

fn build_darktable_invocation_from_source(
    base_dir: &Path,
    _darktable_version: &str,
    xmp_template_path: &Path,
    source_asset_path: &Path,
    output_path: &Path,
    intent: RenderIntent,
    render_source_kind: PreviewRenderSourceKind,
) -> DarktableInvocation {
    let mode = match intent {
        RenderIntent::Preview => "preview",
        RenderIntent::Final => "final",
    };
    let worker_root = base_dir.join(".boothy-darktable").join(mode);
    let configdir = worker_root.join("config");
    let library = worker_root.join("library.db");
    let hq_flag = match intent {
        RenderIntent::Preview => "false",
        RenderIntent::Final => "true",
    };
    let binary_resolution = resolve_darktable_cli_binary();
    let mut arguments = vec![
        source_asset_path.to_string_lossy().replace('\\', "/"),
        xmp_template_path.to_string_lossy().replace('\\', "/"),
        output_path.to_string_lossy().replace('\\', "/"),
        "--hq".into(),
        hq_flag.into(),
    ];

    if matches!(intent, RenderIntent::Preview) {
        arguments.push("--apply-custom-presets".into());
        arguments.push(DARKTABLE_APPLY_CUSTOM_PRESETS_DISABLED.into());
        let (width_cap, height_cap) = preview_render_dimensions(render_source_kind);
        arguments.push("--width".into());
        arguments.push(width_cap.to_string());
        arguments.push("--height".into());
        arguments.push(height_cap.to_string());
    }

    arguments.extend([
        "--core".into(),
        "--configdir".into(),
        configdir.to_string_lossy().replace('\\', "/"),
        "--library".into(),
        library.to_string_lossy().replace('\\', "/"),
    ]);

    DarktableInvocation {
        binary: binary_resolution.binary,
        binary_source: binary_resolution.source,
        render_source_kind,
        arguments,
        working_directory: base_dir.to_path_buf(),
    }
}

fn preview_render_dimensions(source_kind: PreviewRenderSourceKind) -> (u32, u32) {
    match source_kind {
        PreviewRenderSourceKind::RawOriginal => {
            (RAW_PREVIEW_MAX_WIDTH_PX, RAW_PREVIEW_MAX_HEIGHT_PX)
        }
        PreviewRenderSourceKind::FastPreviewRaster => (
            FAST_PREVIEW_RENDER_MAX_WIDTH_PX,
            FAST_PREVIEW_RENDER_MAX_HEIGHT_PX,
        ),
    }
}

struct PreviewRenderSource {
    asset_path: String,
    kind: PreviewRenderSourceKind,
}

fn resolve_preview_render_source(
    capture: &SessionCaptureRecord,
    paths: &SessionPaths,
    intent: RenderIntent,
    forced_source_kind: Option<PreviewRenderSourceKind>,
) -> PreviewRenderSource {
    if matches!(
        forced_source_kind,
        Some(PreviewRenderSourceKind::RawOriginal)
    ) || matches!(intent, RenderIntent::Final)
    {
        return PreviewRenderSource {
            asset_path: capture.raw.asset_path.clone(),
            kind: PreviewRenderSourceKind::RawOriginal,
        };
    }

    if matches!(intent, RenderIntent::Preview) {
        if let Some(preview_asset_path) = capture.preview.asset_path.as_deref() {
            let preview_asset = Path::new(preview_asset_path);

            if is_session_scoped_asset_path(&paths.session_root, preview_asset)
                && is_valid_render_preview_asset(preview_asset)
            {
                return PreviewRenderSource {
                    asset_path: preview_asset_path.to_string(),
                    kind: PreviewRenderSourceKind::FastPreviewRaster,
                };
            }
        }

        let canonical_preview_asset = paths
            .renders_previews_dir
            .join(format!("{}.jpg", capture.capture_id));
        if is_valid_render_preview_asset(&canonical_preview_asset) {
            return PreviewRenderSource {
                asset_path: canonical_preview_asset.to_string_lossy().into_owned(),
                kind: PreviewRenderSourceKind::FastPreviewRaster,
            };
        }
    }

    PreviewRenderSource {
        asset_path: capture.raw.asset_path.clone(),
        kind: PreviewRenderSourceKind::RawOriginal,
    }
}

fn is_session_scoped_asset_path(session_root: &Path, candidate_path: &Path) -> bool {
    let normalized_candidate = normalize_path(candidate_path);
    let Some(normalized_session_root) = canonicalize_existing_root(session_root) else {
        return false;
    };

    normalized_candidate == normalized_session_root
        || normalized_candidate.starts_with(&(normalized_session_root + "/"))
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .to_lowercase()
        .trim_start_matches("//?/")
        .trim_end_matches('/')
        .to_string()
}

fn canonicalize_existing_root(path: &Path) -> Option<String> {
    fs::canonicalize(path)
        .ok()
        .map(|resolved| normalize_path(&resolved))
}

pub fn resolve_darktable_cli_binary() -> DarktableBinaryResolution {
    let env_override = std::env::var(DARKTABLE_CLI_BIN_ENV).ok();
    let candidates = darktable_cli_binary_candidates();
    resolve_darktable_cli_binary_with_candidates(env_override.as_deref(), &candidates)
}

fn resolve_darktable_cli_binary_with_candidates(
    env_override: Option<&str>,
    candidates: &[(&'static str, PathBuf)],
) -> DarktableBinaryResolution {
    if let Some(value) = env_override
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return DarktableBinaryResolution {
            binary: value.to_string(),
            source: "env-override",
        };
    }

    for (source, candidate) in candidates {
        if candidate.is_file() {
            return DarktableBinaryResolution {
                binary: candidate.to_string_lossy().into_owned(),
                source,
            };
        }
    }

    DarktableBinaryResolution {
        binary: "darktable-cli".into(),
        source: "path",
    }
}

/// Story 7.7. 설치본이 동봉한 darktable 트리에서 **반드시 함께 있어야 하는** 항목.
///
/// `darktable-cli.exe` 하나만 있어도 실행은 시작되지만 렌더가 실패한다.
/// `lib/`와 `share/darktable/`이 형제로 있어야 한다.
pub const BUNDLED_DARKTABLE_REQUIRED_ENTRIES: [&str; 3] =
    ["bin/darktable-cli.exe", "lib", "share/darktable"];

/// 설치본에 동봉된 darktable 트리 루트. `<현재 실행 파일 디렉터리>/darktable`.
pub fn bundled_darktable_root() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .map(|dir| dir.join("darktable"))
}

/// 트리에서 빠진 항목. **비어 있어야 정상이다.**
pub fn missing_bundled_darktable_entries(root: &Path) -> Vec<&'static str> {
    BUNDLED_DARKTABLE_REQUIRED_ENTRIES
        .into_iter()
        .filter(|entry| {
            let candidate = entry
                .split('/')
                .fold(root.to_path_buf(), |accumulated, segment| {
                    accumulated.join(segment)
                });

            !candidate.exists()
        })
        .collect()
}

fn darktable_cli_binary_candidates() -> Vec<(&'static str, PathBuf)> {
    let mut candidates = Vec::new();

    if !cfg!(windows) {
        return candidates;
    }

    // Story 7.7. **번들 트리가 가장 먼저다.**
    //
    // 동봉만 하고 순서를 그대로 두면 "부스 PC에 이미 깔린 아무 버전의 darktable"이
    // 고객 화면을 만든다. 그 순간 5.4.1 핀은 문서에만 남는다.
    // 기존 후보는 지우지 않는다 — 개발자 PC에는 번들 트리가 없다.
    push_darktable_cli_candidate(
        &mut candidates,
        "bundled-resource",
        bundled_darktable_root().map(|root| root.join("bin").join("darktable-cli.exe")),
    );
    push_darktable_cli_candidate(
        &mut candidates,
        "program-files-bin",
        std::env::var_os("ProgramFiles")
            .map(PathBuf::from)
            .map(|root| root.join("darktable").join("bin").join("darktable-cli.exe")),
    );
    push_darktable_cli_candidate(
        &mut candidates,
        "program-w6432-bin",
        std::env::var_os("ProgramW6432")
            .map(PathBuf::from)
            .map(|root| root.join("darktable").join("bin").join("darktable-cli.exe")),
    );
    push_darktable_cli_candidate(
        &mut candidates,
        "localappdata-programs-bin",
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|root| {
                root.join("Programs")
                    .join("darktable")
                    .join("bin")
                    .join("darktable-cli.exe")
            }),
    );

    candidates
}

fn push_darktable_cli_candidate(
    candidates: &mut Vec<(&'static str, PathBuf)>,
    source: &'static str,
    candidate: Option<PathBuf>,
) {
    let Some(candidate) = candidate else {
        return;
    };

    if candidates
        .iter()
        .any(|(_, existing)| *existing == candidate)
    {
        return;
    }

    candidates.push((source, candidate));
}

/// Story 7.6. **자식의 자식까지 종료한 결과.** 한 벌만 만든다 — 취소와 timeout이 같은 코드를 쓴다.
///
/// 오늘의 `child.kill()`은 직계 자식만 종료한다. AC 1이 요구하는 것은 tree다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessTreeTermination {
    /// `taskkill` 실행 결과. `exit=<code>` / `spawn-failed` / `unavailable` 중 하나다.
    /// **실패를 무시하지 않는다.** 반환 코드를 그대로 기록한다.
    pub kill_outcome: String,
    /// 폴백 `child.kill()`을 실제로 썼는가.
    pub used_direct_kill_fallback: bool,
    /// 종료 뒤에도 남아 있는 것으로 관측된 자손 프로세스 수. **HV-17A는 `Some(0)`을 요구한다.**
    /// `None`은 CIM probe 실패이며 0과 구분한다.
    pub orphan_count: Option<u32>,
    pub requested_at_micros: u64,
    pub completed_at_micros: u64,
    /// HV-17 evidence mode가 명시됐을 때 생성한 구조화 record. 일반 제품 실행은 `None`이다.
    pub evidence_record_path: Option<PathBuf>,
}

impl ProcessTreeTermination {
    pub fn latency_micros(&self) -> u64 {
        self.completed_at_micros
            .saturating_sub(self.requested_at_micros)
    }

    pub fn as_log_detail(&self) -> String {
        let orphan_count = self
            .orphan_count
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unavailable".to_string());
        format!(
            "killOutcome={};directKillFallback={};orphanCount={};latencyMicros={};evidenceRecord={}",
            self.kill_outcome,
            self.used_direct_kill_fallback,
            orphan_count,
            self.latency_micros(),
            self.evidence_record_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_else(|| "disabled".to_string())
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TaskkillTranscript {
    command: String,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn hv17_evidence_root() -> Option<PathBuf> {
    let raw = std::env::var_os(HV17_EVIDENCE_ROOT_ENV)?;
    validated_hv17_evidence_root(PathBuf::from(raw))
}

fn validated_hv17_evidence_root(root: PathBuf) -> Option<PathBuf> {
    let run_name = root.file_name()?.to_string_lossy().to_ascii_lowercase();

    // 우발적으로 session 또는 사용자 폴더에 원문 로그를 만들지 않는다.
    (root.is_absolute() && run_name.contains("hv17")).then_some(root)
}

fn safe_evidence_label(reason: &str) -> String {
    let label = reason
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let trimmed = label.trim_matches('-');
    if trimmed.is_empty() {
        "unspecified".to_string()
    } else {
        trimmed.to_string()
    }
}

fn hv17_round_label(reason: &str) -> &str {
    match reason {
        "request-forgotten" => "delete",
        "superseded-by-newer-capture" => "newer-capture",
        other => other,
    }
}

fn persist_process_tree_evidence_in_dir(
    evidence_root: &Path,
    reason: &str,
    pid: u32,
    termination: &ProcessTreeTermination,
    transcript: &TaskkillTranscript,
) -> std::io::Result<PathBuf> {
    let round = hv17_round_label(reason);
    let label = safe_evidence_label(round);
    let suffix = format!("{}-{}-pid{}", label, termination.requested_at_micros, pid);
    let log_relative = PathBuf::from("logs").join(format!("taskkill-{suffix}.log"));
    let record_relative = PathBuf::from("scheduler")
        .join("process-tree")
        .join(format!("{suffix}.json"));
    let log_path = evidence_root.join(&log_relative);
    let record_path = evidence_root.join(&record_relative);
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(parent) = record_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut raw_log = format!(
        "command: {}\nkillOutcome: {}\n--- stdout ---\n",
        transcript.command, termination.kill_outcome
    )
    .into_bytes();
    raw_log.extend_from_slice(&transcript.stdout);
    raw_log.extend_from_slice(b"\n--- stderr ---\n");
    raw_log.extend_from_slice(&transcript.stderr);
    raw_log.push(b'\n');
    fs::write(&log_path, raw_log)?;
    let record = serde_json::json!({
        "schemaVersion": "hv17-process-tree-termination/v1",
        "round": round,
        "cancellationReason": reason,
        "processId": pid,
        "cancelRequestedAtMicros": termination.requested_at_micros,
        "cancelCompletedAtMicros": termination.completed_at_micros,
        "killOutcome": termination.kill_outcome,
        "directKillFallback": termination.used_direct_kill_fallback,
        "cancelOrphanCount": termination.orphan_count,
        "orphanProbeAvailable": termination.orphan_count.is_some(),
        "taskkillLogPath": log_relative.to_string_lossy().replace('\\', "/"),
    });
    fs::write(&record_path, serde_json::to_vec_pretty(&record)?)?;
    Ok(record_path)
}

/// Windows 내장 `taskkill`의 절대 경로.
///
/// **`PATH`에 의존하지 않는다.** 의존하면 다른 `taskkill.exe`가 잡힐 수 있고,
/// 취소 경로에서 그것은 "종료했다고 믿었는데 살아 있는" 상태를 만든다.
fn taskkill_binary_path() -> PathBuf {
    let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());

    Path::new(&system_root)
        .join("System32")
        .join("taskkill.exe")
}

/// 자식과 그 자손을 전부 종료한다.
///
/// **새 Rust crate를 추가하지 않는다.** Story 7.7의 offline 설치 인벤토리가 그대로 유지되어야
/// 하며, Story 7.3이 LibRaw를 뺀 것과 같은 이유다. Job Object(`windows-sys`) 방식은
/// `Cargo.toml` 직접 의존성을 늘리므로 이 Story에서는 하지 않는다.
pub fn terminate_process_tree(child: &mut Child, now: &dyn Fn() -> u64) -> ProcessTreeTermination {
    terminate_process_tree_for_reason(child, now, "unspecified")
}

fn terminate_process_tree_for_reason(
    child: &mut Child,
    now: &dyn Fn() -> u64,
    reason: &str,
) -> ProcessTreeTermination {
    let evidence_root = hv17_evidence_root();
    terminate_process_tree_for_reason_in_evidence_root(child, now, reason, evidence_root.as_deref())
}

fn terminate_process_tree_for_reason_in_evidence_root(
    child: &mut Child,
    now: &dyn Fn() -> u64,
    reason: &str,
    evidence_root: Option<&Path>,
) -> ProcessTreeTermination {
    let requested_at_micros = now();
    let pid = child.id();
    // 부모를 죽인 뒤에는 살아남은 손자의 ParentProcessId가 이미 종료된 중간 PID를 가리킨다.
    // 따라서 tree 전체 PID를 종료 전에 고정하고, 종료 뒤 그 PID들이 남았는지 확인한다.
    let descendant_pids = current_descendant_process_ids(pid);
    let mut used_direct_kill_fallback = false;

    let taskkill_path = taskkill_binary_path();
    let taskkill_args = ["/T", "/F", "/PID", &pid.to_string()];
    let (kill_outcome, transcript) = match Command::new(&taskkill_path)
        .args(taskkill_args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(output) => (
            format!("exit={}", output.status.code().unwrap_or(-1)),
            TaskkillTranscript {
                command: format!(
                    "{} {}",
                    taskkill_path.to_string_lossy(),
                    taskkill_args.join(" ")
                ),
                stdout: output.stdout,
                stderr: output.stderr,
            },
        ),
        Err(error) => {
            used_direct_kill_fallback = true;
            (
                format!("spawn-failed:{}", error.kind()),
                TaskkillTranscript {
                    command: format!(
                        "{} {}",
                        taskkill_path.to_string_lossy(),
                        taskkill_args.join(" ")
                    ),
                    stdout: Vec::new(),
                    stderr: error.to_string().into_bytes(),
                },
            )
        }
    };

    // taskkill이 실패했으면 직계 자식만이라도 확실히 죽인다. 실패를 조용히 넘기지 않는다.
    if used_direct_kill_fallback || !kill_outcome.ends_with("=0") {
        used_direct_kill_fallback = true;
        let _ = child.kill();
    }

    let _ = child.wait();
    let orphan_count = descendant_pids
        .as_deref()
        .and_then(count_surviving_processes);

    let mut termination = ProcessTreeTermination {
        kill_outcome,
        used_direct_kill_fallback,
        orphan_count,
        requested_at_micros,
        completed_at_micros: now(),
        evidence_record_path: None,
    };
    if let Some(root) = evidence_root {
        match persist_process_tree_evidence_in_dir(root, reason, pid, &termination, &transcript) {
            Ok(path) => termination.evidence_record_path = Some(path),
            Err(error) => log::warn!(
                "hv17_process_tree_evidence_write_failed reason={} root={} error={error}",
                reason,
                root.to_string_lossy()
            ),
        }
    }
    termination
}

/// 현재 Windows process snapshot을 `(pid, parent_pid)`로 읽는다.
///
/// probe 실패는 `None`이다. **세지 못한 것과 0은 다르다.**
fn process_snapshot() -> Option<Vec<(u32, u32)>> {
    if !cfg!(windows) {
        return Some(Vec::new());
    }

    let script = "Get-CimInstance Win32_Process -ErrorAction Stop | ForEach-Object { '{0},{1}' -f $_.ProcessId, $_.ParentProcessId }";
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();

    let output = match output {
        Ok(output) if output.status.success() => output,
        _ => return None,
    };

    let processes = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let (pid, parent_pid) = line.trim().split_once(',')?;
            Some((pid.parse::<u32>().ok()?, parent_pid.parse::<u32>().ok()?))
        })
        .collect::<Vec<_>>();

    Some(processes)
}

fn descendant_process_ids_from_snapshot(root_pid: u32, processes: &[(u32, u32)]) -> Vec<u32> {
    let mut descendants = HashSet::new();
    let mut frontier = vec![root_pid];

    while let Some(parent_pid) = frontier.pop() {
        for (pid, observed_parent_pid) in processes {
            if *pid != root_pid && *observed_parent_pid == parent_pid && descendants.insert(*pid) {
                frontier.push(*pid);
            }
        }
    }

    descendants.into_iter().collect()
}

fn current_descendant_process_ids(root_pid: u32) -> Option<Vec<u32>> {
    if !cfg!(windows) {
        return Some(Vec::new());
    }

    let processes = process_snapshot().or_else(|| {
        log::warn!("render_cancel_orphan_probe_unavailable phase=before pid={root_pid}");
        None
    })?;
    Some(descendant_process_ids_from_snapshot(root_pid, &processes))
}

fn count_surviving_processes(descendant_pids: &[u32]) -> Option<u32> {
    if !cfg!(windows) || descendant_pids.is_empty() {
        return Some(0);
    }

    let processes = process_snapshot().or_else(|| {
        log::warn!("render_cancel_orphan_probe_unavailable phase=after");
        None
    })?;
    let live_pids = processes
        .into_iter()
        .map(|(pid, _)| pid)
        .collect::<HashSet<_>>();

    Some(
        descendant_pids
            .iter()
            .filter(|pid| live_pids.contains(pid))
            .count() as u32,
    )
}

fn run_darktable_invocation(
    invocation: &DarktableInvocation,
    stage: RenderStage,
    cancellation: Option<&CancellationToken>,
) -> Result<DarktableInvocationResult, RenderWorkerError> {
    let stderr_log_path =
        build_darktable_stderr_log_path(&invocation.working_directory, stage.label);
    let stderr_log = open_darktable_stderr_log(&stderr_log_path, stage)?;
    let mut child = Command::new(&invocation.binary)
        .args(&invocation.arguments)
        .current_dir(&invocation.working_directory)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(stderr_log))
        .spawn()
        .map_err(|error| {
            let reason_code = if error.kind() == std::io::ErrorKind::NotFound {
                "render-cli-missing"
            } else {
                "render-process-launch-failed"
            };

            RenderWorkerError {
                reason_code,
                customer_message: stage.customer_message.into(),
                operator_detail: format!(
                    "darktable-cli를 시작하지 못했어요: binary={} source={} error={error}",
                    invocation.binary, invocation.binary_source
                ),
            }
        })?;

    let started_at = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                child.wait().map_err(|error| RenderWorkerError {
                    reason_code: "render-process-wait-failed",
                    customer_message: stage.customer_message.into(),
                    operator_detail: format!("render 프로세스 종료를 회수하지 못했어요: {error}"),
                })?;

                if status.success() {
                    let _ = fs::remove_file(&stderr_log_path);
                    return Ok(DarktableInvocationResult {
                        exit_code: status.code().unwrap_or(0),
                    });
                }

                return Err(RenderWorkerError {
                    reason_code: "render-process-failed",
                    customer_message: stage.customer_message.into(),
                    operator_detail: format!(
                        "darktable-cli가 실패했어요: exitCode={} stderr={} logPath={}",
                        status.code().unwrap_or(-1),
                        read_darktable_stderr_log(&stderr_log_path),
                        stderr_log_path.to_string_lossy()
                    ),
                });
            }
            Ok(None) => {
                // Story 7.6. 취소 신호가 먼저다. **두 벌의 종료 코드를 만들지 않는다** —
                // 취소와 timeout이 같은 process tree 종료를 쓴다.
                if cancellation.is_some_and(CancellationToken::is_cancelled) {
                    let reason = cancellation
                        .and_then(CancellationToken::reason)
                        .unwrap_or_else(|| "cancelled".to_string());
                    let termination = terminate_process_tree_for_reason(
                        &mut child,
                        &current_monotonic_micros,
                        &reason,
                    );
                    log::info!(
                        "render_cancelled stage={} reason={} {}",
                        stage.label,
                        reason,
                        termination.as_log_detail()
                    );

                    return Err(RenderWorkerError {
                        reason_code: "render-cancelled",
                        customer_message: stage.customer_message.into(),
                        operator_detail: format!(
                            "렌더가 취소됐어요: reason={reason} {}",
                            termination.as_log_detail()
                        ),
                    });
                }

                if started_at.elapsed() >= DEFAULT_RENDER_TIMEOUT {
                    let termination = terminate_process_tree_for_reason(
                        &mut child,
                        &current_monotonic_micros,
                        "timeout",
                    );

                    return Err(RenderWorkerError {
                        reason_code: "render-process-timeout",
                        customer_message: stage.customer_message.into(),
                        operator_detail: format!(
                            "darktable-cli가 제한 시간 안에 끝나지 않았어요: timeoutMs={} {} stderr={} logPath={}",
                            DEFAULT_RENDER_TIMEOUT.as_millis(),
                            termination.as_log_detail(),
                            read_darktable_stderr_log(&stderr_log_path),
                            stderr_log_path.to_string_lossy()
                        ),
                    });
                }

                thread::sleep(RENDER_CANCEL_POLL);
            }
            Err(error) => {
                let termination = terminate_process_tree_for_reason(
                    &mut child,
                    &current_monotonic_micros,
                    "process-state-unavailable",
                );
                return Err(RenderWorkerError {
                    reason_code: "render-process-state-unavailable",
                    customer_message: stage.customer_message.into(),
                    operator_detail: format!(
                        "render 프로세스 상태를 확인하지 못했어요: {error} {}",
                        termination.as_log_detail()
                    ),
                });
            }
        }
    }
}

fn build_darktable_stderr_log_path(working_directory: &Path, stage_label: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    working_directory
        .join(".boothy-darktable")
        .join(stage_label)
        .join("logs")
        .join(format!("{stage_label}-stderr-{unique_suffix}.log"))
}

fn open_darktable_stderr_log(
    log_path: &Path,
    stage: RenderStage,
) -> Result<fs::File, RenderWorkerError> {
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).map_err(|error| RenderWorkerError {
            reason_code: "render-log-dir-unavailable",
            customer_message: stage.customer_message.into(),
            operator_detail: format!("render stderr log dir를 준비하지 못했어요: {error}"),
        })?;
    }

    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(log_path)
        .map_err(|error| RenderWorkerError {
            reason_code: "render-log-open-failed",
            customer_message: stage.customer_message.into(),
            operator_detail: format!("render stderr log file을 열지 못했어요: {error}"),
        })
}

fn read_darktable_stderr_log(log_path: &Path) -> String {
    fs::read(log_path)
        .ok()
        .map(|bytes| sanitize_process_output(&bytes))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "none".into())
}

fn validate_render_output(output_path: &Path, stage: RenderStage) -> Result<(), RenderWorkerError> {
    if !output_path.is_file() {
        return Err(RenderWorkerError {
            reason_code: "render-output-missing",
            customer_message: stage.customer_message.into(),
            operator_detail: format!(
                "render 출력 파일이 존재하지 않아요: {}",
                output_path.to_string_lossy()
            ),
        });
    }

    let metadata = fs::metadata(output_path).map_err(|error| RenderWorkerError {
        reason_code: "render-output-unreadable",
        customer_message: stage.customer_message.into(),
        operator_detail: format!("render 출력 파일 metadata를 읽지 못했어요: {error}"),
    })?;

    if metadata.len() == 0 {
        return Err(RenderWorkerError {
            reason_code: "render-output-empty",
            customer_message: stage.customer_message.into(),
            operator_detail: format!(
                "render 출력 파일이 비어 있어요: {}",
                output_path.to_string_lossy()
            ),
        });
    }

    if !is_valid_render_preview_asset(output_path) {
        return Err(RenderWorkerError {
            reason_code: "render-output-invalid",
            customer_message: stage.customer_message.into(),
            operator_detail: format!(
                "render 출력 파일이 유효한 raster 형식이 아니에요: {}",
                output_path.to_string_lossy()
            ),
        });
    }

    Ok(())
}

fn has_jpeg_signature(path: &Path) -> bool {
    let Ok(bytes) = fs::read(path) else {
        return false;
    };

    bytes.len() >= 4 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF
}

fn has_png_signature(path: &Path) -> bool {
    let Ok(bytes) = fs::read(path) else {
        return false;
    };

    bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A])
}

fn sanitize_process_output(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let trimmed = text.trim();

    if trimmed.is_empty() {
        "none".into()
    } else {
        trimmed.replace('\n', " ").replace('\r', " ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    };

    fn unique_temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "boothy-render-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ))
    }

    fn test_background_operation(label: &str) -> BackgroundRenderOperation {
        BackgroundRenderOperation::SpeculativePreview {
            session_id: "session-flight-test".into(),
            request_id: format!("request-{label}"),
            capture_id: "capture-flight-test".into(),
            preset_id: "preset-flight-test".into(),
            preset_version: "1".into(),
            source_asset_path: PathBuf::from(format!("source-{label}.jpg")),
        }
    }

    fn wait_until_follower_attaches(output_identity: &Path) {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let attached = {
                let flights = BACKGROUND_RENDER_FLIGHTS
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                flights
                    .get(output_identity)
                    .is_some_and(|flight| Arc::strong_count(flight) >= 3)
            };
            if attached {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "follower should attach to the active render flight"
            );
            thread::yield_now();
        }
    }

    #[test]
    fn duplicate_background_renders_execute_once_and_share_the_result() {
        let output_identity = unique_temp_dir("same-flight").join("preview.jpg");
        let operation = test_background_operation("same");
        let execution_count = Arc::new(AtomicUsize::new(0));
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();

        let owner_output = output_identity.clone();
        let owner_operation = operation.clone();
        let owner_count = Arc::clone(&execution_count);
        let owner = thread::spawn(move || {
            run_or_join_background_render(owner_output, owner_operation, || {
                owner_count.fetch_add(1, Ordering::SeqCst);
                started_tx.send(()).expect("owner start should be observed");
                release_rx.recv().expect("owner should be released");
                Ok(BackgroundRenderOutcome::SpeculativePreview(
                    PreparedPreviewRender {
                        detail: "shared-result".into(),
                    },
                ))
            })
        });

        started_rx.recv().expect("owner should start");
        let follower_output = output_identity.clone();
        let follower_operation = operation.clone();
        let follower_count = Arc::clone(&execution_count);
        let follower = thread::spawn(move || {
            run_or_join_background_render(follower_output, follower_operation, || {
                follower_count.fetch_add(1, Ordering::SeqCst);
                panic!("a duplicate follower must not execute")
            })
        });

        wait_until_follower_attaches(&output_identity);
        release_tx.send(()).expect("owner release should succeed");

        let owner_result = owner.join().expect("owner should not panic");
        let follower_result = follower.join().expect("follower should not panic");
        assert_eq!(owner_result, follower_result);
        assert_eq!(execution_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn duplicate_background_renders_share_the_owner_failure() {
        let output_identity = unique_temp_dir("failed-flight").join("preview.jpg");
        let operation = test_background_operation("failed");
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();

        let owner_output = output_identity.clone();
        let owner_operation = operation.clone();
        let owner = thread::spawn(move || {
            run_or_join_background_render(owner_output, owner_operation, || {
                started_tx.send(()).expect("owner start should be observed");
                release_rx.recv().expect("owner should be released");
                Err(RenderWorkerError {
                    reason_code: "test-render-failed",
                    customer_message: "safe failure".into(),
                    operator_detail: "test failure".into(),
                })
            })
        });

        started_rx.recv().expect("owner should start");
        let follower_output = output_identity.clone();
        let follower = thread::spawn(move || {
            run_or_join_background_render(follower_output, operation, || {
                panic!("a duplicate follower must not execute")
            })
        });

        wait_until_follower_attaches(&output_identity);
        release_tx.send(()).expect("owner release should succeed");

        let owner_error = owner
            .join()
            .expect("owner should not panic")
            .expect_err("owner should fail");
        let follower_error = follower
            .join()
            .expect("follower should not panic")
            .expect_err("follower should share the failure");
        assert_eq!(owner_error, follower_error);
    }

    #[test]
    fn different_recipes_for_one_output_are_serialized_not_coalesced() {
        let output_identity = unique_temp_dir("conflicting-flight").join("preview.jpg");
        let first_operation = test_background_operation("first");
        let second_operation = test_background_operation("second");
        let execution_count = Arc::new(AtomicUsize::new(0));
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();

        let first_output = output_identity.clone();
        let first_count = Arc::clone(&execution_count);
        let first = thread::spawn(move || {
            run_or_join_background_render(first_output, first_operation, || {
                first_count.fetch_add(1, Ordering::SeqCst);
                started_tx
                    .send(())
                    .expect("first owner start should be observed");
                release_rx.recv().expect("first owner should be released");
                Ok(BackgroundRenderOutcome::SpeculativePreview(
                    PreparedPreviewRender {
                        detail: "first-result".into(),
                    },
                ))
            })
        });

        started_rx.recv().expect("first owner should start");
        let second_output = output_identity.clone();
        let second_count = Arc::clone(&execution_count);
        let second = thread::spawn(move || {
            run_or_join_background_render(second_output, second_operation, || {
                second_count.fetch_add(1, Ordering::SeqCst);
                Ok(BackgroundRenderOutcome::SpeculativePreview(
                    PreparedPreviewRender {
                        detail: "second-result".into(),
                    },
                ))
            })
        });

        wait_until_follower_attaches(&output_identity);
        assert_eq!(execution_count.load(Ordering::SeqCst), 1);
        release_tx
            .send(())
            .expect("first owner release should succeed");

        let first_result = first.join().expect("first owner should not panic");
        let second_result = second.join().expect("second owner should not panic");
        assert_ne!(first_result, second_result);
        assert_eq!(execution_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn darktable_cli_resolution_prefers_env_override() {
        let resolution =
            resolve_darktable_cli_binary_with_candidates(Some("C:/custom/darktable-cli.exe"), &[]);

        assert_eq!(resolution.binary, "C:/custom/darktable-cli.exe");
        assert_eq!(resolution.source, "env-override");
    }

    #[test]
    fn darktable_cli_resolution_uses_existing_known_install_path() {
        let temp_dir = unique_temp_dir("known-install");
        let candidate = temp_dir
            .join("darktable")
            .join("bin")
            .join("darktable-cli.exe");
        fs::create_dir_all(
            candidate
                .parent()
                .expect("candidate should have a parent directory"),
        )
        .expect("candidate parent directory should be creatable");
        fs::write(&candidate, "cli").expect("candidate binary should be writable");

        let resolution = resolve_darktable_cli_binary_with_candidates(
            None,
            &[("program-files-bin", candidate.clone())],
        );

        assert_eq!(resolution.binary, candidate.to_string_lossy().as_ref());
        assert_eq!(resolution.source, "program-files-bin");

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn darktable_cli_resolution_falls_back_to_path_when_no_known_binary_exists() {
        let resolution = resolve_darktable_cli_binary_with_candidates(
            None,
            &[(
                "program-files-bin",
                PathBuf::from("C:/missing/darktable-cli.exe"),
            )],
        );

        assert_eq!(resolution.binary, "darktable-cli");
        assert_eq!(resolution.source, "path");
    }

    fn write_fake_darktable_tree(root: &Path) -> PathBuf {
        let binary = root.join("bin").join("darktable-cli.exe");
        fs::create_dir_all(binary.parent().expect("bin 디렉터리가 있어야 한다"))
            .expect("bin 디렉터리를 만들 수 있어야 한다");
        fs::write(&binary, "cli").expect("darktable-cli를 쓸 수 있어야 한다");
        fs::create_dir_all(root.join("lib")).expect("lib 디렉터리를 만들 수 있어야 한다");
        fs::create_dir_all(root.join("share").join("darktable"))
            .expect("share/darktable 디렉터리를 만들 수 있어야 한다");
        binary
    }

    /// Story 7.7. **번들 트리가 이미 설치된 darktable보다 먼저다.**
    ///
    /// 이 순서가 뒤집히면 부스 PC에 깔린 아무 버전이 고객 화면을 만든다. 그 회차의
    /// 성능 숫자는 다른 렌더러의 숫자다.
    #[test]
    fn darktable_cli_resolution_prefers_the_bundled_tree_over_an_installed_one() {
        let temp_dir = unique_temp_dir("bundled-first");
        let bundled = write_fake_darktable_tree(&temp_dir.join("bundle").join("darktable"));
        let installed = temp_dir
            .join("program-files")
            .join("darktable")
            .join("bin")
            .join("darktable-cli.exe");
        fs::create_dir_all(installed.parent().expect("설치 후보 부모 디렉터리"))
            .expect("설치 후보 디렉터리를 만들 수 있어야 한다");
        fs::write(&installed, "cli").expect("설치 후보를 쓸 수 있어야 한다");

        let resolution = resolve_darktable_cli_binary_with_candidates(
            None,
            &[
                ("bundled-resource", bundled.clone()),
                ("program-files-bin", installed),
            ],
        );

        assert_eq!(resolution.binary, bundled.to_string_lossy().as_ref());
        assert_eq!(resolution.source, "bundled-resource");

        let _ = fs::remove_dir_all(temp_dir);
    }

    /// **개발용 탈출구는 그대로 남는다.** 번들이 있어도 env override가 이긴다.
    #[test]
    fn darktable_cli_resolution_still_lets_the_env_override_win_over_the_bundle() {
        let temp_dir = unique_temp_dir("bundled-env");
        let bundled = write_fake_darktable_tree(&temp_dir.join("darktable"));

        let resolution = resolve_darktable_cli_binary_with_candidates(
            Some("C:/custom/darktable-cli.exe"),
            &[("bundled-resource", bundled)],
        );

        assert_eq!(resolution.binary, "C:/custom/darktable-cli.exe");
        assert_eq!(resolution.source, "env-override");

        let _ = fs::remove_dir_all(temp_dir);
    }

    /// **개발 루프 무회귀.** 번들 트리가 없는 개발 PC에서는 기존 순서가 그대로다.
    #[test]
    fn darktable_cli_resolution_keeps_the_previous_order_without_a_bundle() {
        let temp_dir = unique_temp_dir("bundled-absent");
        let installed = temp_dir
            .join("program-files")
            .join("darktable")
            .join("bin")
            .join("darktable-cli.exe");
        fs::create_dir_all(installed.parent().expect("설치 후보 부모 디렉터리"))
            .expect("설치 후보 디렉터리를 만들 수 있어야 한다");
        fs::write(&installed, "cli").expect("설치 후보를 쓸 수 있어야 한다");

        let resolution = resolve_darktable_cli_binary_with_candidates(
            None,
            &[
                (
                    "bundled-resource",
                    temp_dir.join("missing-bundle").join("darktable-cli.exe"),
                ),
                ("program-files-bin", installed.clone()),
            ],
        );

        assert_eq!(resolution.binary, installed.to_string_lossy().as_ref());
        assert_eq!(resolution.source, "program-files-bin");

        let _ = fs::remove_dir_all(temp_dir);
    }

    /// 후보 목록 자체의 순서를 고정한다. 목록이 조용히 재배열되면 위 테스트만으로는 안 잡힌다.
    #[test]
    fn the_bundled_darktable_candidate_is_first_in_the_candidate_list() {
        if !cfg!(windows) {
            return;
        }

        let sources = darktable_cli_binary_candidates()
            .into_iter()
            .map(|(source, _)| source)
            .collect::<Vec<_>>();

        assert_eq!(
            sources.first().copied(),
            Some("bundled-resource"),
            "번들 후보가 첫 자리를 잃으면 5.4.1 핀이 문서에만 남는다"
        );
    }

    /// **실행 파일만 있고 데이터가 없으면 렌더가 실패한다.** 트리 전체를 본다.
    #[test]
    fn a_darktable_tree_missing_its_data_directories_is_reported_as_incomplete() {
        let temp_dir = unique_temp_dir("darktable-tree");
        let complete = temp_dir.join("complete");
        write_fake_darktable_tree(&complete);
        assert!(missing_bundled_darktable_entries(&complete).is_empty());

        let binary_only = temp_dir.join("binary-only");
        let binary = binary_only.join("bin").join("darktable-cli.exe");
        fs::create_dir_all(binary.parent().expect("bin 디렉터리가 있어야 한다"))
            .expect("bin 디렉터리를 만들 수 있어야 한다");
        fs::write(&binary, "cli").expect("darktable-cli를 쓸 수 있어야 한다");

        assert_eq!(
            missing_bundled_darktable_entries(&binary_only),
            vec!["lib", "share/darktable"]
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn preview_invocation_uses_display_sized_render_arguments() {
        let temp_dir = unique_temp_dir("preview-invocation");
        let session_id = "session_test";
        let paths = SessionPaths::new(&temp_dir, session_id);
        fs::create_dir_all(&paths.session_root).expect("session root should exist");
        let invocation = build_darktable_invocation(
            &temp_dir,
            PINNED_DARKTABLE_VERSION,
            &temp_dir.join("bundle").join("preview.xmp"),
            &SessionCaptureRecord {
                schema_version: "session-capture/v1".into(),
                session_id: session_id.into(),
                booth_alias: "Booth".into(),
                active_preset_id: Some("preset_test".into()),
                active_preset_version: "2026.03.31".into(),
                active_preset_display_name: Some("Test".into()),
                capture_id: "capture_test".into(),
                request_id: "request_test".into(),
                raw: crate::session::session_manifest::RawCaptureAsset {
                    asset_path: "C:/captures/originals/capture.cr2".into(),
                    persisted_at_ms: 100,
                },
                preview: crate::session::session_manifest::PreviewCaptureAsset {
                    asset_path: None,
                    enqueued_at_ms: Some(100),
                    ready_at_ms: None,
                },
                final_asset: crate::session::session_manifest::FinalCaptureAsset {
                    asset_path: None,
                    ready_at_ms: None,
                },
                render_status: "previewWaiting".into(),
                post_end_state: "activeSession".into(),
                timing: crate::session::session_manifest::CaptureTimingMetrics {
                    capture_acknowledged_at_ms: 100,
                    preview_visible_at_ms: None,
                    fast_preview_visible_at_ms: None,
                    xmp_preview_ready_at_ms: None,
                    capture_budget_ms: 1000,
                    preview_budget_ms: 5000,
                    preview_budget_state: "pending".into(),
                },
            },
            &paths,
            &temp_dir
                .join("renders")
                .join("previews")
                .join("capture.jpg"),
            RenderIntent::Preview,
            None,
        );

        assert!(invocation.arguments.contains(&"--width".to_string()));
        assert!(invocation
            .arguments
            .contains(&RAW_PREVIEW_MAX_WIDTH_PX.to_string()));
        assert!(invocation.arguments.contains(&"--height".to_string()));
        assert!(invocation
            .arguments
            .contains(&RAW_PREVIEW_MAX_HEIGHT_PX.to_string()));
        assert!(invocation
            .arguments
            .windows(2)
            .any(|pair| { pair[0] == "--hq" && pair[1] == "false" }));
        assert!(invocation.arguments.windows(2).any(|pair| {
            pair[0] == "--apply-custom-presets"
                && pair[1] == DARKTABLE_APPLY_CUSTOM_PRESETS_DISABLED
        }));
        assert_eq!(
            invocation.render_source_kind,
            PreviewRenderSourceKind::RawOriginal
        );
    }

    #[test]
    fn final_invocation_keeps_full_resolution_render_arguments() {
        let temp_dir = unique_temp_dir("final-invocation");
        let session_id = "session_test";
        let paths = SessionPaths::new(&temp_dir, session_id);
        fs::create_dir_all(&paths.session_root).expect("session root should exist");
        let invocation = build_darktable_invocation(
            &temp_dir,
            PINNED_DARKTABLE_VERSION,
            &temp_dir.join("bundle").join("final.xmp"),
            &SessionCaptureRecord {
                schema_version: "session-capture/v1".into(),
                session_id: session_id.into(),
                booth_alias: "Booth".into(),
                active_preset_id: Some("preset_test".into()),
                active_preset_version: "2026.03.31".into(),
                active_preset_display_name: Some("Test".into()),
                capture_id: "capture_test".into(),
                request_id: "request_test".into(),
                raw: crate::session::session_manifest::RawCaptureAsset {
                    asset_path: "C:/captures/originals/capture.cr2".into(),
                    persisted_at_ms: 100,
                },
                preview: crate::session::session_manifest::PreviewCaptureAsset {
                    asset_path: None,
                    enqueued_at_ms: Some(100),
                    ready_at_ms: None,
                },
                final_asset: crate::session::session_manifest::FinalCaptureAsset {
                    asset_path: None,
                    ready_at_ms: None,
                },
                render_status: "previewWaiting".into(),
                post_end_state: "activeSession".into(),
                timing: crate::session::session_manifest::CaptureTimingMetrics {
                    capture_acknowledged_at_ms: 100,
                    preview_visible_at_ms: None,
                    fast_preview_visible_at_ms: None,
                    xmp_preview_ready_at_ms: None,
                    capture_budget_ms: 1000,
                    preview_budget_ms: 5000,
                    preview_budget_state: "pending".into(),
                },
            },
            &paths,
            &temp_dir.join("renders").join("finals").join("capture.jpg"),
            RenderIntent::Final,
            None,
        );

        assert!(!invocation.arguments.contains(&"--width".to_string()));
        assert!(!invocation.arguments.contains(&"--height".to_string()));
        assert!(invocation
            .arguments
            .windows(2)
            .any(|pair| { pair[0] == "--hq" && pair[1] == "true" }));
        assert!(!invocation
            .arguments
            .contains(&"--apply-custom-presets".to_string()));
    }

    #[test]
    fn preview_renderer_warmup_source_is_written_as_png() {
        let temp_dir = unique_temp_dir("preview-warmup-source");
        let warmup_source = ensure_preview_renderer_warmup_source(&temp_dir)
            .expect("warmup source should be creatable");
        let bytes = fs::read(&warmup_source).expect("warmup source should be readable");

        assert!(bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]));

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn preview_invocation_prefers_same_capture_fast_preview_raster_when_available() {
        let temp_dir = unique_temp_dir("preview-fast-source");
        let session_id = "session_test";
        let paths = SessionPaths::new(&temp_dir, session_id);
        fs::create_dir_all(&paths.renders_previews_dir).expect("preview dir should exist");
        fs::create_dir_all(&paths.captures_originals_dir).expect("raw dir should exist");
        let fast_preview_path = paths.renders_previews_dir.join("capture_test.jpg");
        fs::write(&fast_preview_path, [0xFF, 0xD8, 0xFF, 0xE0, 0x00])
            .expect("jpeg preview should exist");

        let invocation = build_darktable_invocation(
            &temp_dir,
            PINNED_DARKTABLE_VERSION,
            &temp_dir.join("bundle").join("preview.xmp"),
            &SessionCaptureRecord {
                schema_version: "session-capture/v1".into(),
                session_id: session_id.into(),
                booth_alias: "Booth".into(),
                active_preset_id: Some("preset_test".into()),
                active_preset_version: "2026.03.31".into(),
                active_preset_display_name: Some("Test".into()),
                capture_id: "capture_test".into(),
                request_id: "request_test".into(),
                raw: crate::session::session_manifest::RawCaptureAsset {
                    asset_path: paths
                        .captures_originals_dir
                        .join("capture_test.cr2")
                        .to_string_lossy()
                        .into_owned(),
                    persisted_at_ms: 100,
                },
                preview: crate::session::session_manifest::PreviewCaptureAsset {
                    asset_path: Some(fast_preview_path.to_string_lossy().into_owned()),
                    enqueued_at_ms: Some(100),
                    ready_at_ms: None,
                },
                final_asset: crate::session::session_manifest::FinalCaptureAsset {
                    asset_path: None,
                    ready_at_ms: None,
                },
                render_status: "previewWaiting".into(),
                post_end_state: "activeSession".into(),
                timing: crate::session::session_manifest::CaptureTimingMetrics {
                    capture_acknowledged_at_ms: 100,
                    preview_visible_at_ms: None,
                    fast_preview_visible_at_ms: None,
                    xmp_preview_ready_at_ms: None,
                    capture_budget_ms: 1000,
                    preview_budget_ms: 5000,
                    preview_budget_state: "pending".into(),
                },
            },
            &paths,
            &temp_dir
                .join("renders")
                .join("previews")
                .join("capture_test.rendered.jpg"),
            RenderIntent::Preview,
            None,
        );

        assert_eq!(
            invocation.render_source_kind,
            PreviewRenderSourceKind::FastPreviewRaster
        );
        assert!(invocation
            .arguments
            .contains(&FAST_PREVIEW_RENDER_MAX_WIDTH_PX.to_string()));
        assert!(invocation
            .arguments
            .contains(&FAST_PREVIEW_RENDER_MAX_HEIGHT_PX.to_string()));
        assert_eq!(
            invocation.arguments.first().map(String::as_str),
            Some(
                fast_preview_path
                    .to_string_lossy()
                    .replace('\\', "/")
                    .as_str()
            )
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn preview_invocation_uses_canonical_preview_asset_even_when_manifest_preview_is_empty() {
        let temp_dir = unique_temp_dir("preview-canonical-fallback");
        let session_id = "session_test";
        let paths = SessionPaths::new(&temp_dir, session_id);
        fs::create_dir_all(&paths.renders_previews_dir).expect("preview dir should exist");
        fs::create_dir_all(&paths.captures_originals_dir).expect("raw dir should exist");
        let canonical_preview_path = paths.renders_previews_dir.join("capture_test.jpg");
        fs::write(&canonical_preview_path, [0xFF, 0xD8, 0xFF, 0xE0, 0x00])
            .expect("jpeg preview should exist");

        let invocation = build_darktable_invocation(
            &temp_dir,
            PINNED_DARKTABLE_VERSION,
            &temp_dir.join("bundle").join("preview.xmp"),
            &SessionCaptureRecord {
                schema_version: "session-capture/v1".into(),
                session_id: session_id.into(),
                booth_alias: "Booth".into(),
                active_preset_id: Some("preset_test".into()),
                active_preset_version: "2026.03.31".into(),
                active_preset_display_name: Some("Test".into()),
                capture_id: "capture_test".into(),
                request_id: "request_test".into(),
                raw: crate::session::session_manifest::RawCaptureAsset {
                    asset_path: paths
                        .captures_originals_dir
                        .join("capture_test.cr2")
                        .to_string_lossy()
                        .into_owned(),
                    persisted_at_ms: 100,
                },
                preview: crate::session::session_manifest::PreviewCaptureAsset {
                    asset_path: None,
                    enqueued_at_ms: Some(100),
                    ready_at_ms: None,
                },
                final_asset: crate::session::session_manifest::FinalCaptureAsset {
                    asset_path: None,
                    ready_at_ms: None,
                },
                render_status: "previewWaiting".into(),
                post_end_state: "activeSession".into(),
                timing: crate::session::session_manifest::CaptureTimingMetrics {
                    capture_acknowledged_at_ms: 100,
                    preview_visible_at_ms: None,
                    fast_preview_visible_at_ms: None,
                    xmp_preview_ready_at_ms: None,
                    capture_budget_ms: 1000,
                    preview_budget_ms: 5000,
                    preview_budget_state: "pending".into(),
                },
            },
            &paths,
            &temp_dir
                .join("renders")
                .join("previews")
                .join("capture_test.rendered.jpg"),
            RenderIntent::Preview,
            None,
        );

        assert_eq!(
            invocation.render_source_kind,
            PreviewRenderSourceKind::FastPreviewRaster
        );
        assert!(invocation
            .arguments
            .contains(&FAST_PREVIEW_RENDER_MAX_WIDTH_PX.to_string()));
        assert!(invocation
            .arguments
            .contains(&FAST_PREVIEW_RENDER_MAX_HEIGHT_PX.to_string()));
        assert_eq!(
            invocation.arguments.first().map(String::as_str),
            Some(
                canonical_preview_path
                    .to_string_lossy()
                    .replace('\\', "/")
                    .as_str()
            )
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn staging_render_output_path_stays_separate_from_the_canonical_preview_asset() {
        let temp_dir = unique_temp_dir("staging-path");
        let paths = SessionPaths::new(&temp_dir, "session_test");
        let canonical = canonical_render_output_path(&paths, "capture_test", RenderIntent::Preview);
        let staging = build_staging_render_output_path(
            &paths.renders_previews_dir,
            "capture_test",
            RenderIntent::Preview,
        );

        assert_ne!(canonical, staging);
        assert_eq!(
            canonical.file_name().and_then(|value| value.to_str()),
            Some("capture_test.jpg")
        );
        assert_eq!(
            staging.file_name().and_then(|value| value.to_str()),
            Some("capture_test.preview-rendering.jpg")
        );
    }

    #[test]
    fn failed_output_promotion_restores_the_existing_preview_asset() {
        let temp_dir = unique_temp_dir("promote-restore");
        let output_root = temp_dir.join("renders").join("previews");
        fs::create_dir_all(&output_root).expect("output root should exist");

        let canonical = output_root.join("capture_test.jpg");
        fs::write(&canonical, b"existing-preview").expect("existing preview should be writable");
        let missing_staging = output_root.join("capture_test.preview-rendering.jpg");

        let error = promote_render_output(&missing_staging, &canonical, RenderIntent::Preview)
            .expect_err("missing staging output should fail promotion");

        assert_eq!(error.reason_code, "render-output-promote-failed");
        assert_eq!(
            fs::read(&canonical).expect("existing preview should be restored"),
            b"existing-preview"
        );
        assert!(
            !output_root.join("capture_test.preview-backup.jpg").exists(),
            "temporary backup should be cleaned up after restore"
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    // -----------------------------------------------------------------------
    // Story 7.4: display-fit preset proxy invocation.
    // -----------------------------------------------------------------------

    fn display_proxy_spec<'a>(
        source: &'a Path,
        xmp: &'a Path,
        output: &'a Path,
    ) -> DisplayProxyRenderSpec<'a> {
        DisplayProxyRenderSpec {
            source_asset_path: source,
            xmp_template_path: xmp,
            output_path: output,
            target_width_px: 1620,
            target_height_px: 1080,
            output_color_space: "sRGB",
            jpeg_quality: 92,
            icc_intent: "perceptual",
            source_is_raw_original: true,
            schedule: DisplayRenderSchedule {
                priority: JobPriority::P0CurrentProxy,
                job_key: display_job_key(
                    "session-1",
                    "req-1",
                    "cap-1",
                    "displayFitPresetProxy",
                    "preset_soft-glow",
                    "2026.08.01",
                ),
                deadline_micros: 5_000_000,
                coordinates: JobCoordinates {
                    session_id: "session-1".into(),
                    request_id: "req-1".into(),
                    capture_id: Some("cap-1".into()),
                    capture_order: Some(0),
                    viewer_epoch: 3,
                },
            },
        }
    }

    #[test]
    fn display_proxy_invocation_uses_the_caller_supplied_screen_size() {
        // 크기는 상수가 아니라 viewer photoRect × DPR에서만 온다.
        let base_dir = PathBuf::from("C:/base");
        let source = PathBuf::from("C:/session/captures/originals/a.cr2");
        let xmp = PathBuf::from("C:/bundle/preset.xmp");
        let output = PathBuf::from("C:/session/renders/display/.staging/a-proxy-render.jpg");
        let mut spec = display_proxy_spec(&source, &xmp, &output);
        spec.target_width_px = 2880;
        spec.target_height_px = 1920;

        let invocation = build_display_proxy_invocation(
            &base_dir,
            &spec,
            DisplayRenderQuality::Fast,
            "SRGB",
            "PERCEPTUAL",
        );

        let width_index = invocation
            .arguments
            .iter()
            .position(|value| value == "--width")
            .expect("--width must be present");
        assert_eq!(invocation.arguments[width_index + 1], "2880");
        let height_index = invocation
            .arguments
            .iter()
            .position(|value| value == "--height")
            .expect("--height must be present");
        assert_eq!(invocation.arguments[height_index + 1], "1920");

        // 384px 상수는 이 경로에 존재하지 않는다.
        assert!(!invocation.arguments.iter().any(|value| value == "384"));
    }

    #[test]
    fn display_proxy_invocation_never_upscales() {
        // upscaled frame은 PRD NFR-003의 zero 항목이다.
        let base_dir = PathBuf::from("C:/base");
        let source = PathBuf::from("C:/session/captures/originals/a.cr2");
        let xmp = PathBuf::from("C:/bundle/preset.xmp");
        let output = PathBuf::from("C:/out/a.jpg");
        let spec = display_proxy_spec(&source, &xmp, &output);

        let invocation = build_display_proxy_invocation(
            &base_dir,
            &spec,
            DisplayRenderQuality::Fast,
            "SRGB",
            "PERCEPTUAL",
        );
        let upscale_index = invocation
            .arguments
            .iter()
            .position(|value| value == "--upscale")
            .expect("--upscale must be present");

        assert_eq!(invocation.arguments[upscale_index + 1], "false");
    }

    #[test]
    fn display_proxy_invocation_pins_the_approved_output_profile() {
        let base_dir = PathBuf::from("C:/base");
        let source = PathBuf::from("C:/session/captures/originals/a.cr2");
        let xmp = PathBuf::from("C:/bundle/preset.xmp");
        let output = PathBuf::from("C:/out/a.jpg");
        let spec = display_proxy_spec(&source, &xmp, &output);

        let invocation = build_display_proxy_invocation(
            &base_dir,
            &spec,
            DisplayRenderQuality::Fast,
            "SRGB",
            "PERCEPTUAL",
        );
        let arguments = invocation.arguments.join(" ");

        assert!(arguments.contains("--icc-type SRGB"));
        assert!(arguments.contains("--icc-intent PERCEPTUAL"));
        assert!(arguments.contains("--out-ext jpg"));
        assert!(arguments.contains(&format!("{DARKTABLE_JPEG_QUALITY_CONF_KEY}=92")));
        // custom preset 오염 방지는 preview 경로와 동일하게 유지한다.
        assert!(arguments.contains("--apply-custom-presets false"));
        assert!(arguments.contains("--hq false"));
    }

    #[test]
    fn display_proxy_core_arguments_stay_last() {
        // `--core` 뒤는 전부 darktable core로 넘어간다. 앞에 오면 조용히 무시된다.
        let base_dir = PathBuf::from("C:/base");
        let source = PathBuf::from("C:/session/captures/originals/a.cr2");
        let xmp = PathBuf::from("C:/bundle/preset.xmp");
        let output = PathBuf::from("C:/out/a.jpg");
        let spec = display_proxy_spec(&source, &xmp, &output);

        let invocation = build_display_proxy_invocation(
            &base_dir,
            &spec,
            DisplayRenderQuality::Fast,
            "SRGB",
            "PERCEPTUAL",
        );
        let core_index = invocation
            .arguments
            .iter()
            .position(|value| value == "--core")
            .expect("--core must be present");

        for flag in [
            "--width",
            "--height",
            "--upscale",
            "--icc-type",
            "--out-ext",
        ] {
            let flag_index = invocation
                .arguments
                .iter()
                .position(|value| value == flag)
                .unwrap_or_else(|| panic!("{flag} must be present"));
            assert!(flag_index < core_index, "{flag} must precede --core");
        }

        let conf_index = invocation
            .arguments
            .iter()
            .position(|value| value == "--conf")
            .expect("--conf must be present");
        assert!(conf_index > core_index, "--conf must follow --core");
    }

    #[test]
    fn display_proxy_worker_root_is_separate_from_preview_and_final() {
        // config·library.db를 공유하면 동시 실행이 서로를 막아 첫 화면이 늦어진다.
        let base_dir = PathBuf::from("C:/base");
        let source = PathBuf::from("C:/session/captures/originals/a.cr2");
        let xmp = PathBuf::from("C:/bundle/preset.xmp");
        let output = PathBuf::from("C:/out/a.jpg");
        let spec = display_proxy_spec(&source, &xmp, &output);

        let invocation = build_display_proxy_invocation(
            &base_dir,
            &spec,
            DisplayRenderQuality::Fast,
            "SRGB",
            "PERCEPTUAL",
        );
        let arguments = invocation.arguments.join(" ").replace('\\', "/");

        assert!(arguments.contains(".boothy-darktable/display-proxy/config"));
        assert!(arguments.contains(".boothy-darktable/display-proxy/library.db"));
        assert!(!arguments.contains(".boothy-darktable/preview/"));
        assert!(!arguments.contains(".boothy-darktable/final/"));
    }

    // -----------------------------------------------------------------------
    // Story 7.6: 정밀본 lane과 취소.
    // -----------------------------------------------------------------------

    #[test]
    fn the_refined_lane_differs_from_the_proxy_lane_by_exactly_one_argument() {
        // AC 6이 비교하려는 것은 "`--hq` 하나만 다른 두 결과"다.
        // 다른 인자가 하나라도 갈라지면 그 측정은 `--hq`를 재는 것이 아니게 된다.
        let base_dir = PathBuf::from("C:/base");
        let source = PathBuf::from("C:/session/captures/originals/a.cr2");
        let xmp = PathBuf::from("C:/bundle/preset.xmp");
        let output = PathBuf::from("C:/out/a.jpg");
        let spec = display_proxy_spec(&source, &xmp, &output);

        let proxy = build_display_proxy_invocation(
            &base_dir,
            &spec,
            DisplayRenderQuality::Fast,
            "SRGB",
            "PERCEPTUAL",
        );
        let refined = build_display_proxy_invocation(
            &base_dir,
            &spec,
            DisplayRenderQuality::High,
            "SRGB",
            "PERCEPTUAL",
        );

        let differences: Vec<(usize, &String, &String)> = proxy
            .arguments
            .iter()
            .zip(refined.arguments.iter())
            .enumerate()
            .filter(|(_, (left, right))| left != right)
            .map(|(index, (left, right))| (index, left, right))
            .collect();

        assert_eq!(
            proxy.arguments.len(),
            refined.arguments.len(),
            "두 lane의 인자 개수가 달라지면 안 된다"
        );
        // `--hq` 값 하나와 worker root 두 개(configdir / library)만 다르다.
        assert_eq!(differences.len(), 3, "예상 밖의 인자 차이: {differences:?}");
        assert_eq!(differences[0].1, "false");
        assert_eq!(differences[0].2, "true");
    }

    #[test]
    fn the_refined_lane_uses_a_fourth_worker_root() {
        // display-proxy와 `configdir`/`library.db`를 공유하면 P0와 P1이 서로를 막는다.
        let base_dir = PathBuf::from("C:/base");
        let source = PathBuf::from("C:/session/captures/originals/a.cr2");
        let xmp = PathBuf::from("C:/bundle/preset.xmp");
        let output = PathBuf::from("C:/out/a.jpg");
        let spec = display_proxy_spec(&source, &xmp, &output);

        let invocation = build_display_proxy_invocation(
            &base_dir,
            &spec,
            DisplayRenderQuality::High,
            "SRGB",
            "PERCEPTUAL",
        );
        let arguments = invocation.arguments.join(" ").replace('\\', "/");

        assert!(arguments.contains(".boothy-darktable/raw-refined/config"));
        assert!(arguments.contains(".boothy-darktable/raw-refined/library.db"));
        assert!(!arguments.contains(".boothy-darktable/display-proxy/"));
        assert!(!arguments.contains(".boothy-darktable/preview/"));
        assert!(!arguments.contains(".boothy-darktable/final/"));
    }

    /// **취소된 작업은 중간 산출물을 남기지 않는다.**
    /// 중간에 죽은 파일이 다음 시도의 "성공"으로 오인되면 안 된다.
    #[test]
    fn a_refined_render_cancelled_while_queued_writes_nothing() {
        let base_dir = std::env::temp_dir().join(format!(
            "boothy-refined-cancel-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&base_dir).expect("prepare test root");
        let source = base_dir.join("a.cr2");
        let xmp = base_dir.join("preset.xmp");
        let output = base_dir.join("staging").join("a-refined-render.jpg");
        fs::write(&source, b"raw").expect("prepare source");
        fs::write(&xmp, b"xmp").expect("prepare xmp");

        // 현재 촬영 lane을 P0가 점유한다. 정밀본은 줄을 서게 된다.
        let scheduler = scheduler_for(&base_dir);
        let occupied = match scheduler.admit(
            &RenderJobRequest {
                priority: JobPriority::P0CurrentProxy,
                job_key: "proxy/cap-1".into(),
                deadline_micros: 5_000_000,
                coordinates: JobCoordinates {
                    session_id: "session-1".into(),
                    request_id: "req-1".into(),
                    capture_id: Some("cap-1".into()),
                    capture_order: Some(0),
                    viewer_epoch: 3,
                },
            },
            &current_monotonic_micros,
        ) {
            SchedulerAdmission::Granted(lease) => lease,
            other => panic!("P0가 슬롯을 잡지 못했다: {other:?}"),
        };

        let worker_base = base_dir.clone();
        let worker_source = source.clone();
        let worker_xmp = xmp.clone();
        let worker_output = output.clone();
        let worker = thread::spawn(move || {
            let spec = DisplayProxyRenderSpec {
                source_asset_path: &worker_source,
                xmp_template_path: &worker_xmp,
                output_path: &worker_output,
                target_width_px: 1620,
                target_height_px: 1080,
                output_color_space: "sRGB",
                jpeg_quality: 92,
                icc_intent: "perceptual",
                source_is_raw_original: true,
                schedule: DisplayRenderSchedule {
                    priority: JobPriority::P1CurrentRawRefined,
                    job_key: "refined/cap-1".into(),
                    deadline_micros: 5_000_000,
                    coordinates: JobCoordinates {
                        session_id: "session-1".into(),
                        request_id: "req-1".into(),
                        capture_id: Some("cap-1".into()),
                        capture_order: Some(0),
                        viewer_epoch: 3,
                    },
                },
            };

            render_raw_refined_display_to_path_in_dir(
                &worker_base,
                &spec,
                &current_monotonic_micros,
            )
        });

        for _ in 0..200 {
            if scheduler.depth().0 == 1 {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }

        let summary = scheduler.cancel(&CancelScope::Request("req-1".into()));
        assert_eq!(summary.cancelled_waiting, 1);

        match worker
            .join()
            .expect("worker thread")
            .expect("취소는 오류가 아니라 정상 결과다")
        {
            DisplayRenderDisposition::Cancelled { reason, spans } => {
                assert_eq!(reason, "request-forgotten");
                assert_eq!(spans.preempted_by.as_deref(), Some("request-forgotten"));
            }
            other => panic!("취소된 작업이 렌더로 넘어갔다: {other:?}"),
        }

        assert!(
            !output.exists(),
            "취소된 작업이 staging 산출물을 남겼다: {}",
            output.to_string_lossy()
        );
        // 취소는 RAW를 건드리지 않는다.
        assert!(source.is_file());

        drop(occupied);
        let _ = fs::remove_dir_all(&base_dir);
    }

    /// **자식의 자식까지 죽는지를 darktable 없이 결정적으로 검증한다.**
    ///
    /// `cmd.exe`가 `ping`을 자식으로 낳는다. 오늘의 `child.kill()`은 `cmd`만 죽이고
    /// `ping`을 남겼다 — AC 1이 요구하는 것은 tree다.
    #[cfg(windows)]
    #[test]
    fn cancelling_a_render_terminates_the_whole_process_tree() {
        let mut child = Command::new("cmd.exe")
            .args(["/C", "ping", "-n", "30", "127.0.0.1"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn a process that has a child of its own");
        let pid = child.id();

        let mut descendants = 0;
        for _ in 0..10 {
            descendants = current_descendant_process_ids(pid)
                .expect("process snapshot must be available for the cancellation test")
                .len() as u32;
            if descendants > 0 {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        assert!(
            descendants > 0,
            "테스트 전제가 깨졌다: 종료 대상이 자손을 만들지 않았다"
        );

        let ticks = std::cell::Cell::new(0u64);
        let now = || {
            ticks.set(ticks.get().saturating_add(1_000));
            ticks.get()
        };
        let termination = terminate_process_tree(&mut child, &now);

        assert_eq!(
            termination.orphan_count,
            Some(0),
            "process tree 종료 뒤에도 자손이 살아남았다: {}",
            termination.as_log_detail()
        );
        assert!(
            termination.kill_outcome.starts_with("exit="),
            "taskkill 실행 결과를 기록하지 못했다: {}",
            termination.kill_outcome
        );
        assert!(termination.completed_at_micros > termination.requested_at_micros);
    }

    #[cfg(windows)]
    fn run_hv17_cancel_evidence_rounds(
        evidence_root: &Path,
    ) -> Result<Vec<serde_json::Value>, String> {
        fs::create_dir_all(evidence_root.join("scheduler")).map_err(|error| error.to_string())?;
        let mut rounds = Vec::new();

        for round in [
            "delete",
            "session-replaced",
            "viewer-epoch-changed",
            "newer-capture",
        ] {
            let scheduler = Arc::new(scheduler::RenderScheduler::default());
            let display_session = if round == "session-replaced" {
                "session-previous"
            } else {
                "session-current"
            };
            let display_priority = if round == "newer-capture" {
                JobPriority::P1CurrentRawRefined
            } else {
                JobPriority::P0CurrentProxy
            };
            let display_request = RenderJobRequest {
                priority: display_priority,
                job_key: format!("hv17/{round}/display"),
                deadline_micros: u64::MAX,
                coordinates: JobCoordinates {
                    session_id: display_session.into(),
                    request_id: "request-display".into(),
                    capture_id: Some("capture-display".into()),
                    capture_order: Some(1),
                    viewer_epoch: 1,
                },
            };
            let final_request = RenderJobRequest {
                priority: JobPriority::P2Background,
                job_key: format!("hv17/{round}/final"),
                deadline_micros: u64::MAX,
                coordinates: JobCoordinates {
                    session_id: "session-current".into(),
                    request_id: "request-final".into(),
                    capture_id: Some("capture-final".into()),
                    capture_order: Some(2),
                    viewer_epoch: 2,
                },
            };
            let now = || current_monotonic_micros();
            let final_lease = match scheduler.admit(&final_request, &now) {
                SchedulerAdmission::Granted(lease) => lease,
                other => return Err(format!("{round}: final admission failed: {other:?}")),
            };
            let display_lease = match scheduler.admit(&display_request, &now) {
                SchedulerAdmission::Granted(lease) => lease,
                other => return Err(format!("{round}: display admission failed: {other:?}")),
            };

            let mut display_process = Command::new("cmd.exe")
                .args(["/C", "ping", "-n", "30", "127.0.0.1"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|error| format!("{round}: spawn display process: {error}"))?;
            let mut descendants = 0;
            for _ in 0..20 {
                descendants = current_descendant_process_ids(display_process.id())
                    .ok_or_else(|| format!("{round}: process snapshot unavailable"))?
                    .len();
                if descendants > 0 {
                    break;
                }
                thread::sleep(Duration::from_millis(100));
            }
            if descendants == 0 {
                return Err(format!("{round}: display process created no descendant"));
            }

            let scope = match round {
                "delete" => CancelScope::Request("request-display".into()),
                "session-replaced" => CancelScope::OtherSessions("session-current".into()),
                "viewer-epoch-changed" => CancelScope::StaleViewerEpoch {
                    session_id: "session-current".into(),
                    current_epoch: 2,
                },
                "newer-capture" => CancelScope::SupersededRefined {
                    session_id: "session-current".into(),
                    capture_order: 2,
                },
                _ => unreachable!(),
            };
            let summary = scheduler.cancel(&scope);
            if summary.cancelled_running != 1
                || !display_lease.token().is_cancelled()
                || final_lease.token().is_cancelled()
            {
                return Err(format!(
                    "{round}: scope did not cancel exactly the display job: {summary:?}"
                ));
            }
            let reason = display_lease
                .token()
                .reason()
                .ok_or_else(|| format!("{round}: cancellation reason missing"))?;
            let termination = terminate_process_tree_for_reason_in_evidence_root(
                &mut display_process,
                &now,
                &reason,
                Some(evidence_root),
            );
            if termination.orphan_count != Some(0) {
                return Err(format!(
                    "{round}: process tree termination failed: {}",
                    termination.as_log_detail()
                ));
            }

            let final_status = Command::new("cmd.exe")
                .args(["/C", "exit", "0"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(|error| format!("{round}: final process failed to start: {error}"))?;
            let record_path = termination
                .evidence_record_path
                .ok_or_else(|| format!("{round}: evidence record was not written"))?;
            let mut record: serde_json::Value =
                serde_json::from_slice(&fs::read(&record_path).map_err(|error| error.to_string())?)
                    .map_err(|error| error.to_string())?;
            record["finalRenderCompleted"] = serde_json::Value::Bool(final_status.success());
            rounds.push(record);

            drop(display_lease);
            drop(final_lease);
        }

        let jsonl = rounds
            .iter()
            .map(|round| serde_json::to_string(round).map_err(|error| error.to_string()))
            .collect::<Result<Vec<_>, _>>()?
            .join("\n");
        fs::write(
            evidence_root.join("scheduler/cancel-rounds.jsonl"),
            format!("{jsonl}\n"),
        )
        .map_err(|error| error.to_string())?;
        Ok(rounds)
    }

    #[cfg(windows)]
    #[test]
    fn hv17_evidence_run_executes_all_four_scheduler_cancellation_scopes() {
        let configured_root = hv17_evidence_root();
        let root = configured_root
            .clone()
            .unwrap_or_else(|| unique_temp_dir("run-hv17-four-cancel-rounds"));

        let rounds = run_hv17_cancel_evidence_rounds(&root)
            .expect("execute the four real scheduler cancellation rounds");

        assert_eq!(rounds.len(), 4);
        assert_eq!(
            rounds
                .iter()
                .filter_map(|round| round["round"].as_str())
                .collect::<HashSet<_>>(),
            HashSet::from([
                "delete",
                "session-replaced",
                "viewer-epoch-changed",
                "newer-capture",
            ])
        );
        assert!(rounds.iter().all(|round| round["cancelOrphanCount"] == 0));
        assert!(rounds
            .iter()
            .all(|round| round["finalRenderCompleted"] == true));
        assert!(root.join("scheduler/cancel-rounds.jsonl").is_file());

        if configured_root.is_none() {
            let _ = fs::remove_dir_all(root);
        }
    }

    #[test]
    fn hv17_process_tree_evidence_preserves_raw_taskkill_output_and_probe_truth() {
        let root = unique_temp_dir("run-hv17-process-tree");
        let termination = ProcessTreeTermination {
            kill_outcome: "exit=0".to_string(),
            used_direct_kill_fallback: false,
            orphan_count: Some(0),
            requested_at_micros: 11_000,
            completed_at_micros: 12_500,
            evidence_record_path: None,
        };
        let transcript = TaskkillTranscript {
            command: r"C:\Windows\System32\taskkill.exe /T /F /PID 4312".to_string(),
            stdout: b"SUCCESS: process tree terminated".to_vec(),
            stderr: b"localized stderr remains verbatim".to_vec(),
        };

        let record_path = persist_process_tree_evidence_in_dir(
            &root,
            "request-forgotten",
            4312,
            &termination,
            &transcript,
        )
        .expect("persist evidence");
        let record: serde_json::Value =
            serde_json::from_slice(&fs::read(&record_path).expect("read structured record"))
                .expect("parse structured record");
        let log_path = root.join(
            record["taskkillLogPath"]
                .as_str()
                .expect("relative taskkill log path"),
        );
        let raw_log = fs::read_to_string(log_path).expect("read raw transcript");

        assert_eq!(record["cancelOrphanCount"], 0);
        assert_eq!(record["orphanProbeAvailable"], true);
        assert_eq!(record["round"], "delete");
        assert_eq!(record["cancellationReason"], "request-forgotten");
        assert_eq!(record["cancelRequestedAtMicros"], 11_000);
        assert_eq!(record["cancelCompletedAtMicros"], 12_500);
        assert!(raw_log.contains("SUCCESS: process tree terminated"));
        assert!(raw_log.contains("localized stderr remains verbatim"));
        assert!(record_path.to_string_lossy().contains("delete"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn hv17_evidence_root_requires_an_explicit_absolute_hv17_run() {
        assert_eq!(
            validated_hv17_evidence_root(PathBuf::from(r"C:\evidence\run-hv17")),
            Some(PathBuf::from(r"C:\evidence\run-hv17"))
        );
        assert_eq!(
            validated_hv17_evidence_root(PathBuf::from(r"C:\evidence\ordinary-run")),
            None
        );
        assert_eq!(
            validated_hv17_evidence_root(PathBuf::from("run-hv17")),
            None
        );
        assert_eq!(safe_evidence_label("../newer capture"), "newer-capture");
        assert_eq!(safe_evidence_label("***"), "unspecified");
        assert_eq!(hv17_round_label("request-forgotten"), "delete");
        assert_eq!(
            hv17_round_label("superseded-by-newer-capture"),
            "newer-capture"
        );
    }

    #[test]
    fn descendant_snapshot_walk_includes_grandchildren() {
        let processes = vec![(20, 10), (30, 20), (40, 30), (50, 999)];
        let descendants = descendant_process_ids_from_snapshot(10, &processes)
            .into_iter()
            .collect::<HashSet<_>>();

        assert_eq!(descendants, HashSet::from([20, 30, 40]));
    }

    #[test]
    fn the_384px_preview_contract_is_untouched_by_story_74() {
        // booth 사진 레일 경로는 이 Story의 범위 밖이다.
        assert_eq!(RAW_PREVIEW_MAX_WIDTH_PX, 384);
        assert_eq!(RAW_PREVIEW_MAX_HEIGHT_PX, 384);
        assert_eq!(FAST_PREVIEW_RENDER_MAX_WIDTH_PX, 384);
        assert_eq!(FAST_PREVIEW_RENDER_MAX_HEIGHT_PX, 384);
        assert_eq!(
            preview_render_dimensions(PreviewRenderSourceKind::RawOriginal),
            (384, 384)
        );
        assert_eq!(
            preview_render_dimensions(PreviewRenderSourceKind::FastPreviewRaster),
            (384, 384)
        );
    }

    #[test]
    fn unapproved_color_spaces_and_intents_are_errors_not_silent_defaults() {
        // 잘못된 토큰을 넘기면 darktable-cli는 렌더 대신 도움말을 출력하고 0으로 끝난다.
        // 그러면 "성공했는데 파일이 없는" 상태가 되어 원인을 찾기 어려워진다.
        assert_eq!(resolve_darktable_icc_type("sRGB").unwrap(), "SRGB");
        assert_eq!(resolve_darktable_icc_type("srgb").unwrap(), "SRGB");
        assert!(resolve_darktable_icc_type("ProPhoto").is_err());
        assert!(resolve_darktable_icc_type("").is_err());

        assert_eq!(
            resolve_darktable_icc_intent("perceptual").unwrap(),
            "PERCEPTUAL"
        );
        // 실측 확인: darktable-cli는 `INTENT_PERCEPTUAL`을 거부한다.
        assert!(resolve_darktable_icc_intent("INTENT_PERCEPTUAL").is_err());
        assert!(resolve_darktable_icc_intent("guess").is_err());
    }

    #[test]
    fn display_proxy_refuses_to_render_before_the_viewer_reports_its_size() {
        let base_dir = PathBuf::from("C:/base");
        let source = PathBuf::from("C:/session/captures/originals/a.cr2");
        let xmp = PathBuf::from("C:/bundle/preset.xmp");
        let output = PathBuf::from("C:/out/a.jpg");
        let mut spec = display_proxy_spec(&source, &xmp, &output);
        spec.target_width_px = 0;

        let error = render_display_proxy_to_path_in_dir(&base_dir, &spec, &|| 0)
            .expect_err("unmeasured viewer must not start a render");

        assert_eq!(error.reason_code, "display-proxy-target-unmeasured");
        assert!(!output.exists());
    }

    #[test]
    fn display_proxy_refuses_a_quality_outside_the_approved_range() {
        let base_dir = PathBuf::from("C:/base");
        let source = PathBuf::from("C:/session/captures/originals/a.cr2");
        let xmp = PathBuf::from("C:/bundle/preset.xmp");
        let output = PathBuf::from("C:/out/a.jpg");
        let mut spec = display_proxy_spec(&source, &xmp, &output);
        spec.jpeg_quality = 0;

        let error = render_display_proxy_to_path_in_dir(&base_dir, &spec, &|| 0)
            .expect_err("out-of-range quality must not start a render");

        assert_eq!(error.reason_code, "display-proxy-quality-out-of-range");
    }
}
