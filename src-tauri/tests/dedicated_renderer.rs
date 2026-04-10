use std::{
    fs,
    path::PathBuf,
    sync::Once,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use boothy_lib::{
    capture::{
        normalized_state::{get_capture_readiness_in_dir, request_capture_in_dir},
        sidecar_client::{
            read_capture_request_messages, CanonHelperCaptureRequestMessage,
            CANON_HELPER_CAPTURE_ACCEPTED_SCHEMA_VERSION, CANON_HELPER_FILE_ARRIVED_SCHEMA_VERSION,
        },
    },
    contracts::dto::{CaptureReadinessInputDto, CaptureRequestInputDto, SessionStartInputDto},
    preset::{
        default_catalog::ensure_default_preset_catalog_in_dir,
        preset_catalog::resolve_published_preset_catalog_dir,
    },
    render::dedicated_renderer::complete_capture_preview_with_dedicated_renderer_in_dir,
    session::{
        session_manifest::current_timestamp,
        session_paths::SessionPaths,
        session_repository::{select_active_preset_in_dir, start_session_in_dir},
    },
};

static FAKE_DARKTABLE_SETUP: Once = Once::new();

fn unique_test_root(test_name: &str) -> PathBuf {
    ensure_fake_darktable_cli();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    std::env::temp_dir().join(format!("boothy-dedicated-renderer-{test_name}-{stamp}"))
}

fn ensure_fake_darktable_cli() {
    FAKE_DARKTABLE_SETUP.call_once(|| {
        let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("support")
            .join("fake-darktable-cli.cmd");
        std::env::set_var("BOOTHY_DARKTABLE_CLI_BIN", script_path);
        std::env::set_var("BOOTHY_TEST_RENDER_QUEUE_LIMIT", "unbounded");
    });
}

#[test]
fn queue_saturated_dedicated_renderer_submission_falls_back_without_false_ready() {
    let base_dir = unique_test_root("queue-saturated-fallback");
    ensure_default_preset_catalog_in_dir(&base_dir).expect("default catalog should exist");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(&catalog_root);
    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
        },
    )
    .expect("preset should become active");

    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    let readiness_before = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("readiness should resolve before preview completion");

    assert_eq!(readiness_before.reason_code, "preview-waiting");
    assert_eq!(
        readiness_before
            .latest_capture
            .as_ref()
            .and_then(|value| value.preview.ready_at_ms),
        None
    );

    let env_guard = ScopedEnvVarGuard::set("BOOTHY_TEST_DEDICATED_RENDERER_OUTCOME", "queue-saturated");
    let completed_capture = complete_capture_preview_with_dedicated_renderer_in_dir(
        None,
        &base_dir,
        &session.session_id,
        &capture.capture.capture_id,
    )
    .expect("queue saturation should fall back to the inline truthful renderer");
    drop(env_guard);

    assert_eq!(completed_capture.render_status, "previewReady");
    assert!(completed_capture.preview.ready_at_ms.is_some());
    let canonical_preview_path = SessionPaths::new(&base_dir, &session.session_id)
        .renders_previews_dir
        .join(format!("{}.jpg", capture.capture.capture_id))
        .to_string_lossy()
        .into_owned();
    assert_eq!(
        completed_capture.preview.asset_path.as_deref(),
        Some(canonical_preview_path.as_str())
    );

    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing log should exist");
    assert!(timing_events.contains("event=preview-render-queue-saturated"));
    assert!(timing_events.contains("event=capture_preview_ready"));
    assert!(timing_events.contains("event=capture_preview_transition_summary"));
    assert!(timing_events.contains("laneOwner=inline-truthful-fallback"));
    assert!(timing_events.contains("fallbackReason=render-queue-saturated"));

    let readiness_after = get_capture_readiness_in_dir(
        &base_dir,
        CaptureReadinessInputDto {
            session_id: session.session_id.clone(),
        },
    )
    .expect("readiness should resolve after fallback completion");
    assert_eq!(readiness_after.reason_code, "ready");
    assert_eq!(readiness_after.surface_state, "previewReady");

    let _ = fs::remove_dir_all(base_dir);
}

