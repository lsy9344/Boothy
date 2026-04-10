use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use tauri::{async_runtime, AppHandle};
use tauri_plugin_shell::ShellExt;

use crate::{
    capture::ingest_pipeline::{complete_preview_render_in_dir, finish_preview_render_in_dir},
    contracts::dto::{
        DedicatedRendererPreviewJobRequestDto, DedicatedRendererPreviewJobResultDto,
        DedicatedRendererRenderProfileDto, DedicatedRendererWarmupRequestDto,
        DedicatedRendererWarmupResultDto, HostErrorEnvelope,
    },
    preset::{
        preset_bundle::PublishedPresetRuntimeBundle,
        preset_catalog::{find_published_preset_runtime_bundle, resolve_published_preset_catalog_dir},
    },
    render::{
        RenderIntent, RenderedCaptureAsset, is_valid_render_preview_asset,
        log_render_failure_in_dir, log_render_ready_in_dir,
    },
    session::{
        session_manifest::SessionCaptureRecord,
        session_paths::SessionPaths,
        session_repository::read_session_manifest,
    },
    timing::{SessionTimingEventInput, append_session_timing_event_in_dir},
};

pub const DEDICATED_RENDERER_EXTERNAL_BIN: &str =
    "../sidecar/dedicated-renderer/boothy-dedicated-renderer";
const DEDICATED_RENDERER_ENABLE_SPAWN_ENV: &str = "BOOTHY_DEDICATED_RENDERER_ENABLE_SPAWN";
const DEDICATED_RENDERER_PREVIEW_PROTOCOL: &str = "preview-job-v1";
const DEDICATED_RENDERER_WARMUP_PROTOCOL: &str = "warmup-v1";
const DEDICATED_RENDERER_REQUEST_SCHEMA_VERSION: &str =
    "dedicated-renderer-preview-job-request/v1";
const DEDICATED_RENDERER_RESULT_SCHEMA_VERSION: &str =
    "dedicated-renderer-preview-job-result/v1";
const DEDICATED_RENDERER_WARMUP_REQUEST_SCHEMA_VERSION: &str =
    "dedicated-renderer-warmup-request/v1";
const DEDICATED_RENDERER_WARMUP_RESULT_SCHEMA_VERSION: &str =
    "dedicated-renderer-warmup-result/v1";
const DEDICATED_RENDERER_TEST_OUTCOME_ENV: &str = "BOOTHY_TEST_DEDICATED_RENDERER_OUTCOME";

pub fn schedule_preview_renderer_warmup_with_dedicated_sidecar_in_dir(
    app_handle: Option<&AppHandle>,
    base_dir: &Path,
    session_id: &str,
    preset_id: &str,
    preset_version: &str,
) {
    if let Ok((request, request_path, result_path)) =
        build_warmup_request_in_dir(base_dir, session_id, preset_id, preset_version)
    {
        let _ = write_json_file(&request_path, &request);
        let result = submit_warmup_request(app_handle, &request, &request_path, &result_path);
        if let Ok(result) = result {
            let _ = write_json_file(&result_path, &result);
        }
    }

    super::schedule_preview_renderer_warmup_in_dir(base_dir, session_id, preset_id, preset_version);
}

