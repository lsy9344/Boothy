use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const DARKTABLE_CLI_BIN_ENV: &str = "BOOTHY_DARKTABLE_CLI_BIN";
const RAW_PREVIEW_MAX_WIDTH_PX: u32 = 1024;
const RAW_PREVIEW_MAX_HEIGHT_PX: u32 = 1024;
const FAST_PREVIEW_RENDER_MAX_WIDTH_PX: u32 = 512;
const FAST_PREVIEW_RENDER_MAX_HEIGHT_PX: u32 = 512;
const DARKTABLE_APPLY_CUSTOM_PRESETS_DISABLED: &str = "false";
const DARKTABLE_PREVIEW_LIBRARY_IN_MEMORY: &str = ":memory:";
const FAST_PREVIEW_SOURCE_WAIT_POLL_MS: u64 = 25;
const FAST_PREVIEW_SOURCE_WAIT_BUDGET_MS: u64 = 500;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let request_json = fs::read_to_string(&args.request_path)
        .map_err(|error| format!("failed to read request: {error}"))?;

    match args.protocol.as_str() {
        "warmup-v1" => handle_warmup(&request_json, &args.result_path),
        "preview-job-v1" => handle_preview(&request_json, &args.result_path),
        other => Err(format!("unsupported protocol: {other}")),
    }
}

struct SidecarArgs {
    protocol: String,
    request_path: PathBuf,
    result_path: PathBuf,
}

fn parse_args() -> Result<SidecarArgs, String> {
    let mut protocol = None;
    let mut request_path = None;
    let mut result_path = None;
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--protocol" => protocol = args.next(),
            "--request" => request_path = args.next().map(PathBuf::from),
            "--result" => result_path = args.next().map(PathBuf::from),
            _ => {}
        }
    }

    Ok(SidecarArgs {
        protocol: protocol.ok_or_else(|| "missing --protocol".to_string())?,
        request_path: request_path.ok_or_else(|| "missing --request".to_string())?,
        result_path: result_path.ok_or_else(|| "missing --result".to_string())?,
    })
}

fn handle_warmup(request_json: &str, result_path: &Path) -> Result<(), String> {
    let session_id = extract_json_string(request_json, "sessionId")?;
    let preset_id = extract_json_string(request_json, "presetId")?;
    let published_version = extract_json_string(request_json, "publishedVersion")?;
    let request_detail_path = extract_json_string(request_json, "diagnosticsDetailPath")?;
    let warm_state_path = warm_state_detail_path(
        Path::new(&request_detail_path),
        &preset_id,
        &published_version,
    )?;

    write_warm_state_file(
        &warm_state_path,
        &session_id,
        &preset_id,
        &published_version,
        "warm-ready",
    )?;
    write_json_file(
        result_path,
        &build_warmup_result_json(
            result_path,
            &session_id,
            &preset_id,
            &published_version,
            "warmed-up",
            Some("renderer-warm"),
            Some("resident prototype warm state가 준비됐어요."),
            Some("warm-ready"),
            Some(path_to_runtime_string(&warm_state_path)),
        ),
    )?;

    Ok(())
}

fn handle_preview(request_json: &str, result_path: &Path) -> Result<(), String> {
    let session_id = extract_json_string(request_json, "sessionId")?;
    let request_id = extract_json_string(request_json, "requestId")?;
    let capture_id = extract_json_string(request_json, "captureId")?;
    let preset_id = extract_json_string(request_json, "presetId")?;
    let published_version = extract_json_string(request_json, "publishedVersion")?;
    let request_detail_path = extract_json_string(request_json, "diagnosticsDetailPath")?;
    let diagnostics_detail_path = PathBuf::from(&request_detail_path);
    let warm_state_path =
        warm_state_detail_path(&diagnostics_detail_path, &preset_id, &published_version)?;
    let warmup_result_path = diagnostics_detail_path
        .parent()
        .ok_or_else(|| "request diagnostics path must have a parent".to_string())?
        .join(format!(
            "warmup-{preset_id}-{published_version}.result.json"
        ));

    if !warm_state_matches_request(
        &warm_state_path,
        &session_id,
        &preset_id,
        &published_version,
    )? {
        let (detail_code, detail_message, warm_state) = if warmup_result_indicates_ready(
            &warmup_result_path,
            &session_id,
            &preset_id,
            &published_version,
        )? {
            (
                "warm-state-loss",
                "resident prototype warm state가 사라져 booth-safe fallback으로 내려가요.",
                "warm-state-lost",
            )
        } else {
            (
                "resident-not-warmed",
                "resident prototype warm state가 아직 준비되지 않아 booth-safe fallback으로 내려가요.",
                "cold",
            )
        };
        write_warm_state_file(
            &warm_state_path,
            &session_id,
            &preset_id,
            &published_version,
            warm_state,
        )?;
        write_json_file(
            result_path,
            &build_preview_result_json(
                result_path,
                &session_id,
                &request_id,
                &capture_id,
                "fallback-suggested",
                None,
                Some(detail_code),
                Some(detail_message),
                Some(warm_state),
                Some(path_to_runtime_string(&warm_state_path)),
            ),
        )?;
        return Ok(());
    }

    write_warm_state_file(
        &warm_state_path,
        &session_id,
        &preset_id,
        &published_version,
        "warm-hit",
    )?;
    let source_asset_path = extract_json_string(request_json, "sourceAssetPath")?;
    let preview_source_asset_path =
        extract_optional_json_string(request_json, "previewSourceAssetPath")?;
    let xmp_template_path = extract_json_string(request_json, "xmpTemplatePath")?;
    let canonical_preview_output_path =
        extract_json_string(request_json, "canonicalPreviewOutputPath")?;
    let render_source = resolve_preview_render_source(
        Path::new(&source_asset_path),
        preview_source_asset_path.as_deref().map(Path::new),
    );
    let render_status = render_preview_output(
        render_source.source_asset_path,
        Path::new(&xmp_template_path),
        Path::new(&canonical_preview_output_path),
        render_source.kind,
    );
    let (status, output_path, detail_code, detail_message): (
        &str,
        Option<String>,
        Option<String>,
        Option<String>,
    ) = match render_status {
        Ok(()) if Path::new(&canonical_preview_output_path).is_file() => (
            "accepted",
            Some(canonical_preview_output_path),
            Some("accepted".into()),
            Some("resident prototype가 canonical preview close를 만들었어요.".into()),
        ),
        Ok(()) => (
            "fallback-suggested",
            None,
            Some("invalid-output".into()),
            Some(
                "resident prototype output이 canonical preview를 만들지 못해 booth-safe fallback으로 내려가요."
                    .into(),
            ),
        ),
        Err(error) => (
            "fallback-suggested",
            None,
            Some("render-process-failed".into()),
            Some(error),
        ),
    };
    write_json_file(
        result_path,
        &build_preview_result_json(
            result_path,
            &session_id,
            &request_id,
            &capture_id,
            status,
            output_path,
            detail_code.as_deref(),
            detail_message.as_deref(),
            Some("warm-hit"),
            Some(path_to_runtime_string(&warm_state_path)),
        ),
    )?;

    Ok(())
}

