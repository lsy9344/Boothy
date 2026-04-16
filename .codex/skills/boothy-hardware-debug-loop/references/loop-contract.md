# Loop Contract

Follow this contract every time the skill is used.

## Turn Sequence

1. Identify the latest run.
   - Capture the newest date, session ID, request ID, or evidence bundle identifier that can anchor the analysis.
2. Restate the observed behavior from logs and user notes.
   - Quote exact evidence briefly when useful.
3. Restate the expected product outcome only as needed.
   - If needed, compare it with hardware-validation truth or one targeted current-architecture reference for the changed boundary.
4. Decide the next action.
   - Choose one bounded fix, one instrumentation improvement, or one explicit no-code environment action.
5. Verify locally.
   - Run the smallest relevant automated checks.
6. Update the canonical history.
   - Append cause analysis, what changed, verification, and the next hardware retest request to `history/camera-capture-validation-history.md`.
7. Hand back a retest request.
   - Tell the user exactly what to test and what evidence to return.

## Response Shape

Keep the user-facing response short and product-focused.

- `Current read`: Which latest run or log slice you used.
- `Assessment`: The product-impact summary and the likely failing boundary.
- `Action`: What you changed, or why no safe code change was made yet.
- `Retest`: Short exact hardware steps for the user.
- `Bring back`: Exact logs, screenshots, or evidence files needed for the next loop.

## Comparison Rule For Repeated Loops

When the user comes back after testing:

- Compare the new evidence against the previous hypothesis.
- Say whether the result improved, regressed, or stayed unchanged.
- Do not keep pushing the previous theory if the new evidence contradicts it.

## Product-Safety Rules

- Do not claim success from local tests alone when the issue is release-gated by hardware validation.
- Do not mask truth-state problems with softer copy.
- Do not widen scope without evidence. Each loop should leave the user with one clear next hardware test.
- Do not reread full architecture plans by habit. Use one small current-architecture reference only when the fix needs it.