pub fn complete_capture_preview_with_dedicated_renderer_in_dir(
    app_handle: Option<&AppHandle>,
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
) -> Result<SessionCaptureRecord, HostErrorEnvelope> {
    let (request, request_path, result_path) =
        build_preview_job_request_in_dir(base_dir, session_id, capture_id)?;
    write_json_file(&request_path, &request)?;

    let preview_result = match submit_preview_job(app_handle, &request, &request_path, &result_path) {
        Ok(result) => {
            let _ = write_json_file(&result_path, &result);
            if validate_preview_job_result(&request, &result).is_err() {
                log_render_failure_in_dir(
                    base_dir,
                    session_id,
                    capture_id,
                    Some(&request.request_id),
                    RenderIntent::Preview,
                    "invalid-output",
                );
            } else {
                log_preview_submission_result(base_dir, &request, &result);
            }

            Some(result)
        }
        Err(reason_code) => {
            log_render_failure_in_dir(
                base_dir,
                session_id,
                capture_id,
                Some(&request.request_id),
                RenderIntent::Preview,
                reason_code,
            );

            None
        }
    };

    if let Some(result) = preview_result.as_ref() {
        if let Some(capture) =
            try_complete_preview_from_dedicated_result_in_dir(base_dir, session_id, &request, result)?
        {
            append_preview_transition_summary_in_dir(
                base_dir,
                &capture,
                "dedicated-renderer",
                None,
            );
            return Ok(capture);
        }
    }

    let capture = complete_preview_render_in_dir(base_dir, session_id, capture_id)?;
    append_preview_transition_summary_in_dir(
        base_dir,
        &capture,
        "inline-truthful-fallback",
        preview_result
            .as_ref()
            .and_then(|result| result.detail_code.as_deref()),
    );

    Ok(capture)
}

fn build_warmup_request_in_dir(
    base_dir: &Path,
    session_id: &str,
    preset_id: &str,
    preset_version: &str,
) -> Result<
    (
        DedicatedRendererWarmupRequestDto,
        PathBuf,
        PathBuf,
    ),
    HostErrorEnvelope,
> {
    let catalog_root = resolve_published_preset_catalog_dir(base_dir);
    let bundle = find_published_preset_runtime_bundle(&catalog_root, preset_id, preset_version)
        .ok_or_else(|| {
            HostErrorEnvelope::preset_catalog_unavailable(
                "dedicated renderer warm-up에 필요한 preset bundle을 찾지 못했어요.",
            )
        })?;
    let diagnostics_dir = dedicated_renderer_diagnostics_dir(base_dir, session_id)?;
    let request_path = diagnostics_dir.join(format!("warmup-{preset_id}-{preset_version}.request.json"));
    let result_path = diagnostics_dir.join(format!("warmup-{preset_id}-{preset_version}.result.json"));

    Ok((
        DedicatedRendererWarmupRequestDto {
            schema_version: DEDICATED_RENDERER_WARMUP_REQUEST_SCHEMA_VERSION.into(),
            session_id: session_id.into(),
            preset_id: preset_id.into(),
            published_version: preset_version.into(),
            darktable_version: bundle.darktable_version.clone(),
            xmp_template_path: path_to_runtime_string(&bundle.xmp_template_path),
            preview_profile: render_profile_from_bundle(&bundle),
            diagnostics_detail_path: path_to_runtime_string(&request_path),
        },
        request_path,
        result_path,
    ))
}

fn build_preview_job_request_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
) -> Result<
    (
        DedicatedRendererPreviewJobRequestDto,
        PathBuf,
        PathBuf,
    ),
    HostErrorEnvelope,
> {
    let paths = SessionPaths::try_new(base_dir, session_id)?;
    let manifest = read_session_manifest(&paths.manifest_path)?;
    let capture = manifest
        .captures
        .iter()
        .find(|value| value.capture_id == capture_id)
        .cloned()
        .ok_or_else(|| {
            HostErrorEnvelope::session_not_found("preview render 대상 capture를 찾지 못했어요.")
        })?;
    let preset_id = capture.active_preset_id.clone().ok_or_else(|| {
        HostErrorEnvelope::preset_catalog_unavailable(
            "capture-bound preset이 없어 dedicated renderer 요청을 만들 수 없어요.",
        )
    })?;
    let catalog_root = resolve_published_preset_catalog_dir(base_dir);
    let bundle =
        find_published_preset_runtime_bundle(&catalog_root, &preset_id, &capture.active_preset_version)
            .ok_or_else(|| {
                HostErrorEnvelope::preset_catalog_unavailable(
                    "capture-bound dedicated renderer bundle을 찾지 못했어요.",
                )
            })?;
    let diagnostics_dir = dedicated_renderer_diagnostics_dir(base_dir, session_id)?;
    let request_path = diagnostics_dir.join(format!(
        "{capture_id}-{}.preview-request.json",
        capture.request_id
    ));
    let result_path = diagnostics_dir.join(format!(
        "{capture_id}-{}.preview-result.json",
        capture.request_id
    ));
    let canonical_output_path = paths.renders_previews_dir.join(format!("{capture_id}.jpg"));

    Ok((
        DedicatedRendererPreviewJobRequestDto {
            schema_version: DEDICATED_RENDERER_REQUEST_SCHEMA_VERSION.into(),
            session_id: session_id.into(),
            request_id: capture.request_id.clone(),
            capture_id: capture_id.into(),
            preset_id,
            published_version: capture.active_preset_version.clone(),
            darktable_version: bundle.darktable_version.clone(),
            xmp_template_path: path_to_runtime_string(&bundle.xmp_template_path),
            preview_profile: render_profile_from_bundle(&bundle),
            source_asset_path: capture.raw.asset_path.clone(),
            canonical_preview_output_path: path_to_runtime_string(&canonical_output_path),
            diagnostics_detail_path: path_to_runtime_string(&request_path),
        },
        request_path,
        result_path,
    ))
}

