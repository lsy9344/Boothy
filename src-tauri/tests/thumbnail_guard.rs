use std::{fs, path::PathBuf};

use boothy_lib::{
    contracts::dto::{DeleteSessionPhotoRequest, SessionGalleryRequest},
    export::thumbnail_guard::ThumbnailGuard,
    session::{
        manifest::{create_session_manifest, ManifestCaptureRecord, SessionManifestDraft},
        session_paths::resolve_session_paths,
        session_repository::SessionRepository,
    },
};

fn write_capture(root: &PathBuf, relative_path: &str) {
    let capture_path = root.join(relative_path);
    fs::create_dir_all(capture_path.parent().expect("capture parent should exist"))
        .expect("capture directory should exist");
    fs::write(capture_path, b"fake-image").expect("capture should be written");
}

fn write_events_artifact(root: &PathBuf, session_id: &str) {
    let events_path = root.join(session_id).join("events.ndjson");
    fs::create_dir_all(events_path.parent().expect("events parent should exist"))
        .expect("events parent should exist");
    fs::write(events_path, "").expect("events artifact should be written");
}

fn build_manifest(
    session_root_base: &str,
    session_id: &str,
    latest_capture_id: Option<&str>,
    captures: Vec<ManifestCaptureRecord>,
) -> boothy_lib::session::manifest::SessionManifest {
    create_session_manifest(SessionManifestDraft {
        session_id: session_id.into(),
        session_name: format!("{session_id}-name"),
        operational_date: "2026-03-08".into(),
        created_at: "2026-03-08T09:00:00.000Z".into(),
        reservation_start_at: "2026-03-08T09:00:00.000Z".into(),
        session_type: "standard".into(),
        capture_revision: 0,
        active_preset_name: Some("Classic Mono".into()),
        active_preset: None,
        latest_capture_id: latest_capture_id.map(str::to_owned),
        captures,
        paths: resolve_session_paths(session_root_base, session_id),
    })
    .expect("manifest should build")
}

#[test]
fn gallery_query_returns_only_manifest_backed_captures_for_the_requested_session() {
    let temp_dir = tempfile::tempdir().expect("temporary directory should exist");
    let repository = SessionRepository::new();
    let guard = ThumbnailGuard::new(repository.clone());

    let manifest = build_manifest(
        temp_dir.path().to_string_lossy().as_ref(),
        "session-001",
        Some("capture-002"),
        vec![
            ManifestCaptureRecord {
                capture_id: "capture-001".into(),
                original_file_name: "originals/capture-001.nef".into(),
                processed_file_name: "capture-001.jpg".into(),
                captured_at: "2026-03-08T09:00:00.000Z".into(),
            },
            ManifestCaptureRecord {
                capture_id: "capture-002".into(),
                original_file_name: "originals/capture-002.nef".into(),
                processed_file_name: "capture-002.jpg".into(),
                captured_at: "2026-03-08T09:02:00.000Z".into(),
            },
        ],
    );

    write_capture(&temp_dir.path().to_path_buf(), "session-001/originals/capture-001.nef");
    write_capture(&temp_dir.path().to_path_buf(), "session-001/originals/capture-002.nef");
    write_capture(&temp_dir.path().to_path_buf(), "session-001/processed/capture-001.jpg");
    write_capture(&temp_dir.path().to_path_buf(), "session-001/processed/capture-002.jpg");
    write_capture(&temp_dir.path().to_path_buf(), "session-999/processed/foreign.jpg");

    repository
        .save_manifest(&manifest.manifest_path, &manifest)
        .expect("manifest should save");

    let gallery = guard
        .load_session_gallery(SessionGalleryRequest {
            session_id: "session-001".into(),
            manifest_path: manifest.manifest_path.clone(),
        })
        .expect("gallery should load");

    assert_eq!(gallery.items.len(), 2);
    assert!(gallery
        .items
        .iter()
        .all(|item| item.session_id == "session-001"));
    assert_eq!(gallery.latest_capture_id.as_deref(), Some("capture-002"));
    assert_eq!(gallery.items[0].display_order, 0);
    assert_eq!(gallery.items[1].display_order, 1);
    assert_eq!(gallery.items[1].capture_id, "capture-002");
}

