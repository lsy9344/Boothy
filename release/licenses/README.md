# Licensing Evidence for Shipped Components

Acceptance Criterion 1 of Story 7.7 requires licensing evidence to travel with the installer
inventory. These documents are that evidence.

They live here, in version control, rather than under `release/vendor/`. The vendor tree is ignored
by git, so evidence placed there would never be committed and would drift out of existence exactly
when a release needed to cite it.

Every entry in `release/inventory-spec.json` whose component is actually shipped points at one of
these files through `licenseEvidencePath`, and the whole directory is bundled into the installer as
`licenses/` so a booth PC carries its own license texts.

| Component | Evidence |
| --- | --- |
| darktable 5.4.1 (raw renderer, colour profile) | [`darktable-5.4.1.md`](./darktable-5.4.1.md) |
| Canon EDSDK 13.19.0 (camera runtime) | [`canon-edsdk-13.19.0.md`](./canon-edsdk-13.19.0.md) |
| Camera helper .NET dependencies | [`canon-helper-dependencies.md`](./canon-helper-dependencies.md) |
| Microsoft Edge WebView2 Runtime | [`webview2-runtime.md`](./webview2-runtime.md) |

## Rule

A gap is written down as a gap. If a redistribution right has not been confirmed, the evidence file
says so and the inventory carries that state — it is never rounded up to "fine". An unresolved
licensing gap is a release blocker in exactly the same way an unsigned installer is.
