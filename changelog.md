# Changelog (Boothy)

## 2026-01-04 — `npm start` 빌드 실패(ort-sys가 cdn.pyke.io에서 다운로드 시도) 해결

### 증상
- `npm start` 실행 시 Rust 빌드 단계에서 아래와 유사한 패닉 발생:
  - `ort-sys-2.0.0-rc.10 build.rs: Failed to GET https://cdn.pyke.io/... (os error 11001)`

### 원인
- `ort` 크레이트의 기본 feature에 `download-binaries`가 포함되어 있어, 빌드 시 `cdn.pyke.io`에서 ONNX Runtime 프리빌트 바이너리를 내려받으려 시도함.
- 이 프로젝트는 이미 `src-tauri/resources`에 ONNX Runtime dylib(`onnxruntime.dll` 등)을 포함하고, 런타임에 `ORT_DYLIB_PATH`로 경로를 지정하므로(예: `src-tauri/src/main.rs`), `ort` 쪽 다운로드 전략이 불필요/충돌 가능.

### 변경 사항(정확한 적용 지침)

#### 1) `upstream/RapidRAW/src-tauri/Cargo.toml`
- 기존:
  - `ort = { version = "2.0.0-rc.10", features = ["ndarray", "load-dynamic"] }`
- 변경:
  - `ort = { version = "2.0.0-rc.10", default-features = false, features = ["ndarray", "load-dynamic"] }`
- (설명용 주석 2줄 추가됨)

#### 2) `upstream/RapidRAW/src-tauri/src/kiosk_config.rs`
- 함수: `pin_utils::generate_salt()`
- 기존:
  - `let salt_bytes: Vec<u8> = (0..32).map(|_| rng.random()).collect();`
- 변경:
  - `let salt_bytes: Vec<u8> = (0..32).map(|_| rng.random::<u8>()).collect();`

### 다른 PC에서 “그대로 적용”하는 방법

#### 방법 A) 수동 수정
1. 위 “변경 사항”대로 두 파일을 동일하게 수정.

#### 방법 B) 패치로 적용
1. 아래 내용을 그대로 복사해 `boothy-fix.patch` 파일로 저장
2. **Boothy 루트(이 `changelog.md`가 있는 폴더)**에서 `git apply boothy-fix.patch` 실행

```diff
diff --git a/upstream/RapidRAW/src-tauri/Cargo.toml b/upstream/RapidRAW/src-tauri/Cargo.toml
--- a/upstream/RapidRAW/src-tauri/Cargo.toml
+++ b/upstream/RapidRAW/src-tauri/Cargo.toml
@@ -27,7 +27,9 @@ walkdir = "2.5.0"
 trash = "5.2.5"
 imageproc = "0.25.0"
-ort = { version = "2.0.0-rc.10", features = ["ndarray", "load-dynamic"] }
+# We ship our own ONNX Runtime dylib via `src-tauri/resources` (see `build.rs`),
+# so disable ort's default download/copy behavior to avoid fetching from cdn.pyke.io at build time.
+ort = { version = "2.0.0-rc.10", default-features = false, features = ["ndarray", "load-dynamic"] }
 ndarray = "0.16"
 reqwest = { version = "0.12", features = ["json", "multipart"] }
 tokio-tungstenite = { version = "0.28", features = ["native-tls"] }

diff --git a/upstream/RapidRAW/src-tauri/src/kiosk_config.rs b/upstream/RapidRAW/src-tauri/src/kiosk_config.rs
--- a/upstream/RapidRAW/src-tauri/src/kiosk_config.rs
+++ b/upstream/RapidRAW/src-tauri/src/kiosk_config.rs
@@ -284,7 +284,7 @@ pub mod pin_utils {
     /// Generate a random salt for PIN hashing
     pub fn generate_salt() -> String {
         let mut rng = rand::rng();
-        let salt_bytes: Vec<u8> = (0..32).map(|_| rng.random()).collect();
+        let salt_bytes: Vec<u8> = (0..32).map(|_| rng.random::<u8>()).collect();
         hex::encode(salt_bytes)
     }
```

### 검증 방법
1. Rust 빌드 확인:
   - `cd upstream/RapidRAW/src-tauri`
   - `cargo build`
2. 앱 실행:
   - `cd upstream/RapidRAW`
   - `npm start`

### 참고(필수는 아님)
- `upstream/RapidRAW/src-tauri/resources/onnxruntime.dll`(Windows)가 존재해야 런타임 로딩이 정상 동작함.
