# HV-13A environment fingerprint

- 실행 시각: 2026-08-11 15:47–16:01 KST
- 검증자: Codex (운영자: Noah Lee)
- 부스 PC: `NOAH_WIN`
- OS: Windows 11 Pro
- CPU: Intel Core i9-9900KF 3.60GHz
- RAM: 63.9 GB
- GPU: NVIDIA GeForce GTX 1080
- GPU driver: 32.0.15.6094
- WebView2 Runtime: 151.0.4129.72
- 앱 실행 파일: `C:\Code\Boothy\src-tauri\target\debug\boothy.exe`
- Git commit: `c390f53` + Story 7.1 working-tree changes
- 카메라: Canon EOS 700D
- helper: local Debug `canon-helper.exe`, protocol `camera-helper-sidecar/v1`
- Canon SDK: `canon-edsdk` 13.19.0

## 연결 디스플레이

- `DISPLAY1`: 2160×3840 portrait, non-primary, Windows scale 150%
- `DISPLAY2`: 2560×1080 landscape, primary
- `DISPLAY3`: 1920×1080 landscape, non-primary, DPR 1 — 현재 승인 고객 모니터
- EDID inventory: LG ULTRAWIDE, Samsung U32J59x, LG FULL HD

## 실행 설정

- 정상 경로: `BOOTHY_APPROVED_CUSTOMER_MONITOR=\\.\DISPLAY3`
- 존재하지 않는 이름 경로: `BOOTHY_APPROVED_CUSTOMER_MONITOR=U32J59x`
- 실행 명령: `pnpm tauri dev --no-watch`

