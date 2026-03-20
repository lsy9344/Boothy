# Story 1.1: 초기 프로젝트를 스타터 템플릿에서 설정

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

owner / brand operator로서,
고객용 부스, 운영자 콘솔, 내부 프리셋 저작 화면이 분리된 하나의 패키지형 런타임을 원한다.
그래서 고객용 기능을 내부 도구 노출 없이 안전한 데스크톱 기반 위에 출시할 수 있다.

## Acceptance Criteria

1. 초기 상태의 새 프로젝트를 MVP 개발용으로 초기화하면, 승인된 `Vite react-ts + Tauri` 기준선 위에 `/booth`, `/operator`, `/authoring`, `/settings` 최상위 surface가 존재해야 한다. 또한 운영자/저작 surface는 기본 고객 흐름에 노출되지 않고 capability check 뒤에 있어야 한다.
2. 기본 고객 실행 경로에서 앱을 열면 관리자 인증 없이 도달 가능한 화면은 booth surface뿐이어야 한다. 또한 고객 UI 어디에서도 운영자 제어 또는 내부 프리셋 저작 제어가 보이면 안 된다.

## Tasks / Subtasks

- [x] 공식 부트스트랩 경로로 프론트엔드/데스크톱 기반을 초기화한다. (AC: 1)
  - [x] 현재 루트에서 `pnpm create vite . --template react-ts --no-interactive` 기준으로 Vite React TypeScript 프로젝트를 생성한다.
  - [x] `@tauri-apps/cli`를 dev dependency로 추가하고 `pnpm tauri init` 기반으로 `src-tauri/`를 초기화한다.
  - [x] `package.json` 스크립트를 `pnpm dev`, `pnpm build`, `pnpm tauri` 흐름에 맞게 정리한다.
- [x] Tauri + Vite 통합 설정을 아키텍처 기준선에 맞춘다. (AC: 1)
  - [x] `src-tauri/tauri.conf.json`의 build 설정을 `devUrl: http://localhost:5173`, `frontendDist: ../dist` 기준으로 맞춘다.
  - [x] `vite.config.ts`를 Tauri 호환 설정으로 조정하고 고정 포트, `src-tauri` watch 제외, 디버그 빌드 sourcemap 규칙을 반영한다.
  - [x] Windows 개발 전제조건(Node, Rust MSVC, WebView2/빌드 도구) 확인 메모를 `README.md` 또는 적절한 개발 문서에 남긴다.
- [x] 최상위 route 및 surface 골격을 만든다. (AC: 1, 2)
  - [x] `src/app/routes.tsx`에 `/booth`, `/operator`, `/authoring`, `/settings` top-level route를 선언한다.
  - [x] `/` 또는 기본 진입점이 고객용 `booth` surface로 연결되게 한다.
  - [x] `src/booth-shell/`, `src/operator-console/`, `src/preset-authoring/`, `src/shared-ui/`, `src/shared-contracts/` 등 아키텍처가 요구하는 상위 디렉터리 골격과 placeholder screen/component를 만든다.
- [x] 내부 surface 비노출 및 capability gating 기준선을 만든다. (AC: 1, 2)
  - [x] booth surface는 기본 접근 가능 상태로 두고, operator/authoring/settings는 guard 또는 capability service 뒤로 숨긴다.
  - [x] 고객 UI 네비게이션, 버튼, 링크, 시작 화면 어디에서도 운영자/저작 진입점을 노출하지 않는다.
  - [x] 실제 관리자 비밀번호 기능이 아직 없더라도, 이후 story에서 연결할 수 있는 typed auth/capability seam을 만든다.
- [x] 호스트 호출 경계와 계약 계층의 시작점을 만든다. (AC: 1)
  - [x] React 컴포넌트가 직접 `invoke`를 호출하지 않도록 adapter/service 진입점 위치를 먼저 만든다.
  - [x] `src/shared-contracts/`에 Zod 4 기반 schema entrypoint와 공통 DTO 자리 표시자를 만든다.
  - [x] `src-tauri/capabilities/booth-window.json`, `operator-window.json`, `authoring-window.json`을 아키텍처 경로에 맞게 생성한다.
