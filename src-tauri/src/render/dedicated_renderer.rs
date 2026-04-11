use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Deserialize;
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
        preset_catalog::{
            find_published_preset_runtime_bundle, resolve_published_preset_catalog_dir,
        },
    },
    render::{
        is_valid_render_preview_asset, log_render_failure_in_dir, log_render_ready_in_dir,
        RenderIntent, RenderedCaptureAsset,
    },
    session::{
        session_manifest::{PreviewRendererRouteSnapshot, SessionCaptureRecord},
        session_paths::SessionPaths,
        session_repository::read_session_manifest,
    },
    timing::{append_session_timing_event_in_dir, SessionTimingEventInput},
};

pub const DEDICATED_RENDERER_EXTERNAL_BIN: &str =
    "../sidecar/dedicated-renderer/boothy-dedicated-renderer";
const DEDICATED_RENDERER_PREVIEW_PROTOCOL: &str = "preview-job-v1";
const DEDICATED_RENDERER_WARMUP_PROTOCOL: &str = "warmup-v1";
const DEDICATED_RENDERER_REQUEST_SCHEMA_VERSION: &str = "dedicated-renderer-preview-job-request/v1";
const DEDICATED_RENDERER_RESULT_SCHEMA_VERSION: &str = "dedicated-renderer-preview-job-result/v1";
const DEDICATED_RENDERER_WARMUP_REQUEST_SCHEMA_VERSION: &str =
    "dedicated-renderer-warmup-request/v1";
const DEDICATED_RENDERER_WARMUP_RESULT_SCHEMA_VERSION: &str = "dedicated-renderer-warmup-result/v1";
const DEDICATED_RENDERER_TEST_OUTCOME_ENV: &str = "BOOTHY_TEST_DEDICATED_RENDERER_OUTCOME";
const PREVIEW_RENDERER_ROUTE_POLICY_SCHEMA_VERSION: &str = "preview-renderer-route-policy/v1";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum PreviewRendererRouteKind {
    Darktable,
    LocalRendererSidecar,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewRendererRoutePolicy {
    schema_version: String,
    default_route: PreviewRendererRouteKind,
    #[serde(default)]
    canary_routes: Vec<PreviewRendererRouteRule>,
    #[serde(default)]
    forced_fallback_routes: Vec<PreviewRendererRouteRule>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewRendererRouteRule {
    route: PreviewRendererRouteKind,
    preset_id: String,
    preset_version: String,
    #[serde(default, rename = "reason")]
    _reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedPreviewRendererRoute {
    route: PreviewRendererRouteKind,
    route_stage: &'static str,
    fallback_reason_code: Option<&'static str>,
}

enum PreviewRendererRoutePolicyLoadResult {
    Missing,
    Invalid,
    Loaded(PreviewRendererRoutePolicy),
}

impl PreviewRendererRouteKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Darktable => "darktable",
            Self::LocalRendererSidecar => "local-renderer-sidecar",
        }
    }

    fn from_snapshot_route(value: &str) -> Option<Self> {
        match value {
            "darktable" => Some(Self::Darktable),
            "local-renderer-sidecar" => Some(Self::LocalRendererSidecar),
            _ => None,
        }
    }
}

impl ResolvedPreviewRendererRoute {
    fn snapshot(&self) -> PreviewRendererRouteSnapshot {
        PreviewRendererRouteSnapshot {
            route: self.route.as_str().into(),
            route_stage: self.route_stage.into(),
            fallback_reason_code: self.fallback_reason_code.map(str::to_string),
        }
    }
}

