---
project_name: 'Boothy'
user_name: 'Noah Lee'
date: '2026-03-08'
status: 'complete'
sections_completed:
  - 'technology_stack'
  - 'language_specific_rules'
  - 'framework_specific_rules'
  - 'testing_rules'
  - 'code_quality_style_rules'
  - 'development_workflow_rules'
  - 'critical_dont_miss_rules'
rule_count: 47
optimized_for_llm: true
existing_patterns_found:
  technologies_discovered: 12
  implementation_patterns: 8
  coding_conventions: 6
  critical_rules_candidates: 14
source_documents:
  - '_bmad-output/planning-artifacts/architecture.md'
  - 'reference/uxui_presetfunction/package.json'
  - 'reference/uxui_presetfunction/tsconfig.json'
  - 'reference/uxui_presetfunction/vite.config.js'
  - 'reference/uxui_presetfunction/.eslintrc.json'
  - 'reference/uxui_presetfunction/.prettierrc'
  - 'reference/uxui_presetfunction/tailwind.config.js'
  - 'reference/uxui_presetfunction/src-tauri/Cargo.toml'
---

# Project Context for AI Agents

_This file contains critical rules and patterns that AI agents must follow when implementing code in this project. Focus on unobvious details that agents might otherwise miss._

---

## Technology Stack & Versions

- Frontend baseline: React 19.2.3, React DOM 19.2.3, TypeScript 5.9.3, Vite 7.3.0.
- Desktop boundary: Tauri 2.x (`@tauri-apps/api` 2.9.1, `@tauri-apps/cli` 2.9.6) with a Rust 2024 backend.
- Styling/tooling baseline from donor reference: Tailwind CSS 3.4.19, ESLint 9.39.2, `@vitejs/plugin-react` 5.1.2.
- Architecture-required stack for implementation: React Router 7.9.4, Zod 4, local SQLite, Tauri Store, GitHub Actions with `tauri-action`.
- Native integration model: React SPA talks to Tauri commands/channels; the host talks to a bundled sidecar through versioned JSON-line messages over stdio.
- When architecture and donor code disagree, follow the architecture document over donor implementation.

## Critical Implementation Rules

### Language-Specific Rules

- TypeScript runs in `strict` mode with `isolatedModules`, `module: nodenext`, `moduleResolution: nodenext`, `target: es2024`, and `resolveJsonModule: true`; new code must compile cleanly under those constraints.
- Cross-boundary DTOs and payloads must stay fully typed. Do not introduce `any` for Tauri command payloads, channel payloads, session manifests, or sidecar messages.
- TypeScript-side boundary validation uses Zod 4 before data crosses into Tauri commands; Rust revalidates the same DTOs again on the host side.
- Keep naming asymmetric on purpose: Rust command identifiers remain `snake_case`, but TypeScript wrappers and exported helpers remain `camelCase`.
- Do not hardcode raw Tauri command strings outside the host adapter layer. React/UI code should call typed wrapper functions, not `invoke('...')` directly.
- Use JSON modules and shared schema files as contract sources where needed; do not duplicate contract shapes in multiple frontend files.
- Error handling at the TypeScript boundary must preserve the typed host error envelope. UI code translates `code`, `severity`, customer/operator states, and retryability; it must not guess device truth from generic exceptions.

### Framework-Specific Rules

- Use React Router only for top-level surfaces and shell entry points such as `/customer`, `/operator`, and `/settings`; do not model booth truth or customer journey progression with routes.
- Organize frontend code by product domain first: `customer-flow`, `operator-console`, `session-domain`, `timing-policy`, `capture-adapter`, `export-pipeline`, `branch-config`, and `diagnostics-log`.
- Keep `shared-ui` presentation-only. Do not move domain rules, customer copy translation, or operator recovery logic into shared UI components.
- React components must not call Tauri directly. UI code talks to typed adapter/service modules, and those modules own `invoke`, channels, and host-bound orchestration.
- State management must use explicit domain reducers and React Context providers. Do not introduce a generic global mutable store for MVP.
- Customer-facing camera guidance and operator-facing diagnostics must both derive from one host-normalized camera connection model. Do not translate raw sidecar/device states in the UI.
- Keep the operator surface separate from the customer surface in both UI structure and capability boundaries; customer-facing code must not expose diagnostics or recovery actions.
- Prefer React 19 async UI patterns for non-blocking updates on customer-critical flows. Lazy-load diagnostics/configuration surfaces, and only virtualize larger lists where measurement justifies it.

### Testing Rules