- [x] 부트스트랩 스모크 테스트를 추가한다. (AC: 1, 2)
  - [x] 기본 라우팅이 booth로 연결되는지 검증한다.
  - [x] operator/authoring route가 고객 기본 흐름에서 숨겨지는지 검증한다.
  - [x] 최소 1개의 contract 또는 schema smoke test로 shared-contracts 경계를 검증한다.

## Dev Notes

### 스토리 범위와 목적

- 이 스토리는 고객 가치를 직접 완성하는 기능 스토리가 아니라, 이후 Epic 1 구현이 같은 구조 위에서 안전하게 진행되도록 만드는 부트스트랩 선행 작업이다.
- 세션 생성, 프리셋 카탈로그, 카메라 연동, darktable 렌더링, 운영자 복구 기능을 이 스토리에서 완성하려고 하면 범위 초과다.
- 목표는 "작동하는 최소 앱 골격 + 올바른 경계 + 숨겨진 내부 surface"다.

### 현재 워크스페이스 상태

- 현재 저장소에는 계획 문서와 참조 문서만 있고, 실제 앱 스캐폴드는 아직 없다.
- 확인 결과 `package.json`, `pnpm-lock.yaml`, `src/`, `src-tauri/`, `Cargo.toml`이 존재하지 않는다.
- 따라서 dev agent는 brownfield 보정이 아니라 greenfield bootstrap으로 진행하면 된다.

### 구현 가드레일

