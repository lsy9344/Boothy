use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::contracts::dto::{HostErrorEnvelope, HostFieldErrors, SessionStartInputDto};

pub const SESSION_MANIFEST_SCHEMA_VERSION: &str = "session-manifest/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCustomer {
    pub name: String,
    pub phone_last_four: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionLifecycle {
    pub status: String,
    pub stage: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivePresetBinding {
    pub preset_id: String,
    pub published_version: String,
}

pub const SESSION_CAPTURE_SCHEMA_VERSION: &str = "session-capture/v1";
pub const CAPTURE_BUDGET_MS: u64 = 1_000;
pub const PREVIEW_BUDGET_MS: u64 = 5_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawCaptureAsset {
    pub asset_path: String,
    pub persisted_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewCaptureAsset {
    pub asset_path: Option<String>,
    pub enqueued_at_ms: Option<u64>,
    pub ready_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalCaptureAsset {
    pub asset_path: Option<String>,
    pub ready_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureTimingMetrics {
    pub capture_acknowledged_at_ms: u64,
    pub preview_visible_at_ms: Option<u64>,
    pub capture_budget_ms: u64,
    pub preview_budget_ms: u64,
    pub preview_budget_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCaptureRecord {
    pub schema_version: String,
    pub session_id: String,
    pub booth_alias: String,
    pub active_preset_version: String,
    pub capture_id: String,
    pub request_id: String,
    pub raw: RawCaptureAsset,
    pub preview: PreviewCaptureAsset,
    #[serde(rename = "final")]
    pub final_asset: FinalCaptureAsset,
    pub render_status: String,
    pub post_end_state: String,
    pub timing: CaptureTimingMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionManifest {
    pub schema_version: String,
    pub session_id: String,
    pub booth_alias: String,
    pub customer: SessionCustomer,
    pub created_at: String,
    pub updated_at: String,
    pub lifecycle: SessionLifecycle,
    #[serde(default)]
    pub active_preset: Option<ActivePresetBinding>,
    #[serde(default)]
    pub active_preset_id: Option<String>,
    #[serde(default)]
    pub captures: Vec<SessionCaptureRecord>,
    pub post_end: Option<serde_json::Value>,
}

pub fn normalize_customer_name(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn validate_session_start_input(
    input: &SessionStartInputDto,
) -> Result<SessionStartInputDto, HostErrorEnvelope> {
    let normalized_name = normalize_customer_name(&input.name);
    let normalized_phone = input.phone_last_four.clone();

    let mut field_errors = HostFieldErrors {
        name: None,
        phone_last_four: None,
    };

    if normalized_name.is_empty() {
        field_errors.name = Some("이름을 입력해 주세요.".into());
    }

    if normalized_phone.len() != 4 || !normalized_phone.chars().all(|char| char.is_ascii_digit()) {
        field_errors.phone_last_four = Some("휴대전화 뒤 4자리는 숫자 4자리여야 해요.".into());
    }

    if field_errors.name.is_some() || field_errors.phone_last_four.is_some() {
        return Err(HostErrorEnvelope::validation(field_errors));
    }

    Ok(SessionStartInputDto {
        name: normalized_name,
        phone_last_four: normalized_phone,
    })
}

pub fn build_booth_alias(name: &str, phone_last_four: &str) -> String {
    format!("{name} {phone_last_four}")
}

pub fn build_session_manifest(
    session_id: String,
    input: SessionStartInputDto,
) -> Result<SessionManifest, HostErrorEnvelope> {
    build_session_manifest_at(session_id, input, SystemTime::now())
}

pub fn build_session_manifest_at(
    session_id: String,
    input: SessionStartInputDto,
    now: SystemTime,
) -> Result<SessionManifest, HostErrorEnvelope> {
    let timestamp = current_timestamp(now)?;
    let booth_alias = build_booth_alias(&input.name, &input.phone_last_four);

    Ok(SessionManifest {
        schema_version: SESSION_MANIFEST_SCHEMA_VERSION.into(),
        session_id,
        booth_alias,
        customer: SessionCustomer {
            name: input.name,
            phone_last_four: input.phone_last_four,
        },
        created_at: timestamp.clone(),
        updated_at: timestamp,
        lifecycle: SessionLifecycle {
            status: "active".into(),
            stage: "session-started".into(),
        },
        active_preset: None,
        active_preset_id: None,
        captures: Vec::new(),
        post_end: None,
    })
}

pub fn current_timestamp(now: SystemTime) -> Result<String, HostErrorEnvelope> {
    let unix_seconds = now
        .duration_since(UNIX_EPOCH)
        .map_err(|_| {
            HostErrorEnvelope::persistence("시스템 시계를 확인할 수 없어 세션을 시작하지 못했어요.")
        })?
        .as_secs();

    Ok(unix_seconds_to_rfc3339(unix_seconds))
}

fn unix_seconds_to_rfc3339(unix_seconds: u64) -> String {
    let seconds_per_day = 86_400;
    let days = (unix_seconds / seconds_per_day) as i64;
    let seconds_of_day = unix_seconds % seconds_per_day;

    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let adjusted_year = year + if month <= 2 { 1 } else { 0 };

    (adjusted_year as i32, month as u32, day as u32)
}
