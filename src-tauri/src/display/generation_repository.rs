//! Story 7.2: immutable generation 파일과 display pointer의 durable 경계.
//!
//! commit 순서를 이 모듈이 강제한다.
//! `staging write → flush → sync_all → close → 구조 probe → 크기/correlation 검증
//!  → rename → pointer rename → journal append → (호출자가) notify`
//!
//! 한 단계라도 앞당기면 커밋되지 않은 generation이 밖으로 나간다.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::contracts::dto::{
    is_safe_draft_folder_name, DisplayGenerationDto, DisplayPointerSnapshotDto, HostErrorEnvelope,
    VIEWER_DISPLAY_SCHEMA_VERSION,
};
use crate::session::session_paths::SessionPaths;

use super::image_probe::{content_hash, probe_jpeg, JpegProbe, ProbeError};

pub const POINTER_FILE_NAME: &str = "pointer.json";
pub const JOURNAL_FILE_NAME: &str = "generations.jsonl";
pub const STAGING_DIR_NAME: &str = ".staging";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayJournalRecord {
    pub schema_version: String,
    pub outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reject_reason: Option<String>,
    pub session_id: String,
    pub request_id: String,
    pub generation_seq: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation: Option<DisplayGenerationDto>,
    pub recorded_at_host_micros: u64,
}

#[derive(Debug, Clone)]
pub struct StagedGeneration {
    pub staging_path: PathBuf,
    pub byte_size: u64,
    pub probe: JpegProbe,
    pub source_hash: String,
    pub file_ready_at_micros: u64,
    pub probe_ok_at_micros: u64,
}

#[derive(Debug)]
pub enum StageGenerationError {
    Persistence(HostErrorEnvelope),
    Probe(ProbeError),
}

fn validate_request_path_component(request_id: &str) -> Result<(), HostErrorEnvelope> {
    let is_safe = request_id == request_id.trim()
        && request_id.chars().count() <= 120
        && !request_id.contains(['/', '\\'])
        && is_safe_draft_folder_name(request_id);

    if is_safe {
        Ok(())
    } else {
        Err(HostErrorEnvelope::validation_message(
            "표시 요청 식별자를 다시 확인해 주세요.",
        ))
    }
}

pub fn display_root(base_dir: &Path, session_id: &str) -> PathBuf {
    SessionPaths::new(base_dir, session_id)
        .session_root
        .join("renders")
        .join("display")
}

fn checked_display_root(base_dir: &Path, session_id: &str) -> Result<PathBuf, HostErrorEnvelope> {
    Ok(SessionPaths::try_new(base_dir, session_id)?
        .session_root
        .join("renders")
        .join("display"))
}

pub fn pointer_path(base_dir: &Path, session_id: &str) -> PathBuf {
    display_root(base_dir, session_id).join(POINTER_FILE_NAME)
}

pub fn journal_path(base_dir: &Path, session_id: &str) -> PathBuf {
    display_root(base_dir, session_id).join(JOURNAL_FILE_NAME)
}

fn staging_path(
    base_dir: &Path,
    session_id: &str,
    request_id: &str,
    seq: u64,
) -> Result<PathBuf, HostErrorEnvelope> {
    validate_request_path_component(request_id)?;

    Ok(checked_display_root(base_dir, session_id)?
        .join(STAGING_DIR_NAME)
        .join(format!("{request_id}-{seq:06}.jpg")))
}

/// 확정 경로. `<display_root>/<requestId>/<seq>-<variant>.jpg`
pub fn generation_path(
    base_dir: &Path,
    session_id: &str,
    request_id: &str,
    seq: u64,
    variant: &str,
) -> Result<PathBuf, HostErrorEnvelope> {
    validate_request_path_component(request_id)?;

    Ok(checked_display_root(base_dir, session_id)?
        .join(request_id)
        .join(format!("{seq:06}-{variant}.jpg")))
}

pub fn generation_id(request_id: &str, seq: u64) -> String {
    format!("{request_id}-{seq:06}")
}

fn ensure_dir(path: &Path) -> Result<(), HostErrorEnvelope> {
    fs::create_dir_all(path).map_err(|error| {
        HostErrorEnvelope::persistence(format!("표시 자산 경로를 준비하지 못했어요: {error}"))
    })
}

