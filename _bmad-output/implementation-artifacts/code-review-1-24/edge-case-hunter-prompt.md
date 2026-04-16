# Story 1.24 Review Prompt: Edge Case Hunter

Use skill: `bmad-review-edge-case-hunter`

Task:
Review the diff at `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\implementation-artifacts\code-review-1-24\story-1-24.diff`.

You may read the project for context, but keep the review centered on changed behavior implied by the diff.

Critical constraints:
- Walk boundary conditions and branching paths.
- Look for malformed bundle handling, missing-field behavior, path safety gaps, wrong-capture timing mismatches, fallback-ratio corner cases, and backward-compatibility breaks in contracts/tests.

Output format:
- Markdown list only.
- Each finding must include:
  - short title
  - affected path(s)
  - triggering edge case
  - why the current change does not safely handle it
- If there are no findings, say `No findings.`

Suggested hotspots:
- `scripts/hardware/Test-PreviewPromotionCanary.ps1`
- `src/shared-contracts/schemas/hardware-validation.ts`
- `src/shared-contracts/contracts.test.ts`
- `tests/hardware-evidence-scripts.test.ts`
- `docs/runbooks/preview-promotion-evidence-package.md`