#[test]
fn delete_rejects_cross_session_and_out_of_root_targets() {
    let temp_dir = tempfile::tempdir().expect("temporary directory should exist");
    let repository = SessionRepository::new();
    let guard = ThumbnailGuard::new(repository.clone());

    let manifest = build_manifest(
        temp_dir.path().to_string_lossy().as_ref(),
        "session-001",
        Some("capture-001"),
        vec![ManifestCaptureRecord {
            capture_id: "capture-001".into(),
            original_file_name: "originals/capture-001.nef".into(),
            processed_file_name: "../session-999/processed/foreign.jpg".into(),
            captured_at: "2026-03-08T09:00:00.000Z".into(),
        }],
    );

    repository
        .save_manifest(&manifest.manifest_path, &manifest)
        .expect("manifest should save");

    let error = guard
        .delete_session_capture(DeleteSessionPhotoRequest {
            session_id: "session-001".into(),
            capture_id: "capture-001".into(),
            manifest_path: manifest.manifest_path.clone(),
        })
        .expect_err("out-of-root capture should fail");

    assert_eq!(error.code.as_str(), "session.capture.out_of_root");
}

#[test]
fn delete_rejects_audit_paths_that_do_not_target_events_ndjson() {
    let temp_dir = tempfile::tempdir().expect("temporary directory should exist");
    let repository = SessionRepository::new();
    let guard = ThumbnailGuard::new(repository.clone());

    let mut manifest = build_manifest(
        temp_dir.path().to_string_lossy().as_ref(),
        "session-001",
        Some("capture-001"),
        vec![ManifestCaptureRecord {
            capture_id: "capture-001".into(),
            original_file_name: "originals/capture-001.nef".into(),
            processed_file_name: "capture-001.jpg".into(),
            captured_at: "2026-03-08T09:00:00.000Z".into(),
        }],
    );
    manifest.events_path = temp_dir
        .path()
        .join("session-001/session.json")
        .to_string_lossy()
        .replace('\\', "/");

    write_capture(&temp_dir.path().to_path_buf(), "session-001/originals/capture-001.nef");
    write_capture(&temp_dir.path().to_path_buf(), "session-001/processed/capture-001.jpg");
    write_events_artifact(&temp_dir.path().to_path_buf(), "session-001");

    repository
        .save_manifest(&manifest.manifest_path, &manifest)
        .expect("manifest should save");

    let error = guard
        .delete_session_capture(DeleteSessionPhotoRequest {
            session_id: "session-001".into(),
            capture_id: "capture-001".into(),
            manifest_path: manifest.manifest_path.clone(),
        })
        .expect_err("non-events audit target should fail");

    assert_eq!(error.code.as_str(), "session.manifest.invalid");
}

#[test]
fn delete_requires_the_existing_events_ndjson_artifact_before_appending_audit_history() {
    let temp_dir = tempfile::tempdir().expect("temporary directory should exist");
    let repository = SessionRepository::new();
    let guard = ThumbnailGuard::new(repository.clone());

    let manifest = build_manifest(
        temp_dir.path().to_string_lossy().as_ref(),
        "session-001",
        Some("capture-001"),
        vec![ManifestCaptureRecord {
            capture_id: "capture-001".into(),
            original_file_name: "originals/capture-001.nef".into(),
            processed_file_name: "capture-001.jpg".into(),
            captured_at: "2026-03-08T09:00:00.000Z".into(),
        }],
    );

    write_capture(&temp_dir.path().to_path_buf(), "session-001/originals/capture-001.nef");
    write_capture(&temp_dir.path().to_path_buf(), "session-001/processed/capture-001.jpg");
    write_events_artifact(&temp_dir.path().to_path_buf(), "session-001");

    repository
        .save_manifest(&manifest.manifest_path, &manifest)
        .expect("manifest should save");

    fs::remove_file(temp_dir.path().join("session-001/events.ndjson"))
        .expect("events artifact should be removed to reproduce integrity loss");

    let error = guard
        .delete_session_capture(DeleteSessionPhotoRequest {
            session_id: "session-001".into(),
            capture_id: "capture-001".into(),
            manifest_path: manifest.manifest_path.clone(),
        })
        .expect_err("missing events artifact should fail");

    assert_eq!(error.code.as_str(), "session.manifest.invalid");
    assert!(temp_dir
        .path()
        .join("session-001/originals/capture-001.nef")
        .exists());
    assert!(temp_dir
        .path()
        .join("session-001/processed/capture-001.jpg")
        .exists());
}

