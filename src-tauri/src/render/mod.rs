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

#[cfg(windows)]
use std::os::windows::process::CommandExt;
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
const PRESET_APPLIED_PREVIEW_KIND: &str = "preset-applied-preview";
const RAW_ORIGINAL_PREVIEW_KIND: &str = "raw-original";
// The truthful recent-session rail preview does not need the old 512px cap.
// Keep the render-backed close accurate, but shrink the booth-safe preview
// artifact so preset-applied replacement lands materially sooner.
const RAW_PREVIEW_MAX_WIDTH_PX: u32 = 288;
const RAW_PREVIEW_MAX_HEIGHT_PX: u32 = 288;
const FAST_PREVIEW_RENDER_MAX_WIDTH_PX: u32 = 256;
const FAST_PREVIEW_RENDER_MAX_HEIGHT_PX: u32 = 256;
const RAW_ORIGINAL_NATIVE_DECODE_MAX_WIDTH_PX: u32 = 512;
const RAW_ORIGINAL_NATIVE_DECODE_MAX_HEIGHT_PX: u32 = 512;
const FAST_PREVIEW_XMP_CACHE_DIR_NAME: &str = "xmp-cache";
const WINDOWS_HIGH_PRIORITY_CLASS: u32 = 0x0000_0080;
const FAST_PREVIEW_XMP_CACHE_SUFFIX: &str = "fast-preview";
const FAST_PREVIEW_STRIPPED_RAW_ONLY_OPERATIONS: [&str; 4] =
    ["rawprepare", "demosaic", "denoiseprofile", "hotpixels"];
const DARKTABLE_APPLY_CUSTOM_PRESETS_DISABLED: &str = "false";
const DARKTABLE_MEMORY_LIBRARY: &str = ":memory:";
const RESIDENT_PREVIEW_WORKER_QUEUE_CAPACITY: usize = 2;
const RESIDENT_PREVIEW_WORKER_IDLE_TIMEOUT: Duration = Duration::from_secs(90);
const PREVIEW_RENDER_WARMUP_SETTLE_POLL_MS: u64 = 50;
// Keep warm-up on the same JPEG raster family as the real fast-preview lane so
// the first customer-visible render does not pay a separate decoder cold-start.
#[allow(dead_code)]
const LEGACY_PREVIEW_RENDER_WARMUP_INPUT_JPEG: &[u8] = &[
    0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x01, 0x00, 0x60,
    0x00, 0x60, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x03, 0x02, 0x02, 0x03, 0x02, 0x02, 0x03,
    0x03, 0x03, 0x03, 0x04, 0x03, 0x03, 0x04, 0x05, 0x08, 0x05, 0x05, 0x04, 0x04, 0x05, 0x0A, 0x07,
    0x07, 0x06, 0x08, 0x0C, 0x0A, 0x0C, 0x0C, 0x0B, 0x0A, 0x0B, 0x0B, 0x0D, 0x0E, 0x12, 0x10, 0x0D,
    0x0E, 0x11, 0x0E, 0x0B, 0x0B, 0x10, 0x16, 0x10, 0x11, 0x13, 0x14, 0x15, 0x15, 0x15, 0x0C, 0x0F,
    0x17, 0x18, 0x16, 0x14, 0x18, 0x12, 0x14, 0x15, 0x14, 0xFF, 0xDB, 0x00, 0x43, 0x01, 0x03, 0x04,
    0x04, 0x05, 0x04, 0x05, 0x09, 0x05, 0x05, 0x09, 0x14, 0x0D, 0x0B, 0x0D, 0x14, 0x14, 0x14, 0x14,
    0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14,
    0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14,
    0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0xFF, 0xC0,
    0x00, 0x11, 0x08, 0x00, 0x20, 0x00, 0x20, 0x03, 0x01, 0x22, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11,
    0x01, 0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09,
    0x0A, 0x0B, 0xFF, 0xC4, 0x00, 0xB5, 0x10, 0x00, 0x02, 0x01, 0x03, 0x03, 0x02, 0x04, 0x03, 0x05,
    0x05, 0x04, 0x04, 0x00, 0x00, 0x01, 0x7D, 0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21,
    0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xA1, 0x08, 0x23,
    0x42, 0xB1, 0xC1, 0x15, 0x52, 0xD1, 0xF0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0A, 0x16, 0x17,
    0x18, 0x19, 0x1A, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A,
    0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A,
    0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A,
    0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99,
    0x9A, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7,
    0xB8, 0xB9, 0xBA, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4, 0xD5,
    0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xE1, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xF1,
    0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA, 0xFF, 0xC4, 0x00, 0x1F, 0x01, 0x00, 0x03,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
    0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0xFF, 0xC4, 0x00, 0xB5, 0x11, 0x00,
    0x02, 0x01, 0x02, 0x04, 0x04, 0x03, 0x04, 0x07, 0x05, 0x04, 0x04, 0x00, 0x01, 0x02, 0x77, 0x00,
    0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61, 0x71, 0x13,
    0x22, 0x32, 0x81, 0x08, 0x14, 0x42, 0x91, 0xA1, 0xB1, 0xC1, 0x09, 0x23, 0x33, 0x52, 0xF0, 0x15,
    0x62, 0x72, 0xD1, 0x0A, 0x16, 0x24, 0x34, 0xE1, 0x25, 0xF1, 0x17, 0x18, 0x19, 0x1A, 0x26, 0x27,
    0x28, 0x29, 0x2A, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49,
    0x4A, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,
    0x6A, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88,
    0x89, 0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6,
    0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA, 0xC2, 0xC3, 0xC4,
    0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xE2,
    0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9,
    0xFA, 0xFF, 0xDA, 0x00, 0x0C, 0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3F, 0x00, 0xE1,
    0x6D, 0xB4, 0xDE, 0x9C, 0x56, 0xB5, 0xB6, 0x9B, 0xD3, 0x8A, 0xD2, 0xB6, 0xD3, 0x7A, 0x71, 0x5A,
    0xD6, 0xDA, 0x6F, 0x4E, 0x2B, 0xD9, 0xF6, 0xA7, 0x85, 0x82, 0xC6, 0x6D, 0xA9, 0x9B, 0x6D, 0xA6,
    0xF4, 0xE2, 0xB4, 0x62, 0xB2, 0xC7, 0x00, 0x73, 0x5A, 0x51, 0x59, 0x6D, 0xE0, 0x0E, 0x6B, 0x42,
    0xDB, 0x4F, 0xE9, 0xF2, 0xD7, 0x97, 0x89, 0xC7, 0x5B, 0xDC, 0x83, 0x3F, 0x40, 0xC0, 0xE2, 0xF6,
    0x6D, 0x94, 0x2D, 0xB4, 0xDE, 0x9C, 0x56, 0x8C, 0x56, 0x38, 0xE0, 0x0E, 0x7D, 0x6B, 0x4E, 0x2B,
    0x1C, 0x70, 0xA3, 0x9F, 0x5A, 0xD0, 0xB6, 0xD3, 0xBA, 0x71, 0x5E, 0x56, 0x27, 0x1B, 0x6F, 0x72,
    0x0C, 0xFC, 0x27, 0x03, 0x8B, 0xD9, 0xB6, 0x66, 0xDB, 0x69, 0xDD, 0x3E, 0x5A, 0xD5, 0xB6, 0xD3,
    0xFA, 0x7C, 0xB5, 0xA7, 0x6D, 0xA7, 0x74, 0xF9, 0x6B, 0x56, 0xDB, 0x4F, 0xE9, 0xF2, 0xD7, 0x91,
    0xED, 0x4F, 0xD0, 0x30, 0x58, 0xCD, 0xB5, 0x3F, 0xFF, 0xD9,
];

static RENDER_QUEUE_DEPTH: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));
static FAST_PREVIEW_XMP_CACHE_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
static FAST_PREVIEW_XMP_CACHE_WRITE_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
static PREVIEW_RENDER_WARMUP_INPUT_JPEG: LazyLock<Vec<u8>> =
    LazyLock::new(build_preview_renderer_warmup_input_jpeg);
