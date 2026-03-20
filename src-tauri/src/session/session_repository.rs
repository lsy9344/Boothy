use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{
    contracts::dto::{
        validate_session_id, HostErrorEnvelope, PresetSelectionInputDto, PresetSelectionResultDto,
        SessionStartInputDto,
    },
    preset::preset_catalog::{
        find_selectable_published_preset_summary, resolve_published_preset_catalog_dir,
    },
    session::{
        session_manifest::{
            build_session_manifest, current_timestamp, validate_session_start_input,
            ActivePresetBinding, SessionManifest,
        },
        session_paths::SessionPaths,
    },
};

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStartResultDto {
    pub session_id: String,
    pub booth_alias: String,
    pub manifest: SessionManifest,
}

pub fn start_session_in_dir(
    base_dir: &Path,
    input: SessionStartInputDto,
) -> Result<SessionStartResultDto, HostErrorEnvelope> {
    let validated_input = validate_session_start_input(&input)?;
    let session_id = generate_session_id();
    let paths = SessionPaths::new(base_dir, &session_id);
    let manifest = build_session_manifest(session_id.clone(), validated_input)?;

    create_session_root(&paths, &manifest)?;

    Ok(SessionStartResultDto {
        session_id,
        booth_alias: manifest.booth_alias.clone(),
        manifest,
    })
}

pub fn select_active_preset_in_dir(
    base_dir: &Path,
    input: PresetSelectionInputDto,
) -> Result<PresetSelectionResultDto, HostErrorEnvelope> {
    validate_session_id(&input.session_id)?;
    let paths = SessionPaths::try_new(base_dir, &input.session_id)?;
    let mut manifest = read_session_manifest(&paths.manifest_path)?;
    let catalog_root = resolve_published_preset_catalog_dir(base_dir);
    let selected_preset = find_selectable_published_preset_summary(
        &catalog_root,
        &input.preset_id,
        &input.published_version,
    )
    .map_err(|error| match error.code.as_str() {
        "validation-error" => HostErrorEnvelope::preset_not_available(
            "지금 고른 프리셋은 사용할 수 없어요. 다른 프리셋을 골라 주세요.",
        ),
        _ => error,
    })?
    .ok_or_else(|| {
        HostErrorEnvelope::preset_not_available(
            "지금 고른 프리셋은 사용할 수 없어요. 다른 프리셋을 골라 주세요.",
        )
    })?;

    let active_preset = ActivePresetBinding {
        preset_id: selected_preset.preset_id.clone(),
        published_version: selected_preset.published_version.clone(),
    };

    if manifest.active_preset.as_ref() == Some(&active_preset) {
        return Ok(PresetSelectionResultDto {
            session_id: manifest.session_id.clone(),
            active_preset,
            manifest,
        });
    }

    manifest.active_preset = Some(active_preset);
    manifest.active_preset_id = Some(selected_preset.preset_id.clone());
    manifest.updated_at = current_timestamp(SystemTime::now())?;
    if manifest.lifecycle.stage == "session-started" {
        manifest.lifecycle.stage = "preset-selected".into();
    }

    write_session_manifest(&paths.manifest_path, &manifest)?;

    Ok(PresetSelectionResultDto {
        session_id: manifest.session_id.clone(),
        active_preset: manifest
            .active_preset
            .clone()
            .expect("active preset just set"),
        manifest,
    })
}

