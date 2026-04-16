# Story 1.23 Review Prompt: Edge Case Hunter

Use skill: `bmad-review-edge-case-hunter`

Task:
Review the diff at `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\implementation-artifacts\code-review-1-23\story-1-23.diff`.

You may read the project for context, but keep the review centered on changed behavior implied by the diff.

Critical constraints:
- Walk boundary conditions and branching paths.
- Look for unhandled edge cases, mismatched state transitions, partial evidence assembly, wrong-capture leakage, stale snapshot reuse, and invalid fallback semantics.

Output format:
- Markdown list only.
- Each finding must include:
  - short title
  - affected path(s)
  - triggering edge case
  - why the current change does not safely handle it
- If there are no findings, say `No findings.`

Suggested hotspots:
- `src-tauri/src/render/dedicated_renderer.rs`
- `src-tauri/src/capture/ingest_pipeline.rs`
- `src-tauri/src/diagnostics/mod.rs`
- `scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1`
- `src/shared-contracts/schemas/*.ts`
- `tests/hardware-evidence-scripts.test.ts`
