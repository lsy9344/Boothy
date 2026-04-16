pub mod dedicated_renderer;

use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{self, RecvTimeoutError, SyncSender, TrySendError},
        LazyLock, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[cfg(test)]
use std::sync::atomic::AtomicBool;

use crate::{
    preset::preset_bundle::PublishedPresetRuntimeBundle,
    preset::preset_catalog::{
        find_published_preset_runtime_bundle, resolve_published_preset_catalog_dir,
    },
    session::{session_manifest::SessionCaptureRecord, session_paths::SessionPaths},
};

const PINNED_DARKTABLE_VERSION: &str = "5.4.1";
const MAX_IN_FLIGHT_RENDER_JOBS: usize = if cfg!(test) { 64 } else { 2 };
const DEFAULT_RENDER_TIMEOUT: Duration = Duration::from_secs(45);
const DARKTABLE_CLI_BIN_ENV: &str = "BOOTHY_DARKTABLE_CLI_BIN";
#[cfg(test)]
const TEST_RENDER_QUEUE_LIMIT_ENV: &str = "BOOTHY_TEST_RENDER_QUEUE_LIMIT";
// Keep the render-backed close display-sized enough to stay faithful on hardware,
// while still avoiding the old full-size RAW path.
pub(crate) const RAW_PREVIEW_MAX_WIDTH_PX: u32 = 1024;
pub(crate) const RAW_PREVIEW_MAX_HEIGHT_PX: u32 = 1024;
pub(crate) const FAST_PREVIEW_RENDER_MAX_WIDTH_PX: u32 = 768;
pub(crate) const FAST_PREVIEW_RENDER_MAX_HEIGHT_PX: u32 = 768;
const DARKTABLE_APPLY_CUSTOM_PRESETS_DISABLED: &str = "false";
const RESIDENT_PREVIEW_WORKER_QUEUE_CAPACITY: usize = 2;
const RESIDENT_PREVIEW_WORKER_IDLE_TIMEOUT: Duration = Duration::from_secs(90);
const PREVIEW_RENDER_WARMUP_INPUT_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x01, 0x73, 0x52, 0x47, 0x42, 0x00, 0xAE, 0xCE, 0x1C, 0xE9, 0x00, 0x00,
    0x00, 0x04, 0x67, 0x41, 0x4D, 0x41, 0x00, 0x00, 0xB1, 0x8F, 0x0B, 0xFC, 0x61, 0x05, 0x00, 0x00,
    0x00, 0x09, 0x70, 0x48, 0x59, 0x73, 0x00, 0x00, 0x0E, 0xC3, 0x00, 0x00, 0x0E, 0xC3, 0x01, 0xC7,
    0x6F, 0xA8, 0x64, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x18, 0x57, 0x63, 0xF8, 0xFF,
    0xFF, 0xFF, 0x7F, 0x00, 0x09, 0xFB, 0x03, 0xFD, 0x05, 0x43, 0x45, 0xCA, 0x00, 0x00, 0x00, 0x00,
    0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

static RENDER_QUEUE_DEPTH: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));
static PREVIEW_RENDER_WARMUP_IN_FLIGHT: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));
static RESIDENT_PREVIEW_WORKERS: LazyLock<Mutex<HashMap<String, ResidentPreviewWorkerHandle>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static RESIDENT_PREVIEW_WORKER_GENERATION: AtomicU64 = AtomicU64::new(1);
#[cfg(test)]
static RESIDENT_PREVIEW_WORKER_RUN_INLINE_IN_TESTS: AtomicBool = AtomicBool::new(true);
#[cfg(test)]
static RESIDENT_PREVIEW_WORKER_TEST_IDLE_TIMEOUT_MS: AtomicU64 = AtomicU64::new(0);

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

pub struct PreparedPreviewRender {
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PreviewInvocationProfile {
    apply_custom_presets: bool,
    disable_opencl: bool,
    allow_fast_preview_raster: bool,
}

impl PreviewInvocationProfile {
    fn approved_booth_safe() -> Self {
        Self {
            apply_custom_presets: false,
            disable_opencl: true,
            allow_fast_preview_raster: true,
        }
    }
}

#[derive(Debug, Clone)]
struct ResidentPreviewWorkerHandle {
    generation: u64,
    sender: SyncSender<ResidentPreviewWorkerJob>,
}

#[derive(Debug, Clone)]
enum ResidentPreviewWorkerJob {
    Render(ResidentPreviewRenderJob),
}

#[derive(Debug, Clone)]
struct ResidentPreviewRenderJob {
    base_dir: PathBuf,
    session_id: String,
    request_id: String,
    capture_id: String,
    preset_id: String,
    preset_version: String,
    source_asset_path: PathBuf,
    source_cleanup_path: Option<PathBuf>,
    output_path: PathBuf,
    detail_path: PathBuf,
    lock_path: PathBuf,
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
    let _queue_guard = acquire_render_queue_slot()?;
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

