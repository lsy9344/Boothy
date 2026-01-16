# QA Artifacts

This folder is the workspace for QA-related artifacts referenced by BMAD configuration (`qa.qaLocation: docs/qa`).

## Structure

- `docs/qa/assessments/` ??QA assessments (risk/test design/trace/NFR/other checklists)
- `docs/qa/gates/` ??QA gate decisions (PASS/CONCERNS/FAIL/WAIVED) per story

## Gate File Naming (recommended)

Use the BMAD convention shown in the QA agent definition:

- `docs/qa/gates/{epic}.{story}-{slug}.yml`

Examples:

- `docs/qa/gates/1.1-booth-foundation.yml`
- `docs/qa/gates/1.2-camera-ingest.yml`

## Minimum Gate Contents

For each story gate file, capture:

- `decision`: PASS | CONCERNS | FAIL | WAIVED
- `rationale`: why (short, evidence-based)
- `risks`: key risks and mitigations
- `required_followups`: must-fix items before merge/release


