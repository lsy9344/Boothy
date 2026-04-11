## Acceptance Auditor Prompt

Target story: `1-12-dual-close-topology-정착과-same-slot-truthful-replacement-전환`

Workspace: `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40`

Spec file:

`C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40\_bmad-output\implementation-artifacts\1-12-dual-close-topology-정착과-same-slot-truthful-replacement-전환.md`

There are no extra context docs loaded from frontmatter for this run.

Primary diff command:

```powershell
git diff -- src-tauri/src/render/dedicated_renderer.rs src-tauri/src/render/mod.rs src-tauri/tests/operator_diagnostics.rs src/booth-shell/screens/CaptureScreen.test.tsx src/operator-console/providers/operator-diagnostics-context.ts src/operator-console/providers/operator-diagnostics-provider.tsx src/operator-console/screens/OperatorSummaryScreen.test.tsx src/operator-console/screens/OperatorSummaryScreen.tsx src/operator-console/services/operator-diagnostics-service.test.ts src/shared-contracts/contracts.test.ts src/shared-contracts/schemas/operator-diagnostics.ts src/shared-contracts/schemas/operator-recovery.ts
```

Task:

1. Read the spec file completely.
2. Review the diff against the spec and acceptance criteria.
3. Check for violations of acceptance criteria, deviations from spec intent, missing implementation of required behavior, or contradictions with stated guardrails.
4. Output findings as a Markdown list.

For each finding include:

- A one-line title
- Which AC or guardrail it violates
- Why this matters to the product behavior
- Evidence from the diff with file and line references

If no issues are found, say `No findings`.
