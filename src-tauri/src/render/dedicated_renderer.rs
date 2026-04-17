use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tauri::{async_runtime, AppHandle};
use tauri_plugin_shell::{process::CommandEvent, ShellExt};

use crate::{
    capture::{
        ingest_pipeline::{complete_preview_render_in_dir, finish_preview_render_in_dir},
        CAPTURE_PIPELINE_LOCK,
    },
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
        session_manifest::{
            current_timestamp, PreviewRendererRouteSnapshot, PreviewRendererWarmStateSnapshot,
            SessionCaptureRecord,
        },
        session_paths::SessionPaths,
        session_repository::{read_session_manifest, write_session_manifest},
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
const PREVIEW_PROMOTION_EVIDENCE_RECORD_SCHEMA_VERSION: &str =
    "preview-promotion-evidence-record/v1";
const ACTUAL_PRIMARY_LANE_TRACK: &str = "actual-primary-lane";
const ACTUAL_PRIMARY_LANE_ROUTE_KIND: &str = "actual-primary-lane";
const ACTUAL_PRIMARY_LANE_OWNER: &str = "local-fullscreen-lane";
const ACTUAL_PRIMARY_LANE_BINARY_IDENTITY: &str = "actual-primary-lane-host";
const ACTUAL_PRIMARY_LANE_SOURCE_IDENTITY: &str = "local-native-gpu-resident-full-screen-lane";
const LEGACY_DEDICATED_RENDERER_OWNER: &str = "dedicated-renderer";
const PROTOTYPE_TRACK: &str = "prototype-track";
const DEDICATED_RENDERER_TEST_OUTCOME_ENV: &str = "BOOTHY_TEST_DEDICATED_RENDERER_OUTCOME";
const DEDICATED_RENDERER_TEST_START_FAILURE_ENV: &str =
    "BOOTHY_TEST_DEDICATED_RENDERER_START_FAILURE";
const PREVIEW_PROMOTION_EVIDENCE_WRITE_FAILURE_ENV: &str =
    "BOOTHY_TEST_PREVIEW_PROMOTION_EVIDENCE_WRITE_FAILURE";
const PREVIEW_RENDERER_ROUTE_POLICY_SCHEMA_VERSION: &str = "preview-renderer-route-policy/v1";
const PREVIEW_PROMOTION_IMPROVEMENT_SUMMARY: &str =
    "strategyVersion=2026-04-16d;promotionGateTargetMs=2500;displaySizedClosePreview=true;sidecarStagingPromote=true;sidecarWindowsPathNormalization=true;alwaysPassCanonicalPreviewCandidate=true;waitForLateFastPreviewCandidate=true;lateFastPreviewWaitBudgetMs=500;waitForLateHelperFastPreviewReady=true;lateHelperFastPreviewWaitBudgetMs=500;dedupeEarlyFastPreviewPromotion=true;skipRedundantShadowWarmupAfterDedicatedWarmup=true;skipSpeculativeCloseWhenDedicatedRouteWarm=true;previewCliLibrary=memory;previewCliDisableOpencl=true;hostPreviewDisableOpencl=true;fastPreviewCapPx=768x768;rawPreviewCapPx=1024x1024";
const DEDICATED_RENDERER_PROCESS_TIMEOUT: Duration = Duration::from_secs(50);

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
    default_routes: Vec<PreviewRendererRouteRule>,
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
    implementation_track: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewPromotionEvidenceRecord {
    schema_version: String,
    observed_at: String,
    session_id: String,
    request_id: String,
    capture_id: String,
    preset_id: Option<String>,
    published_version: String,
    lane_owner: String,
    fallback_reason_code: Option<String>,
    route_stage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    implementation_track: Option<String>,
    warm_state: Option<String>,
    capture_requested_at_ms: u64,
    raw_persisted_at_ms: u64,
    truthful_artifact_ready_at_ms: u64,
    visible_owner: String,
    visible_owner_transition_at_ms: u64,
    first_visible_ms: Option<u64>,
    same_capture_full_screen_visible_ms: Option<u64>,
    replacement_ms: Option<u64>,
    original_visible_to_preset_applied_visible_ms: Option<u64>,
    session_manifest_path: String,
    timing_events_path: String,
    route_policy_snapshot_path: String,
    published_bundle_path: Option<String>,
    catalog_state_path: String,
    preview_asset_path: Option<String>,
    warm_state_detail_path: Option<String>,
    improvement_summary: String,
}

fn preview_promotion_evidence_path_in_dir(base_dir: &Path, session_id: &str) -> Option<PathBuf> {
    Some(
        SessionPaths::try_new(base_dir, session_id)
            .ok()?
            .diagnostics_dir
            .join("dedicated-renderer")
            .join("preview-promotion-evidence.jsonl"),
    )
}

fn find_preview_promotion_evidence_record_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
    request_id: &str,
) -> Result<Option<PreviewPromotionEvidenceRecord>, HostErrorEnvelope> {
    let Some(evidence_path) = preview_promotion_evidence_path_in_dir(base_dir, session_id) else {
        return Ok(None);
    };
    if !evidence_path.is_file() {
        return Ok(None);
    }

    let contents = fs::read_to_string(&evidence_path).map_err(|error| {
        HostErrorEnvelope::persistence(format!(
            "preview promotion evidence를 읽지 못했어요: {error}"
        ))
    })?;

    Ok(contents
        .lines()
        .rev()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<PreviewPromotionEvidenceRecord>(line).ok())
        .find(|record| {
            record.session_id == session_id
                && record.capture_id == capture_id
                && record.request_id == request_id
        }))
}

pub(crate) fn resolve_capture_visibility_owner_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
    request_id: &str,
) -> Result<Option<String>, HostErrorEnvelope> {
    Ok(find_preview_promotion_evidence_record_in_dir(
        base_dir, session_id, capture_id, request_id,
    )?
    .map(|record| {
        if record.visible_owner.trim().is_empty() {
            record.lane_owner
        } else {
            record.visible_owner
        }
    }))
}