fn warm_state_matches_request(
    warm_state_path: &Path,
    session_id: &str,
    preset_id: &str,
    published_version: &str,
) -> Result<bool, String> {
    let Some(warm_state) = load_warm_state_record(warm_state_path)? else {
        return Ok(false);
    };

    Ok(warm_state.session_id == session_id
        && warm_state.preset_id == preset_id
        && warm_state.published_version == published_version
        && matches!(warm_state.state.as_str(), "warm-ready" | "warm-hit"))
}

fn warmup_result_indicates_ready(
    warmup_result_path: &Path,
    session_id: &str,
    preset_id: &str,
    published_version: &str,
) -> Result<bool, String> {
    let Some(status) = load_json_string_field(warmup_result_path, "status")? else {
        return Ok(false);
    };
    let Some(result_session_id) = load_json_string_field(warmup_result_path, "sessionId")? else {
        return Ok(false);
    };
    let Some(result_preset_id) = load_json_string_field(warmup_result_path, "presetId")? else {
        return Ok(false);
    };
    let Some(result_published_version) =
        load_json_string_field(warmup_result_path, "publishedVersion")?
    else {
        return Ok(false);
    };
    let Some(warm_state) = load_json_string_field(warmup_result_path, "warmState")? else {
        return Ok(false);
    };

    Ok(result_session_id == session_id
        && result_preset_id == preset_id
        && result_published_version == published_version
        && status == "warmed-up"
        && warm_state == "warm-ready")
}

struct WarmStateRecord {
    session_id: String,
    preset_id: String,
    published_version: String,
    state: String,
}

fn load_warm_state_record(path: &Path) -> Result<Option<WarmStateRecord>, String> {
    let Some(session_id) = load_json_string_field(path, "sessionId")? else {
        return Ok(None);
    };
    let Some(preset_id) = load_json_string_field(path, "presetId")? else {
        return Ok(None);
    };
    let Some(published_version) = load_json_string_field(path, "publishedVersion")? else {
        return Ok(None);
    };
    let Some(state) = load_json_string_field(path, "state")? else {
        return Ok(None);
    };

    if !matches!(
        state.as_str(),
        "warm-ready" | "warm-hit" | "cold" | "warm-state-lost"
    ) {
        return Ok(None);
    }

    Ok(Some(WarmStateRecord {
        session_id,
        preset_id,
        published_version,
        state,
    }))
}

fn load_json_string_field(path: &Path, key: &str) -> Result<Option<String>, String> {
    if !path.is_file() {
        return Ok(None);
    }

    let document =
        fs::read_to_string(path).map_err(|error| format!("failed to read json file: {error}"))?;
    extract_json_string(&document, key).map(Some)
}

fn warm_state_detail_path(
    request_detail_path: &Path,
    preset_id: &str,
    published_version: &str,
) -> Result<PathBuf, String> {
    let diagnostics_dir = request_detail_path
        .parent()
        .ok_or_else(|| "request diagnostics path must have a parent".to_string())?;
    Ok(diagnostics_dir.join(format!("warm-state-{preset_id}-{published_version}.json")))
}

fn write_warm_state_file(
    path: &Path,
    session_id: &str,
    preset_id: &str,
    published_version: &str,
    state: &str,
) -> Result<(), String> {
    let payload = format!(
        concat!(
            "{{\n",
            "  \"schemaVersion\": \"resident-renderer-warm-state/v1\",\n",
            "  \"sessionId\": \"{}\",\n",
            "  \"presetId\": \"{}\",\n",
            "  \"publishedVersion\": \"{}\",\n",
            "  \"state\": \"{}\",\n",
            "  \"observedAt\": \"{}\"\n",
            "}}\n"
        ),
        json_escape(session_id),
        json_escape(preset_id),
        json_escape(published_version),
        json_escape(state),
        json_escape(&current_timestamp_string()?),
    );
    write_json_file(path, &payload)
}

fn write_json_file(path: &Path, payload: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to prepare output directory: {error}"))?;
    }
    fs::write(path, payload).map_err(|error| format!("failed to write json file: {error}"))
}

#[derive(Clone, Copy)]
enum PreviewRenderSourceKind {
    RawOriginal,
    FastPreviewRaster,
}

struct PreviewRenderSource<'a> {
    source_asset_path: &'a Path,
    kind: PreviewRenderSourceKind,
}

fn resolve_preview_render_source<'a>(
    source_asset_path: &'a Path,
    preview_source_asset_path: Option<&'a Path>,
) -> PreviewRenderSource<'a> {
    if let Some(preview_source_asset_path) = preview_source_asset_path {
        if preview_source_asset_path.is_file()
            || wait_for_fast_preview_source_asset(preview_source_asset_path)
        {
            return PreviewRenderSource {
                source_asset_path: preview_source_asset_path,
                kind: PreviewRenderSourceKind::FastPreviewRaster,
            };
        }
    }

    PreviewRenderSource {
        source_asset_path,
        kind: PreviewRenderSourceKind::RawOriginal,
    }
}

