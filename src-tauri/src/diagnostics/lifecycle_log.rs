use chrono::DateTime;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tauri::State;

use crate::{
    db::sqlite::{open_operational_log_connection, OperationalLogState},
    diagnostics::error::OperationalLogError,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleEventKind {
    FirstScreenDisplayed,
    SessionCreated,
    ReadinessReached,
    WarningShown,
    ActualShootEnd,
    ExportStateChanged,
    PresetCatalogFallback,
    SessionCompleted,
    PhoneRequired,
}

impl LifecycleEventKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::FirstScreenDisplayed => "first_screen_displayed",
            Self::SessionCreated => "session_created",
            Self::ReadinessReached => "readiness_reached",
            Self::WarningShown => "warning_shown",
            Self::ActualShootEnd => "actual_shoot_end",
            Self::ExportStateChanged => "export_state_changed",
            Self::PresetCatalogFallback => "preset_catalog_fallback",
            Self::SessionCompleted => "session_completed",
            Self::PhoneRequired => "phone_required",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LifecycleEventWrite {
    pub payload_version: i64,
    pub event_type: LifecycleEventKind,
    pub occurred_at: String,
    pub branch_id: String,
    pub session_id: Option<String>,
    pub session_name: Option<String>,
    pub current_stage: String,
    pub actual_shoot_end_at: Option<String>,
    pub catalog_fallback_reason: Option<String>,
    pub extension_status: Option<String>,
    pub recent_fault_category: Option<String>,
}

pub fn insert_lifecycle_event(
    connection: &Connection,
    event: &LifecycleEventWrite,
) -> Result<(), OperationalLogError> {
    validate_lifecycle_event(event)?;

    connection.execute(
        "INSERT INTO session_events (
            payload_version,
            event_type,
            occurred_at,
            branch_id,
            session_id,
            session_name,
            current_stage,
            actual_shoot_end_at,
            extension_status,
            recent_fault_category,
            payload_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            event.payload_version,
            event.event_type.as_str(),
            event.occurred_at,
            event.branch_id,
            event.session_id,
            event.session_name,
            event.current_stage,
            event.actual_shoot_end_at,
            event.extension_status,
            event.recent_fault_category,
            build_payload_json(
                event.payload_version,
                event.actual_shoot_end_at.as_deref(),
                event.catalog_fallback_reason.as_deref(),
                event.extension_status.as_deref(),
                event.recent_fault_category.as_deref(),
                None,
            )
            .to_string(),
        ],
    )?;

    Ok(())
}

pub fn parse_lifecycle_event(event: Value) -> Result<LifecycleEventWrite, OperationalLogError> {
    let event: LifecycleEventWrite = serde_json::from_value(event)?;
    validate_lifecycle_event(&event)?;
    Ok(event)
}

#[tauri::command]
pub fn record_lifecycle_event(
    state: State<'_, OperationalLogState>,
    event: Value,
) -> Result<(), OperationalLogError> {
    let event = parse_lifecycle_event(event)?;
    let connection = open_operational_log_connection(state.db_path())?;
    insert_lifecycle_event(&connection, &event)
}

fn validate_lifecycle_event(event: &LifecycleEventWrite) -> Result<(), OperationalLogError> {
    validate_payload_version(event.payload_version)?;
    validate_timestamp("occurredAt", &event.occurred_at)?;
    validate_required_text("branchId", &event.branch_id, 120)?;
    validate_required_text("currentStage", &event.current_stage, 80)?;
    validate_optional_text("sessionId", event.session_id.as_deref(), 120)?;
    validate_optional_text("sessionName", event.session_name.as_deref(), 160)?;
    validate_optional_timestamp("actualShootEndAt", event.actual_shoot_end_at.as_deref())?;
    validate_optional_catalog_fallback_reason(event.catalog_fallback_reason.as_deref())?;
    validate_optional_text("extensionStatus", event.extension_status.as_deref(), 80)?;
    validate_optional_text("recentFaultCategory", event.recent_fault_category.as_deref(), 120)?;

    if matches!(event.event_type, LifecycleEventKind::PresetCatalogFallback)
        && event.catalog_fallback_reason.is_none()
    {
        return Err(OperationalLogError::invalid_payload(
            "catalogFallbackReason is required",
        ));
    }

    Ok(())
}

pub(crate) fn validate_payload_version(payload_version: i64) -> Result<(), OperationalLogError> {
    if payload_version < 1 {
        return Err(OperationalLogError::invalid_payload(
            "payloadVersion must be greater than or equal to 1",
        ));
    }

    Ok(())
}

pub(crate) fn validate_required_text(
    field_name: &str,
    value: &str,
    max_length: usize,
) -> Result<(), OperationalLogError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(OperationalLogError::invalid_payload(format!("{field_name} is required")));
    }
    if trimmed.len() > max_length {
        return Err(OperationalLogError::invalid_payload(format!(
            "{field_name} must be at most {max_length} characters"
        )));
    }

    Ok(())
}

pub(crate) fn validate_optional_text(
    field_name: &str,
    value: Option<&str>,
    max_length: usize,
) -> Result<(), OperationalLogError> {
    if let Some(value) = value {
        validate_required_text(field_name, value, max_length)?;
    }

    Ok(())
}

pub(crate) fn validate_timestamp(field_name: &str, value: &str) -> Result<(), OperationalLogError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(OperationalLogError::invalid_payload(format!(
            "{field_name} must be a non-empty ISO-8601 timestamp"
        )));
    }

    if DateTime::parse_from_rfc3339(trimmed).is_err() {
        return Err(OperationalLogError::invalid_payload(format!(
            "{field_name} must be a non-empty ISO-8601 timestamp"
        )));
    }

    Ok(())
}

pub(crate) fn validate_optional_timestamp(
    field_name: &str,
    value: Option<&str>,
) -> Result<(), OperationalLogError> {
    if let Some(value) = value {
        validate_timestamp(field_name, value)?;
    }

    Ok(())
}

pub(crate) fn validate_optional_catalog_fallback_reason(
    value: Option<&str>,
) -> Result<(), OperationalLogError> {
    let Some(value) = value else {
        return Ok(());
    };

    validate_required_text("catalogFallbackReason", value, 40)?;

    match value.trim() {
        "invalid_id"
        | "invalid_catalog_shape"
        | "missing_catalog_input"
        | "name_mismatch"
        | "oversized_catalog"
        | "reordered_catalog" => Ok(()),
        _ => Err(OperationalLogError::invalid_payload(
            "catalogFallbackReason must be an approved reason code",
        )),
    }
}

pub(crate) fn build_payload_json(
    payload_version: i64,
    actual_shoot_end_at: Option<&str>,
    catalog_fallback_reason: Option<&str>,
    extension_status: Option<&str>,
    recent_fault_category: Option<&str>,
    intervention_outcome: Option<&str>,
) -> Value {
    let mut payload = Map::new();
    payload.insert("payloadVersion".into(), json!(payload_version));

    if let Some(value) = actual_shoot_end_at {
        payload.insert("actualShootEndAt".into(), json!(value));
    }
    if let Some(value) = catalog_fallback_reason {
        payload.insert("catalogFallbackReason".into(), json!(value));
    }
    if let Some(value) = extension_status {
        payload.insert("extensionStatus".into(), json!(value));
    }
    if let Some(value) = recent_fault_category {
        payload.insert("recentFaultCategory".into(), json!(value));
    }
    if let Some(value) = intervention_outcome {
        payload.insert("interventionOutcome".into(), json!(value));
    }

    Value::Object(payload)
}
