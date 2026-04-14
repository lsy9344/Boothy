# Boothy Dedicated Renderer Sidecar

이 디렉터리는 Canon helper와 분리된 `local dedicated renderer` 실행 경계를 고정한다.

## Story 1.18 resident prototype

- packaged binary 이름 기준: `boothy-dedicated-renderer`
- Tauri bundle 기준 externalBin: `../sidecar/dedicated-renderer/boothy-dedicated-renderer`
- 현재 단계는 resident warm-state prototype을 같은 packaging boundary에서 실행한다.
- warm-up은 `warmup-v1`, preview close 후보는 `preview-job-v1` typed contract로만 통신한다.
- warm-state evidence는 세션 diagnostics 아래 `diagnostics/dedicated-renderer/`에 남기고, host가 `session.json`과 operator diagnostics로 투영한다.
- queue saturation, warm-state loss, invalid output, rollback에서는 booth-safe inline truthful fallback이 계속 우선한다.
- `scripts/prepare-dedicated-renderer-sidecar.mjs`가 resident prototype binary를 준비해 Tauri packaging proof를 유지한다.