#[test]
fn delete_reconciles_latest_capture_and_selection_after_success() {
    let temp_dir = tempfile::tempdir().expect("temporary directory should exist");
    let repository = SessionRepository::new();
    let guard = ThumbnailGuard::new(repository.clone());

    let manifest = build_manifest(
        temp_dir.path().to_string_lossy().as_ref(),
        "session-001",
        Some("capture-002"),
        vec![
            ManifestCaptureRecord {
                capture_id: "capture-001".into(),
                original_file_name: "originals/capture-001.nef".into(),
                processed_file_name: "capture-001.jpg".into(),
                captured_at: "2026-03-08T09:00:00.000Z".into(),
            },
            ManifestCaptureRecord {
                capture_id: "capture-002".into(),
                original_file_name: "originals/capture-002.nef".into(),
                processed_file_name: "capture-002.jpg".into(),
                captured_at: "2026-03-08T09:02:00.000Z".into(),
            },
        ],
    );

    write_capture(&temp_dir.path().to_path_buf(), "session-001/originals/capture-001.nef");
    write_capture(&temp_dir.path().to_path_buf(), "session-001/originals/capture-002.nef");
    write_capture(&temp_dir.path().to_path_buf(), "session-001/processed/capture-001.jpg");
    write_capture(&temp_dir.path().to_path_buf(), "session-001/processed/capture-002.jpg");
    write_events_artifact(&temp_dir.path().to_path_buf(), "session-001");

    repository
        .save_manifest(&manifest.manifest_path, &manifest)
        .expect("manifest should save");

    let response = guard
        .delete_session_capture(DeleteSessionPhotoRequest {
            session_id: "session-001".into(),
            capture_id: "capture-002".into(),
            manifest_path: manifest.manifest_path.clone(),
        })
        .expect("delete should succeed");

    assert_eq!(response.deleted_capture_id, "capture-002");
    assert_eq!(response.confirmation_message, "사진이 삭제되었습니다.");
    assert_eq!(response.gallery.latest_capture_id.as_deref(), Some("capture-001"));
    assert_eq!(response.gallery.selected_capture_id.as_deref(), Some("capture-001"));
    assert_eq!(response.gallery.items.len(), 1);
    assert!(!temp_dir
        .path()
        .join("session-001/originals/capture-002.nef")
        .exists());
    assert!(!temp_dir
        .path()
        .join("session-001/processed/capture-002.jpg")
        .exists());

    let events = fs::read_to_string(temp_dir.path().join("session-001/events.ndjson"))
        .expect("events file should be readable");
    let event: serde_json::Value = serde_json::from_str(
        events
            .lines()
            .last()
            .expect("delete should append one audit event"),
    )
    .expect("audit event should parse");

    assert_eq!(event["eventType"], "photo_deleted");
    assert_eq!(event["sessionId"], "session-001");
    assert_eq!(event["captureId"], "capture-002");
    assert!(event["occurredAt"].as_str().is_some());
}

#[test]
fn delete_selects_the_nearest_remaining_capture_after_removing_the_current_selection() {
    let temp_dir = tempfile::tempdir().expect("temporary directory should exist");
    let repository = SessionRepository::new();
    let guard = ThumbnailGuard::new(repository.clone());

    let manifest = build_manifest(
        temp_dir.path().to_string_lossy().as_ref(),
        "session-001",
        Some("capture-003"),
        vec![
            ManifestCaptureRecord {
                capture_id: "capture-001".into(),
                original_file_name: "originals/capture-001.nef".into(),
                processed_file_name: "capture-001.jpg".into(),
                captured_at: "2026-03-08T09:00:00.000Z".into(),
            },
            ManifestCaptureRecord {
                capture_id: "capture-002".into(),
                original_file_name: "originals/capture-002.nef".into(),
                processed_file_name: "capture-002.jpg".into(),
                captured_at: "2026-03-08T09:02:00.000Z".into(),
            },
            ManifestCaptureRecord {
                capture_id: "capture-003".into(),
                original_file_name: "originals/capture-003.nef".into(),
                processed_file_name: "capture-003.jpg".into(),
                captured_at: "2026-03-08T09:04:00.000Z".into(),
            },
        ],
    );

    write_capture(&temp_dir.path().to_path_buf(), "session-001/originals/capture-001.nef");
    write_capture(&temp_dir.path().to_path_buf(), "session-001/originals/capture-002.nef");
    write_capture(&temp_dir.path().to_path_buf(), "session-001/originals/capture-003.nef");
    write_capture(&temp_dir.path().to_path_buf(), "session-001/processed/capture-001.jpg");
    write_capture(&temp_dir.path().to_path_buf(), "session-001/processed/capture-002.jpg");
    write_capture(&temp_dir.path().to_path_buf(), "session-001/processed/capture-003.jpg");
    write_events_artifact(&temp_dir.path().to_path_buf(), "session-001");

    repository
        .save_manifest(&manifest.manifest_path, &manifest)
        .expect("manifest should save");

    let response = guard
        .delete_session_capture(DeleteSessionPhotoRequest {
            session_id: "session-001".into(),
            capture_id: "capture-002".into(),
            manifest_path: manifest.manifest_path.clone(),
        })
        .expect("delete should succeed");

    assert_eq!(response.gallery.selected_capture_id.as_deref(), Some("capture-003"));
    assert_eq!(response.gallery.latest_capture_id.as_deref(), Some("capture-003"));
    assert_eq!(response.gallery.items.len(), 2);
}