    let paths = SessionPaths::new(base_dir, session_id);
    let output_path = canonical_render_output_path(&paths, &capture.capture_id, intent);
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
    let invocation_result = run_darktable_invocation(&invocation, intent)?;
    if let Err(error) = validate_render_output(&staging_output_path, intent) {
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
    let _queue_guard = acquire_render_queue_slot()?;
    render_preview_asset_to_path_with_queue_guard_in_dir(
        base_dir,
        session_id,
        request_id,
        capture_id,
        preset_id,
        preset_version,
        source_asset_path,
        output_path,
    )
}

fn render_preview_asset_to_path_with_background_capacity_in_dir(
    base_dir: &Path,
    session_id: &str,
    request_id: &str,
    capture_id: &str,
    preset_id: &str,
    preset_version: &str,
    source_asset_path: &Path,
    output_path: &Path,
) -> Result<PreparedPreviewRender, RenderWorkerError> {
    let Some(_queue_guard) = try_acquire_resident_preview_render_queue_slot() else {
        return Err(RenderWorkerError {
            reason_code: "render-queue-saturated",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail:
                "resident first-visible worker가 남은 render slot을 확보하지 못했어요.".into(),
        });
    };
    render_preview_asset_to_path_with_queue_guard_in_dir(
        base_dir,
        session_id,
        request_id,
        capture_id,
        preset_id,
        preset_version,
        source_asset_path,
        output_path,
    )
}

fn render_preview_asset_to_path_with_queue_guard_in_dir(
    base_dir: &Path,
    session_id: &str,
    request_id: &str,
    capture_id: &str,
    preset_id: &str,
    preset_version: &str,
    source_asset_path: &Path,
    output_path: &Path,
) -> Result<PreparedPreviewRender, RenderWorkerError> {
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
    let invocation_result = run_darktable_invocation(&invocation, RenderIntent::Preview)?;
    validate_render_output(output_path, RenderIntent::Preview)?;
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

pub fn prime_preview_worker_runtime_in_dir(base_dir: &Path, _session_id: &str) {
    let worker_root = base_dir.join(".boothy-darktable").join("preview");
    let _ = fs::create_dir_all(worker_root.join("warmup"));
    let _ = ensure_preview_renderer_warmup_source(base_dir);
}

pub fn enqueue_resident_preview_render_in_dir(
    base_dir: &Path,
    session_id: &str,
    request_id: &str,
    capture_id: &str,
    preset_id: &str,
    preset_version: &str,
    source_asset_path: &Path,
    source_cleanup_path: Option<&Path>,
    output_path: &Path,
    detail_path: &Path,
    lock_path: &Path,
) -> Result<(), RenderWorkerError> {
    let worker_key = build_resident_preview_worker_key(session_id, preset_id, preset_version);
    let job = ResidentPreviewWorkerJob::Render(ResidentPreviewRenderJob {
        base_dir: base_dir.to_path_buf(),
        session_id: session_id.to_string(),
        request_id: request_id.to_string(),
        capture_id: capture_id.to_string(),
        preset_id: preset_id.to_string(),
        preset_version: preset_version.to_string(),
        source_asset_path: source_asset_path.to_path_buf(),
        source_cleanup_path: source_cleanup_path.map(PathBuf::from),
        output_path: output_path.to_path_buf(),
        detail_path: detail_path.to_path_buf(),
        lock_path: lock_path.to_path_buf(),
    });

    enqueue_resident_preview_worker_job(worker_key, job)
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

    prime_preview_worker_runtime_in_dir(base_dir, session_id);

    if cfg!(test) {
        clear_preview_render_warmup_in_flight(&warmup_key);
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
        clear_preview_render_warmup_in_flight(&build_preview_render_warmup_key(
            &session_id,
            &preset_id,
            &preset_version,
        ));
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
    match intent {
        RenderIntent::Preview => {
            "확인용 사진을 아직 준비하지 못했어요. 가까운 직원에게 알려 주세요.".into()
        }
        RenderIntent::Final => {
            "결과 사진을 아직 마무리하지 못했어요. 가까운 직원에게 알려 주세요.".into()
        }
    }
}

fn render_stage_label(intent: RenderIntent) -> &'static str {
    match intent {
        RenderIntent::Preview => "preview",
        RenderIntent::Final => "final",
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

fn build_resident_preview_worker_key(
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

fn enqueue_resident_preview_worker_job(
    worker_key: String,
    job: ResidentPreviewWorkerJob,
) -> Result<(), RenderWorkerError> {
    if resident_preview_worker_runs_inline_in_tests() {
        let ResidentPreviewWorkerJob::Render(job) = job;
        run_resident_preview_render_job(job);
        return Ok(());
    }

    let mut handle = ensure_resident_preview_worker(&worker_key)?;

    match handle.sender.try_send(job.clone()) {
        Ok(()) => Ok(()),
        Err(TrySendError::Full(_)) => Err(RenderWorkerError {
            reason_code: "render-queue-saturated",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!(
                "resident first-visible worker queue가 가득 찼어요: key={worker_key}"
            ),
        }),
        Err(TrySendError::Disconnected(_)) => {
            remove_resident_preview_worker(&worker_key, handle.generation);
            handle = ensure_resident_preview_worker(&worker_key)?;
            match handle.sender.try_send(job) {
                Ok(()) => Ok(()),
                Err(TrySendError::Full(_)) => Err(RenderWorkerError {
                    reason_code: "render-queue-saturated",
                    customer_message: safe_render_failure_message(RenderIntent::Preview),
                    operator_detail: format!(
                        "resident first-visible worker queue가 재시도 뒤에도 가득 찼어요: key={worker_key}"
                    ),
                }),
                Err(TrySendError::Disconnected(_)) => Err(RenderWorkerError {
                    reason_code: "render-queue-unavailable",
                    customer_message: safe_render_failure_message(RenderIntent::Preview),
                    operator_detail: format!(
                        "resident first-visible worker를 다시 시작하지 못했어요: key={worker_key}"
                    ),
                }),
            }
        }
    }
}

fn ensure_resident_preview_worker(
    worker_key: &str,
) -> Result<ResidentPreviewWorkerHandle, RenderWorkerError> {
    let mut workers = RESIDENT_PREVIEW_WORKERS
        .lock()
        .map_err(|_| RenderWorkerError {
            reason_code: "render-queue-unavailable",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: "resident preview worker registry mutex를 잠그지 못했어요.".into(),
        })?;

    if let Some(handle) = workers.get(worker_key).cloned() {
        return Ok(handle);
    }

    let generation = RESIDENT_PREVIEW_WORKER_GENERATION.fetch_add(1, Ordering::Relaxed);
    let (sender, receiver) = mpsc::sync_channel(RESIDENT_PREVIEW_WORKER_QUEUE_CAPACITY);
    let worker_key_owned = worker_key.to_string();

    thread::spawn(move || {
        run_resident_preview_worker(worker_key_owned, generation, receiver);
    });

    let handle = ResidentPreviewWorkerHandle { generation, sender };
    workers.insert(worker_key.to_string(), handle.clone());

    Ok(handle)
}

fn remove_resident_preview_worker(worker_key: &str, generation: u64) {
    let Ok(mut workers) = RESIDENT_PREVIEW_WORKERS.lock() else {
        return;
    };

    if workers
        .get(worker_key)
        .map(|handle| handle.generation == generation)
        .unwrap_or(false)
    {
        workers.remove(worker_key);
    }
}

fn run_resident_preview_worker(
    worker_key: String,
    generation: u64,
    receiver: mpsc::Receiver<ResidentPreviewWorkerJob>,
) {
    loop {
        match receiver.recv_timeout(resident_preview_worker_idle_timeout()) {
            Ok(ResidentPreviewWorkerJob::Render(job)) => {
                run_resident_preview_render_job(job);
            }
            Err(RecvTimeoutError::Timeout) => {
                remove_resident_preview_worker(&worker_key, generation);
                break;
            }
            Err(RecvTimeoutError::Disconnected) => {
                remove_resident_preview_worker(&worker_key, generation);
                break;
            }
        }
    }
}

fn run_resident_preview_render_job(job: ResidentPreviewRenderJob) {
    resident_preview_worker_test_delay();

    log_render_start_in_dir(
        &job.base_dir,
        &job.session_id,
        &job.capture_id,
        &job.request_id,
        RenderIntent::Preview,
    );

    let render_result = render_preview_asset_to_path_with_background_capacity_in_dir(
        &job.base_dir,
        &job.session_id,
        &job.request_id,
        &job.capture_id,
        &job.preset_id,
        &job.preset_version,
        &job.source_asset_path,
        &job.output_path,
    );

    match render_result {
        Ok(prepared_render) => {
            let _ = fs::write(&job.detail_path, prepared_render.detail);
        }
        Err(error) => {
            log::warn!(
                "resident_first_visible_render_failed session={} capture_id={} request_id={} reason_code={} detail={}",
                job.session_id,
                job.capture_id,
                job.request_id,
                error.reason_code,
                error.operator_detail
            );
            log_render_failure_in_dir(
                &job.base_dir,
                &job.session_id,
                &job.capture_id,
                Some(&job.request_id),
                RenderIntent::Preview,
                error.reason_code,
            );
            let _ = fs::remove_file(&job.output_path);
            let _ = fs::remove_file(&job.detail_path);
        }
    }

    if let Some(source_cleanup_path) = job.source_cleanup_path.as_ref() {
        let _ = fs::remove_file(source_cleanup_path);
    }
    let _ = fs::remove_file(&job.lock_path);
}

fn run_preview_renderer_warmup_in_dir(
    base_dir: &Path,
    session_id: &str,
    preset_id: &str,
    preset_version: &str,
) -> Result<(), RenderWorkerError> {
    let Some(_queue_guard) = try_acquire_background_render_queue_slot() else {
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

    let result = run_darktable_invocation(&invocation, RenderIntent::Preview);
    match result {
        Ok(_) => {
            let _ = validate_render_output(&warmup_output_path, RenderIntent::Preview);
            let _ = fs::remove_file(&warmup_output_path);
            log::info!(
                "preview_renderer_warmup_completed session={} preset_id={} published_version={}",
                session_id,
                preset_id,
                preset_version
            );
            Ok(())
        }
        Err(error) => Err(RenderWorkerError {
            reason_code: error.reason_code,
            customer_message: error.customer_message,
            operator_detail: format!("preview renderer warm-up failed: {}", error.operator_detail),
        }),
    }
}

fn ensure_preview_renderer_warmup_source(base_dir: &Path) -> Result<PathBuf, RenderWorkerError> {
    let warmup_source_path = base_dir
        .join(".boothy-darktable")
        .join("preview")
        .join("warmup")
        .join("preview-renderer-warmup-source.png");
    if let Some(parent) = warmup_source_path.parent() {
        fs::create_dir_all(parent).map_err(|error| RenderWorkerError {
            reason_code: "render-warmup-dir-unavailable",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!(
                "preview renderer warm-up source dir를 준비하지 못했어요: {error}"
            ),
        })?;
    }

    let needs_refresh = match fs::read(&warmup_source_path) {
        Ok(existing_bytes) => existing_bytes != PREVIEW_RENDER_WARMUP_INPUT_PNG,
        Err(_) => true,
    };

    if needs_refresh {
        fs::write(&warmup_source_path, PREVIEW_RENDER_WARMUP_INPUT_PNG).map_err(|error| {
            RenderWorkerError {
                reason_code: "render-warmup-source-write-failed",
                customer_message: safe_render_failure_message(RenderIntent::Preview),
                operator_detail: format!(
                    "preview renderer warm-up source를 쓰지 못했어요: {error}"
                ),
            }
        })?;
    }

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

fn acquire_render_queue_slot() -> Result<RenderQueueGuard, RenderWorkerError> {
    let max_in_flight_render_jobs = max_in_flight_render_jobs();
    let mut depth = RENDER_QUEUE_DEPTH.lock().map_err(|_| RenderWorkerError {
        reason_code: "render-queue-unavailable",
        customer_message: "결과 사진을 준비하지 못했어요. 가까운 직원에게 알려 주세요.".into(),
        operator_detail: "render queue mutex를 잠그지 못했어요.".into(),
    })?;

    if *depth >= max_in_flight_render_jobs {
        return Err(RenderWorkerError {
            reason_code: "render-queue-saturated",
            customer_message: "결과 사진을 준비하지 못했어요. 가까운 직원에게 알려 주세요.".into(),
            operator_detail: format!(
                "bounded render queue가 가득 찼어요. inFlight={}, max={max_in_flight_render_jobs}",
                *depth
            ),
        });
    }

    *depth += 1;

    Ok(RenderQueueGuard {})
}

fn try_acquire_background_render_queue_slot() -> Option<RenderQueueGuard> {
    let Ok(mut depth) = RENDER_QUEUE_DEPTH.lock() else {
        return None;
    };

    if *depth != 0 {
        return None;
    }

    *depth += 1;
    Some(RenderQueueGuard {})
}

fn try_acquire_resident_preview_render_queue_slot() -> Option<RenderQueueGuard> {
    let max_in_flight_render_jobs = max_in_flight_render_jobs();
    let Ok(mut depth) = RENDER_QUEUE_DEPTH.lock() else {
        return None;
    };

    if *depth >= max_in_flight_render_jobs {
        return None;
    }

    *depth += 1;
    Some(RenderQueueGuard {})
}

fn resident_preview_worker_idle_timeout() -> Duration {
    #[cfg(test)]
    {
        let override_ms = RESIDENT_PREVIEW_WORKER_TEST_IDLE_TIMEOUT_MS.load(Ordering::Relaxed);
        if override_ms > 0 {
            return Duration::from_millis(override_ms);
        }
    }

    RESIDENT_PREVIEW_WORKER_IDLE_TIMEOUT
}

#[cfg(test)]
static RESIDENT_PREVIEW_WORKER_TEST_DELAY_MS: AtomicU64 = AtomicU64::new(0);

fn resident_preview_worker_runs_inline_in_tests() -> bool {
    #[cfg(test)]
    {
        return RESIDENT_PREVIEW_WORKER_RUN_INLINE_IN_TESTS.load(Ordering::Relaxed);
    }

    #[allow(unreachable_code)]
    false
}

fn resident_preview_worker_test_delay() {
    #[cfg(test)]
    {
        let delay_ms = RESIDENT_PREVIEW_WORKER_TEST_DELAY_MS.load(Ordering::Relaxed);
        if delay_ms > 0 {
            thread::sleep(Duration::from_millis(delay_ms));
        }
    }
}

fn current_time_ms() -> Result<u64, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "render worker가 시스템 시계를 읽지 못했어요.".to_string())?
        .as_millis() as u64)
}

fn max_in_flight_render_jobs() -> usize {
    #[cfg(test)]
    {
        if let Ok(value) = std::env::var(TEST_RENDER_QUEUE_LIMIT_ENV) {
            let normalized = value.trim();
            if normalized == "0" || normalized.eq_ignore_ascii_case("unbounded") {
                return usize::MAX;
            }

            if let Ok(parsed) = normalized.parse::<usize>() {
                return parsed.max(1);
            }
        }
    }

    MAX_IN_FLIGHT_RENDER_JOBS
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

struct RenderQueueGuard {}

impl Drop for RenderQueueGuard {
    fn drop(&mut self) {
        if let Ok(mut depth) = RENDER_QUEUE_DEPTH.lock() {
            *depth = depth.saturating_sub(1);
        }
    }
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
struct DarktableBinaryResolution {
    binary: String,
    source: &'static str,
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
    let profile = approved_preview_invocation_profile();
    let render_source =
        resolve_preview_render_source(capture, paths, intent, forced_source_kind, profile);
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
    let profile = approved_preview_invocation_profile();
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
        if !profile.apply_custom_presets {
            arguments.push("--apply-custom-presets".into());
            arguments.push(DARKTABLE_APPLY_CUSTOM_PRESETS_DISABLED.into());
        }
        if profile.disable_opencl {
            arguments.push("--disable-opencl".into());
        }
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

fn approved_preview_invocation_profile() -> PreviewInvocationProfile {
    PreviewInvocationProfile::approved_booth_safe()
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
    profile: PreviewInvocationProfile,
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
        let canonical_preview_asset = paths
            .renders_previews_dir
            .join(format!("{}.jpg", capture.capture_id));

        if profile.allow_fast_preview_raster {
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

            if is_valid_render_preview_asset(&canonical_preview_asset) {
                return PreviewRenderSource {
                    asset_path: canonical_preview_asset.to_string_lossy().into_owned(),
                    kind: PreviewRenderSourceKind::FastPreviewRaster,
                };
            }
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

fn resolve_darktable_cli_binary() -> DarktableBinaryResolution {
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

fn darktable_cli_binary_candidates() -> Vec<(&'static str, PathBuf)> {
    let mut candidates = Vec::new();

    if !cfg!(windows) {
        return candidates;
    }

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

fn run_darktable_invocation(
    invocation: &DarktableInvocation,
    intent: RenderIntent,
) -> Result<DarktableInvocationResult, RenderWorkerError> {
    let stderr_log_path =
        build_darktable_stderr_log_path(&invocation.working_directory, render_stage_label(intent));
    let stderr_log = open_darktable_stderr_log(&stderr_log_path, intent)?;
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
                customer_message: safe_render_failure_message(intent),
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
                    customer_message: safe_render_failure_message(intent),
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
                    customer_message: safe_render_failure_message(intent),
                    operator_detail: format!(
                        "darktable-cli가 실패했어요: exitCode={} stderr={} logPath={}",
                        status.code().unwrap_or(-1),
                        read_darktable_stderr_log(&stderr_log_path),
                        stderr_log_path.to_string_lossy()
                    ),
                });
            }
            Ok(None) => {
                if started_at.elapsed() >= DEFAULT_RENDER_TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(RenderWorkerError {
                        reason_code: "render-process-timeout",
                        customer_message: safe_render_failure_message(intent),
                        operator_detail: format!(
                            "darktable-cli가 제한 시간 안에 끝나지 않았어요: timeoutMs={} stderr={} logPath={}",
                            DEFAULT_RENDER_TIMEOUT.as_millis(),
                            read_darktable_stderr_log(&stderr_log_path),
                            stderr_log_path.to_string_lossy()
                        ),
                    });
                }

                thread::sleep(Duration::from_millis(100));
            }
            Err(error) => {
                let _ = child.kill();
                return Err(RenderWorkerError {
                    reason_code: "render-process-state-unavailable",
                    customer_message: safe_render_failure_message(intent),
                    operator_detail: format!("render 프로세스 상태를 확인하지 못했어요: {error}"),
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
    intent: RenderIntent,
) -> Result<fs::File, RenderWorkerError> {
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).map_err(|error| RenderWorkerError {
            reason_code: "render-log-dir-unavailable",
            customer_message: safe_render_failure_message(intent),
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
            customer_message: safe_render_failure_message(intent),
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

fn validate_render_output(
    output_path: &Path,
    intent: RenderIntent,
) -> Result<(), RenderWorkerError> {
    if !output_path.is_file() {
        return Err(RenderWorkerError {
            reason_code: "render-output-missing",
            customer_message: safe_render_failure_message(intent),
            operator_detail: format!(
                "render 출력 파일이 존재하지 않아요: {}",
                output_path.to_string_lossy()
            ),
        });
    }

    let metadata = fs::metadata(output_path).map_err(|error| RenderWorkerError {
        reason_code: "render-output-unreadable",
        customer_message: safe_render_failure_message(intent),
        operator_detail: format!("render 출력 파일 metadata를 읽지 못했어요: {error}"),
    })?;

    if metadata.len() == 0 {
        return Err(RenderWorkerError {
            reason_code: "render-output-empty",
            customer_message: safe_render_failure_message(intent),
            operator_detail: format!(
                "render 출력 파일이 비어 있어요: {}",
                output_path.to_string_lossy()
            ),
        });
    }

    if !is_valid_render_preview_asset(output_path) {
        return Err(RenderWorkerError {
            reason_code: "render-output-invalid",
            customer_message: safe_render_failure_message(intent),
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

    static RESIDENT_PREVIEW_WORKER_TEST_MUTEX: LazyLock<Mutex<()>> =
        LazyLock::new(|| Mutex::new(()));

    fn unique_temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "boothy-render-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ))
    }

    struct ResidentPreviewWorkerTestGuard {
        _guard: std::sync::MutexGuard<'static, ()>,
    }

    impl ResidentPreviewWorkerTestGuard {
        fn new(run_inline: bool, idle_timeout_ms: u64) -> Self {
            let guard = RESIDENT_PREVIEW_WORKER_TEST_MUTEX
                .lock()
                .expect("resident worker test mutex should lock");
            RESIDENT_PREVIEW_WORKER_RUN_INLINE_IN_TESTS.store(run_inline, Ordering::Relaxed);
            RESIDENT_PREVIEW_WORKER_TEST_IDLE_TIMEOUT_MS.store(idle_timeout_ms, Ordering::Relaxed);
            RESIDENT_PREVIEW_WORKER_TEST_DELAY_MS.store(0, Ordering::Relaxed);
            RESIDENT_PREVIEW_WORKERS
                .lock()
                .expect("resident preview workers should lock")
                .clear();
            Self { _guard: guard }
        }
    }

    impl Drop for ResidentPreviewWorkerTestGuard {
        fn drop(&mut self) {
            RESIDENT_PREVIEW_WORKER_RUN_INLINE_IN_TESTS.store(true, Ordering::Relaxed);
            RESIDENT_PREVIEW_WORKER_TEST_IDLE_TIMEOUT_MS.store(0, Ordering::Relaxed);
            RESIDENT_PREVIEW_WORKER_TEST_DELAY_MS.store(0, Ordering::Relaxed);
            RESIDENT_PREVIEW_WORKERS
                .lock()
                .expect("resident preview workers should lock")
                .clear();
        }
    }

    fn resident_preview_worker_test_job(label: &str) -> ResidentPreviewWorkerJob {
        let temp_dir = unique_temp_dir(label);
        ResidentPreviewWorkerJob::Render(ResidentPreviewRenderJob {
            base_dir: temp_dir.clone(),
            session_id: format!("session_{label}"),
            request_id: format!("request_{label}"),
            capture_id: format!("capture_{label}"),
            preset_id: "preset_test".into(),
            preset_version: "2026.03.31".into(),
            source_asset_path: temp_dir.join("source.jpg"),
            source_cleanup_path: None,
            output_path: temp_dir.join("output.jpg"),
            detail_path: temp_dir.join("output.detail"),
            lock_path: temp_dir.join("output.lock"),
        })
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
                preview_renderer_route: None,
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
                    preview_budget_ms: 2500,
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
        assert!(invocation
            .arguments
            .contains(&"--disable-opencl".to_string()));
        assert_eq!(
            invocation.render_source_kind,
            PreviewRenderSourceKind::RawOriginal
        );
    }

    #[test]
    fn preview_invocation_reuses_a_pending_canonical_fast_preview_as_the_fallback_source() {
        let temp_dir = unique_temp_dir("pending-canonical-fast-preview");
        let session_id = "session_test";
        let paths = SessionPaths::new(&temp_dir, session_id);
        let canonical_preview_path = paths.renders_previews_dir.join("capture_test.jpg");
        let jpeg_bytes = [
            0xFF, 0xD8, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x08, 0x06, 0x06, 0x07, 0x06, 0x05, 0x08,
            0x07, 0x07, 0x07, 0x09, 0x09, 0x08, 0x0A, 0x0C, 0x14, 0x0D, 0x0C, 0x0B, 0x0B, 0x0C,
            0x19, 0x12, 0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D, 0x1A, 0x1C, 0x1C, 0x20,
            0x24, 0x2E, 0x27, 0x20, 0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28, 0x37, 0x29, 0x2C, 0x30,
            0x31, 0x34, 0x34, 0x34, 0x1F, 0x27, 0x39, 0x3D, 0x38, 0x32, 0x3C, 0x2E, 0x33, 0x34,
            0x32, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01, 0x00, 0x01, 0x01, 0x01, 0x11, 0x00,
            0xFF, 0xC4, 0x00, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01,
            0x00, 0x00, 0x3F, 0x00, 0xD2, 0xCF, 0x20, 0xFF, 0xD9,
        ];

        fs::create_dir_all(&paths.renders_previews_dir).expect("preview directory should exist");
        fs::write(&canonical_preview_path, jpeg_bytes).expect("preview fixture should be writable");

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
                preview_renderer_route: None,
                capture_id: "capture_test".into(),
                request_id: "request_test".into(),
                raw: crate::session::session_manifest::RawCaptureAsset {
                    asset_path: temp_dir
                        .join("captures")
                        .join("originals")
                        .join("capture_test.cr2")
                        .to_string_lossy()
                        .into_owned(),
                    persisted_at_ms: 100,
                },
                preview: crate::session::session_manifest::PreviewCaptureAsset {
                    asset_path: Some(canonical_preview_path.to_string_lossy().into_owned()),
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
                    fast_preview_visible_at_ms: Some(120),
                    xmp_preview_ready_at_ms: None,
                    capture_budget_ms: 1000,
                    preview_budget_ms: 2500,
                    preview_budget_state: "pending".into(),
                },
            },
            &paths,
            &paths
                .renders_previews_dir
                .join("capture_test.preview-rendering.jpg"),
            RenderIntent::Preview,
            None,
        );

        assert_eq!(
            invocation.render_source_kind,
            PreviewRenderSourceKind::FastPreviewRaster
        );
        let expected_source_path = canonical_preview_path.to_string_lossy().replace('\\', "/");
        assert_eq!(
            invocation.arguments.first().map(String::as_str),
            Some(expected_source_path.as_str())
        );

        let _ = fs::remove_dir_all(temp_dir);
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
                preview_renderer_route: None,
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
                    preview_budget_ms: 2500,
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

        assert_eq!(bytes, PREVIEW_RENDER_WARMUP_INPUT_PNG);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn preview_renderer_warmup_source_embeds_complete_png_chunks() {
        let bytes = PREVIEW_RENDER_WARMUP_INPUT_PNG;

        assert!(bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]));
        assert!(bytes.windows(4).any(|window| window == b"sRGB"));
        assert!(bytes.windows(4).any(|window| window == b"gAMA"));
        assert!(bytes.windows(4).any(|window| window == b"pHYs"));
        assert!(bytes.windows(4).any(|window| window == b"IDAT"));
        assert!(bytes.windows(4).any(|window| window == b"IEND"));
    }

    #[test]
    fn preview_renderer_warmup_source_rewrites_stale_png_bytes() {
        let temp_dir = unique_temp_dir("preview-warmup-source-refresh");
        let warmup_source = temp_dir
            .join(".boothy-darktable")
            .join("preview")
            .join("warmup")
            .join("preview-renderer-warmup-source.png");
        fs::create_dir_all(
            warmup_source
                .parent()
                .expect("warmup source should have a parent"),
        )
        .expect("warmup source parent should be creatable");
        fs::write(&warmup_source, b"broken-png").expect("stale warmup source should be writable");

        let warmup_source = ensure_preview_renderer_warmup_source(&temp_dir)
            .expect("warmup source should be refreshed");
        let bytes = fs::read(&warmup_source).expect("refreshed warmup source should be readable");

        assert_eq!(bytes, PREVIEW_RENDER_WARMUP_INPUT_PNG);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn preview_invocation_prefers_same_capture_raster_when_available() {
        let temp_dir = unique_temp_dir("preview-fast-source");
        let session_id = "session_test";
        let paths = SessionPaths::new(&temp_dir, session_id);
        fs::create_dir_all(&paths.renders_previews_dir).expect("preview dir should exist");
        fs::create_dir_all(&paths.captures_originals_dir).expect("raw dir should exist");
        let fast_preview_path = paths
            .handoff_dir
            .join("fast-preview")
            .join("capture_test.jpg");
        fs::create_dir_all(
            fast_preview_path
                .parent()
                .expect("fast preview path should have a parent"),
        )
        .expect("fast preview dir should exist");
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
                preview_renderer_route: None,
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
                    preview_budget_ms: 2500,
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
    fn preview_invocation_avoids_pending_canonical_preview_assets_during_truthful_close() {
        let temp_dir = unique_temp_dir("preview-canonical-pending-fallback");
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
                preview_renderer_route: None,
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
                    preview_budget_ms: 2500,
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
            PreviewRenderSourceKind::RawOriginal
        );
        assert!(invocation
            .arguments
            .contains(&RAW_PREVIEW_MAX_WIDTH_PX.to_string()));
        assert!(invocation
            .arguments
            .contains(&RAW_PREVIEW_MAX_HEIGHT_PX.to_string()));
        assert_eq!(
            invocation.arguments.first().map(String::as_str),
            Some(
                paths
                    .captures_originals_dir
                    .join("capture_test.cr2")
                    .to_string_lossy()
                    .replace('\\', "/")
                    .as_str()
            )
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn fast_preview_raster_invocation_uses_a_smaller_cap_than_raw_preview() {
        let temp_dir = unique_temp_dir("fast-preview-raster-cap");
        let output_path = temp_dir
            .join("renders")
            .join("previews")
            .join("capture_test.jpg");
        let source_path = temp_dir
            .join("renders")
            .join("previews")
            .join("capture_test.source.jpg");

        fs::create_dir_all(
            output_path
                .parent()
                .expect("fast preview output path should have a parent"),
        )
        .expect("fast preview output directory should exist");
        fs::write(&source_path, [0xFF, 0xD8, 0xFF, 0xE0, 0x00])
            .expect("fast preview source should be writable");

        let invocation = build_darktable_invocation_from_source(
            &temp_dir,
            PINNED_DARKTABLE_VERSION,
            &temp_dir.join("bundle").join("preview.xmp"),
            &source_path,
            &output_path,
            RenderIntent::Preview,
            PreviewRenderSourceKind::FastPreviewRaster,
        );

        assert!(invocation.arguments.contains(&"--width".to_string()));
        assert!(invocation.arguments.contains(&"--height".to_string()));
        assert!(invocation
            .arguments
            .contains(&FAST_PREVIEW_RENDER_MAX_WIDTH_PX.to_string()));
        assert!(invocation
            .arguments
            .contains(&FAST_PREVIEW_RENDER_MAX_HEIGHT_PX.to_string()));
        assert!(
            FAST_PREVIEW_RENDER_MAX_WIDTH_PX < RAW_PREVIEW_MAX_WIDTH_PX,
            "fast-preview-raster should use a smaller cap than raw-original preview"
        );
        assert!(
            FAST_PREVIEW_RENDER_MAX_HEIGHT_PX < RAW_PREVIEW_MAX_HEIGHT_PX,
            "fast-preview-raster should use a smaller cap than raw-original preview"
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

    #[test]
    fn resident_preview_worker_can_use_remaining_render_capacity() {
        let first_guard = acquire_render_queue_slot().expect("first slot should be available");
        let resident_guard = try_acquire_resident_preview_render_queue_slot();

        assert!(
            resident_guard.is_some(),
            "resident preview worker should use the remaining render slot"
        );

        drop(resident_guard);
        drop(first_guard);
    }

    #[test]
    fn resident_preview_worker_restarts_when_stale_handle_disconnects() {
        let _config = ResidentPreviewWorkerTestGuard::new(false, 0);
        let worker_key = "session_test:preset_test:2026.03.31".to_string();
        let (sender, receiver) = mpsc::sync_channel(RESIDENT_PREVIEW_WORKER_QUEUE_CAPACITY);
        let stale_generation = RESIDENT_PREVIEW_WORKER_GENERATION.fetch_add(1, Ordering::Relaxed);
        drop(receiver);

        RESIDENT_PREVIEW_WORKERS
            .lock()
            .expect("resident preview workers should lock")
            .insert(
                worker_key.clone(),
                ResidentPreviewWorkerHandle {
                    generation: stale_generation,
                    sender,
                },
            );

        enqueue_resident_preview_worker_job(
            worker_key.clone(),
            resident_preview_worker_test_job("disconnected"),
        )
        .expect("enqueue should recreate a disconnected resident worker");

        let refreshed = RESIDENT_PREVIEW_WORKERS
            .lock()
            .expect("resident preview workers should lock")
            .get(&worker_key)
            .cloned()
            .expect("worker should be re-registered");
        assert_ne!(refreshed.generation, stale_generation);
    }

    #[test]
    fn resident_preview_worker_reports_queue_saturation_for_full_async_queue() {
        let _config = ResidentPreviewWorkerTestGuard::new(false, 0);
        let worker_key = "session_test:preset_test:2026.03.31".to_string();
        let (sender, receiver) = mpsc::sync_channel(RESIDENT_PREVIEW_WORKER_QUEUE_CAPACITY);
        let generation = RESIDENT_PREVIEW_WORKER_GENERATION.fetch_add(1, Ordering::Relaxed);

        sender
            .try_send(resident_preview_worker_test_job("queued-1"))
            .expect("first queue slot should be available");
        sender
            .try_send(resident_preview_worker_test_job("queued-2"))
            .expect("second queue slot should be available");

        RESIDENT_PREVIEW_WORKERS
            .lock()
            .expect("resident preview workers should lock")
            .insert(
                worker_key.clone(),
                ResidentPreviewWorkerHandle { generation, sender },
            );

        let error = enqueue_resident_preview_worker_job(
            worker_key,
            resident_preview_worker_test_job("queue-full"),
        )
        .expect_err("enqueue should fail when the resident worker queue is full");

        assert_eq!(error.reason_code, "render-queue-saturated");
        drop(receiver);
    }

    #[test]
    fn resident_preview_worker_restarts_after_idle_timeout() {
        let _config = ResidentPreviewWorkerTestGuard::new(false, 20);
        let worker_key = "session_test:preset_test:2026.03.31";
        let first_handle = ensure_resident_preview_worker(worker_key)
            .expect("resident worker should start on first access");

        let mut removed = false;
        for _ in 0..30 {
            if !RESIDENT_PREVIEW_WORKERS
                .lock()
                .expect("resident preview workers should lock")
                .contains_key(worker_key)
            {
                removed = true;
                break;
            }

            thread::sleep(Duration::from_millis(10));
        }

        assert!(
            removed,
            "idle resident worker should tear itself down after the timeout"
        );

        let second_handle = ensure_resident_preview_worker(worker_key)
            .expect("resident worker should restart after idle teardown");
        assert_ne!(first_handle.generation, second_handle.generation);
    }
}
