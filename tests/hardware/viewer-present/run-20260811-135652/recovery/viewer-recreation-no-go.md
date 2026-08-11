# Viewer recreation — No-Go

- 실행 시각: 2026-08-11 14:05 KST
- 사전 상태: `viewerEpoch: 1`, `viewerReady: true`, `reasonCode: viewer-ready`
- 관람 창 종료 직후: `windowState: closed`, `revision: 7`, `viewerReady: false`, `reasonCode: viewer-window-closed`
- 부스 결과: 즉시 `Preparing`으로 내려가며 촬영 버튼 비활성화 — 안전 차단 통과
- 재생성 작업: booth WebView에서 `ensure_viewer_window` 호출
- 관찰 결과: 호출이 30초 이상 반환되지 않음. 새 `Boothy Viewer` 창은 생성됐지만 흰 화면에 머물고 viewer route/listener/layout readiness로 수렴하지 않음.
- 판정: **No-Go** — 창 종료 후 자동/수동 복구가 완료되지 않아 운영 중 관람 화면을 회복할 수 없음.

## 증거

- `booth-viewer-window-closed.png`
- `booth-after-viewer-recreate-hang.png`
- `viewer-after-reload.png`