fn submit_warmup_request(
    app_handle: Option<&AppHandle>,
    request: &DedicatedRendererWarmupRequestDto,
    request_path: &Path,
    result_path: &Path,
) -> Result<DedicatedRendererWarmupResultDto, &'static str> {
    if should_attempt_dedicated_renderer_spawn() {
        let _ = try_spawn_dedicated_renderer(
            app_handle,
            DEDICATED_RENDERER_WARMUP_PROTOCOL,
            request_path,
            result_path,
        )?;
    }

    if let Ok(bytes) = fs::read_to_string(result_path) {
        match serde_json::from_str::<DedicatedRendererWarmupResultDto>(&bytes) {
            Ok(result) => match validate_warmup_result(request, &result) {
                Ok(()) => return Ok(result),
                Err("protocol-mismatch") => {
                    return Ok(build_warmup_result(
                        request,
                        "protocol-mismatch",
                        Some("protocol-mismatch"),
                        Some(
                            "warm-up result schema 또는 상태 값이 계약과 맞지 않아 shadow fallback으로 유지해요.",
                        ),
                    ));
                }
                Err(_) => {
                    return Ok(build_warmup_result(
                        request,
                        "fallback-suggested",
                        Some("shadow-inline-warmup"),
                        Some(
                            "warm-up result correlation이 맞지 않아 host-owned inline warm-up을 유지해요.",
                        ),
                    ));
                }
            },
            Err(_) => {
                return Ok(build_warmup_result(
                    request,
                    "protocol-mismatch",
                    Some("protocol-mismatch"),
                    Some(
                        "warm-up result를 dedicated renderer 계약으로 해석하지 못해 shadow fallback으로 유지해요.",
                    ),
                ));
            }
        }
    }

    Ok(build_warmup_result(
        request,
        "fallback-suggested",
        Some("shadow-inline-warmup"),
        Some("approved cutover 전까지 warm-up은 host-owned inline renderer가 계속 소유해요."),
    ))
}

