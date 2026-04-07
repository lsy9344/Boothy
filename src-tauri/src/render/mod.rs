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

use serde::{Deserialize, Serialize};

use crate::{
    contracts::dto::{is_valid_branch_id, HostErrorEnvelope},
    preset::preset_bundle::PublishedPresetRuntimeBundle,
    preset::preset_catalog::{
        find_published_preset_runtime_bundle, resolve_published_preset_catalog_dir,
    },
    session::{
        session_manifest::{SessionCaptureRecord, SessionManifest},
        session_paths::SessionPaths,
    },
};

const PINNED_DARKTABLE_VERSION: &str = "5.4.1";
const MAX_IN_FLIGHT_RENDER_JOBS: usize = if cfg!(test) { 64 } else { 2 };
const DEFAULT_RENDER_TIMEOUT: Duration = Duration::from_secs(45);
const DARKTABLE_CLI_BIN_ENV: &str = "BOOTHY_DARKTABLE_CLI_BIN";
const LOCAL_RENDERER_BIN_ENV: &str = "BOOTHY_LOCAL_RENDERER_BIN";
const LOCAL_RENDERER_TIMEOUT_MS_ENV: &str = "BOOTHY_LOCAL_RENDERER_TIMEOUT_MS";
const RUNTIME_BRANCH_ID_ENV: &str = "BOOTHY_BRANCH_ID";
const PREVIEW_RENDER_ROUTE_POLICY_SCHEMA_VERSION: &str = "preview-renderer-route-policy/v1";
const LOCAL_RENDERER_REQUEST_SCHEMA_VERSION: &str = "local-renderer-request/v1";
const LOCAL_RENDERER_RESPONSE_SCHEMA_VERSION: &str = "local-renderer-response/v1";
const LOCAL_RENDERER_DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const BRANCH_ROLLOUT_STORE_SCHEMA_VERSION: &str = "branch-rollout-store/v1";
const SESSION_LOCKED_PREVIEW_RENDER_ROUTE_POLICY_FILE_NAME: &str =
    "preview-renderer-policy.lock.json";
const DARKTABLE_FIDELITY_VERDICT: &str = "approved-baseline";
const DARKTABLE_FIDELITY_DETAIL: &str = "engine=darktable-cli;comparison=baseline-owner";
// The truthful recent-session rail preview must stay visually acceptable even
// while we chase latency, so keep the booth-safe fast-preview cap at the
// restored quality floor.
const RAW_PREVIEW_MAX_WIDTH_PX: u32 = 384;
const RAW_PREVIEW_MAX_HEIGHT_PX: u32 = 384;
const FAST_PREVIEW_RENDER_MAX_WIDTH_PX: u32 = 256;
const FAST_PREVIEW_RENDER_MAX_HEIGHT_PX: u32 = 256;
const DARKTABLE_APPLY_CUSTOM_PRESETS_DISABLED: &str = "false";
const RESIDENT_PREVIEW_WORKER_QUEUE_CAPACITY: usize = 2;
const RESIDENT_PREVIEW_WORKER_IDLE_TIMEOUT: Duration = Duration::from_secs(90);
const PREVIEW_RENDER_WARMUP_INPUT_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0B, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x60, 0x00, 0x02, 0x00,
    0x00, 0x05, 0x00, 0x01, 0x7A, 0x5E, 0xAB, 0x3F, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44,
    0xAE, 0x42, 0x60, 0x82,
];

static RENDER_QUEUE_DEPTH: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));
static PREVIEW_RENDER_WARMUP_IN_FLIGHT: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));
static RESIDENT_PREVIEW_WORKERS: LazyLock<Mutex<HashMap<String, ResidentPreviewWorkerHandle>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static RESIDENT_PREVIEW_WORKER_GENERATION: AtomicU64 = AtomicU64::new(1);
static PREVIEW_ROUTE_LOCK_FAILURE_INJECTION: LazyLock<Mutex<usize>> =
    LazyLock::new(|| Mutex::new(0));
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

    let should_route_preview_close =
        matches!(intent, RenderIntent::Preview) && forced_source_kind.is_none();
    let selected_route = if should_route_preview_close {
        let route = resolve_selected_preview_render_route_in_dir(base_dir, &paths, capture);
        append_render_event(
            &paths,
            &capture.capture_id,
            Some(&capture.request_id),
            RenderIntent::Preview,
            "renderer-route-selected",
            Some(route.route.as_reason_code()),
            Some(&selected_route_event_detail(&route)),
        );
        route
    } else {
        ResolvedPreviewRenderRoute {
            route: PreviewRenderRoute::Darktable,
            reason_code: "direct-render",
            fallback_reason: None,
        }
    };

    let run_darktable_route = || -> Result<(u128, String, String, String), RenderWorkerError> {
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

        Ok((
            render_elapsed_ms,
            format!(
                "presetId={};publishedVersion={};binary={};source={};elapsedMs={};detail={};args={};status={};fidelityVerdict={};fidelityDetail={}",
                bundle.preset_id,
                bundle.published_version,
                invocation.binary,
                invocation.binary_source,
                render_elapsed_ms,
                render_invocation_detail_with_source(intent, Some(invocation.render_source_kind)),
                invocation.arguments.join(" "),
                invocation_result.exit_code,
                DARKTABLE_FIDELITY_VERDICT,
                diagnostic_detail_value(Some(DARKTABLE_FIDELITY_DETAIL))
            ),
            DARKTABLE_FIDELITY_VERDICT.into(),
            DARKTABLE_FIDELITY_DETAIL.into(),
        ))
    };

    let (_render_elapsed_ms, render_detail, render_fidelity_verdict, render_fidelity_detail) =
        if should_route_preview_close
            && selected_route.route == PreviewRenderRoute::LocalRendererSidecar
        {
            let local_renderer_source_kind = resolve_preview_render_source(
                capture,
                &paths,
                RenderIntent::Preview,
                None,
                approved_preview_invocation_profile(),
            )
            .kind;
            match render_preview_via_local_renderer_sidecar_in_dir(
                base_dir,
                &paths,
                capture,
                &bundle,
                &staging_output_path,
            ) {
                Ok(candidate) => {
                    if let Err(error) =
                        promote_render_output(&candidate.candidate_path, &output_path, intent)
                    {
                        let _ = fs::remove_file(&staging_output_path);
                        return Err(error);
                    }
                    append_render_event(
                    &paths,
                    &capture.capture_id,
                    Some(&capture.request_id),
                    RenderIntent::Preview,
                    "renderer-close-owner",
                    Some(selected_route.route.as_reason_code()),
                    Some(&format!(
                        "route={};result=accepted;fidelityVerdict={};fidelityDetail={};retryOrdinal={};elapsedMs={}",
                        selected_route.route.as_reason_code(),
                        &candidate.fidelity_verdict,
                        diagnostic_detail_value(candidate.fidelity_detail.as_deref()),
                        candidate.retry_ordinal,
                        candidate.elapsed_ms
                    )),
                );
                    (
                    u128::from(candidate.elapsed_ms),
                    format!(
                        "presetId={};publishedVersion={};binary=local-renderer-sidecar;source=sidecar-candidate;elapsedMs={};detail={};route={};fidelityVerdict={};fidelityDetail={};retryOrdinal={}",
                        bundle.preset_id,
                        bundle.published_version,
                        candidate.elapsed_ms,
                        render_invocation_detail_with_source(
                            intent,
                            Some(local_renderer_source_kind),
                        ),
                        selected_route.route.as_reason_code(),
                        &candidate.fidelity_verdict,
                        diagnostic_detail_value(candidate.fidelity_detail.as_deref()),
                        candidate.retry_ordinal
                    ),
                    candidate.fidelity_verdict,
                    candidate.fidelity_detail.unwrap_or_else(|| "none".into()),
                )
                }
                Err(error) => {
                    if should_force_darktable_for_session_after_local_renderer_error(&error) {
                        if let Err(lock_error) =
                            mark_session_locked_preview_route_forced_fallback_after_local_renderer_error(
                                &paths, capture,
                            )
                        {
                            log::warn!(
                                "preview_route_forced_fallback_write_failed session={} capture_id={} request_id={} code={} detail={}",
                                capture.session_id,
                                capture.capture_id,
                                capture.request_id,
                                lock_error.reason_code,
                                lock_error.operator_detail
                            );
                        }
                    }
                    append_render_event(
                        &paths,
                        &capture.capture_id,
                        Some(&capture.request_id),
                        RenderIntent::Preview,
                        "renderer-route-fallback",
                        Some(error.reason_code),
                        Some(&format!(
                            "from={};to=darktable;reasonDetail={}",
                            selected_route.route.as_reason_code(),
                            diagnostic_detail_value(Some(error.operator_detail.as_str()))
                        )),
                    );
                    let _ = fs::remove_file(&staging_output_path);
                    let darktable_result = run_darktable_route()?;
                    append_render_event(
                    &paths,
                    &capture.capture_id,
                    Some(&capture.request_id),
                    RenderIntent::Preview,
                    "renderer-close-owner",
                    Some(PreviewRenderRoute::Darktable.as_reason_code()),
                    Some(&format!(
                        "route=darktable;result=fallback-accepted;selectedRoute={};fidelityVerdict={};fidelityDetail={}",
                        selected_route.route.as_reason_code(),
                        DARKTABLE_FIDELITY_VERDICT,
                        diagnostic_detail_value(Some(DARKTABLE_FIDELITY_DETAIL))
                    )),
                );
                    darktable_result
                }
            }
        } else {
            let darktable_result = run_darktable_route()?;
            if should_route_preview_close {
                append_render_event(
                    &paths,
                    &capture.capture_id,
                    Some(&capture.request_id),
                    RenderIntent::Preview,
                    "renderer-close-owner",
                    Some(PreviewRenderRoute::Darktable.as_reason_code()),
                    Some(&format!(
                        "route=darktable;result=accepted;fidelityVerdict={};fidelityDetail={}",
                        DARKTABLE_FIDELITY_VERDICT,
                        diagnostic_detail_value(Some(DARKTABLE_FIDELITY_DETAIL))
                    )),
                );
            }
            darktable_result
        };

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
            "{};closeOwnerFidelityVerdict={};closeOwnerFidelityDetail={}",
            render_detail,
            render_fidelity_verdict,
            diagnostic_detail_value(Some(&render_fidelity_detail))
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
    // Keep the local-renderer canary scoped to the truthful close hot path.
    // Speculative first-visible work stays on the approved darktable baseline.
    let selected_route = ResolvedPreviewRenderRoute {
        route: PreviewRenderRoute::Darktable,
        reason_code: "speculative-baseline",
        fallback_reason: None,
    };

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
        detail: with_speculative_route_metadata(
            format!(
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
            &selected_route,
            None,
            None,
            PreviewRenderRoute::Darktable,
            "accepted",
            DARKTABLE_FIDELITY_VERDICT,
            DARKTABLE_FIDELITY_DETAIL,
        ),
    })
}

