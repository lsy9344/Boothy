use chrono::{DateTime, Duration, Local, SecondsFormat, Timelike, Utc};

use crate::diagnostics::error::OperationalLogError;
use crate::session::session_manifest::SessionTiming;

fn parse_utc_timestamp(value: &str) -> Result<DateTime<Utc>, OperationalLogError> {
    DateTime::parse_from_rfc3339(value)
        .map(|parsed| parsed.with_timezone(&Utc))
        .map_err(|error| OperationalLogError::invalid_payload(format!("invalid ISO timestamp: {error}")))
}

pub fn resolve_reservation_start_at(created_at: &str) -> Result<String, OperationalLogError> {
    let local_timestamp = parse_utc_timestamp(created_at)?.with_timezone(&Local);
    let reservation_start = local_timestamp
        .with_minute(0)
        .and_then(|value| value.with_second(0))
        .and_then(|value| value.with_nanosecond(0))
        .ok_or_else(|| OperationalLogError::invalid_payload("failed to resolve reservationStartAt"))?;

    Ok(reservation_start
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Millis, true))
}

pub fn calculate_authoritative_shoot_end_at(
    reservation_start_at: &str,
    session_type: &str,
) -> Result<String, OperationalLogError> {
    let reservation_start = parse_utc_timestamp(reservation_start_at)?;
    let duration_minutes = match session_type {
        "standard" => 50,
        "couponExtended" => 100,
        _ => {
            return Err(OperationalLogError::invalid_payload(format!(
                "unsupported sessionType: {session_type}"
            )))
        }
    };

    Ok((reservation_start + Duration::minutes(duration_minutes))
        .to_rfc3339_opts(SecondsFormat::Millis, true))
}

pub fn create_session_timing_state(
    reservation_start_at: &str,
    session_type: &str,
    updated_at: &str,
) -> Result<SessionTiming, OperationalLogError> {
    let last_timing_update_at = parse_utc_timestamp(updated_at)?.to_rfc3339_opts(SecondsFormat::Millis, true);

    Ok(SessionTiming {
        reservation_start_at: reservation_start_at.into(),
        actual_shoot_end_at: calculate_authoritative_shoot_end_at(reservation_start_at, session_type)?,
        session_type: session_type.into(),
        operator_extension_count: 0,
        last_timing_update_at,
    })
}