pub fn resolve_preview_renderer_route_snapshot_in_dir(
    base_dir: &Path,
    preset_id: &str,
    preset_version: &str,
) -> PreviewRendererRouteSnapshot {
    resolve_preview_renderer_route_in_dir(base_dir, preset_id, preset_version).snapshot()
}

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
        let route = resolve_preview_renderer_route_in_dir(base_dir, preset_id, preset_version);
        let result =
            submit_warmup_request(app_handle, &request, &request_path, &result_path, &route);
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
    let (request, request_path, result_path, route) =
        build_preview_job_request_in_dir(base_dir, session_id, capture_id)?;
    write_json_file(&request_path, &request)?;

    let preview_result =
        match submit_preview_job(app_handle, &request, &request_path, &result_path, &route) {
            Ok(result) => {
                let _ = write_json_file(&result_path, &result);
                if validate_preview_job_result(&request, &result, &result_path).is_err() {
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
        if let Some(capture) = try_complete_preview_from_dedicated_result_in_dir(
            base_dir, session_id, &request, result,
        )? {
            append_preview_transition_summary_in_dir(
                base_dir,
                &capture,
                "dedicated-renderer",
                None,
                route.route_stage,
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
        route.route_stage,
    );

    Ok(capture)
}

fn build_warmup_request_in_dir(
    base_dir: &Path,
    session_id: &str,
    preset_id: &str,
    preset_version: &str,
) -> Result<(DedicatedRendererWarmupRequestDto, PathBuf, PathBuf), HostErrorEnvelope> {
    let catalog_root = resolve_published_preset_catalog_dir(base_dir);
    let bundle = find_published_preset_runtime_bundle(&catalog_root, preset_id, preset_version)
        .ok_or_else(|| {
            HostErrorEnvelope::preset_catalog_unavailable(
                "dedicated renderer warm-up에 필요한 preset bundle을 찾지 못했어요.",
            )
        })?;
    let diagnostics_dir = dedicated_renderer_diagnostics_dir(base_dir, session_id)?;
    let request_path =
        diagnostics_dir.join(format!("warmup-{preset_id}-{preset_version}.request.json"));
    let result_path =
        diagnostics_dir.join(format!("warmup-{preset_id}-{preset_version}.result.json"));

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
        ResolvedPreviewRendererRoute,
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
    let bundle = find_published_preset_runtime_bundle(
        &catalog_root,
        &preset_id,
        &capture.active_preset_version,
    )
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
    let route = resolve_preview_renderer_route_for_capture(base_dir, &capture);

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
        route,
    ))
}

fn submit_warmup_request(
    app_handle: Option<&AppHandle>,
    request: &DedicatedRendererWarmupRequestDto,
    request_path: &Path,
    result_path: &Path,
    route: &ResolvedPreviewRendererRoute,
) -> Result<DedicatedRendererWarmupResultDto, &'static str> {
    clear_previous_result_file(result_path);

    if route.route == PreviewRendererRouteKind::Darktable {
        return Ok(with_warmup_result_detail_path(
            build_warmup_result(
                request,
                "fallback-suggested",
                route.fallback_reason_code,
                Some("승인된 shadow route를 유지해 inline warm-up을 계속 사용해요."),
            ),
            result_path,
        ));
    }

    if app_handle.is_some() {
        if let Err(reason_code) = try_spawn_dedicated_renderer(
            app_handle,
            DEDICATED_RENDERER_WARMUP_PROTOCOL,
            request_path,
            result_path,
        ) {
            return Ok(with_warmup_result_detail_path(
                build_warmup_spawn_failure_result(request, reason_code),
                result_path,
            ));
        }
    }

    if let Ok(bytes) = fs::read_to_string(result_path) {
        match serde_json::from_str::<DedicatedRendererWarmupResultDto>(&bytes) {
            Ok(result) => match validate_warmup_result(request, &result, result_path) {
                Ok(()) => return Ok(result),
                Err("protocol-mismatch") => {
                    return Ok(with_warmup_result_detail_path(
                        build_warmup_result(
                            request,
                            "protocol-mismatch",
                            Some("protocol-mismatch"),
                            Some(
                                "warm-up result schema 또는 상태 값이 계약과 맞지 않아 shadow fallback으로 유지해요.",
                            ),
                        ),
                        result_path,
                    ));
                }
                Err(_) => {
                    return Ok(with_warmup_result_detail_path(
                        build_warmup_result(
                            request,
                            "fallback-suggested",
                            Some("shadow-inline-warmup"),
                            Some(
                                "warm-up result correlation이 맞지 않아 host-owned inline warm-up을 유지해요.",
                            ),
                        ),
                        result_path,
                    ));
                }
            },
            Err(_) => {
                return Ok(with_warmup_result_detail_path(
                    build_warmup_result(
                        request,
                        "protocol-mismatch",
                        Some("protocol-mismatch"),
                        Some(
                            "warm-up result를 dedicated renderer 계약으로 해석하지 못해 shadow fallback으로 유지해요.",
                        ),
                    ),
                    result_path,
                ));
            }
        }
    }

    Ok(with_warmup_result_detail_path(
        build_warmup_result(
            request,
            "fallback-suggested",
            Some("shadow-inline-warmup"),
            Some("approved cutover 전까지 warm-up은 host-owned inline renderer가 계속 소유해요."),
        ),
        result_path,
    ))
}

