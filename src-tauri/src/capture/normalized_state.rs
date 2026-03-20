use std::{fs, path::Path};

use crate::{
    capture::ingest_pipeline::persist_capture_in_dir,
    contracts::dto::{
        CaptureReadinessDto, CaptureReadinessInputDto, CaptureRequestInputDto,
        CaptureRequestResultDto, HostErrorEnvelope,
    },
    preset::preset_catalog::{find_published_preset_summary, resolve_published_preset_catalog_dir},
    session::{
        session_manifest::{ActivePresetBinding, SessionManifest},
        session_paths::SessionPaths,
    },
};

pub fn get_capture_readiness_in_dir(
    base_dir: &Path,
    input: CaptureReadinessInputDto,
) -> Result<CaptureReadinessDto, HostErrorEnvelope> {
    let manifest = read_session_manifest(base_dir, &input.session_id)?;

    Ok(normalize_capture_readiness(base_dir, &manifest))
}

pub fn request_capture_in_dir(
    base_dir: &Path,
    input: CaptureRequestInputDto,
) -> Result<CaptureRequestResultDto, HostErrorEnvelope> {
    let readiness = get_capture_readiness_in_dir(
        base_dir,
        CaptureReadinessInputDto {
            session_id: input.session_id.clone(),
        },
    )?;

    if !readiness.can_capture {
        return Err(HostErrorEnvelope::capture_not_ready(
            "지금은 촬영할 수 없어요.",
            readiness,
        ));
    }

    let (manifest, capture) = persist_capture_in_dir(base_dir, &input)?;

    Ok(CaptureRequestResultDto {
        schema_version: "capture-request-result/v1".into(),
        session_id: input.session_id,
        status: "capture-saved".into(),
        capture: capture.clone(),
        readiness: CaptureReadinessDto::capture_saved(manifest.session_id.clone(), capture),
    })
}

pub fn normalize_capture_readiness(
    base_dir: &Path,
    manifest: &SessionManifest,
) -> CaptureReadinessDto {
    if !has_valid_active_preset(base_dir, manifest.active_preset.as_ref()) {
        return CaptureReadinessDto::preset_missing(manifest.session_id.clone());
    }

    let latest_capture = manifest.captures.last().cloned();

    match manifest.lifecycle.stage.as_str() {
        "ready" | "capture-ready" | "preset-selected" => match latest_capture {
            Some(capture)
                if capture.render_status == "previewWaiting"
                    || capture.render_status == "captureSaved" =>
            {
                CaptureReadinessDto::preview_waiting(manifest.session_id.clone(), Some(capture))
            }
            Some(capture) if capture.render_status == "renderFailed" => {
                CaptureReadinessDto::phone_required(manifest.session_id.clone())
            }
            Some(capture) if capture.render_status == "previewReady" => {
                CaptureReadinessDto::preview_ready(manifest.session_id.clone(), capture)
            }
            _ => CaptureReadinessDto::ready(
                manifest.session_id.clone(),
                "captureReady",
                latest_capture,
            ),
        },
        "phone-required" | "blocked" => {
            CaptureReadinessDto::phone_required(manifest.session_id.clone())
        }
        "preview-waiting" => {
            CaptureReadinessDto::preview_waiting(manifest.session_id.clone(), latest_capture)
        }
        "export-waiting" => {
            CaptureReadinessDto::export_waiting(manifest.session_id.clone(), latest_capture)
        }
        "completed" => CaptureReadinessDto::completed(manifest.session_id.clone(), latest_capture),
        "warning" => CaptureReadinessDto::warning(manifest.session_id.clone(), latest_capture),
        "helper-preparing" => CaptureReadinessDto::helper_preparing(manifest.session_id.clone()),
        "camera-preparing" | "preparing" => {
            CaptureReadinessDto::camera_preparing(manifest.session_id.clone())
        }
        _ => CaptureReadinessDto::camera_preparing(manifest.session_id.clone()),
    }
}

fn has_valid_active_preset(base_dir: &Path, active_preset: Option<&ActivePresetBinding>) -> bool {
    let Some(active_preset) = active_preset else {
        return false;
    };

    let catalog_root = resolve_published_preset_catalog_dir(base_dir);

    find_published_preset_summary(
        &catalog_root,
        &active_preset.preset_id,
        &active_preset.published_version,
    )
    .is_some()
}

fn read_session_manifest(
    base_dir: &Path,
    session_id: &str,
) -> Result<SessionManifest, HostErrorEnvelope> {
    let manifest_path = SessionPaths::try_new(base_dir, session_id)?.manifest_path;

    if !manifest_path.is_file() {
        return Err(HostErrorEnvelope::session_not_found(
            "진행 중인 세션을 찾지 못했어요. 처음 화면에서 다시 시작해 주세요.",
        ));
    }

    let manifest_bytes = fs::read_to_string(manifest_path).map_err(|error| {
        HostErrorEnvelope::persistence(format!("세션 매니페스트를 읽지 못했어요: {error}"))
    })?;

    serde_json::from_str(&manifest_bytes).map_err(|error| {
        HostErrorEnvelope::persistence(format!("세션 매니페스트를 읽지 못했어요: {error}"))
    })
}
