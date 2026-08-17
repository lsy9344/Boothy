# HV-18A environment — 2026-08-17 current-PC run

- Host: `NOAH_WIN`, Windows 11 Pro 10.0.26200 x64
- CPU / RAM / GPU: Intel Core i9-9900KF (8C/16T) / 68.6 GB / NVIDIA GeForce GTX 1080
- Camera: Canon EOS 700D, helper preflight camera count 1
- Production candidate: `Boothy_0.1.0_x64-setup.exe`, 405257006 bytes, SHA-256 `ffb5d5d96d311894ad5f1e59b713b35f5310c31b7899b2d940bb290a6df043b6`
- Upgrade candidate: `Boothy_0.1.1_per-machine_x64-setup.exe`, 405281948 bytes, SHA-256 `c8c6314d0f51d594e1fe3d0aab73d309b1ea39462fcaa59987997f20bf9cc176`
- Lifecycle test package A: `Boothy_0.1.0_current-user_x64-setup.exe`, SHA-256 `b74dbd024cb535f405eb78ec2b888df4aaec4d80f4ed8dadf2b8c09ea42ebee7`
- Lifecycle test package B: `Boothy_0.1.1_current-user_x64-setup.exe`, SHA-256 `44a73ea0b64d92abc2861c43d7fdb498bd722e46fc49da61a5cfe1b53335901c`
- Signature: `unsigned` — `waived-by-owner`
- Canon EDSDK redistribution evidence: `not-evidenced` — `waived-by-owner`
- Clean/offline environment: not used — `waived-by-owner`
- Installed self-check: pass, darktable source `bundled-resource`, WebView2 151.0.4129.86, helper 0.1.0 / EDSDK 13.19.0
- Lifecycle install-mode deviation: production packages remain `perMachine`; because the non-elevated test terminal could not answer UAC, the automated lifecycle used equivalent `currentUser` test bundles. This does not prove per-machine UAC behavior.
- Customer data baseline/final: 1117 files / 4535271014 bytes, unchanged

This run is current-PC internal-validation evidence with explicit waivers, not clean/offline or public-distribution evidence.
