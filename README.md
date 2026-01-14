# Boothy

Boothy는 두 개의 OSS 레퍼런스(카메라/편집)를 기반으로 **단일 Windows 앱(Tauri + React)** 으로 “촬영 → 실시간 확인 → 프리셋/편집 → Export”까지 하나의 UX로 통합하는 프로젝트입니다.

## Repo 구조(현재)

- `docs/`: PRD/아키텍처/UX 정책 문서
- `reference/`: 기능 참고용 벤더/레퍼런스 코드
  - `reference/camerafunction/`: digiCamControl 2.0.0 (C#/.NET Framework 4.0, WPF) — 기능 참고용
  - `reference/uxui_presetfunction/`: RapidRAW (React/Vite + Tauri/Rust) — 편집/프리셋/Export 베이스 참고용

현재 리포 루트에는 “1차 Boothy 앱 코드”가 아직 없습니다. (계획: RapidRAW를 `apps/boothy`로 승격 후 제품 코드로 발전)

## 빠른 실행(레퍼런스: RapidRAW)

Windows에서 아래 실행이 가능합니다:

1) `reference/uxui_presetfunction`로 이동  
2) `npm install`  
3) `npm start`

필요 도구(권장): Node.js 22, Rust 1.92, Tauri CLI 2.9.x, Visual Studio Build Tools + Windows SDK

## 제품 정책(MVP)

- **오프라인/무계정:** 기본 워크플로우는 네트워크 없이 동작해야 하며, 로그인/커뮤니티/클라우드/모델 다운로드 등 온라인 기능은 Boothy 빌드에서 제거/비활성화합니다.
- **라이선스/배포 게이트:** RapidRAW(AGPL-3.0) 및 Canon EDSDK 재배포 조건이 확정되기 전에는 외부 배포를 하지 않고 내부 테스트 배포로 제한합니다.
