use std::{
    cmp::Reverse,
    fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    sync::Arc,
    thread,
    thread::JoinHandle,
    time::{Duration, SystemTime},
};

use serde::{Deserialize, Serialize};

use crate::contracts::dto::{
    validate_operator_audit_query_filter, CapabilitySnapshotDto, HostErrorEnvelope,
    OperatorAuditEntryDto, OperatorAuditLatestOutcomeDto, OperatorAuditQueryFilterDto,
    OperatorAuditQueryResultDto, OperatorAuditQuerySummaryDto,
};

const OPERATOR_AUDIT_ENTRY_SCHEMA_VERSION: &str = "operator-audit-entry/v1";
const OPERATOR_AUDIT_QUERY_RESULT_SCHEMA_VERSION: &str = "operator-audit-query-result/v1";
const OPERATOR_AUDIT_STORE_SCHEMA_VERSION: &str = "operator-audit-store/v1";
const OPERATOR_AUDIT_LOCK_RETRY_DELAY_MS: u64 = 10;
const OPERATOR_AUDIT_LOCK_MAX_ATTEMPTS: u32 = 500;
const OPERATOR_AUDIT_LOCK_HEARTBEAT_MS: u64 = 250;
const OPERATOR_AUDIT_LOCK_STALE_AFTER_MS: u64 = 1_500;

