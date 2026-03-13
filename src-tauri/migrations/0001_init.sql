CREATE TABLE session_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    payload_version INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    branch_id TEXT NOT NULL,
    session_id TEXT,
    session_name TEXT,
    current_stage TEXT NOT NULL,
    actual_shoot_end_at TEXT,
    extension_status TEXT,
    recent_fault_category TEXT,
    payload_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_session_events_session_id_occurred_at
    ON session_events (session_id, occurred_at);

CREATE INDEX idx_session_events_session_name_occurred_at
    ON session_events (session_name, occurred_at);

CREATE TABLE operator_interventions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    payload_version INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    branch_id TEXT NOT NULL,
    session_id TEXT,
    session_name TEXT,
    current_stage TEXT NOT NULL,
    actual_shoot_end_at TEXT,
    extension_status TEXT,
    recent_fault_category TEXT,
    intervention_outcome TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_operator_interventions_session_id_occurred_at
    ON operator_interventions (session_id, occurred_at);

CREATE INDEX idx_operator_interventions_session_name_occurred_at
    ON operator_interventions (session_name, occurred_at);
