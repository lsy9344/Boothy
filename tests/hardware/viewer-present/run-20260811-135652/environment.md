# HV-13A environment fingerprint

- 실행 시각: 2026-08-11 13:56–14:12 KST
- 검증자: Codex (운영자: Noah Lee)
- 부스 PC: `NOAH_WIN`
- 제조사/모델: OEM 식별값 미설정
- OS: Windows 11 Pro
- CPU: Intel Core i9-9900KF 3.60GHz
- RAM: 63.9 GB
- GPU: NVIDIA GeForce GTX 1080
- GPU driver: 32.0.15.6094
- WebView2 Runtime: 151.0.4129.72
- 앱 버전: 0.1.0
- Git commit: `89ae52c831abf335fb8d986c16a5e1a160c86220`
- 카메라: Canon EOS 700D (`cameraState: ready`, `helperState: healthy`)
- helper: local Debug `canon-helper.exe`; SDK 버전 식별값은 이번 회차에 미수집
- ICC/HDR: 이번 회차에 미수집

## 연결 디스플레이

- `DISPLAY1`: 1440×2560, non-primary, DPR 1.5 — `unapproved-profile` 검증 대상
- `DISPLAY2`: 2560×1080, primary
- `DISPLAY3`: 1920×1080, non-primary, DPR 1 — 승인 고객 모니터 검증 대상
- 모니터 EDID inventory: LG ULTRAWIDE, Samsung U32J59x, LG FULL HD

## 실행 설정

- 정상 경로: `BOOTHY_APPROVED_CUSTOMER_MONITOR=\\.\DISPLAY3`
- 미승인 profile 경로: `BOOTHY_APPROVED_CUSTOMER_MONITOR=\\.\DISPLAY1`
- 존재하지 않는 이름 경로: `BOOTHY_APPROVED_CUSTOMER_MONITOR=U32J59x` → Tauri device name과 불일치
- 실행 명령: `pnpm tauri dev --no-watch`

