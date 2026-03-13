use chrono::{DateTime, Duration, SecondsFormat, Utc};

use crate::diagnostics::error::OperationalLogError;
use crate::session::session_manifest::SessionTiming;

fn parse_utc_timestamp(value: &str) -> Result<DateTime<Utc>, OperationalLogError> {
    DateTime::parse_from_rfc3339(value)
        .map(|parsed| parsed.with_timezone(&Utc))
        .map_err(|error| OperationalLogError::invalid_payload(format!("invalid ISO timestamp: {error}")))
}

pub fn apply_operator_session_extension(
    timing: &SessionTiming,
    updated_at: &str,
) -> Result<SessionTiming, OperationalLogError> {
    let actual_shoot_end_at = parse_utc_timestamp(&timing.actual_shoot_end_at)?
        + Duration::minutes(60);
    let last_timing_update_at = parse_utc_timestamp(updated_at)?.to_rfc3339_opts(SecondsFormat::Millis, true);

    Ok(SessionTiming {
        reservation_start_at: timing.reservation_start_at.clone(),
        actual_shoot_end_at: actual_shoot_end_at.to_rfc3339_opts(SecondsFormat::Millis, true),
        session_type: timing.session_type.clone(),
        operator_extension_count: timing.operator_extension_count + 1,
        last_timing_update_at,
    })
}
