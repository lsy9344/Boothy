use std::{fs, path::Path};

use crate::{
    capture::{ingest_pipeline::persist_capture_in_dir, CAPTURE_PIPELINE_LOCK},
    contracts::dto::{
        CaptureDeleteInputDto, CaptureDeleteResultDto, CaptureReadinessDto,
        CaptureReadinessInputDto, CaptureRequestInputDto, CaptureRequestResultDto,
        HostErrorEnvelope,
    },
    preset::preset_catalog::{find_published_preset_summary, resolve_published_preset_catalog_dir},
    session::{
        session_manifest::{ActivePresetBinding, SessionManifest},
        session_paths::SessionPaths,
        session_repository::write_session_manifest,
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

pub fn delete_capture_in_dir(
    base_dir: &Path,
    input: CaptureDeleteInputDto,
) -> Result<CaptureDeleteResultDto, HostErrorEnvelope> {
    let paths = SessionPaths::try_new(base_dir, &input.session_id)?;
    let _pipeline_guard = CAPTURE_PIPELINE_LOCK.lock().map_err(|_| {
        HostErrorEnvelope::persistence("촬영 상태를 잠그지 못했어요. 잠시 후 다시 시도해 주세요.")
    })?;
    let mut manifest = read_session_manifest(base_dir, &input.session_id)?;
    let capture_index = manifest
        .captures
        .iter()
        .position(|capture| capture.capture_id == input.capture_id)
        .ok_or_else(|| {
            HostErrorEnvelope::capture_delete_blocked(
                "이 사진은 지금 정리할 수 없어요. 잠시 후 다시 확인해 주세요.",
                normalize_capture_readiness(base_dir, &manifest),
            )
        })?;
    let capture = manifest.captures[capture_index].clone();

    if capture.session_id != input.session_id
        || !matches!(
            capture.render_status.as_str(),
            "previewReady" | "finalReady"
        )
        || capture.post_end_state == "completed"
    {
        return Err(HostErrorEnvelope::capture_delete_blocked(
            "이 사진은 지금 정리할 수 없어요. 잠시 후 다시 확인해 주세요.",
            normalize_capture_readiness(base_dir, &manifest),
        ));
    }

    let staged_assets = stage_capture_asset_deletions(&paths, &capture)?;

    manifest.captures.remove(capture_index);
    manifest.lifecycle.stage = derive_capture_lifecycle_stage(&manifest);
    manifest.updated_at =
        crate::session::session_manifest::current_timestamp(std::time::SystemTime::now())?;
    if let Err(error) = write_session_manifest(&paths.manifest_path, &manifest) {
        rollback_staged_asset_deletions(&staged_assets);
        return Err(error);
    }
    finalize_staged_asset_deletions(&staged_assets);

    Ok(CaptureDeleteResultDto {
        schema_version: "capture-delete-result/v1".into(),
        session_id: input.session_id,
        capture_id: input.capture_id,
        status: "capture-deleted".into(),
        readiness: normalize_capture_readiness(base_dir, &manifest),
        manifest,
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

fn derive_capture_lifecycle_stage(manifest: &SessionManifest) -> String {
    match manifest.captures.last() {
        Some(capture)
            if capture.render_status == "previewWaiting"
                || capture.render_status == "captureSaved" =>
        {
            "preview-waiting".into()
        }
        Some(capture) if capture.render_status == "renderFailed" => "phone-required".into(),
        Some(_) => "capture-ready".into(),
        None if manifest.active_preset.is_some() => "capture-ready".into(),
        None => "session-started".into(),
    }
}

#[derive(Debug, Clone)]
struct StagedAssetDeletion {
    original_path: String,
    staged_path: String,
}

fn stage_capture_asset_deletions(
    paths: &SessionPaths,
    capture: &crate::session::session_manifest::SessionCaptureRecord,
) -> Result<Vec<StagedAssetDeletion>, HostErrorEnvelope> {
    let mut staged_assets = Vec::new();

    stage_session_scoped_asset_if_present(
        paths,
        &capture.raw.asset_path,
        &capture.capture_id,
        "raw",
        &mut staged_assets,
    )?;

    if let Some(preview_path) = capture.preview.asset_path.as_deref() {
        stage_session_scoped_asset_if_present(
            paths,
            preview_path,
            &capture.capture_id,
            "preview",
            &mut staged_assets,
        )?;
    }

    if let Some(final_path) = capture.final_asset.asset_path.as_deref() {
        stage_session_scoped_asset_if_present(
            paths,
            final_path,
            &capture.capture_id,
            "final",
            &mut staged_assets,
        )?;
    }

    Ok(staged_assets)
}

fn stage_session_scoped_asset_if_present(
    paths: &SessionPaths,
    asset_path: &str,
    capture_id: &str,
    asset_kind: &str,
    staged_assets: &mut Vec<StagedAssetDeletion>,
) -> Result<(), HostErrorEnvelope> {
    if !is_session_scoped_asset_path(paths, asset_path) {
        return Ok(());
    }

    let asset = Path::new(asset_path);

    if !asset.exists() {
        return Ok(());
    }

    let metadata = fs::metadata(asset).map_err(|error| {
        HostErrorEnvelope::persistence(format!("세션 파일을 정리하지 못했어요: {error}"))
    })?;

    if !metadata.is_file() {
        return Err(HostErrorEnvelope::persistence(
            "세션 파일을 정리하지 못했어요. 잠시 후 다시 시도해 주세요.",
        ));
    }

    let staged_path = format!("{asset_path}.delete-{capture_id}-{asset_kind}");

    if Path::new(&staged_path).exists() {
        fs::remove_file(&staged_path).map_err(|error| {
            HostErrorEnvelope::persistence(format!("세션 파일을 정리하지 못했어요: {error}"))
        })?;
    }

    fs::rename(asset_path, &staged_path).map_err(|error| {
        HostErrorEnvelope::persistence(format!("세션 파일을 정리하지 못했어요: {error}"))
    })?;

    staged_assets.push(StagedAssetDeletion {
        original_path: asset_path.into(),
        staged_path,
    });

    Ok(())
}

fn rollback_staged_asset_deletions(staged_assets: &[StagedAssetDeletion]) {
    for staged_asset in staged_assets.iter().rev() {
        if Path::new(&staged_asset.staged_path).exists() {
            let _ = fs::rename(&staged_asset.staged_path, &staged_asset.original_path);
        }
    }
}

fn finalize_staged_asset_deletions(staged_assets: &[StagedAssetDeletion]) {
    for staged_asset in staged_assets {
        if Path::new(&staged_asset.staged_path).exists() {
            let _ = fs::remove_file(&staged_asset.staged_path);
        }
    }
}

fn is_session_scoped_asset_path(paths: &SessionPaths, asset_path: &str) -> bool {
    let normalized_asset_path = asset_path.replace('\\', "/").to_lowercase();
    let normalized_session_root = format!(
        "{}/",
        paths
            .session_root
            .to_string_lossy()
            .replace('\\', "/")
            .to_lowercase()
    );

    normalized_asset_path.starts_with(&normalized_session_root)
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
