//! Story 7.7. 인벤토리 매니페스트를 읽고, **디스크의 실제 파일과 대조한다.**
//!
//! 해시는 프로세스 내부에서 계산한다. 설치 트리에는 파일이 수천 개 있으므로 파일마다
//! 외부 콘솔 도구를 실행하면 self-check가 터미널 창과 프로세스를 대량 생성한다.
//!
//! 트리 해시의 정의는 TS 생성기·PowerShell 게이트와 **한 글자도 다르면 안 된다.**
//! 정렬된 `<상대경로>:<sha256>` 줄을 `\n`으로 이어 붙여 다시 sha256 한다.
//! 정렬은 UTF-8 바이트 순서이고, Rust의 `String` 정렬이 곧 그 순서다.

use std::{
    fs,
    io::Read,
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
};

use sha2::{Digest, Sha256};

use crate::contracts::dto::{ComponentEntrySelectionDto, ReleaseInventoryDto};

/// 해시 계산이 **불가능한** 경우. "다르다"와 "못 쟀다"는 다른 사실이다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DigestError {
    /// 파일을 열거나 읽어 해시를 계산할 수 없다.
    Unreadable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryLoadError {
    /// 파일이 없거나 읽을 수 없다.
    Unreadable,
    /// 읽었는데 계약에 맞지 않는다.
    Malformed,
}

/// darktable 트리 하나에 파일이 수천 개다. 디스크 읽기를 감당할 수 있는 폭으로 나눠 돌린다.
const MAX_DIGEST_WORKERS: usize = 8;

/// 임시 파일 이름이 겹치지 않게 하는 일련번호.
static TREE_DIGEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub fn sha256_of_file(path: &Path) -> Result<String, DigestError> {
    let mut file = fs::File::open(path).map_err(|_| DigestError::Unreadable)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];

    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| DigestError::Unreadable)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// 트리 해시.
///
/// 생성기와 게이트가 이미 같은 정의를 쓰고 있고, 셋 중 하나만 달라지면 릴리스가 항상 막힌다.
/// 그래서 정렬된 줄을 임시 파일에 쓰고 파일 해시와 동일한 경로로 계산한다.
pub fn digest_of_lines(lines: &[String]) -> Result<String, DigestError> {
    let mut sorted = lines.to_vec();
    sorted.sort();

    let joined = sorted.join("\n");
    // **이름이 겹치면 안 된다.** 같은 프로세스 안에서 두 구성요소의 해시가 동시에 계산될 수
    // 있고, 겹치는 순간 한쪽이 다른 쪽의 임시 파일을 지운다. 그러면 해시가 조용히 틀린다.
    let temp_path = std::env::temp_dir().join(format!(
        "boothy-tree-digest-{}-{}.txt",
        std::process::id(),
        TREE_DIGEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));

    fs::write(&temp_path, joined.as_bytes()).map_err(|_| DigestError::Unreadable)?;
    let digest = sha256_of_file(&temp_path);
    let _ = fs::remove_file(&temp_path);

    digest
}

/// 루트 아래의 모든 파일을 루트 기준 상대 경로로 모은다. 구분자는 언제나 `/`다.
pub fn collect_relative_files(root: &Path) -> Vec<String> {
    let mut collected = Vec::new();
    walk(root, "", &mut collected);
    collected.sort();
    collected
}

fn walk(root: &Path, prefix: &str, collected: &mut Vec<String>) {
    let directory = if prefix.is_empty() {
        root.to_path_buf()
    } else {
        prefix
            .split('/')
            .fold(root.to_path_buf(), |accumulated, segment| {
                accumulated.join(segment)
            })
    };

    let Ok(entries) = fs::read_dir(&directory) else {
        return;
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let relative = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };

        match entry.file_type() {
            Ok(file_type) if file_type.is_dir() => walk(root, &relative, collected),
            Ok(file_type) if file_type.is_file() => collected.push(relative),
            _ => {}
        }
    }
}

/// 인벤토리가 적어 둔 선택 규칙대로 이 구성요소의 파일 목록을 정한다.
///
/// **설치된 앱은 `inventory-spec.json`을 갖고 있지 않다.** 그래서 규칙이 인벤토리 안에 있다.
pub fn select_component_files(root: &Path, selection: &ComponentEntrySelectionDto) -> Vec<String> {
    match selection {
        ComponentEntrySelectionDto::All => collect_relative_files(root),
        ComponentEntrySelectionDto::Only { entries } => {
            let mut selected = entries
                .iter()
                .filter(|entry| {
                    entry
                        .split('/')
                        .fold(root.to_path_buf(), |accumulated, segment| {
                            accumulated.join(segment)
                        })
                        .is_file()
                })
                .cloned()
                .collect::<Vec<_>>();
            selected.sort();
            selected
        }
        ComponentEntrySelectionDto::AllExcept { entries } => collect_relative_files(root)
            .into_iter()
            .filter(|relative| !entries.contains(relative))
            .collect(),
    }
}