#[test]
fn accepted_dedicated_renderer_result_claims_truthful_close_without_inline_overwrite() {
    let base_dir = unique_test_root("accepted-close-owner");
    ensure_default_preset_catalog_in_dir(&base_dir).expect("default catalog should exist");
    let session = start_session_in_dir(
        &base_dir,
        SessionStartInputDto {
            name: "Kim".into(),
            phone_last_four: "4821".into(),
        },
    )
    .expect("session should be created");
    let catalog_root = resolve_published_preset_catalog_dir(&base_dir);

    create_published_bundle(&catalog_root);
    select_active_preset_in_dir(
        &base_dir,
        boothy_lib::contracts::dto::PresetSelectionInputDto {
            session_id: session.session_id.clone(),
            preset_id: "preset_soft-glow".into(),
            published_version: "2026.04.10".into(),
        },
    )
    .expect("preset should become active");

    let capture = request_capture_with_helper_success(&base_dir, &session.session_id);
    let canonical_preview_path = SessionPaths::new(&base_dir, &session.session_id)
        .renders_previews_dir
        .join(format!("{}.jpg", capture.capture.capture_id));
    let dedicated_renderer_bytes = [0xFF, 0xD8, 0xFF, 0xD9];
    fs::write(&canonical_preview_path, dedicated_renderer_bytes)
        .expect("accepted dedicated renderer output should exist");

    let env_guard = ScopedEnvVarGuard::set("BOOTHY_TEST_DEDICATED_RENDERER_OUTCOME", "accepted");
    let completed_capture = complete_capture_preview_with_dedicated_renderer_in_dir(
        None,
        &base_dir,
        &session.session_id,
        &capture.capture.capture_id,
    )
    .expect("accepted dedicated renderer output should close the truthful preview");
    drop(env_guard);

    assert_eq!(completed_capture.render_status, "previewReady");
    assert!(completed_capture.preview.ready_at_ms.is_some());
    let normalized_canonical_preview_path = canonical_preview_path
        .to_string_lossy()
        .replace('\\', "/");
    assert_eq!(
        completed_capture
            .preview
            .asset_path
            .as_deref()
            .map(|path| path.replace('\\', "/")),
        Some(normalized_canonical_preview_path)
    );
    assert_eq!(
        fs::read(&canonical_preview_path).expect("accepted preview should stay on disk"),
        dedicated_renderer_bytes,
        "accepted dedicated renderer output should not be overwritten by the inline fallback path"
    );
    let timing_events = fs::read_to_string(
        SessionPaths::new(&base_dir, &session.session_id)
            .diagnostics_dir
            .join("timing-events.log"),
    )
    .expect("timing log should exist");
    assert!(timing_events.contains("event=capture_preview_ready"));
    assert!(timing_events.contains("event=capture_preview_transition_summary"));
    assert!(timing_events.contains("laneOwner=dedicated-renderer"));
    assert!(timing_events.contains("fallbackReason=none"));

    let _ = fs::remove_dir_all(base_dir);
}

fn create_published_bundle(catalog_root: &PathBuf) {
    let bundle_dir = catalog_root.join("preset_soft-glow").join("2026.04.10");
    fs::create_dir_all(bundle_dir.join("xmp")).expect("xmp directory should exist");
    fs::write(bundle_dir.join("preview.jpg"), b"preview").expect("preview should exist");
    fs::write(
        bundle_dir.join("xmp").join("template.xmp"),
        concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
            "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">",
            "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">",
            "<rdf:Description xmlns:darktable=\"http://darktable.sf.net/\">",
            "<darktable:history><rdf:Seq><rdf:li><darktable:module>preset_soft-glow</darktable:module></rdf:li></rdf:Seq></darktable:history>",
            "</rdf:Description></rdf:RDF></x:xmpmeta>"
        ),
    )
    .expect("xmp template should exist");

    let bundle = serde_json::json!({
      "schemaVersion": "published-preset-bundle/v1",
      "presetId": "preset_soft-glow",
      "displayName": "Soft Glow",
      "publishedVersion": "2026.04.10",
      "lifecycleStatus": "published",
      "boothStatus": "booth-safe",
      "darktableVersion": "5.4.1",
      "xmpTemplatePath": "xmp/template.xmp",
      "previewProfile": {
        "profileId": "soft-glow-preview",
        "displayName": "Soft Glow Preview",
        "outputColorSpace": "sRGB"
      },
      "finalProfile": {
        "profileId": "soft-glow-final",
        "displayName": "Soft Glow Final",
        "outputColorSpace": "sRGB"
      },
      "preview": {
        "kind": "preview-tile",
        "assetPath": "preview.jpg",
        "altText": "Soft Glow sample portrait"
      }
    });

    fs::write(
        bundle_dir.join("bundle.json"),
        serde_json::to_vec_pretty(&bundle).expect("bundle should serialize"),
    )
    .expect("bundle should be writable");
}

