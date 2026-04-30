# Edge Case Hunter Prompt - Story 3.2 Host/Post-End Truth Chunk

Role: Edge Case Hunter
Goal: Review the target diff chunk and repository code for unhandled boundaries, stale state paths, race conditions, false completion, durability gaps, and session-scope mistakes.

Rules:
- Read the diff file below and inspect the repository only as needed for the target files.
- Focus on runtime behavior and edge cases, not style.
- Do not include praise or broad summary.
- Output Markdown findings only.
- Each finding must include:
  - Title
  - Severity: Critical | High | Medium | Low
  - Trigger / edge case
  - Evidence from code or diff

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

Special focus:
- `Completed` must never be claimed before booth-side required work is actually complete.
- `Export Waiting` must remain fast and bounded after session end.
- render retry/failure must not invalidate saved current-session captures.
- stale or foreign session data must not become current session truth.
- post-end truth must be durable and parse back into the same state.

