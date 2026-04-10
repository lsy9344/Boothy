# Boothy Dedicated Renderer Sidecar

이 디렉터리는 Canon helper와 분리된 `local dedicated renderer` 실행 경계를 고정한다.

## Story 1.11 baseline

- packaged binary 이름 기준: `boothy-dedicated-renderer`
- Tauri bundle 기준 externalBin: `../sidecar/dedicated-renderer/boothy-dedicated-renderer`
- 현재 단계는 submission/warm-up/fallback contract를 먼저 닫는 shadow baseline이다.
- truthful `previewReady` 승격은 여전히 host-owned inline render path가 소유한다.
- `scripts/prepare-dedicated-renderer-sidecar.mjs`가 shadow baseline용 최소 실행 파일을 준비해 Tauri packaging proof를 유지한다.
