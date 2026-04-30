# Blind Hunter Prompt - Story 3.2 Host/Post-End Truth Chunk

Role: Blind Hunter
Goal: Review only the target diff chunk for bugs, regressions, consistency errors, and product-risky behavior. Use diff evidence only. Do not use story/spec/context.

Rules:
- Read the diff file below, but review only the target files listed here.
- Do not inspect the repository or story file.
- Do not include praise, summary, or improvement ideas.
- Output Markdown findings only.
- Each finding must include:
  - Title
  - Severity: Critical | High | Medium | Low
  - Evidence
  - File/line or diff hunk reference

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

