# Development Environment

## Local Setup (MVP)

**Target OS:** Windows 10/11

**Required Tooling (권장):**
- Node.js `22` + npm (RapidRAW workflows 기준)
- Rust `1.92` (edition `2024`) + `cargo`
- Tauri CLI `2.9.x` (`@tauri-apps/cli`)
- (Windows) Visual Studio 2022 Build Tools + Windows SDK (Tauri 빌드 필수)
- (Sidecar) .NET SDK / Visual Studio (sidecar 타겟 프레임워크에 맞춤)
- (Camera) Canon EDSDK DLL 번들 준비(내부 매장 배포)

**Human-only prerequisites (MVP):**
- Canon EDSDK 배포 방식: 내부 매장 배포는 번들링(외부 배포는 별도 승인)
- 대상 PC에 카메라 드라이버/SDK 설치(오프라인 현장 설치 절차 포함)
- 배포 형태 결정(내부 매장 배포 vs 외부 배포) 및 라이선스 준수(AGPL 포함)

**Local Run (현재 리포 상태 기준):**
- RapidRAW 레퍼런스 실행: `reference/uxui_presetfunction`에서 `npm install` → `npm start`
- Boothy 앱(타겟): `reference/uxui_presetfunction`을 `apps/boothy`로 승격한 뒤, `apps/boothy`에서 `npm install` → `npm run start`(또는 `npm run tauri dev`)
- Sidecar(타겟): `apps/camera-sidecar`에서 빌드/실행 후, Boothy가 IPC로 연결

**Diagnostics:**
- 앱 로그: `%APPDATA%\\Boothy\\logs\\boothy.log`
- sidecar 로그: `%APPDATA%\\Boothy\\logs\\camera-sidecar.log`
- 세션 데이터: `%USERPROFILE%\\Pictures\\dabi_shoot\\<session>\\{Raw,Jpg}`