pub(crate) fn append_capture_visibility_evidence_update_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
    request_id: &str,
    visible_at_ms: u64,
    visible_owner: Option<&str>,
) -> Result<(), HostErrorEnvelope> {
    let Some(mut record) =
        find_preview_promotion_evidence_record_in_dir(base_dir, session_id, capture_id, request_id)?
    else {
        let Some(evidence_path) = preview_promotion_evidence_path_in_dir(base_dir, session_id) else {
            return Ok(());
        };
        if !evidence_path.is_file() {
            return Ok(());
        }
        return Ok(());
    };
    let Some(evidence_path) = preview_promotion_evidence_path_in_dir(base_dir, session_id) else {
        return Ok(());
    };

    let same_capture_full_screen_visible_ms =
        visible_at_ms.saturating_sub(record.capture_requested_at_ms);
    record.observed_at = current_timestamp(SystemTime::now())?;
    record.visible_owner = visible_owner
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(record.lane_owner.as_str())
        .to_string();
    record.visible_owner_transition_at_ms = visible_at_ms;
    record.same_capture_full_screen_visible_ms = Some(same_capture_full_screen_visible_ms);
    record.replacement_ms = Some(same_capture_full_screen_visible_ms);
    record.original_visible_to_preset_applied_visible_ms = record
        .first_visible_ms
        .map(|first_visible_ms| same_capture_full_screen_visible_ms.saturating_sub(first_visible_ms));

    let serialized = serde_json::to_string(&record).map_err(|error| {
        HostErrorEnvelope::persistence(format!(
            "preview promotion evidence를 직렬화하지 못했어요: {error}"
        ))
    })?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&evidence_path)
        .map_err(|error| {
            HostErrorEnvelope::persistence(format!(
                "preview promotion evidence를 남기지 못했어요: {error}"
            ))
        })?;
    writeln!(file, "{serialized}").map_err(|error| {
        HostErrorEnvelope::persistence(format!(
            "preview promotion evidence를 남기지 못했어요: {error}"
        ))
    })?;

    Ok(())
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
            ACTUAL_PRIMARY_LANE_ROUTE_KIND => Some(Self::LocalRendererSidecar),
            "local-renderer-sidecar" => Some(Self::LocalRendererSidecar),
            _ => None,
        }
    }
}

impl ResolvedPreviewRendererRoute {
    fn is_actual_primary_lane(&self) -> bool {
        matches!(self.route, PreviewRendererRouteKind::LocalRendererSidecar)
            && self.implementation_track == Some(ACTUAL_PRIMARY_LANE_TRACK)
    }

    fn snapshot_route_kind(&self) -> &'static str {
        if self.is_actual_primary_lane() {
            ACTUAL_PRIMARY_LANE_ROUTE_KIND
        } else {
            self.route.as_str()
        }
    }

    fn success_lane_owner(&self) -> &'static str {
        if self.is_actual_primary_lane() {
            ACTUAL_PRIMARY_LANE_OWNER
        } else {
            LEGACY_DEDICATED_RENDERER_OWNER
        }
    }

    fn diagnostics_identity(&self) -> (&'static str, &'static str) {
        if self.is_actual_primary_lane() {
            (
                ACTUAL_PRIMARY_LANE_BINARY_IDENTITY,
                ACTUAL_PRIMARY_LANE_SOURCE_IDENTITY,
            )
        } else {
            ("dedicated-renderer", "dedicated-renderer")
        }
    }

    fn snapshot(&self) -> PreviewRendererRouteSnapshot {
        PreviewRendererRouteSnapshot {
            route: self.snapshot_route_kind().into(),
            route_stage: self.route_stage.into(),
            fallback_reason_code: self.fallback_reason_code.map(str::to_string),
            implementation_track: self.implementation_track.map(str::to_string),
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
        let result = submit_warmup_request(
            base_dir,
            app_handle,
            &request,
            &request_path,
            &result_path,
            &route,
        );
        if let Ok(result) = result {
            let _ = write_json_file(&result_path, &result);
            let _ = sync_active_preview_warm_state_in_manifest(
                base_dir,
                session_id,
                &request.preset_id,
                &request.published_version,
                result
                    .warm_state
                    .as_deref()
                    .or_else(|| match result.status.as_str() {
                        "warmed-up" => Some("warm-ready"),
                        _ => None,
                    }),
                result.warm_state_detail_path.as_deref(),
            );
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

    if route.is_actual_primary_lane() {
        let capture = complete_preview_render_in_dir(base_dir, session_id, capture_id)?;
        let warm_state = resolve_actual_primary_lane_warm_state_in_dir(
            base_dir,
            session_id,
            &request.preset_id,
            &request.published_version,
        );
        append_preview_transition_summary_in_dir(
            base_dir,
            &capture,
            route.success_lane_owner(),
            None,
            route.route_stage,
            route.implementation_track,
            Some(warm_state.state.as_str()),
            warm_state.diagnostics_detail_path.as_deref(),
        );
        let _ = sync_active_preview_warm_state_in_manifest(
            base_dir,
            session_id,
            &request.preset_id,
            &request.published_version,
            Some(warm_state.state.as_str()),
            warm_state.diagnostics_detail_path.as_deref(),
        );
        return Ok(capture);
    }

    write_json_file(&request_path, &request)?;

    let preview_result = match submit_preview_job(
        base_dir,
        app_handle,
        &request,
        &request_path,
        &result_path,
        &route,
    ) {
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
            base_dir, session_id, &route, &request, result,
        )? {
            append_preview_transition_summary_in_dir(
                base_dir,
                &capture,
                route.success_lane_owner(),
                None,
                route.route_stage,
                route.implementation_track,
                result.warm_state.as_deref(),
                result.warm_state_detail_path.as_deref(),
            );
            let _ = sync_active_preview_warm_state_in_manifest(
                base_dir,
                session_id,
                &request.preset_id,
                &request.published_version,
                result.warm_state.as_deref(),
                result.warm_state_detail_path.as_deref(),
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
        route.implementation_track,
        preview_result
            .as_ref()
            .and_then(|result| result.warm_state.as_deref()),
        preview_result
            .as_ref()
            .and_then(|result| result.warm_state_detail_path.as_deref()),
    );
    if let Some(result) = preview_result.as_ref() {
        let _ = sync_active_preview_warm_state_in_manifest(
            base_dir,
            session_id,
            &request.preset_id,
            &request.published_version,
            result.warm_state.as_deref(),
            result.warm_state_detail_path.as_deref(),
        );
    }

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
            preview_source_asset_path: resolve_preview_source_asset_path(&paths, &capture),
            canonical_preview_output_path: path_to_runtime_string(&canonical_output_path),
            diagnostics_detail_path: path_to_runtime_string(&request_path),
        },
        request_path,
        result_path,
        route,
    ))
}

fn resolve_preview_source_asset_path(
    paths: &SessionPaths,
    capture: &SessionCaptureRecord,
) -> Option<String> {
    if let Some(preview_asset_path) = capture.preview.asset_path.as_deref() {
        let preview_asset = Path::new(preview_asset_path);
        if is_session_scoped_asset_path(&paths.session_root, preview_asset)
            && is_valid_render_preview_asset(preview_asset)
        {
            return Some(preview_asset_path.to_string());
        }
    }

    let canonical_preview_asset = paths
        .renders_previews_dir
        .join(format!("{}.jpg", capture.capture_id));
    if is_valid_render_preview_asset(&canonical_preview_asset) {
        return Some(path_to_runtime_string(&canonical_preview_asset));
    }

    None
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

fn submit_warmup_request(
    base_dir: &Path,
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
                None,
                None,
            ),
            result_path,
        ));
    }

    if let Err(reason_code) = try_start_dedicated_renderer(
        app_handle,
        DEDICATED_RENDERER_WARMUP_PROTOCOL,
        request_path,
        result_path,
    ) {
        let warm_state = infer_warmup_spawn_failure_warm_state(
            base_dir,
            &request.session_id,
            &request.preset_id,
            &request.published_version,
        );
        return Ok(with_warmup_result_detail_path(
            build_warmup_spawn_failure_result(request, reason_code, &warm_state),
            result_path,
        ));
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
                            None,
                            None,
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
                            None,
                            None,
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
                        None,
                        None,
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
            None,
            None,
        ),
        result_path,
    ))
}

