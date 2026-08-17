# HV-17 remediation rerun environment

- Run time: 2026-08-17 02:30–03:20 KST
- Workstation: Windows 11 Pro 10.0.26200, Intel Core i9-9900KF, 63.92 GiB RAM, NVIDIA GTX 1080
- Customer display: LG Full HD (`DISPLAY\\GSM5B55`), 1920×1080, DPR 1.0, 60 Hz, approved source 1429×953
- Camera: Canon EOS 700D over EDSDK 13.19.0, one directly connected camera
- Renderer: darktable 5.4.1 at `C:\Program Files\darktable\bin\darktable-cli.exe`
- Application baseline: `c390f53e2079e9379cb36501517fa05e26beb41d` plus this remediation worktree
- Product lane settings during corpus capture: display proxy `on`; raw-refined `off`; sample/source-compare/resident `off`
- Camera descriptor/readback: RAW-only `0x0064FF0F`, Av `0x28`, Tv `0x50`, ISO `0x68`
- Camera preflight evidence: `camera-preflight-probe.json`, `camera-preflight-applied.json`, `camera-preflight-applied-2.json`
- Normal-exposure sessions: `session_000000000018cc59e9f76c5488`, `session_000000000018cc5b2aefceddf8`, `session_000000000018cc5b4bf8ed513c`
- Proxy mean luminance: 81.81, 81.59, 81.98 of 255; no highlight clipping was observed visually
- Tier corpus: 3 real EOS RAW files × Daylight/Soft Glow/Mono Pop; production dimensions, JPEG 95, sRGB/perceptual; proxy/refined differ by `--hq` only
- Human detection trial: not run because the measured tier failed its adoption gate and was not published
- 120 fps physical-frame and compositor-to-photon evidence: not measured here; owned by HV-18B

One earlier sample at F/4, 1/25 s, ISO 800 averaged 20/255 and was excluded before the tier corpus was formed.