fn resolve_local_renderer_binary(base_dir: &Path) -> Option<PathBuf> {
    if let Some(value) = std::env::var_os(LOCAL_RENDERER_BIN_ENV) {
        return Some(PathBuf::from(value));
    }

    for candidate in local_renderer_binary_candidates(base_dir) {
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

fn local_renderer_binary_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(current_dir) = current_exe.parent() {
            candidates.push(current_dir.join("local-renderer-sidecar.cmd"));
            candidates.push(
                current_dir
                    .join("local-renderer")
                    .join("local-renderer-sidecar.cmd"),
            );
            candidates.push(local_renderer_sidecar_relative_path(current_dir));
            candidates.push(
                current_dir
                    .join("resources")
                    .join("_up_")
                    .join("sidecar")
                    .join("local-renderer")
                    .join("local-renderer-sidecar.cmd"),
            );
            candidates.push(
                current_dir
                    .join("resources")
                    .join("sidecar")
                    .join("local-renderer")
                    .join("local-renderer-sidecar.cmd"),
            );
        }
    }

    candidates.push(local_renderer_sidecar_relative_path(base_dir));
    let repo_root_candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    candidates.push(local_renderer_sidecar_relative_path(&repo_root_candidate));
    candidates
}

fn local_renderer_sidecar_relative_path(root: &Path) -> PathBuf {
    root.join("sidecar")
        .join("local-renderer")
        .join("local-renderer-sidecar.cmd")
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalRendererErrorResponse {
    schema_version: String,
    error: LocalRendererErrorPayload,
}

#[derive(Debug, Clone, Deserialize)]
struct LocalRendererErrorPayload {
    message: String,
}

fn run_local_renderer_sidecar(
    binary_path: &Path,
    request_path: &Path,
    response_path: &Path,
    darktable_cli_env_value: Option<&str>,
) -> Result<LocalRendererSuccessResponse, RenderWorkerError> {
    let mut command = Command::new(binary_path);
    command
        .arg(request_path)
        .arg(response_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(darktable_cli_env_value) = darktable_cli_env_value {
        command.env(DARKTABLE_CLI_BIN_ENV, darktable_cli_env_value);
    }
    let mut child = command.spawn().map_err(|error| RenderWorkerError {
        reason_code: "local-renderer-launch-failed",
        customer_message: safe_render_failure_message(RenderIntent::Preview),
        operator_detail: format!(
            "local renderer sidecar를 시작하지 못했어요: binary={} error={error}",
            binary_path.to_string_lossy()
        ),
    })?;

    let started_at = Instant::now();
    let timeout = local_renderer_timeout();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let payload = fs::read_to_string(response_path).ok();

                if !status.success() {
                    if let Some(payload) = payload.as_deref() {
                        if let Err(error) = parse_local_renderer_response(payload) {
                            if error.reason_code == "local-renderer-sidecar-error" {
                                return Err(error);
                            }
                        }
                    }

                    return Err(RenderWorkerError {
                        reason_code: "local-renderer-exit-failed",
                        customer_message: safe_render_failure_message(RenderIntent::Preview),
                        operator_detail: format!(
                            "local renderer sidecar가 실패했어요: binary={} exit_code={:?}",
                            binary_path.to_string_lossy(),
                            status.code()
                        ),
                    });
                }

                let payload = payload.ok_or_else(|| RenderWorkerError {
                    reason_code: "local-renderer-response-missing",
                    customer_message: safe_render_failure_message(RenderIntent::Preview),
                    operator_detail: format!(
                        "local renderer response를 읽지 못했어요: path={} error={error}",
                        response_path.to_string_lossy(),
                        error = "missing file"
                    ),
                })?;

                return parse_local_renderer_response(&payload);
            }
            Ok(None) => {
                if started_at.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(RenderWorkerError {
                        reason_code: "local-renderer-timeout",
                        customer_message: safe_render_failure_message(RenderIntent::Preview),
                        operator_detail: format!(
                            "local renderer sidecar가 제한 시간 안에 끝나지 않았어요: binary={} timeoutMs={}",
                            binary_path.to_string_lossy(),
                            timeout.as_millis()
                        ),
                    });
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => {
                return Err(RenderWorkerError {
                    reason_code: "local-renderer-wait-failed",
                    customer_message: safe_render_failure_message(RenderIntent::Preview),
                    operator_detail: format!(
                        "local renderer sidecar 상태를 확인하지 못했어요: binary={} error={error}",
                        binary_path.to_string_lossy()
                    ),
                });
            }
        }
    }
}

fn render_preview_via_local_renderer_sidecar_in_dir(
    base_dir: &Path,
    paths: &SessionPaths,
    capture: &SessionCaptureRecord,
    bundle: &PublishedPresetRuntimeBundle,
    staging_output_path: &Path,
) -> Result<ValidatedLocalRendererCandidate, RenderWorkerError> {
    let preview_source = resolve_preview_render_source(
        capture,
        paths,
        RenderIntent::Preview,
        None,
        approved_preview_invocation_profile(),
    );
    render_preview_via_local_renderer_sidecar_for_source_in_dir(
        base_dir,
        paths,
        capture,
        bundle,
        Path::new(&preview_source.asset_path),
        preview_source.kind,
        staging_output_path,
    )
}

fn render_preview_via_local_renderer_sidecar_for_source_in_dir(
    base_dir: &Path,
    paths: &SessionPaths,
    capture: &SessionCaptureRecord,
    bundle: &PublishedPresetRuntimeBundle,
    source_asset_path: &Path,
    source_kind: PreviewRenderSourceKind,
    staging_output_path: &Path,
) -> Result<ValidatedLocalRendererCandidate, RenderWorkerError> {
    let binary_path = resolve_local_renderer_binary(base_dir).ok_or_else(|| RenderWorkerError {
        reason_code: "local-renderer-binary-missing",
        customer_message: safe_render_failure_message(RenderIntent::Preview),
        operator_detail:
            "local renderer sidecar binary를 찾지 못했어요. approved canary는 fallback으로 내려갑니다."
                .into(),
    })?;

    if let Some(parent) = staging_output_path.parent() {
        fs::create_dir_all(parent).map_err(|error| RenderWorkerError {
            reason_code: "render-output-dir-unavailable",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!(
                "local renderer output directory를 준비하지 못했어요: {error}"
            ),
        })?;
    }

    let (preview_width_cap, preview_height_cap) = preview_render_dimensions(source_kind);

    let request = LocalRendererRequest {
        schema_version: LOCAL_RENDERER_REQUEST_SCHEMA_VERSION.into(),
        session_id: capture.session_id.clone(),
        capture_id: capture.capture_id.clone(),
        request_id: capture.request_id.clone(),
        booth_alias: capture.booth_alias.clone(),
        preset_id: capture.active_preset_id.clone().unwrap_or_default(),
        preset_version: capture.active_preset_version.clone(),
        raw_asset_path: capture.raw.asset_path.clone(),
        candidate_output_path: staging_output_path.to_string_lossy().into_owned(),
        xmp_template_path: bundle.xmp_template_path.to_string_lossy().into_owned(),
        darktable_version: bundle.darktable_version.clone(),
        capture_persisted_at_ms: capture.raw.persisted_at_ms,
        preview_width_cap,
        preview_height_cap,
        source_asset_path: source_asset_path.to_string_lossy().into_owned(),
    };

    let working_dir = paths.diagnostics_dir.join("local-renderer");
    fs::create_dir_all(&working_dir).map_err(|error| RenderWorkerError {
        reason_code: "local-renderer-request-dir-unavailable",
        customer_message: safe_render_failure_message(RenderIntent::Preview),
        operator_detail: format!(
            "local renderer diagnostics directory를 준비하지 못했어요: {error}"
        ),
    })?;

    let request_path = working_dir.join(format!(
        "{}-{}.request.json",
        capture.capture_id, capture.request_id
    ));
    let response_path = working_dir.join(format!(
        "{}-{}.response.json",
        capture.capture_id, capture.request_id
    ));
    let _ = fs::remove_file(&response_path);
    fs::write(
        &request_path,
        serde_json::to_vec_pretty(&request).map_err(|error| RenderWorkerError {
            reason_code: "local-renderer-request-serialize-failed",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!("local renderer request를 직렬화하지 못했어요: {error}"),
        })?,
    )
    .map_err(|error| RenderWorkerError {
        reason_code: "local-renderer-request-write-failed",
        customer_message: safe_render_failure_message(RenderIntent::Preview),
        operator_detail: format!(
            "local renderer request를 쓰지 못했어요: path={} error={error}",
            request_path.to_string_lossy()
        ),
    })?;

    let darktable_cli_env_value = local_renderer_darktable_cli_env_value();
    let response = run_local_renderer_sidecar(
        &binary_path,
        &request_path,
        &response_path,
        darktable_cli_env_value.as_deref(),
    )?;
    validate_local_renderer_candidate_response(paths, capture, staging_output_path, &response)
}