fn submit_preview_job(
    app_handle: Option<&AppHandle>,
    request: &DedicatedRendererPreviewJobRequestDto,
    request_path: &Path,
    result_path: &Path,
    route: &ResolvedPreviewRendererRoute,
) -> Result<DedicatedRendererPreviewJobResultDto, &'static str> {
    if let Some(result) = synthetic_preview_result_for_test(request, route) {
        return Ok(with_preview_result_detail_path(result, result_path));
    }

    clear_previous_result_file(result_path);

    if route.route == PreviewRendererRouteKind::Darktable {
        return Ok(with_preview_result_detail_path(
            build_preview_result(
                request,
                "fallback-suggested",
                None,
                route.fallback_reason_code,
                Some("승인된 shadow route를 유지해 inline truthful close로 내려가요."),
            ),
            result_path,
        ));
    }

    if app_handle.is_some() {
        if let Err(reason_code) = try_spawn_dedicated_renderer(
            app_handle,
            DEDICATED_RENDERER_PREVIEW_PROTOCOL,
            request_path,
            result_path,
        ) {
            return Ok(with_preview_result_detail_path(
                build_preview_spawn_failure_result(request, reason_code),
                result_path,
            ));
        }
    } else if resolve_dedicated_renderer_executable().is_none() {
        return Ok(with_preview_result_detail_path(
            build_preview_result(
                request,
                "fallback-suggested",
                None,
                Some("sidecar-unavailable"),
                Some(
                    "dedicated renderer binary가 아직 bundle/runtime에서 확인되지 않아 truthful fallback path로 내려가요.",
                ),
            ),
            result_path,
        ));
    } else {
        return Ok(with_preview_result_detail_path(
            build_preview_result(
                request,
                "fallback-suggested",
                None,
                Some("sidecar-unavailable"),
                Some(
                    "승인된 local renderer route지만 현재 runtime handle이 없어 truthful fallback path로 내려가요.",
                ),
            ),
            result_path,
        ));
    }

    if let Ok(bytes) = fs::read_to_string(result_path) {
        match serde_json::from_str::<DedicatedRendererPreviewJobResultDto>(&bytes) {
            Ok(result) => match validate_preview_job_result(request, &result, result_path) {
                Ok(()) => return Ok(result),
                Err("protocol-mismatch") => {
                    return Ok(with_preview_result_detail_path(
                        build_preview_result(
                            request,
                            "protocol-mismatch",
                            None,
                            Some("protocol-mismatch"),
                            Some(
                                "preview job result schema 또는 상태 값이 계약과 맞지 않아 fallback path로 내려가요.",
                            ),
                        ),
                        result_path,
                    ));
                }
                Err(_) => {
                    return Ok(with_preview_result_detail_path(
                        build_preview_result(
                            request,
                            "invalid-output",
                            None,
                            Some("invalid-output"),
                            Some(
                                "preview job result correlation 또는 canonical output 검증에 실패해 fallback path로 내려가요.",
                            ),
                        ),
                        result_path,
                    ));
                }
            },
            Err(_) => {
                return Ok(with_preview_result_detail_path(
                    build_preview_result(
                        request,
                        "protocol-mismatch",
                        None,
                        Some("protocol-mismatch"),
                        Some(
                            "preview job result를 dedicated renderer 계약으로 해석하지 못해 fallback path로 내려가요.",
                        ),
                    ),
                    result_path,
                ));
            }
        }
    }

    Ok(with_preview_result_detail_path(
        build_preview_result(
            request,
            "fallback-suggested",
            None,
            Some(fallback_reason_for_missing_preview_result(route.route_stage)),
            Some(missing_preview_result_message(route.route_stage)),
        ),
        result_path,
    ))
}

fn fallback_reason_for_missing_preview_result(route_stage: &str) -> &'static str {
    match route_stage {
        "canary" | "default" => "dedicated-renderer-no-result",
        _ => "shadow-submission-only",
    }
}

fn missing_preview_result_message(route_stage: &str) -> &'static str {
    match route_stage {
        "canary" | "default" => {
            "dedicated renderer route가 승인됐지만 결과 파일이 남지 않아 truthful fallback path로 내려가요."
        }
        _ => {
            "Story 1.11 baseline에서는 dedicated renderer submission만 고정하고 truthful close는 inline path가 계속 소유해요."
        }
    }
}

