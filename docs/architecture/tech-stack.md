# Tech Stack

## Frontend

- React 19 + TypeScript (Vite)
- Tailwind CSS
- Tauri 2 (`@tauri-apps/api` 및 플러그인: dialog/os/process/shell 등)

## Backend

- Rust (edition 2024), Tauri 2.9
- Async/parallel: Tokio + Rayon
- GPU processing: wgpu 28 + WGSL shaders
- RAW develop: `rawler`
- Export/IO/utilities: `image`, `serde/serde_json`, `walkdir`, `tempfile`, `trash`, `chrono` 등

## External / Constraints

- Canon EDSDK: 사용자 로컬 설치(경로 설정), Windows-only, SDK 파일을 repo/installer에 포함하지 않음
- (업스트림) AI/ComfyUI/Clerk 등 네트워크 의존 기능은 키오스크 운영 리스크가 있어 Boothy 범위에서 비활성화/제거 여부를 명확히 관리

## Build / Packaging

- Frontend: `npm run start`(tauri dev), `npm run build`(vite build)
- Desktop bundle: `tauri build`
- Windows installer: NSIS(업스트림 tauri config 기반)

---