fn request_capture_with_helper_success(
    base_dir: &PathBuf,
    session_id: &str,
) -> boothy_lib::contracts::dto::CaptureRequestResultDto {
    write_ready_helper_status(base_dir, session_id);
    let existing_request_count = read_capture_request_messages(base_dir, session_id)
        .expect("request log should be readable")
        .len();

    let helper_base_dir = base_dir.clone();
    let helper_session_id = session_id.to_string();
    let helper_thread = thread::spawn(move || {
        let request =
            wait_for_latest_capture_request(&helper_base_dir, &helper_session_id, existing_request_count);
        let capture_id = format!("capture_helper_{}", &request.request_id[8..]);
        let raw_path = SessionPaths::new(&helper_base_dir, &helper_session_id)
            .captures_originals_dir
            .join(format!("{capture_id}.jpg"));

        fs::create_dir_all(
            raw_path
                .parent()
                .expect("raw capture path should have a parent directory"),
        )
        .expect("raw capture directory should exist");
        fs::write(&raw_path, b"helper-raw").expect("helper raw should be writable");

        append_helper_event(
            &helper_base_dir,
            &helper_session_id,
            serde_json::json!({
              "schemaVersion": CANON_HELPER_CAPTURE_ACCEPTED_SCHEMA_VERSION,
              "type": "capture-accepted",
              "sessionId": helper_session_id,
              "requestId": request.request_id,
            }),
        );
        append_helper_event(
            &helper_base_dir,
            &helper_session_id,
            serde_json::json!({
              "schemaVersion": CANON_HELPER_FILE_ARRIVED_SCHEMA_VERSION,
              "type": "file-arrived",
              "sessionId": request.session_id,
              "requestId": request.request_id,
              "captureId": capture_id,
              "arrivedAt": current_timestamp(SystemTime::now()).expect("arrival timestamp should serialize"),
              "rawPath": raw_path.to_string_lossy().into_owned(),
            }),
        );
    });

    let result = request_capture_in_dir(
        base_dir,
        CaptureRequestInputDto {
            session_id: session_id.into(),
            request_id: Some("request_20260410_001".into()),
        },
    )
    .expect("capture should save");

    helper_thread
        .join()
        .expect("helper capture thread should complete");

    result
}

fn wait_for_latest_capture_request(
    base_dir: &PathBuf,
    session_id: &str,
    existing_request_count: usize,
) -> CanonHelperCaptureRequestMessage {
    for _ in 0..200 {
        let requests = read_capture_request_messages(base_dir, session_id)
            .expect("capture request log should be readable");

        if requests.len() > existing_request_count {
            if let Some(request) = requests.last() {
                return request.clone();
            }
        }

        thread::sleep(Duration::from_millis(10));
    }

    panic!("capture request should have been written")
}

fn append_helper_event(base_dir: &PathBuf, session_id: &str, event: serde_json::Value) {
    let event_path = SessionPaths::new(base_dir, session_id)
        .diagnostics_dir
        .join("camera-helper-events.jsonl");
    fs::create_dir_all(
        event_path
            .parent()
            .expect("helper event path should have a parent directory"),
    )
    .expect("helper event directory should exist");

    let serialized_event = serde_json::to_string(&event).expect("helper event should serialize");
    let existing = fs::read_to_string(&event_path).unwrap_or_default();
    let next_contents = if existing.trim().is_empty() {
        format!("{serialized_event}\n")
    } else {
        format!("{existing}{serialized_event}\n")
    };

    fs::write(event_path, next_contents).expect("helper event log should be writable");
}

fn write_ready_helper_status(base_dir: &PathBuf, session_id: &str) {
    let status_path = SessionPaths::new(base_dir, session_id)
        .diagnostics_dir
        .join("camera-helper-status.json");
    fs::create_dir_all(
        status_path
            .parent()
            .expect("helper status should have a diagnostics directory"),
    )
    .expect("diagnostics directory should exist");
    fs::write(
        status_path,
        serde_json::to_vec_pretty(&serde_json::json!({
          "schemaVersion": "canon-helper-status/v1",
          "type": "camera-status",
          "sessionId": session_id,
          "sequence": 1,
          "observedAt": current_timestamp(SystemTime::now()).expect("helper timestamp should serialize"),
          "cameraState": "ready",
          "helperState": "healthy"
        }))
        .expect("helper status should serialize"),
    )
    .expect("helper status should be writable");
}

struct ScopedEnvVarGuard {
    key: String,
    original_value: Option<std::ffi::OsString>,
}

impl ScopedEnvVarGuard {
    fn set(key: &str, value: &str) -> Self {
        let original_value = std::env::var_os(key);
        std::env::set_var(key, value);
        Self {
            key: key.into(),
            original_value,
        }
    }
}

impl Drop for ScopedEnvVarGuard {
    fn drop(&mut self) {
        match self.original_value.take() {
            Some(value) => std::env::set_var(&self.key, value),
            None => std::env::remove_var(&self.key),
        }
    }
}
