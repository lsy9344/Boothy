# Acceptance Auditor Prompt - Story 3.2 Host/Post-End Truth Chunk

Role: Acceptance Auditor
Goal: Review the target diff chunk against Story 3.2 and its product constraints. Find violations of acceptance criteria, missing behavior, and contradictions between implementation and spec intent.

Rules:
- Read the story/spec file and the diff file below.
- Review only the target files listed here.
- Output Markdown findings only.
- Each finding must include:
  - One-line title
  - Violated AC or constraint
  - Evidence from diff/code

Story/spec file:
- `C:/Code/Project/Boothy_lrc_first_visible/_bmad-output/implementation-artifacts/3-2-export-waiting과-truthful-completion-안내.md`

Diff file:
- `C:/Code/Project/Boothy_lrc_first_visible/_bmad-output/implementation-artifacts/code-review-3-2/story-3-2.diff`

Target files:
- `src-tauri/src/handoff/mod.rs`
- `src-tauri/src/session/session_manifest.rs`
- `src-tauri/src/session/session_repository.rs`
- `src-tauri/src/capture/normalized_state.rs`
- `src-tauri/src/contracts/dto.rs`
- `src-tauri/tests/capture_readiness.rs`
- `src-tauri/tests/session_manifest.rs`

Acceptance focus:
- AC1: after capture end, show `Export Waiting` while required deliverables are not ready; capture remains disabled.
- AC2: `Completed` only after required booth-side work is truly complete, and must map to `Local Deliverable Ready` or `Handoff Ready`.
- AC3: after adjusted end time, 90%+ sessions should enter explicit post-end state within 10 seconds; render retry/failure must not invalidate saved captures.

