# HV-14 Decision

- Decision: `No-Go`
- Selected route: `raw-original exact-reference-renderer remains the explicit alternative; no fast-source candidate is approved`
- Baseline improvement: `not measurable: all three candidate routes recorded 0 accepted results from 35 attempts; no candidate may claim an improvement over the incumbent`
- Rejected routes: `embedded-jpeg 0/35 and camera-paired-jpeg 0/35 due to orientation-unsupported; windows-shell-thumbnail 0/35 because the comparison probe observed absent before its later fast-preview-ready event`
- Story 7.4 constraint: `HV-15 may use raw-original with the exact reference renderer, but must not promote embedded, paired, or shell fast-source input from this run`

## Evidence scope

The user approved a 35-shutter run using the current fixed booth composition only. This is sufficient to
establish the technical failure rates above, request/capture/group correlation, and RAW truth preservation.
It does not satisfy the four-scene human quality corpus and therefore does not support a fast-source `Go`.
