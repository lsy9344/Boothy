# Boothy (RapidTetherRAW) Brownfield Enhancement Architecture

문서 버전: v0.1 (Draft)  
작성일: 2026-01-02  
관련 PRD: `docs/prd.md` (sharded at `docs/prd/`)  

---

## Overview

Boothy(working title: RapidTetherRAW)는 RapidRAW(업스트림) 기반의 Windows 전용 테더 촬영 + RAW 현상 + Export 데스크톱 앱이다. 본 문서는 “Customer Mode(무인 키오스크 플로우) + Admin Mode(PIN) + EOS 700D EDSDK 테더링 + Smart Export Pipeline + ExportLock 게이트 + 프라이버시 리셋”을 기존 RapidRAW 구조에 안전하게 통합하기 위한 아키텍처 가이드를 제공한다.

프로젝트 컨텍스트:

- 업스트림 코드: `upstream/RapidRAW` (commit `a931728`) — 구현 시 실제 파일/엔트리포인트는 업스트림 기준으로 검증한다.
- 운영 환경: Windows 10/11 x64
- 개발 환경: macOS에서 개발 가능하나 EDSDK/테더 기능은 Windows 실검증이 필요하다.
- 컴플라이언스: RapidRAW 기반 **AGPL-3.0** 준수, Canon EDSDK는 **비동봉/비재배포**(사용자 로컬 설치 + 경로 설정)

---

## Tech Stack

### Frontend

- React 19 + TypeScript (Vite)
- Tailwind CSS
- Tauri 2 (`@tauri-apps/api` 및 플러그인: dialog/os/process/shell 등)

### Backend

- Rust (edition 2024), Tauri 2.9
- Async/parallel: Tokio + Rayon
- GPU processing: wgpu 28 + WGSL shaders
- RAW develop: `rawler`
- Export/IO/utilities: `image`, `serde/serde_json`, `walkdir`, `tempfile`, `trash`, `chrono` 등

### External / Constraints

- Canon EDSDK: 사용자 로컬 설치(경로 설정), Windows-only, SDK 파일을 repo/installer에 포함하지 않음
- (업스트림) AI/ComfyUI/Clerk 등 네트워크 의존 기능은 키오스크 운영 리스크가 있어 Boothy 범위에서 비활성화/제거 여부를 명확히 관리

### Build / Packaging

- Frontend: `npm run start`(tauri dev), `npm run build`(vite build)
- Desktop bundle: `tauri build`
- Windows installer: NSIS(업스트림 tauri config 기반)

---

## Source Tree

Boothy 저장소의 현재 형태는 “분석/요구사항 문서 + 업스트림 코드 클론” 구성이다.

### Root

- `docs/`: PRD/Architecture/Stories 등 BMAD 문서 루트
- `upstream/RapidRAW/`: RapidRAW 업스트림 코드(참조 및 포크 베이스)

### Upstream RapidRAW (핵심 엔트리포인트)

Frontend (React/TS):

- `upstream/RapidRAW/src/main.tsx`: React entry
- `upstream/RapidRAW/src/App.tsx`: App shell(뷰/상태 오케스트레이션 후보)
- `upstream/RapidRAW/src/components/ui/AppProperties.tsx`: Frontend↔Backend invoke 계약(문자열 커맨드 enum)
- `upstream/RapidRAW/src/components/panel/*`: Editor/FolderTree/Filmstrip 및 패널 구조

Backend (Rust / Tauri):

- `upstream/RapidRAW/src-tauri/src/main.rs`: Tauri entrypoint + command handler registration
- `upstream/RapidRAW/src-tauri/src/file_management.rs`: 라이브러리/썸네일/프리셋/설정/sidecar 관련 기능
- `upstream/RapidRAW/src-tauri/src/raw_processing.rs`: RAW develop
- `upstream/RapidRAW/src-tauri/src/gpu_processing.rs`: GPU compute pipeline
- `upstream/RapidRAW/src-tauri/src/shaders/*.wgsl`: GPU shaders

### Boothy Enhancement Code Placement (권장)

포크 구현 시 신규 기능은 기존 구조를 유지하면서 모듈 단위로 추가한다:

- Frontend: `src/modes/*`, `src/session/*` 등 Customer/Admin 상태머신 및 화면 단위 모듈화(기존 패널/컴포넌트는 재사용 우선)
- Backend: `src-tauri/src/tethering/*`(EDSDK), `src-tauri/src/session/*`(세션/ExportLock/큐/리셋)처럼 격리하여 `main.rs` 커맨드 표면을 통제

---

## Coding Standards

### General

- 브라운필드 원칙: “기존 동작 유지 + 신규 기능 추가”를 기본으로 하고, `invoke`/event 계약 이름 변경은 회귀 리스크가 높으므로 최소화한다.
- Customer Mode는 “화이트리스트 노출”을 원칙으로 한다(숨길 컨트롤을 나열하기보다, 보여줄 것만 명시).
- Reset은 “앱 상태/캐시/백그라운드 작업 정리”만 수행하며 세션 폴더의 이미지/sidecar 파일은 삭제하지 않는다(확정 요구사항).

### Frontend (React/TypeScript)

- 기존 ESLint/Prettier/tsconfig 규칙을 따른다.
- 모드/상태머신 전이는 App Shell 레벨에서 단일 소스로 관리하고(분기 난립 금지), UI에서는 상태에 따라 가능한 액션만 활성화한다.
- Customer Mode 에러 UX: “짧고 구체적 + 도움 요청 1가지 액션”만 제공하고, 상세 원인/로그/재시도는 Admin 전용으로 제공한다.

### Backend (Rust/Tauri)

- Windows-only(EOS 700D/EDSDK) 코드는 feature flag 또는 플랫폼 가드로 격리해 macOS 개발 흐름을 막지 않는다.
- 긴 작업(Export/큐 처리/인덱싱 등)은 백그라운드에서 처리하고, 진행률은 이벤트로 UI에 전달한다.
- 동시성(토키오/레이온) 작업은 Cancel/Reset 시 안전하게 정리되도록 설계한다(데드락/리소스 누수 방지).
- 외부 SDK/경로/권한 실패는 “진입 차단 + 진단 가능” 상태로 처리하고, Admin에서 원인/로그를 제공한다.

### Testing & Validation (권장 최소 게이트)

- Frontend: lint/typecheck + build (`npm run build`)
- Backend: `cargo check`(또는 `cargo test` 가능 범위) + 핵심 시나리오 스모크(상태머신/ExportLock/Reset)
- 회귀 위험이 큰 구간(커맨드 계약, ExportLock, Reset)은 자동화된 최소 스모크를 우선한다.

