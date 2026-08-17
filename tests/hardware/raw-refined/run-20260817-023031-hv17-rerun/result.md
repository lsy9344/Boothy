# HV-17 remediation rerun result

## Product verdict

- HV-17A: **No-Go** — the four live cancellation/process-tree rounds remain uncollected.
- HV-17B: **No-Go** — the real tier corpus found no useful detail gain and detected look drift.
- Product action: keep the RAW-refined lane **off** and record the tier as `tier-not-justified`.

## Verified on real hardware

- Canon EOS 700D settings were selected only from camera descriptors and confirmed by readback.
- Three normally exposed RAW captures completed and produced three displayed proxies.
- Nine proxy/refined pairs were rendered from the same RAW and preset inputs without advancing the customer pointer.
- Median refined/proxy MTF50 ratio was `0.9994705`, below the `1.10` threshold, with five regressions.
- Median ΔE00 was `4.5213` and median p95 ΔE00 was `15.0056`; Daylight and Soft Glow exceeded the same-look thresholds.
- Dimensions matched for all nine pairs; clipping increase stayed within threshold.
- An independent ffmpeg JPEG decode reproduced the same verdict (`0.9988553` detail ratio, median ΔE00 `4.5298`, median p95 ΔE00 `15.0262`).

## Evidence and limitations

- Tier verdict: `tier-justification/verdict.json`
- Independent decode check: `tier-justification/ffmpeg-verify/`
- Commands and raw outputs: `tier-justification/commands.jsonl`
- Camera proof: `camera-preflight-*.json`
- The observer trial, seamless swap recording, and four cancellation rounds were not fabricated after the tier failed.
- The booth WebView became transparent after one capture in one session; the app was restarted and the remaining real captures were taken in fresh sessions. Camera and saved RAW data remained healthy.
