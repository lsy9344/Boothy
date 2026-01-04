# Source Tree

Boothy 저장소의 현재 형태는 “분석/요구사항 문서 + 업스트림 코드 클론” 구성이다.

## Root

- `docs/`: PRD/Architecture/Stories 등 BMAD 문서 루트
- `upstream/RapidRAW/`: RapidRAW 업스트림 코드(참조 및 포크 베이스)

## Upstream RapidRAW (핵심 엔트리포인트)

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

## Boothy Enhancement Code Placement (권장)

포크 구현 시 신규 기능은 기존 구조를 유지하면서 모듈 단위로 추가한다:

- Frontend: `src/modes/*`, `src/session/*` 등 Customer/Admin 상태머신 및 화면 단위 모듈화(기존 패널/컴포넌트는 재사용 우선)
- Backend: `src-tauri/src/tethering/*`(EDSDK), `src-tauri/src/session/*`(세션/ExportLock/큐/리셋)처럼 격리하여 `main.rs` 커맨드 표면을 통제

---