fn submit_preview_job(
    base_dir: &Path,
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
                None,
                None,
            ),
            result_path,
        ));
    }

    if let Err(reason_code) = try_start_dedicated_renderer(
        app_handle,
        DEDICATED_RENDERER_PREVIEW_PROTOCOL,
        request_path,
        result_path,
    ) {
        let warm_state = infer_preview_spawn_failure_warm_state(
            base_dir,
            &request.session_id,
            &request.preset_id,
            &request.published_version,
        );
        return Ok(with_preview_result_detail_path(
            build_preview_spawn_failure_result(request, reason_code, Some(&warm_state)),
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
                            None,
                            None,
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
                            result.warm_state.as_deref(),
                            result.warm_state_detail_path.clone(),
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
                        None,
                        None,
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
            Some(fallback_reason_for_missing_preview_result(
                route.route_stage,
            )),
            Some(missing_preview_result_message(route.route_stage)),
            Some("cold"),
            None,
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
    route: &ResolvedPreviewRendererRoute,
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
        &build_dedicated_renderer_ready_detail(route, request, result, ready_at_ms),
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
    route: &ResolvedPreviewRendererRoute,
    request: &DedicatedRendererPreviewJobRequestDto,
    result: &DedicatedRendererPreviewJobResultDto,
    ready_at_ms: u64,
) -> String {
    let (binary_identity, source_identity) = route.diagnostics_identity();
    format!(
        "presetId={};publishedVersion={};binary={binary_identity};source={source_identity};readyAtMs={ready_at_ms};detail=status={};detailCode={};warmState={}",
        request.preset_id,
        request.published_version,
        result.status,
        result.detail_code.as_deref().unwrap_or("accepted"),
        result.warm_state.as_deref().unwrap_or("none"),
    )
}

fn append_preview_transition_summary_in_dir(
    base_dir: &Path,
    capture: &SessionCaptureRecord,
    lane_owner: &str,
    fallback_reason: Option<&str>,
    route_stage: &str,
    implementation_track: Option<&str>,
    warm_state: Option<&str>,
    warm_state_detail_path: Option<&str>,
) {
    let first_visible_ms = capture
        .timing
        .fast_preview_visible_at_ms
        .or(capture.timing.preview_visible_at_ms)
        .map(|visible_at_ms| {
            visible_at_ms.saturating_sub(capture.timing.capture_acknowledged_at_ms)
        });
    let replacement_ms = match (
        capture.timing.capture_acknowledged_at_ms,
        capture
            .timing
            .preview_visible_at_ms
            .or(capture.timing.xmp_preview_ready_at_ms),
    ) {
        (capture_acknowledged_at_ms, Some(full_screen_visible_at_ms)) => {
            Some(full_screen_visible_at_ms.saturating_sub(capture_acknowledged_at_ms))
        }
        _ => None,
    };
    let visible_owner_transition_at_ms = capture
        .timing
        .preview_visible_at_ms
        .or(capture.timing.xmp_preview_ready_at_ms);
    let detail = build_preview_transition_summary_detail(
        lane_owner,
        fallback_reason,
        route_stage,
        implementation_track
            .or_else(|| {
                capture
                    .preview_renderer_route
                    .as_ref()
                    .and_then(|snapshot| snapshot.implementation_track.as_deref())
            }),
        visible_owner_transition_at_ms,
        first_visible_ms,
        replacement_ms,
        warm_state,
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
    if let Err(error) = append_preview_promotion_evidence_record_in_dir(
        base_dir,
        capture,
        lane_owner,
        fallback_reason,
        route_stage,
        implementation_track
            .or_else(|| {
                capture
                    .preview_renderer_route
                    .as_ref()
                    .and_then(|snapshot| snapshot.implementation_track.as_deref())
            }),
        warm_state,
        warm_state_detail_path,
        visible_owner_transition_at_ms,
        first_visible_ms,
        replacement_ms,
    ) {
        let detail = format!("reason={}", error.message.replace(char::is_whitespace, "_"));
        let _ = append_session_timing_event_in_dir(
            base_dir,
            SessionTimingEventInput {
                session_id: &capture.session_id,
                event: "preview-promotion-evidence-write-failed",
                capture_id: Some(&capture.capture_id),
                request_id: Some(&capture.request_id),
                detail: Some(&detail),
            },
        );
        log::error!(
            "preview promotion evidence write failed session={} capture_id={} request_id={} message={}",
            capture.session_id,
            capture.capture_id,
            capture.request_id,
            error.message
        );
    }
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
    implementation_track: Option<&str>,
    visible_owner_transition_at_ms: Option<u64>,
    first_visible_ms: Option<u64>,
    replacement_ms: Option<u64>,
    warm_state: Option<&str>,
) -> String {
    let original_visible_to_preset_applied_visible_ms = match (first_visible_ms, replacement_ms) {
        (Some(first_visible_ms), Some(replacement_ms)) => {
            Some(replacement_ms.saturating_sub(first_visible_ms))
        }
        _ => None,
    };
    let mut detail = format!(
        "laneOwner={lane_owner};fallbackReason={};routeStage={route_stage};visibleOwner={lane_owner};visibleOwnerTransitionAtMs={};warmState={};firstVisibleMs={};replacementMs={};originalVisibleToPresetAppliedVisibleMs={}",
        fallback_reason.unwrap_or("none"),
        format_optional_metric(visible_owner_transition_at_ms),
        warm_state.unwrap_or("none"),
        format_optional_metric(first_visible_ms),
        format_optional_metric(replacement_ms),
        format_optional_metric(original_visible_to_preset_applied_visible_ms),
    );
    if let Some(implementation_track) = implementation_track {
        detail = format!("{detail};implementationTrack={implementation_track}");
    }

    detail
}

fn append_preview_promotion_evidence_record_in_dir(
    base_dir: &Path,
    capture: &SessionCaptureRecord,
    lane_owner: &str,
    fallback_reason: Option<&str>,
    route_stage: &str,
    implementation_track: Option<&str>,
    warm_state: Option<&str>,
    warm_state_detail_path: Option<&str>,
    visible_owner_transition_at_ms: Option<u64>,
    first_visible_ms: Option<u64>,
    replacement_ms: Option<u64>,
) -> Result<(), HostErrorEnvelope> {
    if let Some(reason) = env::var_os(PREVIEW_PROMOTION_EVIDENCE_WRITE_FAILURE_ENV) {
        return Err(HostErrorEnvelope::persistence(format!(
            "preview promotion evidence를 남기지 못했어요: {}",
            reason.to_string_lossy()
        )));
    }

    let original_visible_to_preset_applied_visible_ms = match (first_visible_ms, replacement_ms) {
        (Some(first_visible_ms), Some(replacement_ms)) => {
            Some(replacement_ms.saturating_sub(first_visible_ms))
        }
        _ => None,
    };
    let paths = SessionPaths::try_new(base_dir, &capture.session_id)?;
    let diagnostics_dir = dedicated_renderer_diagnostics_dir(base_dir, &capture.session_id)?;
    let evidence_path = diagnostics_dir.join("preview-promotion-evidence.jsonl");
    let (route_policy_snapshot_path, catalog_state_snapshot_path) =
        preview_promotion_snapshot_paths_in_dir(
            base_dir,
            &capture.session_id,
            &capture.capture_id,
        )?;
    if !route_policy_snapshot_path.is_file() {
        return Err(HostErrorEnvelope::persistence(
            "capture 시점 route policy snapshot을 찾지 못했어요.",
        ));
    }
    if !catalog_state_snapshot_path.is_file() {
        return Err(HostErrorEnvelope::persistence(
            "capture 시점 catalog snapshot을 찾지 못했어요.",
        ));
    }
    let published_bundle_path = capture.active_preset_id.as_ref().map(|preset_id| {
        path_to_runtime_string(
            &resolve_published_preset_catalog_dir(base_dir)
                .join(preset_id)
                .join(&capture.active_preset_version)
                .join("bundle.json"),
        )
    });
    let record = PreviewPromotionEvidenceRecord {
        schema_version: PREVIEW_PROMOTION_EVIDENCE_RECORD_SCHEMA_VERSION.into(),
        observed_at: current_timestamp(SystemTime::now())?,
        session_id: capture.session_id.clone(),
        request_id: capture.request_id.clone(),
        capture_id: capture.capture_id.clone(),
        preset_id: capture.active_preset_id.clone(),
        published_version: capture.active_preset_version.clone(),
        lane_owner: lane_owner.into(),
        fallback_reason_code: fallback_reason.map(str::to_string),
        route_stage: route_stage.into(),
        implementation_track: capture
            .preview_renderer_route
            .as_ref()
            .and_then(|snapshot| snapshot.implementation_track.clone())
            .or_else(|| implementation_track.map(str::to_string)),
        warm_state: warm_state.map(str::to_string),
        capture_requested_at_ms: capture.timing.capture_acknowledged_at_ms,
        raw_persisted_at_ms: capture.raw.persisted_at_ms,
        truthful_artifact_ready_at_ms: capture
            .preview
            .ready_at_ms
            .or(capture.timing.xmp_preview_ready_at_ms)
            .ok_or_else(|| {
                HostErrorEnvelope::persistence(
                    "truthful artifact ready 시점을 찾지 못했어요.".to_string(),
                )
            })?,
        visible_owner: lane_owner.into(),
        visible_owner_transition_at_ms: visible_owner_transition_at_ms.ok_or_else(|| {
            HostErrorEnvelope::persistence(
                "visible owner transition 시점을 찾지 못했어요.".to_string(),
            )
        })?,
        first_visible_ms,
        same_capture_full_screen_visible_ms: replacement_ms,
        replacement_ms,
        original_visible_to_preset_applied_visible_ms,
        session_manifest_path: path_to_runtime_string(&paths.manifest_path),
        timing_events_path: path_to_runtime_string(
            &paths.diagnostics_dir.join("timing-events.log"),
        ),
        route_policy_snapshot_path: path_to_runtime_string(&route_policy_snapshot_path),
        published_bundle_path,
        catalog_state_path: path_to_runtime_string(&catalog_state_snapshot_path),
        preview_asset_path: capture.preview.asset_path.clone(),
        warm_state_detail_path: warm_state_detail_path.map(str::to_string),
        improvement_summary: PREVIEW_PROMOTION_IMPROVEMENT_SUMMARY.into(),
    };
    let serialized = serde_json::to_string(&record).map_err(|error| {
        HostErrorEnvelope::persistence(format!(
            "preview promotion evidence를 직렬화하지 못했어요: {error}"
        ))
    })?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&evidence_path)
        .map_err(|error| {
            HostErrorEnvelope::persistence(format!(
                "preview promotion evidence를 남기지 못했어요: {error}"
            ))
        })?;
    writeln!(file, "{serialized}").map_err(|error| {
        HostErrorEnvelope::persistence(format!(
            "preview promotion evidence를 남기지 못했어요: {error}"
        ))
    })?;

    Ok(())
}

pub(crate) fn preview_promotion_snapshot_paths_in_dir(
    base_dir: &Path,
    session_id: &str,
    capture_id: &str,
) -> Result<(PathBuf, PathBuf), HostErrorEnvelope> {
    let diagnostics_dir = dedicated_renderer_diagnostics_dir(base_dir, session_id)?;
    Ok((
        diagnostics_dir.join(format!(
            "captured-preview-renderer-policy-{capture_id}.json"
        )),
        diagnostics_dir.join(format!("captured-catalog-state-{capture_id}.json")),
    ))
}

fn sync_active_preview_warm_state_in_manifest(
    base_dir: &Path,
    session_id: &str,
    preset_id: &str,
    published_version: &str,
    warm_state: Option<&str>,
    diagnostics_detail_path: Option<&str>,
) -> Result<(), HostErrorEnvelope> {
    let Some(warm_state) = warm_state else {
        return Ok(());
    };
    let paths = SessionPaths::try_new(base_dir, session_id)?;
    let _pipeline_guard = CAPTURE_PIPELINE_LOCK.lock().map_err(|_| {
        HostErrorEnvelope::persistence("세션 상태를 잠그지 못했어요. 잠시 후 다시 시도해 주세요.")
    })?;
    let mut manifest = read_session_manifest(&paths.manifest_path)?;
    let active_preset_matches = manifest
        .active_preset
        .as_ref()
        .map(|binding| {
            binding.preset_id == preset_id && binding.published_version == published_version
        })
        .or_else(|| {
            Some(
                manifest.active_preset_id.as_deref() == Some(preset_id)
                    && manifest
                        .active_preview_renderer_warm_state
                        .as_ref()
                        .map(|snapshot| snapshot.published_version.as_str())
                        .unwrap_or(published_version)
                        == published_version,
            )
        })
        .unwrap_or(false);
    if !active_preset_matches {
        return Ok(());
    }
    manifest.active_preview_renderer_warm_state = Some(PreviewRendererWarmStateSnapshot {
        preset_id: preset_id.into(),
        published_version: published_version.into(),
        state: warm_state.into(),
        observed_at: current_timestamp(SystemTime::now())?,
        diagnostics_detail_path: diagnostics_detail_path.map(str::to_string),
    });
    write_session_manifest(&paths.manifest_path, &manifest)
}

fn read_matching_active_warm_state_snapshot_in_dir(
    base_dir: &Path,
    session_id: &str,
    preset_id: &str,
    published_version: &str,
) -> Option<PreviewRendererWarmStateSnapshot> {
    SessionPaths::try_new(base_dir, session_id)
        .ok()
        .and_then(|paths| read_session_manifest(&paths.manifest_path).ok())
        .and_then(|manifest| manifest.active_preview_renderer_warm_state)
        .filter(|snapshot| {
            snapshot.preset_id == preset_id && snapshot.published_version == published_version
        })
}

#[derive(Debug, Clone)]
struct ActualPrimaryLaneWarmState {
    state: String,
    diagnostics_detail_path: Option<String>,
}

fn resolve_actual_primary_lane_warm_state_in_dir(
    base_dir: &Path,
    session_id: &str,
    preset_id: &str,
    published_version: &str,
) -> ActualPrimaryLaneWarmState {
    let Some(snapshot) = read_matching_active_warm_state_snapshot_in_dir(
        base_dir,
        session_id,
        preset_id,
        published_version,
    ) else {
        return ActualPrimaryLaneWarmState {
            state: "cold".into(),
            diagnostics_detail_path: None,
        };
    };

    let diagnostics_detail_path = snapshot.diagnostics_detail_path.clone();
    let detail_exists = diagnostics_detail_path
        .as_deref()
        .map(PathBuf::from)
        .map(|path| path.is_file())
        .unwrap_or(false);
    let state = match snapshot.state.as_str() {
        "warm-ready" | "warm-hit" if detail_exists => "warm-hit",
        "warm-ready" | "warm-hit" => "warm-state-lost",
        state => state,
    };

    ActualPrimaryLaneWarmState {
        state: state.into(),
        diagnostics_detail_path,
    }
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

fn try_start_dedicated_renderer(
    app_handle: Option<&AppHandle>,
    protocol: &str,
    request_path: &Path,
    result_path: &Path,
) -> Result<(), &'static str> {
    if let Some(reason_code) = forced_start_failure_for_test() {
        return Err(reason_code);
    }

    if let Some(app_handle) = app_handle {
        let shell_result = match app_handle.shell().sidecar(DEDICATED_RENDERER_EXTERNAL_BIN) {
            Ok(command) => wait_for_shell_command_with_timeout(
                command
                    .arg("--protocol")
                    .arg(protocol)
                    .arg("--request")
                    .arg(request_path)
                    .arg("--result")
                    .arg(result_path),
                DEDICATED_RENDERER_PROCESS_TIMEOUT,
            ),
            Err(_) => Err("sidecar-unavailable"),
        };
        if matches!(
            shell_result,
            Err("sidecar-unavailable") | Err("sidecar-launch-failed")
        ) {
            return reconcile_sidecar_start_result(
                shell_result,
                try_start_dedicated_renderer_directly(protocol, request_path, result_path),
            );
        }

        return shell_result;
    }

    try_start_dedicated_renderer_directly(protocol, request_path, result_path)
}

fn try_start_dedicated_renderer_directly(
    protocol: &str,
    request_path: &Path,
    result_path: &Path,
) -> Result<(), &'static str> {
    let executable = resolve_dedicated_renderer_executable().ok_or("sidecar-unavailable")?;
    let mut child = Command::new(executable)
        .arg("--protocol")
        .arg(protocol)
        .arg("--request")
        .arg(request_path)
        .arg("--result")
        .arg(result_path)
        .spawn()
        .map_err(|_| "sidecar-launch-failed")?;

    wait_for_child_exit_with_timeout(&mut child, DEDICATED_RENDERER_PROCESS_TIMEOUT)
}

fn reconcile_sidecar_start_result(
    shell_result: Result<(), &'static str>,
    direct_result: Result<(), &'static str>,
) -> Result<(), &'static str> {
    match shell_result {
        Ok(()) => Ok(()),
        Err("sidecar-unavailable" | "sidecar-launch-failed") => direct_result,
        Err(reason_code) => Err(reason_code),
    }
}

