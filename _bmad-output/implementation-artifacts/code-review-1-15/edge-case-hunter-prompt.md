# Edge Case Hunter Prompt

Review this diff for unhandled paths and boundary conditions:

- Diff: `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\implementation-artifacts\code-review-1-15\story-1-15.diff`

You may read the repository for context, but prioritize edge cases over general commentary.

Focus on:

- schema/document/test mismatch
- fixture validity gaps
- backwards-compatibility holes
- parse/serialize asymmetry between TypeScript, Rust, docs, and fixtures
- optional/nullability mistakes
- versioning mistakes
- cases where tests validate examples but runtime code still permits divergent shapes

Output format:

- Markdown list only.
- Each finding must include:
  - one-line title
  - severity: `high`, `medium`, or `low`
  - the edge case or boundary being missed
  - evidence with file names
  - concrete impact

If no issues are found, say `No findings.`
