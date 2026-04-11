use std::{fs, path::PathBuf};

use boothy_lib::{
    capture::sidecar_client::{
        CanonHelperErrorMessage, CanonHelperFileArrivedMessage, CanonHelperReadyMessage,
        CanonHelperRecoveryStatusMessage, CanonHelperStatusMessage,
    },
    contracts::dto::{
        CapabilitySnapshotDto, DedicatedRendererPreviewJobRequestDto,
        DedicatedRendererPreviewJobResultDto, HostErrorEnvelope,
    },
    preset::preset_bundle::load_published_preset_runtime_bundle,
    session::session_manifest::SessionManifest,
};
use serde_json::Value;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root should exist")
        .to_path_buf()
}

fn read_json_fixture(relative_path: &str) -> String {
    let path = repo_root().join(relative_path);
    fs::read_to_string(path).expect("fixture should be readable")
}

#[test]
fn parses_frozen_contract_fixtures_shared_with_typescript() {
    let manifest_fixture = read_json_fixture("tests/fixtures/contracts/session-manifest-v1.json");
    let error_fixture =
        read_json_fixture("tests/fixtures/contracts/host-error-envelope-capture-not-ready.json");
    let capability_fixture =
        read_json_fixture("tests/fixtures/contracts/runtime-capability-authoring-enabled.json");

    let manifest: SessionManifest =
        serde_json::from_str(&manifest_fixture).expect("manifest fixture should deserialize");
    let error: HostErrorEnvelope =
        serde_json::from_str(&error_fixture).expect("host error fixture should deserialize");
    let capability_snapshot: CapabilitySnapshotDto =
        serde_json::from_str(&capability_fixture).expect("capability fixture should deserialize");

    assert_eq!(manifest.schema_version, "session-manifest/v1");
    assert_eq!(
        manifest
            .active_preset
            .as_ref()
            .map(|preset| preset.published_version.as_str()),
        Some("2026.04.10")
    );
    assert_eq!(error.code, "capture-not-ready");
    assert_eq!(
        capability_snapshot.allowed_surfaces,
        vec!["booth", "operator", "authoring", "settings"]
    );
}

#[test]
fn parses_frozen_helper_protocol_examples() {
    let camera_status_fixture = read_json_fixture("sidecar/protocol/examples/camera-status.json");
    let file_arrived_fixture = read_json_fixture("sidecar/protocol/examples/file-arrived.json");
    let helper_ready_fixture = read_json_fixture("sidecar/protocol/examples/helper-ready.json");
    let recovery_status_fixture =
        read_json_fixture("sidecar/protocol/examples/recovery-status.json");
    let helper_error_fixture = read_json_fixture("sidecar/protocol/examples/helper-error.json");

    let camera_status: CanonHelperStatusMessage = serde_json::from_str(&camera_status_fixture)
        .expect("camera-status fixture should deserialize");
    let file_arrived: CanonHelperFileArrivedMessage = serde_json::from_str(&file_arrived_fixture)
        .expect("file-arrived fixture should deserialize");
    let helper_ready: CanonHelperReadyMessage = serde_json::from_str(&helper_ready_fixture)
        .expect("helper-ready fixture should deserialize");
    let recovery_status: CanonHelperRecoveryStatusMessage =
        serde_json::from_str(&recovery_status_fixture)
            .expect("recovery-status fixture should deserialize");
    let helper_error: CanonHelperErrorMessage = serde_json::from_str(&helper_error_fixture)
        .expect("helper-error fixture should deserialize");

    assert_eq!(camera_status.schema_version, "canon-helper-status/v1");
    assert_eq!(camera_status.message_type.as_deref(), Some("camera-status"));
    assert_eq!(file_arrived.schema_version, "canon-helper-file-arrived/v1");
    assert_eq!(file_arrived.message_type, "file-arrived");
    assert_eq!(file_arrived.request_id, "request_20260410_001");
    assert_eq!(helper_ready.schema_version, "canon-helper-ready/v1");
    assert_eq!(helper_ready.message_type, "helper-ready");
    assert_eq!(helper_ready.sdk_family.as_deref(), Some("canon-edsdk"));
    assert_eq!(helper_ready.protocol_version.as_deref(), Some("v1"));
    assert_eq!(
        recovery_status.schema_version,
        "canon-helper-recovery-status/v1"
    );
    assert_eq!(recovery_status.message_type, "recovery-status");
    assert_eq!(recovery_status.recovery_state, "recovering");
    assert_eq!(helper_error.schema_version, "canon-helper-error/v1");
    assert_eq!(helper_error.message_type, "helper-error");
    assert_eq!(helper_error.detail_code, "capture-download-timeout");
}

