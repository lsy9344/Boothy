## Blind Hunter Prompt

Target story: `1-12-dual-close-topology-정착과-same-slot-truthful-replacement-전환`

Review mode: diff-only. Do not use any repo context, story file, or surrounding project explanation. Judge only the patch.

Workspace: `C:\Code\Project\Boothy_thumbnail-reset-at-2c89c40`

Generate the diff with:

```powershell
git diff -- src-tauri/src/render/dedicated_renderer.rs src-tauri/src/render/mod.rs src-tauri/tests/operator_diagnostics.rs src/booth-shell/screens/CaptureScreen.test.tsx src/operator-console/providers/operator-diagnostics-context.ts src/operator-console/providers/operator-diagnostics-provider.tsx src/operator-console/screens/OperatorSummaryScreen.test.tsx src/operator-console/screens/OperatorSummaryScreen.tsx src/operator-console/services/operator-diagnostics-service.test.ts src/shared-contracts/contracts.test.ts src/shared-contracts/schemas/operator-diagnostics.ts src/shared-contracts/schemas/operator-recovery.ts
```

Task:

1. Read only the diff output.
2. Find concrete bugs, regressions, broken assumptions, or suspicious changes.
3. Ignore style nits and speculative refactors.
4. Output findings as a Markdown list, ordered by severity.

For each finding include:

- A one-line title
- Severity
- Why it is a real product or correctness risk
- Evidence with file and line references from the diff

If no issues are found, say `No findings`.
