# Tech Stack

## Existing Technology Stack

| Category | Current Technology | Version | Usage in Enhancement | Notes |
| --- | --- | --- | --- | --- |
| Desktop App Framework | Tauri | `2.9.x` | **Boothy 메인 앱 런타임** | RapidRAW가 이미 Tauri 기반이며 Windows 번들링(NSIS) 구성 존재 |
| Frontend UI | React | `19.2.3` | **Boothy UI 베이스** | RapidRAW UI 스타일/컴포넌트를 기준으로 Boothy 화면 추가 |
| Frontend Language | TypeScript | `5.9.3` | **Boothy UI 개발 언어** | RapidRAW 프론트엔드와 동일 |
| Frontend Build | Vite | `7.3.0` | **프론트 빌드/번들** | RapidRAW 스택 유지 |
| Styling | Tailwind CSS | `3.4.19` | **스타일링** | RapidRAW UI 스타일 일관성 유지에 활용 |
| Animation/UI Utils | framer-motion, lucide-react 등 | `12.23.x`, `0.562.x` | **UI 상호작용** | RapidRAW 의존성 범위 내에서 사용 |
| Backend Language | Rust | `1.92` (edition `2024`) | **Tauri backend(Boothy 로직)** | 세션 관리/파일 감지/프리셋 적용 파이프라인 오케스트레이션 |
| Async Runtime | tokio | `1.x` | **백그라운드 처리** | import/프리셋 적용/썸네일/Export 작업 큐잉 및 UI 비블로킹(NFR4) |
| GPU/Image | wgpu, image, rawler 등 | `28.0`, `0.25.x`, (path) | **RAW 처리/프리셋 적용/Export** | RapidRAW 코어를 재사용(CR1), Boothy는 “언제/어떤 프리셋을 적용”만 제어 |
| Camera Reference Stack | digiCamControl | `2.0.0` | **카메라 기능 기준/재사용 후보** | .NET Framework 4.0 + WPF(제품 UI는 사용 금지), 기능/SDK 연동은 재사용 가능 |
| Canon SDK | Canon EDSDK | (license-dependent DLL) | **MVP Canon 지원** | 내부 매장 배포용 인스톨러에 DLL 번들링(EDSDK), x86/x64 정합성 확인 필요 |
| IPC (camera) | Windows Named Pipe | (N/A) | **Boothy ↔ Camera Sidecar 통신** | digiCamControl에 Named Pipe 패턴 관찰, Boothy는 버전드 IPC 계약으로 확장 |
| Packaging | NSIS (Tauri bundler) | (config) | **Windows 설치 패키징** | `reference/uxui_presetfunction/src-tauri/tauri.conf.json`에 nsis 설정 존재 |
| Logging | log + fern (Rust), log4net(.NET) | `0.4`, `0.7`, (legacy) | **진단/현장 디버깅** | correlation id 기반으로 end-to-end 로그 연결(NFR8) 필요 |

## New Technology Additions

| Technology | Version | Purpose | Rationale | Integration Method |
| --- | --- | --- | --- | --- |
| `notify` (Rust crate) | latest compatible | 세션 폴더 실시간 감지 | NFR3(≤1s) 달성을 위해 polling보다 이벤트 기반 감지가 유리 | Tauri backend에서 `Raw/` 신규 파일 생성/완료 감지에 사용 |
| `argon2` (Rust crate) | latest compatible | admin 비밀번호 salted hash | 단순 해시(sha2)보다 password hashing에 적합, NFR6 충족 | 설정 저장 시 hash+salt만 저장(AppData) |