fn selected_route_event_detail(route: &ResolvedPreviewRenderRoute) -> String {
    format!(
        "policyReason={};fallbackReason={}",
        route.reason_code,
        route.fallback_reason.as_deref().unwrap_or("none")
    )
}

fn resolve_selected_preview_render_route_in_dir(
    base_dir: &Path,
    paths: &SessionPaths,
    capture: &SessionCaptureRecord,
) -> ResolvedPreviewRenderRoute {
    let runtime_branch_id = resolve_runtime_branch_id_in_dir(base_dir, &capture.session_id);
    match load_session_locked_preview_render_route_policy(paths) {
        Ok(locked_policy) => {
            resolve_preview_render_route(&locked_policy, capture, runtime_branch_id.as_deref())
        }
        Err(error) => {
            append_render_event(
                paths,
                &capture.capture_id,
                Some(&capture.request_id),
                RenderIntent::Preview,
                "renderer-route-fallback",
                Some(error.reason_code),
                Some(&format!(
                    "from=policy-lock;to=darktable;reasonDetail={}",
                    diagnostic_detail_value(Some(error.operator_detail.as_str()))
                )),
            );
            ResolvedPreviewRenderRoute {
                route: PreviewRenderRoute::Darktable,
                reason_code: "lock-unavailable",
                fallback_reason: Some(error.operator_detail),
            }
        }
    }
}

fn load_capture_for_session(
    paths: &SessionPaths,
    capture_id: &str,
) -> Option<SessionCaptureRecord> {
    let manifest = fs::read_to_string(&paths.manifest_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<SessionManifest>(&raw).ok())?;
    manifest
        .captures
        .into_iter()
        .find(|capture| capture.capture_id == capture_id)
}

fn load_truth_closed_capture_for_session(
    paths: &SessionPaths,
    capture_id: &str,
) -> Option<SessionCaptureRecord> {
    load_capture_for_session(paths, capture_id).filter(|capture| {
        matches!(
            capture.render_status.as_str(),
            "previewReady" | "finalReady"
        ) && capture.preview.asset_path.is_some()
            && capture.preview.ready_at_ms.is_some()
    })
}

fn detail_field_value(detail: &str, key: &str) -> Option<String> {
    detail
        .split(';')
        .find_map(|segment| segment.strip_prefix(&format!("{key}=")))
        .map(str::to_string)
        .filter(|value| !value.trim().is_empty())
}

fn with_speculative_route_metadata(
    detail: String,
    selected_route: &ResolvedPreviewRenderRoute,
    route_fallback_reason_code: Option<&str>,
    route_fallback_reason_detail: Option<&str>,
    close_owner_route: PreviewRenderRoute,
    close_owner_result: &str,
    close_owner_fidelity_verdict: &str,
    close_owner_fidelity_detail: &str,
) -> String {
    format!(
        "{detail};selectedRoute={};selectedPolicyReason={};selectedFallbackReason={};routeFallbackReasonCode={};routeFallbackReasonDetail={};closeOwnerRoute={};closeOwnerResult={};closeOwnerFidelityVerdict={};closeOwnerFidelityDetail={}",
        selected_route.route.as_reason_code(),
        selected_route.reason_code,
        diagnostic_detail_value(selected_route.fallback_reason.as_deref()),
        route_fallback_reason_code.unwrap_or("none"),
        diagnostic_detail_value(route_fallback_reason_detail),
        close_owner_route.as_reason_code(),
        close_owner_result,
        close_owner_fidelity_verdict,
        diagnostic_detail_value(Some(close_owner_fidelity_detail))
    )
}

pub fn log_preview_route_events_from_prepared_detail_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
    request_id: &str,
    detail: &str,
) {
    let Some(selected_route) = detail_field_value(detail, "selectedRoute") else {
        return;
    };

    let paths = SessionPaths::new(base_dir, session_id);
    let selected_policy_reason =
        detail_field_value(detail, "selectedPolicyReason").unwrap_or_else(|| "unknown".into());
    let selected_fallback_reason =
        detail_field_value(detail, "selectedFallbackReason").unwrap_or_else(|| "none".into());
    append_render_event(
        &paths,
        capture_id,
        Some(request_id),
        RenderIntent::Preview,
        "renderer-route-selected",
        Some(selected_route.as_str()),
        Some(&format!(
            "policyReason={selected_policy_reason};fallbackReason={selected_fallback_reason}"
        )),
    );

    let route_fallback_reason_code =
        detail_field_value(detail, "routeFallbackReasonCode").unwrap_or_else(|| "none".into());
    if route_fallback_reason_code != "none" {
        let route_fallback_reason_detail = detail_field_value(detail, "routeFallbackReasonDetail")
            .unwrap_or_else(|| "none".into());
        append_render_event(
            &paths,
            capture_id,
            Some(request_id),
            RenderIntent::Preview,
            "renderer-route-fallback",
            Some(route_fallback_reason_code.as_str()),
            Some(&format!(
                "from={selected_route};to=darktable;reasonDetail={route_fallback_reason_detail}"
            )),
        );
    }

    let close_owner_route =
        detail_field_value(detail, "closeOwnerRoute").unwrap_or_else(|| "none".into());
    if close_owner_route == "none" {
        return;
    }

    let close_owner_result =
        detail_field_value(detail, "closeOwnerResult").unwrap_or_else(|| "accepted".into());
    let close_owner_fidelity_verdict = detail_field_value(detail, "closeOwnerFidelityVerdict")
        .unwrap_or_else(|| DARKTABLE_FIDELITY_VERDICT.into());
    let close_owner_fidelity_detail = detail_field_value(detail, "closeOwnerFidelityDetail")
        .unwrap_or_else(|| diagnostic_detail_value(Some(DARKTABLE_FIDELITY_DETAIL)));
    let close_owner_detail = if close_owner_result == "fallback-accepted" {
        format!(
            "route={close_owner_route};result={close_owner_result};selectedRoute={selected_route};fidelityVerdict={close_owner_fidelity_verdict};fidelityDetail={close_owner_fidelity_detail}"
        )
    } else {
        format!(
            "route={close_owner_route};result={close_owner_result};fidelityVerdict={close_owner_fidelity_verdict};fidelityDetail={close_owner_fidelity_detail}"
        )
    };
    append_render_event(
        &paths,
        capture_id,
        Some(request_id),
        RenderIntent::Preview,
        "renderer-close-owner",
        Some(close_owner_route.as_str()),
        Some(&close_owner_detail),
    );
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
            let paths = SessionPaths::new(&job.base_dir, &job.session_id);
            if load_truth_closed_capture_for_session(&paths, &job.capture_id).is_some() {
                log::info!(
                    "resident_first_visible_render_superseded session={} capture_id={} request_id={} reason_code={}",
                    job.session_id,
                    job.capture_id,
                    job.request_id,
                    error.reason_code
                );
                let _ = fs::remove_file(&job.output_path);
                let _ = fs::remove_file(&job.detail_path);
                if let Some(source_cleanup_path) = job.source_cleanup_path.as_ref() {
                    let _ = fs::remove_file(source_cleanup_path);
                }
                let _ = fs::remove_file(&job.lock_path);
                return;
            }
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

fn should_mirror_render_event_to_runtime_log(event: &str) -> bool {
    matches!(
        event,
        "renderer-route-selected" | "renderer-route-fallback" | "renderer-close-owner"
    )
}

fn should_warn_for_render_event(event: &str) -> bool {
    matches!(event, "renderer-route-fallback")
}

fn render_event_runtime_log_summary(
    session_id: &str,
    capture_id: &str,
    request_id: &str,
    intent: RenderIntent,
    event: &str,
    reason_code: Option<&str>,
    detail: Option<&str>,
) -> Option<String> {
    if !should_mirror_render_event_to_runtime_log(event) {
        return None;
    }

    Some(format!(
        "render_route_event session={session_id} capture_id={capture_id} request_id={request_id} stage={} event={event} reason_code={} detail={}",
        render_stage_label(intent),
        reason_code.unwrap_or("none"),
        detail.unwrap_or("none")
    ))
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

    if let Some(summary) = render_event_runtime_log_summary(
        &paths
            .session_root
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default(),
        capture_id,
        request_id,
        intent,
        event,
        Some(reason_code),
        Some(detail),
    ) {
        if should_warn_for_render_event(event) {
            log::warn!("{summary}");
        } else {
            log::info!("{summary}");
        }
    }
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
    let cachedir = worker_root.join("cache");
    let library = worker_root.join("library.db");
    let hq_flag = match intent {
        RenderIntent::Preview => "false",
        RenderIntent::Final => "true",
    };
    let binary_resolution = resolve_darktable_cli_binary();
    let mut arguments = vec![
        darktable_cli_path_arg(source_asset_path),
        darktable_cli_path_arg(xmp_template_path),
        darktable_cli_path_arg(output_path),
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
        darktable_cli_path_arg(&configdir),
        "--cachedir".into(),
        darktable_cli_path_arg(&cachedir),
        "--library".into(),
        darktable_cli_path_arg(&library),
    ]);

    DarktableInvocation {
        binary: binary_resolution.binary,
        binary_source: binary_resolution.source,
        render_source_kind,
        arguments,
        working_directory: base_dir.to_path_buf(),
    }
}