fn create_session_root(
    paths: &SessionPaths,
    manifest: &SessionManifest,
) -> Result<(), HostErrorEnvelope> {
    fs::create_dir_all(&paths.sessions_root).map_err(|error| {
        HostErrorEnvelope::persistence(format!("세션 저장 위치를 준비하지 못했어요: {error}"))
    })?;

    if paths.session_root.exists() || paths.temp_root.exists() {
        return Err(HostErrorEnvelope::persistence(
            "같은 세션 식별자가 이미 존재해요. 다시 시도해 주세요.",
        ));
    }

    let creation_result = (|| -> Result<(), HostErrorEnvelope> {
        fs::create_dir_all(paths.temp_captures_originals_dir()).map_err(map_fs_error)?;
        fs::create_dir_all(paths.temp_renders_previews_dir()).map_err(map_fs_error)?;
        fs::create_dir_all(paths.temp_renders_finals_dir()).map_err(map_fs_error)?;
        fs::create_dir_all(paths.temp_handoff_dir()).map_err(map_fs_error)?;
        fs::create_dir_all(paths.temp_diagnostics_dir()).map_err(map_fs_error)?;

        let manifest_bytes = serde_json::to_vec_pretty(manifest).map_err(|error| {
            HostErrorEnvelope::persistence(format!(
                "세션 매니페스트를 직렬화하지 못했어요: {error}"
            ))
        })?;

        fs::write(paths.temp_manifest_path(), manifest_bytes).map_err(map_fs_error)?;
        fs::rename(&paths.temp_root, &paths.session_root).map_err(map_fs_error)?;

        Ok(())
    })();

    if creation_result.is_err() && paths.temp_root.exists() {
        let _ = fs::remove_dir_all(&paths.temp_root);
    }

    creation_result
}

pub(crate) fn read_session_manifest(
    manifest_path: &Path,
) -> Result<SessionManifest, HostErrorEnvelope> {
    let backup_path = manifest_backup_path(manifest_path);

    if !manifest_path.is_file() {
        if backup_path.is_file() {
            fs::rename(&backup_path, manifest_path).map_err(map_fs_error)?;
        } else {
            return Err(HostErrorEnvelope::session_not_found(
                "진행 중인 세션을 찾지 못했어요. 처음 화면에서 다시 시작해 주세요.",
            ));
        }
    }

    if !manifest_path.is_file() {
        return Err(HostErrorEnvelope::session_not_found(
            "진행 중인 세션을 찾지 못했어요. 처음 화면에서 다시 시작해 주세요.",
        ));
    }

    let manifest_bytes = fs::read_to_string(manifest_path).map_err(map_fs_error)?;

    serde_json::from_str(&manifest_bytes).map_err(|error| {
        HostErrorEnvelope::persistence(format!("세션 매니페스트를 읽지 못했어요: {error}"))
    })
}

pub(crate) fn write_session_manifest(
    manifest_path: &Path,
    manifest: &SessionManifest,
) -> Result<(), HostErrorEnvelope> {
    let manifest_bytes = serde_json::to_vec_pretty(manifest).map_err(|error| {
        HostErrorEnvelope::persistence(format!("세션 매니페스트를 직렬화하지 못했어요: {error}"))
    })?;
    let temp_path = manifest_temp_path(manifest_path);
    let backup_path = manifest_backup_path(manifest_path);

    if temp_path.exists() {
        fs::remove_file(&temp_path).map_err(map_fs_error)?;
    }

    fs::write(&temp_path, manifest_bytes).map_err(map_fs_error)?;

    if backup_path.exists() {
        fs::remove_file(&backup_path).map_err(map_fs_error)?;
    }

    if manifest_path.exists() {
        fs::rename(manifest_path, &backup_path).map_err(|error| {
            let _ = fs::remove_file(&temp_path);
            map_fs_error(error)
        })?;
    }

    if let Err(error) = fs::rename(&temp_path, manifest_path) {
        if backup_path.exists() {
            let _ = fs::rename(&backup_path, manifest_path);
        }
        let _ = fs::remove_file(&temp_path);

        return Err(map_fs_error(error));
    }

    if backup_path.exists() {
        fs::remove_file(&backup_path).map_err(map_fs_error)?;
    }

    Ok(())
}

fn manifest_temp_path(manifest_path: &Path) -> PathBuf {
    manifest_path.with_extension("json.tmp")
}

fn manifest_backup_path(manifest_path: &Path) -> PathBuf {
    manifest_path.with_extension("json.bak")
}

fn generate_session_id() -> String {
    let unix_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let counter = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed) as u128;
    let value = unix_nanos ^ (counter << 16);

    format!("session_{value:026x}")
}

fn map_fs_error(error: std::io::Error) -> HostErrorEnvelope {
    HostErrorEnvelope::persistence(format!("세션 파일을 만들지 못했어요: {error}"))
}

pub fn resolve_app_session_base_dir(app_local_data_dir: PathBuf) -> PathBuf {
    app_local_data_dir.join("booth-runtime")
}
