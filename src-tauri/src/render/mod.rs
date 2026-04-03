use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{LazyLock, Mutex},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crate::{
    preset::preset_bundle::PublishedPresetRuntimeBundle,
    preset::preset_catalog::{
        find_published_preset_runtime_bundle, resolve_published_preset_catalog_dir,
    },
    session::{session_manifest::SessionCaptureRecord, session_paths::SessionPaths},
};

const PINNED_DARKTABLE_VERSION: &str = "5.4.1";
const MAX_IN_FLIGHT_RENDER_JOBS: usize = 2;
const DEFAULT_RENDER_TIMEOUT: Duration = Duration::from_secs(45);
const DARKTABLE_CLI_BIN_ENV: &str = "BOOTHY_DARKTABLE_CLI_BIN";
const RAW_PREVIEW_MAX_WIDTH_PX: u32 = 512;
const RAW_PREVIEW_MAX_HEIGHT_PX: u32 = 512;
const FAST_PREVIEW_RENDER_MAX_WIDTH_PX: u32 = 384;
const FAST_PREVIEW_RENDER_MAX_HEIGHT_PX: u32 = 384;
const DARKTABLE_APPLY_CUSTOM_PRESETS_DISABLED: &str = "false";
const PREVIEW_RENDER_WARMUP_INPUT_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

static RENDER_QUEUE_DEPTH: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));
static PREVIEW_RENDER_WARMUP_IN_FLIGHT: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

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
        arguments.push("--disable-opencl".into());
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

    fn unique_temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "boothy-render-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ))
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
}