fn submit_preview_job(
    app_handle: Option<&AppHandle>,
    request: &DedicatedRendererPreviewJobRequestDto,
    request_path: &Path,
    result_path: &Path,
) -> Result<DedicatedRendererPreviewJobResultDto, &'static str> {
    if let Some(result) = synthetic_preview_result_for_test(request) {
        return Ok(result);
    }

    if should_attempt_dedicated_renderer_spawn() {
        let _ = try_spawn_dedicated_renderer(
            app_handle,
            DEDICATED_RENDERER_PREVIEW_PROTOCOL,
            request_path,
            result_path,
        )?;
    } else if resolve_dedicated_renderer_executable().is_none() {
        return Ok(build_preview_result(
            request,
            "fallback-suggested",
            None,
            Some("sidecar-unavailable"),
            Some(
                "dedicated renderer binary가 아직 bundle/runtime에서 확인되지 않아 truthful fallback path로 내려가요.",
            ),
        ));
    }

    if let Ok(bytes) = fs::read_to_string(result_path) {
        match serde_json::from_str::<DedicatedRendererPreviewJobResultDto>(&bytes) {
            Ok(result) => match validate_preview_job_result(request, &result) {
                Ok(()) => return Ok(result),
                Err("protocol-mismatch") => {
                    return Ok(build_preview_result(
                        request,
                        "protocol-mismatch",
                        None,
                        Some("protocol-mismatch"),
                        Some(
                            "preview job result schema 또는 상태 값이 계약과 맞지 않아 fallback path로 내려가요.",
                        ),
                    ));
                }
                Err(_) => {
                    return Ok(build_preview_result(
                        request,
                        "invalid-output",
                        None,
                        Some("invalid-output"),
                        Some(
                            "preview job result correlation 또는 canonical output 검증에 실패해 fallback path로 내려가요.",
                        ),
                    ));
                }
            },
            Err(_) => {
                return Ok(build_preview_result(
                    request,
                    "protocol-mismatch",
                    None,
                    Some("protocol-mismatch"),
                    Some(
                        "preview job result를 dedicated renderer 계약으로 해석하지 못해 fallback path로 내려가요.",
                    ),
                ));
            }
        }
    }

    Ok(build_preview_result(
        request,
        "fallback-suggested",
        None,
        Some("shadow-submission-only"),
        Some("Story 1.11 baseline에서는 dedicated renderer submission만 고정하고 truthful close는 inline path가 계속 소유해요."),
    ))
}

fn validate_preview_job_result(
    request: &DedicatedRendererPreviewJobRequestDto,
    result: &DedicatedRendererPreviewJobResultDto,
) -> Result<(), &'static str> {
    if result.schema_version != DEDICATED_RENDERER_RESULT_SCHEMA_VERSION
        || !matches!(
            result.status.as_str(),
            "accepted"
                | "fallback-suggested"
                | "queue-saturated"
                | "protocol-mismatch"
                | "invalid-output"
                | "restarted"
        )
    {
        return Err("protocol-mismatch");
    }

    if result.diagnostics_detail_path != request.diagnostics_detail_path {
        return Err("protocol-mismatch");
    }

    if request.session_id != result.session_id
        || request.request_id != result.request_id
        || request.capture_id != result.capture_id
    {
        return Err("wrong-session");
    }

    if let Some(output_path) = result.output_path.as_deref() {
        if output_path != request.canonical_preview_output_path {
            return Err("non-canonical-output");
        }
    } else if result.status == "accepted" {
        return Err("invalid-output");
    }

    Ok(())
}

fn try_complete_preview_from_dedicated_result_in_dir(
    base_dir: &Path,
    session_id: &str,
    request: &DedicatedRendererPreviewJobRequestDto,
    result: &DedicatedRendererPreviewJobResultDto,
) -> Result<Option<SessionCaptureRecord>, HostErrorEnvelope> {
    if result.status != "accepted" {
        return Ok(None);
    }

    let Some(output_path) = result.output_path.as_deref().map(PathBuf::from) else {
        log_render_failure_in_dir(
            base_dir,
            session_id,
            &request.capture_id,
            Some(&request.request_id),
            RenderIntent::Preview,
            "invalid-output",
        );
        return Ok(None);
    };

    if !is_valid_render_preview_asset(&output_path) {
        log_render_failure_in_dir(
            base_dir,
            session_id,
            &request.capture_id,
            Some(&request.request_id),
            RenderIntent::Preview,
            "invalid-output",
        );
        return Ok(None);
    }

    let ready_at_ms = current_time_ms()?;
    log_render_ready_in_dir(
        base_dir,
        session_id,
        &request.capture_id,
        &request.request_id,
        RenderIntent::Preview,
        &build_dedicated_renderer_ready_detail(request, result, ready_at_ms),
    );

    let paths = SessionPaths::try_new(base_dir, session_id)?;
    finish_preview_render_in_dir(
        base_dir,
        &paths,
        session_id,
        &request.capture_id,
        RenderedCaptureAsset {
            asset_path: path_to_runtime_string(&output_path),
            ready_at_ms,
        },
    )
    .map(Some)
}