fn wait_for_fast_preview_source_asset(preview_source_asset_path: &Path) -> bool {
    if preview_source_asset_path.is_file() {
        return true;
    }

    let deadline = Instant::now() + Duration::from_millis(FAST_PREVIEW_SOURCE_WAIT_BUDGET_MS);
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(FAST_PREVIEW_SOURCE_WAIT_POLL_MS));
        if preview_source_asset_path.is_file() {
            return true;
        }
    }

    false
}

fn render_preview_output(
    source_asset_path: &Path,
    xmp_template_path: &Path,
    output_path: &Path,
    render_source_kind: PreviewRenderSourceKind,
) -> Result<(), String> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to prepare render output directory: {error}"))?;
    }
    let staged_output_path = staged_render_output_path(output_path);
    let _ = fs::remove_file(&staged_output_path);

    let resolution = resolve_darktable_cli_binary();
    let (width_cap, height_cap) = preview_render_dimensions(render_source_kind);
    let status = Command::new(&resolution.binary)
        .arg(path_to_runtime_string(source_asset_path))
        .arg(path_to_runtime_string(xmp_template_path))
        .arg(path_to_runtime_string(&staged_output_path))
        .arg("--hq")
        .arg("false")
        .arg("--apply-custom-presets")
        .arg(DARKTABLE_APPLY_CUSTOM_PRESETS_DISABLED)
        .arg("--width")
        .arg(width_cap.to_string())
        .arg("--height")
        .arg(height_cap.to_string())
        .arg("--core")
        .arg("--library")
        .arg(DARKTABLE_PREVIEW_LIBRARY_IN_MEMORY)
        .arg("--disable-opencl")
        .status()
        .map_err(|error| {
            format!(
                "resident prototype render failed to launch: source={} binary={} error={error}",
                resolution.source, resolution.binary
            )
        })?;

    if !status.success() {
        let _ = fs::remove_file(&staged_output_path);
        return Err(format!(
            "resident prototype render process exited with status {:?}",
            status.code()
        ));
    }

    if !staged_output_path.is_file() {
        return Err("resident prototype render did not produce a staged preview output".into());
    }

    fs::copy(&staged_output_path, output_path).map_err(|error| {
        format!("resident prototype could not replace canonical preview output: {error}")
    })?;
    let _ = fs::remove_file(&staged_output_path);

    Ok(())
}

fn preview_render_dimensions(source_kind: PreviewRenderSourceKind) -> (u32, u32) {
    match source_kind {
        PreviewRenderSourceKind::RawOriginal => {
            (RAW_PREVIEW_MAX_WIDTH_PX, RAW_PREVIEW_MAX_HEIGHT_PX)
        }
        PreviewRenderSourceKind::FastPreviewRaster => (
            FAST_PREVIEW_RENDER_MAX_WIDTH_PX,
            FAST_PREVIEW_RENDER_MAX_HEIGHT_PX,
        ),
    }
}

fn staged_render_output_path(output_path: &Path) -> PathBuf {
    let parent = output_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = output_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("preview");

    parent.join(format!("{stem}.preview-rendering.jpg"))
}

struct DarktableBinaryResolution {
    binary: String,
    source: &'static str,
}

fn resolve_darktable_cli_binary() -> DarktableBinaryResolution {
    resolve_darktable_cli_binary_with_candidates(
        env::var(DARKTABLE_CLI_BIN_ENV).ok().as_deref(),
        &darktable_cli_binary_candidates(),
    )
}

