# Acceptance Auditor Prompt

Review this diff against the story and contract-freeze intent.

Inputs:

- Diff: `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\implementation-artifacts\code-review-1-14\story-1-14.diff`
- Story/spec: `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\implementation-artifacts\1-14-공유-계약-동결과-검증-기준-확정.md`

Task:

- Check for violations of acceptance criteria.
- Check for deviations from the story intent.
- Check for missing implementation of specified behavior.
- Check for contradictions between the story constraints and the actual changes.

Acceptance criteria to enforce:

1. `session.json` session manifest schema must be frozen with explicit version and aligned across docs and shared schema, including capture correlation, preset version reference, and post-end fields.
2. Preset bundle schema and runtime profile/capability baseline must be fixed so booth, operator, and authoring surfaces reference the same contract.
3. Error envelope and helper/sidecar protocol contract must be inspectable, with testable examples or executable validation left behind.

Output format:

- Markdown list only.
- Each finding must include:
  - one-line title
  - violated AC or story constraint
  - evidence from the diff
  - concrete impact

If no issues are found, say `No findings.`