fn wait_for_shell_command_with_timeout(
    command: tauri_plugin_shell::process::Command,
    timeout: Duration,
) -> Result<(), &'static str> {
    let (mut receiver, child) = command.spawn().map_err(|_| "sidecar-launch-failed")?;
    let (result_tx, result_rx) = mpsc::channel();

    thread::spawn(move || {
        let result = async_runtime::block_on(async move {
            while let Some(event) = receiver.recv().await {
                match event {
                    CommandEvent::Terminated(payload) => {
                        return if payload.code.unwrap_or_default() == 0 {
                            Ok(())
                        } else {
                            Err("sidecar-launch-failed")
                        };
                    }
                    CommandEvent::Error(_) => return Err("sidecar-launch-failed"),
                    _ => {}
                }
            }

            Err("sidecar-launch-failed")
        });
        let _ = result_tx.send(result);
    });

    match result_rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            let _ = child.kill();
            Err("sidecar-launch-timeout")
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => Err("sidecar-launch-failed"),
    }
}

fn wait_for_child_exit_with_timeout(
    child: &mut std::process::Child,
    timeout: Duration,
) -> Result<(), &'static str> {
    let started_at = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return if status.success() {
                    Ok(())
                } else {
                    Err("sidecar-launch-failed")
                };
            }
            Ok(None) => {
                if started_at.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("sidecar-launch-timeout");
                }

                thread::sleep(Duration::from_millis(50));
            }
            Err(_) => {
                let _ = child.kill();
                return Err("sidecar-launch-failed");
            }
        }
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
                implementation_track: Some(PROTOTYPE_TRACK),
            };
        }
        PreviewRendererRoutePolicyLoadResult::Invalid => {
            return ResolvedPreviewRendererRoute {
                route: PreviewRendererRouteKind::Darktable,
                route_stage: "shadow",
                fallback_reason_code: Some("route-policy-invalid"),
                implementation_track: Some(PROTOTYPE_TRACK),
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
            implementation_track: Some(PROTOTYPE_TRACK),
        };
    }

    if let Some(route) = policy
        .default_routes
        .iter()
        .find(|route| route_matches_preset(route, preset_id, preset_version))
    {
        return ResolvedPreviewRendererRoute {
            route: route.route.clone(),
            route_stage: match route.route {
                PreviewRendererRouteKind::Darktable => "shadow",
                PreviewRendererRouteKind::LocalRendererSidecar => "default",
            },
            fallback_reason_code: match route.route {
                PreviewRendererRouteKind::Darktable => Some("route-policy-shadow"),
                PreviewRendererRouteKind::LocalRendererSidecar => None,
            },
            implementation_track: Some(match route.route {
                PreviewRendererRouteKind::Darktable => PROTOTYPE_TRACK,
                PreviewRendererRouteKind::LocalRendererSidecar => ACTUAL_PRIMARY_LANE_TRACK,
            }),
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
            implementation_track: Some(match route.route {
                PreviewRendererRouteKind::Darktable => PROTOTYPE_TRACK,
                PreviewRendererRouteKind::LocalRendererSidecar => ACTUAL_PRIMARY_LANE_TRACK,
            }),
        };
    }

    match policy.default_route {
        PreviewRendererRouteKind::Darktable => ResolvedPreviewRendererRoute {
            route: PreviewRendererRouteKind::Darktable,
            route_stage: "shadow",
            fallback_reason_code: Some("route-policy-shadow"),
            implementation_track: Some(PROTOTYPE_TRACK),
        },
        PreviewRendererRouteKind::LocalRendererSidecar => ResolvedPreviewRendererRoute {
            route: PreviewRendererRouteKind::LocalRendererSidecar,
            route_stage: "default",
            fallback_reason_code: None,
            implementation_track: Some(ACTUAL_PRIMARY_LANE_TRACK),
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
        implementation_track: match snapshot.implementation_track.as_deref() {
            Some(ACTUAL_PRIMARY_LANE_TRACK) => Some(ACTUAL_PRIMARY_LANE_TRACK),
            Some(PROTOTYPE_TRACK) => Some(PROTOTYPE_TRACK),
            Some(_) => return None,
            None => None,
        },
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
    let target = compiled_target_triple()?;
    let repo_binary_name = sidecar_binary_name_for_target(target, false)?;
    let packaged_binary_name = sidecar_binary_name_for_target(target, true)?;
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    let repo_candidate = repo_root
        .join("sidecar")
        .join("dedicated-renderer")
        .join(repo_binary_name);

    if repo_candidate.is_file() {
        return Some(repo_candidate);
    }

    if let Ok(current_exe) = env::current_exe() {
        if let Some(current_dir) = current_exe.parent() {
            let packaged_candidate = current_dir.join(packaged_binary_name);
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

fn compiled_target_triple() -> Option<&'static str> {
    if cfg!(all(target_arch = "x86_64", target_os = "windows")) {
        Some("x86_64-pc-windows-msvc")
    } else if cfg!(all(target_arch = "x86_64", target_os = "linux")) {
        Some("x86_64-unknown-linux-gnu")
    } else if cfg!(all(target_arch = "x86_64", target_os = "macos")) {
        Some("x86_64-apple-darwin")
    } else if cfg!(all(target_arch = "aarch64", target_os = "macos")) {
        Some("aarch64-apple-darwin")
    } else {
        None
    }
}

fn sidecar_binary_name_for_target(target: &str, packaged: bool) -> Option<String> {
    if target.trim().is_empty() {
        return None;
    }

    let stem = dedicated_renderer_binary_stem();
    if packaged {
        return Some(if target.contains("windows") {
            format!("{stem}.exe")
        } else {
            stem.to_string()
        });
    }

    Some(if target.contains("windows") {
        format!("{stem}-{target}.exe")
    } else {
        format!("{stem}-{target}")
    })
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
    warm_state: Option<&str>,
    warm_state_detail_path: Option<String>,
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
        warm_state: warm_state.map(str::to_string),
        warm_state_detail_path,
    }
}

fn build_preview_spawn_failure_result(
    request: &DedicatedRendererPreviewJobRequestDto,
    reason_code: &'static str,
    warm_state: Option<&PreviewSpawnFailureWarmState>,
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
            "sidecar-launch-timeout" => {
                "dedicated renderer sidecar 응답이 제한 시간 안에 끝나지 않아 truthful fallback path로 내려가요."
            }
            "sidecar-launch-failed" => {
                "dedicated renderer sidecar 실행에 실패해 truthful fallback path로 내려가요."
            }
            _ => "dedicated renderer sidecar를 시작하지 못해 truthful fallback path로 내려가요.",
        }),
        warm_state.map(|state| state.state),
        warm_state.and_then(|state| state.diagnostics_detail_path.clone()),
    )
}