fn darktable_cli_path_arg(path: &Path) -> String {
    let raw = path.to_string_lossy();
    let trimmed = raw
        .strip_prefix(r"\\?\")
        .or_else(|| raw.strip_prefix("//?/"))
        .unwrap_or(raw.as_ref());

    trimmed.replace('\\', "/")
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum PreviewRenderRoute {
    Darktable,
    LocalRendererSidecar,
}

impl PreviewRenderRoute {
    fn as_reason_code(self) -> &'static str {
        match self {
            Self::Darktable => "darktable",
            Self::LocalRendererSidecar => "local-renderer-sidecar",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewRenderRouteRule {
    route: PreviewRenderRoute,
    #[serde(default)]
    booth_alias: Option<String>,
    #[serde(default)]
    branch_id: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    preset_id: Option<String>,
    #[serde(default)]
    preset_version: Option<String>,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewRenderRoutePolicy {
    schema_version: String,
    default_route: PreviewRenderRoute,
    #[serde(default)]
    canary_routes: Vec<PreviewRenderRouteRule>,
    #[serde(default)]
    forced_fallback_routes: Vec<PreviewRenderRouteRule>,
}

impl Default for PreviewRenderRoutePolicy {
    fn default() -> Self {
        Self {
            schema_version: PREVIEW_RENDER_ROUTE_POLICY_SCHEMA_VERSION.into(),
            default_route: PreviewRenderRoute::Darktable,
            canary_routes: Vec::new(),
            forced_fallback_routes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedPreviewRenderRoute {
    route: PreviewRenderRoute,
    reason_code: &'static str,
    fallback_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalRendererRequest {
    schema_version: String,
    session_id: String,
    capture_id: String,
    request_id: String,
    booth_alias: String,
    preset_id: String,
    preset_version: String,
    raw_asset_path: String,
    candidate_output_path: String,
    xmp_template_path: String,
    darktable_version: String,
    capture_persisted_at_ms: u64,
    preview_width_cap: u32,
    preview_height_cap: u32,
    source_asset_path: String,
}

pub fn initialize_session_locked_preview_render_route_policy_in_dir(
    base_dir: &Path,
    session_id: &str,
) -> Result<(), HostErrorEnvelope> {
    let paths = SessionPaths::try_new(base_dir, session_id)?;
    persist_session_locked_preview_render_route_policy(base_dir, &paths)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalRendererFidelityMetadata {
    verdict: String,
    #[serde(default)]
    detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalRendererAttemptMetadata {
    retry_ordinal: u32,
    completion_ordinal: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalRendererSuccessResponse {
    schema_version: String,
    route: PreviewRenderRoute,
    session_id: String,
    capture_id: String,
    request_id: String,
    preset_id: String,
    preset_version: String,
    candidate_path: String,
    candidate_written_at_ms: u64,
    elapsed_ms: u64,
    fidelity: LocalRendererFidelityMetadata,
    attempt: LocalRendererAttemptMetadata,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeBranchRolloutStore {
    schema_version: String,
    #[serde(default)]
    branches: Vec<RuntimeBranchRolloutBranchRecord>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeBranchRolloutBranchRecord {
    branch_id: String,
    #[serde(default)]
    active_session: Option<RuntimeBranchActiveSessionRecord>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeBranchActiveSessionRecord {
    session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidatedLocalRendererCandidate {
    candidate_path: PathBuf,
    elapsed_ms: u64,
    fidelity_verdict: String,
    fidelity_detail: Option<String>,
    retry_ordinal: u32,
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

fn preview_render_route_policy_path(base_dir: &Path) -> PathBuf {
    base_dir
        .join("branch-config")
        .join("preview-renderer-policy.json")
}

fn local_renderer_darktable_cli_env_value() -> Option<String> {
    let env_override = std::env::var(DARKTABLE_CLI_BIN_ENV).ok();
    local_renderer_darktable_cli_env_value_with_candidates(
        env_override.as_deref(),
        &darktable_cli_binary_candidates(),
    )
}

fn local_renderer_darktable_cli_env_value_with_candidates(
    env_override: Option<&str>,
    candidates: &[(&'static str, PathBuf)],
) -> Option<String> {
    Some(resolve_darktable_cli_binary_with_candidates(env_override, candidates).binary)
}

fn session_locked_preview_render_route_policy_path(paths: &SessionPaths) -> PathBuf {
    paths
        .diagnostics_dir
        .join(SESSION_LOCKED_PREVIEW_RENDER_ROUTE_POLICY_FILE_NAME)
}

fn persist_session_locked_preview_render_route_policy(
    base_dir: &Path,
    paths: &SessionPaths,
) -> Result<(), HostErrorEnvelope> {
    if let Some(error) = take_injected_preview_route_lock_failure() {
        return Err(error);
    }

    let locked_policy_path = session_locked_preview_render_route_policy_path(paths);
    let policy = load_preview_render_route_policy(base_dir);
    let policy_bytes = serde_json::to_vec_pretty(&policy).map_err(|error| {
        HostErrorEnvelope::persistence(format!(
            "preview route lock policy를 직렬화하지 못했어요: {error}"
        ))
    })?;

    fs::create_dir_all(&paths.diagnostics_dir).map_err(|error| {
        HostErrorEnvelope::persistence(format!(
            "preview route lock directory를 준비하지 못했어요: {error}"
        ))
    })?;
    fs::write(&locked_policy_path, policy_bytes).map_err(|error| {
        HostErrorEnvelope::persistence(format!(
            "preview route lock policy를 저장하지 못했어요: {error}"
        ))
    })?;

    Ok(())
}

#[doc(hidden)]
pub fn set_preview_route_lock_failures_for_tests(failures: usize) {
    let mut guard = PREVIEW_ROUTE_LOCK_FAILURE_INJECTION
        .lock()
        .expect("preview route lock failure injection lock should be available");
    *guard = failures;
}

fn take_injected_preview_route_lock_failure() -> Option<HostErrorEnvelope> {
    let mut guard = PREVIEW_ROUTE_LOCK_FAILURE_INJECTION
        .lock()
        .expect("preview route lock failure injection lock should be available");
    if *guard == 0 {
        return None;
    }

    *guard -= 1;
    Some(HostErrorEnvelope::persistence(
        "preview route lock failure injected for tests",
    ))
}

fn local_renderer_timeout() -> Duration {
    std::env::var(LOCAL_RENDERER_TIMEOUT_MS_ENV)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .map(Duration::from_millis)
        .unwrap_or(LOCAL_RENDERER_DEFAULT_TIMEOUT)
}

fn load_preview_render_route_policy(base_dir: &Path) -> PreviewRenderRoutePolicy {
    let policy_path = preview_render_route_policy_path(base_dir);
    load_preview_render_route_policy_from_path(&policy_path)
}

fn load_preview_render_route_policy_from_path(policy_path: &Path) -> PreviewRenderRoutePolicy {
    let Ok(bytes) = fs::read_to_string(policy_path) else {
        return PreviewRenderRoutePolicy::default();
    };

    match serde_json::from_str::<PreviewRenderRoutePolicy>(&bytes) {
        Ok(policy) if policy.schema_version == PREVIEW_RENDER_ROUTE_POLICY_SCHEMA_VERSION => {
            sanitize_preview_render_route_policy(policy)
        }
        Ok(_) => PreviewRenderRoutePolicy::default(),
        Err(error) => {
            log::warn!(
                "preview_render_route_policy_invalid path={} error={error}",
                policy_path.to_string_lossy()
            );
            PreviewRenderRoutePolicy::default()
        }
    }
}

fn load_session_locked_preview_render_route_policy(
    paths: &SessionPaths,
) -> Result<PreviewRenderRoutePolicy, RenderWorkerError> {
    let locked_policy_path = session_locked_preview_render_route_policy_path(paths);
    if locked_policy_path.is_file() {
        return Ok(load_preview_render_route_policy_from_path(
            &locked_policy_path,
        ));
    }

    Err(RenderWorkerError {
        reason_code: "preview-route-policy-lock-missing",
        customer_message: safe_render_failure_message(RenderIntent::Preview),
        operator_detail: format!(
            "session-locked preview route policy가 비어 있어 default darktable route로 강등합니다: path={}",
            locked_policy_path.to_string_lossy()
        ),
    })
}

fn should_force_darktable_for_session_after_local_renderer_error(
    error: &RenderWorkerError,
) -> bool {
    error.reason_code.starts_with("local-renderer-")
}

fn mark_session_locked_preview_route_forced_fallback_after_local_renderer_error(
    paths: &SessionPaths,
    capture: &SessionCaptureRecord,
) -> Result<(), RenderWorkerError> {
    let locked_policy_path = session_locked_preview_render_route_policy_path(paths);
    let mut policy = if locked_policy_path.is_file() {
        load_preview_render_route_policy_from_path(&locked_policy_path)
    } else {
        PreviewRenderRoutePolicy::default()
    };

    let session_rule_already_exists = policy.forced_fallback_routes.iter().any(|rule| {
        rule.route == PreviewRenderRoute::LocalRendererSidecar
            && rule.session_id.as_deref() == Some(capture.session_id.as_str())
            && rule.preset_id.as_deref() == capture.active_preset_id.as_deref()
            && rule.preset_version.as_deref() == Some(capture.active_preset_version.as_str())
            && rule.reason.as_deref() == Some("session-sidecar-health-check-failed")
    });
    if session_rule_already_exists {
        return Ok(());
    }

    policy.forced_fallback_routes.push(PreviewRenderRouteRule {
        route: PreviewRenderRoute::LocalRendererSidecar,
        booth_alias: None,
        branch_id: None,
        session_id: Some(capture.session_id.clone()),
        preset_id: capture.active_preset_id.clone(),
        preset_version: Some(capture.active_preset_version.clone()),
        reason: Some("session-sidecar-health-check-failed".into()),
    });

    let policy_bytes = serde_json::to_vec_pretty(&policy).map_err(|error| RenderWorkerError {
        reason_code: "preview-route-policy-lock-write-failed",
        customer_message: safe_render_failure_message(RenderIntent::Preview),
        operator_detail: format!(
            "session-locked preview route policy forced fallback를 직렬화하지 못했어요: {error}"
        ),
    })?;

    fs::create_dir_all(&paths.diagnostics_dir).map_err(|error| RenderWorkerError {
        reason_code: "preview-route-policy-lock-write-failed",
        customer_message: safe_render_failure_message(RenderIntent::Preview),
        operator_detail: format!(
            "session-locked preview route policy directory를 준비하지 못했어요: {error}"
        ),
    })?;
    fs::write(&locked_policy_path, policy_bytes).map_err(|error| RenderWorkerError {
        reason_code: "preview-route-policy-lock-write-failed",
        customer_message: safe_render_failure_message(RenderIntent::Preview),
        operator_detail: format!(
            "session-locked preview route policy forced fallback를 저장하지 못했어요: {error}"
        ),
    })?;

    Ok(())
}

fn sanitize_preview_render_route_policy(
    policy: PreviewRenderRoutePolicy,
) -> PreviewRenderRoutePolicy {
    PreviewRenderRoutePolicy {
        schema_version: PREVIEW_RENDER_ROUTE_POLICY_SCHEMA_VERSION.into(),
        default_route: PreviewRenderRoute::Darktable,
        canary_routes: policy
            .canary_routes
            .into_iter()
            .filter(|rule| rule.route == PreviewRenderRoute::LocalRendererSidecar)
            .collect(),
        forced_fallback_routes: policy
            .forced_fallback_routes
            .into_iter()
            .filter(|rule| rule.route == PreviewRenderRoute::LocalRendererSidecar)
            .collect(),
    }
}

fn preview_render_route_rule_matches(
    rule: &PreviewRenderRouteRule,
    capture: &SessionCaptureRecord,
    runtime_branch_id: Option<&str>,
) -> bool {
    if rule.booth_alias.is_some() && effective_route_rule_branch_id(rule).is_none() {
        return false;
    }

    effective_route_rule_branch_id(rule)
        .map(|value| runtime_branch_id == Some(value))
        .unwrap_or(true)
        && rule
            .session_id
            .as_deref()
            .map(|value| value == capture.session_id)
            .unwrap_or(true)
        && rule
            .preset_id
            .as_deref()
            .map(|value| capture.active_preset_id.as_deref() == Some(value))
            .unwrap_or(true)
        && rule
            .preset_version
            .as_deref()
            .map(|value| value == capture.active_preset_version)
            .unwrap_or(true)
}

fn preview_render_route_rule_priority(rule: &PreviewRenderRouteRule) -> (u8, u8, u8, u8) {
    (
        u8::from(rule.session_id.is_some()),
        u8::from(rule.preset_version.is_some()),
        u8::from(rule.preset_id.is_some()),
        u8::from(effective_route_rule_branch_id(rule).is_some()),
    )
}

fn effective_route_rule_branch_id(rule: &PreviewRenderRouteRule) -> Option<&str> {
    rule.branch_id.as_deref().or(rule
        .booth_alias
        .as_deref()
        .filter(|value| is_valid_branch_id(value)))
}

fn resolve_runtime_branch_id() -> Option<String> {
    std::env::var(RUNTIME_BRANCH_ID_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| is_valid_branch_id(value))
}

fn resolve_runtime_branch_id_in_dir(base_dir: &Path, session_id: &str) -> Option<String> {
    resolve_runtime_branch_id()
        .or_else(|| resolve_runtime_branch_id_from_rollout_store(base_dir, session_id))
}

fn resolve_runtime_branch_id_from_rollout_store(
    base_dir: &Path,
    session_id: &str,
) -> Option<String> {
    let store_path = base_dir.join("branch-config").join("state.json");
    let bytes = fs::read_to_string(store_path).ok()?;
    let store = serde_json::from_str::<RuntimeBranchRolloutStore>(&bytes).ok()?;
    if store.schema_version != BRANCH_ROLLOUT_STORE_SCHEMA_VERSION {
        return None;
    }

    if let Some(active_branch_id) = store
        .branches
        .iter()
        .find(|branch| {
            branch
                .active_session
                .as_ref()
                .map(|active_session| active_session.session_id.as_str() == session_id)
                .unwrap_or(false)
                && is_valid_branch_id(&branch.branch_id)
        })
        .map(|branch| branch.branch_id.clone())
    {
        return Some(active_branch_id);
    }

    let valid_branch_ids = store
        .branches
        .iter()
        .filter_map(|branch| {
            is_valid_branch_id(&branch.branch_id).then_some(branch.branch_id.as_str())
        })
        .collect::<Vec<_>>();
    if valid_branch_ids.len() == 1 {
        return Some(valid_branch_ids[0].to_string());
    }

    None
}

fn select_preview_render_route_rule<'a>(
    rules: &'a [PreviewRenderRouteRule],
    capture: &SessionCaptureRecord,
    runtime_branch_id: Option<&str>,
    expected_route: PreviewRenderRoute,
) -> Option<&'a PreviewRenderRouteRule> {
    rules
        .iter()
        .filter(|rule| {
            rule.route == expected_route
                && preview_render_route_rule_matches(rule, capture, runtime_branch_id)
        })
        .max_by_key(|rule| preview_render_route_rule_priority(rule))
}

fn resolve_preview_render_route(
    policy: &PreviewRenderRoutePolicy,
    capture: &SessionCaptureRecord,
    runtime_branch_id: Option<&str>,
) -> ResolvedPreviewRenderRoute {
    let best_forced_fallback = select_preview_render_route_rule(
        &policy.forced_fallback_routes,
        capture,
        runtime_branch_id,
        PreviewRenderRoute::LocalRendererSidecar,
    );
    let best_canary = select_preview_render_route_rule(
        &policy.canary_routes,
        capture,
        runtime_branch_id,
        PreviewRenderRoute::LocalRendererSidecar,
    );

    match (best_canary, best_forced_fallback) {
        (Some(canary), Some(fallback)) => {
            if preview_render_route_rule_priority(canary)
                > preview_render_route_rule_priority(fallback)
            {
                return ResolvedPreviewRenderRoute {
                    route: PreviewRenderRoute::LocalRendererSidecar,
                    reason_code: "canary-match",
                    fallback_reason: None,
                };
            }

            return ResolvedPreviewRenderRoute {
                route: PreviewRenderRoute::Darktable,
                reason_code: "forced-fallback",
                fallback_reason: fallback.reason.clone(),
            };
        }
        (Some(_), None) => {
            return ResolvedPreviewRenderRoute {
                route: PreviewRenderRoute::LocalRendererSidecar,
                reason_code: "canary-match",
                fallback_reason: None,
            };
        }
        (None, Some(rule)) => {
            return ResolvedPreviewRenderRoute {
                route: PreviewRenderRoute::Darktable,
                reason_code: "forced-fallback",
                fallback_reason: rule.reason.clone(),
            };
        }
        (None, None) => {}
    }

    if best_canary.is_some() {
        return ResolvedPreviewRenderRoute {
            route: PreviewRenderRoute::LocalRendererSidecar,
            reason_code: "canary-match",
            fallback_reason: None,
        };
    }

    ResolvedPreviewRenderRoute {
        route: policy.default_route,
        reason_code: "default-route",
        fallback_reason: None,
    }
}

fn parse_local_renderer_response(
    payload: &str,
) -> Result<LocalRendererSuccessResponse, RenderWorkerError> {
    if let Ok(error_response) = serde_json::from_str::<LocalRendererErrorResponse>(payload) {
        if error_response.schema_version == LOCAL_RENDERER_RESPONSE_SCHEMA_VERSION {
            return Err(RenderWorkerError {
                reason_code: "local-renderer-sidecar-error",
                customer_message: safe_render_failure_message(RenderIntent::Preview),
                operator_detail: format!(
                    "local renderer sidecar가 오류 envelope를 반환했어요: {}",
                    error_response.error.message
                ),
            });
        }
    }

    let response =
        serde_json::from_str::<LocalRendererSuccessResponse>(payload).map_err(|error| {
            RenderWorkerError {
                reason_code: "local-renderer-malformed-response",
                customer_message: safe_render_failure_message(RenderIntent::Preview),
                operator_detail: format!(
                    "local renderer response가 유효한 JSON이 아니에요: {error}"
                ),
            }
        })?;

    if response.schema_version != LOCAL_RENDERER_RESPONSE_SCHEMA_VERSION {
        return Err(RenderWorkerError {
            reason_code: "local-renderer-schema-mismatch",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!(
                "local renderer response schema가 달라요: expected={LOCAL_RENDERER_RESPONSE_SCHEMA_VERSION}, actual={}",
                response.schema_version
            ),
        });
    }

    Ok(response)
}

fn validate_local_renderer_candidate_response(
    paths: &SessionPaths,
    capture: &SessionCaptureRecord,
    requested_output_path: &Path,
    response: &LocalRendererSuccessResponse,
) -> Result<ValidatedLocalRendererCandidate, RenderWorkerError> {
    if response.route != PreviewRenderRoute::LocalRendererSidecar {
        return Err(RenderWorkerError {
            reason_code: "local-renderer-route-mismatch",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: "local renderer response route가 예상과 다릅니다.".into(),
        });
    }

    if response.session_id != capture.session_id {
        return Err(RenderWorkerError {
            reason_code: "local-renderer-session-mismatch",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!(
                "local renderer candidate session이 다릅니다: expected={}, actual={}",
                capture.session_id, response.session_id
            ),
        });
    }

    if response.capture_id != capture.capture_id {
        return Err(RenderWorkerError {
            reason_code: "local-renderer-capture-mismatch",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!(
                "local renderer candidate capture가 다릅니다: expected={}, actual={}",
                capture.capture_id, response.capture_id
            ),
        });
    }

    if response.request_id != capture.request_id {
        return Err(RenderWorkerError {
            reason_code: "local-renderer-request-mismatch",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!(
                "local renderer candidate request가 다릅니다: expected={}, actual={}",
                capture.request_id, response.request_id
            ),
        });
    }

    if capture.active_preset_id.as_deref() != Some(response.preset_id.as_str()) {
        return Err(RenderWorkerError {
            reason_code: "local-renderer-preset-mismatch",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!(
                "local renderer candidate preset이 다릅니다: expected={:?}, actual={}",
                capture.active_preset_id, response.preset_id
            ),
        });
    }

    if response.preset_version != capture.active_preset_version {
        return Err(RenderWorkerError {
            reason_code: "local-renderer-preset-version-mismatch",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!(
                "local renderer candidate preset version이 다릅니다: expected={}, actual={}",
                capture.active_preset_version, response.preset_version
            ),
        });
    }

    if response.attempt.completion_ordinal != 1 {
        return Err(RenderWorkerError {
            reason_code: "local-renderer-duplicate-completion",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!(
                "local renderer candidate completion ordinal이 중복입니다: {}",
                response.attempt.completion_ordinal
            ),
        });
    }

    let stale_cutoff = capture
        .preview
        .enqueued_at_ms
        .unwrap_or(capture.raw.persisted_at_ms);
    if response.candidate_written_at_ms < stale_cutoff {
        return Err(RenderWorkerError {
            reason_code: "local-renderer-stale-output",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!(
                "local renderer candidate가 현재 capture보다 오래됐어요: cutoff={stale_cutoff}, actual={}",
                response.candidate_written_at_ms
            ),
        });
    }

    let candidate_path = PathBuf::from(&response.candidate_path);
    if !is_session_scoped_asset_path(&paths.session_root, &candidate_path) {
        return Err(RenderWorkerError {
            reason_code: "local-renderer-invalid-path",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!(
                "local renderer candidate path가 세션 범위를 벗어났어요: {}",
                candidate_path.to_string_lossy()
            ),
        });
    }

    if normalize_path(&candidate_path) != normalize_path(requested_output_path) {
        return Err(RenderWorkerError {
            reason_code: "local-renderer-invalid-path",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!(
                "local renderer candidate path가 요청된 output과 다릅니다: expected={}, actual={}",
                requested_output_path.to_string_lossy(),
                candidate_path.to_string_lossy()
            ),
        });
    }

    if !is_valid_render_preview_asset(&candidate_path) {
        return Err(RenderWorkerError {
            reason_code: "local-renderer-invalid-raster",
            customer_message: safe_render_failure_message(RenderIntent::Preview),
            operator_detail: format!(
                "local renderer candidate output이 유효한 raster가 아니에요: {}",
                candidate_path.to_string_lossy()
            ),
        });
    }

    Ok(ValidatedLocalRendererCandidate {
        candidate_path,
        elapsed_ms: response.elapsed_ms,
        fidelity_verdict: response.fidelity.verdict.clone(),
        fidelity_detail: response.fidelity.detail.clone(),
        retry_ordinal: response.attempt.retry_ordinal,
    })
}

fn diagnostic_detail_value(value: Option<&str>) -> String {
    value
        .map(|detail| detail.replace([';', '\r', '\n'], ","))
        .filter(|detail| !detail.trim().is_empty())
        .unwrap_or_else(|| "none".into())
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
    use crate::session::session_manifest::{
        SessionCustomer, SessionLifecycle, SESSION_MANIFEST_SCHEMA_VERSION,
    };

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

    fn test_capture_record(
        temp_dir: &Path,
        session_id: &str,
        booth_alias: &str,
        capture_id: &str,
        request_id: &str,
        preset_id: &str,
        preset_version: &str,
    ) -> SessionCaptureRecord {
        let paths = SessionPaths::new(temp_dir, session_id);
        SessionCaptureRecord {
            schema_version: "session-capture/v1".into(),
            session_id: session_id.into(),
            booth_alias: booth_alias.into(),
            active_preset_id: Some(preset_id.into()),
            active_preset_version: preset_version.into(),
            active_preset_display_name: Some("Test".into()),
            capture_id: capture_id.into(),
            request_id: request_id.into(),
            raw: crate::session::session_manifest::RawCaptureAsset {
                asset_path: paths
                    .captures_originals_dir
                    .join(format!("{capture_id}.cr3"))
                    .to_string_lossy()
                    .into_owned(),
                persisted_at_ms: 1_000,
            },
            preview: crate::session::session_manifest::PreviewCaptureAsset {
                asset_path: None,
                enqueued_at_ms: Some(1_000),
                ready_at_ms: None,
            },
            final_asset: crate::session::session_manifest::FinalCaptureAsset {
                asset_path: None,
                ready_at_ms: None,
            },
            render_status: "previewWaiting".into(),
            post_end_state: "activeSession".into(),
            timing: crate::session::session_manifest::CaptureTimingMetrics {
                capture_acknowledged_at_ms: 1_000,
                preview_visible_at_ms: None,
                fast_preview_visible_at_ms: None,
                xmp_preview_ready_at_ms: None,
                preset_applied_delta_ms: None,
                capture_budget_ms: 1_000,
                preview_budget_ms: 5_000,
                preview_budget_state: "pending".into(),
            },
        }
    }

    fn write_test_manifest(temp_dir: &Path, session_id: &str, capture: SessionCaptureRecord) {
        let paths = SessionPaths::new(temp_dir, session_id);
        fs::create_dir_all(&paths.session_root).expect("session root should exist");
        crate::session::session_repository::write_session_manifest(
            &paths.manifest_path,
            &SessionManifest {
                schema_version: SESSION_MANIFEST_SCHEMA_VERSION.into(),
                session_id: session_id.into(),
                booth_alias: "Kim 4821".into(),
                customer: SessionCustomer {
                    name: "Kim".into(),
                    phone_last_four: "4821".into(),
                },
                created_at: "2026-04-07T00:00:00Z".into(),
                updated_at: "2026-04-07T00:00:00Z".into(),
                lifecycle: SessionLifecycle {
                    status: "active".into(),
                    stage: "capture-ready".into(),
                },
                catalog_revision: None,
                catalog_snapshot: None,
                active_preset: None,
                active_preset_id: Some("preset_test".into()),
                active_preset_display_name: Some("Test".into()),
                timing: None,
                captures: vec![capture],
                post_end: None,
            },
        )
        .expect("manifest should write");
    }

    #[test]
    fn resident_preview_worker_suppresses_stale_failure_after_truth_close() {
        let _guard = ResidentPreviewWorkerTestGuard::new(true, 0);
        let temp_dir = unique_temp_dir("resident-preview-superseded");
        let session_id = "session_test";
        let paths = SessionPaths::new(&temp_dir, session_id);
        let mut capture = test_capture_record(
            &temp_dir,
            session_id,
            "Kim 4821",
            "capture_test",
            "request_test",
            "preset_test",
            "2026.03.31",
        );
        capture.render_status = "previewReady".into();
        capture.preview.asset_path = Some(
            paths
                .renders_previews_dir
                .join("capture_test.jpg")
                .to_string_lossy()
                .into_owned(),
        );
        capture.preview.ready_at_ms = Some(2_000);
        capture.timing.preview_visible_at_ms = Some(2_000);
        capture.timing.xmp_preview_ready_at_ms = Some(2_000);

        write_test_manifest(&temp_dir, session_id, capture);
        fs::create_dir_all(&paths.diagnostics_dir).expect("diagnostics dir should exist");
        fs::create_dir_all(&paths.renders_previews_dir).expect("preview dir should exist");
        fs::write(
            paths.renders_previews_dir.join("capture_test.jpg"),
            [0xFF, 0xD8, 0xFF, 0xD9],
        )
        .expect("canonical preview should exist");

        run_resident_preview_render_job(ResidentPreviewRenderJob {
            base_dir: temp_dir.clone(),
            session_id: session_id.into(),
            request_id: "request_test".into(),
            capture_id: "capture_test".into(),
            preset_id: "preset_test".into(),
            preset_version: "2026.03.31".into(),
            source_asset_path: temp_dir.join("missing-source.jpg"),
            source_cleanup_path: None,
            output_path: temp_dir.join("speculative-output.jpg"),
            detail_path: temp_dir.join("speculative-output.detail"),
            lock_path: temp_dir.join("speculative-output.lock"),
        });

        let timing_events =
            fs::read_to_string(paths.diagnostics_dir.join("timing-events.log")).unwrap_or_default();
        assert!(
            !timing_events.contains("event=preview-render-failed"),
            "truth-closed capture should not emit a stale preview failure"
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn preview_route_policy_prefers_matching_canary_and_honors_forced_fallback() {
        let temp_dir = unique_temp_dir("route-policy");
        let capture = test_capture_record(
            &temp_dir,
            "session_test",
            "Kim 4821",
            "capture_test",
            "request_test",
            "preset_soft-glow",
            "2026.03.20",
        );

        let selected = resolve_preview_render_route(
            &PreviewRenderRoutePolicy {
                schema_version: PREVIEW_RENDER_ROUTE_POLICY_SCHEMA_VERSION.into(),
                default_route: PreviewRenderRoute::Darktable,
                canary_routes: vec![PreviewRenderRouteRule {
                    route: PreviewRenderRoute::LocalRendererSidecar,
                    booth_alias: None,
                    branch_id: None,
                    session_id: None,
                    preset_id: Some("preset_soft-glow".into()),
                    preset_version: Some("2026.03.20".into()),
                    reason: Some("booth-canary".into()),
                }],
                forced_fallback_routes: vec![],
            },
            &capture,
            None,
        );

        assert_eq!(selected.route, PreviewRenderRoute::LocalRendererSidecar);
        assert_eq!(selected.reason_code, "canary-match");

        let forced_fallback = resolve_preview_render_route(
            &PreviewRenderRoutePolicy {
                schema_version: PREVIEW_RENDER_ROUTE_POLICY_SCHEMA_VERSION.into(),
                default_route: PreviewRenderRoute::Darktable,
                canary_routes: vec![PreviewRenderRouteRule {
                    route: PreviewRenderRoute::LocalRendererSidecar,
                    booth_alias: None,
                    branch_id: None,
                    session_id: None,
                    preset_id: Some("preset_soft-glow".into()),
                    preset_version: Some("2026.03.20".into()),
                    reason: Some("booth-canary".into()),
                }],
                forced_fallback_routes: vec![PreviewRenderRouteRule {
                    route: PreviewRenderRoute::LocalRendererSidecar,
                    booth_alias: None,
                    branch_id: None,
                    session_id: None,
                    preset_id: Some("preset_soft-glow".into()),
                    preset_version: Some("2026.03.20".into()),
                    reason: Some("manual-disable".into()),
                }],
            },
            &capture,
            None,
        );

        assert_eq!(forced_fallback.route, PreviewRenderRoute::Darktable);
        assert_eq!(forced_fallback.reason_code, "forced-fallback");
        assert_eq!(
            forced_fallback.fallback_reason.as_deref(),
            Some("manual-disable")
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn load_preview_render_route_policy_rejects_non_darktable_default_route() {
        let temp_dir = unique_temp_dir("local-renderer-policy-default");
        let policy_path = preview_render_route_policy_path(&temp_dir);
        fs::create_dir_all(
            policy_path
                .parent()
                .expect("policy path should have a parent directory"),
        )
        .expect("policy directory should exist");
        fs::write(
            &policy_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "schemaVersion": PREVIEW_RENDER_ROUTE_POLICY_SCHEMA_VERSION,
                "defaultRoute": "local-renderer-sidecar",
                "canaryRoutes": [],
                "forcedFallbackRoutes": []
            }))
            .expect("policy should serialize"),
        )
        .expect("policy file should be writable");

        let policy = load_preview_render_route_policy(&temp_dir);

        assert_eq!(policy.default_route, PreviewRenderRoute::Darktable);
        assert!(policy.canary_routes.is_empty());
        assert!(policy.forced_fallback_routes.is_empty());

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn resolve_preview_render_route_prefers_session_specific_rules_over_broad_branch_rules() {
        let temp_dir = unique_temp_dir("local-renderer-session-priority");
        let capture = test_capture_record(
            &temp_dir,
            "session_canary",
            "Kim 4821",
            "capture_test",
            "request_test",
            "preset_soft-glow",
            "2026.03.20",
        );

        let resolved = resolve_preview_render_route(
            &PreviewRenderRoutePolicy {
                schema_version: PREVIEW_RENDER_ROUTE_POLICY_SCHEMA_VERSION.into(),
                default_route: PreviewRenderRoute::Darktable,
                canary_routes: vec![
                    PreviewRenderRouteRule {
                        route: PreviewRenderRoute::LocalRendererSidecar,
                        booth_alias: None,
                        branch_id: Some("gangnam-01".into()),
                        session_id: None,
                        preset_id: None,
                        preset_version: None,
                        reason: Some("broad-branch".into()),
                    },
                    PreviewRenderRouteRule {
                        route: PreviewRenderRoute::LocalRendererSidecar,
                        booth_alias: None,
                        branch_id: Some("gangnam-01".into()),
                        session_id: Some("session_canary".into()),
                        preset_id: None,
                        preset_version: None,
                        reason: Some("session-canary".into()),
                    },
                ],
                forced_fallback_routes: vec![
                    PreviewRenderRouteRule {
                        route: PreviewRenderRoute::LocalRendererSidecar,
                        booth_alias: None,
                        branch_id: Some("gangnam-01".into()),
                        session_id: None,
                        preset_id: None,
                        preset_version: None,
                        reason: Some("broad-disable".into()),
                    },
                    PreviewRenderRouteRule {
                        route: PreviewRenderRoute::LocalRendererSidecar,
                        booth_alias: None,
                        branch_id: Some("gangnam-01".into()),
                        session_id: Some("session_canary".into()),
                        preset_id: None,
                        preset_version: None,
                        reason: Some("session-disable".into()),
                    },
                ],
            },
            &capture,
            Some("gangnam-01"),
        );

        assert_eq!(resolved.route, PreviewRenderRoute::Darktable);
        assert_eq!(resolved.reason_code, "forced-fallback");
        assert_eq!(resolved.fallback_reason.as_deref(), Some("session-disable"));

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn resolve_preview_render_route_prefers_session_canary_over_broad_forced_fallback() {
        let temp_dir = unique_temp_dir("local-renderer-session-canary-override");
        let capture = test_capture_record(
            &temp_dir,
            "session_canary",
            "Kim 4821",
            "capture_test",
            "request_test",
            "preset_soft-glow",
            "2026.03.20",
        );

        let resolved = resolve_preview_render_route(
            &PreviewRenderRoutePolicy {
                schema_version: PREVIEW_RENDER_ROUTE_POLICY_SCHEMA_VERSION.into(),
                default_route: PreviewRenderRoute::Darktable,
                canary_routes: vec![PreviewRenderRouteRule {
                    route: PreviewRenderRoute::LocalRendererSidecar,
                    booth_alias: None,
                    branch_id: Some("gangnam-01".into()),
                    session_id: Some("session_canary".into()),
                    preset_id: None,
                    preset_version: None,
                    reason: Some("session-canary".into()),
                }],
                forced_fallback_routes: vec![PreviewRenderRouteRule {
                    route: PreviewRenderRoute::LocalRendererSidecar,
                    booth_alias: None,
                    branch_id: Some("gangnam-01".into()),
                    session_id: None,
                    preset_id: None,
                    preset_version: None,
                    reason: Some("broad-disable".into()),
                }],
            },
            &capture,
            Some("gangnam-01"),
        );

        assert_eq!(resolved.route, PreviewRenderRoute::LocalRendererSidecar);
        assert_eq!(resolved.reason_code, "canary-match");
        assert_eq!(resolved.fallback_reason, None);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn preview_route_policy_does_not_treat_customer_booth_alias_as_branch_scope() {
        let temp_dir = unique_temp_dir("local-renderer-invalid-branch-scope");
        let capture = test_capture_record(
            &temp_dir,
            "session_canary",
            "Kim 4821",
            "capture_test",
            "request_test",
            "preset_soft-glow",
            "2026.03.20",
        );

        let resolved = resolve_preview_render_route(
            &PreviewRenderRoutePolicy {
                schema_version: PREVIEW_RENDER_ROUTE_POLICY_SCHEMA_VERSION.into(),
                default_route: PreviewRenderRoute::Darktable,
                canary_routes: vec![PreviewRenderRouteRule {
                    route: PreviewRenderRoute::LocalRendererSidecar,
                    booth_alias: Some("Kim 4821".into()),
                    branch_id: None,
                    session_id: None,
                    preset_id: None,
                    preset_version: None,
                    reason: Some("customer-alias-should-not-match".into()),
                }],
                forced_fallback_routes: vec![],
            },
            &capture,
            Some("gangnam-01"),
        );

        assert_eq!(resolved.route, PreviewRenderRoute::Darktable);
        assert_eq!(resolved.reason_code, "default-route");

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn resolve_local_renderer_binary_falls_back_to_repo_sidecar_when_base_dir_is_runtime_data() {
        let temp_dir = unique_temp_dir("local-renderer-runtime-data");
        let previous = std::env::var_os(LOCAL_RENDERER_BIN_ENV);
        std::env::remove_var(LOCAL_RENDERER_BIN_ENV);

        let resolved = resolve_local_renderer_binary(&temp_dir)
            .expect("repo sidecar should still be discoverable outside the runtime base dir");

        assert!(
            normalize_path(&resolved)
                .ends_with("/sidecar/local-renderer/local-renderer-sidecar.cmd"),
            "unexpected resolver target: {}",
            resolved.to_string_lossy()
        );

        match previous {
            Some(value) => std::env::set_var(LOCAL_RENDERER_BIN_ENV, value),
            None => std::env::remove_var(LOCAL_RENDERER_BIN_ENV),
        }
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn local_renderer_candidate_validation_rejects_stale_duplicate_and_wrong_session_results() {
        let temp_dir = unique_temp_dir("local-renderer-validate");
        let session_id = "session_test";
        let paths = SessionPaths::new(&temp_dir, session_id);
        fs::create_dir_all(&paths.renders_previews_dir).expect("preview dir should exist");
        fs::create_dir_all(&paths.captures_originals_dir).expect("raw dir should exist");

        let capture = test_capture_record(
            &temp_dir,
            session_id,
            "Kim 4821",
            "capture_test",
            "request_test",
            "preset_soft-glow",
            "2026.03.20",
        );
        let candidate_path = paths
            .renders_previews_dir
            .join("capture_test.local-renderer.jpg");
        fs::write(&candidate_path, [0xFF, 0xD8, 0xFF, 0xD9])
            .expect("candidate preview should be writable");

        let accepted_response = LocalRendererSuccessResponse {
            schema_version: LOCAL_RENDERER_RESPONSE_SCHEMA_VERSION.into(),
            route: PreviewRenderRoute::LocalRendererSidecar,
            session_id: session_id.into(),
            capture_id: "capture_test".into(),
            request_id: "request_test".into(),
            preset_id: "preset_soft-glow".into(),
            preset_version: "2026.03.20".into(),
            candidate_path: candidate_path.to_string_lossy().into_owned(),
            candidate_written_at_ms: 1_010,
            elapsed_ms: 120,
            fidelity: LocalRendererFidelityMetadata {
                verdict: "matched".into(),
                detail: Some("deltaE=0.4".into()),
            },
            attempt: LocalRendererAttemptMetadata {
                retry_ordinal: 2,
                completion_ordinal: 1,
            },
        };

        let accepted = validate_local_renderer_candidate_response(
            &paths,
            &capture,
            &candidate_path,
            &accepted_response,
        )
        .expect("idempotent retry should still accept a valid candidate");

        assert_eq!(accepted.candidate_path, candidate_path);
        assert_eq!(accepted.retry_ordinal, 2);

        let stale_error = validate_local_renderer_candidate_response(
            &paths,
            &capture,
            &candidate_path,
            &LocalRendererSuccessResponse {
                candidate_written_at_ms: 999,
                ..accepted_response.clone()
            },
        )
        .expect_err("stale output should be rejected");
        assert_eq!(stale_error.reason_code, "local-renderer-stale-output");

        let duplicate_error = validate_local_renderer_candidate_response(
            &paths,
            &capture,
            &candidate_path,
            &LocalRendererSuccessResponse {
                attempt: LocalRendererAttemptMetadata {
                    retry_ordinal: 2,
                    completion_ordinal: 2,
                },
                ..accepted_response.clone()
            },
        )
        .expect_err("duplicate completion should be rejected");
        assert_eq!(
            duplicate_error.reason_code,
            "local-renderer-duplicate-completion"
        );

        let wrong_session_error = validate_local_renderer_candidate_response(
            &paths,
            &capture,
            &candidate_path,
            &LocalRendererSuccessResponse {
                session_id: "session_other".into(),
                ..accepted_response.clone()
            },
        )
        .expect_err("wrong-session candidate should be rejected");
        assert_eq!(
            wrong_session_error.reason_code,
            "local-renderer-session-mismatch"
        );

        let wrong_capture_error = validate_local_renderer_candidate_response(
            &paths,
            &capture,
            &candidate_path,
            &LocalRendererSuccessResponse {
                capture_id: "capture_other".into(),
                ..accepted_response.clone()
            },
        )
        .expect_err("wrong-capture candidate should be rejected");
        assert_eq!(
            wrong_capture_error.reason_code,
            "local-renderer-capture-mismatch"
        );

        let wrong_preset_error = validate_local_renderer_candidate_response(
            &paths,
            &capture,
            &candidate_path,
            &LocalRendererSuccessResponse {
                preset_version: "2026.03.21".into(),
                ..accepted_response.clone()
            },
        )
        .expect_err("wrong-preset candidate should be rejected");
        assert_eq!(
            wrong_preset_error.reason_code,
            "local-renderer-preset-version-mismatch"
        );

        let malformed_error = parse_local_renderer_response("{not-json")
            .expect_err("malformed payload should fail parsing");
        assert_eq!(
            malformed_error.reason_code,
            "local-renderer-malformed-response"
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn local_renderer_timeout_uses_env_override_when_present() {
        let previous = std::env::var_os(LOCAL_RENDERER_TIMEOUT_MS_ENV);
        std::env::set_var(LOCAL_RENDERER_TIMEOUT_MS_ENV, "25");

        assert_eq!(local_renderer_timeout(), Duration::from_millis(25));

        match previous {
            Some(value) => std::env::set_var(LOCAL_RENDERER_TIMEOUT_MS_ENV, value),
            None => std::env::remove_var(LOCAL_RENDERER_TIMEOUT_MS_ENV),
        }
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
    fn local_renderer_sidecar_reuses_the_host_darktable_resolution() {
        let temp_dir = unique_temp_dir("local-renderer-darktable-resolution");
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

        let env_value = local_renderer_darktable_cli_env_value_with_candidates(
            None,
            &[("program-files-bin", candidate.clone())],
        );

        assert_eq!(
            env_value.as_deref(),
            Some(candidate.to_string_lossy().as_ref())
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
                    preset_applied_delta_ms: None,
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
            .contains(&"--disable-opencl".to_string()));
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
                    preset_applied_delta_ms: None,
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

        assert_eq!(bytes, PREVIEW_RENDER_WARMUP_INPUT_PNG);

        let _ = fs::remove_dir_all(temp_dir);
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
                    preset_applied_delta_ms: None,
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
                    preset_applied_delta_ms: None,
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
    fn preview_invocation_uses_a_runtime_scoped_cachedir() {
        let temp_dir = unique_temp_dir("preview-invocation-cachedir");
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
                .expect("preview output path should have a parent"),
        )
        .expect("preview output directory should exist");
        fs::write(&source_path, [0xFF, 0xD8, 0xFF, 0xE0, 0x00])
            .expect("preview source should be writable");

        let invocation = build_darktable_invocation_from_source(
            &temp_dir,
            PINNED_DARKTABLE_VERSION,
            &temp_dir.join("bundle").join("preview.xmp"),
            &source_path,
            &output_path,
            RenderIntent::Preview,
            PreviewRenderSourceKind::FastPreviewRaster,
        );

        let expected_cachedir = darktable_cli_path_arg(
            &temp_dir
                .join(".boothy-darktable")
                .join("preview")
                .join("cache"),
        );
        assert!(invocation
            .arguments
            .windows(2)
            .any(|pair| { pair[0] == "--cachedir" && pair[1] == expected_cachedir }));

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn fast_preview_raster_invocation_restores_a_sharper_than_legacy_128_cap() {
        assert!(
            FAST_PREVIEW_RENDER_MAX_WIDTH_PX >= 256,
            "fast-preview truthful close should stay at the restored 256px booth-safe floor"
        );
        assert!(
            FAST_PREVIEW_RENDER_MAX_HEIGHT_PX >= 256,
            "fast-preview truthful close should stay at the restored 256px booth-safe floor"
        );
    }

    #[test]
    fn preview_renderer_warmup_source_matches_the_known_good_png_fixture() {
        let known_good_png: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0B, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x63, 0x60, 0x00, 0x02, 0x00, 0x00, 0x05, 0x00, 0x01, 0x7A, 0x5E, 0xAB, 0x3F,
            0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];

        assert_eq!(
            PREVIEW_RENDER_WARMUP_INPUT_PNG, known_good_png,
            "warmup source should stay a valid 1x1 PNG fixture so preview warmup can actually run"
        );
    }

    #[test]
    fn preview_renderer_warmup_source_fixture_has_valid_png_chunk_crcs() {
        assert!(
            png_fixture_has_valid_chunk_crcs(PREVIEW_RENDER_WARMUP_INPUT_PNG),
            "warmup source must stay a structurally valid PNG so darktable warmup actually primes the booth runtime"
        );
    }

    fn png_fixture_has_valid_chunk_crcs(bytes: &[u8]) -> bool {
        const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
        if !bytes.starts_with(PNG_SIGNATURE) {
            return false;
        }

        let mut offset = PNG_SIGNATURE.len();
        while offset < bytes.len() {
            if offset + 12 > bytes.len() {
                return false;
            }

            let length = u32::from_be_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]) as usize;
            let chunk_type_start = offset + 4;
            let chunk_data_start = chunk_type_start + 4;
            let chunk_data_end = chunk_data_start + length;
            let chunk_crc_end = chunk_data_end + 4;
            if chunk_crc_end > bytes.len() {
                return false;
            }

            let expected_crc = u32::from_be_bytes([
                bytes[chunk_data_end],
                bytes[chunk_data_end + 1],
                bytes[chunk_data_end + 2],
                bytes[chunk_data_end + 3],
            ]);
            let calculated_crc = crc32(&bytes[chunk_type_start..chunk_data_end]);
            if expected_crc != calculated_crc {
                return false;
            }

            offset = chunk_crc_end;
        }

        true
    }

    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for &byte in bytes {
            crc ^= byte as u32;
            for _ in 0..8 {
                let mask = (crc & 1).wrapping_neg() & 0xEDB8_8320;
                crc = (crc >> 1) ^ mask;
            }
        }
        !crc
    }

    #[test]
    fn darktable_invocation_strips_windows_extended_length_prefixes() {
        let temp_dir = unique_temp_dir("extended-length-prefix");
        let output_path = PathBuf::from(r"\\?\C:\captures\renders\capture_test.jpg");
        let source_path = PathBuf::from(r"\\?\C:\captures\previews\capture_test.jpg");
        let xmp_path = PathBuf::from(r"\\?\C:\bundles\preview.xmp");

        let invocation = build_darktable_invocation_from_source(
            &temp_dir,
            PINNED_DARKTABLE_VERSION,
            &xmp_path,
            &source_path,
            &output_path,
            RenderIntent::Preview,
            PreviewRenderSourceKind::FastPreviewRaster,
        );

        assert_eq!(
            invocation.arguments.first().map(String::as_str),
            Some("C:/captures/previews/capture_test.jpg")
        );
        assert_eq!(
            invocation.arguments.get(1).map(String::as_str),
            Some("C:/bundles/preview.xmp")
        );
        assert_eq!(
            invocation.arguments.get(2).map(String::as_str),
            Some("C:/captures/renders/capture_test.jpg")
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

    #[test]
    fn renderer_route_events_are_mirrored_into_runtime_log_messages() {
        let summary = render_event_runtime_log_summary(
            "session_test",
            "capture_test",
            "request_test",
            RenderIntent::Preview,
            "renderer-route-selected",
            Some("local-renderer-sidecar"),
            Some("policyReason=canary-match;fallbackReason=none"),
        )
        .expect("route selection should produce a runtime log summary");

        assert_eq!(
            summary,
            "render_route_event session=session_test capture_id=capture_test request_id=request_test stage=preview event=renderer-route-selected reason_code=local-renderer-sidecar detail=policyReason=canary-match;fallbackReason=none"
        );
    }

    #[test]
    fn renderer_route_fallback_events_are_marked_as_warn_level_candidates() {
        let summary = render_event_runtime_log_summary(
            "session_test",
            "capture_test",
            "request_test",
            RenderIntent::Preview,
            "renderer-route-fallback",
            Some("candidate-invalid"),
            Some("from=local-renderer-sidecar;to=darktable;reasonDetail=wrong-session"),
        )
        .expect("route fallback should produce a runtime log summary");

        assert_eq!(
            summary,
            "render_route_event session=session_test capture_id=capture_test request_id=request_test stage=preview event=renderer-route-fallback reason_code=candidate-invalid detail=from=local-renderer-sidecar;to=darktable;reasonDetail=wrong-session"
        );
        assert!(should_warn_for_render_event("renderer-route-fallback"));
    }

    #[test]
    fn non_route_render_events_do_not_emit_runtime_route_summaries() {
        assert!(
            render_event_runtime_log_summary(
                "session_test",
                "capture_test",
                "request_test",
                RenderIntent::Preview,
                "preview-render-ready",
                Some("preview-ready"),
                Some("detail=none"),
            )
            .is_none(),
            "only route-audit events should mirror into runtime route summaries"
        );
        assert!(!should_warn_for_render_event("renderer-close-owner"));
    }
}
