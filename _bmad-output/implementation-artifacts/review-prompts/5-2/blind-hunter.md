# Blind Hunter Prompt for Story 5-2

Use the `bmad-review-adversarial-general` skill.

Review scope:
- Story spec: `C:/Code/Project/Boothy/_bmad-output/implementation-artifacts/5-2-정책-기반-복구-액션과-phone-required-라우팅.md`
- Review only the Story 5-2 file list from that spec.

Rules:
- Do not inspect unrelated repository files.
- Treat the story file's file list and the corresponding diff as your only intended scope.
- Hunt for regressions, incorrect assumptions, unsafe recovery behavior, stale-state leaks, and missing tests.
- Output findings only, as a Markdown list.
- For each finding include: severity, one-line title, why it matters, and evidence.
- If there are no findings, say that explicitly.