fn build_dedicated_renderer_ready_detail(
    request: &DedicatedRendererPreviewJobRequestDto,
    result: &DedicatedRendererPreviewJobResultDto,
    ready_at_ms: u64,
) -> String {
    format!(
        "presetId={};publishedVersion={};binary=dedicated-renderer;source=dedicated-renderer;readyAtMs={ready_at_ms};detail=status={};detailCode={}",
        request.preset_id,
        request.published_version,
        result.status,
        result.detail_code.as_deref().unwrap_or("accepted"),
    )
}

fn append_preview_transition_summary_in_dir(
    base_dir: &Path,
    capture: &SessionCaptureRecord,
    lane_owner: &str,
    fallback_reason: Option<&str>,
) {
    let first_visible_ms = capture
        .timing
        .fast_preview_visible_at_ms
        .map(|visible_at_ms| {
            visible_at_ms.saturating_sub(capture.timing.capture_acknowledged_at_ms)
        });
    let replacement_ms = match (
        capture.timing.fast_preview_visible_at_ms,
        capture.timing.xmp_preview_ready_at_ms,
    ) {
        (Some(first_visible_at_ms), Some(truthful_close_at_ms)) => {
            Some(truthful_close_at_ms.saturating_sub(first_visible_at_ms))
        }
        _ => None,
    };
    let detail = format!(
        "laneOwner={lane_owner};fallbackReason={};firstVisibleMs={};replacementMs={};originalVisibleToPresetAppliedVisibleMs={}",
        fallback_reason.unwrap_or("none"),
        format_optional_metric(first_visible_ms),
        format_optional_metric(replacement_ms),
        format_optional_metric(replacement_ms),
    );
    let _ = append_session_timing_event_in_dir(
        base_dir,
        SessionTimingEventInput {
            session_id: &capture.session_id,
            event: "capture_preview_transition_summary",
            capture_id: Some(&capture.capture_id),
            request_id: Some(&capture.request_id),
            detail: Some(&detail),
        },
    );
    log::info!(
        "capture_preview_transition_summary session={} capture_id={} request_id={} lane_owner={} fallback_reason={} first_visible_ms={} replacement_ms={}",
        capture.session_id,
        capture.capture_id,
        capture.request_id,
        lane_owner,
        fallback_reason.unwrap_or("none"),
        format_optional_metric(first_visible_ms),
        format_optional_metric(replacement_ms),
    );
}

fn format_optional_metric(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".into())
}

fn validate_warmup_result(
    request: &DedicatedRendererWarmupRequestDto,
    result: &DedicatedRendererWarmupResultDto,
) -> Result<(), &'static str> {
    if result.schema_version != DEDICATED_RENDERER_WARMUP_RESULT_SCHEMA_VERSION
        || !matches!(
            result.status.as_str(),
            "fallback-suggested" | "warmed-up" | "restarted" | "protocol-mismatch"
        )
    {
        return Err("protocol-mismatch");
    }

    if result.diagnostics_detail_path != request.diagnostics_detail_path {
        return Err("protocol-mismatch");
    }

    if result.session_id != request.session_id
        || result.preset_id != request.preset_id
        || result.published_version != request.published_version
    {
        return Err("wrong-session");
    }

    Ok(())
}

fn log_preview_submission_result(
    base_dir: &Path,
    request: &DedicatedRendererPreviewJobRequestDto,
    result: &DedicatedRendererPreviewJobResultDto,
) {
    match result.status.as_str() {
        "queue-saturated" => log_render_failure_in_dir(
            base_dir,
            &request.session_id,
            &request.capture_id,
            Some(&request.request_id),
            RenderIntent::Preview,
            result.detail_code.as_deref().unwrap_or("render-queue-saturated"),
        ),
        "fallback-suggested" | "protocol-mismatch" | "invalid-output" | "restarted" => {
            log_render_failure_in_dir(
                base_dir,
                &request.session_id,
                &request.capture_id,
                Some(&request.request_id),
                RenderIntent::Preview,
                result.detail_code.as_deref().unwrap_or("sidecar-fallback"),
            )
        }
        _ => {}
    }
}

