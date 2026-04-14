use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../sidecar/dedicated-renderer/main.rs");
    ensure_dedicated_renderer_sidecar().expect("dedicated renderer sidecar should exist");
    tauri_build::build()
}

fn ensure_dedicated_renderer_sidecar() -> std::io::Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap_or_default());
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
        .join(&binary_name);

    let source_path = repo_root
        .join("sidecar")
        .join("dedicated-renderer")
        .join("main.rs");
    let compiled_binary_path = out_dir.join(&binary_name);

    if let Some(parent) = binary_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(parent) = compiled_binary_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let status = Command::new(rustc)
        .arg(&source_path)
        .arg("--edition=2021")
        .arg("--target")
        .arg(&target)
        .arg("-C")
        .arg("opt-level=0")
        .arg("-o")
        .arg(&compiled_binary_path)
        .status()?;

    if status.success() {
        fs::copy(&compiled_binary_path, &binary_path)?;
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "failed to compile dedicated renderer sidecar",
        ))
    }
}
