# HV-18A result — Internal Go with owner-approved waivers

This PC completed the full install lifecycle with a real Canon EOS 700D capture.

Passed:

- install, launch, installed-tree self-check, upgrade, rollback, uninstall
- bundled display sample B shown on the 1920×1080 customer display
- real Canon capture bound to RAW original, display proxy, and final output
- the same completed session read successfully after upgrade and rollback
- uninstall removed the app, bundled darktable, helper, registry entry, and Start Menu entry
- customer data remained intact: 1,152 files before and after uninstall
- HV-17 was accepted as `Partial`; raw-refined remains off because the measured tier benefit was not justified

Owner-approved waivers:

- code signing
- Canon EDSDK redistribution evidence
- clean/offline environment

Mechanical verdict: **Go-candidate-with-waivers**. These waived items are recorded as waivers, not as verified passes.

Owner product disposition: **Go-with-waivers for the approved current-PC internal scope**. This does not claim signed, clean/offline, or public-distribution readiness.

After the lifecycle run, the viewer-to-capture readiness refresh fix was added and the complete installer was rebuilt successfully. The new source-aligned package is stored separately as `installer/Boothy_0.1.0_latest-source_x64-setup.exe` (405,260,278 bytes, SHA-256 `202518faf04be56d4b47fc1a7ba7e180366e08be551700bc637885e93b94140c`). It passed strict inventory verification and all automated tests. It is not falsely substituted for the package used by the lifecycle evidence above.

The rebuilt executable also completed a windowless full-tree self-check with the preserved session: `self-check/latest-source-build.json`, overall `pass`, including `session-compatibility: pass`.
