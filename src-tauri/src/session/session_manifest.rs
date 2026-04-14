use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::contracts::dto::{HostErrorEnvelope, HostFieldErrors, SessionStartInputDto};

pub const SESSION_MANIFEST_SCHEMA_VERSION: &str = "session-manifest/v1";
pub const SESSION_TIMING_SCHEMA_VERSION: &str = "session-timing/v1";
pub const SESSION_POST_END_EXPORT_WAITING: &str = "export-waiting";
pub const SESSION_POST_END_COMPLETED: &str = "completed";
pub const SESSION_POST_END_PHONE_REQUIRED: &str = "phone-required";
pub const SESSION_POST_END_LOCAL_DELIVERABLE_READY: &str = "local-deliverable-ready";
pub const SESSION_POST_END_HANDOFF_READY: &str = "handoff-ready";
pub const DEFAULT_SESSION_DURATION_SECONDS: u64 = 15 * 60;
pub const WARNING_LEAD_SECONDS: u64 = 5 * 60;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTiming {
    pub schema_version: String,
    pub session_id: String,
    pub adjusted_end_at: String,
    pub warning_at: String,
    pub phase: String,
    pub capture_allowed: bool,
    pub approved_extension_minutes: u32,
    pub approved_extension_audit_ref: Option<String>,
    pub warning_triggered_at: Option<String>,
    pub ended_triggered_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportWaitingPostEnd {
    pub state: String,
    #[serde(default = "legacy_post_end_evaluated_at")]
    pub evaluated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletedPostEnd {
    pub state: String,
    #[serde(default = "legacy_post_end_evaluated_at")]
    pub evaluated_at: String,
    pub completion_variant: String,
    #[serde(default)]
    pub approved_recipient_label: Option<String>,
    #[serde(default)]
    pub next_location_label: Option<String>,
    pub primary_action_label: String,
    #[serde(default)]
    pub support_action_label: Option<String>,
    pub show_booth_alias: bool,
    #[serde(default)]
    pub handoff: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoneRequiredPostEnd {
    pub state: String,
    #[serde(default = "legacy_post_end_evaluated_at")]
    pub evaluated_at: String,
    pub primary_action_label: String,
    #[serde(default)]
    pub support_action_label: Option<String>,
    pub unsafe_action_warning: String,
    #[serde(default)]
    pub show_booth_alias: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SessionPostEnd {
    ExportWaiting(ExportWaitingPostEnd),
    Completed(CompletedPostEnd),
    PhoneRequired(PhoneRequiredPostEnd),
}

impl SessionPostEnd {
    pub fn export_waiting(evaluated_at: String) -> Self {
        Self::ExportWaiting(ExportWaitingPostEnd {
            state: SESSION_POST_END_EXPORT_WAITING.into(),
            evaluated_at,
        })
    }

    pub fn completed(
        evaluated_at: String,
        completion_variant: String,
        primary_action_label: String,
        support_action_label: Option<String>,
        show_booth_alias: bool,
        handoff: Option<serde_json::Value>,
    ) -> Self {
        Self::Completed(CompletedPostEnd {
            state: SESSION_POST_END_COMPLETED.into(),
            evaluated_at,
            completion_variant,
            approved_recipient_label: None,
            next_location_label: None,
            primary_action_label,
            support_action_label,
            show_booth_alias,
            handoff,
        })
    }

    pub fn phone_required(
        evaluated_at: String,
        primary_action_label: String,
        support_action_label: Option<String>,
        unsafe_action_warning: String,
        show_booth_alias: bool,
    ) -> Self {
        Self::PhoneRequired(PhoneRequiredPostEnd {
            state: SESSION_POST_END_PHONE_REQUIRED.into(),
            evaluated_at,
            primary_action_label,
            support_action_label,
            unsafe_action_warning,
            show_booth_alias,
        })
    }

    pub fn state(&self) -> &str {
        match self {
            Self::ExportWaiting(value) => &value.state,
            Self::Completed(value) => &value.state,
            Self::PhoneRequired(value) => &value.state,
        }
    }

    pub fn evaluated_at(&self) -> &str {
        match self {
            Self::ExportWaiting(value) => &value.evaluated_at,
            Self::Completed(value) => &value.evaluated_at,
            Self::PhoneRequired(value) => &value.evaluated_at,
        }
    }

    pub fn completion_variant(&self) -> Option<&str> {
        match self {
            Self::Completed(value) => Some(value.completion_variant.as_str()),
            _ => None,
        }
    }

    pub fn handoff(&self) -> Option<&serde_json::Value> {
        match self {
            Self::Completed(value) => value.handoff.as_ref(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivePresetBinding {
    pub preset_id: String,
    pub published_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyPhoneRequiredPostEnd {
    pub state: String,
    pub primary_action_label: String,
    #[serde(default)]
    pub support_action_label: Option<String>,
    pub unsafe_action_warning: String,
    #[serde(default)]
    pub show_booth_alias: bool,
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
    #[serde(default)]
    pub fast_preview_visible_at_ms: Option<u64>,
    #[serde(default)]
    pub xmp_preview_ready_at_ms: Option<u64>,
    pub capture_budget_ms: u64,
    pub preview_budget_ms: u64,
    pub preview_budget_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRendererRouteSnapshot {
    pub route: String,
    pub route_stage: String,
    #[serde(default)]
    pub fallback_reason_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRendererWarmStateSnapshot {
    pub preset_id: String,
    pub published_version: String,
    pub state: String,
    pub observed_at: String,
    #[serde(default)]
    pub diagnostics_detail_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCaptureRecord {
    pub schema_version: String,
    pub session_id: String,
    pub booth_alias: String,
    #[serde(default)]
    pub active_preset_id: Option<String>,
    pub active_preset_version: String,
    #[serde(default)]
    pub active_preset_display_name: Option<String>,
    #[serde(default)]
    pub preview_renderer_route: Option<PreviewRendererRouteSnapshot>,
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
    pub catalog_revision: Option<u64>,
    #[serde(default)]
    pub catalog_snapshot: Option<Vec<ActivePresetBinding>>,
    #[serde(default)]
    pub active_preset: Option<ActivePresetBinding>,
    #[serde(default)]
    pub active_preset_id: Option<String>,
    #[serde(default)]
    pub active_preset_display_name: Option<String>,
    #[serde(default)]
    pub active_preview_renderer_route: Option<PreviewRendererRouteSnapshot>,
    #[serde(default)]
    pub active_preview_renderer_warm_state: Option<PreviewRendererWarmStateSnapshot>,
    #[serde(default)]
    pub timing: Option<SessionTiming>,
    #[serde(default)]
    pub captures: Vec<SessionCaptureRecord>,
    #[serde(default)]
    pub post_end: Option<SessionPostEnd>,
}

pub fn normalize_legacy_manifest(manifest: &mut SessionManifest) {
    let fallback_active_preset_id = manifest.active_preset_id.clone().or_else(|| {
        manifest
            .active_preset
            .as_ref()
            .map(|preset| preset.preset_id.clone())
    });
    let fallback_active_preset_version = manifest
        .active_preset
        .as_ref()
        .map(|preset| preset.published_version.clone());
    let fallback_active_preset_display_name = manifest.active_preset_display_name.clone();
    let fallback_active_preview_renderer_route = manifest.active_preview_renderer_route.clone();

    for capture in &mut manifest.captures {
        let matches_manifest_active_preset = fallback_active_preset_version
            .as_ref()
            .map(|published_version| published_version == &capture.active_preset_version)
            .unwrap_or(false);

        if capture.active_preset_id.is_none() && matches_manifest_active_preset {
            capture.active_preset_id = fallback_active_preset_id.clone();
        }

        if capture.active_preset_display_name.is_none()
            && matches_manifest_active_preset
            && capture.active_preset_id == fallback_active_preset_id
        {
            capture.active_preset_display_name = fallback_active_preset_display_name.clone();
        }

        if capture.preview_renderer_route.is_none()
            && matches_manifest_active_preset
            && capture.active_preset_id == fallback_active_preset_id
        {
            capture.preview_renderer_route = fallback_active_preview_renderer_route.clone();
        }
    }
}

fn legacy_post_end_evaluated_at() -> String {
    "1970-01-01T00:00:00Z".into()
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
    let timing = build_default_session_timing(session_id.clone(), &timestamp)?;

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
        catalog_revision: None,
        catalog_snapshot: None,
        active_preset: None,
        active_preset_id: None,
        active_preset_display_name: None,
        active_preview_renderer_route: None,
        active_preview_renderer_warm_state: None,
        timing: Some(timing),
        captures: Vec::new(),
        post_end: None,
    })
}

pub fn build_default_session_timing(
    session_id: String,
    started_at: &str,
) -> Result<SessionTiming, HostErrorEnvelope> {
    build_default_session_timing_for_mode(session_id, started_at, false)
}

pub fn build_default_session_timing_for_mode(
    session_id: String,
    started_at: &str,
    _use_local_test_timing: bool,
) -> Result<SessionTiming, HostErrorEnvelope> {
    let started_at_seconds = rfc3339_to_unix_seconds(started_at)?;
    let adjusted_end_at_seconds =
        started_at_seconds.saturating_add(DEFAULT_SESSION_DURATION_SECONDS);
    let adjusted_end_at = unix_seconds_to_rfc3339(adjusted_end_at_seconds);
    let warning_at =
        unix_seconds_to_rfc3339(adjusted_end_at_seconds.saturating_sub(WARNING_LEAD_SECONDS));

    Ok(SessionTiming {
        schema_version: SESSION_TIMING_SCHEMA_VERSION.into(),
        session_id,
        adjusted_end_at,
        warning_at,
        phase: "active".into(),
        capture_allowed: true,
        approved_extension_minutes: 0,
        approved_extension_audit_ref: None,
        warning_triggered_at: None,
        ended_triggered_at: None,
    })
}

pub fn resolve_warning_lead_seconds(_use_local_test_timing: bool) -> u64 {
    WARNING_LEAD_SECONDS
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

pub(crate) fn unix_seconds_to_rfc3339(unix_seconds: u64) -> String {
    let seconds_per_day = 86_400;
    let days = (unix_seconds / seconds_per_day) as i64;
    let seconds_of_day = unix_seconds % seconds_per_day;

    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

pub fn rfc3339_to_unix_seconds(timestamp: &str) -> Result<u64, HostErrorEnvelope> {
    let timestamp = timestamp.trim();
    let (timestamp, offset_seconds) = split_rfc3339_offset(timestamp)?;
    let (date, time) = timestamp.split_once('T').ok_or_else(|| {
        HostErrorEnvelope::persistence("세션 타이밍을 읽지 못했어요. 잠시 후 다시 확인해 주세요.")
    })?;
    let time = time.split_once('.').map(|(value, _)| value).unwrap_or(time);
    let (year, month, day) = parse_rfc3339_date(date)?;
    let (hour, minute, second) = parse_rfc3339_time(time)?;
    let days = days_from_civil(year, month, day);

    if days < 0 {
        return Err(HostErrorEnvelope::persistence(
            "세션 타이밍을 읽지 못했어요. 잠시 후 다시 확인해 주세요.",
        ));
    }

    let timestamp_seconds =
        (days as i64) * 86_400 + (hour as i64) * 3_600 + (minute as i64) * 60 + second as i64;
    let utc_seconds = timestamp_seconds
        .checked_sub(offset_seconds)
        .ok_or_else(|| {
            HostErrorEnvelope::persistence(
                "세션 타이밍을 읽지 못했어요. 잠시 후 다시 확인해 주세요.",
            )
        })?;

    if utc_seconds < 0 {
        return Err(HostErrorEnvelope::persistence(
            "세션 타이밍을 읽지 못했어요. 잠시 후 다시 확인해 주세요.",
        ));
    }

    Ok(utc_seconds as u64)
}

fn split_rfc3339_offset(timestamp: &str) -> Result<(&str, i64), HostErrorEnvelope> {
    if let Some(stripped) = timestamp.strip_suffix('Z') {
        return Ok((stripped, 0));
    }

    let time_index = timestamp.find('T').ok_or_else(|| {
        HostErrorEnvelope::persistence("세션 타이밍을 읽지 못했어요. 잠시 후 다시 확인해 주세요.")
    })?;
    let offset_index = timestamp[time_index + 1..]
        .find(['+', '-'])
        .map(|index| time_index + 1 + index)
        .ok_or_else(|| {
            HostErrorEnvelope::persistence(
                "세션 타이밍을 읽지 못했어요. 잠시 후 다시 확인해 주세요.",
            )
        })?;
    let (date_time, offset) = timestamp.split_at(offset_index);

    Ok((date_time, parse_rfc3339_offset(offset)?))
}

fn parse_rfc3339_offset(offset: &str) -> Result<i64, HostErrorEnvelope> {
    if offset.len() != 6 {
        return Err(HostErrorEnvelope::persistence(
            "세션 타이밍을 읽지 못했어요. 잠시 후 다시 확인해 주세요.",
        ));
    }

    let sign = match offset.as_bytes()[0] {
        b'+' => 1_i64,
        b'-' => -1_i64,
        _ => {
            return Err(HostErrorEnvelope::persistence(
                "세션 타이밍을 읽지 못했어요. 잠시 후 다시 확인해 주세요.",
            ))
        }
    };

    if offset.as_bytes()[3] != b':' {
        return Err(HostErrorEnvelope::persistence(
            "세션 타이밍을 읽지 못했어요. 잠시 후 다시 확인해 주세요.",
        ));
    }

    let hours = offset[1..3].parse::<i64>().map_err(|_| {
        HostErrorEnvelope::persistence("세션 타이밍을 읽지 못했어요. 잠시 후 다시 확인해 주세요.")
    })?;
    let minutes = offset[4..6].parse::<i64>().map_err(|_| {
        HostErrorEnvelope::persistence("세션 타이밍을 읽지 못했어요. 잠시 후 다시 확인해 주세요.")
    })?;

    if hours > 23 || minutes > 59 {
        return Err(HostErrorEnvelope::persistence(
            "세션 타이밍을 읽지 못했어요. 잠시 후 다시 확인해 주세요.",
        ));
    }

    Ok(sign * (hours * 3_600 + minutes * 60))
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

fn parse_rfc3339_date(date: &str) -> Result<(i32, u32, u32), HostErrorEnvelope> {
    if date.len() != 10 {
        return Err(HostErrorEnvelope::persistence(
            "세션 타이밍을 읽지 못했어요. 잠시 후 다시 확인해 주세요.",
        ));
    }

    let year = date[0..4].parse::<i32>().map_err(|_| {
        HostErrorEnvelope::persistence("세션 타이밍을 읽지 못했어요. 잠시 후 다시 확인해 주세요.")
    })?;
    let month = date[5..7].parse::<u32>().map_err(|_| {
        HostErrorEnvelope::persistence("세션 타이밍을 읽지 못했어요. 잠시 후 다시 확인해 주세요.")
    })?;
    let day = date[8..10].parse::<u32>().map_err(|_| {
        HostErrorEnvelope::persistence("세션 타이밍을 읽지 못했어요. 잠시 후 다시 확인해 주세요.")
    })?;

    Ok((year, month, day))
}

fn parse_rfc3339_time(time: &str) -> Result<(u32, u32, u32), HostErrorEnvelope> {
    if time.len() != 8 {
        return Err(HostErrorEnvelope::persistence(
            "세션 타이밍을 읽지 못했어요. 잠시 후 다시 확인해 주세요.",
        ));
    }

    let hour = time[0..2].parse::<u32>().map_err(|_| {
        HostErrorEnvelope::persistence("세션 타이밍을 읽지 못했어요. 잠시 후 다시 확인해 주세요.")
    })?;
    let minute = time[3..5].parse::<u32>().map_err(|_| {
        HostErrorEnvelope::persistence("세션 타이밍을 읽지 못했어요. 잠시 후 다시 확인해 주세요.")
    })?;
    let second = time[6..8].parse::<u32>().map_err(|_| {
        HostErrorEnvelope::persistence("세션 타이밍을 읽지 못했어요. 잠시 후 다시 확인해 주세요.")
    })?;

    Ok((hour, minute, second))
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let adjusted_year = year - if month <= 2 { 1 } else { 0 };
    let era = if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    } / 400;
    let year_of_era = adjusted_year - era * 400;
    let month_prime = month as i32 + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day as i32 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;

    (era * 146_097 + day_of_era - 719_468) as i64
}
