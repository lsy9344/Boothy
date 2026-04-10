# Edge Case Hunter Prompt

Review this diff for unhandled paths and boundary conditions:

- Diff: `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\implementation-artifacts\code-review-1-16\story-1-16.diff`

You may read the repository for context, but prioritize edge cases over general commentary.

Focus on:

- Windows CI trigger and runner-path edge cases
- signing input validation gaps
- workflow and documentation drift
- unsigned proof vs release-draft proof boundary leaks
- active-session safety guardrail regressions
- cases where governance tests pass but the actual release path can still diverge

Output format:

- Markdown list only.
- Each finding must include:
  - one-line title
  - severity: `high`, `medium`, or `low`
  - the edge case or boundary being missed
  - evidence with file names
  - concrete impact

If no issues are found, say `No findings.`
