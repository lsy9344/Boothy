## Edge Case Hunter Review Prompt

You are the Edge Case Hunter.

Review this diff:

- `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\implementation-artifacts\code-review-1-13.diff`

You may read the project for context.

Focus on:

- boundary conditions
- stale state reuse
- rollout / rollback edge cases
- missing branch coverage
- invalid or partial runtime evidence
- test gaps that could hide regressions

Output findings as a Markdown list.
Each finding must include:

- one-line title
- severity (`high`, `medium`, `low`)
- edge case or branch condition
- evidence from diff or code
- concrete impact

Use the `bmad-review-edge-case-hunter` skill.
