# Runtime Capability Model 계약

## 목적

이 문서는 booth, operator, authoring surface가 같은 runtime profile / capability snapshot baseline을 공유하도록 고정한다.

## Authoritative Source / 소비 경계

- 문서 기준: 이 문서
- TypeScript 기준: `src/shared-contracts/schemas/capabilities.ts`, `src/app/services/capability-service.ts`
- Rust 기준: `src-tauri/src/commands/runtime_commands.rs`, `src-tauri/src/contracts/dto.rs`
- Tauri window capability 파일:
  - `src-tauri/capabilities/booth-window.json`
  - `src-tauri/capabilities/operator-window.json`
  - `src-tauri/capabilities/authoring-window.json`
- 대표 fixture: `tests/fixtures/contracts/runtime-capability-authoring-enabled.json`

## Snapshot Shape

```json
{
  "isAdminAuthenticated": true,
  "allowedSurfaces": ["booth", "operator", "authoring", "settings"]
}
```

## 규칙

- `booth`는 항상 포함된다.
- `isAdminAuthenticated=false`면 frontend는 admin-only surface를 열어도 접근 권한을 주면 안 된다.
- runtime profile baseline:
  - `booth`: `["booth"]`
  - `operator-enabled`: `["booth", "operator", "settings"]`
  - `authoring-enabled`: `["booth", "operator", "authoring", "settings"]`
- window boundary baseline:
  - `operator` surface는 `operator-window`에서만 열린다.
  - `authoring` surface는 `authoring-window`에서만 열린다.
  - `settings`는 operator/authoring 관리 문맥에서만 허용된다.

## 범위 경계

- Story 1.14가 닫는 범위:
  - runtime profile -> allowed surface mapping
  - frontend capability snapshot normalization
  - Tauri window label baseline
- Story 1.15 이후가 닫는 범위:
  - helper/profile deeper operational policy
- Story 1.16이 닫는 범위:
  - build/release에 필요한 capability packaging 검증