#[test]
fn delete_rejects_stale_capture_ids_without_mutating_the_active_session() {
    let temp_dir = tempfile::tempdir().expect("temporary directory should exist");
    let repository = SessionRepository::new();
    let guard = ThumbnailGuard::new(repository.clone());

    let manifest = build_manifest(
        temp_dir.path().to_string_lossy().as_ref(),
        "session-001",
        Some("capture-001"),
        vec![ManifestCaptureRecord {
            capture_id: "capture-001".into(),
            original_file_name: "originals/capture-001.nef".into(),
            processed_file_name: "capture-001.jpg".into(),
            captured_at: "2026-03-08T09:00:00.000Z".into(),
        }],
    );

    write_capture(&temp_dir.path().to_path_buf(), "session-001/originals/capture-001.nef");
    write_capture(&temp_dir.path().to_path_buf(), "session-001/processed/capture-001.jpg");
    write_events_artifact(&temp_dir.path().to_path_buf(), "session-001");

    repository
        .save_manifest(&manifest.manifest_path, &manifest)
        .expect("manifest should save");

    let error = guard
        .delete_session_capture(DeleteSessionPhotoRequest {
            session_id: "session-001".into(),
            capture_id: "capture-999".into(),
            manifest_path: manifest.manifest_path.clone(),
        })
        .expect_err("stale capture id should fail");

    assert_eq!(error.code.as_str(), "session.capture.not_found");
    assert!(temp_dir
        .path()
        .join("session-001/originals/capture-001.nef")
        .exists());
    assert!(temp_dir
        .path()
        .join("session-001/processed/capture-001.jpg")
        .exists());

    let persisted_manifest = repository
        .load_manifest(&manifest.manifest_path)
        .expect("manifest should remain readable");
    assert_eq!(persisted_manifest.captures.len(), 1);
    assert_eq!(persisted_manifest.latest_capture_id.as_deref(), Some("capture-001"));
}

#[test]
fn delete_restores_the_capture_file_when_manifest_persistence_fails() {
    let temp_dir = tempfile::tempdir().expect("temporary directory should exist");
    let repository = SessionRepository::new();
    let guard = ThumbnailGuard::new(repository.clone());

    let manifest = build_manifest(
        temp_dir.path().to_string_lossy().as_ref(),
        "session-001",
        Some("capture-001"),
        vec![ManifestCaptureRecord {
            capture_id: "capture-001".into(),
            original_file_name: "originals/capture-001.nef".into(),
            processed_file_name: "capture-001.jpg".into(),
            captured_at: "2026-03-08T09:00:00.000Z".into(),
        }],
    );

    let original_path = temp_dir.path().join("session-001/originals/capture-001.nef");
    let capture_path = temp_dir.path().join("session-001/processed/capture-001.jpg");
    write_capture(&temp_dir.path().to_path_buf(), "session-001/originals/capture-001.nef");
    write_capture(&temp_dir.path().to_path_buf(), "session-001/processed/capture-001.jpg");
    write_events_artifact(&temp_dir.path().to_path_buf(), "session-001");

    repository
        .save_manifest(&manifest.manifest_path, &manifest)
        .expect("manifest should save");

    let mut readonly_permissions = fs::metadata(&manifest.manifest_path)
        .expect("manifest metadata should exist")
        .permissions();
    readonly_permissions.set_readonly(true);
    fs::set_permissions(&manifest.manifest_path, readonly_permissions)
        .expect("manifest should become readonly");

    let error = guard
        .delete_session_capture(DeleteSessionPhotoRequest {
            session_id: "session-001".into(),
            capture_id: "capture-001".into(),
            manifest_path: manifest.manifest_path.clone(),
        })
        .expect_err("manifest persistence failure should roll back the delete");

    assert_eq!(error.code.as_str(), "session.manifest.invalid");
    assert!(original_path.exists());
    assert!(capture_path.exists());

    let mut writable_permissions = fs::metadata(&manifest.manifest_path)
        .expect("manifest metadata should still exist")
        .permissions();
    writable_permissions.set_readonly(false);
    fs::set_permissions(&manifest.manifest_path, writable_permissions)
        .expect("manifest permissions should be restored for cleanup");

    let persisted_manifest = repository
        .load_manifest(&manifest.manifest_path)
        .expect("manifest should remain unchanged");
    assert_eq!(persisted_manifest.captures.len(), 1);
    assert_eq!(persisted_manifest.latest_capture_id.as_deref(), Some("capture-001"));
}
