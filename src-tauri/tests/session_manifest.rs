use std::{
  fs,
  path::PathBuf,
  time::{SystemTime, UNIX_EPOCH},
};

use boothy_lib::{
  contracts::dto::SessionStartInputDto,
  session::{
    session_manifest::{SessionManifest, SESSION_MANIFEST_SCHEMA_VERSION},
    session_paths::SessionPaths,
    session_repository::start_session_in_dir,
  },
};

fn unique_test_root(test_name: &str) -> PathBuf {
  let stamp = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_nanos();

  std::env::temp_dir().join(format!("boothy-{test_name}-{stamp}"))
}

#[test]
fn creates_session_root_and_manifest_for_valid_input() {
  let base_dir = unique_test_root("valid-start");
  let result = start_session_in_dir(
    &base_dir,
    SessionStartInputDto {
      name: " Kim  Noah ".into(),
      phone_last_four: "4821".into(),
    },
  )
  .expect("valid input should create a session");

  let paths = SessionPaths::new(&base_dir, &result.session_id);
  let manifest_bytes =
    fs::read_to_string(&paths.manifest_path).expect("manifest should be written");
  let manifest: SessionManifest =
    serde_json::from_str(&manifest_bytes).expect("manifest should deserialize");

  assert_eq!(result.booth_alias, "Kim Noah 4821");
  assert_eq!(manifest.schema_version, SESSION_MANIFEST_SCHEMA_VERSION);
  assert_eq!(manifest.session_id, result.session_id);
  assert_eq!(manifest.booth_alias, result.booth_alias);
  assert_eq!(manifest.customer.name, "Kim Noah");
  assert_eq!(manifest.customer.phone_last_four, "4821");
  assert_eq!(manifest.lifecycle.status, "active");
  assert_eq!(manifest.lifecycle.stage, "session-started");
  assert!(manifest.captures.is_empty());
  assert!(manifest.post_end.is_none());
  assert!(paths.captures_originals_dir.exists());
  assert!(paths.renders_previews_dir.exists());
  assert!(paths.renders_finals_dir.exists());
  assert!(paths.handoff_dir.exists());
  assert!(paths.diagnostics_dir.exists());

  let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn rejects_invalid_input_without_creating_durable_session_artifacts() {
  let base_dir = unique_test_root("invalid-start");
  let result = start_session_in_dir(
    &base_dir,
    SessionStartInputDto {
      name: "   ".into(),
      phone_last_four: "12a4".into(),
    },
  );

  let error = result.expect_err("invalid input should fail");

  assert_eq!(error.code, "validation-error");
  assert!(error.field_errors.is_some());
  assert!(!base_dir.join("sessions").exists());

  let _ = fs::remove_dir_all(base_dir);
}
