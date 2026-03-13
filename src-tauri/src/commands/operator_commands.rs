use crate::{
    contracts::dto::{
        ExtendSessionTimingPayload, SessionTimingEnvelope, SessionTimingFailureEnvelope, SessionTimingResult,
        SessionTimingSuccessEnvelope,
    },
    session::session_repository::extend_session_timing as extend_session_timing_record,
};

#[tauri::command]
pub fn extend_session_timing(payload: ExtendSessionTimingPayload) -> SessionTimingEnvelope {
    match extend_session_timing_record(
        std::path::Path::new(&payload.manifest_path),
        &payload.session_id,
        &payload.updated_at,
    ) {
        Ok(record) => SessionTimingEnvelope::Success(SessionTimingSuccessEnvelope {
            ok: true,
            value: SessionTimingResult {
                session_id: record.session_id,
                manifest_path: record.manifest_path,
                timing: record.timing,
            },
        }),
        Err(error) => {
            let error_code = if error.code == "diagnostics.invalidPayload" && error.message.contains("not found") {
                "session_timing.not_found"
            } else if error.code == "diagnostics.invalidPayload" {
                "session_timing.invalid_payload"
            } else {
                "session_timing.persistence_failed"
            };

            SessionTimingEnvelope::Failure(SessionTimingFailureEnvelope {
                ok: false,
                error_code: error_code.into(),
                message: error.message,
            })
        }
    }
}
