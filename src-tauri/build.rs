use std::{env, fs, path::PathBuf};

fn main() {
    ensure_dedicated_renderer_placeholder().expect("dedicated renderer placeholder should exist");
    tauri_build::build()
}

fn ensure_dedicated_renderer_placeholder() -> std::io::Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let repo_root = manifest_dir
        .parent()
        .map(PathBuf::from)
        .unwrap_or(manifest_dir);
    let target = env::var("TARGET").unwrap_or_else(|_| "x86_64-pc-windows-msvc".into());
    let binary_name = if target.contains("windows") {
        format!("boothy-dedicated-renderer-{target}.exe")
    } else {
        format!("boothy-dedicated-renderer-{target}")
    };
    let binary_path = repo_root
        .join("sidecar")
        .join("dedicated-renderer")
        .join(binary_name);

    if binary_path.exists() {
        return Ok(());
    }

    if let Some(parent) = binary_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(
        binary_path,
        b"Boothy dedicated renderer placeholder for Story 1.11 packaging baseline.\n",
    )
}