static OPERATOR_AUDIT_EVENT_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct OperatorAuditRecordInput {
    pub occurred_at: String,
    pub session_id: Option<String>,
    pub event_category: &'static str,
    pub event_type: &'static str,
    pub summary: String,
    pub detail: String,
    pub actor_id: Option<String>,
    pub source: &'static str,
    pub capture_id: Option<String>,
    pub preset_id: Option<String>,
    pub published_version: Option<String>,
    pub reason_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OperatorAuditStore {
    schema_version: String,
    entries: Vec<OperatorAuditEntryDto>,
}

struct OperatorAuditStoreLock {
    lock_path: PathBuf,
    heartbeat_stop: Arc<std::sync::atomic::AtomicBool>,
    heartbeat_handle: Option<JoinHandle<()>>,
}

impl Drop for OperatorAuditStoreLock {
    fn drop(&mut self) {
        self.heartbeat_stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.heartbeat_handle.take() {
            let _ = handle.join();
        }
        if self.lock_path.exists() {
            let _ = fs::remove_file(&self.lock_path);
        }
    }
}

pub fn load_operator_audit_history_in_dir(
    base_dir: &Path,
    capability_snapshot: &CapabilitySnapshotDto,
    input: OperatorAuditQueryFilterDto,
) -> Result<OperatorAuditQueryResultDto, HostErrorEnvelope> {
    super::ensure_operator_access(capability_snapshot)?;
    validate_operator_audit_query_filter(&input)?;
    wait_for_audit_store_idle(base_dir)?;

    let normalized_filter = OperatorAuditQueryFilterDto {
        session_id: input.session_id.clone(),
        event_categories: input.event_categories.clone(),
        limit: Some(input.limit.unwrap_or(20)),
    };
    let limit = normalized_filter.limit.unwrap_or(20) as usize;
    let store = read_audit_store(base_dir)?;
    let mut events = store
        .entries
        .into_iter()
        .filter(|entry| match normalized_filter.session_id.as_deref() {
            Some(session_id) => entry.session_id.as_deref() == Some(session_id),
            None => true,
        })
        .filter(|entry| {
            normalized_filter.event_categories.is_empty()
                || normalized_filter
                    .event_categories
                    .iter()
                    .any(|category| category == &entry.event_category)
        })
        .collect::<Vec<_>>();

    events.sort_by_key(|entry| Reverse((entry.occurred_at.clone(), entry.event_id.clone())));
    events.truncate(limit);

    Ok(OperatorAuditQueryResultDto {
        schema_version: OPERATOR_AUDIT_QUERY_RESULT_SCHEMA_VERSION.into(),
        filter: normalized_filter,
        summary: build_summary(&events),
        events,
    })
}

pub fn append_operator_audit_record(
    base_dir: &Path,
    input: OperatorAuditRecordInput,
) -> Result<(), HostErrorEnvelope> {
    let _lock = acquire_audit_store_lock(base_dir)?;
    let mut store = read_audit_store(base_dir)?;
    store.entries.push(OperatorAuditEntryDto {
        schema_version: OPERATOR_AUDIT_ENTRY_SCHEMA_VERSION.into(),
        event_id: build_event_id(
            input.occurred_at.as_str(),
            input.event_type,
            input.session_id.as_deref(),
        ),
        occurred_at: input.occurred_at,
        session_id: input.session_id,
        event_category: input.event_category.into(),
        event_type: input.event_type.into(),
        summary: input.summary,
        detail: input.detail,
        actor_id: input.actor_id,
        source: input.source.into(),
        capture_id: input.capture_id,
        preset_id: input.preset_id,
        published_version: input.published_version,
        reason_code: input.reason_code,
    });
    persist_audit_store(base_dir, &store)
}

pub fn try_append_operator_audit_record(base_dir: &Path, input: OperatorAuditRecordInput) {
    let _ = append_operator_audit_record(base_dir, input);
}

fn build_summary(events: &[OperatorAuditEntryDto]) -> OperatorAuditQuerySummaryDto {
    let session_lifecycle_events = count_by_category(events, "session-lifecycle");
    let timing_transition_events = count_by_category(events, "timing-transition");
    let post_end_outcome_events = count_by_category(events, "post-end-outcome");
    let operator_intervention_events = count_by_category(events, "operator-intervention");
    let publication_recovery_events = count_by_category(events, "publication-recovery");
    let release_governance_events = count_by_category(events, "release-governance");
    let critical_failure_events = count_by_category(events, "critical-failure");

    OperatorAuditQuerySummaryDto {
        total_events: events.len() as u32,
        session_lifecycle_events,
        timing_transition_events,
        post_end_outcome_events,
        operator_intervention_events,
        publication_recovery_events,
        release_governance_events,
        critical_failure_events,
        latest_outcome: events.first().map(|event| OperatorAuditLatestOutcomeDto {
            occurred_at: event.occurred_at.clone(),
            event_category: event.event_category.clone(),
            event_type: event.event_type.clone(),
            summary: event.summary.clone(),
        }),
    }
}

fn count_by_category(events: &[OperatorAuditEntryDto], category: &str) -> u32 {
    events
        .iter()
        .filter(|event| event.event_category == category)
        .count() as u32
}

fn resolve_audit_store_path(base_dir: &Path) -> PathBuf {
    base_dir.join("diagnostics").join("operator-audit-log.json")
}

fn resolve_audit_store_lock_path(base_dir: &Path) -> PathBuf {
    base_dir.join("diagnostics").join("operator-audit-log.lock")
}

fn read_audit_store(base_dir: &Path) -> Result<OperatorAuditStore, HostErrorEnvelope> {
    let store_path = resolve_audit_store_path(base_dir);
    let backup_path = store_path.with_extension("json.bak");

    if !store_path.exists() {
        if backup_path.is_file() {
            return read_audit_store_from_path(&backup_path);
        }

        return Ok(OperatorAuditStore {
            schema_version: OPERATOR_AUDIT_STORE_SCHEMA_VERSION.into(),
            entries: Vec::new(),
        });
    }

    match read_audit_store_from_path(&store_path) {
        Ok(store) => Ok(store),
        Err(error) if backup_path.is_file() => {
            read_audit_store_from_path(&backup_path).or(Err(error))
        }
        Err(error) => Err(error),
    }
}

fn read_audit_store_from_path(path: &Path) -> Result<OperatorAuditStore, HostErrorEnvelope> {
    let bytes = fs::read_to_string(path).map_err(|error| {
        HostErrorEnvelope::persistence(format!("operator audit store를 읽지 못했어요: {error}"))
    })?;

    serde_json::from_str(&bytes).map_err(|error| {
        HostErrorEnvelope::persistence(format!("operator audit store를 읽지 못했어요: {error}"))
    })
}

fn persist_audit_store(
    base_dir: &Path,
    store: &OperatorAuditStore,
) -> Result<(), HostErrorEnvelope> {
    let store_path = resolve_audit_store_path(base_dir);
    let store_dir = store_path.parent().ok_or_else(|| {
        HostErrorEnvelope::persistence("operator audit 저장 경로를 준비하지 못했어요.")
    })?;
    fs::create_dir_all(store_dir).map_err(|error| {
        HostErrorEnvelope::persistence(format!(
            "operator audit 저장 경로를 준비하지 못했어요: {error}"
        ))
    })?;
    let bytes = serde_json::to_vec_pretty(store).map_err(|error| {
        HostErrorEnvelope::persistence(format!(
            "operator audit store를 직렬화하지 못했어요: {error}"
        ))
    })?;

    write_json_bytes_atomically(&store_path, &bytes)
}

fn write_json_bytes_atomically(path: &Path, bytes: &[u8]) -> Result<(), HostErrorEnvelope> {
    let temp_path = path.with_extension("json.tmp");
    let backup_path = path.with_extension("json.bak");

    if temp_path.exists() {
        fs::remove_file(&temp_path).map_err(map_fs_error)?;
    }

    fs::write(&temp_path, bytes).map_err(map_fs_error)?;

    if backup_path.exists() {
        fs::remove_file(&backup_path).map_err(map_fs_error)?;
    }

    if path.exists() {
        fs::rename(path, &backup_path).map_err(|error| {
            let _ = fs::remove_file(&temp_path);
            map_fs_error(error)
        })?;
    }

    if let Err(error) = fs::rename(&temp_path, path) {
        if backup_path.exists() {
            let _ = fs::rename(&backup_path, path);
        }
        let _ = fs::remove_file(&temp_path);

        return Err(map_fs_error(error));
    }

    if backup_path.exists() {
        fs::remove_file(&backup_path).map_err(map_fs_error)?;
    }

    Ok(())
}

fn build_event_id(occurred_at: &str, event_type: &str, session_id: Option<&str>) -> String {
    let safe_timestamp = sanitize_id_segment(occurred_at, 16);
    let safe_event_type = sanitize_id_segment(event_type, 20);
    let safe_session = session_id
        .map(|value| sanitize_id_suffix(value, 6))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "global".into());
    let counter = OPERATOR_AUDIT_EVENT_COUNTER.fetch_add(1, Ordering::Relaxed) & 0xffff_ffff;

    format!("audit-{safe_timestamp}-{counter:08x}-{safe_event_type}-{safe_session}")
}