fn resolve_darktable_cli_binary_with_candidates(
    env_override: Option<&str>,
    candidates: &[(&'static str, PathBuf)],
) -> DarktableBinaryResolution {
    if let Some(binary) = env_override.filter(|value| !value.trim().is_empty()) {
        return DarktableBinaryResolution {
            binary: binary.to_string(),
            source: "env-override",
        };
    }

    for (source, candidate) in candidates {
        if candidate.is_file() {
            return DarktableBinaryResolution {
                binary: candidate.to_string_lossy().into_owned(),
                source,
            };
        }
    }

    DarktableBinaryResolution {
        binary: "darktable-cli".into(),
        source: "path",
    }
}

fn darktable_cli_binary_candidates() -> Vec<(&'static str, PathBuf)> {
    let mut candidates = Vec::new();

    if !cfg!(windows) {
        return candidates;
    }

    push_darktable_cli_candidate(
        &mut candidates,
        "program-files-bin",
        env::var_os("ProgramFiles")
            .map(PathBuf::from)
            .map(|root| root.join("darktable").join("bin").join("darktable-cli.exe")),
    );
    push_darktable_cli_candidate(
        &mut candidates,
        "program-w6432-bin",
        env::var_os("ProgramW6432")
            .map(PathBuf::from)
            .map(|root| root.join("darktable").join("bin").join("darktable-cli.exe")),
    );
    push_darktable_cli_candidate(
        &mut candidates,
        "localappdata-programs-bin",
        env::var_os("LOCALAPPDATA").map(PathBuf::from).map(|root| {
            root.join("Programs")
                .join("darktable")
                .join("bin")
                .join("darktable-cli.exe")
        }),
    );

    candidates
}

fn push_darktable_cli_candidate(
    candidates: &mut Vec<(&'static str, PathBuf)>,
    source: &'static str,
    candidate: Option<PathBuf>,
) {
    let Some(candidate) = candidate else {
        return;
    };

    if candidates.iter().any(|(_, existing)| *existing == candidate) {
        return;
    }

    candidates.push((source, candidate));
}

fn build_warmup_result_json(
    result_path: &Path,
    session_id: &str,
    preset_id: &str,
    published_version: &str,
    status: &str,
    detail_code: Option<&str>,
    detail_message: Option<&str>,
    warm_state: Option<&str>,
    warm_state_detail_path: Option<String>,
) -> String {
    let mut body = vec![
        json_field(
            "schemaVersion",
            "dedicated-renderer-warmup-result/v1".to_string(),
        ),
        json_field("sessionId", session_id.to_string()),
        json_field("presetId", preset_id.to_string()),
        json_field("publishedVersion", published_version.to_string()),
        json_field("status", status.to_string()),
        json_field("diagnosticsDetailPath", path_to_runtime_string(result_path)),
    ];
    if let Some(detail_code) = detail_code {
        body.push(json_field("detailCode", detail_code.to_string()));
    }
    if let Some(detail_message) = detail_message {
        body.push(json_field("detailMessage", detail_message.to_string()));
    }
    if let Some(warm_state) = warm_state {
        body.push(json_field("warmState", warm_state.to_string()));
    }
    if let Some(warm_state_detail_path) = warm_state_detail_path {
        body.push(json_field("warmStateDetailPath", warm_state_detail_path));
    }
    format!("{{\n{}\n}}\n", body.join(",\n"))
}

fn build_preview_result_json(
    result_path: &Path,
    session_id: &str,
    request_id: &str,
    capture_id: &str,
    status: &str,
    output_path: Option<String>,
    detail_code: Option<&str>,
    detail_message: Option<&str>,
    warm_state: Option<&str>,
    warm_state_detail_path: Option<String>,
) -> String {
    let mut body = vec![
        json_field(
            "schemaVersion",
            "dedicated-renderer-preview-job-result/v1".to_string(),
        ),
        json_field("sessionId", session_id.to_string()),
        json_field("requestId", request_id.to_string()),
        json_field("captureId", capture_id.to_string()),
        json_field("status", status.to_string()),
        json_field("diagnosticsDetailPath", path_to_runtime_string(result_path)),
    ];
    if let Some(output_path) = output_path {
        body.push(json_field("outputPath", output_path));
    }
    if let Some(detail_code) = detail_code {
        body.push(json_field("detailCode", detail_code.to_string()));
    }
    if let Some(detail_message) = detail_message {
        body.push(json_field("detailMessage", detail_message.to_string()));
    }
    if let Some(warm_state) = warm_state {
        body.push(json_field("warmState", warm_state.to_string()));
    }
    if let Some(warm_state_detail_path) = warm_state_detail_path {
        body.push(json_field("warmStateDetailPath", warm_state_detail_path));
    }
    format!("{{\n{}\n}}\n", body.join(",\n"))
}

fn json_field(key: &str, value: String) -> String {
    format!("  \"{}\": \"{}\"", json_escape(key), json_escape(&value))
}

fn extract_json_string(document: &str, key: &str) -> Result<String, String> {
    let key_pattern = format!("\"{key}\"");
    let key_index = document
        .find(&key_pattern)
        .ok_or_else(|| format!("missing json key: {key}"))?;
    let value_start = document[key_index + key_pattern.len()..]
        .find(':')
        .map(|index| key_index + key_pattern.len() + index + 1)
        .ok_or_else(|| format!("missing json value separator: {key}"))?;
    let after_colon = &document[value_start..];
    let opening_quote = after_colon
        .char_indices()
        .find(|(_, char)| !char.is_whitespace())
        .map(|(index, _)| value_start + index)
        .ok_or_else(|| format!("missing opening quote for key: {key}"))?;

    let (decoded, _) = parse_json_string_literal(document, opening_quote)?;
    Ok(decoded)
}

fn extract_optional_json_string(document: &str, key: &str) -> Result<Option<String>, String> {
    let key_pattern = format!("\"{key}\"");
    let Some(key_index) = document.find(&key_pattern) else {
        return Ok(None);
    };
    let value_start = document[key_index + key_pattern.len()..]
        .find(':')
        .map(|index| key_index + key_pattern.len() + index + 1)
        .ok_or_else(|| format!("missing json value separator: {key}"))?;
    let after_colon = &document[value_start..];
    let opening_quote = after_colon
        .char_indices()
        .find(|(_, char)| !char.is_whitespace())
        .map(|(index, _)| value_start + index)
        .ok_or_else(|| format!("missing opening quote for key: {key}"))?;

    let (decoded, _) = parse_json_string_literal(document, opening_quote)?;
    Ok(Some(decoded))
}

fn parse_json_string_literal(
    document: &str,
    start_index: usize,
) -> Result<(String, usize), String> {
    let bytes = document.as_bytes();
    if bytes.get(start_index) != Some(&b'"') {
        return Err("json string literal must start with a quote".into());
    }

    let mut index = start_index + 1;
    let mut decoded = String::new();

    while let Some(byte) = bytes.get(index) {
        match byte {
            b'"' => return Ok((decoded, index + 1)),
            b'\\' => {
                index += 1;
                let Some(escaped) = bytes.get(index) else {
                    return Err("unterminated json escape".into());
                };
                match escaped {
                    b'"' => decoded.push('"'),
                    b'\\' => decoded.push('\\'),
                    b'/' => decoded.push('/'),
                    b'b' => decoded.push('\u{0008}'),
                    b'f' => decoded.push('\u{000C}'),
                    b'n' => decoded.push('\n'),
                    b'r' => decoded.push('\r'),
                    b't' => decoded.push('\t'),
                    b'u' => {
                        let hex_start = index + 1;
                        let hex_end = hex_start + 4;
                        let Some(hex) = document.get(hex_start..hex_end) else {
                            return Err("unterminated unicode escape".into());
                        };
                        let codepoint = u32::from_str_radix(hex, 16)
                            .map_err(|_| "invalid unicode escape".to_string())?;
                        let character = char::from_u32(codepoint)
                            .ok_or_else(|| "invalid unicode scalar value".to_string())?;
                        decoded.push(character);
                        index = hex_end - 1;
                    }
                    _ => return Err("unsupported json escape".into()),
                }
            }
            value if *value < 0x20 => return Err("json string contains a control character".into()),
            value => decoded.push(*value as char),
        }
        index += 1;
    }

    Err("unterminated json string literal".into())
}

fn path_to_runtime_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn current_timestamp_string() -> Result<String, String> {
    let unix_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "failed to read system clock".to_string())?
        .as_secs();
    Ok(unix_seconds_to_rfc3339(unix_seconds))
}

fn unix_seconds_to_rfc3339(unix_seconds: u64) -> String {
    let seconds_per_day = 86_400;
    let days = (unix_seconds / seconds_per_day) as i64;
    let seconds_of_day = unix_seconds % seconds_per_day;

    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let adjusted_year = year + if month <= 2 { 1 } else { 0 };

    (adjusted_year as i32, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{Mutex, OnceLock},
        time::{SystemTime, UNIX_EPOCH},
    };

    fn darktable_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = env::temp_dir().join(format!("boothy-dedicated-renderer-{label}-{unique}"));
        fs::create_dir_all(&path).expect("temp dir should be created");
        path
    }

    #[test]
    fn extract_json_string_decodes_escaped_characters() {
        let decoded = extract_json_string(
            r#"{
              "diagnosticsDetailPath": "C:\\temp\\boothy\\\"quoted\\\"\\request.json"
            }"#,
            "diagnosticsDetailPath",
        )
        .expect("json field should decode");

        assert_eq!(decoded, r#"C:\temp\boothy\"quoted\"\request.json"#);
    }

    #[test]
    fn extract_optional_json_string_returns_none_when_field_is_missing() {
        let decoded = extract_optional_json_string(
            r#"{
              "diagnosticsDetailPath": "C:\\temp\\boothy\\request.json"
            }"#,
            "previewSourceAssetPath",
        )
        .expect("missing optional field should not fail");

        assert_eq!(decoded, None);
    }

    #[test]
    fn preview_does_not_treat_mismatched_warm_state_file_as_warm_hit() {
        let temp_dir = unique_temp_dir("stale-warm-state");
        let diagnostics_dir = temp_dir.join("diagnostics").join("dedicated-renderer");
        let request_path = diagnostics_dir.join("capture.preview-request.json");
        let result_path = diagnostics_dir.join("capture.preview-result.json");
        let warm_state_path = diagnostics_dir.join("warm-state-preset_soft-glow-2026.04.10.json");
        let request_json = format!(
            concat!(
                "{{\n",
                "  \"sessionId\": \"session_01hs6n1r8b8zc5v4ey2x7b9g1m\",\n",
                "  \"requestId\": \"request_01\",\n",
                "  \"captureId\": \"capture_01\",\n",
                "  \"presetId\": \"preset_soft-glow\",\n",
                "  \"publishedVersion\": \"2026.04.10\",\n",
                "  \"diagnosticsDetailPath\": \"{}\"\n",
                "}}\n"
            ),
            path_to_runtime_string(&request_path)
        );

        write_warm_state_file(
            &warm_state_path,
            "session_01hs6n1r8b8zc5v4ey2x7b9zzz",
            "preset_soft-glow",
            "2026.04.10",
            "warm-ready",
        )
        .expect("stale warm state file should write");

        handle_preview(&request_json, &result_path).expect("preview should still return fallback");

        let result_json = fs::read_to_string(&result_path).expect("result should be written");
        assert!(result_json.contains("\"detailCode\": \"resident-not-warmed\""));
        assert!(result_json.contains("\"warmState\": \"cold\""));
        assert!(!result_json.contains("\"warmState\": \"warm-hit\""));

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn preview_only_reports_warm_state_loss_after_a_real_warmup() {
        let temp_dir = unique_temp_dir("warmup-result-residue");
        let diagnostics_dir = temp_dir.join("diagnostics").join("dedicated-renderer");
        let request_path = diagnostics_dir.join("capture.preview-request.json");
        let result_path = diagnostics_dir.join("capture.preview-result.json");
        let warmup_result_path =
            diagnostics_dir.join("warmup-preset_soft-glow-2026.04.10.result.json");
        let request_json = format!(
            concat!(
                "{{\n",
                "  \"sessionId\": \"session_01hs6n1r8b8zc5v4ey2x7b9g1m\",\n",
                "  \"requestId\": \"request_01\",\n",
                "  \"captureId\": \"capture_01\",\n",
                "  \"presetId\": \"preset_soft-glow\",\n",
                "  \"publishedVersion\": \"2026.04.10\",\n",
                "  \"diagnosticsDetailPath\": \"{}\"\n",
                "}}\n"
            ),
            path_to_runtime_string(&request_path)
        );

        write_json_file(
            &warmup_result_path,
            r#"{
  "schemaVersion": "dedicated-renderer-warmup-result/v1",
  "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
  "presetId": "preset_soft-glow",
  "publishedVersion": "2026.04.10",
  "status": "fallback-suggested",
  "diagnosticsDetailPath": "C:/temp/warmup.result.json",
  "detailCode": "sidecar-unavailable",
  "warmState": "cold"
}
"#,
        )
        .expect("fallback warmup result should write");

        handle_preview(&request_json, &result_path).expect("preview should still return fallback");

        let result_json = fs::read_to_string(&result_path).expect("result should be written");
        assert!(result_json.contains("\"detailCode\": \"resident-not-warmed\""));
        assert!(result_json.contains("\"warmState\": \"cold\""));
        assert!(!result_json.contains("\"detailCode\": \"warm-state-loss\""));

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn preview_returns_accepted_after_warm_hit_when_render_succeeds() {
        let temp_dir = unique_temp_dir("accepted-preview");
        let diagnostics_dir = temp_dir.join("diagnostics").join("dedicated-renderer");
        let request_path = diagnostics_dir.join("capture.preview-request.json");
        let result_path = diagnostics_dir.join("capture.preview-result.json");
        let source_asset_path = temp_dir.join("captures").join("capture_01.cr3");
        let xmp_template_path = temp_dir.join("preset").join("xmp").join("template.xmp");
        let canonical_preview_output_path = temp_dir.join("renders").join("capture_01.jpg");
        let fake_darktable_path = temp_dir.join("fake-darktable.cmd");
        let warm_state_path = diagnostics_dir.join("warm-state-preset_soft-glow-2026.04.10.json");
        let request_json = format!(
            concat!(
                "{{\n",
                "  \"sessionId\": \"session_01hs6n1r8b8zc5v4ey2x7b9g1m\",\n",
                "  \"requestId\": \"request_01\",\n",
                "  \"captureId\": \"capture_01\",\n",
                "  \"presetId\": \"preset_soft-glow\",\n",
                "  \"publishedVersion\": \"2026.04.10\",\n",
                "  \"sourceAssetPath\": \"{}\",\n",
                "  \"xmpTemplatePath\": \"{}\",\n",
                "  \"canonicalPreviewOutputPath\": \"{}\",\n",
                "  \"diagnosticsDetailPath\": \"{}\"\n",
                "}}\n"
            ),
            path_to_runtime_string(&source_asset_path),
            path_to_runtime_string(&xmp_template_path),
            path_to_runtime_string(&canonical_preview_output_path),
            path_to_runtime_string(&request_path)
        );

        fs::create_dir_all(source_asset_path.parent().expect("source dir should exist"))
            .expect("source dir should be created");
        fs::create_dir_all(xmp_template_path.parent().expect("xmp dir should exist"))
            .expect("xmp dir should be created");
        fs::write(&source_asset_path, b"raw").expect("source asset should exist");
        fs::write(&xmp_template_path, b"xmp").expect("xmp template should exist");
        fs::write(
            &fake_darktable_path,
            "@echo off\r\nfor %%I in (\"%~3\") do if not exist \"%%~dpI\" mkdir \"%%~dpI\" >nul 2>&1\r\npowershell -NoProfile -Command \"$bytes=[Convert]::FromBase64String('/9j/4AAQSkZJRgABAQAAAQABAAD/2Q==');[IO.File]::WriteAllBytes('%~3',$bytes)\"\r\nexit /b 0\r\n",
        )
        .expect("fake darktable should be written");
        write_warm_state_file(
            &warm_state_path,
            "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
            "preset_soft-glow",
            "2026.04.10",
            "warm-ready",
        )
        .expect("warm state should exist");

        let _env_guard = darktable_env_lock()
            .lock()
            .expect("darktable env test lock should be available");
        let previous = env::var_os("BOOTHY_DARKTABLE_CLI_BIN");
        env::set_var("BOOTHY_DARKTABLE_CLI_BIN", &fake_darktable_path);
        let preview_result = handle_preview(&request_json, &result_path);
        match previous {
            Some(value) => env::set_var("BOOTHY_DARKTABLE_CLI_BIN", value),
            None => env::remove_var("BOOTHY_DARKTABLE_CLI_BIN"),
        }

        preview_result.expect("preview should accept a warmed render");

        let result_json = fs::read_to_string(&result_path).expect("result should be written");
        assert!(result_json.contains("\"status\": \"accepted\""));
        assert!(result_json.contains("\"outputPath\":"));
        assert!(result_json.contains("\"warmState\": \"warm-hit\""));
        assert!(canonical_preview_output_path.is_file());

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn preview_prefers_fast_preview_source_and_display_sized_export_arguments() {
        let temp_dir = unique_temp_dir("fast-preview-source");
        let diagnostics_dir = temp_dir.join("diagnostics").join("dedicated-renderer");
        let request_path = diagnostics_dir.join("capture.preview-request.json");
        let result_path = diagnostics_dir.join("capture.preview-result.json");
        let source_asset_path = temp_dir.join("captures").join("capture_01.cr3");
        let fast_preview_asset_path = temp_dir.join("renders").join("capture_01-fast.jpg");
        let xmp_template_path = temp_dir.join("preset").join("xmp").join("template.xmp");
        let canonical_preview_output_path = temp_dir.join("renders").join("capture_01.jpg");
        let fake_darktable_path = temp_dir.join("fake-darktable.cmd");
        let invocation_log_path = temp_dir.join("darktable-args.txt");
        let warm_state_path = diagnostics_dir.join("warm-state-preset_soft-glow-2026.04.10.json");
        let request_json = format!(
            concat!(
                "{{\n",
                "  \"sessionId\": \"session_01hs6n1r8b8zc5v4ey2x7b9g1m\",\n",
                "  \"requestId\": \"request_01\",\n",
                "  \"captureId\": \"capture_01\",\n",
                "  \"presetId\": \"preset_soft-glow\",\n",
                "  \"publishedVersion\": \"2026.04.10\",\n",
                "  \"sourceAssetPath\": \"{}\",\n",
                "  \"previewSourceAssetPath\": \"{}\",\n",
                "  \"xmpTemplatePath\": \"{}\",\n",
                "  \"canonicalPreviewOutputPath\": \"{}\",\n",
                "  \"diagnosticsDetailPath\": \"{}\"\n",
                "}}\n"
            ),
            path_to_runtime_string(&source_asset_path),
            path_to_runtime_string(&fast_preview_asset_path),
            path_to_runtime_string(&xmp_template_path),
            path_to_runtime_string(&canonical_preview_output_path),
            path_to_runtime_string(&request_path)
        );

        fs::create_dir_all(source_asset_path.parent().expect("source dir should exist"))
            .expect("source dir should be created");
        fs::create_dir_all(
            fast_preview_asset_path
                .parent()
                .expect("fast preview dir should exist"),
        )
        .expect("fast preview dir should be created");
        fs::create_dir_all(xmp_template_path.parent().expect("xmp dir should exist"))
            .expect("xmp dir should be created");
        fs::write(&source_asset_path, b"raw").expect("source asset should exist");
        fs::write(&fast_preview_asset_path, b"jpeg").expect("fast preview asset should exist");
        fs::write(&xmp_template_path, b"xmp").expect("xmp template should exist");
        fs::write(
            &fake_darktable_path,
            format!(
                "@echo off\r\nset \"LOG={}\"\r\necho %* > \"%LOG%\"\r\nfor %%I in (\"%~3\") do if not exist \"%%~dpI\" mkdir \"%%~dpI\" >nul 2>&1\r\npowershell -NoProfile -Command \"$bytes=[Convert]::FromBase64String('/9j/4AAQSkZJRgABAQAAAQABAAD/2Q==');[IO.File]::WriteAllBytes('%~3',$bytes)\"\r\nexit /b 0\r\n",
                path_to_runtime_string(&invocation_log_path)
            ),
        )
        .expect("fake darktable should be written");
        write_warm_state_file(
            &warm_state_path,
            "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
            "preset_soft-glow",
            "2026.04.10",
            "warm-ready",
        )
        .expect("warm state should exist");

        let _env_guard = darktable_env_lock()
            .lock()
            .expect("darktable env test lock should be available");
        let previous = env::var_os("BOOTHY_DARKTABLE_CLI_BIN");
        env::set_var("BOOTHY_DARKTABLE_CLI_BIN", &fake_darktable_path);
        let preview_result = handle_preview(&request_json, &result_path);
        match previous {
            Some(value) => env::set_var("BOOTHY_DARKTABLE_CLI_BIN", value),
            None => env::remove_var("BOOTHY_DARKTABLE_CLI_BIN"),
        }

        preview_result.expect("preview should accept a warmed render");

        let invocation_args =
            fs::read_to_string(&invocation_log_path).expect("invocation log should exist");
        assert!(invocation_args.contains(&path_to_runtime_string(&fast_preview_asset_path)));
        assert!(invocation_args.contains("--width"));
        assert!(invocation_args.contains("--height"));
        assert!(invocation_args.contains(&FAST_PREVIEW_RENDER_MAX_WIDTH_PX.to_string()));
        assert!(invocation_args.contains("--core"));
        assert!(invocation_args.contains("--library"));
        assert!(invocation_args.contains(DARKTABLE_PREVIEW_LIBRARY_IN_MEMORY));
        assert!(invocation_args.contains("--disable-opencl"));

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn preview_replaces_canonical_output_after_rendering_from_same_fast_preview_path() {
        let temp_dir = unique_temp_dir("same-fast-preview-and-output");
        let diagnostics_dir = temp_dir.join("diagnostics").join("dedicated-renderer");
        let request_path = diagnostics_dir.join("capture.preview-request.json");
        let result_path = diagnostics_dir.join("capture.preview-result.json");
        let source_asset_path = temp_dir.join("captures").join("capture_01.cr3");
        let canonical_preview_output_path = temp_dir.join("renders").join("capture_01.jpg");
        let xmp_template_path = temp_dir.join("preset").join("xmp").join("template.xmp");
        let fake_darktable_path = temp_dir.join("fake-darktable.cmd");
        let invocation_log_path = temp_dir.join("darktable-output-arg.txt");
        let warm_state_path = diagnostics_dir.join("warm-state-preset_soft-glow-2026.04.10.json");
        let request_json = format!(
            concat!(
                "{{\n",
                "  \"sessionId\": \"session_01hs6n1r8b8zc5v4ey2x7b9g1m\",\n",
                "  \"requestId\": \"request_01\",\n",
                "  \"captureId\": \"capture_01\",\n",
                "  \"presetId\": \"preset_soft-glow\",\n",
                "  \"publishedVersion\": \"2026.04.10\",\n",
                "  \"sourceAssetPath\": \"{}\",\n",
                "  \"previewSourceAssetPath\": \"{}\",\n",
                "  \"xmpTemplatePath\": \"{}\",\n",
                "  \"canonicalPreviewOutputPath\": \"{}\",\n",
                "  \"diagnosticsDetailPath\": \"{}\"\n",
                "}}\n"
            ),
            path_to_runtime_string(&source_asset_path),
            path_to_runtime_string(&canonical_preview_output_path),
            path_to_runtime_string(&xmp_template_path),
            path_to_runtime_string(&canonical_preview_output_path),
            path_to_runtime_string(&request_path)
        );

        fs::create_dir_all(source_asset_path.parent().expect("source dir should exist"))
            .expect("source dir should be created");
        fs::create_dir_all(
            canonical_preview_output_path
                .parent()
                .expect("preview dir should exist"),
        )
        .expect("preview dir should be created");
        fs::create_dir_all(xmp_template_path.parent().expect("xmp dir should exist"))
            .expect("xmp dir should be created");
        fs::write(&source_asset_path, b"raw").expect("source asset should exist");
        fs::write(&canonical_preview_output_path, b"original-preview")
            .expect("existing preview should exist");
        fs::write(&xmp_template_path, b"xmp").expect("xmp template should exist");
        fs::write(
            &fake_darktable_path,
            format!(
                "@echo off\r\nset \"LOG={}\"\r\necho %3>\"%LOG%\"\r\nfor %%I in (\"%~3\") do if not exist \"%%~dpI\" mkdir \"%%~dpI\" >nul 2>&1\r\necho rendered-preview>\"%~3\"\r\nexit /b 0\r\n",
                path_to_runtime_string(&invocation_log_path)
            ),
        )
        .expect("fake darktable should be written");
        write_warm_state_file(
            &warm_state_path,
            "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
            "preset_soft-glow",
            "2026.04.10",
            "warm-ready",
        )
        .expect("warm state should exist");

        let _env_guard = darktable_env_lock()
            .lock()
            .expect("darktable env test lock should be available");
        let previous = env::var_os("BOOTHY_DARKTABLE_CLI_BIN");
        env::set_var("BOOTHY_DARKTABLE_CLI_BIN", &fake_darktable_path);
        let preview_result = handle_preview(&request_json, &result_path);
        match previous {
            Some(value) => env::set_var("BOOTHY_DARKTABLE_CLI_BIN", value),
            None => env::remove_var("BOOTHY_DARKTABLE_CLI_BIN"),
        }

        preview_result.expect("preview should accept a warmed render");

        let output_bytes =
            fs::read(&canonical_preview_output_path).expect("canonical preview should exist");
        let darktable_output_arg =
            fs::read_to_string(&invocation_log_path).expect("invocation log should exist");

        assert!(!darktable_output_arg.contains(&path_to_runtime_string(&canonical_preview_output_path)));
        assert!(darktable_output_arg.contains(".preview-rendering.jpg"));
        assert_ne!(output_bytes, b"original-preview");
        assert_eq!(output_bytes, b"rendered-preview\r\n");

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn preview_waits_briefly_for_fast_preview_candidate_before_falling_back_to_raw() {
        let temp_dir = unique_temp_dir("late-fast-preview");
        let diagnostics_dir = temp_dir.join("diagnostics").join("dedicated-renderer");
        let request_path = diagnostics_dir.join("capture.preview-request.json");
        let result_path = diagnostics_dir.join("capture.preview-result.json");
        let source_asset_path = temp_dir.join("captures").join("capture_01.cr3");
        let late_fast_preview_asset_path = temp_dir.join("renders").join("capture_01.jpg");
        let xmp_template_path = temp_dir.join("preset").join("xmp").join("template.xmp");
        let canonical_preview_output_path = temp_dir.join("renders").join("capture_01-close.jpg");
        let fake_darktable_path = temp_dir.join("fake-darktable.cmd");
        let invocation_log_path = temp_dir.join("darktable-args.txt");
        let warm_state_path = diagnostics_dir.join("warm-state-preset_soft-glow-2026.04.10.json");
        let request_json = format!(
            concat!(
                "{{\n",
                "  \"sessionId\": \"session_01hs6n1r8b8zc5v4ey2x7b9g1m\",\n",
                "  \"requestId\": \"request_01\",\n",
                "  \"captureId\": \"capture_01\",\n",
                "  \"presetId\": \"preset_soft-glow\",\n",
                "  \"publishedVersion\": \"2026.04.10\",\n",
                "  \"sourceAssetPath\": \"{}\",\n",
                "  \"previewSourceAssetPath\": \"{}\",\n",
                "  \"xmpTemplatePath\": \"{}\",\n",
                "  \"canonicalPreviewOutputPath\": \"{}\",\n",
                "  \"diagnosticsDetailPath\": \"{}\"\n",
                "}}\n"
            ),
            path_to_runtime_string(&source_asset_path),
            path_to_runtime_string(&late_fast_preview_asset_path),
            path_to_runtime_string(&xmp_template_path),
            path_to_runtime_string(&canonical_preview_output_path),
            path_to_runtime_string(&request_path)
        );

        fs::create_dir_all(source_asset_path.parent().expect("source dir should exist"))
            .expect("source dir should be created");
        fs::create_dir_all(
            late_fast_preview_asset_path
                .parent()
                .expect("fast preview dir should exist"),
        )
        .expect("fast preview dir should be created");
        fs::create_dir_all(xmp_template_path.parent().expect("xmp dir should exist"))
            .expect("xmp dir should be created");
        fs::write(&source_asset_path, b"raw").expect("source asset should exist");
        fs::write(&xmp_template_path, b"xmp").expect("xmp template should exist");
        fs::write(
            &fake_darktable_path,
            format!(
                "@echo off\r\nset \"LOG={}\"\r\necho %1>\"%LOG%\"\r\nfor %%I in (\"%~3\") do if not exist \"%%~dpI\" mkdir \"%%~dpI\" >nul 2>&1\r\necho rendered-preview>\"%~3\"\r\nexit /b 0\r\n",
                path_to_runtime_string(&invocation_log_path)
            ),
        )
        .expect("fake darktable should be written");
        write_warm_state_file(
            &warm_state_path,
            "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
            "preset_soft-glow",
            "2026.04.10",
            "warm-ready",
        )
        .expect("warm state should exist");

        let late_path = late_fast_preview_asset_path.clone();
        let writer = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(75));
            fs::write(&late_path, b"jpeg").expect("late fast preview should be created");
        });

        let _env_guard = darktable_env_lock()
            .lock()
            .expect("darktable env test lock should be available");
        let previous = env::var_os("BOOTHY_DARKTABLE_CLI_BIN");
        env::set_var("BOOTHY_DARKTABLE_CLI_BIN", &fake_darktable_path);
        let preview_result = handle_preview(&request_json, &result_path);
        match previous {
            Some(value) => env::set_var("BOOTHY_DARKTABLE_CLI_BIN", value),
            None => env::remove_var("BOOTHY_DARKTABLE_CLI_BIN"),
        }
        writer.join().expect("late preview writer should finish");

        preview_result.expect("preview should accept a warmed render");

        let invocation_source =
            fs::read_to_string(&invocation_log_path).expect("invocation log should exist");
        assert!(invocation_source.contains(&path_to_runtime_string(&late_fast_preview_asset_path)));

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn darktable_cli_resolution_uses_existing_known_install_path() {
        let temp_dir = unique_temp_dir("known-darktable-install");
        let candidate = temp_dir
            .join("darktable")
            .join("bin")
            .join("darktable-cli.exe");
        fs::create_dir_all(
            candidate
                .parent()
                .expect("candidate should have a parent directory"),
        )
        .expect("candidate parent directory should be creatable");
        fs::write(&candidate, "cli").expect("candidate binary should be writable");

        let resolution = resolve_darktable_cli_binary_with_candidates(
            None,
            &[("program-files-bin", candidate.clone())],
        );

        assert_eq!(resolution.binary, candidate.to_string_lossy().as_ref());
        assert_eq!(resolution.source, "program-files-bin");

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn darktable_cli_resolution_falls_back_to_path_when_no_known_binary_exists() {
        let resolution = resolve_darktable_cli_binary_with_candidates(
            None,
            &[(
                "program-files-bin",
                PathBuf::from("C:/missing/darktable-cli.exe"),
            )],
        );

        assert_eq!(resolution.binary, "darktable-cli");
        assert_eq!(resolution.source, "path");
    }

    #[test]
    fn warm_state_evidence_includes_observed_at_for_freshness_proof() {
        let temp_dir = unique_temp_dir("warm-state-observed-at");
        let warm_state_path = temp_dir.join("warm-state.json");

        write_warm_state_file(
            &warm_state_path,
            "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
            "preset_soft-glow",
            "2026.04.10",
            "warm-ready",
        )
        .expect("warm state should write");

        let warm_state_json =
            fs::read_to_string(&warm_state_path).expect("warm state file should be readable");
        assert!(warm_state_json.contains("\"observedAt\":"));

        let _ = fs::remove_dir_all(temp_dir);
    }
}
