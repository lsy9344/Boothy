# Story 1.24 Review Prompt: Acceptance Auditor

Task:
Review the diff at `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\implementation-artifacts\code-review-1-24\story-1-24.diff` against the spec and context below.

Primary spec:
- `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\implementation-artifacts\1-24-local-lane-hardware-canary-validation.md`

Relevant context docs:
- `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\docs\runbooks\preview-promotion-evidence-package.md`
- `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\planning-artifacts\epics.md`
- `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\planning-artifacts\prd.md`
- `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\implementation-artifacts\1-22-capture-full-screen-visible-evidence-chain-trace-reset.md`
- `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\implementation-artifacts\1-23-local-full-screen-lane-prototype과-truthful-artifact-generation.md`

Review goal:
Check for:
- violations of acceptance criteria
- deviations from spec intent
- missing implementation of specified behavior
- contradictions between spec constraints and actual code

Output format:
- Markdown list only.
- Each finding must include:
  - one-line title
  - which AC or constraint it violates
  - evidence from the diff
  - why that evidence fails the story intent
- If there are no findings, say `No findings.`

Important story boundaries:
- Story 1.24 owns canary proof only.
- Story 1.25 owns default/rollback authority.
- Story 1.13 owns final guarded release close.
- Missing field, malformed bundle, stale evidence, correlation drift, non-canary route, or missing rollback proof must stay fail-closed `No-Go`.
