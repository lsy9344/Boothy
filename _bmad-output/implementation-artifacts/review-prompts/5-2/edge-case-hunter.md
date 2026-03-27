# Edge Case Hunter Prompt for Story 5-2

Use the `bmad-review-edge-case-hunter` skill.

Primary context:
- Story spec: `C:/Code/Project/Boothy/_bmad-output/implementation-artifacts/5-2-정책-기반-복구-액션과-phone-required-라우팅.md`
- Review only the Story 5-2 file list from that spec.

Project access:
- Read access to the repository is allowed.
- Focus on boundary conditions, stale state, malformed inputs, no-session cases, recovery rejection paths, and timing/post-end drift.
- Prioritize issues that would escape happy-path tests.

Output format:
- Markdown list of findings only.
- Each finding should include: severity, one-line title, affected path(s), scenario, and why the current implementation misses it.
- If there are no findings, say that explicitly.