fn sanitize_id_segment(value: &str, max_len: usize) -> String {
    let filtered = value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>();

    if filtered.len() <= max_len {
        return filtered;
    }

    filtered[..max_len].to_string()
}

fn sanitize_id_suffix(value: &str, max_len: usize) -> String {
    let filtered = value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>();

    if filtered.len() <= max_len {
        return filtered;
    }

    filtered[filtered.len() - max_len..].to_string()
}

fn acquire_audit_store_lock(base_dir: &Path) -> Result<OperatorAuditStoreLock, HostErrorEnvelope> {
    let lock_path = resolve_audit_store_lock_path(base_dir);
    let lock_dir = lock_path.parent().ok_or_else(|| {
        HostErrorEnvelope::persistence("operator audit 저장 경로를 준비하지 못했어요.")
    })?;
    fs::create_dir_all(lock_dir).map_err(|error| {
        HostErrorEnvelope::persistence(format!(
            "operator audit 저장 경로를 준비하지 못했어요: {error}"
        ))
    })?;

    for _ in 0..OPERATOR_AUDIT_LOCK_MAX_ATTEMPTS {
        match OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&lock_path)
        {
            Ok(mut lock_file) => {
                let _ = writeln!(lock_file, "pid={}", std::process::id());
                let heartbeat_stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
                let heartbeat_handle = Some(spawn_audit_store_lock_heartbeat(
                    lock_path.clone(),
                    heartbeat_stop.clone(),
                ));
                return Ok(OperatorAuditStoreLock {
                    lock_path,
                    heartbeat_stop,
                    heartbeat_handle,
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                try_clear_stale_audit_store_lock(&lock_path)?;
                thread::sleep(Duration::from_millis(OPERATOR_AUDIT_LOCK_RETRY_DELAY_MS));
            }
            Err(error) => {
                return Err(HostErrorEnvelope::persistence(format!(
                    "operator audit 저장 경로를 준비하지 못했어요: {error}"
                )));
            }
        }
    }

    Err(HostErrorEnvelope::persistence(
        "operator audit 저장 잠금을 기다리는 중 시간이 초과되었어요.",
    ))
}

fn spawn_audit_store_lock_heartbeat(
    lock_path: PathBuf,
    heartbeat_stop: Arc<std::sync::atomic::AtomicBool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        while !heartbeat_stop.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(OPERATOR_AUDIT_LOCK_HEARTBEAT_MS));

            if heartbeat_stop.load(Ordering::Relaxed) {
                break;
            }

            if lock_path.exists() {
                let _ = fs::write(&lock_path, format!("pid={}", std::process::id()));
            }
        }
    })
}