#[test]
fn loads_the_frozen_published_preset_bundle_fixture() {
    let bundle_dir =
        repo_root().join("tests/fixtures/contracts/preset-bundle-v1/preset_soft-glow/2026.04.10");
    let bundle = load_published_preset_runtime_bundle(&bundle_dir)
        .expect("published preset bundle fixture should load");

    assert_eq!(bundle.preset_id, "preset_soft-glow");
    assert_eq!(bundle.published_version, "2026.04.10");
    assert_eq!(bundle.preview_profile.profile_id, "soft-glow-preview");
    assert_eq!(bundle.final_profile.profile_id, "soft-glow-final");
}

#[test]
fn parses_dedicated_renderer_preview_protocol_examples() {
    let request_fixture =
        read_json_fixture("sidecar/protocol/examples/preview-render-request.json");
    let result_fixture = read_json_fixture("sidecar/protocol/examples/preview-render-result.json");

    let request: DedicatedRendererPreviewJobRequestDto = serde_json::from_str(&request_fixture)
        .expect("preview-render-request fixture should deserialize");
    let result: DedicatedRendererPreviewJobResultDto = serde_json::from_str(&result_fixture)
        .expect("preview-render-result fixture should deserialize");

    assert_eq!(
        request.schema_version,
        "dedicated-renderer-preview-job-request/v1"
    );
    assert_eq!(request.capture_id, "capture_20260410_001");
    assert_eq!(request.preset_id, "preset_soft-glow");
    assert_eq!(
        result.schema_version,
        "dedicated-renderer-preview-job-result/v1"
    );
    assert_eq!(result.status, "fallback-suggested");
    assert_eq!(result.request_id, "request_20260410_001");
}

#[test]
fn tauri_packaging_freezes_the_dedicated_renderer_sidecar_boundary() {
    let cargo_toml = fs::read_to_string(repo_root().join("src-tauri/Cargo.toml"))
        .expect("Cargo.toml should be readable");
    let tauri_conf: Value = serde_json::from_str(
        &fs::read_to_string(repo_root().join("src-tauri/tauri.conf.json"))
            .expect("tauri.conf.json should be readable"),
    )
    .expect("tauri.conf.json should parse");
    let booth_capability: Value = serde_json::from_str(
        &fs::read_to_string(repo_root().join("src-tauri/capabilities/booth-window.json"))
            .expect("booth capability should be readable"),
    )
    .expect("booth capability should parse");

    assert!(
        cargo_toml.contains("tauri-plugin-shell"),
        "Cargo.toml should include the shell plugin for dedicated renderer launch"
    );

    let external_bins = tauri_conf
        .get("bundle")
        .and_then(|bundle| bundle.get("externalBin"))
        .and_then(Value::as_array)
        .expect("bundle.externalBin should exist");
    assert!(
        external_bins
            .iter()
            .any(|value| value == "../sidecar/dedicated-renderer/boothy-dedicated-renderer"),
        "tauri bundle should include the dedicated renderer sidecar binary"
    );

    let permissions = booth_capability
        .get("permissions")
        .and_then(Value::as_array)
        .expect("capability permissions should be an array");
    assert!(
        permissions.iter().any(|entry| {
            entry.get("identifier").and_then(Value::as_str) == Some("shell:allow-execute")
        }),
        "booth capability should freeze the dedicated renderer execute allowlist"
    );
}
