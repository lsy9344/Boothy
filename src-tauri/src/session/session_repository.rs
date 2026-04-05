use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    sync::{Mutex, OnceLock},
    thread,
    time::Duration,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{
    capture::normalized_state::normalize_capture_readiness,
    contracts::dto::{
        validate_session_id, HostErrorEnvelope, PresetSelectionInputDto, PresetSelectionResultDto,
        SessionStartInputDto,
    },
    diagnostics::audit_log::{try_append_operator_audit_record, OperatorAuditRecordInput},
    handoff::sync_post_end_state_in_dir,
    preset::{
        preset_catalog::{
            find_selectable_published_preset_summary, resolve_published_preset_catalog_dir,
        },
        preset_catalog_state::capture_live_catalog_snapshot,
    },
    render::initialize_session_locked_preview_render_route_policy_in_dir,
    session::{
        session_manifest::{
            build_session_manifest, current_timestamp, normalize_legacy_manifest,
            validate_session_start_input, ActivePresetBinding, SessionManifest,
        },
        session_paths::SessionPaths,
    },
    timing::sync_session_timing_in_dir,
};

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);
const MANIFEST_WRITE_RETRY_ATTEMPTS: usize = 12;
const MANIFEST_WRITE_RETRY_DELAY_MS: u64 = 25;

#[derive(Debug, Default)]
struct ManifestWriteFailureInjection {
    remaining_failures_by_path: HashMap<PathBuf, usize>,
}

static MANIFEST_WRITE_FAILURE_INJECTION: OnceLock<Mutex<ManifestWriteFailureInjection>> =
    OnceLock::new();

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
    if let Err(error) =
        initialize_session_locked_preview_render_route_policy_in_dir(base_dir, &session_id)
    {
        log::warn!(
            "preview_route_policy_lock_init_failed session={} code={} detail={}",
            session_id,
            error.code,
            error.message
        );
    }
    try_append_operator_audit_record(
        base_dir,
        OperatorAuditRecordInput {
            occurred_at: manifest.created_at.clone(),
            session_id: Some(session_id.clone()),
            event_category: "session-lifecycle",
            event_type: "session-started",
            summary: "새 세션을 시작했어요.".into(),
            detail: "운영자 review에 사용할 현재 세션 식별자와 기본 문맥이 생성되었어요.".into(),
            actor_id: None,
            source: "session-repository",
            capture_id: None,
            preset_id: None,
            published_version: None,
            reason_code: None,
        },
    );

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
    manifest =
        sync_session_timing_in_dir(base_dir, &paths.manifest_path, manifest, SystemTime::now())?;
    manifest =
        sync_post_end_state_in_dir(base_dir, &paths.manifest_path, manifest, SystemTime::now())?;
    if manifest.catalog_revision.is_none() || manifest.catalog_snapshot.is_none() {
        let (catalog_revision, catalog_snapshot) = capture_live_catalog_snapshot(base_dir)?;
        manifest.catalog_revision = Some(catalog_revision);
        manifest.catalog_snapshot = Some(catalog_snapshot);
        write_session_manifest(&paths.manifest_path, &manifest)?;
    }

    if manifest.post_end.is_some()
        || manifest.timing.as_ref().map(|timing| timing.phase.as_str()) == Some("ended")
    {
        return Err(HostErrorEnvelope::capture_not_ready(
            "지금은 룩을 바꿀 수 없어요.",
            normalize_capture_readiness(base_dir, &manifest),
        ));
    }

    let catalog_root = resolve_published_preset_catalog_dir(base_dir);
    let selected_preset = find_selectable_published_preset_summary(
        &catalog_root,
        &input.preset_id,
        &input.published_version,
        manifest.catalog_snapshot.as_deref().unwrap_or(&[]),
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
        let mut changed = false;

        if manifest.active_preset_id.as_deref() != Some(selected_preset.preset_id.as_str()) {
            manifest.active_preset_id = Some(selected_preset.preset_id.clone());
            changed = true;
        }

        if manifest.active_preset_display_name.as_deref()
            != Some(selected_preset.display_name.as_str())
        {
            manifest.active_preset_display_name = Some(selected_preset.display_name.clone());
            changed = true;
        }

        if changed {
            write_session_manifest(&paths.manifest_path, &manifest)?;
        }

        return Ok(PresetSelectionResultDto {
            session_id: manifest.session_id.clone(),
            active_preset,
            manifest,
        });
    }

    manifest.active_preset = Some(active_preset);
    manifest.active_preset_id = Some(selected_preset.preset_id.clone());
    manifest.active_preset_display_name = Some(selected_preset.display_name.clone());
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
    let mut manifest: SessionManifest = serde_json::from_str(&manifest_bytes).map_err(|error| {
        HostErrorEnvelope::persistence(format!("세션 매니페스트를 읽지 못했어요: {error}"))
    })?;

    normalize_legacy_manifest(&mut manifest);

    Ok(manifest)
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

    let mut last_error: Option<std::io::Error> = None;

    for attempt in 0..MANIFEST_WRITE_RETRY_ATTEMPTS {
        match write_session_manifest_once(manifest_path, &temp_path, &backup_path, &manifest_bytes)
        {
            Ok(()) => {
                if backup_path.exists() {
                    if let Err(error) = fs::remove_file(&backup_path) {
                        if !is_retryable_manifest_write_error(&error) {
                            return Err(map_fs_error(error));
                        }
                    }
                }

                return Ok(());
            }
            Err(error) if is_retryable_manifest_write_error(&error) => {
                log::warn!(
                    "manifest_write_retry path={} attempt={} reason={}",
                    manifest_path.display(),
                    attempt + 1,
                    error
                );
                last_error = Some(error);
                restore_manifest_from_backup_if_needed(manifest_path, &backup_path);
                remove_temp_manifest_if_present(&temp_path);

                if attempt + 1 < MANIFEST_WRITE_RETRY_ATTEMPTS {
                    thread::sleep(Duration::from_millis(MANIFEST_WRITE_RETRY_DELAY_MS));
                }
            }
            Err(error) => {
                restore_manifest_from_backup_if_needed(manifest_path, &backup_path);
                remove_temp_manifest_if_present(&temp_path);
                return Err(map_fs_error(error));
            }
        }
    }

    restore_manifest_from_backup_if_needed(manifest_path, &backup_path);
    remove_temp_manifest_if_present(&temp_path);

    Err(map_fs_error(last_error.unwrap_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "manifest write retry budget exhausted",
        )
    })))
}

