# HV-14 Display Completeness

**Not applicable for this source-comparison run.**

The HV-14 runbook requires `BOOTHY_DISPLAY_SAMPLE_MODE` to remain off so fixture display generations do
not contaminate source comparison. Consequently, `display/generations.jsonl` was not produced. Source
telemetry completeness passed independently with 35 requests and exactly three canonical route rows per
request. This collector/runbook contradiction must be corrected before a future combined gate is required.