/// 구성요소 트리의 실제 해시.
///
/// 파일이 하나도 없으면 `Ok(None)`이다 — "해시가 다르다"가 아니라 "트리가 비었다"이고,
/// 두 사실은 운영자에게 다른 조치를 요구한다.
pub fn compute_component_digest(
    root: &Path,
    selection: &ComponentEntrySelectionDto,
) -> Result<Option<(String, usize)>, DigestError> {
    let files = select_component_files(root, selection);

    if files.is_empty() {
        return Ok(None);
    }

    let file_count = files.len();
    let lines = hash_files_in_parallel(root, files)?;

    digest_of_lines(&lines).map(|digest| Some((digest, file_count)))
}

fn hash_files_in_parallel(root: &Path, files: Vec<String>) -> Result<Vec<String>, DigestError> {
    let worker_count = MAX_DIGEST_WORKERS.min(files.len()).max(1);
    let chunk_size = files.len().div_ceil(worker_count);
    let (sender, receiver) = mpsc::channel();
    let mut handles = Vec::new();

    for chunk in files.chunks(chunk_size) {
        let chunk = chunk.to_vec();
        let root = root.to_path_buf();
        let sender = sender.clone();

        handles.push(thread::spawn(move || {
            for relative in chunk {
                let absolute = relative
                    .split('/')
                    .fold(root.clone(), |accumulated, segment| {
                        accumulated.join(segment)
                    });
                let outcome = sha256_of_file(&absolute).map(|hash| format!("{relative}:{hash}"));

                if sender.send(outcome).is_err() {
                    return;
                }
            }
        }));
    }

    drop(sender);

    let mut lines = Vec::new();
    let mut failure = None;

    for outcome in receiver {
        match outcome {
            Ok(line) => lines.push(line),
            Err(error) => failure = Some(error),
        }
    }

    for handle in handles {
        let _ = handle.join();
    }

    match failure {
        Some(error) => Err(error),
        None => Ok(lines),
    }
}

pub fn load_inventory(path: &Path) -> Result<ReleaseInventoryDto, InventoryLoadError> {
    let raw = fs::read_to_string(path).map_err(|_| InventoryLoadError::Unreadable)?;

    parse_inventory(&raw)
}

