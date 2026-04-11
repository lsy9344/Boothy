## Edge Case Hunter Prompt

Target story: `1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate`

Review mode: diff plus read-only project access.

Workspace: `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40`

Primary review diff:

- `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\implementation-artifacts\code-review-1-13.diff`

Task:

1. Review the diff first.
2. Read surrounding project code only where needed to verify behavior.
3. Hunt for missed edge cases, state machine gaps, stale-data paths, rollout and rollback regressions, partial-failure handling issues, and test gaps.
4. Output only actionable findings as a Markdown list, ordered by severity.

For each finding include:

- A one-line title
- The exact edge case or boundary condition
- Why current code mishandles it
- Evidence with file and line references

If no issues are found, say `No findings`.