fn try_spawn_dedicated_renderer(
    app_handle: Option<&AppHandle>,
    protocol: &str,
    request_path: &Path,
    result_path: &Path,
) -> Result<(), &'static str> {
    let app_handle = app_handle.ok_or("sidecar-unavailable")?;
    let command = app_handle
        .shell()
        .sidecar(DEDICATED_RENDERER_EXTERNAL_BIN)
        .map_err(|_| "sidecar-unavailable")?
        .arg("--protocol")
        .arg(protocol)
        .arg("--request")
        .arg(request_path)
        .arg("--result")
        .arg(result_path);

    let status = async_runtime::block_on(command.status()).map_err(|_| "sidecar-launch-failed")?;
    if status.success() {
        Ok(())
    } else {
        Err("sidecar-launch-failed")
    }
}

fn dedicated_renderer_diagnostics_dir(
    base_dir: &Path,
    session_id: &str,
) -> Result<PathBuf, HostErrorEnvelope> {
    let diagnostics_dir = SessionPaths::try_new(base_dir, session_id)?
        .diagnostics_dir
        .join("dedicated-renderer");
    fs::create_dir_all(&diagnostics_dir).map_err(|error| {
        HostErrorEnvelope::persistence(format!(
            "dedicated renderer 진단 경로를 준비하지 못했어요: {error}"
        ))
    })?;

    Ok(diagnostics_dir)
}

fn render_profile_from_bundle(
    bundle: &PublishedPresetRuntimeBundle,
) -> DedicatedRendererRenderProfileDto {
    DedicatedRendererRenderProfileDto {
        profile_id: bundle.preview_profile.profile_id.clone(),
        display_name: bundle.preview_profile.display_name.clone(),
        output_color_space: bundle.preview_profile.output_color_space.clone(),
    }
}

fn write_json_file<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), HostErrorEnvelope> {
    let parent = path.parent().ok_or_else(|| {
        HostErrorEnvelope::persistence("dedicated renderer 진단 파일 경로가 올바르지 않아요.")
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        HostErrorEnvelope::persistence(format!(
            "dedicated renderer 진단 디렉터리를 만들지 못했어요: {error}"
        ))
    })?;
    fs::write(
        path,
        serde_json::to_vec_pretty(value).map_err(|error| {
            HostErrorEnvelope::persistence(format!(
                "dedicated renderer 계약 payload를 직렬화하지 못했어요: {error}"
            ))
        })?,
    )
    .map_err(|error| {
        HostErrorEnvelope::persistence(format!(
            "dedicated renderer 계약 payload를 저장하지 못했어요: {error}"
        ))
    })
}