#[derive(Debug, Clone)]
struct PreviewSpawnFailureWarmState {
    state: &'static str,
    diagnostics_detail_path: Option<String>,
}

type WarmupSpawnFailureWarmState = PreviewSpawnFailureWarmState;

fn infer_warmup_spawn_failure_warm_state(
    base_dir: &Path,
    session_id: &str,
    preset_id: &str,
    published_version: &str,
) -> WarmupSpawnFailureWarmState {
    infer_preview_spawn_failure_warm_state(base_dir, session_id, preset_id, published_version)
}

fn infer_preview_spawn_failure_warm_state(
    base_dir: &Path,
    session_id: &str,
    preset_id: &str,
    published_version: &str,
) -> PreviewSpawnFailureWarmState {
    let snapshot = SessionPaths::try_new(base_dir, session_id)
        .ok()
        .and_then(|paths| read_session_manifest(&paths.manifest_path).ok())
        .and_then(|manifest| manifest.active_preview_renderer_warm_state)
        .filter(|snapshot| {
            snapshot.preset_id == preset_id && snapshot.published_version == published_version
        });

    match snapshot {
        Some(snapshot) if matches!(snapshot.state.as_str(), "warm-ready" | "warm-hit") => {
            PreviewSpawnFailureWarmState {
                state: "warm-state-lost",
                diagnostics_detail_path: snapshot.diagnostics_detail_path,
            }
        }
        Some(snapshot) if snapshot.state == "warm-state-lost" => PreviewSpawnFailureWarmState {
            state: "warm-state-lost",
            diagnostics_detail_path: snapshot.diagnostics_detail_path,
        },
        Some(snapshot) => PreviewSpawnFailureWarmState {
            state: "cold",
            diagnostics_detail_path: snapshot.diagnostics_detail_path,
        },
        None => PreviewSpawnFailureWarmState {
            state: "cold",
            diagnostics_detail_path: None,
        },
    }
}

