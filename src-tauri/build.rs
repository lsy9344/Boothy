use std::{fs, path::Path};

/// Story 7.7. `bundle.resources`가 가리키는 오프라인 페이로드 자리를 **보장만** 한다.
///
/// `tauri-build`는 존재하지 않는 resource 경로를 컴파일 에러로 만든다. 벤더 페이로드는
/// 저장소에 없으므로(`release/vendor/`, `release/dist/`는 `.gitignore`), 자리를 만들어 두지
/// 않으면 `cargo test`조차 돌지 않는다.
///
/// **자리를 만들 뿐 내용을 지어내지 않는다.** 비어 있는 자리는 `PAYLOAD-NOT-STAGED.md`와
/// `staging: "not-staged"` 인벤토리로 **스스로 결손을 신고한다.** 그 상태의 산출물은
/// `release:verify`에서 막히고, 설치되더라도 `--self-check`가 `inventory-not-staged`로 실패한다.
///
/// 이미 staged된 실제 페이로드는 **절대 덮어쓰지 않는다.**
const NOT_STAGED_MARKER: &str = "PAYLOAD-NOT-STAGED.md";

fn main() {
    ensure_offline_payload_placeholders();
    tauri_build::build()
}

fn ensure_offline_payload_placeholders() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let Some(repo_root) = Path::new(&manifest_dir).parent() else {
        return;
    };

    ensure_marked_directory(
        &repo_root.join("release").join("dist").join("canon-helper"),
        "camera-helper",
        "release/README.md 의 `dotnet publish ... --self-contained true` 절차로 채운다.",
    );
    ensure_marked_directory(
        &repo_root
            .join("release")
            .join("vendor")
            .join("darktable-5.4.1"),
        "raw-renderer (darktable 5.4.1)",
        "release/README.md 의 darktable 조달 절차로 채운다.",
    );
    ensure_not_staged_inventory(
        &repo_root
            .join("release")
            .join("dist")
            .join("inventory.json"),
    );
}

fn ensure_marked_directory(directory: &Path, component: &str, remedy: &str) {
    if directory.is_dir() {
        return;
    }

    if fs::create_dir_all(directory).is_err() {
        return;
    }

    let body = format!(
        "# 이 자리는 아직 채워지지 않았습니다\n\n\
         - 구성요소: `{component}`\n\
         - 채우는 방법: {remedy}\n\n\
         이 파일이 설치본 안에서 발견되면 그 설치본은 release candidate가 아닙니다.\n\
         `release:verify`가 `inventory-component-missing`으로 막고,\n\
         `boothy.exe --self-check`가 종료 코드 1로 실패합니다.\n"
    );
    let _ = fs::write(directory.join(NOT_STAGED_MARKER), body);
    println!("cargo:warning=Story 7.7: {component} 페이로드가 staged 되지 않아 결손 표시 자리를 만들었습니다. 이 빌드는 release candidate가 아닙니다.");
}

fn ensure_not_staged_inventory(inventory_path: &Path) {
    if inventory_path.is_file() {
        return;
    }

    let Some(parent) = inventory_path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }

    let body = concat!(
        "{\n",
        "  \"schemaVersion\": \"release-inventory/v1\",\n",
        "  \"staging\": \"not-staged\",\n",
        "  \"reason\": \"release:stage 가 실행되지 않았습니다. 이 빌드는 release candidate가 아닙니다.\"\n",
        "}\n"
    );
    let _ = fs::write(inventory_path, body);
    println!("cargo:warning=Story 7.7: release/dist/inventory.json 이 없어 not-staged 인벤토리를 놓았습니다. 이 빌드는 release candidate가 아닙니다.");
}
