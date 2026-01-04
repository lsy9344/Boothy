# Coding Standards

## General

- 브라운필드 원칙: “기존 동작 유지 + 신규 기능 추가”를 기본으로 하고, `invoke`/event 계약 이름 변경은 회귀 리스크가 높으므로 최소화한다.
- Customer Mode는 “화이트리스트 노출”을 원칙으로 한다(숨길 컨트롤을 나열하기보다, 보여줄 것만 명시).
- Reset은 “앱 상태/캐시/백그라운드 작업 정리”만 수행하며 세션 폴더의 이미지/sidecar 파일은 삭제하지 않는다(확정 요구사항).

## Frontend (React/TypeScript)

- 기존 ESLint/Prettier/tsconfig 규칙을 따른다.
- 모드/상태머신 전이는 App Shell 레벨에서 단일 소스로 관리하고(분기 난립 금지), UI에서는 상태에 따라 가능한 액션만 활성화한다.
- Customer Mode 에러 UX: “짧고 구체적 + 도움 요청 1가지 액션”만 제공하고, 상세 원인/로그/재시도는 Admin 전용으로 제공한다.

## Backend (Rust/Tauri)

- Windows-only(EOS 700D/EDSDK) 코드는 feature flag 또는 플랫폼 가드로 격리해 macOS 개발 흐름을 막지 않는다.
- 긴 작업(Export/큐 처리/인덱싱 등)은 백그라운드에서 처리하고, 진행률은 이벤트로 UI에 전달한다.
- 동시성(토키오/레이온) 작업은 Cancel/Reset 시 안전하게 정리되도록 설계한다(데드락/리소스 누수 방지).
- 외부 SDK/경로/권한 실패는 “진입 차단 + 진단 가능” 상태로 처리하고, Admin에서 원인/로그를 제공한다.

## Testing & Validation (권장 최소 게이트)

- Frontend: lint/typecheck + build (`npm run build`)
- Backend: `cargo check`(또는 `cargo test` 가능 범위) + 핵심 시나리오 스모크(상태머신/ExportLock/Reset)
- 회귀 위험이 큰 구간(커맨드 계약, ExportLock, Reset)은 자동화된 최소 스모크를 우선한다.