/// 1~3단계: staging 파일에 완전히 쓰고 닫은 뒤 구조를 검증한다.
///
/// `sync_all()` 이후에 읽어야 캐시에만 존재하는 상태를 "완료"로 오판하지 않는다.
pub fn stage_generation(
    base_dir: &Path,
    session_id: &str,
    request_id: &str,
    seq: u64,
    source_bytes: &[u8],
    now: &dyn Fn() -> u64,
) -> Result<StagedGeneration, StageGenerationError> {
    let staging_path = staging_path(base_dir, session_id, request_id, seq)
        .map_err(StageGenerationError::Persistence)?;

    if let Some(parent) = staging_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            StageGenerationError::Persistence(HostErrorEnvelope::persistence(format!(
                "표시 staging 경로를 준비하지 못했어요: {error}"
            )))
        })?;
    }

    // 같은 seq가 두 번 예약되는 일은 없어야 한다. 남은 파일이 있으면 이전 시도의 잔여물이다.
    let _ = fs::remove_file(&staging_path);

    let write_result = (|| -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staging_path)?;
        file.write_all(source_bytes)?;
        file.flush()?;
        file.sync_all()?;
        Ok(())
    })();

    if let Err(error) = write_result {
        let _ = fs::remove_file(&staging_path);
        return Err(StageGenerationError::Persistence(
            HostErrorEnvelope::persistence(format!("표시 자산을 저장하지 못했어요: {error}")),
        ));
    }
    let file_ready_at_micros = now();

    let written = match fs::read(&staging_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = fs::remove_file(&staging_path);
            return Err(StageGenerationError::Persistence(
                HostErrorEnvelope::persistence(format!("표시 자산을 다시 읽지 못했어요: {error}")),
            ));
        }
    };

    let probe = match probe_jpeg(&written) {
        Ok(probe) => probe,
        Err(error) => {
            let _ = fs::remove_file(&staging_path);
            return Err(StageGenerationError::Probe(error));
        }
    };
    let probe_ok_at_micros = now();

    Ok(StagedGeneration {
        byte_size: written.len() as u64,
        source_hash: content_hash(&written),
        staging_path,
        probe,
        file_ready_at_micros,
        probe_ok_at_micros,
    })
}

pub fn discard_staged(staged: &StagedGeneration) {
    let _ = fs::remove_file(&staged.staging_path);
}

/// 6단계: 확정 경로로 승격한다. 이미 존재하면 **하드 에러**다 (immutable 보장).
///
/// 게시는 `DisplayState` mutex로 직렬화되고 seq는 세션 안에서 유일하므로
/// 존재 확인과 rename 사이에 경합이 생기지 않는다.
pub fn promote_generation(
    staged: &StagedGeneration,
    final_path: &Path,
) -> Result<(), HostErrorEnvelope> {
    if let Some(parent) = final_path.parent() {
        ensure_dir(parent)?;
    }

    if final_path.try_exists().unwrap_or(false) {
        return Err(HostErrorEnvelope::persistence(
            "이미 존재하는 표시 자산 경로를 덮어쓸 수 없어요.",
        ));
    }

    fs::rename(&staged.staging_path, final_path).map_err(|error| {
        HostErrorEnvelope::persistence(format!("표시 자산을 확정하지 못했어요: {error}"))
    })
}

/// 7단계: pointer를 원자적으로 교체한다.
///
/// Windows의 `std::fs::rename`은 `MoveFileExW + MOVEFILE_REPLACE_EXISTING`이라
/// 대상이 있어도 교체된다.
pub fn write_pointer(
    base_dir: &Path,
    session_id: &str,
    snapshot: &DisplayPointerSnapshotDto,
) -> Result<(), HostErrorEnvelope> {
    let root = checked_display_root(base_dir, session_id)?;
    ensure_dir(&root)?;

    let pointer_path = root.join(POINTER_FILE_NAME);
    let temp_path = root.join(format!("{POINTER_FILE_NAME}.tmp"));
    let payload = serde_json::to_vec_pretty(snapshot).map_err(|error| {
        HostErrorEnvelope::persistence(format!("표시 포인터를 준비하지 못했어요: {error}"))
    })?;

    let write_result = (|| -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)?;
        file.write_all(&payload)?;
        file.flush()?;
        file.sync_all()?;
        Ok(())
    })();

    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(HostErrorEnvelope::persistence(format!(
            "표시 포인터를 저장하지 못했어요: {error}"
        )));
    }

    fs::rename(&temp_path, &pointer_path).map_err(|error| {
        let _ = fs::remove_file(&temp_path);
        HostErrorEnvelope::persistence(format!("표시 포인터를 교체하지 못했어요: {error}"))
    })
}