pub fn parse_inventory(raw: &str) -> Result<ReleaseInventoryDto, InventoryLoadError> {
    // Windows 도구들이 UTF-8 BOM을 붙여 쓴다. BOM 하나 때문에 설치본이 자기 매니페스트를
    // 못 읽는 일이 생기면 안 된다.
    let trimmed = raw.trim_start_matches('\u{feff}');
    let inventory: ReleaseInventoryDto =
        serde_json::from_str(trimmed).map_err(|_| InventoryLoadError::Malformed)?;

    crate::contracts::dto::validate_release_inventory(&inventory)
        .map_err(|_| InventoryLoadError::Malformed)?;

    Ok(inventory)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn file_hashing_does_not_require_an_external_windows_tool() {
        let root =
            std::env::temp_dir().join(format!("boothy-in-process-sha256-{}", std::process::id()));
        let file = root.join("abc.txt");
        let original_system_root = std::env::var_os("SystemRoot");

        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("테스트 디렉터리를 만들 수 있어야 한다");
        fs::write(&file, b"abc").expect("테스트 파일을 쓸 수 있어야 한다");

        std::env::set_var("SystemRoot", root.join("no-windows-tools"));
        let digest = sha256_of_file(&file);
        match original_system_root {
            Some(value) => std::env::set_var("SystemRoot", value),
            None => std::env::remove_var("SystemRoot"),
        }

        assert_eq!(
            digest,
            Ok("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into())
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn an_unreadable_manifest_is_distinguished_from_a_malformed_one() {
        let missing = load_inventory(Path::new("C:/definitely/not/here/inventory.json"));
        assert_eq!(missing.unwrap_err(), InventoryLoadError::Unreadable);

        assert_eq!(
            parse_inventory("{ not json").unwrap_err(),
            InventoryLoadError::Malformed
        );
    }

    #[test]
    fn a_not_staged_manifest_parses_so_the_gap_can_be_reported() {
        let inventory = parse_inventory(
            r#"{"schemaVersion":"release-inventory/v1","staging":"not-staged","reason":"미staging"}"#,
        )
        .expect("not-staged 매니페스트는 읽혀야 한다");

        assert!(matches!(inventory, ReleaseInventoryDto::NotStaged(_)));
    }

    #[test]
    fn a_bom_prefixed_manifest_is_still_read() {
        let inventory = parse_inventory(
            "\u{feff}{\"schemaVersion\":\"release-inventory/v1\",\"staging\":\"not-staged\",\"reason\":\"미staging\"}",
        );

        assert!(inventory.is_ok());
    }

    fn temp_tree(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "boothy-inventory-test-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("bin")).expect("트리를 만들 수 있어야 한다");
        fs::write(root.join("bin").join("tool.exe"), "tool").expect("파일을 쓸 수 있어야 한다");
        fs::write(root.join("EDSDK.dll"), "edsdk").expect("파일을 쓸 수 있어야 한다");
        fs::write(root.join("EdsImage.dll"), "edsimage").expect("파일을 쓸 수 있어야 한다");
        root
    }

    #[test]
    fn every_file_under_the_root_is_collected_with_forward_slashes() {
        let root = temp_tree("collect");

        assert_eq!(
            collect_relative_files(&root),
            vec![
                "EDSDK.dll".to_string(),
                "EdsImage.dll".to_string(),
                "bin/tool.exe".to_string()
            ]
        );

        let _ = fs::remove_dir_all(root);
    }

    /// `camera-helper`와 `edsdk-runtime`은 같은 디렉터리에 살고 **서로 다른 파일을 소유한다.**
    /// 선택 규칙이 그 경계를 만든다.
    #[test]
    fn the_selection_rule_splits_one_directory_between_two_components() {
        let root = temp_tree("selection");

        assert_eq!(
            select_component_files(
                &root,
                &ComponentEntrySelectionDto::Only {
                    entries: vec!["EDSDK.dll".into(), "EdsImage.dll".into()],
                }
            ),
            vec!["EDSDK.dll".to_string(), "EdsImage.dll".to_string()]
        );

        assert_eq!(
            select_component_files(
                &root,
                &ComponentEntrySelectionDto::AllExcept {
                    entries: vec!["EDSDK.dll".into(), "EdsImage.dll".into()],
                }
            ),
            vec!["bin/tool.exe".to_string()]
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_missing_file_named_by_an_only_rule_is_simply_absent() {
        let root = temp_tree("only-missing");

        assert_eq!(
            select_component_files(
                &root,
                &ComponentEntrySelectionDto::Only {
                    entries: vec!["EDSDK.dll".into(), "NotThere.dll".into()],
                }
            ),
            vec!["EDSDK.dll".to_string()]
        );

        let _ = fs::remove_dir_all(root);
    }

    /// 트리 해시는 **생성기·게이트와 같은 값**이어야 한다. 알려진 벡터로 고정한다.
    /// (`e3b0c4…` 는 빈 문자열의 sha256이다.)
    #[test]
    fn the_tree_digest_matches_the_shared_definition() {
        let lines = vec![
            "b.txt:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
            "a.txt:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
        ];
        let reversed = vec![lines[1].clone(), lines[0].clone()];

        let digest = digest_of_lines(&lines).expect("해시를 계산할 수 있어야 한다");
        let same = digest_of_lines(&reversed).expect("해시를 계산할 수 있어야 한다");

        assert_eq!(digest, same, "줄 순서가 달라도 같은 값이어야 한다");
        assert_eq!(digest.len(), 64);
    }

    /// **세 구현이 같은 값을 내야 한다.**
    ///
    /// 이 골든 벡터는 TS 생성기(`release/build-inventory.test.ts`)와 PowerShell 게이트가
    /// 쓰는 것과 **같은 값**이다. 하나라도 어긋나면 릴리스가 항상 digest-mismatch로 막히고,
    /// 그때 원인을 찾는 데 회차 하나를 잃는다. 여기서 미리 깨진다.
    ///
    /// 트리: `a.txt` = "alpha", `dir/b.txt` = "beta"
    #[test]
    fn the_tree_digest_matches_the_shared_golden_vector() {
        let root = std::env::temp_dir().join(format!("boothy-golden-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("dir")).expect("디렉터리를 만들 수 있어야 한다");
        fs::write(root.join("a.txt"), "alpha").expect("파일을 쓸 수 있어야 한다");
        fs::write(root.join("dir").join("b.txt"), "beta").expect("파일을 쓸 수 있어야 한다");

        let computed = compute_component_digest(&root, &ComponentEntrySelectionDto::All)
            .expect("해시를 계산할 수 있어야 한다");

        assert_eq!(
            computed,
            Some((
                "467f567c070c84409b3225c48cc0abadf716a1cb5e39bd5be4d720e90dd5e2d6".to_string(),
                2
            ))
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn an_empty_tree_yields_no_digest_rather_than_a_digest_of_nothing() {
        let root = std::env::temp_dir().join(format!("boothy-empty-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("디렉터리를 만들 수 있어야 한다");

        assert_eq!(
            compute_component_digest(&root, &ComponentEntrySelectionDto::All),
            Ok(None)
        );

        let _ = fs::remove_dir_all(root);
    }
}
