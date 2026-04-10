# Blind Hunter Prompt

Review only the diff at:

- `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\implementation-artifacts\code-review-1-14\story-1-14.diff`

Constraints:

- Do not read the repository.
- Do not read the spec or story file.
- Treat this as a blind adversarial review of the diff alone.
- Focus on likely bugs, contract drift, silent regressions, broken invariants, and missing validation implied by the changed code.
- Ignore style nits and low-value suggestions.

Output format:

- Markdown list only.
- Each finding must include:
  - one-line title
  - severity: `high`, `medium`, or `low`
  - evidence from the diff
  - concrete impact

If no issues are found, say `No findings.`