fn should_attempt_dedicated_renderer_spawn() -> bool {
    env::var(DEDICATED_RENDERER_ENABLE_SPAWN_ENV)
        .ok()
        .as_deref()
        .map(|value| matches!(value, "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn resolve_dedicated_renderer_executable() -> Option<PathBuf> {
    let host_suffix = format!("{}-x86_64-pc-windows-msvc.exe", dedicated_renderer_binary_stem());
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    let repo_candidate = repo_root
        .join("sidecar")
        .join("dedicated-renderer")
        .join(host_suffix);

    if repo_candidate.is_file() {
        return Some(repo_candidate);
    }

    if let Ok(current_exe) = env::current_exe() {
        if let Some(current_dir) = current_exe.parent() {
            let packaged_candidate = current_dir.join(format!("{}.exe", dedicated_renderer_binary_stem()));
            if packaged_candidate.is_file() {
                return Some(packaged_candidate);
            }
        }
    }

    None
}

fn dedicated_renderer_binary_stem() -> &'static str {
    "boothy-dedicated-renderer"
}

fn path_to_runtime_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn build_preview_result(
    request: &DedicatedRendererPreviewJobRequestDto,
    status: &str,
    output_path: Option<String>,
    detail_code: Option<&str>,
    detail_message: Option<&str>,
) -> DedicatedRendererPreviewJobResultDto {
    DedicatedRendererPreviewJobResultDto {
        schema_version: DEDICATED_RENDERER_RESULT_SCHEMA_VERSION.into(),
        session_id: request.session_id.clone(),
        request_id: request.request_id.clone(),
        capture_id: request.capture_id.clone(),
        status: status.into(),
        diagnostics_detail_path: request.diagnostics_detail_path.clone(),
        output_path,
        detail_code: detail_code.map(str::to_string),
        detail_message: detail_message.map(str::to_string),
    }
}

fn build_warmup_result(
    request: &DedicatedRendererWarmupRequestDto,
    status: &str,
    detail_code: Option<&str>,
    detail_message: Option<&str>,
) -> DedicatedRendererWarmupResultDto {
    DedicatedRendererWarmupResultDto {
        schema_version: DEDICATED_RENDERER_WARMUP_RESULT_SCHEMA_VERSION.into(),
        session_id: request.session_id.clone(),
        preset_id: request.preset_id.clone(),
        published_version: request.published_version.clone(),
        status: status.into(),
        diagnostics_detail_path: request.diagnostics_detail_path.clone(),
        detail_code: detail_code.map(str::to_string),
        detail_message: detail_message.map(str::to_string),
    }
}

fn synthetic_preview_result_for_test(
    request: &DedicatedRendererPreviewJobRequestDto,
) -> Option<DedicatedRendererPreviewJobResultDto> {
    let outcome = env::var(DEDICATED_RENDERER_TEST_OUTCOME_ENV).ok()?;

    Some(match outcome.as_str() {
        "accepted" => build_preview_result(
            request,
            "accepted",
            Some(request.canonical_preview_output_path.clone()),
            Some("accepted"),
            Some("accepted"),
        ),
        "queue-saturated" => build_preview_result(
            request,
            "queue-saturated",
            None,
            Some("render-queue-saturated"),
            Some("dedicated renderer queue가 포화되어 inline truthful fallback으로 내려가요."),
        ),
        "protocol-mismatch" => build_preview_result(
            request,
            "protocol-mismatch",
            None,
            Some("protocol-mismatch"),
            Some("preview job protocol version이 맞지 않아 fallback path로 내려가요."),
        ),
        "invalid-output" => build_preview_result(
            request,
            "invalid-output",
            Some("C:/outside/non-canonical.jpg".into()),
            Some("invalid-output"),
            Some("non-canonical output을 감지해 fallback path로 내려가요."),
        ),
        "restarted" => build_preview_result(
            request,
            "restarted",
            None,
            Some("renderer-restarted"),
            Some("renderer restart가 감지되어 fallback path로 내려가요."),
        ),
        _ => build_preview_result(
            request,
            "fallback-suggested",
            None,
            Some("sidecar-unavailable"),
            Some("test fallback path"),
        ),
    })
}

fn current_time_ms() -> Result<u64, HostErrorEnvelope> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .map_err(|_| {
            HostErrorEnvelope::persistence(
                "dedicated renderer 완료 시각을 기록하지 못했어요. 잠시 후 다시 시도해 주세요.",
            )
        })
}

#[cfg(test)]
mod tests {
    use super::{
        DEDICATED_RENDERER_REQUEST_SCHEMA_VERSION, DEDICATED_RENDERER_RESULT_SCHEMA_VERSION,
        DEDICATED_RENDERER_WARMUP_REQUEST_SCHEMA_VERSION,
        DEDICATED_RENDERER_WARMUP_RESULT_SCHEMA_VERSION, build_preview_result,
        build_warmup_result, validate_preview_job_result, validate_warmup_result,
    };
    use crate::contracts::dto::{
        DedicatedRendererPreviewJobRequestDto, DedicatedRendererRenderProfileDto,
        DedicatedRendererWarmupRequestDto,
    };