fn validate_preview_job_result(
    request: &DedicatedRendererPreviewJobRequestDto,
    result: &DedicatedRendererPreviewJobResultDto,
    expected_result_path: &Path,
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

    if result.diagnostics_detail_path != path_to_runtime_string(expected_result_path) {
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

    let request_path = PathBuf::from(&request.diagnostics_detail_path);
    if !preview_output_is_fresh_for_request(&output_path, &request_path) {
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
    route_stage: &str,
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
    let detail = build_preview_transition_summary_detail(
        lane_owner,
        fallback_reason,
        route_stage,
        first_visible_ms,
        replacement_ms,
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
        "capture_preview_transition_summary session={} capture_id={} request_id={} lane_owner={} fallback_reason={} route_stage={} first_visible_ms={} replacement_ms={}",
        capture.session_id,
        capture.capture_id,
        capture.request_id,
        lane_owner,
        fallback_reason.unwrap_or("none"),
        route_stage,
        format_optional_metric(first_visible_ms),
        format_optional_metric(replacement_ms),
    );
}

fn format_optional_metric(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".into())
}

fn build_preview_transition_summary_detail(
    lane_owner: &str,
    fallback_reason: Option<&str>,
    route_stage: &str,
    first_visible_ms: Option<u64>,
    replacement_ms: Option<u64>,
) -> String {
    format!(
        "laneOwner={lane_owner};fallbackReason={};routeStage={route_stage};firstVisibleMs={};replacementMs={};originalVisibleToPresetAppliedVisibleMs={}",
        fallback_reason.unwrap_or("none"),
        format_optional_metric(first_visible_ms),
        format_optional_metric(replacement_ms),
        format_optional_metric(replacement_ms),
    )
}

fn clear_previous_result_file(result_path: &Path) {
    if let Err(error) = fs::remove_file(result_path) {
        if error.kind() != std::io::ErrorKind::NotFound {
            log::warn!(
                "dedicated renderer previous result cleanup failed path={} error={error}",
                result_path.display()
            );
        }
    }
}

fn validate_warmup_result(
    request: &DedicatedRendererWarmupRequestDto,
    result: &DedicatedRendererWarmupResultDto,
    expected_result_path: &Path,
) -> Result<(), &'static str> {
    if result.schema_version != DEDICATED_RENDERER_WARMUP_RESULT_SCHEMA_VERSION
        || !matches!(
            result.status.as_str(),
            "fallback-suggested" | "warmed-up" | "restarted" | "protocol-mismatch"
        )
    {
        return Err("protocol-mismatch");
    }

    if result.diagnostics_detail_path != path_to_runtime_string(expected_result_path) {
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
            result
                .detail_code
                .as_deref()
                .unwrap_or("render-queue-saturated"),
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

fn resolve_preview_renderer_route_in_dir(
    base_dir: &Path,
    preset_id: &str,
    preset_version: &str,
) -> ResolvedPreviewRendererRoute {
    let policy = match load_preview_renderer_route_policy_in_dir(base_dir) {
        PreviewRendererRoutePolicyLoadResult::Loaded(policy) => policy,
        PreviewRendererRoutePolicyLoadResult::Missing => {
            return ResolvedPreviewRendererRoute {
                route: PreviewRendererRouteKind::Darktable,
                route_stage: "shadow",
                fallback_reason_code: Some("route-policy-shadow"),
            };
        }
        PreviewRendererRoutePolicyLoadResult::Invalid => {
            return ResolvedPreviewRendererRoute {
                route: PreviewRendererRouteKind::Darktable,
                route_stage: "shadow",
                fallback_reason_code: Some("route-policy-invalid"),
            };
        }
    };

    if policy
        .forced_fallback_routes
        .iter()
        .any(|route| route_matches_preset(route, preset_id, preset_version))
    {
        return ResolvedPreviewRendererRoute {
            route: PreviewRendererRouteKind::Darktable,
            route_stage: "shadow",
            fallback_reason_code: Some("route-policy-rollback"),
        };
    }

    if let Some(route) = policy
        .canary_routes
        .iter()
        .find(|route| route_matches_preset(route, preset_id, preset_version))
    {
        return ResolvedPreviewRendererRoute {
            route: route.route.clone(),
            route_stage: match route.route {
                PreviewRendererRouteKind::Darktable => "shadow",
                PreviewRendererRouteKind::LocalRendererSidecar => "canary",
            },
            fallback_reason_code: match route.route {
                PreviewRendererRouteKind::Darktable => Some("route-policy-shadow"),
                PreviewRendererRouteKind::LocalRendererSidecar => None,
            },
        };
    }

    match policy.default_route {
        PreviewRendererRouteKind::Darktable => ResolvedPreviewRendererRoute {
            route: PreviewRendererRouteKind::Darktable,
            route_stage: "shadow",
            fallback_reason_code: Some("route-policy-shadow"),
        },
        PreviewRendererRouteKind::LocalRendererSidecar => ResolvedPreviewRendererRoute {
            route: PreviewRendererRouteKind::LocalRendererSidecar,
            route_stage: "default",
            fallback_reason_code: None,
        },
    }
}

fn resolve_preview_renderer_route_for_capture(
    base_dir: &Path,
    capture: &SessionCaptureRecord,
) -> ResolvedPreviewRendererRoute {
    capture
        .preview_renderer_route
        .as_ref()
        .and_then(resolved_preview_renderer_route_from_snapshot)
        .unwrap_or_else(|| {
            resolve_preview_renderer_route_in_dir(
                base_dir,
                capture.active_preset_id.as_deref().unwrap_or_default(),
                &capture.active_preset_version,
            )
        })
}

fn resolved_preview_renderer_route_from_snapshot(
    snapshot: &PreviewRendererRouteSnapshot,
) -> Option<ResolvedPreviewRendererRoute> {
    let fallback_reason_code = match snapshot.fallback_reason_code.as_deref() {
        Some("route-policy-shadow") => Some("route-policy-shadow"),
        Some("route-policy-invalid") => Some("route-policy-invalid"),
        Some("route-policy-rollback") => Some("route-policy-rollback"),
        Some(_) => return None,
        None => None,
    };

    Some(ResolvedPreviewRendererRoute {
        route: PreviewRendererRouteKind::from_snapshot_route(&snapshot.route)?,
        route_stage: match snapshot.route_stage.as_str() {
            "shadow" => "shadow",
            "canary" => "canary",
            "default" => "default",
            _ => return None,
        },
        fallback_reason_code,
    })
}

fn load_preview_renderer_route_policy_in_dir(
    base_dir: &Path,
) -> PreviewRendererRoutePolicyLoadResult {
    let policy_path = base_dir
        .join("branch-config")
        .join("preview-renderer-policy.json");
    let bytes = match fs::read_to_string(policy_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return PreviewRendererRoutePolicyLoadResult::Missing;
        }
        Err(_) => return PreviewRendererRoutePolicyLoadResult::Invalid,
    };
    let policy = match serde_json::from_str::<PreviewRendererRoutePolicy>(&bytes) {
        Ok(policy) => policy,
        Err(_) => return PreviewRendererRoutePolicyLoadResult::Invalid,
    };

    if policy.schema_version != PREVIEW_RENDERER_ROUTE_POLICY_SCHEMA_VERSION {
        return PreviewRendererRoutePolicyLoadResult::Invalid;
    }

    PreviewRendererRoutePolicyLoadResult::Loaded(policy)
}

fn route_matches_preset(
    route: &PreviewRendererRouteRule,
    preset_id: &str,
    preset_version: &str,
) -> bool {
    route.preset_id == preset_id && route.preset_version == preset_version
}

fn resolve_dedicated_renderer_executable() -> Option<PathBuf> {
    let host_suffix = format!(
        "{}-x86_64-pc-windows-msvc.exe",
        dedicated_renderer_binary_stem()
    );
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
            let packaged_candidate =
                current_dir.join(format!("{}.exe", dedicated_renderer_binary_stem()));
            if packaged_candidate.is_file() {
                return Some(packaged_candidate);
            }
        }
    }

    None
}