pub fn read_pointer(base_dir: &Path, session_id: &str) -> Option<DisplayPointerSnapshotDto> {
    let raw = fs::read_to_string(
        checked_display_root(base_dir, session_id)
            .ok()?
            .join(POINTER_FILE_NAME),
    )
    .ok()?;

    serde_json::from_str(&raw).ok()
}

/// reconcile 결과를 현재 세션과 이전 세션의 durable pointer에 함께 반영한다.
/// 세션 교체 시 이전 세션 pointer도 빈 상태로 덮어써 재사용될 여지를 없앤다.
pub fn write_reconciled_pointer(
    base_dir: &Path,
    previous_session_id: Option<&str>,
    snapshot: &DisplayPointerSnapshotDto,
) -> Result<(), HostErrorEnvelope> {
    if previous_session_id != snapshot.session_id.as_deref() {
        if let Some(previous_session_id) = previous_session_id {
            let mut cleared = snapshot.clone();
            cleared.session_id = None;
            cleared.active_generation = None;
            cleared.required_source_width_px = 0;
            cleared.required_source_height_px = 0;
            cleared.measurement_lane_enabled = false;
            write_pointer(base_dir, previous_session_id, &cleared)?;
        }
    }

    if let Some(session_id) = snapshot.session_id.as_deref() {
        write_pointer(base_dir, session_id, snapshot)?;
    }

    Ok(())
}

/// 8단계: append-only journal. 승격과 거부를 모두 기록한다.
pub fn append_journal(
    base_dir: &Path,
    session_id: &str,
    record: &DisplayJournalRecord,
) -> Result<(), HostErrorEnvelope> {
    let root = checked_display_root(base_dir, session_id)?;
    ensure_dir(&root)?;

    let mut line = serde_json::to_string(record).map_err(|error| {
        HostErrorEnvelope::persistence(format!("표시 이력을 준비하지 못했어요: {error}"))
    })?;
    line.push('\n');

    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(root.join(JOURNAL_FILE_NAME))
        .map_err(|error| {
            HostErrorEnvelope::persistence(format!("표시 이력을 열지 못했어요: {error}"))
        })?;

    file.write_all(line.as_bytes()).map_err(|error| {
        HostErrorEnvelope::persistence(format!("표시 이력을 저장하지 못했어요: {error}"))
    })?;
    file.flush().map_err(|error| {
        HostErrorEnvelope::persistence(format!("표시 이력을 flush하지 못했어요: {error}"))
    })?;
    file.sync_all().map_err(|error| {
        HostErrorEnvelope::persistence(format!("표시 이력을 동기화하지 못했어요: {error}"))
    })
}

pub fn read_journal(base_dir: &Path, session_id: &str) -> Vec<DisplayJournalRecord> {
    let Ok(root) = checked_display_root(base_dir, session_id) else {
        return Vec::new();
    };
    let Ok(raw) = fs::read_to_string(root.join(JOURNAL_FILE_NAME)) else {
        return Vec::new();
    };

    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// capture 삭제 시 해당 request의 immutable generation 디렉터리를 정리한다.
/// journal은 append-only 감사 기록이므로 지우지 않고 `request-forgotten` 항목을 덧붙인다.
pub fn remove_request_generations(
    base_dir: &Path,
    session_id: &str,
    request_id: &str,
    recorded_at_host_micros: u64,
) -> Result<(), HostErrorEnvelope> {
    validate_request_path_component(request_id)?;
    let request_dir = checked_display_root(base_dir, session_id)?.join(request_id);

    if !request_dir.try_exists().unwrap_or(false) {
        return Ok(());
    }

    fs::remove_dir_all(&request_dir).map_err(|error| {
        HostErrorEnvelope::persistence(format!("표시 자산을 정리하지 못했어요: {error}"))
    })?;

    append_journal(
        base_dir,
        session_id,
        &DisplayJournalRecord {
            schema_version: VIEWER_DISPLAY_SCHEMA_VERSION.into(),
            outcome: "request-forgotten".into(),
            reject_reason: None,
            session_id: session_id.into(),
            request_id: request_id.into(),
            generation_seq: 0,
            generation: None,
            recorded_at_host_micros,
        },
    )
}