fn write_session_manifest_once(
    manifest_path: &Path,
    temp_path: &Path,
    backup_path: &Path,
    manifest_bytes: &[u8],
) -> Result<(), std::io::Error> {
    if let Some(error) = take_injected_manifest_write_failure(manifest_path) {
        return Err(error);
    }

    if temp_path.exists() {
        fs::remove_file(temp_path)?;
    }

    fs::write(temp_path, manifest_bytes)?;

    if backup_path.exists() {
        fs::remove_file(backup_path)?;
    }

    if manifest_path.exists() {
        fs::rename(manifest_path, backup_path)?;
    }

    if let Err(error) = fs::rename(temp_path, manifest_path) {
        if backup_path.exists() {
            let _ = fs::rename(backup_path, manifest_path);
        }
        let _ = fs::remove_file(temp_path);

        return Err(error);
    }

    Ok(())
}

fn is_retryable_manifest_write_error(error: &std::io::Error) -> bool {
    matches!(error.kind(), std::io::ErrorKind::PermissionDenied)
        || matches!(error.raw_os_error(), Some(32 | 33 | 5))
}

fn restore_manifest_from_backup_if_needed(manifest_path: &Path, backup_path: &Path) {
    if !manifest_path.exists() && backup_path.exists() {
        let _ = fs::rename(backup_path, manifest_path);
    }
}

fn remove_temp_manifest_if_present(temp_path: &Path) {
    if temp_path.exists() {
        let _ = fs::remove_file(temp_path);
    }
}

#[doc(hidden)]
pub fn set_manifest_write_retryable_failures_for_tests(manifest_path: &Path, failures: usize) {
    let injection = MANIFEST_WRITE_FAILURE_INJECTION
        .get_or_init(|| Mutex::new(ManifestWriteFailureInjection::default()));
    let mut guard = injection
        .lock()
        .expect("manifest write failure injection lock should be available");

    if failures == 0 {
        guard.remaining_failures_by_path.remove(manifest_path);
        return;
    }

    guard
        .remaining_failures_by_path
        .insert(manifest_path.to_path_buf(), failures);
}

fn take_injected_manifest_write_failure(manifest_path: &Path) -> Option<std::io::Error> {
    let injection = MANIFEST_WRITE_FAILURE_INJECTION
        .get_or_init(|| Mutex::new(ManifestWriteFailureInjection::default()));
    let mut guard = injection
        .lock()
        .expect("manifest write failure injection lock should be available");

    let remaining_failures = guard.remaining_failures_by_path.get_mut(manifest_path)?;

    if *remaining_failures == 0 {
        guard.remaining_failures_by_path.remove(manifest_path);
        return None;
    }

    *remaining_failures -= 1;

    if *remaining_failures == 0 {
        guard.remaining_failures_by_path.remove(manifest_path);
    }

    Some(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "injected manifest write retryable failure",
    ))
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
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .map(|user_profile| user_profile.join("Pictures").join("dabi_shoot"))
        .unwrap_or_else(|| app_local_data_dir.join("dabi_shoot"))
}