fn wait_for_audit_store_idle(base_dir: &Path) -> Result<(), HostErrorEnvelope> {
    let lock_path = resolve_audit_store_lock_path(base_dir);

    for _ in 0..OPERATOR_AUDIT_LOCK_MAX_ATTEMPTS {
        if !lock_path.exists() {
            return Ok(());
        }

        try_clear_stale_audit_store_lock(&lock_path)?;

        if !lock_path.exists() {
            return Ok(());
        }

        thread::sleep(Duration::from_millis(OPERATOR_AUDIT_LOCK_RETRY_DELAY_MS));
    }

    Err(HostErrorEnvelope::persistence(
        "operator audit 저장이 끝나기를 기다리는 중 시간이 초과되었어요.",
    ))
}

fn try_clear_stale_audit_store_lock(lock_path: &Path) -> Result<(), HostErrorEnvelope> {
    if !lock_path.exists() || !is_stale_audit_store_lock(lock_path)? {
        return Ok(());
    }

    fs::remove_file(lock_path).map_err(|error| {
        HostErrorEnvelope::persistence(format!(
            "stale operator audit 잠금을 정리하지 못했어요: {error}"
        ))
    })
}

fn is_stale_audit_store_lock(lock_path: &Path) -> Result<bool, HostErrorEnvelope> {
    let metadata = match fs::metadata(lock_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => return Ok(false),
        Err(error) => {
            return Err(HostErrorEnvelope::persistence(format!(
                "operator audit 잠금을 확인하지 못했어요: {error}"
            )))
        }
    };
    let modified_at = match metadata.modified() {
        Ok(modified_at) => modified_at,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => return Ok(false),
        Err(error) => {
            return Err(HostErrorEnvelope::persistence(format!(
                "operator audit 잠금을 확인하지 못했어요: {error}"
            )))
        }
    };

    Ok(SystemTime::now()
        .duration_since(modified_at)
        .unwrap_or_default()
        >= Duration::from_millis(OPERATOR_AUDIT_LOCK_STALE_AFTER_MS))
}

fn map_fs_error(error: std::io::Error) -> HostErrorEnvelope {
    HostErrorEnvelope::persistence(format!("operator audit store를 저장하지 못했어요: {error}"))
}
