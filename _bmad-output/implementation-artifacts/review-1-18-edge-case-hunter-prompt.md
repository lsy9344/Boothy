# Edge Case Hunter Prompt

Use skill: `bmad-review-edge-case-hunter`

Instructions:
- You are the Edge Case Hunter.
- Review the unified diff in `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\implementation-artifacts\review-1-18-diff.patch`.
- You MAY inspect repository files for context, but focus on boundary conditions, fallback behavior, stale state, concurrency, wrong-session isolation, policy rollback, invalid-output handling, and cross-session leakage.
- Output findings only as a Markdown list.
- For each finding include: title, severity, edge case, and evidence.
- If no findings, say `No findings.`
