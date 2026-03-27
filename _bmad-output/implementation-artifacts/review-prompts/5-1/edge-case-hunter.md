# Edge Case Hunter Prompt for Story 5-1

Use the `bmad-review-edge-case-hunter` skill.

Primary diff input:
`C:/Code/Project/Boothy/_bmad-output/implementation-artifacts/review-prompts/5-1/5-1-review.diff`

Project access:
- Read access to the repository is allowed.
- Focus on boundary conditions, stale state, malformed inputs, no-session cases, race conditions, and contract mismatches.
- Prioritize issues that would escape happy-path tests.

Output format:
- Markdown list of findings only.
- Each finding should include: severity, one-line title, affected path(s), scenario, and why the current implementation misses it.
- If there are no findings, say that explicitly.