fn forced_start_failure_for_test() -> Option<&'static str> {
    match env::var(DEDICATED_RENDERER_TEST_START_FAILURE_ENV)
        .ok()?
        .as_str()
    {
        "sidecar-unavailable" => Some("sidecar-unavailable"),
        "sidecar-launch-timeout" => Some("sidecar-launch-timeout"),
        "sidecar-launch-failed" => Some("sidecar-launch-failed"),
        _ => None,
    }
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
    warm_state: Option<&str>,
    warm_state_detail_path: Option<String>,
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
        warm_state: warm_state.map(str::to_string),
        warm_state_detail_path,
    }
}

fn build_warmup_spawn_failure_result(
    request: &DedicatedRendererWarmupRequestDto,
    reason_code: &'static str,
    warm_state: &WarmupSpawnFailureWarmState,
) -> DedicatedRendererWarmupResultDto {
    build_warmup_result(
        request,
        "fallback-suggested",
        Some(reason_code),
        Some(match reason_code {
            "sidecar-unavailable" => {
                "dedicated renderer sidecar를 찾지 못해 inline warm-up을 유지해요."
            }
            "sidecar-launch-timeout" => {
                "dedicated renderer sidecar 응답이 제한 시간 안에 끝나지 않아 inline warm-up을 유지해요."
            }
            "sidecar-launch-failed" => {
                "dedicated renderer sidecar 실행에 실패해 inline warm-up을 유지해요."
            }
            _ => "dedicated renderer sidecar를 시작하지 못해 inline warm-up을 유지해요.",
        }),
        Some(warm_state.state),
        warm_state.diagnostics_detail_path.clone(),
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
                Some("warm-hit"),
                None,
            )
        }
        "queue-saturated" => build_preview_result(
            request,
            "queue-saturated",
            None,
            Some("render-queue-saturated"),
            Some("dedicated renderer queue가 포화되어 inline truthful fallback으로 내려가요."),
            Some("warm-state-lost"),
            None,
        ),
        "protocol-mismatch" => build_preview_result(
            request,
            "protocol-mismatch",
            None,
            Some("protocol-mismatch"),
            Some("preview job protocol version이 맞지 않아 fallback path로 내려가요."),
            None,
            None,
        ),
        "invalid-output" => build_preview_result(
            request,
            "invalid-output",
            Some("C:/outside/non-canonical.jpg".into()),
            Some("invalid-output"),
            Some("non-canonical output을 감지해 fallback path로 내려가요."),
            Some("warm-hit"),
            None,
        ),
        "restarted" => build_preview_result(
            request,
            "restarted",
            None,
            Some("renderer-restarted"),
            Some("renderer restart가 감지되어 fallback path로 내려가요."),
            Some("warm-state-lost"),
            None,
        ),
        _ => build_preview_result(
            request,
            "fallback-suggested",
            None,
            Some("sidecar-unavailable"),
            Some("test fallback path"),
            None,
            None,
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
    use std::{fs, path::Path, process::Command, thread, time::Duration};

    use super::{
        build_preview_result, build_preview_transition_summary_detail, build_warmup_result,
        fallback_reason_for_missing_preview_result, preview_output_is_fresh_for_request,
        reconcile_sidecar_start_result, resolve_preview_renderer_route_in_dir,
        resolve_preview_renderer_route_snapshot_in_dir, resolve_preview_source_asset_path,
        resolved_preview_renderer_route_from_snapshot, sidecar_binary_name_for_target,
        validate_preview_job_result, validate_warmup_result,
        wait_for_child_exit_with_timeout, PreviewRendererRouteKind,
        PreviewRendererRouteSnapshot, ACTUAL_PRIMARY_LANE_TRACK, PROTOTYPE_TRACK,
        DEDICATED_RENDERER_REQUEST_SCHEMA_VERSION, DEDICATED_RENDERER_RESULT_SCHEMA_VERSION,
        DEDICATED_RENDERER_WARMUP_REQUEST_SCHEMA_VERSION,
        DEDICATED_RENDERER_WARMUP_RESULT_SCHEMA_VERSION,
        PREVIEW_RENDERER_ROUTE_POLICY_SCHEMA_VERSION,
    };
    use crate::contracts::dto::{
        DedicatedRendererPreviewJobRequestDto, DedicatedRendererRenderProfileDto,
        DedicatedRendererWarmupRequestDto,
    };
    use crate::session::{
        session_manifest::{
            CaptureTimingMetrics, FinalCaptureAsset, PreviewCaptureAsset, RawCaptureAsset,
            SessionCaptureRecord, CAPTURE_BUDGET_MS, PREVIEW_BUDGET_MS,
            SESSION_CAPTURE_SCHEMA_VERSION,
        },
        session_paths::SessionPaths,
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
            preview_source_asset_path: None,
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
            None,
            None,
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
            None,
            None,
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
            None,
            None,
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
            None,
            None,
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
            None,
            None,
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
        let mut result = build_warmup_result(
            &request,
            "warmed-up",
            Some("renderer-warm"),
            Some("warm"),
            Some("warm-ready"),
            None,
        );
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
        let mut result = build_warmup_result(
            &request,
            "warmed-up",
            Some("renderer-warm"),
            Some("warm"),
            Some("warm-ready"),
            None,
        );
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
              "defaultRoutes": [],
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
    fn preview_renderer_route_snapshot_uses_a_distinct_actual_lane_route_kind() {
        let base_dir = std::env::temp_dir().join(format!(
            "boothy-preview-route-distinct-actual-{}",
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
              "defaultRoutes": [],
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

        let snapshot =
            resolve_preview_renderer_route_snapshot_in_dir(&base_dir, "preset_soft-glow", "2026.04.10");

        assert_eq!(snapshot.route, "actual-primary-lane");
        assert_eq!(
            snapshot.implementation_track.as_deref(),
            Some(ACTUAL_PRIMARY_LANE_TRACK)
        );

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
              "defaultRoutes": [],
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
              "defaultRoutes": [],
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
    fn preview_renderer_route_scopes_default_promotion_to_matching_presets_only() {
        let base_dir = std::env::temp_dir().join(format!(
            "boothy-preview-route-scoped-default-{}",
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
              "defaultRoutes": [
                {
                  "route": "local-renderer-sidecar",
                  "presetId": "preset_soft-glow",
                  "presetVersion": "2026.04.10",
                  "reason": "host-approved-default"
                }
              ],
              "canaryRoutes": [],
              "forcedFallbackRoutes": []
            }))
            .expect("policy should serialize"),
        )
        .expect("policy should write");

        let matching_route =
            resolve_preview_renderer_route_in_dir(&base_dir, "preset_soft-glow", "2026.04.10");
        assert_eq!(
            matching_route.route,
            PreviewRendererRouteKind::LocalRendererSidecar
        );
        assert_eq!(matching_route.route_stage, "default");
        assert_eq!(matching_route.fallback_reason_code, None);

        let unrelated_route =
            resolve_preview_renderer_route_in_dir(&base_dir, "preset_cool-tone", "2026.04.11");
        assert_eq!(unrelated_route.route, PreviewRendererRouteKind::Darktable);
        assert_eq!(unrelated_route.route_stage, "shadow");
        assert_eq!(
            unrelated_route.fallback_reason_code,
            Some("route-policy-shadow")
        );

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
    fn local_sidecar_process_times_out_and_is_killed() {
        let mut child = Command::new("powershell")
            .args(["-NoProfile", "-Command", "Start-Sleep -Milliseconds 4000"])
            .spawn()
            .expect("sleeping process should spawn");

        let started_at = std::time::Instant::now();
        let result = wait_for_child_exit_with_timeout(&mut child, Duration::from_millis(100));

        assert_eq!(result, Err("sidecar-launch-timeout"));
        assert!(
            started_at.elapsed() < Duration::from_secs(2),
            "timed out child should be cut off quickly"
        );
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
    fn preview_transition_summary_reports_original_visible_delta() {
        let detail = build_preview_transition_summary_detail(
            "inline-truthful-fallback",
            Some("shadow-submission-only"),
            "shadow",
            Some(PROTOTYPE_TRACK),
            Some(6425),
            Some(2810),
            Some(3615),
            Some("warm-ready"),
        );

        assert!(detail.contains("visibleOwnerTransitionAtMs=6425"));
        assert!(detail.contains("firstVisibleMs=2810"));
        assert!(detail.contains("replacementMs=3615"));
        assert!(detail.contains("originalVisibleToPresetAppliedVisibleMs=805"));
        assert!(detail.contains("routeStage=shadow"));
        assert!(detail.contains("warmState=warm-ready"));
    }

    #[test]
    fn preview_transition_summary_marks_actual_lane_track_separately_from_prototype_route() {
        let detail = build_preview_transition_summary_detail(
            "dedicated-renderer",
            None,
            "canary",
            Some(ACTUAL_PRIMARY_LANE_TRACK),
            Some(2410),
            Some(1605),
            Some(2410),
            Some("warm-ready"),
        );

        assert!(detail.contains("laneOwner=dedicated-renderer"));
        assert!(detail.contains("implementationTrack=actual-primary-lane"));
        assert!(detail.contains("routeStage=canary"));
    }

    #[test]
    fn legacy_snapshot_without_track_stays_untyped() {
        let route = resolved_preview_renderer_route_from_snapshot(&PreviewRendererRouteSnapshot {
            route: "local-renderer-sidecar".into(),
            route_stage: "canary".into(),
            fallback_reason_code: None,
            implementation_track: None,
        })
        .expect("legacy snapshot should still parse");

        assert_eq!(route.route_stage, "canary");
        assert_eq!(route.implementation_track, None);
    }

    #[test]
    fn sidecar_binary_names_cover_supported_targets() {
        assert_eq!(
            sidecar_binary_name_for_target("x86_64-pc-windows-msvc", false).as_deref(),
            Some("boothy-dedicated-renderer-x86_64-pc-windows-msvc.exe")
        );
        assert_eq!(
            sidecar_binary_name_for_target("x86_64-pc-windows-msvc", true).as_deref(),
            Some("boothy-dedicated-renderer.exe")
        );
        assert_eq!(
            sidecar_binary_name_for_target("x86_64-unknown-linux-gnu", false).as_deref(),
            Some("boothy-dedicated-renderer-x86_64-unknown-linux-gnu")
        );
        assert_eq!(
            sidecar_binary_name_for_target("x86_64-unknown-linux-gnu", true).as_deref(),
            Some("boothy-dedicated-renderer")
        );
        assert_eq!(
            sidecar_binary_name_for_target("aarch64-apple-darwin", false).as_deref(),
            Some("boothy-dedicated-renderer-aarch64-apple-darwin")
        );
        assert_eq!(
            sidecar_binary_name_for_target("aarch64-apple-darwin", true).as_deref(),
            Some("boothy-dedicated-renderer")
        );
    }

    #[test]
    fn shell_sidecar_launch_failure_can_fall_back_to_direct_start() {
        assert_eq!(
            reconcile_sidecar_start_result(Err("sidecar-launch-failed"), Ok(())),
            Ok(())
        );
        assert_eq!(
            reconcile_sidecar_start_result(Err("sidecar-unavailable"), Ok(())),
            Ok(())
        );
    }

    #[test]
    fn successful_shell_sidecar_start_does_not_require_direct_retry() {
        assert_eq!(
            reconcile_sidecar_start_result(Ok(()), Err("sidecar-launch-failed")),
            Ok(())
        );
    }

    #[test]
    fn shell_sidecar_timeout_keeps_the_timeout_result() {
        assert_eq!(
            reconcile_sidecar_start_result(Err("sidecar-launch-timeout"), Ok(())),
            Err("sidecar-launch-timeout")
        );
    }

    #[test]
    fn preview_request_can_include_fast_preview_source_asset() {
        let mut request = preview_request();
        request.preview_source_asset_path = Some(
            "C:/boothy/sessions/session/renders/previews/capture_20260410_001-fast.jpg".into(),
        );

        let serialized = serde_json::to_string(&request).expect("request should serialize");

        assert!(serialized.contains("previewSourceAssetPath"));
        assert!(serialized.contains("capture_20260410_001-fast.jpg"));
    }

    #[test]
    fn preview_source_asset_path_requires_an_existing_canonical_preview_file() {
        let base_dir = std::env::temp_dir().join(format!(
            "boothy-preview-source-guard-{}",
            std::process::id()
        ));
        let session_id = "session_01hs6n1r8b8zc5v4ey2x7b9g1m";
        let paths = SessionPaths::new(&base_dir, session_id);
        let capture = SessionCaptureRecord {
            schema_version: SESSION_CAPTURE_SCHEMA_VERSION.into(),
            session_id: session_id.into(),
            booth_alias: "Booth A".into(),
            active_preset_id: Some("preset_soft-glow".into()),
            active_preset_version: "2026.04.10".into(),
            active_preset_display_name: Some("Soft Glow".into()),
            preview_renderer_route: None,
            capture_id: "capture_20260410_001".into(),
            request_id: "request_20260410_001".into(),
            raw: RawCaptureAsset {
                asset_path: "C:/boothy/sessions/session/captures/originals/capture_20260410_001.cr3"
                    .into(),
                persisted_at_ms: 100,
            },
            preview: PreviewCaptureAsset {
                asset_path: None,
                enqueued_at_ms: Some(120),
                ready_at_ms: None,
            },
            final_asset: FinalCaptureAsset {
                asset_path: None,
                ready_at_ms: None,
            },
            render_status: "previewPending".into(),
            post_end_state: "activeSession".into(),
            timing: CaptureTimingMetrics {
                capture_acknowledged_at_ms: 100,
                preview_visible_at_ms: None,
                fast_preview_visible_at_ms: None,
                xmp_preview_ready_at_ms: None,
                capture_budget_ms: CAPTURE_BUDGET_MS,
                preview_budget_ms: PREVIEW_BUDGET_MS,
                preview_budget_state: "pending".into(),
            },
        };

        assert_eq!(resolve_preview_source_asset_path(&paths, &capture), None);
    }
}