    fn preview_request() -> DedicatedRendererPreviewJobRequestDto {
        DedicatedRendererPreviewJobRequestDto {
            schema_version: DEDICATED_RENDERER_REQUEST_SCHEMA_VERSION.into(),
            session_id: "session_01hs6n1r8b8zc5v4ey2x7b9g1m".into(),
            request_id: "request_20260410_001".into(),
            capture_id: "capture_20260410_001".into(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
            darktable_version: "5.4.1".into(),
            xmp_template_path: "C:/boothy/preset-catalog/published/preset_soft-glow/2026.04.10/xmp/template.xmp".into(),
            preview_profile: DedicatedRendererRenderProfileDto {
                profile_id: "soft-glow-preview".into(),
                display_name: "Soft Glow Preview".into(),
                output_color_space: "sRGB".into(),
            },
            source_asset_path: "C:/boothy/sessions/session/captures/originals/capture_20260410_001.cr3".into(),
            canonical_preview_output_path: "C:/boothy/sessions/session/renders/previews/capture_20260410_001.jpg".into(),
            diagnostics_detail_path: "C:/boothy/sessions/session/diagnostics/dedicated-renderer/request.json".into(),
        }
    }

    fn warmup_request() -> DedicatedRendererWarmupRequestDto {
        DedicatedRendererWarmupRequestDto {
            schema_version: DEDICATED_RENDERER_WARMUP_REQUEST_SCHEMA_VERSION.into(),
            session_id: "session_01hs6n1r8b8zc5v4ey2x7b9g1m".into(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
            darktable_version: "5.4.1".into(),
            xmp_template_path:
                "C:/boothy/preset-catalog/published/preset_soft-glow/2026.04.10/xmp/template.xmp"
                    .into(),
            preview_profile: DedicatedRendererRenderProfileDto {
                profile_id: "soft-glow-preview".into(),
                display_name: "Soft Glow Preview".into(),
                output_color_space: "sRGB".into(),
            },
            diagnostics_detail_path:
                "C:/boothy/sessions/session/diagnostics/dedicated-renderer/warmup-request.json"
                    .into(),
        }
    }

    #[test]
    fn preview_result_validation_rejects_wrong_session_or_capture() {
        let request = preview_request();
        let mut result = build_preview_result(
            &request,
            "fallback-suggested",
            None,
            Some("sidecar-unavailable"),
            Some("fallback"),
        );
        result.schema_version = DEDICATED_RENDERER_RESULT_SCHEMA_VERSION.into();
        result.capture_id = "capture_other".into();

        assert_eq!(
            validate_preview_job_result(&request, &result),
            Err("wrong-session")
        );
    }

    #[test]
    fn preview_result_validation_rejects_non_canonical_output_paths() {
        let request = preview_request();
        let result = build_preview_result(
            &request,
            "invalid-output",
            Some("C:/outside/non-canonical.jpg".into()),
            Some("invalid-output"),
            Some("fallback"),
        );

        assert_eq!(
            validate_preview_job_result(&request, &result),
            Err("non-canonical-output")
        );
    }

    #[test]
    fn preview_result_validation_rejects_schema_mismatch() {
        let request = preview_request();
        let mut result = build_preview_result(
            &request,
            "accepted",
            None,
            Some("accepted"),
            Some("accepted"),
        );
        result.schema_version = "dedicated-renderer-preview-job-result/v9".into();

        assert_eq!(
            validate_preview_job_result(&request, &result),
            Err("protocol-mismatch")
        );
    }

    #[test]
    fn preview_result_validation_rejects_accepted_status_without_canonical_output() {
        let request = preview_request();
        let result = build_preview_result(
            &request,
            "accepted",
            None,
            Some("accepted"),
            Some("accepted"),
        );

        assert_eq!(
            validate_preview_job_result(&request, &result),
            Err("invalid-output")
        );
    }

    #[test]
    fn warmup_result_validation_accepts_typed_runtime_states() {
        let request = warmup_request();
        let mut result = build_warmup_result(
            &request,
            "warmed-up",
            Some("renderer-warm"),
            Some("warm"),
        );
        result.schema_version = DEDICATED_RENDERER_WARMUP_RESULT_SCHEMA_VERSION.into();

        assert_eq!(validate_warmup_result(&request, &result), Ok(()));
    }
}