- 아키텍처가 선택한 스타터는 `official Vite react-ts + manual Tauri CLI initialization`이다. `create-tauri-app` 전체 템플릿에 의존하는 방향으로 바꾸지 말 것. [Source: _bmad-output/planning-artifacts/architecture.md#Selected Starter: Official `Vite react-ts` + manual `Tauri CLI` initialization]
- 제품은 하나의 패키지된 Tauri 앱 안에 `booth`, `operator`, `authoring` 3개 capability-gated surface를 둔다. 기본 고객 surface는 booth다. [Source: _bmad-output/planning-artifacts/epics.md#Additional Requirements]
- React Router는 `/booth`, `/operator`, `/authoring`, `/settings` 같은 최상위 surface 중심으로 제한한다. [Source: _bmad-output/planning-artifacts/epics.md#Additional Requirements]
- UI 컴포넌트는 직접 `invoke`를 호출하지 않고 typed adapter/service 계층을 통해서만 호스트 기능에 접근해야 한다. [Source: _bmad-output/planning-artifacts/epics.md#Additional Requirements]
- 운영자/프리셋 저작 진입점은 고객 기본 흐름에서 숨겨져 있어야 하며 관리자 비밀번호 인증 전에는 시각적으로도 노출되면 안 된다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Reading Guide]
- 이 스토리에서 실제 관리자 인증 완성은 요구되지 않지만, 이후 story에서 연결할 수 있는 guard seam은 반드시 남겨야 한다.

### 아키텍처 준수사항

- 앱은 로컬 우선 Windows 데스크톱 Tauri 애플리케이션이다. [Source: _bmad-output/planning-artifacts/architecture.md#System Overview]
- Rust host가 장기적으로 세션, 타이밍, 캡처, 렌더, 완료 상태의 단일 정규화 계층이 된다. 이번 story에서는 그 경계를 허물지 않는 폴더 구조와 호출 경계만 먼저 만든다. [Source: _bmad-output/planning-artifacts/architecture.md#Core Architectural Decisions]
- 활성 세션의 진실은 나중에 세션 파일시스템 루트와 `session.json`이 소유해야 하므로, 지금 단계에서 React route state나 local component state를 장기 truth처럼 설계하면 안 된다. [Source: _bmad-output/planning-artifacts/epics.md#Additional Requirements]
- SQLite는 감사 로그용이지 사진/세션 자산의 원본 진실이 아니다. Story 1.1에서는 SQLite 중심 설계를 도입하지 말 것. [Source: _bmad-output/planning-artifacts/epics.md#Additional Requirements]

### 프로젝트 구조 요구사항

- 아키텍처 문서의 권장 루트 골격을 최대한 그대로 따른다.
- Story 1.1에서 최소한 아래 경로들은 생성 대상으로 본다.
  - `src/app/`
  - `src/shared-ui/`
  - `src/shared-contracts/`
  - `src/booth-shell/`
  - `src/operator-console/`
  - `src/preset-authoring/`
  - `src/session-domain/`
  - `src-tauri/src/`
  - `src-tauri/capabilities/`
- 모든 하위 도메인을 완성할 필요는 없지만, 이후 story에서 파일을 자연스럽게 추가할 수 있도록 디렉터리 문법은 먼저 맞춰 둔다. [Source: _bmad-output/planning-artifacts/architecture.md#Complete Project Directory Structure]

### 기술 요구사항

- 패키지 매니저는 `pnpm` 기준으로 통일한다. [Source: _bmad-output/planning-artifacts/architecture.md#Selected Starter: Official `Vite react-ts` + manual `Tauri CLI` initialization]
- 공식 Vite 문서 기준 2026-03-20 현재 `react-ts` 템플릿, 현재 디렉터리(`.`) 스캐폴딩, `--no-interactive` 사용이 가능하다. 또한 최신 Vite는 Node.js `20.19+` 또는 `22.12+`를 요구한다. [Source: https://vite.dev/guide/]
- 공식 Tauri v2 문서는 기존 프론트엔드가 있을 때 수동 `tauri init` 경로를 지원하며, Vite 기준 `devUrl=http://localhost:5173`, `frontendDist=../dist`, `pnpm run dev`, `pnpm run build` 구성을 예시로 제시한다. [Source: https://v2.tauri.app/start/create-project/] [Source: https://v2.tauri.app/start/frontend/vite/]
- React Router v7 공식 문서는 Vite 기반 시작 경로와 `createBrowserRouter` + `RouterProvider` 조합을 안내한다. 또한 React Router의 Vite plugin 채택 경로는 Node.js `20+`와 Vite `5+`를 전제로 한다. 이번 story에서는 plugin 도입 여부와 무관하게 route 객체 기반 구성을 우선 적용한다. [Source: https://reactrouter.com/start/data/installation] [Source: https://reactrouter.com/upgrading/router-provider]
- Zod 4는 안정 버전이며 루트 패키지 `zod`에서 사용할 수 있다. TypeScript `strict` 모드 활성화가 권장된다. [Source: https://zod.dev/] [Source: https://zod.dev/v4/versioning?id=update--july-8th-2025]

### 최신 기술 확인 메모

- 2026-03-20 기준으로 최신 공식 문서 흐름은 Tauri 2 + Vite SPA 조합이다. 아키텍처의 선택과 일치한다.
- 주의할 점은 Tauri prerequisite 문서의 Node 예시는 오래된 예시일 수 있고, 실제 최신 Vite 요구사항은 더 엄격하다. 로컬 개발 환경은 Node `20.19+` 이상으로 맞추는 편이 안전하다.
- React Router는 v7 계열을 기준으로 top-level route 객체 구성을 바로 시작할 수 있으므로, 별도 legacy router 패턴을 도입할 이유가 없다.
- Zod는 이번 story에서 전체 계약 완성이 아니라 "shared-contracts 경계의 시작점" 정도만 만들면 충분하다.

### 파일 구조/구현 세부 가이드

- `src/main.tsx`는 `RouterProvider`를 루트에 연결하는 단순 진입점으로 유지한다.
- `src/app/routes.tsx`는 route 정의만 담당하고, 각 surface 내부 구현 세부사항을 끌어안지 않는다.
- `src/app/App.tsx`가 필요하다면 shell composition 수준으로만 사용하고, 진짜 화면 책임은 `booth-shell`, `operator-console`, `preset-authoring` 아래 screen으로 보낸다.
- 고객 기본 surface에서는 내부 화면 링크를 렌더링하지 않는다. guard는 "링크 자체를 숨김"과 "직접 URL 접근 차단"을 둘 다 제공해야 한다.
- restricted route 접근 시 동작은 redirect 또는 blocked placeholder 둘 중 하나로 통일해도 되지만, 고객에게 내부 surface 존재를 친절히 설명하는 UX는 만들지 말 것.

### 테스트 요구사항

- 테스트는 이 story에서 "행동 보장"에 집중한다.
- 최소 검증 범위:
  - 앱 기본 진입 시 booth surface로 연결된다.
  - 고객 기본 흐름에서 operator/authoring 진입 링크가 보이지 않는다.
  - restricted route 직접 접근이 차단된다.
  - shared-contracts의 최소 schema 파싱이 동작한다.
- 이 story는 카메라, 파일시스템 세션 저장, darktable, operator recovery의 통합 테스트를 요구하지 않는다.
- 테스트 스택은 Vite/React 기준 경량 단위 테스트 조합을 선택하되, 아직 최종 E2E 프레임워크까지 잠그지는 말 것. [Source: _bmad-output/planning-artifacts/architecture.md#Testing Framework]

### 금지사항 / 안티패턴

- React 컴포넌트에서 직접 Tauri `invoke` 호출 금지
- booth UI 안에 operator/authoring 링크나 버튼 추가 금지
- session truth를 URL, local state, SQLite에 먼저 얹는 설계 금지
- Story 1.1에서 카메라 SDK, darktable 편집 UI, preset publication 기능까지 과구현하는 것 금지
- `create-tauri-app`가 만든 구조를 그대로 따르기 위해 아키텍처 디렉터리 문법을 포기하는 것 금지

### 참고 문서

- Epic 분해: `_bmad-output/planning-artifacts/epics.md`
- PRD: `_bmad-output/planning-artifacts/prd.md`
- 아키텍처: `_bmad-output/planning-artifacts/architecture.md`
- UX: `_bmad-output/planning-artifacts/ux-design-specification.md`
- 프로젝트 컨텍스트: 없음

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.1: Set up initial project from starter template]
- [Source: _bmad-output/planning-artifacts/epics.md#Additional Requirements]
- [Source: _bmad-output/planning-artifacts/prd.md#MVP Scope Clarifications]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-001 Simple Booth Alias Session Start]
- [Source: _bmad-output/planning-artifacts/architecture.md#Selected Starter: Official `Vite react-ts` + manual `Tauri CLI` initialization]
- [Source: _bmad-output/planning-artifacts/architecture.md#Project Structure & Boundaries]
- [Source: _bmad-output/planning-artifacts/architecture.md#Testing Framework]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Reading Guide]
- [Source: https://vite.dev/guide/]
- [Source: https://v2.tauri.app/start/create-project/]
- [Source: https://v2.tauri.app/start/frontend/vite/]
- [Source: https://v2.tauri.app/start/prerequisites/]
- [Source: https://reactrouter.com/upgrading/router-provider]
- [Source: https://reactrouter.com/start/data/installation]
- [Source: https://zod.dev/]
- [Source: https://zod.dev/v4/versioning?id=update--july-8th-2025]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- Direct root execution of `pnpm create vite . --template react-ts --no-interactive` returned `Operation cancelled` because the repository already contained planning documents; the final scaffold was still derived from an official Vite `react-ts` bootstrap created in a temporary directory and then merged into the repo root.
- Added `@tauri-apps/cli` and initialized `src-tauri/` with `pnpm tauri init --ci` using the architecture baseline values (`devUrl=http://localhost:5173`, `frontendDist=../dist`, `pnpm run dev`, `pnpm run build`).
- Validation completed with `pnpm test:run`, `pnpm lint`, `pnpm build`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- Git history analysis was not available because this workspace is not an initialized git repository.

### Completion Notes List

- Bootstrapped the repository onto the approved Vite React TypeScript + Tauri 2 baseline with `pnpm` package management, updated scripts, and aligned Tauri build configuration.
- Added top-level `/booth`, `/operator`, `/authoring`, and `/settings` route surfaces with booth as the default entry and guard-based redirects for restricted internal surfaces.
- Created placeholder architecture folders and screens for `booth-shell`, `operator-console`, `preset-authoring`, `shared-ui`, `shared-contracts`, and `session-domain`.
- Introduced a typed capability seam plus a runtime host gateway so React components stay clear of direct `invoke` calls.
- Added Zod-based shared contract schemas and smoke tests covering route gating, default booth routing, and contract parsing.
- Updated story status to `review` and sprint status for `1-1-set-up-initial-project-from-starter-template` to `review`.

### File List

- .gitignore
- README.md
- eslint.config.js
- index.html
- package.json
- pnpm-lock.yaml
- tsconfig.app.json
- tsconfig.json
- tsconfig.node.json
- vite.config.ts
- public/favicon.svg
- public/icons.svg
- src/index.css
- src/main.tsx
- src/app/App.tsx
- src/app/routes.test.tsx
- src/app/routes.tsx
- src/app/boot/runtime-profile.ts
- src/app/guards/surface-access-guard.tsx
- src/app/providers/app-providers.tsx
- src/app/providers/capability-context.ts
- src/app/providers/capability-provider.tsx
- src/app/providers/use-capability-service.ts
- src/app/services/capability-service.ts
- src/booth-shell/screens/SessionStartScreen.tsx
- src/operator-console/screens/OperatorSummaryScreen.tsx
- src/preset-authoring/screens/PresetLibraryScreen.tsx
- src/session-domain/selectors/index.ts
- src/session-domain/services/runtime-capability-gateway.ts
- src/session-domain/state/session-draft.ts
- src/settings/screens/SettingsScreen.tsx
- src/shared-contracts/contracts.test.ts
- src/shared-contracts/index.ts
- src/shared-contracts/dto/booth-session.ts
- src/shared-contracts/errors/index.ts
- src/shared-contracts/events/index.ts
- src/shared-contracts/schemas/capabilities.ts
- src/shared-contracts/schemas/index.ts
- src/shared-ui/layout/SurfaceLayout.tsx
- src/shared-ui/tokens/surface-tokens.ts
- src/test/setup.ts
- src-tauri/.gitignore
- src-tauri/build.rs
- src-tauri/Cargo.lock
- src-tauri/Cargo.toml
- src-tauri/tauri.conf.json
- src-tauri/capabilities/authoring-window.json
- src-tauri/capabilities/booth-window.json
- src-tauri/capabilities/default.json (deleted)
- src-tauri/capabilities/operator-window.json
- src-tauri/gen/schemas/acl-manifests.json
- src-tauri/gen/schemas/capabilities.json
- src-tauri/gen/schemas/desktop-schema.json
- src-tauri/gen/schemas/windows-schema.json
- src-tauri/icons/128x128.png
- src-tauri/icons/128x128@2x.png
- src-tauri/icons/32x32.png
- src-tauri/icons/Square107x107Logo.png
- src-tauri/icons/Square142x142Logo.png
- src-tauri/icons/Square150x150Logo.png
- src-tauri/icons/Square284x284Logo.png
- src-tauri/icons/Square30x30Logo.png
- src-tauri/icons/Square310x310Logo.png
- src-tauri/icons/Square44x44Logo.png
- src-tauri/icons/Square71x71Logo.png
- src-tauri/icons/Square89x89Logo.png
- src-tauri/icons/StoreLogo.png
- src-tauri/icons/icon.icns
- src-tauri/icons/icon.ico
- src-tauri/icons/icon.png
- src-tauri/src/lib.rs
- src-tauri/src/main.rs
- _bmad-output/implementation-artifacts/1-1-set-up-initial-project-from-starter-template.md
- _bmad-output/implementation-artifacts/sprint-status.yaml

### Change Log

- 2026-03-20: Bootstrapped the Boothy desktop baseline on Vite React TypeScript + Tauri, added guarded top-level surfaces, introduced shared contract and capability seams, and added smoke tests with lint/build/cargo validation.
