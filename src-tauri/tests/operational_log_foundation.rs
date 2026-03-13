use std::{
    env,
    fs,
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

use boothy_lib::{
    db::{
        migrations::apply_pending_migrations,
        sqlite::{open_operational_log_connection, resolve_operational_log_db_path},
    },
    diagnostics::{
        lifecycle_log::{
            insert_lifecycle_event, parse_lifecycle_event, LifecycleEventKind, LifecycleEventWrite,
        },
        operator_log::{insert_operator_intervention, OperatorInterventionWrite},
    },
};
use rusqlite::Row;
use serde_json::json;

fn unique_temp_dir(test_name: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let path = env::temp_dir().join(format!(
        "boothy-{test_name}-{}-{unique_suffix}",
        process::id()
    ));
    fs::create_dir_all(&path).expect("temp test dir should be created");
    path
}

fn query_single_count(path: &Path, sql: &str) -> i64 {
    let connection = open_operational_log_connection(path).expect("database should open");
    connection
        .query_row(sql, [], |row: &Row<'_>| row.get::<_, i64>(0))
        .expect("query should return a single count")
}

#[test]
fn resolves_operational_database_under_app_local_data_directory() {
    let base_dir = PathBuf::from(r"C:\temp\boothy-app-data");

    let db_path = resolve_operational_log_db_path(&base_dir);

    assert_eq!(db_path.parent(), Some(base_dir.as_path()));
    assert_eq!(
        db_path.file_name().and_then(|value: &std::ffi::OsStr| value.to_str()),
        Some("operational-log.sqlite3")
    );
}

#[test]
fn bootstrap_applies_forward_only_migrations_and_enables_expected_pragmas() {
    let db_path = unique_temp_dir("migration-bootstrap").join("operational-log.sqlite3");
    let mut connection = open_operational_log_connection(&db_path).expect("database should open");

    apply_pending_migrations(&mut connection).expect("migrations should apply");

    let journal_mode = connection
        .pragma_query_value(None, "journal_mode", |row: &Row<'_>| row.get::<_, String>(0))
        .expect("journal_mode should be readable");
    let foreign_keys = connection
        .pragma_query_value(None, "foreign_keys", |row: &Row<'_>| row.get::<_, i64>(0))
        .expect("foreign_keys should be readable");
    let migrations_applied = connection
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row: &Row<'_>| {
            row.get::<_, i64>(0)
        })
        .expect("schema_migrations should be queryable");
    let event_indexes = connection
        .query_row(
            "SELECT name FROM sqlite_master WHERE type = 'index' AND name = 'idx_session_events_session_id_occurred_at'",
            [],
            |row: &Row<'_>| row.get::<_, String>(0),
        )
        .expect("session event index should exist");
    let operator_indexes = connection
        .query_row(
            "SELECT name FROM sqlite_master WHERE type = 'index' AND name = 'idx_operator_interventions_session_name_occurred_at'",
            [],
            |row: &Row<'_>| row.get::<_, String>(0),
        )
        .expect("operator intervention index should exist");

    assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
    assert_eq!(foreign_keys, 1);
    assert_eq!(migrations_applied, 1);
    assert_eq!(event_indexes, "idx_session_events_session_id_occurred_at");
    assert_eq!(
        operator_indexes,
        "idx_operator_interventions_session_name_occurred_at"
    );
}

#[test]
fn lifecycle_and_intervention_rows_survive_reopen() {
    let db_path = unique_temp_dir("reopen-durability").join("operational-log.sqlite3");
    let mut connection = open_operational_log_connection(&db_path).expect("database should open");

    apply_pending_migrations(&mut connection).expect("migrations should apply");

    insert_lifecycle_event(
        &connection,
        &LifecycleEventWrite {
            payload_version: 1,
            event_type: LifecycleEventKind::SessionCreated,
            occurred_at: "2026-03-08T12:00:00Z".into(),
            branch_id: "branch-unconfigured".into(),
            session_id: Some("session-001".into()),
            session_name: Some("Session 001".into()),
            current_stage: "customer-start".into(),
            actual_shoot_end_at: None,
            catalog_fallback_reason: None,
            extension_status: None,
            recent_fault_category: None,
        },
    )
    .expect("lifecycle event should insert");
    insert_operator_intervention(
        &connection,
        &OperatorInterventionWrite {
            payload_version: 1,
            occurred_at: "2026-03-08T12:05:00Z".into(),
            branch_id: "branch-unconfigured".into(),
            session_id: Some("session-001".into()),
            session_name: Some("Session 001".into()),
            current_stage: "export-waiting".into(),
            actual_shoot_end_at: Some("2026-03-08T12:04:00Z".into()),
            extension_status: Some("not-extended".into()),
            recent_fault_category: Some("camera-disconnected".into()),
            intervention_outcome: "recovered".into(),
        },
    )
    .expect("operator intervention should insert");

    drop(connection);

    let lifecycle_count = query_single_count(
        &db_path,
        "SELECT COUNT(*) FROM session_events WHERE session_id = 'session-001'",
    );
    let operator_count = query_single_count(
        &db_path,
        "SELECT COUNT(*) FROM operator_interventions WHERE session_name = 'Session 001'",
    );

    assert_eq!(lifecycle_count, 1);
    assert_eq!(operator_count, 1);
}

#[test]
fn rejects_invalid_migration_state_without_mutating_existing_rows() {
    let db_path = unique_temp_dir("invalid-migration-state").join("operational-log.sqlite3");
    let mut connection = open_operational_log_connection(&db_path).expect("database should open");

    apply_pending_migrations(&mut connection).expect("migrations should apply");
    connection
        .execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, datetime('now'))",
            (99_i64, "future.sql"),
        )
        .expect("corrupt migration entry should be inserted");

    let error = apply_pending_migrations(&mut connection).expect_err("invalid state should fail");
    let applied_versions = connection
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row: &Row<'_>| {
            row.get::<_, i64>(0)
        })
        .expect("schema_migrations should remain queryable");

    assert!(error.to_string().contains("invalid migration state"));
    assert_eq!(applied_versions, 2);
}

