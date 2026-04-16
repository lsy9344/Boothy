# Story 1.23 Review Prompt: Blind Hunter

Use skill: `bmad-review-adversarial-general`

Task:
Review the diff at `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\implementation-artifacts\code-review-1-23\story-1-23.diff`.

Critical constraints:
- You receive diff only.
- Do not use project files, repo context, spec files, or prior conversation.
- Review adversarially for concrete bugs, regressions, unsafe assumptions, broken invariants, and hidden coupling.

Output format:
- Markdown list only.
- Each finding must include:
  - short title
  - severity (`high`, `medium`, or `low`)
  - evidence from the diff
  - why it is a real problem
- If there are no findings, say `No findings.`

Focus:
- logic bugs
- state/correlation drift
- fail-open behavior
- backward compatibility issues
- observability or evidence corruption