- Keep test boundaries explicit: contract tests guard cross-language shapes, domain/unit tests cover isolated business logic, integration tests cover workflow seams, and e2e tests cover customer/operator flows.
- Prioritize contract tests for shared DTOs, camera contract messages, error envelopes, and session manifest schemas before deeper feature work.
- Do not let React component tests become substitutes for host contract validation; UI tests should verify rendering, state transitions, and translation logic, not native/device truth.
- Native host behavior around manifests, SQLite migrations, and contract validation should be tested in Rust-side tests close to the host modules.
- Keep fixtures deterministic and session-scoped. Test data for photos, manifests, and sidecar messages must not leak across sessions or depend on machine-specific state.
- Mock the host boundary at the adapter layer in frontend tests. Do not spread ad hoc `invoke` mocks across unrelated components.
- E2E coverage should focus on capture-critical paths: customer flow, operator recovery, export readiness, and rollout/rollback safety.
- Add tests when changing schemas, command envelopes, migration files, or sidecar protocol messages; those are compatibility-sensitive surfaces.

### Code Quality & Style Rules

- Follow the naming split consistently: React components and component files use `PascalCase`, hooks and exported TS helpers use `camelCase`, Rust modules/files use `snake_case`, and domain directories use `kebab-case`.
- Keep file placement domain-first, not type-first. New code should go into the owning product domain rather than broad shared buckets.
- Prefer small typed adapter/service modules over large feature files that mix UI, host calls, schemas, and business logic.
- Respect the formatter baseline from the donor reference: semicolons on, single quotes in TypeScript, trailing commas enabled, and `printWidth` 120.
- Keep comments lean and high-signal. Add comments only where boundary behavior, contract rationale, or non-obvious recovery logic would otherwise be hard to infer.
- Do not copy donor code patterns that violate the target architecture, especially direct UI `invoke` usage and oversized all-in-one React components.
- Customer-facing copy, operator-facing diagnostics copy, and state translation logic must live in explicit selectors/copy modules, not inline across arbitrary components.
- Shared schemas, contract docs, and manifest definitions should have one authoritative source. Avoid parallel duplicate definitions across frontend, host, and sidecar code.

### Development Workflow Rules

- Freeze shared contracts early: define DTO schemas, error envelopes, camera contract semantics, and session manifest shape before broad feature implementation.
- Treat compatibility-sensitive surfaces as review gates: schema files, command envelopes, sidecar protocol messages, SQLite migrations, capability files, and branch config schemas should not change casually.
- No migration or update flow may interrupt an active session. Any schema or rollout change must preserve rollback safety for booth PCs already in use.
- Keep branch variance minimal and explicit. MVP branch-specific behavior is limited to approved configuration such as phone number and operational toggles.
- Local session folders remain the durable source of truth for capture artifacts and handoff readiness; operational logs and config stores must not silently become more authoritative than session storage.
- Build and release assumptions should stay aligned with the desktop target: signed Windows bundles, staged rollout, preserved rollback artifacts, and no forced updates during active sessions.
- When implementation sequence is unclear, prefer this order: shared contracts -> manifest/log schema -> host boundary -> React shell/state model -> mocked sidecar integration -> real sidecar integration.
- Donor reuse must be selective and contract-first. Reuse assets or patterns only when they fit the target architecture without weakening boundary rules.

### Critical Don't-Miss Rules

- Do not expose raw camera/helper/internal diagnostic states directly to the customer UI. Customer-visible status must come from approved translated states only.
- Do not allow cross-session photo leakage in UI state, thumbnail queries, manifest reads, export checks, or fixture setup.
- Do not treat routes, local caches, or UI memory as the source of truth for capture readiness or handoff completion; the session folder contract remains authoritative.
- Do not bypass capability boundaries. Customer-facing windows must not gain operator diagnostics, sidecar process control, or unrestricted shell access.
- Do not pass captured image bytes through JSON IPC when the contract expects filesystem handoff. Large capture artifacts belong in session storage with manifest correlation.
- Do not infer retry behavior or recovery options from generic errors. Use the typed host error envelope and preserve customer/operator action separation.
- Do not introduce destructive migrations, cleanup routines, or updater behavior that can mutate active-session data or strand rollback compatibility.
- Do not add branch-specific exceptions, hidden toggles, or ad hoc workflow shortcuts outside approved config surfaces; operational consistency across booths is a core requirement.

---

## Usage Guidelines

**For AI Agents:**

- Read this file before implementing any code.
- Follow all rules exactly as documented.
- When in doubt, prefer the more restrictive boundary-preserving option.
- Update this file when new stable project patterns emerge.

**For Humans:**

- Keep this file lean and focused on agent-facing implementation rules.
- Update it when the technology stack, contracts, or architectural boundaries change.
- Review it periodically and remove rules that become obvious or obsolete.
- Use the architecture document as the deeper source, and keep this file as the compressed execution guide.

Last Updated: 2026-03-08