#[test]
fn rejects_sensitive_fields_before_any_insert_occurs() {
    let result = parse_lifecycle_event(json!({
        "payloadVersion": 1,
        "eventType": "session_created",
        "occurredAt": "2026-03-08T12:00:00Z",
        "branchId": "branch-unconfigured",
        "currentStage": "check-in",
        "sessionId": "session-001",
        "fullPhoneNumber": "010-1234-5678",
        "paymentData": {
            "cardLast4": "1234"
        },
        "rawReservationPayload": {
            "guestName": "Kim"
        }
    }));

    let error = result.expect_err("sensitive fields should be rejected");

    assert_eq!(error.code, "diagnostics.invalidPayload");
}

#[test]
fn rejects_malformed_timestamps_before_insert() {
    let result = parse_lifecycle_event(json!({
        "payloadVersion": 1,
        "eventType": "session_created",
        "occurredAt": "This is not an ISO timestamp",
        "branchId": "branch-unconfigured",
        "currentStage": "check-in",
        "sessionId": "session-001"
    }));

    let error = result.expect_err("malformed timestamps should be rejected");

    assert_eq!(error.code, "diagnostics.invalidPayload");
}

#[test]
fn accepts_and_persists_preset_catalog_fallback_reason_codes() {
    let db_path = unique_temp_dir("preset-catalog-fallback").join("operational-log.sqlite3");
    let mut connection = open_operational_log_connection(&db_path).expect("database should open");

    apply_pending_migrations(&mut connection).expect("migrations should apply");

    let event = parse_lifecycle_event(json!({
        "payloadVersion": 1,
        "eventType": "preset_catalog_fallback",
        "occurredAt": "2026-03-13T12:00:00Z",
        "branchId": "gangnam-main",
        "currentStage": "presetSelection",
        "sessionId": "session-007",
        "catalogFallbackReason": "reordered_catalog"
    }))
    .expect("preset catalog fallback payload should parse");

    insert_lifecycle_event(&connection, &event).expect("preset catalog fallback event should insert");

    let payload_json = connection
        .query_row(
            "SELECT payload_json FROM session_events WHERE event_type = 'preset_catalog_fallback' LIMIT 1",
            [],
            |row: &Row<'_>| row.get::<_, String>(0),
        )
        .expect("payload json should be queryable");

    assert!(payload_json.contains("\"catalogFallbackReason\":\"reordered_catalog\""));
}

#[test]
fn session_event_queries_can_be_ordered_by_occurrence_time() {
    let db_path = unique_temp_dir("ordered-session-query").join("operational-log.sqlite3");
    let mut connection = open_operational_log_connection(&db_path).expect("database should open");

    apply_pending_migrations(&mut connection).expect("migrations should apply");

    insert_lifecycle_event(
        &connection,
        &LifecycleEventWrite {
            payload_version: 1,
            event_type: LifecycleEventKind::SessionCreated,
            occurred_at: "2026-03-08T12:10:00Z".into(),
            branch_id: "branch-unconfigured".into(),
            session_id: Some("session-001".into()),
            session_name: Some("Session 001".into()),
            current_stage: "check-in".into(),
            actual_shoot_end_at: None,
            catalog_fallback_reason: None,
            extension_status: None,
            recent_fault_category: None,
        },
    )
    .expect("session_created should insert");
    insert_lifecycle_event(
        &connection,
        &LifecycleEventWrite {
            payload_version: 1,
            event_type: LifecycleEventKind::FirstScreenDisplayed,
            occurred_at: "2026-03-08T12:00:00Z".into(),
            branch_id: "branch-unconfigured".into(),
            session_id: Some("session-001".into()),
            session_name: Some("Session 001".into()),
            current_stage: "check-in".into(),
            actual_shoot_end_at: None,
            catalog_fallback_reason: None,
            extension_status: None,
            recent_fault_category: None,
        },
    )
    .expect("first_screen_displayed should insert");

    let mut statement = connection
        .prepare(
            "SELECT event_type FROM session_events WHERE session_id = 'session-001' ORDER BY occurred_at ASC",
        )
        .expect("query should prepare");
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query should execute");
    let ordered_events = rows
        .collect::<Result<Vec<_>, _>>()
        .expect("rows should be collected");

    assert_eq!(ordered_events, vec!["first_screen_displayed", "session_created"]);
}
