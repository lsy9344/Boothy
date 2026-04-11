## Acceptance Auditor Prompt

Target story: `1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate`

Review mode: diff plus spec.

Workspace: `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40`

Inputs:

- Diff: `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\implementation-artifacts\code-review-1-13.diff`
- Story spec: `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\implementation-artifacts\1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate.md`

Task:

1. Review the diff against the story spec.
2. Check for violations of acceptance criteria, deviations from spec intent, missing implementation of specified behavior, and contradictions between spec constraints and actual code.
3. Output findings as a Markdown list.

For each finding include:

- A one-line title
- Which AC or constraint it violates
- Evidence from the diff
- Concrete impact

If no issues are found, say `No findings`.