static PREVIEW_RENDER_WARMUP_IN_FLIGHT: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));
static PREVIEW_RENDER_WARMUP_RESULTS: LazyLock<Mutex<HashMap<String, bool>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
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
    pub preview_kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderWorkerError {
    pub reason_code: &'static str,
    pub customer_message: String,
    pub operator_detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreviewRenderSourceKind {
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
            // Small truthful-close previews pay more for OpenCL startup than they gain
            // from GPU acceleration, so keep the booth-visible preview path CPU-only.
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
    source_kind: PreviewRenderSourceKind,
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
        Some(&render_ready_event_detail(
            intent,
            invocation.render_source_kind,
            bundle.preset_id,
            bundle.published_version,
            invocation.binary,
            invocation.binary_source,
            render_elapsed_ms,
            render_invocation_detail_with_source(intent, Some(invocation.render_source_kind)),
            invocation.arguments.join(" "),
            invocation_result.exit_code,
        )),
    );

    Ok(RenderedCaptureAsset {
        asset_path: output_path.to_string_lossy().into_owned(),
        ready_at_ms,
        preview_kind: preview_kind_for_render_source(intent, invocation.render_source_kind),
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
    render_preview_asset_to_path_with_source_kind_in_dir(
        base_dir,
        session_id,
        request_id,
        capture_id,
        preset_id,
        preset_version,
        source_asset_path,
        PreviewRenderSourceKind::FastPreviewRaster,
        output_path,
    )
}

pub(crate) fn render_preview_asset_to_path_with_source_kind_in_dir(
    base_dir: &Path,
    session_id: &str,
    request_id: &str,
    capture_id: &str,
    preset_id: &str,
    preset_version: &str,
    source_asset_path: &Path,
    source_kind: PreviewRenderSourceKind,
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
        source_kind,
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
    source_kind: PreviewRenderSourceKind,
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
        source_kind,
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
    source_kind: PreviewRenderSourceKind,
    output_path: &Path,
) -> Result<PreparedPreviewRender, RenderWorkerError> {
    let bundle =
        resolve_runtime_bundle_in_dir(base_dir, preset_id, preset_version, RenderIntent::Preview)?;

    if !is_supported_host_owned_native_preview_source(source_asset_path, source_kind) {
        return Err(RenderWorkerError {
            reason_code: "invalid-preview-source",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!(
                "speculative preview source가 host-owned native preview 입력으로 유효하지 않아요: {}",
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

    if matches!(source_kind, PreviewRenderSourceKind::RawOriginal)
        && is_camera_raw_asset_path(source_asset_path)
    {
        return render_resident_full_preset_preview_to_path(
            base_dir,
            &bundle,
            source_asset_path,
            output_path,
        );
    }

    let render_detail =
        render_invocation_detail_with_source(RenderIntent::Preview, Some(source_kind));
    log::info!(
        "speculative_preview_render_started session={} capture_id={} request_id={} binary={} source={} detail={}",
        session_id,
        capture_id,
        request_id,
        "host-owned-native-preview",
        "host-owned-native",
        render_detail
    );

    let render_started = Instant::now();
    let xmp_source =
        fs::read_to_string(&bundle.xmp_template_path).map_err(|error| RenderWorkerError {
            reason_code: "native-preview-preset-unreadable",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!("native preview preset XMP를 읽지 못했어요: {error}"),
        })?;
    let truth_profile_detail = host_owned_native_truth_profile_detail(source_kind, &xmp_source);
    render_host_owned_native_preset_preview_to_path(
        &bundle.xmp_template_path,
        source_asset_path,
        output_path,
    )?;
    validate_render_output(output_path, RenderIntent::Preview)?;
    let render_elapsed_ms = render_started.elapsed().as_millis();

    Ok(PreparedPreviewRender {
        detail: resident_preview_handoff_ready_detail(
            &bundle.preset_id,
            &bundle.published_version,
            render_elapsed_ms,
            "host-owned-native-preview",
            "host-owned-native",
            &format!("{render_detail};{truth_profile_detail}"),
            &host_owned_native_preview_arguments(source_asset_path, output_path, source_kind),
            0,
        ),
    })
}

fn render_resident_full_preset_preview_to_path(
    base_dir: &Path,
    bundle: &PublishedPresetRuntimeBundle,
    source_asset_path: &Path,
    output_path: &Path,
) -> Result<PreparedPreviewRender, RenderWorkerError> {
    let render_started = Instant::now();
    let invocation = build_darktable_invocation_from_source(
        base_dir,
        &bundle.darktable_version,
        &bundle.xmp_template_path,
        source_asset_path,
        output_path,
        RenderIntent::Preview,
        PreviewRenderSourceKind::RawOriginal,
    );
    let invocation_result = run_darktable_invocation(&invocation, RenderIntent::Preview)?;
    validate_render_output(output_path, RenderIntent::Preview)?;
    let render_elapsed_ms = render_started.elapsed().as_millis();
    let render_detail = format!(
        "{};truthProfile=original-full-preset;engineMode=per-capture-cli;engineAdapter=darktable-compatible;engineAdapterSource={}",
        render_invocation_detail_with_source(
            RenderIntent::Preview,
            Some(PreviewRenderSourceKind::RawOriginal),
        ),
        invocation.binary_source
    );

    Ok(PreparedPreviewRender {
        detail: resident_preview_handoff_ready_detail(
            &bundle.preset_id,
            &bundle.published_version,
            render_elapsed_ms,
            &invocation.binary,
            "host-owned-native",
            &render_detail,
            &invocation.arguments.join(" "),
            invocation_result.exit_code,
        ),
    })
}

fn is_supported_host_owned_native_preview_source(
    source_asset_path: &Path,
    source_kind: PreviewRenderSourceKind,
) -> bool {
    if is_valid_render_preview_asset(source_asset_path) {
        return true;
    }

    matches!(source_kind, PreviewRenderSourceKind::RawOriginal)
        && is_camera_raw_asset_path(source_asset_path)
}

fn render_host_owned_native_preset_preview_to_path(
    xmp_template_path: &Path,
    source_asset_path: &Path,
    output_path: &Path,
) -> Result<(), RenderWorkerError> {
    let source_image = load_host_owned_native_source_image(source_asset_path)?;
    let xmp_source = fs::read_to_string(xmp_template_path).map_err(|error| RenderWorkerError {
        reason_code: "native-preview-preset-unreadable",
        customer_message: safe_render_failure_message(RenderIntent::Preview),
        operator_detail: format!("native preview preset XMP를 읽지 못했어요: {error}"),
    })?;
    let profile = host_owned_native_preview_profile_from_xmp(&xmp_source);
    let mut preview = image::imageops::resize(
        &source_image,
        FAST_PREVIEW_RENDER_MAX_WIDTH_PX,
        FAST_PREVIEW_RENDER_MAX_HEIGHT_PX,
        image::imageops::FilterType::Triangle,
    );
    apply_host_owned_native_preview_profile(&mut preview, profile);

    let temp_output_path = output_path.with_extension("native-rendering.jpg");
    let mut encoded = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, 86);
    encoder
        .encode(
            preview.as_raw(),
            preview.width(),
            preview.height(),
            image::ColorType::Rgb8.into(),
        )
        .map_err(|error| RenderWorkerError {
            reason_code: "native-preview-encode-failed",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!("native preview artifact를 encode하지 못했어요: {error}"),
        })?;
    fs::write(&temp_output_path, encoded).map_err(|error| RenderWorkerError {
        reason_code: "native-preview-write-failed",
        customer_message: safe_render_failure_message(RenderIntent::Preview),
        operator_detail: format!("native preview artifact를 쓰지 못했어요: {error}"),
    })?;
    fs::rename(&temp_output_path, output_path).map_err(|error| RenderWorkerError {
        reason_code: "native-preview-promote-failed",
        customer_message: safe_render_failure_message(RenderIntent::Preview),
        operator_detail: format!("native preview artifact를 승격하지 못했어요: {error}"),
    })?;

    Ok(())
}

fn load_host_owned_native_source_image(
    source_asset_path: &Path,
) -> Result<image::RgbImage, RenderWorkerError> {
    match image::ImageReader::open(source_asset_path) {
        Ok(reader) => match reader.decode() {
            Ok(image) => return Ok(image.to_rgb8()),
            Err(image_error) => {
                if !is_camera_raw_asset_path(source_asset_path) {
                    return Err(RenderWorkerError {
                        reason_code: "native-preview-source-decode-failed",
                        customer_message: safe_render_failure_message(RenderIntent::Preview),
                        operator_detail: format!(
                            "native preview source를 decode하지 못했어요: {image_error}"
                        ),
                    });
                }
                let raw_image =
                    rawloader::decode_file(source_asset_path).map_err(|raw_error| {
                        RenderWorkerError {
                            reason_code: "native-preview-raw-decode-failed",
                            customer_message: safe_render_failure_message(RenderIntent::Preview),
                            operator_detail: format!(
                                "native preview RAW source를 decode하지 못했어요: image={image_error}; rawloader={raw_error}"
                            ),
                        }
                    })?;
                rawloader_image_to_rgb_preview(
                    &raw_image,
                    RAW_ORIGINAL_NATIVE_DECODE_MAX_WIDTH_PX,
                    RAW_ORIGINAL_NATIVE_DECODE_MAX_HEIGHT_PX,
                )
                .map_err(|error| RenderWorkerError {
                    reason_code: "native-preview-raw-convert-failed",
                    customer_message: safe_render_failure_message(RenderIntent::Preview),
                    operator_detail: format!(
                        "native preview RAW source를 RGB preview로 변환하지 못했어요: {error}"
                    ),
                })
            }
        },
        Err(open_error) => Err(RenderWorkerError {
            reason_code: "native-preview-source-unreadable",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!("native preview source를 열지 못했어요: {open_error}"),
        }),
    }
}

fn is_camera_raw_asset_path(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref(),
        Some("cr2" | "crw" | "dng" | "nef" | "arw" | "rw2" | "orf" | "raf" | "pef")
    )
}

fn rawloader_image_to_rgb_preview(
    raw_image: &rawloader::RawImage,
    max_width: u32,
    max_height: u32,
) -> Result<image::RgbImage, String> {
    if raw_image.width == 0 || raw_image.height == 0 || max_width == 0 || max_height == 0 {
        return Err("invalid dimensions".into());
    }
    let raw_data = match &raw_image.data {
        rawloader::RawImageData::Integer(data) => data,
        rawloader::RawImageData::Float(_) => {
            return Err("float RAW data is not supported for booth preview".into())
        }
    };
    let expected_len = raw_image
        .width
        .checked_mul(raw_image.height)
        .and_then(|pixels| pixels.checked_mul(raw_image.cpp.max(1)))
        .ok_or_else(|| "RAW dimensions overflow".to_string())?;
    if raw_data.len() < expected_len {
        return Err(format!(
            "RAW data is shorter than expected: actual={} expected={expected_len}",
            raw_data.len()
        ));
    }

    let crop_top = raw_image.crops[0].min(raw_image.height.saturating_sub(1));
    let crop_right = raw_image.crops[1].min(raw_image.width.saturating_sub(1));
    let crop_bottom = raw_image.crops[2].min(raw_image.height.saturating_sub(1));
    let crop_left = raw_image.crops[3].min(raw_image.width.saturating_sub(1));
    let usable_width = raw_image
        .width
        .saturating_sub(crop_left)
        .saturating_sub(crop_right)
        .max(1);
    let usable_height = raw_image
        .height
        .saturating_sub(crop_top)
        .saturating_sub(crop_bottom)
        .max(1);
    let (target_width, target_height) = scale_dimensions_to_fit(
        usable_width as u32,
        usable_height as u32,
        max_width,
        max_height,
    );

    let mut preview = image::RgbImage::new(target_width, target_height);
    for target_y in 0..target_height {
        for target_x in 0..target_width {
            let source_x = crop_left
                + (((target_x as f32 + 0.5) * usable_width as f32 / target_width as f32).floor()
                    as usize)
                    .min(usable_width.saturating_sub(1));
            let source_y = crop_top
                + (((target_y as f32 + 0.5) * usable_height as f32 / target_height as f32).floor()
                    as usize)
                    .min(usable_height.saturating_sub(1));
            let rgb = if raw_image.cpp >= 3 {
                rawloader_rgb_pixel(raw_image, raw_data, source_x, source_y)
            } else {
                rawloader_bayer_pixel(raw_image, raw_data, source_x, source_y)
            };
            preview.put_pixel(target_x, target_y, image::Rgb(rgb));
        }
    }

    Ok(preview)
}

fn scale_dimensions_to_fit(width: u32, height: u32, max_width: u32, max_height: u32) -> (u32, u32) {
    if width <= max_width && height <= max_height {
        return (width.max(1), height.max(1));
    }

    let scale = (max_width as f32 / width.max(1) as f32)
        .min(max_height as f32 / height.max(1) as f32)
        .max(0.001);
    (
        ((width as f32 * scale).round() as u32).max(1),
        ((height as f32 * scale).round() as u32).max(1),
    )
}

fn rawloader_rgb_pixel(
    raw_image: &rawloader::RawImage,
    raw_data: &[u16],
    source_x: usize,
    source_y: usize,
) -> [u8; 3] {
    let base = (source_y * raw_image.width + source_x) * raw_image.cpp;
    [
        rawloader_channel_to_u8(raw_data[base], raw_image, 0),
        rawloader_channel_to_u8(raw_data[base + 1], raw_image, 1),
        rawloader_channel_to_u8(raw_data[base + 2], raw_image, 2),
    ]
}

fn rawloader_bayer_pixel(
    raw_image: &rawloader::RawImage,
    raw_data: &[u16],
    source_x: usize,
    source_y: usize,
) -> [u8; 3] {
    let mut sums = [0.0f32; 3];
    let mut counts = [0.0f32; 3];
    let min_x = source_x.saturating_sub(2);
    let max_x = (source_x + 2).min(raw_image.width.saturating_sub(1));
    let min_y = source_y.saturating_sub(2);
    let max_y = (source_y + 2).min(raw_image.height.saturating_sub(1));

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let raw_color = raw_image.cfa.color_at(y, x).min(3);
            let rgb_channel = match raw_color {
                0 => 0,
                2 => 2,
                _ => 1,
            };
            let value = raw_data[y * raw_image.width + x];
            sums[rgb_channel] += rawloader_channel_to_unit(value, raw_image, raw_color);
            counts[rgb_channel] += 1.0;
        }
    }

    let average = |channel: usize| -> f32 {
        if counts[channel] > 0.0 {
            sums[channel] / counts[channel]
        } else {
            let total_count: f32 = counts.iter().sum();
            if total_count > 0.0 {
                sums.iter().sum::<f32>() / total_count
            } else {
                0.0
            }
        }
    };

    [
        (average(0).clamp(0.0, 1.0) * 255.0).round() as u8,
        (average(1).clamp(0.0, 1.0) * 255.0).round() as u8,
        (average(2).clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}

fn rawloader_channel_to_u8(value: u16, raw_image: &rawloader::RawImage, channel: usize) -> u8 {
    (rawloader_channel_to_unit(value, raw_image, channel).clamp(0.0, 1.0) * 255.0).round() as u8
}

fn rawloader_channel_to_unit(value: u16, raw_image: &rawloader::RawImage, channel: usize) -> f32 {
    let black = raw_image.blacklevels[channel.min(3)] as f32;
    let white =
        raw_image.whitelevels[channel.min(3)].max(raw_image.blacklevels[channel.min(3)] + 1) as f32;
    let wb = raw_image.wb_coeffs[channel.min(3)].max(0.01);
    let wb_exposure_floor = raw_image.wb_coeffs.iter().copied().fold(0.01f32, f32::max);
    (((value as f32 - black) / (white - black)) * (wb / wb_exposure_floor)).clamp(0.0, 1.0)
}

#[derive(Debug, Clone, Copy)]
struct HostOwnedNativePreviewProfile {
    exposure: f32,
    contrast: f32,
    saturation: f32,
    warmth: f32,
    sigmoid: bool,
}

fn host_owned_native_preview_profile_from_xmp(xmp_source: &str) -> HostOwnedNativePreviewProfile {
    let operation_count = |operation: &str| {
        xmp_source
            .matches(&format!("darktable:operation=\"{operation}\""))
            .count() as f32
    };
    let exposure_count = operation_count("exposure").min(2.0);
    let haze_count = operation_count("hazeremoval").min(1.0);
    let channel_mix_count = operation_count("channelmixerrgb").min(1.0);
    let temperature_count = operation_count("temperature").min(1.0);

    HostOwnedNativePreviewProfile {
        exposure: 1.0 + (exposure_count * 0.035),
        contrast: 1.0 + (haze_count * 0.08),
        saturation: 1.0 + (channel_mix_count * 0.06),
        warmth: 1.0 + (temperature_count * 0.03),
        sigmoid: operation_count("sigmoid") > 0.0,
    }
}

fn apply_host_owned_native_preview_profile(
    preview: &mut image::RgbImage,
    profile: HostOwnedNativePreviewProfile,
) {
    for pixel in preview.pixels_mut() {
        let mut r = f32::from(pixel[0]) / 255.0;
        let mut g = f32::from(pixel[1]) / 255.0;
        let mut b = f32::from(pixel[2]) / 255.0;

        r *= profile.exposure * profile.warmth;
        g *= profile.exposure;
        b *= profile.exposure / profile.warmth;

        let luma = (0.2126 * r) + (0.7152 * g) + (0.0722 * b);
        r = luma + ((r - luma) * profile.saturation);
        g = luma + ((g - luma) * profile.saturation);
        b = luma + ((b - luma) * profile.saturation);

        r = ((r - 0.5) * profile.contrast) + 0.5;
        g = ((g - 0.5) * profile.contrast) + 0.5;
        b = ((b - 0.5) * profile.contrast) + 0.5;

        if profile.sigmoid {
            r = sigmoid_preview_tone(r);
            g = sigmoid_preview_tone(g);
            b = sigmoid_preview_tone(b);
        }

        *pixel = image::Rgb([
            float_channel_to_u8(r),
            float_channel_to_u8(g),
            float_channel_to_u8(b),
        ]);
    }
}

fn sigmoid_preview_tone(value: f32) -> f32 {
    1.0 / (1.0 + (-6.0 * (value - 0.5)).exp())
}

fn float_channel_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn host_owned_native_preview_arguments(
    source_asset_path: &Path,
    output_path: &Path,
    source_kind: PreviewRenderSourceKind,
) -> String {
    let profile = match source_kind {
        PreviewRenderSourceKind::RawOriginal => "raw-original-derived",
        PreviewRenderSourceKind::FastPreviewRaster => "operation-derived",
    };
    format!(
        "source={} output={} profile={}",
        source_asset_path.to_string_lossy().replace('\\', "/"),
        output_path.to_string_lossy().replace('\\', "/"),
        profile
    )
}

fn host_owned_native_truth_profile_detail(
    source_kind: PreviewRenderSourceKind,
    xmp_source: &str,
) -> String {
    match source_kind {
        PreviewRenderSourceKind::FastPreviewRaster => {
            "truthProfile=operation-derived-comparison".into()
        }
        PreviewRenderSourceKind::RawOriginal => {
            let unsupported_operations =
                unsupported_host_owned_native_preset_operations(xmp_source);
            if unsupported_operations.is_empty() {
                "truthProfile=host-owned-native-preview-comparison;truthBlocker=full-preset-parity-unverified".into()
            } else {
                format!(
                    "truthProfile=unsupported-preset-comparison;truthBlocker=unsupported-preset-operations;unsupportedOperations={}",
                    unsupported_operations.join(",")
                )
            }
        }
    }
}

fn unsupported_host_owned_native_preset_operations(xmp_source: &str) -> Vec<String> {
    let supported = [
        "cacorrectrgb",
        "channelmixerrgb",
        "colorin",
        "colorout",
        "demosaic",
        "denoiseprofile",
        "exposure",
        "flip",
        "gamma",
        "hazeremoval",
        "highlights",
        "hotpixels",
        "lens",
        "rawprepare",
        "sigmoid",
        "temperature",
    ];
    let mut unsupported = Vec::new();
    for operation in extract_darktable_operations(xmp_source) {
        if !supported.contains(&operation.as_str()) && !unsupported.contains(&operation) {
            unsupported.push(operation);
        }
    }
    unsupported
}

fn extract_darktable_operations(xmp_source: &str) -> Vec<String> {
    xmp_source
        .split("darktable:operation=\"")
        .skip(1)
        .filter_map(|part| {
            part.split_once('"')
                .map(|(operation, _)| operation.to_string())
        })
        .collect()
}

pub fn prime_preview_worker_runtime_in_dir(base_dir: &Path, _session_id: &str) {
    let worker_root = base_dir.join(".boothy-darktable").join("preview");
    let _ = fs::create_dir_all(worker_root.join("warmup"));
    let _ = ensure_preview_renderer_warmup_source(base_dir);
}

pub(crate) fn enqueue_resident_preview_render_in_dir(
    base_dir: &Path,
    session_id: &str,
    request_id: &str,
    capture_id: &str,
    preset_id: &str,
    preset_version: &str,
    source_asset_path: &Path,
    source_kind: PreviewRenderSourceKind,
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
        source_kind,
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
        record_preview_render_warmup_result(&warmup_key, true);
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
        let warmup_succeeded = result.is_ok();
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
        let warmup_key = build_preview_render_warmup_key(&session_id, &preset_id, &preset_version);
        record_preview_render_warmup_result(&warmup_key, warmup_succeeded);
        clear_preview_render_warmup_in_flight(&warmup_key);
    });
}

pub fn wait_for_preview_renderer_warmup_to_settle(
    session_id: &str,
    preset_id: &str,
    preset_version: &str,
    timeout: Duration,
) -> bool {
    let warmup_key = build_preview_render_warmup_key(session_id, preset_id, preset_version);
    let deadline = Instant::now()
        .checked_add(timeout)
        .unwrap_or_else(Instant::now);

    loop {
        if !preview_renderer_warmup_is_in_flight(&warmup_key) {
            return preview_renderer_warmup_succeeded(&warmup_key);
        }

        if Instant::now() >= deadline {
            return false;
        }

        thread::sleep(Duration::from_millis(PREVIEW_RENDER_WARMUP_SETTLE_POLL_MS));
    }
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

fn render_ready_event_detail(
    intent: RenderIntent,
    source_kind: PreviewRenderSourceKind,
    preset_id: String,
    published_version: String,
    binary: String,
    binary_source: &'static str,
    render_elapsed_ms: u128,
    render_detail: String,
    arguments: String,
    exit_code: i32,
) -> String {
    let normalized_detail = match (intent, source_kind) {
        (RenderIntent::Preview, PreviewRenderSourceKind::FastPreviewRaster) => render_detail
            .replace(
                "sourceAsset=fast-preview-raster",
                "inputSourceAsset=fast-preview-raster;sourceAsset=preset-applied-preview",
            ),
        _ => render_detail,
    };

    format!(
        "presetId={preset_id};publishedVersion={published_version};binary={binary};source={binary_source};elapsedMs={render_elapsed_ms};detail={normalized_detail};args={arguments};status={exit_code}"
    )
}

fn resident_preview_handoff_ready_detail(
    preset_id: &str,
    published_version: &str,
    render_elapsed_ms: u128,
    engine_binary: &str,
    engine_source: &str,
    render_detail: &str,
    arguments: &str,
    exit_code: i32,
) -> String {
    let normalized_detail = render_detail
        .replace(
            "sourceAsset=fast-preview-raster",
            "inputSourceAsset=fast-preview-raster;sourceAsset=preset-applied-preview",
        )
        .replace(
            "sourceAsset=raw-original",
            "inputSourceAsset=raw-original;sourceAsset=preset-applied-preview",
        );
    let normalized_detail = if normalized_detail.contains("truthOwner=") {
        normalized_detail
    } else {
        format!("{normalized_detail};truthOwner=display-sized-preset-applied")
    };
    let normalized_detail = if normalized_detail.contains("inputSourceAsset=fast-preview-raster")
        && !normalized_detail.contains("truthProfile=")
    {
        format!("{normalized_detail};truthProfile=operation-derived-comparison")
    } else {
        normalized_detail
    };
    let normalized_detail = if normalized_detail.contains("inputSourceAsset=fast-preview-raster")
        && !normalized_detail.contains("truthBlocker=")
    {
        format!("{normalized_detail};truthBlocker=fast-preview-raster-input;requiredInputSourceAsset=raw-original")
    } else {
        normalized_detail
    };

    format!(
        "presetId={preset_id};publishedVersion={published_version};binary=fast-preview-handoff;source=fast-preview-handoff;elapsedMs={render_elapsed_ms};detail={normalized_detail};engineBinary={engine_binary};engineSource={engine_source};args={arguments};status={exit_code}"
    )
}

fn preview_kind_for_render_source(
    intent: RenderIntent,
    source_kind: PreviewRenderSourceKind,
) -> Option<String> {
    match intent {
        RenderIntent::Preview => Some(match source_kind {
            PreviewRenderSourceKind::RawOriginal => RAW_ORIGINAL_PREVIEW_KIND.into(),
            PreviewRenderSourceKind::FastPreviewRaster => PRESET_APPLIED_PREVIEW_KIND.into(),
        }),
        RenderIntent::Final => None,
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

    let inserted = in_flight.insert(key.to_string());
    if inserted {
        clear_preview_render_warmup_result(key);
    }
    inserted
}

fn clear_preview_render_warmup_in_flight(key: &str) {
    if let Ok(mut in_flight) = PREVIEW_RENDER_WARMUP_IN_FLIGHT.lock() {
        in_flight.remove(key);
    }
}

fn preview_renderer_warmup_is_in_flight(key: &str) -> bool {
    PREVIEW_RENDER_WARMUP_IN_FLIGHT
        .lock()
        .map(|in_flight| in_flight.contains(key))
        .unwrap_or(false)
}

fn record_preview_render_warmup_result(key: &str, succeeded: bool) {
    if let Ok(mut results) = PREVIEW_RENDER_WARMUP_RESULTS.lock() {
        results.insert(key.to_string(), succeeded);
    }
}

fn clear_preview_render_warmup_result(key: &str) {
    if let Ok(mut results) = PREVIEW_RENDER_WARMUP_RESULTS.lock() {
        results.remove(key);
    }
}

fn preview_renderer_warmup_succeeded(key: &str) -> bool {
    PREVIEW_RENDER_WARMUP_RESULTS
        .lock()
        .ok()
        .and_then(|results| results.get(key).copied())
        .unwrap_or(false)
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
        job.source_kind,
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
        return Err(RenderWorkerError {
            reason_code: "render-warmup-skipped",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: "preview renderer warm-up skipped because the render queue was busy"
                .into(),
        });
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
            validate_render_output(&warmup_output_path, RenderIntent::Preview)?;
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
        .join("preview-renderer-warmup-source.jpg");
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
        Ok(existing_bytes) => existing_bytes != *PREVIEW_RENDER_WARMUP_INPUT_JPEG,
        Err(_) => true,
    };

    if needs_refresh {
        fs::write(
            &warmup_source_path,
            PREVIEW_RENDER_WARMUP_INPUT_JPEG.as_slice(),
        )
        .map_err(|error| RenderWorkerError {
            reason_code: "render-warmup-source-write-failed",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!("preview renderer warm-up source를 쓰지 못했어요: {error}"),
        })?;
    }

    Ok(warmup_source_path)
}

fn build_preview_renderer_warmup_input_jpeg() -> Vec<u8> {
    let image = image::RgbImage::from_fn(
        FAST_PREVIEW_RENDER_MAX_WIDTH_PX,
        FAST_PREVIEW_RENDER_MAX_HEIGHT_PX,
        |x, y| {
            let base = 96 + (((x + y) % 96) as u8);
            image::Rgb([base, base.saturating_add(16), base.saturating_add(28)])
        },
    );
    let mut bytes = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 85);
    encoder
        .encode(
            image.as_raw(),
            FAST_PREVIEW_RENDER_MAX_WIDTH_PX,
            FAST_PREVIEW_RENDER_MAX_HEIGHT_PX,
            image::ColorType::Rgb8.into(),
        )
        .expect("built-in preview warm-up JPEG should encode");
    bytes
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
    let mut depth = RENDER_QUEUE_DEPTH.lock().map_err(|_| RenderWorkerError {
        reason_code: "render-queue-unavailable",
        customer_message: "결과 사진을 준비하지 못했어요. 가까운 직원에게 알려 주세요.".into(),
        operator_detail: "render queue mutex를 잠그지 못했어요.".into(),
    })?;

    if *depth >= MAX_IN_FLIGHT_RENDER_JOBS {
        return Err(RenderWorkerError {
            reason_code: "render-queue-saturated",
            customer_message: "결과 사진을 준비하지 못했어요. 가까운 직원에게 알려 주세요.".into(),
            operator_detail: format!(
                "bounded render queue가 가득 찼어요. inFlight={}, max={MAX_IN_FLIGHT_RENDER_JOBS}",
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
    let Ok(mut depth) = RENDER_QUEUE_DEPTH.lock() else {
        return None;
    };

    if *depth >= MAX_IN_FLIGHT_RENDER_JOBS {
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
    let effective_xmp_template_path = preview_invocation_xmp_template_path(
        base_dir,
        xmp_template_path,
        intent,
        render_source_kind,
    );
    let mode = match intent {
        RenderIntent::Preview => "preview",
        RenderIntent::Final => "final",
    };
    let worker_root = base_dir.join(".boothy-darktable").join(mode);
    let configdir = worker_root.join("config");
    let hq_flag = match intent {
        RenderIntent::Preview => "false",
        RenderIntent::Final => "true",
    };
    let library = match intent {
        // Preview renders do not need persistent catalog state, so keep the
        // library in memory and avoid extra per-invocation sqlite startup work.
        RenderIntent::Preview => DARKTABLE_MEMORY_LIBRARY.to_string(),
        RenderIntent::Final => worker_root
            .join("library.db")
            .to_string_lossy()
            .replace('\\', "/"),
    };
    let binary_resolution = resolve_darktable_cli_binary();
    let mut arguments = vec![
        darktable_cli_path_argument(source_asset_path),
        darktable_cli_path_argument(&effective_xmp_template_path),
        darktable_cli_path_argument(output_path),
        "--hq".into(),
        hq_flag.into(),
    ];

    if matches!(intent, RenderIntent::Preview) {
        if !profile.apply_custom_presets {
            arguments.push("--apply-custom-presets".into());
            arguments.push(DARKTABLE_APPLY_CUSTOM_PRESETS_DISABLED.into());
        }
        let (width_cap, height_cap) = preview_render_dimensions(render_source_kind);
        arguments.push("--width".into());
        arguments.push(width_cap.to_string());
        arguments.push("--height".into());
        arguments.push(height_cap.to_string());
    }

    arguments.push("--core".into());
    if matches!(intent, RenderIntent::Preview) && profile.disable_opencl {
        arguments.push("--disable-opencl".into());
    }
    arguments.extend([
        "--configdir".into(),
        darktable_cli_path_argument(&configdir),
        "--library".into(),
        library,
    ]);

    DarktableInvocation {
        binary: binary_resolution.binary,
        binary_source: binary_resolution.source,
        render_source_kind,
        arguments,
        working_directory: base_dir.to_path_buf(),
    }
}

fn darktable_cli_path_argument(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    normalized
        .strip_prefix("//?/")
        .unwrap_or(&normalized)
        .to_string()
}

fn preview_invocation_xmp_template_path(
    base_dir: &Path,
    xmp_template_path: &Path,
    intent: RenderIntent,
    render_source_kind: PreviewRenderSourceKind,
) -> PathBuf {
    if matches!(intent, RenderIntent::Preview)
        && matches!(
            render_source_kind,
            PreviewRenderSourceKind::FastPreviewRaster
        )
    {
        return prepare_fast_preview_xmp_template_in_dir(base_dir, xmp_template_path)
            .unwrap_or_else(|| xmp_template_path.to_path_buf());
    }

    xmp_template_path.to_path_buf()
}

fn prepare_fast_preview_xmp_template_in_dir(
    base_dir: &Path,
    xmp_template_path: &Path,
) -> Option<PathBuf> {
    let source_xmp = fs::read_to_string(xmp_template_path).ok()?;
    let trimmed_xmp = trim_xmp_for_fast_preview(&source_xmp)?;
    if trimmed_xmp == source_xmp {
        return Some(xmp_template_path.to_path_buf());
    }

    let cache_path = build_fast_preview_xmp_cache_path(base_dir, xmp_template_path);
    if fs::read_to_string(&cache_path)
        .ok()
        .as_deref()
        .map(|existing| existing == trimmed_xmp.as_str())
        .unwrap_or(false)
    {
        return Some(cache_path);
    }

    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent).ok()?;
    }
    write_fast_preview_xmp_cache_atomically(&cache_path, &trimmed_xmp).ok()?;
    Some(cache_path)
}

fn build_fast_preview_xmp_cache_path(base_dir: &Path, xmp_template_path: &Path) -> PathBuf {
    let stem = xmp_template_path
        .file_stem()
        .and_then(|value| value.to_str())
        .map(sanitize_xmp_cache_segment)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "preview".into());
    let published_version = xmp_template_path
        .parent()
        .and_then(Path::parent)
        .and_then(|value| value.file_name())
        .and_then(|value| value.to_str())
        .map(sanitize_xmp_cache_segment)
        .filter(|value| !value.is_empty());
    let preset_id = xmp_template_path
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .and_then(|value| value.file_name())
        .and_then(|value| value.to_str())
        .map(sanitize_xmp_cache_segment)
        .filter(|value| !value.is_empty());

    let mut file_name_segments = Vec::new();
    if let Some(preset_id) = preset_id {
        file_name_segments.push(preset_id);
    }
    if let Some(published_version) = published_version {
        file_name_segments.push(published_version);
    }
    file_name_segments.push(stem);
    file_name_segments.push(stable_fast_preview_xmp_cache_identity(xmp_template_path));
    file_name_segments.push(FAST_PREVIEW_XMP_CACHE_SUFFIX.into());

    base_dir
        .join(".boothy-darktable")
        .join("preview")
        .join(FAST_PREVIEW_XMP_CACHE_DIR_NAME)
        .join(format!("{}.xmp", file_name_segments.join("-")))
}

fn stable_fast_preview_xmp_cache_identity(xmp_template_path: &Path) -> String {
    let normalized_path = xmp_template_path.to_string_lossy().replace('\\', "/");
    let mut hash = 0xcbf29ce484222325u64;
    for byte in normalized_path.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn write_fast_preview_xmp_cache_atomically(
    cache_path: &Path,
    trimmed_xmp: &str,
) -> std::io::Result<()> {
    let _write_guard = FAST_PREVIEW_XMP_CACHE_WRITE_LOCK
        .lock()
        .map_err(|_| std::io::Error::other("fast preview xmp cache write lock poisoned"))?;
    let temp_path = fast_preview_xmp_cache_temp_path(cache_path);
    fs::write(&temp_path, trimmed_xmp)?;
    let result = replace_file_atomically(&temp_path, cache_path);
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn fast_preview_xmp_cache_temp_path(cache_path: &Path) -> PathBuf {
    let counter = FAST_PREVIEW_XMP_CACHE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    cache_path.with_extension(format!("xmp.{}.{}.tmp", std::process::id(), counter))
}

#[cfg(windows)]
fn replace_file_atomically(source_path: &Path, target_path: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x0000_0001;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x0000_0008;

    #[link(name = "kernel32")]
    extern "system" {
        fn MoveFileExW(
            lpExistingFileName: *const u16,
            lpNewFileName: *const u16,
            dwFlags: u32,
        ) -> i32;
    }

    let source = source_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let target = target_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();

    let moved = unsafe {
        MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file_atomically(source_path: &Path, target_path: &Path) -> std::io::Result<()> {
    fs::rename(source_path, target_path)
}

fn sanitize_xmp_cache_segment(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn trim_xmp_for_fast_preview(source_xmp: &str) -> Option<String> {
    let builtin_default_duplicate_operations =
        find_fast_preview_builtin_default_duplicate_operations(source_xmp);
    let mut trimmed_history = String::new();
    let mut kept_history_count = 0usize;
    let mut captured_history = false;
    let mut in_history_sequence = false;
    let mut history_block = String::new();
    let mut collecting_history_block = false;
    let mut kept_builtin_auto_signatures = HashSet::new();
    let mut kept_iop_order_pairs = HashSet::new();

    for segment in source_xmp.split_inclusive('\n') {
        if !captured_history {
            if segment.contains("<darktable:history>") {
                captured_history = true;
            }
            trimmed_history.push_str(segment);
            continue;
        }

        if !in_history_sequence {
            if segment.contains("<rdf:Seq>") {
                in_history_sequence = true;
            }
            trimmed_history.push_str(segment);
            continue;
        }

        if collecting_history_block || segment.trim_start().starts_with("<rdf:li") {
            collecting_history_block = true;
            history_block.push_str(segment);
            if !darktable_history_block_is_complete(&history_block) {
                continue;
            }

            collecting_history_block = false;
            let repeated_builtin_auto = fast_preview_builtin_auto_signature(&history_block)
                .map(|signature| !kept_builtin_auto_signatures.insert(signature))
                .unwrap_or(false);
            if !repeated_builtin_auto
                && !should_strip_fast_preview_history_block(
                    &history_block,
                    &builtin_default_duplicate_operations,
                )
            {
                let rewritten_block =
                    rewrite_darktable_history_num(&history_block, kept_history_count);
                trimmed_history.push_str(&rewritten_block);
                if let Some(operation_priority) =
                    fast_preview_history_operation_priority(&history_block)
                {
                    kept_iop_order_pairs.insert(operation_priority);
                }
                kept_history_count += 1;
            }
            history_block.clear();
            continue;
        }

        if segment.contains("</rdf:Seq>") {
            in_history_sequence = false;
        }
        trimmed_history.push_str(segment);
    }

    if collecting_history_block {
        return None;
    }

    let history_end = if kept_history_count == 0 {
        "-1".to_string()
    } else {
        kept_history_count.saturating_sub(1).to_string()
    };
    let with_history_end =
        rewrite_xml_attribute_value_once(&trimmed_history, "darktable:history_end", &history_end);
    let with_iop_order = rewrite_darktable_iop_order_list(&with_history_end, &kept_iop_order_pairs);
    Some(with_iop_order)
}

fn find_fast_preview_builtin_default_duplicate_operations(source_xmp: &str) -> HashSet<String> {
    let mut builtin_default_operations = HashSet::new();
    let mut user_operations = HashSet::new();

    for history_block in extract_darktable_history_blocks(source_xmp) {
        let Some(operation) = extract_darktable_history_operation(&history_block) else {
            continue;
        };

        if is_fast_preview_builtin_default_history_block(&history_block) {
            builtin_default_operations.insert(operation.to_string());
        } else {
            user_operations.insert(operation.to_string());
        }
    }

    builtin_default_operations
        .intersection(&user_operations)
        .cloned()
        .collect()
}

fn extract_darktable_history_blocks(source_xmp: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut captured_history = false;
    let mut in_history_sequence = false;
    let mut history_block = String::new();
    let mut collecting_history_block = false;

    for segment in source_xmp.split_inclusive('\n') {
        if !captured_history {
            if segment.contains("<darktable:history>") {
                captured_history = true;
            }
            continue;
        }

        if !in_history_sequence {
            if segment.contains("<rdf:Seq>") {
                in_history_sequence = true;
            }
            continue;
        }

        if collecting_history_block || segment.trim_start().starts_with("<rdf:li") {
            collecting_history_block = true;
            history_block.push_str(segment);
            if !darktable_history_block_is_complete(&history_block) {
                continue;
            }

            collecting_history_block = false;
            blocks.push(history_block.clone());
            history_block.clear();
        }
    }

    blocks
}

fn darktable_history_block_is_complete(history_block: &str) -> bool {
    history_block.contains("/>") || history_block.contains("</rdf:li>")
}

fn should_strip_fast_preview_history_block(
    history_block: &str,
    builtin_default_duplicate_operations: &HashSet<String>,
) -> bool {
    extract_darktable_history_operation(history_block)
        .map(|operation| {
            FAST_PREVIEW_STRIPPED_RAW_ONLY_OPERATIONS
                .iter()
                .any(|candidate| operation == *candidate)
                || (builtin_default_duplicate_operations.contains(operation)
                    && is_fast_preview_builtin_default_history_block(history_block))
        })
        .unwrap_or(false)
}

fn is_fast_preview_builtin_default_history_block(history_block: &str) -> bool {
    extract_xml_attribute_value(history_block, "darktable:multi_name")
        .map(|multi_name| multi_name.starts_with("_builtin_scene-referred default"))
        .unwrap_or(false)
}

fn fast_preview_builtin_auto_signature(history_block: &str) -> Option<String> {
    let multi_name = extract_xml_attribute_value(history_block, "darktable:multi_name")?;
    if !multi_name.starts_with("_builtin_auto") {
        return None;
    }

    let operation = extract_darktable_history_operation(history_block)?;
    let params = extract_xml_attribute_value(history_block, "darktable:params").unwrap_or("");
    Some(format!("{operation}\0{multi_name}\0{params}"))
}

fn extract_darktable_history_operation(history_block: &str) -> Option<&str> {
    extract_xml_attribute_value(history_block, "darktable:operation")
}

fn fast_preview_history_operation_priority(history_block: &str) -> Option<String> {
    let operation = extract_darktable_history_operation(history_block)?;
    let priority = extract_xml_attribute_value(history_block, "darktable:multi_priority")
        .filter(|value| !value.is_empty())
        .unwrap_or("0");
    Some(format!("{operation}\0{priority}"))
}

fn rewrite_darktable_history_num(history_block: &str, new_num: usize) -> String {
    rewrite_xml_attribute_value_once(history_block, "darktable:num", &new_num.to_string())
}

fn rewrite_darktable_iop_order_list(
    source_xmp: &str,
    kept_iop_order_pairs: &HashSet<String>,
) -> String {
    let Some(existing_iop_order_list) =
        extract_xml_attribute_value(source_xmp, "darktable:iop_order_list")
    else {
        return source_xmp.to_string();
    };

    let mut tokens = existing_iop_order_list.split(',');
    let mut kept_tokens = Vec::new();
    let mut kept_operation_priorities = HashSet::new();
    while let Some(operation) = tokens.next() {
        let Some(priority) = tokens.next() else {
            kept_tokens.push(operation.to_string());
            break;
        };

        if FAST_PREVIEW_STRIPPED_RAW_ONLY_OPERATIONS
            .iter()
            .any(|candidate| operation == *candidate)
        {
            continue;
        }

        let operation_priority = format!("{operation}\0{priority}");
        if !kept_iop_order_pairs.is_empty() && !kept_iop_order_pairs.contains(&operation_priority) {
            continue;
        }
        if !kept_operation_priorities.insert(operation_priority) {
            continue;
        }

        kept_tokens.push(operation.to_string());
        kept_tokens.push(priority.to_string());
    }

    rewrite_xml_attribute_value_once(
        source_xmp,
        "darktable:iop_order_list",
        &kept_tokens.join(","),
    )
}

fn extract_xml_attribute_value<'a>(source: &'a str, attribute_name: &str) -> Option<&'a str> {
    let prefix = format!("{attribute_name}=\"");
    let start = source.find(&prefix)? + prefix.len();
    let remaining = source.get(start..)?;
    let end = remaining.find('"')?;
    remaining.get(..end)
}

fn rewrite_xml_attribute_value_once(
    source: &str,
    attribute_name: &str,
    replacement_value: &str,
) -> String {
    let prefix = format!("{attribute_name}=\"");
    let Some(start) = source.find(&prefix) else {
        return source.to_string();
    };
    let value_start = start + prefix.len();
    let Some(remaining) = source.get(value_start..) else {
        return source.to_string();
    };
    let Some(value_end_offset) = remaining.find('"') else {
        return source.to_string();
    };
    let value_end = value_start + value_end_offset;

    let mut rewritten = String::with_capacity(
        source
            .len()
            .saturating_sub(value_end.saturating_sub(value_start))
            + replacement_value.len(),
    );
    rewritten.push_str(&source[..value_start]);
    rewritten.push_str(replacement_value);
    rewritten.push_str(&source[value_end..]);
    rewritten
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
    let mut command = Command::new(&invocation.binary);
    command
        .args(&invocation.arguments)
        .current_dir(&invocation.working_directory)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(stderr_log));
    configure_darktable_process(&mut command, intent);

    let mut child = command.spawn().map_err(|error| {
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

                thread::sleep(render_process_poll_interval(intent));
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

fn render_process_poll_interval(intent: RenderIntent) -> Duration {
    match intent {
        RenderIntent::Preview => Duration::from_millis(5),
        RenderIntent::Final => Duration::from_millis(100),
    }
}

fn darktable_process_creation_flags(intent: RenderIntent) -> u32 {
    match intent {
        RenderIntent::Preview => WINDOWS_HIGH_PRIORITY_CLASS,
        RenderIntent::Final => 0,
    }
}

fn configure_darktable_process(command: &mut Command, intent: RenderIntent) {
    let creation_flags = darktable_process_creation_flags(intent);
    if creation_flags == 0 {
        return;
    }

    #[cfg(windows)]
    {
        command.creation_flags(creation_flags);
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

    struct EnvVarTestGuard {
        key: &'static str,
        previous_value: Option<String>,
    }

    impl EnvVarTestGuard {
        fn set_path(key: &'static str, value: &Path) -> Self {
            let previous_value = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self {
                key,
                previous_value,
            }
        }
    }

    impl Drop for EnvVarTestGuard {
        fn drop(&mut self) {
            match self.previous_value.as_ref() {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
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
            source_kind: PreviewRenderSourceKind::FastPreviewRaster,
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
                    kind: None,
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
        assert!(invocation
            .arguments
            .windows(2)
            .any(|pair| pair[0] == "--library" && pair[1] == DARKTABLE_MEMORY_LIBRARY));
        assert!(invocation
            .arguments
            .contains(&"--disable-opencl".to_string()));
        let core_index = invocation
            .arguments
            .iter()
            .position(|argument| argument == "--core")
            .expect("darktable core separator should be present");
        let disable_opencl_index = invocation
            .arguments
            .iter()
            .position(|argument| argument == "--disable-opencl")
            .expect("preview invocation should disable OpenCL");
        assert!(
            disable_opencl_index > core_index,
            "darktable only treats --disable-opencl as a core option after --core"
        );
        assert_eq!(
            invocation.render_source_kind,
            PreviewRenderSourceKind::RawOriginal
        );
    }

    #[test]
    fn darktable_invocation_strips_windows_extended_path_prefixes() {
        let temp_dir = unique_temp_dir("darktable-extended-path-prefix");
        let invocation = build_darktable_invocation_from_source(
            &temp_dir,
            PINNED_DARKTABLE_VERSION,
            Path::new(r"\\?\C:\boothy\preset\look2.xmp"),
            Path::new(r"\\?\C:\boothy\sessions\capture.CR2"),
            Path::new(r"\\?\C:\boothy\sessions\capture.jpg"),
            RenderIntent::Preview,
            PreviewRenderSourceKind::RawOriginal,
        );

        assert_eq!(invocation.arguments[0], "C:/boothy/sessions/capture.CR2");
        assert_eq!(invocation.arguments[1], "C:/boothy/preset/look2.xmp");
        assert_eq!(invocation.arguments[2], "C:/boothy/sessions/capture.jpg");
        assert!(
            invocation
                .arguments
                .iter()
                .all(|argument| !argument.contains("//?/")),
            "darktable-cli does not accept //?/ prefixed source paths"
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn rawloader_bayer_image_converts_to_rgb_preview_pixels() {
        let raw_image = rawloader::RawImage {
            make: "Canon".into(),
            model: "EOS 700D".into(),
            clean_make: "canon".into(),
            clean_model: "eos700d".into(),
            width: 4,
            height: 4,
            cpp: 1,
            wb_coeffs: [1.0, 1.0, 1.0, 1.0],
            whitelevels: [1023, 1023, 1023, 1023],
            blacklevels: [0, 0, 0, 0],
            xyz_to_cam: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 0.0],
            ],
            cfa: rawloader::CFA::new("RGGB"),
            crops: [0, 0, 0, 0],
            blackareas: Vec::new(),
            orientation: rawloader::Orientation::Normal,
            data: rawloader::RawImageData::Integer(vec![
                1023, 512, 1023, 512, 512, 128, 512, 128, 1023, 512, 1023, 512, 512, 128, 512, 128,
            ]),
        };

        let preview = rawloader_image_to_rgb_preview(&raw_image, 2, 2)
            .expect("rawloader image should convert to rgb preview");

        assert_eq!(preview.dimensions(), (2, 2));
        assert!(
            preview.pixels().any(|pixel| pixel[0] > pixel[2]),
            "RGGB sample should preserve a red-dominant signal"
        );
    }

    #[test]
    fn rawloader_white_balance_does_not_clip_midtones_to_white() {
        let raw_image = rawloader::RawImage {
            make: "Canon".into(),
            model: "EOS 700D".into(),
            clean_make: "canon".into(),
            clean_model: "eos700d".into(),
            width: 1,
            height: 1,
            cpp: 1,
            wb_coeffs: [4.0, 2.0, 1.0, 1.0],
            whitelevels: [1000, 1000, 1000, 1000],
            blacklevels: [0, 0, 0, 0],
            xyz_to_cam: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 0.0],
            ],
            cfa: rawloader::CFA::new("RGGB"),
            crops: [0, 0, 0, 0],
            blackareas: Vec::new(),
            orientation: rawloader::Orientation::Normal,
            data: rawloader::RawImageData::Integer(vec![600]),
        };

        let normalized_red = rawloader_channel_to_unit(600, &raw_image, 0);

        assert!(
            normalized_red < 0.95,
            "white balance should not clip a midtone RAW sample to display white"
        );
        assert!(normalized_red > 0.5);
    }

    #[test]
    fn env_raw_original_fixture_decodes_with_host_owned_native_loader() {
        let Ok(raw_path) = std::env::var("BOOTHY_RAW_ORIGINAL_FIXTURE") else {
            return;
        };

        let preview = load_host_owned_native_source_image(Path::new(&raw_path))
            .expect("raw original fixture should decode through the host-owned native loader");

        assert!(preview.width() > 0);
        assert!(preview.height() > 0);
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
                    kind: None,
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
        assert!(invocation.arguments.windows(2).any(|pair| {
            pair[0] == "--library"
                && pair[1]
                    == temp_dir
                        .join(".boothy-darktable")
                        .join("final")
                        .join("library.db")
                        .to_string_lossy()
                        .replace('\\', "/")
        }));
    }

    #[test]
    fn preview_renderer_warmup_source_is_written_as_jpeg() {
        let temp_dir = unique_temp_dir("preview-warmup-source");
        let warmup_source = ensure_preview_renderer_warmup_source(&temp_dir)
            .expect("warmup source should be creatable");
        let bytes = fs::read(&warmup_source).expect("warmup source should be readable");

        assert_eq!(
            warmup_source.extension().and_then(|value| value.to_str()),
            Some("jpg")
        );
        assert_eq!(bytes, *PREVIEW_RENDER_WARMUP_INPUT_JPEG);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn preview_renderer_warmup_source_is_decodable_jpeg() {
        let image = image::load_from_memory(PREVIEW_RENDER_WARMUP_INPUT_JPEG.as_slice())
            .expect("warm-up source must be a valid JPEG raster");

        assert_eq!(image.width(), FAST_PREVIEW_RENDER_MAX_WIDTH_PX);
        assert_eq!(image.height(), FAST_PREVIEW_RENDER_MAX_HEIGHT_PX);
    }

    #[test]
    fn preview_renderer_warmup_source_rewrites_stale_jpeg_bytes() {
        let temp_dir = unique_temp_dir("preview-warmup-source-refresh");
        let warmup_source = temp_dir
            .join(".boothy-darktable")
            .join("preview")
            .join("warmup")
            .join("preview-renderer-warmup-source.jpg");
        fs::create_dir_all(
            warmup_source
                .parent()
                .expect("warmup source should have a parent"),
        )
        .expect("warmup source parent should be creatable");
        fs::write(&warmup_source, b"broken-jpeg").expect("stale warmup source should be writable");

        let warmup_source = ensure_preview_renderer_warmup_source(&temp_dir)
            .expect("warmup source should be refreshed");
        let bytes = fs::read(&warmup_source).expect("refreshed warmup source should be readable");

        assert_eq!(bytes, *PREVIEW_RENDER_WARMUP_INPUT_JPEG);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn preview_invocation_prefers_same_capture_raster_when_available() {
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
                    kind: None,
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
        assert!(invocation
            .arguments
            .windows(2)
            .any(|pair| pair[0] == "--library" && pair[1] == DARKTABLE_MEMORY_LIBRARY));
        assert!(invocation
            .arguments
            .contains(&"--disable-opencl".to_string()));
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
    fn preview_invocation_reuses_canonical_preview_asset_in_default_booth_safe_mode() {
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
                    kind: None,
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
        assert!(invocation
            .arguments
            .windows(2)
            .any(|pair| pair[0] == "--library" && pair[1] == DARKTABLE_MEMORY_LIBRARY));
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
    fn fast_preview_raster_uses_gate_safe_truthful_close_cap() {
        assert_eq!(FAST_PREVIEW_RENDER_MAX_WIDTH_PX, 256);
        assert_eq!(FAST_PREVIEW_RENDER_MAX_HEIGHT_PX, 256);
    }

    #[test]
    fn raw_original_preview_uses_gate_safe_truthful_close_cap() {
        assert_eq!(RAW_PREVIEW_MAX_WIDTH_PX, 288);
        assert_eq!(RAW_PREVIEW_MAX_HEIGHT_PX, 288);
    }

    #[test]
    fn fast_preview_xmp_trim_preserves_look_affecting_operations_in_history_and_iop_order() {
        let source_xmp = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description
   darktable:history_end="6"
   darktable:iop_order_list="rawprepare,0,demosaic,0,exposure,0,hotpixels,0,lens,0,hazeremoval,0,highlights,0,cacorrectrgb,0,sigmoid,0">
   <darktable:history>
    <rdf:Seq>
     <rdf:li
      darktable:num="0"
      darktable:operation="rawprepare"/>
     <rdf:li
      darktable:num="1"
      darktable:operation="demosaic"/>
     <rdf:li
      darktable:num="2"
      darktable:operation="exposure"/>
     <rdf:li
      darktable:num="3"
      darktable:operation="lens"/>
     <rdf:li
      darktable:num="4"
      darktable:operation="hazeremoval"/>
     <rdf:li
      darktable:num="5"
      darktable:operation="highlights"/>
     <rdf:li
      darktable:num="6"
      darktable:operation="cacorrectrgb"/>
     <rdf:li
      darktable:num="7"
      darktable:operation="sigmoid"/>
    </rdf:Seq>
   </darktable:history>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
"#;

        let trimmed_xmp =
            trim_xmp_for_fast_preview(source_xmp).expect("preview xmp should be trimmed");

        assert!(!trimmed_xmp.contains("darktable:operation=\"rawprepare\""));
        assert!(!trimmed_xmp.contains("darktable:operation=\"demosaic\""));
        assert!(trimmed_xmp.contains("darktable:operation=\"lens\""));
        assert!(trimmed_xmp.contains("darktable:operation=\"hazeremoval\""));
        assert!(trimmed_xmp.contains("darktable:operation=\"highlights\""));
        assert!(trimmed_xmp.contains("darktable:operation=\"cacorrectrgb\""));
        assert_eq!(
            trimmed_xmp
                .matches("darktable:operation=\"exposure\"")
                .count(),
            1
        );
        assert_eq!(
            trimmed_xmp
                .matches("darktable:operation=\"sigmoid\"")
                .count(),
            1
        );
        assert!(trimmed_xmp.contains("darktable:num=\"0\""));
        assert!(trimmed_xmp.contains("darktable:num=\"1\""));
        assert!(trimmed_xmp.contains("darktable:num=\"2\""));
        assert!(trimmed_xmp.contains("darktable:num=\"3\""));
        assert!(trimmed_xmp.contains("darktable:num=\"4\""));
        assert!(trimmed_xmp.contains("darktable:num=\"5\""));
        assert!(trimmed_xmp.contains("darktable:history_end=\"5\""));
        assert!(trimmed_xmp.contains(
            "darktable:iop_order_list=\"exposure,0,lens,0,hazeremoval,0,highlights,0,cacorrectrgb,0,sigmoid,0\""
        ));
    }

    #[test]
    fn fast_preview_xmp_trim_preserves_lens_correction_for_preview_parity() {
        let source_xmp = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description
   darktable:history_end="2"
   darktable:iop_order_list="rawprepare,0,lens,0,exposure,0">
   <darktable:history>
    <rdf:Seq>
     <rdf:li
      darktable:num="0"
      darktable:operation="rawprepare"/>
     <rdf:li
      darktable:num="1"
      darktable:operation="lens"/>
     <rdf:li
      darktable:num="2"
      darktable:operation="exposure"/>
    </rdf:Seq>
   </darktable:history>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
"#;

        let trimmed_xmp =
            trim_xmp_for_fast_preview(source_xmp).expect("preview xmp should be trimmed");

        assert!(!trimmed_xmp.contains("darktable:operation=\"rawprepare\""));
        assert!(trimmed_xmp.contains("darktable:operation=\"lens\""));
        assert!(trimmed_xmp.contains("darktable:operation=\"exposure\""));
        assert!(trimmed_xmp.contains("darktable:iop_order_list=\"lens,0,exposure,0\""));
    }

    #[test]
    fn fast_preview_xmp_cache_writes_do_not_collide_for_same_process_writers() {
        let temp_dir = unique_temp_dir("fast-preview-xmp-cache-concurrent");
        let cache_path = temp_dir.join("preview-cache.xmp");
        fs::create_dir_all(
            cache_path
                .parent()
                .expect("cache path should have a parent directory"),
        )
        .expect("cache parent should be creatable");

        let writer_count = 16usize;
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(writer_count));
        let handles = (0..writer_count)
            .map(|index| {
                let cache_path = cache_path.clone();
                let barrier = std::sync::Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    write_fast_preview_xmp_cache_atomically(
                        &cache_path,
                        &format!("<xmp>writer-{index}</xmp>"),
                    )
                })
            })
            .collect::<Vec<_>>();

        let results = handles
            .into_iter()
            .map(|handle| handle.join().expect("cache writer thread should join"))
            .collect::<Vec<_>>();

        assert!(
            results.iter().all(Result::is_ok),
            "same-process cache writers should not fail because they share a temp path: {results:?}"
        );
        let final_xmp = fs::read_to_string(&cache_path).expect("cache file should be readable");
        assert!(final_xmp.starts_with("<xmp>writer-"));
        assert!(
            fs::read_dir(&temp_dir)
                .expect("temp dir should be readable")
                .filter_map(Result::ok)
                .all(|entry| !entry.file_name().to_string_lossy().contains(".tmp")),
            "cache writer should not leave temporary files behind"
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn fast_preview_xmp_trim_removes_duplicate_builtin_defaults_when_user_work_exists() {
        let source_xmp = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description
   darktable:history_end="5"
   darktable:iop_order_list="channelmixerrgb,0,exposure,0,flip,0,sigmoid,0,channelmixerrgb,1,exposure,1,flip,1,sigmoid,1,colorout,0">
   <darktable:history>
    <rdf:Seq>
     <rdf:li
      darktable:num="0"
      darktable:operation="channelmixerrgb"
      darktable:multi_name="_builtin_scene-referred default"/>
     <rdf:li
      darktable:num="1"
      darktable:operation="exposure"
      darktable:multi_name="_builtin_scene-referred default"/>
     <rdf:li
      darktable:num="2"
      darktable:operation="flip"
      darktable:multi_name="_builtin_auto"/>
     <rdf:li
      darktable:num="3"
      darktable:operation="channelmixerrgb"
      darktable:multi_name=""/>
     <rdf:li
      darktable:num="4"
      darktable:operation="exposure"
      darktable:multi_name=""/>
     <rdf:li
      darktable:num="5"
      darktable:operation="colorout"
      darktable:multi_name=""/>
    </rdf:Seq>
   </darktable:history>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
"#;

        let trimmed_xmp =
            trim_xmp_for_fast_preview(source_xmp).expect("preview xmp should be trimmed");

        assert!(!trimmed_xmp.contains("_builtin_scene-referred default"));
        assert!(trimmed_xmp.contains("darktable:operation=\"channelmixerrgb\""));
        assert!(trimmed_xmp.contains("darktable:operation=\"exposure\""));
        assert!(trimmed_xmp.contains("darktable:operation=\"flip\""));
        assert!(trimmed_xmp.contains("darktable:operation=\"colorout\""));
        assert!(trimmed_xmp.contains("darktable:history_end=\"3\""));
        assert!(trimmed_xmp.contains(
            "darktable:iop_order_list=\"channelmixerrgb,0,exposure,0,flip,0,colorout,0\""
        ));
    }

    #[test]
    fn fast_preview_xmp_trim_removes_repeated_builtin_auto_blocks_and_iop_pairs() {
        let source_xmp = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description
   darktable:history_end="3"
   darktable:iop_order_list="flip,0,exposure,0,sigmoid,1,sigmoid,1,colorout,0">
   <darktable:history>
    <rdf:Seq>
     <rdf:li
      darktable:num="0"
      darktable:operation="flip"
      darktable:params="ffffffff"
      darktable:multi_name="_builtin_auto"/>
     <rdf:li
      darktable:num="1"
      darktable:operation="exposure"
      darktable:multi_name=""/>
     <rdf:li
      darktable:num="2"
      darktable:operation="flip"
      darktable:params="ffffffff"
      darktable:multi_name="_builtin_auto"/>
     <rdf:li
      darktable:num="3"
      darktable:operation="colorout"
      darktable:multi_name=""/>
    </rdf:Seq>
   </darktable:history>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
"#;

        let trimmed_xmp =
            trim_xmp_for_fast_preview(source_xmp).expect("preview xmp should be trimmed");

        assert_eq!(
            trimmed_xmp.matches("darktable:operation=\"flip\"").count(),
            1
        );
        assert_eq!(
            trimmed_xmp
                .matches("darktable:operation=\"exposure\"")
                .count(),
            1
        );
        assert_eq!(
            trimmed_xmp
                .matches("darktable:operation=\"colorout\"")
                .count(),
            1
        );
        assert!(trimmed_xmp.contains("darktable:history_end=\"2\""));
        assert!(trimmed_xmp.contains("darktable:iop_order_list=\"flip,0,exposure,0,colorout,0\""));
    }

    #[test]
    fn fast_preview_xmp_trim_marks_empty_history_when_all_blocks_are_removed() {
        let source_xmp = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description
   darktable:history_end="1"
   darktable:iop_order_list="rawprepare,0,demosaic,0">
   <darktable:history>
    <rdf:Seq>
     <rdf:li
      darktable:num="0"
      darktable:operation="rawprepare"/>
     <rdf:li
      darktable:num="1"
      darktable:operation="demosaic"/>
    </rdf:Seq>
   </darktable:history>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
"#;

        let trimmed_xmp =
            trim_xmp_for_fast_preview(source_xmp).expect("preview xmp should be trimmed");

        assert!(!trimmed_xmp.contains("<rdf:li"));
        assert!(trimmed_xmp.contains("darktable:history_end=\"-1\""));
        assert!(trimmed_xmp.contains("darktable:iop_order_list=\"\""));
    }

    #[test]
    fn fast_preview_xmp_trim_handles_open_close_rdf_li_blocks() {
        let source_xmp = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description
   darktable:history_end="1"
   darktable:iop_order_list="rawprepare,0,exposure,0">
   <darktable:history>
    <rdf:Seq>
     <rdf:li
      darktable:num="0"
      darktable:operation="rawprepare">
     </rdf:li>
     <rdf:li
      darktable:num="1"
      darktable:operation="exposure">
     </rdf:li>
    </rdf:Seq>
   </darktable:history>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
"#;

        let trimmed_xmp =
            trim_xmp_for_fast_preview(source_xmp).expect("preview xmp should be trimmed");

        assert!(!trimmed_xmp.contains("darktable:operation=\"rawprepare\""));
        assert_eq!(
            trimmed_xmp
                .matches("darktable:operation=\"exposure\"")
                .count(),
            1
        );
        assert!(trimmed_xmp.contains("darktable:num=\"0\""));
        assert!(trimmed_xmp.contains("darktable:history_end=\"0\""));
        assert!(trimmed_xmp.contains("darktable:iop_order_list=\"exposure,0\""));
    }

    #[test]
    fn fast_preview_xmp_cache_path_includes_stable_identity_to_avoid_slug_collisions() {
        let temp_dir = unique_temp_dir("fast-preview-xmp-cache-collision");
        let first_xmp_path = temp_dir
            .join("preset-catalog")
            .join("published")
            .join("preset_test")
            .join("2026.03.31")
            .join("xmp")
            .join("preview.xmp");
        let second_xmp_path = temp_dir
            .join("preset-catalog")
            .join("published")
            .join("preset-test")
            .join("2026.03.31")
            .join("xmp")
            .join("preview.xmp");

        let first_cache_path = build_fast_preview_xmp_cache_path(&temp_dir, &first_xmp_path);
        let second_cache_path = build_fast_preview_xmp_cache_path(&temp_dir, &second_xmp_path);

        assert_ne!(
            first_cache_path, second_cache_path,
            "different source XMP paths must not share one fast-preview cache file"
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn preview_process_poll_interval_is_tighter_than_final_polling() {
        assert!(
            render_process_poll_interval(RenderIntent::Preview) <= Duration::from_millis(5),
            "preview close should not lose a visible tail to process-exit polling"
        );
        assert!(
            render_process_poll_interval(RenderIntent::Preview)
                < render_process_poll_interval(RenderIntent::Final)
        );
    }

    #[test]
    fn preview_darktable_process_uses_latency_priority_without_final_render_priority() {
        assert_eq!(
            darktable_process_creation_flags(RenderIntent::Preview),
            WINDOWS_HIGH_PRIORITY_CLASS
        );
        assert_eq!(darktable_process_creation_flags(RenderIntent::Final), 0);
    }

    #[test]
    fn resident_preview_detail_claims_host_owned_handoff_not_darktable_fallback() {
        let detail = resident_preview_handoff_ready_detail(
            "preset_test",
            "2026.04.10",
            2_998,
            "C:\\Program Files\\darktable\\bin\\darktable-cli.exe",
            "program-files-bin",
            "widthCap=256;heightCap=256;hq=false;sourceAsset=fast-preview-raster",
            "source.jpg preset.xmp output.jpg",
            0,
        );

        assert!(detail.contains("binary=fast-preview-handoff"));
        assert!(detail.contains("source=fast-preview-handoff"));
        assert!(
            detail.contains("engineBinary=C:\\Program Files\\darktable\\bin\\darktable-cli.exe")
        );
        assert!(detail.contains("engineSource=program-files-bin"));
        assert!(detail.contains("inputSourceAsset=fast-preview-raster"));
        assert!(detail.contains("sourceAsset=preset-applied-preview"));
        assert!(detail.contains("truthOwner=display-sized-preset-applied"));
    }

    #[test]
    fn fast_preview_raster_render_uses_host_owned_native_artifact() {
        let temp_dir = unique_temp_dir("fast-preview-native-handoff");
        let bundle_xmp_path = temp_dir
            .join("preset-catalog")
            .join("published")
            .join("preset_test")
            .join("2026.03.31")
            .join("xmp")
            .join("preview.xmp");
        let output_path = temp_dir
            .join("renders")
            .join("previews")
            .join("capture_test.preview-speculative.jpg");
        let source_path = temp_dir
            .join("renders")
            .join("previews")
            .join("capture_test.source.jpg");

        fs::create_dir_all(
            bundle_xmp_path
                .parent()
                .expect("xmp path should have a parent"),
        )
        .expect("bundle xmp parent should exist");
        fs::create_dir_all(
            source_path
                .parent()
                .expect("source path should have a parent"),
        )
        .expect("source parent should exist");
        fs::write(&source_path, PREVIEW_RENDER_WARMUP_INPUT_JPEG.as_slice())
            .expect("source preview should be writable");
        fs::write(
            &bundle_xmp_path,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description darktable:history_end="2">
   <darktable:history>
    <rdf:Seq>
     <rdf:li darktable:num="0" darktable:operation="exposure"/>
     <rdf:li darktable:num="1" darktable:operation="hazeremoval"/>
     <rdf:li darktable:num="2" darktable:operation="sigmoid"/>
    </rdf:Seq>
   </darktable:history>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
"#,
        )
        .expect("bundle xmp should be writable");
        fs::write(
            bundle_xmp_path
                .parent()
                .and_then(Path::parent)
                .expect("bundle root should resolve")
                .join("preview.jpg"),
            PREVIEW_RENDER_WARMUP_INPUT_JPEG.as_slice(),
        )
        .expect("bundle preview should be writable");
        fs::write(
            bundle_xmp_path
                .parent()
                .and_then(Path::parent)
                .expect("bundle root should resolve")
                .join("bundle.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
              "schemaVersion": "published-preset-bundle/v1",
              "presetId": "preset_test",
              "displayName": "Test",
              "publishedVersion": "2026.03.31",
              "lifecycleStatus": "published",
              "boothStatus": "booth-safe",
              "darktableVersion": PINNED_DARKTABLE_VERSION,
              "xmpTemplatePath": "xmp/preview.xmp",
              "preview": {
                "kind": "preview-tile",
                "assetPath": "preview.jpg",
                "altText": "Test preview"
              }
            }))
            .expect("bundle json should serialize"),
        )
        .expect("bundle json should be writable");

        let prepared = render_preview_asset_to_path_in_dir(
            &temp_dir,
            "session_test",
            "request_test",
            "capture_test",
            "preset_test",
            "2026.03.31",
            &source_path,
            &output_path,
        )
        .expect("native fast preview render should succeed");

        assert!(is_valid_render_preview_asset(&output_path));
        assert!(prepared.detail.contains("binary=fast-preview-handoff"));
        assert!(prepared.detail.contains("engineSource=host-owned-native"));
        assert!(prepared
            .detail
            .contains("engineBinary=host-owned-native-preview"));
        assert!(prepared
            .detail
            .contains("truthBlocker=fast-preview-raster-input"));
        assert!(prepared
            .detail
            .contains("requiredInputSourceAsset=raw-original"));
        assert!(!prepared
            .detail
            .to_ascii_lowercase()
            .contains("darktable-cli"));

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn raw_original_darktable_preview_claims_per_capture_full_preset_truth_not_resident() {
        let temp_dir = unique_temp_dir("raw-original-resident-full-preset");
        let bundle_xmp_path = temp_dir
            .join("preset-catalog")
            .join("published")
            .join("preset_test")
            .join("2026.03.31")
            .join("xmp")
            .join("preview.xmp");
        let output_path = temp_dir
            .join("renders")
            .join("previews")
            .join("capture_test.preview-speculative.jpg");
        let source_path = temp_dir
            .join("captures")
            .join("originals")
            .join("capture_test.CR2");
        let fake_darktable_cli = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("support")
            .join("fake-darktable-cli.cmd");
        let _darktable_cli = EnvVarTestGuard::set_path(DARKTABLE_CLI_BIN_ENV, &fake_darktable_cli);

        fs::create_dir_all(
            bundle_xmp_path
                .parent()
                .expect("bundle xmp path should have a parent"),
        )
        .expect("bundle xmp parent should exist");
        fs::create_dir_all(
            source_path
                .parent()
                .expect("source path should have a parent"),
        )
        .expect("source parent should exist");
        fs::write(&source_path, b"fake-raw-original").expect("raw source should be writable");
        fs::write(
            &bundle_xmp_path,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description darktable:history_end="1">
   <darktable:history>
    <rdf:Seq>
     <rdf:li darktable:num="0" darktable:operation="rawprepare"/>
     <rdf:li darktable:num="1" darktable:operation="sigmoid"/>
    </rdf:Seq>
   </darktable:history>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
"#,
        )
        .expect("bundle xmp should be writable");
        fs::write(
            bundle_xmp_path
                .parent()
                .and_then(Path::parent)
                .expect("bundle root should resolve")
                .join("preview.jpg"),
            PREVIEW_RENDER_WARMUP_INPUT_JPEG.as_slice(),
        )
        .expect("bundle preview should be writable");
        fs::write(
            bundle_xmp_path
                .parent()
                .and_then(Path::parent)
                .expect("bundle root should resolve")
                .join("bundle.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
              "schemaVersion": "published-preset-bundle/v1",
              "presetId": "preset_test",
              "displayName": "Test",
              "publishedVersion": "2026.03.31",
              "lifecycleStatus": "published",
              "boothStatus": "booth-safe",
              "darktableVersion": PINNED_DARKTABLE_VERSION,
              "xmpTemplatePath": "xmp/preview.xmp",
              "preview": {
                "kind": "preview-tile",
                "assetPath": "preview.jpg",
                "altText": "Test preview"
              }
            }))
            .expect("bundle json should serialize"),
        )
        .expect("bundle json should be writable");

        let prepared = render_preview_asset_to_path_with_source_kind_in_dir(
            &temp_dir,
            "session_test",
            "request_test",
            "capture_test",
            "preset_test",
            "2026.03.31",
            &source_path,
            PreviewRenderSourceKind::RawOriginal,
            &output_path,
        )
        .expect("resident full-preset preview render should succeed");

        assert!(is_valid_render_preview_asset(&output_path));
        assert!(prepared.detail.contains("binary=fast-preview-handoff"));
        assert!(prepared.detail.contains("inputSourceAsset=raw-original"));
        assert!(prepared
            .detail
            .contains("sourceAsset=preset-applied-preview"));
        assert!(prepared
            .detail
            .contains("truthOwner=display-sized-preset-applied"));
        assert!(prepared
            .detail
            .contains("truthProfile=original-full-preset"));
        assert!(prepared.detail.contains("engineSource=host-owned-native"));
        assert!(!prepared.detail.contains("engineMode=resident-full-preset"));
        assert!(prepared.detail.contains("engineMode=per-capture-cli"));
        assert!(prepared
            .detail
            .contains("engineAdapter=darktable-compatible"));
        assert!(!prepared
            .detail
            .contains("truthBlocker=resident-engine-not-implemented"));
        assert!(!prepared
            .detail
            .contains("host-owned-native-preview-comparison"));
        assert!(!prepared
            .detail
            .contains("engineBinary=host-owned-native-preview"));

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn raw_sdk_preview_render_requires_supported_preset_before_claiming_full_preset_truth() {
        let temp_dir = unique_temp_dir("raw-sdk-preview-unsupported-preset");
        let bundle_xmp_path = temp_dir
            .join("preset-catalog")
            .join("published")
            .join("preset_test")
            .join("2026.03.31")
            .join("xmp")
            .join("preview.xmp");
        let output_path = temp_dir
            .join("renders")
            .join("previews")
            .join("capture_test.preview-speculative.jpg");
        let source_path = temp_dir
            .join("renders")
            .join("previews")
            .join("capture_test.raw-sdk-preview.jpg");

        fs::create_dir_all(
            bundle_xmp_path
                .parent()
                .expect("xmp path should have a parent"),
        )
        .expect("bundle xmp parent should exist");
        fs::create_dir_all(
            source_path
                .parent()
                .expect("source path should have a parent"),
        )
        .expect("source parent should exist");
        fs::write(&source_path, PREVIEW_RENDER_WARMUP_INPUT_JPEG.as_slice())
            .expect("source preview should be writable");
        fs::write(
            &bundle_xmp_path,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description darktable:history_end="1">
   <darktable:history>
    <rdf:Seq>
     <rdf:li darktable:num="0" darktable:operation="exposure"/>
     <rdf:li darktable:num="1" darktable:operation="retouch"/>
    </rdf:Seq>
   </darktable:history>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
"#,
        )
        .expect("bundle xmp should be writable");
        fs::write(
            bundle_xmp_path
                .parent()
                .and_then(Path::parent)
                .expect("bundle root should resolve")
                .join("preview.jpg"),
            PREVIEW_RENDER_WARMUP_INPUT_JPEG.as_slice(),
        )
        .expect("bundle preview should be writable");
        fs::write(
            bundle_xmp_path
                .parent()
                .and_then(Path::parent)
                .expect("bundle root should resolve")
                .join("bundle.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
              "schemaVersion": "published-preset-bundle/v1",
              "presetId": "preset_test",
              "displayName": "Test",
              "publishedVersion": "2026.03.31",
              "lifecycleStatus": "published",
              "boothStatus": "booth-safe",
              "darktableVersion": PINNED_DARKTABLE_VERSION,
              "xmpTemplatePath": "xmp/preview.xmp",
              "preview": {
                "kind": "preview-tile",
                "assetPath": "preview.jpg",
                "altText": "Test preview"
              }
            }))
            .expect("bundle json should serialize"),
        )
        .expect("bundle json should be writable");

        let prepared = render_preview_asset_to_path_with_source_kind_in_dir(
            &temp_dir,
            "session_test",
            "request_test",
            "capture_test",
            "preset_test",
            "2026.03.31",
            &source_path,
            PreviewRenderSourceKind::RawOriginal,
            &output_path,
        )
        .expect("native raw-sdk preview render should succeed");

        assert!(prepared.detail.contains("inputSourceAsset=raw-original"));
        assert!(prepared
            .detail
            .contains("truthBlocker=unsupported-preset-operations"));
        assert!(prepared.detail.contains("unsupportedOperations=retouch"));
        assert!(!prepared
            .detail
            .contains("truthProfile=original-full-preset"));

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn raw_original_native_preview_does_not_claim_full_preset_without_parity_engine() {
        let xmp_source = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description darktable:history_end="21">
   <darktable:history>
    <rdf:Seq>
     <rdf:li darktable:operation="channelmixerrgb"/>
     <rdf:li darktable:operation="exposure"/>
     <rdf:li darktable:operation="flip"/>
     <rdf:li darktable:operation="sigmoid"/>
     <rdf:li darktable:operation="cacorrectrgb"/>
     <rdf:li darktable:operation="colorin"/>
     <rdf:li darktable:operation="colorout"/>
     <rdf:li darktable:operation="demosaic"/>
     <rdf:li darktable:operation="denoiseprofile"/>
     <rdf:li darktable:operation="gamma"/>
     <rdf:li darktable:operation="hazeremoval"/>
     <rdf:li darktable:operation="highlights"/>
     <rdf:li darktable:operation="hotpixels"/>
     <rdf:li darktable:operation="lens"/>
     <rdf:li darktable:operation="rawprepare"/>
     <rdf:li darktable:operation="temperature"/>
    </rdf:Seq>
   </darktable:history>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
"#;

        let detail = host_owned_native_truth_profile_detail(
            PreviewRenderSourceKind::RawOriginal,
            xmp_source,
        );

        assert!(detail.contains("truthProfile=host-owned-native-preview-comparison"));
        assert!(detail.contains("truthBlocker=full-preset-parity-unverified"));
        assert!(!detail.contains("truthProfile=original-full-preset"));
    }

    #[test]
    fn fast_preview_raster_invocation_uses_a_trimmed_cached_xmp_when_source_xmp_is_available() {
        let temp_dir = unique_temp_dir("fast-preview-xmp-cache");
        let bundle_xmp_path = temp_dir
            .join("preset-catalog")
            .join("published")
            .join("preset_test")
            .join("2026.03.31")
            .join("xmp")
            .join("preview.xmp");
        let output_path = temp_dir
            .join("renders")
            .join("previews")
            .join("capture_test.jpg");
        let source_path = temp_dir
            .join("renders")
            .join("previews")
            .join("capture_test.source.jpg");
        let source_xmp = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description
   darktable:history_end="2"
   darktable:iop_order_list="rawprepare,0,exposure,0,demosaic,0">
   <darktable:history>
    <rdf:Seq>
     <rdf:li
      darktable:num="0"
      darktable:operation="rawprepare"/>
     <rdf:li
      darktable:num="1"
      darktable:operation="exposure"/>
     <rdf:li
      darktable:num="2"
      darktable:operation="demosaic"/>
    </rdf:Seq>
   </darktable:history>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
"#;

        fs::create_dir_all(
            bundle_xmp_path
                .parent()
                .expect("bundle xmp path should have a parent"),
        )
        .expect("bundle xmp parent should exist");
        fs::create_dir_all(
            output_path
                .parent()
                .expect("fast preview output path should have a parent"),
        )
        .expect("fast preview output directory should exist");
        fs::write(&bundle_xmp_path, source_xmp).expect("source xmp should be writable");
        fs::write(&source_path, [0xFF, 0xD8, 0xFF, 0xE0, 0x00])
            .expect("fast preview source should be writable");

        let invocation = build_darktable_invocation_from_source(
            &temp_dir,
            PINNED_DARKTABLE_VERSION,
            &bundle_xmp_path,
            &source_path,
            &output_path,
            RenderIntent::Preview,
            PreviewRenderSourceKind::FastPreviewRaster,
        );

        let trimmed_xmp_path = PathBuf::from(&invocation.arguments[1]);
        let trimmed_xmp = fs::read_to_string(&trimmed_xmp_path)
            .expect("trimmed fast preview xmp should be written to cache");

        assert_ne!(
            trimmed_xmp_path, bundle_xmp_path,
            "fast preview render should point at the cached trimmed xmp"
        );
        let trimmed_xmp_file_name = trimmed_xmp_path
            .file_name()
            .and_then(|value| value.to_str())
            .expect("trimmed xmp cache should have a filename");
        assert!(trimmed_xmp_file_name.starts_with("preset-test-2026-03-31-preview-"));
        assert!(trimmed_xmp_file_name.ends_with("-fast-preview.xmp"));
        assert!(!trimmed_xmp.contains("darktable:operation=\"rawprepare\""));
        assert!(!trimmed_xmp.contains("darktable:operation=\"demosaic\""));
        assert!(trimmed_xmp.contains("darktable:operation=\"exposure\""));

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
