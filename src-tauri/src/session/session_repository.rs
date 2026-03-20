use std::{
  fs,
  path::{Path, PathBuf},
  sync::atomic::{AtomicU64, Ordering},
  time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{
  contracts::dto::{HostErrorEnvelope, SessionStartInputDto},
  session::{
    session_manifest::{build_session_manifest, validate_session_start_input, SessionManifest},
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
  let manifest = build_session_manifest(session_id.clone(), validated_input);

  create_session_root(&paths, &manifest)?;

  Ok(SessionStartResultDto {
    session_id,
    booth_alias: manifest.booth_alias.clone(),
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
      HostErrorEnvelope::persistence(format!("세션 매니페스트를 직렬화하지 못했어요: {error}"))
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