pub fn dedicated_renderer_hardware_capability() -> &'static str {
    if resolve_dedicated_renderer_executable().is_some() {
        "dedicated-renderer-available"
    } else {
        "dedicated-renderer-missing"
    }
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

fn build_preview_spawn_failure_result(
    request: &DedicatedRendererPreviewJobRequestDto,
    reason_code: &'static str,
) -> DedicatedRendererPreviewJobResultDto {
    build_preview_result(
        request,
        "fallback-suggested",
        None,
        Some(reason_code),
        Some(match reason_code {
            "sidecar-unavailable" => {
                "dedicated renderer sidecar를 찾지 못해 truthful fallback path로 내려가요."
            }
            "sidecar-launch-failed" => {
                "dedicated renderer sidecar 실행에 실패해 truthful fallback path로 내려가요."
            }
            _ => "dedicated renderer sidecar를 시작하지 못해 truthful fallback path로 내려가요.",
        }),
    )
}

fn with_preview_result_detail_path(
    mut result: DedicatedRendererPreviewJobResultDto,
    result_path: &Path,
) -> DedicatedRendererPreviewJobResultDto {
    result.diagnostics_detail_path = path_to_runtime_string(result_path);
    result
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

fn build_warmup_spawn_failure_result(
    request: &DedicatedRendererWarmupRequestDto,
    reason_code: &'static str,
) -> DedicatedRendererWarmupResultDto {
    build_warmup_result(
        request,
        "fallback-suggested",
        Some(reason_code),
        Some(match reason_code {
            "sidecar-unavailable" => {
                "dedicated renderer sidecar를 찾지 못해 inline warm-up을 유지해요."
            }
            "sidecar-launch-failed" => {
                "dedicated renderer sidecar 실행에 실패해 inline warm-up을 유지해요."
            }
            _ => "dedicated renderer sidecar를 시작하지 못해 inline warm-up을 유지해요.",
        }),
    )
}

fn with_warmup_result_detail_path(
    mut result: DedicatedRendererWarmupResultDto,
    result_path: &Path,
) -> DedicatedRendererWarmupResultDto {
    result.diagnostics_detail_path = path_to_runtime_string(result_path);
    result
}

fn synthetic_preview_result_for_test(
    request: &DedicatedRendererPreviewJobRequestDto,
    route: &ResolvedPreviewRendererRoute,
) -> Option<DedicatedRendererPreviewJobResultDto> {
    if route.route != PreviewRendererRouteKind::LocalRendererSidecar {
        return None;
    }

    let outcome = env::var(DEDICATED_RENDERER_TEST_OUTCOME_ENV).ok()?;

    Some(match outcome.as_str() {
        "accepted" => {
            refresh_canonical_preview_output_for_test(request);
            build_preview_result(
                request,
                "accepted",
                Some(request.canonical_preview_output_path.clone()),
                Some("accepted"),
                Some("accepted"),
            )
        }
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

fn refresh_canonical_preview_output_for_test(request: &DedicatedRendererPreviewJobRequestDto) {
    let output_path = PathBuf::from(&request.canonical_preview_output_path);
    let bytes = fs::read(&output_path).unwrap_or_else(|_| vec![0xFF, 0xD8, 0xFF, 0xD9]);

    if let Some(parent) = output_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let _ = fs::write(output_path, bytes);
}

fn preview_output_is_fresh_for_request(output_path: &Path, request_path: &Path) -> bool {
    let Ok(output_metadata) = fs::metadata(output_path) else {
        return false;
    };
    let Ok(request_metadata) = fs::metadata(request_path) else {
        return false;
    };
    let Ok(output_modified_at) = output_metadata.modified() else {
        return false;
    };
    let Ok(request_modified_at) = request_metadata.modified() else {
        return false;
    };

    output_modified_at >= request_modified_at
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
    use std::{fs, path::Path, thread, time::Duration};

    use super::{
        build_preview_result, build_preview_transition_summary_detail, build_warmup_result,
        fallback_reason_for_missing_preview_result, preview_output_is_fresh_for_request,
        resolve_preview_renderer_route_in_dir, validate_preview_job_result,
        validate_warmup_result, PreviewRendererRouteKind,
        DEDICATED_RENDERER_REQUEST_SCHEMA_VERSION, DEDICATED_RENDERER_RESULT_SCHEMA_VERSION,
        DEDICATED_RENDERER_WARMUP_REQUEST_SCHEMA_VERSION,
        DEDICATED_RENDERER_WARMUP_RESULT_SCHEMA_VERSION,
        PREVIEW_RENDERER_ROUTE_POLICY_SCHEMA_VERSION,
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
            xmp_template_path:
                "C:/boothy/preset-catalog/published/preset_soft-glow/2026.04.10/xmp/template.xmp"
                    .into(),
            preview_profile: DedicatedRendererRenderProfileDto {
                profile_id: "soft-glow-preview".into(),
                display_name: "Soft Glow Preview".into(),
                output_color_space: "sRGB".into(),
            },
            source_asset_path:
                "C:/boothy/sessions/session/captures/originals/capture_20260410_001.cr3".into(),
            canonical_preview_output_path:
                "C:/boothy/sessions/session/renders/previews/capture_20260410_001.jpg".into(),
            diagnostics_detail_path:
                "C:/boothy/sessions/session/diagnostics/dedicated-renderer/request.json".into(),
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
        let expected_result_path =
            "C:/boothy/sessions/session/diagnostics/dedicated-renderer/result.json";
        let mut result = build_preview_result(
            &request,
            "fallback-suggested",
            None,
            Some("sidecar-unavailable"),
            Some("fallback"),
        );
        result.schema_version = DEDICATED_RENDERER_RESULT_SCHEMA_VERSION.into();
        result.capture_id = "capture_other".into();
        result.diagnostics_detail_path = expected_result_path.into();

        assert_eq!(
            validate_preview_job_result(&request, &result, Path::new(expected_result_path),),
            Err("wrong-session")
        );
    }

    #[test]
    fn preview_result_validation_rejects_non_canonical_output_paths() {
        let request = preview_request();
        let expected_result_path =
            "C:/boothy/sessions/session/diagnostics/dedicated-renderer/result.json";
        let mut result = build_preview_result(
            &request,
            "invalid-output",
            Some("C:/outside/non-canonical.jpg".into()),
            Some("invalid-output"),
            Some("fallback"),
        );
        result.diagnostics_detail_path = expected_result_path.into();

        assert_eq!(
            validate_preview_job_result(&request, &result, Path::new(expected_result_path),),
            Err("non-canonical-output")
        );
    }

    #[test]
    fn preview_result_validation_rejects_schema_mismatch() {
        let request = preview_request();
        let expected_result_path =
            "C:/boothy/sessions/session/diagnostics/dedicated-renderer/result.json";
        let mut result = build_preview_result(
            &request,
            "accepted",
            None,
            Some("accepted"),
            Some("accepted"),
        );
        result.schema_version = "dedicated-renderer-preview-job-result/v9".into();
        result.diagnostics_detail_path = expected_result_path.into();

        assert_eq!(
            validate_preview_job_result(&request, &result, Path::new(expected_result_path),),
            Err("protocol-mismatch")
        );
    }

    #[test]
    fn preview_result_validation_rejects_accepted_status_without_canonical_output() {
        let request = preview_request();
        let expected_result_path =
            "C:/boothy/sessions/session/diagnostics/dedicated-renderer/result.json";
        let mut result = build_preview_result(
            &request,
            "accepted",
            None,
            Some("accepted"),
            Some("accepted"),
        );
        result.diagnostics_detail_path = expected_result_path.into();

        assert_eq!(
            validate_preview_job_result(&request, &result, Path::new(expected_result_path),),
            Err("invalid-output")
        );
    }

    #[test]
    fn preview_result_validation_accepts_the_result_detail_path_companion_file() {
        let request = preview_request();
        let expected_result_path =
            "C:/boothy/sessions/session/diagnostics/dedicated-renderer/result.json";
        let mut result = build_preview_result(
            &request,
            "accepted",
            Some(request.canonical_preview_output_path.clone()),
            Some("accepted"),
            Some("accepted"),
        );
        result.diagnostics_detail_path = expected_result_path.into();

        assert_eq!(
            validate_preview_job_result(&request, &result, Path::new(expected_result_path)),
            Ok(())
        );
    }

    #[test]
    fn warmup_result_validation_accepts_typed_runtime_states() {
        let request = warmup_request();
        let expected_result_path =
            "C:/boothy/sessions/session/diagnostics/dedicated-renderer/warmup-result.json";
        let mut result =
            build_warmup_result(&request, "warmed-up", Some("renderer-warm"), Some("warm"));
        result.schema_version = DEDICATED_RENDERER_WARMUP_RESULT_SCHEMA_VERSION.into();
        result.diagnostics_detail_path = expected_result_path.into();

        assert_eq!(
            validate_warmup_result(&request, &result, Path::new(expected_result_path),),
            Ok(())
        );
    }

    #[test]
    fn warmup_result_validation_accepts_the_result_detail_path_companion_file() {
        let request = warmup_request();
        let expected_result_path =
            "C:/boothy/sessions/session/diagnostics/dedicated-renderer/warmup-result.json";
        let mut result =
            build_warmup_result(&request, "warmed-up", Some("renderer-warm"), Some("warm"));
        result.diagnostics_detail_path = expected_result_path.into();

        assert_eq!(
            validate_warmup_result(&request, &result, Path::new(expected_result_path)),
            Ok(())
        );
    }

    #[test]
    fn preview_renderer_route_defaults_to_shadow_when_policy_is_missing() {
        let base_dir = std::env::temp_dir().join(format!(
            "boothy-preview-route-default-{}",
            std::process::id()
        ));
        let route =
            resolve_preview_renderer_route_in_dir(&base_dir, "preset_soft-glow", "2026.04.10");

        assert_eq!(route.route, PreviewRendererRouteKind::Darktable);
        assert_eq!(route.route_stage, "shadow");
        assert_eq!(route.fallback_reason_code, Some("route-policy-shadow"));
    }

    #[test]
    fn preview_renderer_route_promotes_matching_canary_presets() {
        let base_dir = std::env::temp_dir().join(format!(
            "boothy-preview-route-canary-{}",
            std::process::id()
        ));
        let policy_path = base_dir
            .join("branch-config")
            .join("preview-renderer-policy.json");
        fs::create_dir_all(
            policy_path
                .parent()
                .expect("policy path should have a parent directory"),
        )
        .expect("policy directory should exist");
        fs::write(
            &policy_path,
            serde_json::to_vec_pretty(&serde_json::json!({
              "schemaVersion": PREVIEW_RENDERER_ROUTE_POLICY_SCHEMA_VERSION,
              "defaultRoute": "darktable",
              "canaryRoutes": [
                {
                  "route": "local-renderer-sidecar",
                  "presetId": "preset_soft-glow",
                  "presetVersion": "2026.04.10",
                  "reason": "manual-canary"
                }
              ],
              "forcedFallbackRoutes": []
            }))
            .expect("policy should serialize"),
        )
        .expect("policy should write");

        let route =
            resolve_preview_renderer_route_in_dir(&base_dir, "preset_soft-glow", "2026.04.10");

        assert_eq!(route.route, PreviewRendererRouteKind::LocalRendererSidecar);
        assert_eq!(route.route_stage, "canary");
        assert_eq!(route.fallback_reason_code, None);

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn preview_renderer_route_uses_default_local_sidecar_when_policy_promotes_it() {
        let base_dir = std::env::temp_dir().join(format!(
            "boothy-preview-route-default-sidecar-{}",
            std::process::id()
        ));
        let policy_path = base_dir
            .join("branch-config")
            .join("preview-renderer-policy.json");
        fs::create_dir_all(
            policy_path
                .parent()
                .expect("policy path should have a parent directory"),
        )
        .expect("policy directory should exist");
        fs::write(
            &policy_path,
            serde_json::to_vec_pretty(&serde_json::json!({
              "schemaVersion": PREVIEW_RENDERER_ROUTE_POLICY_SCHEMA_VERSION,
              "defaultRoute": "local-renderer-sidecar",
              "canaryRoutes": [],
              "forcedFallbackRoutes": []
            }))
            .expect("policy should serialize"),
        )
        .expect("policy should write");

        let route =
            resolve_preview_renderer_route_in_dir(&base_dir, "preset_soft-glow", "2026.04.10");

        assert_eq!(route.route, PreviewRendererRouteKind::LocalRendererSidecar);
        assert_eq!(route.route_stage, "default");
        assert_eq!(route.fallback_reason_code, None);

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn preview_renderer_route_forced_fallback_marks_route_policy_rollback() {
        let base_dir = std::env::temp_dir().join(format!(
            "boothy-preview-route-rollback-{}",
            std::process::id()
        ));
        let policy_path = base_dir
            .join("branch-config")
            .join("preview-renderer-policy.json");
        fs::create_dir_all(
            policy_path
                .parent()
                .expect("policy path should have a parent directory"),
        )
        .expect("policy directory should exist");
        fs::write(
            &policy_path,
            serde_json::to_vec_pretty(&serde_json::json!({
              "schemaVersion": PREVIEW_RENDERER_ROUTE_POLICY_SCHEMA_VERSION,
              "defaultRoute": "local-renderer-sidecar",
              "canaryRoutes": [],
              "forcedFallbackRoutes": [
                {
                  "route": "darktable",
                  "presetId": "preset_soft-glow",
                  "presetVersion": "2026.04.10",
                  "reason": "rollback"
                }
              ]
            }))
            .expect("policy should serialize"),
        )
        .expect("policy should write");

        let route =
            resolve_preview_renderer_route_in_dir(&base_dir, "preset_soft-glow", "2026.04.10");

        assert_eq!(route.route, PreviewRendererRouteKind::Darktable);
        assert_eq!(route.route_stage, "shadow");
        assert_eq!(route.fallback_reason_code, Some("route-policy-rollback"));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn preview_renderer_route_marks_invalid_policy_files_as_invalid_route_state() {
        let base_dir = std::env::temp_dir().join(format!(
            "boothy-preview-route-invalid-policy-{}",
            std::process::id()
        ));
        let policy_path = base_dir
            .join("branch-config")
            .join("preview-renderer-policy.json");
        fs::create_dir_all(
            policy_path
                .parent()
                .expect("policy path should have a parent directory"),
        )
        .expect("policy directory should exist");
        fs::write(&policy_path, b"{ not-valid-json").expect("policy should write");

        let route =
            resolve_preview_renderer_route_in_dir(&base_dir, "preset_soft-glow", "2026.04.10");

        assert_eq!(route.route, PreviewRendererRouteKind::Darktable);
        assert_eq!(route.route_stage, "shadow");
        assert_eq!(route.fallback_reason_code, Some("route-policy-invalid"));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn accepted_preview_output_must_be_fresh_for_the_current_request() {
        let base_dir = std::env::temp_dir().join(format!(
            "boothy-preview-request-freshness-{}",
            std::process::id()
        ));
        fs::create_dir_all(&base_dir).expect("temp dir should exist");
        let output_path = base_dir.join("preview.jpg");
        let request_path = base_dir.join("request.json");
        fs::write(&output_path, [0xFF, 0xD8, 0xFF, 0xD9]).expect("preview should write");
        thread::sleep(Duration::from_millis(20));
        fs::write(&request_path, b"{}").expect("request should write");

        assert!(
            !preview_output_is_fresh_for_request(&output_path, &request_path),
            "pre-existing preview files should not be accepted for a later request"
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn accepted_preview_output_is_fresh_when_written_after_the_request() {
        let base_dir = std::env::temp_dir().join(format!(
            "boothy-preview-request-freshness-ok-{}",
            std::process::id()
        ));
        fs::create_dir_all(&base_dir).expect("temp dir should exist");
        let output_path = base_dir.join("preview.jpg");
        let request_path = base_dir.join("request.json");
        fs::write(&request_path, b"{}").expect("request should write");
        thread::sleep(Duration::from_millis(20));
        fs::write(&output_path, [0xFF, 0xD8, 0xFF, 0xD9]).expect("preview should write");

        assert!(
            preview_output_is_fresh_for_request(&output_path, &request_path),
            "outputs generated after the current request should remain eligible"
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn missing_sidecar_result_uses_route_aware_fallback_reason_codes() {
        assert_eq!(
            fallback_reason_for_missing_preview_result("shadow"),
            "shadow-submission-only"
        );
        assert_eq!(
            fallback_reason_for_missing_preview_result("canary"),
            "dedicated-renderer-no-result"
        );
        assert_eq!(
            fallback_reason_for_missing_preview_result("default"),
            "dedicated-renderer-no-result"
        );
    }

    #[test]
    fn preview_transition_summary_keeps_original_visible_to_preset_applied_metric() {
        let detail = build_preview_transition_summary_detail(
            "inline-truthful-fallback",
            Some("shadow-submission-only"),
            "shadow",
            Some(2810),
            Some(3615),
        );

        assert!(detail.contains("firstVisibleMs=2810"));
        assert!(detail.contains("replacementMs=3615"));
        assert!(detail.contains("originalVisibleToPresetAppliedVisibleMs=3615"));
        assert!(detail.contains("routeStage=shadow"));
    }
}
