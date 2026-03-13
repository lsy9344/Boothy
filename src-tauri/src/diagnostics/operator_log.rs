use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

use crate::{
    db::sqlite::{open_operational_log_connection, OperationalLogState},
    diagnostics::{
        error::OperationalLogError,
        lifecycle_log::{
            build_payload_json, validate_optional_text, validate_optional_timestamp,
            validate_payload_version, validate_required_text, validate_timestamp,
        },
    },
};

const OPERATOR_INTERVENTION_EVENT_TYPE: &str = "operator_intervention_recorded";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperatorInterventionWrite {
    pub payload_version: i64,
    pub occurred_at: String,
    pub branch_id: String,
    pub session_id: Option<String>,
    pub session_name: Option<String>,
    pub current_stage: String,
    pub actual_shoot_end_at: Option<String>,
    pub extension_status: Option<String>,
    pub recent_fault_category: Option<String>,
    pub intervention_outcome: String,
}

pub fn insert_operator_intervention(
    connection: &Connection,
    intervention: &OperatorInterventionWrite,
) -> Result<(), OperationalLogError> {
    validate_operator_intervention(intervention)?;

    connection.execute(
        "INSERT INTO operator_interventions (
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
            intervention_outcome,
            payload_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            intervention.payload_version,
            OPERATOR_INTERVENTION_EVENT_TYPE,
            intervention.occurred_at,
            intervention.branch_id,
            intervention.session_id,
            intervention.session_name,
            intervention.current_stage,
            intervention.actual_shoot_end_at,
            intervention.extension_status,
            intervention.recent_fault_category,
            intervention.intervention_outcome,
            build_payload_json(
                intervention.payload_version,
                intervention.actual_shoot_end_at.as_deref(),
                None,
                intervention.extension_status.as_deref(),
                intervention.recent_fault_category.as_deref(),
                Some(intervention.intervention_outcome.as_str()),
            )
            .to_string(),
        ],
    )?;

    Ok(())
}

pub fn parse_operator_intervention(
    intervention: Value,
) -> Result<OperatorInterventionWrite, OperationalLogError> {
    let intervention: OperatorInterventionWrite = serde_json::from_value(intervention)?;
    validate_operator_intervention(&intervention)?;
    Ok(intervention)
}

#[tauri::command]
pub fn record_operator_intervention(
    state: State<'_, OperationalLogState>,
    intervention: Value,
) -> Result<(), OperationalLogError> {
    let intervention = parse_operator_intervention(intervention)?;
    let connection = open_operational_log_connection(state.db_path())?;
    insert_operator_intervention(&connection, &intervention)
}

fn validate_operator_intervention(intervention: &OperatorInterventionWrite) -> Result<(), OperationalLogError> {
    validate_payload_version(intervention.payload_version)?;
    validate_timestamp("occurredAt", &intervention.occurred_at)?;
    validate_required_text("branchId", &intervention.branch_id, 120)?;
    validate_required_text("currentStage", &intervention.current_stage, 80)?;
    validate_optional_text("sessionId", intervention.session_id.as_deref(), 120)?;
    validate_optional_text("sessionName", intervention.session_name.as_deref(), 160)?;
    validate_optional_timestamp("actualShootEndAt", intervention.actual_shoot_end_at.as_deref())?;
    validate_optional_text("extensionStatus", intervention.extension_status.as_deref(), 80)?;
    validate_optional_text(
        "recentFaultCategory",
        intervention.recent_fault_category.as_deref(),
        120,
    )?;
    validate_required_text("interventionOutcome", &intervention.intervention_outcome, 120)?;

    Ok(())
}
