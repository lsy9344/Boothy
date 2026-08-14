# HV-13A Evidence Package — Story 7.1

이 디렉터리는 Story 7.1 `촬영 전 전용 관람 창 준비와 화면 크기 계약`의 실장비 증거를 보존한다.

**현재 상태: 2026-08-11 승인 부스 하드웨어 재검증 `Go`.**
현재 회차 결과는 `run-20260811-155154/result.md`를 본다. 이전 `No-Go`였던 관람 창 재생성 문제는
같은 장비에서 복구 후 실제 재촬영까지 완료해 해소했다.

## 수집 대상 (ledger required evidence)

`viewer profile` / `viewer snapshot` / `monitor targeting` / `session binding` /
`listener·layout readiness` / `reload·gap recovery` / `physical display-size contract`

## 1. 환경 fingerprint — `environment.md`

- 승인 booth PC 모델, CPU/RAM/GPU/driver
- 고객 모니터 모델, 물리 해상도, Windows 배율(%), 실제 `devicePixelRatio`
- WebView2 Runtime 버전 (`Get-AppxPackage`/레지스트리 확인값)
- ICC 프로파일, HDR on/off
- 앱 버전, 커밋 해시, `BOOTHY_APPROVED_CUSTOMER_MONITOR` 설정값

## 2. Pre-capture readiness — `pre-capture/`

촬영 **입력 전에** viewer가 이미 존재하고 현재 세션에 묶여 있음을 보여야 한다.

- 세션 시작 직후 `get_viewer_readiness` 응답 JSON (`viewerReady: true`, `reasonCode: viewer-ready`)
- 같은 시점의 고객 모니터 사진 또는 화면 캡처
- host 로그의 `viewer_window_opened targeting=... profile=...` 라인
- 촬영 버튼 입력 후 window 생성/navigation/resize가 **발생하지 않았음**을 보이는 로그 구간

## 3. Monitor targeting — `monitor-targeting/`

- `monitorTargeting: approved-customer-monitor`인 정상 snapshot
- 승인 모니터 이름을 잘못 설정했을 때 `monitor-unavailable`이 되고 촬영이 막히는 증거
- 단일 모니터가 실제 승인 운영 구성인 지점에서는 `single-monitor-fallback` 차단 snapshot·부스 화면
- 승인 목록 밖 해상도를 고객 모니터로 운영하는 지점에서는 `unapproved-profile` 차단 snapshot·부스 화면
- 관람 창이 부스 조작 화면이 아닌 고객 모니터를 채우는 사진

## 4. Physical display-size contract — `display-size/`

- 해당 지점에서 실제 승인 운영 중인 display profile의 snapshot `photoRect`
- `requiredSourceWidthPx/HeightPx`가 실측 `cssWidth/cssHeight × devicePixelRatio`와 일치함
- 고정 384px asset이 해당 profile 요구치를 만족하지 못함을 보이는 대조표

현재 승인되지 않은 대체 profile 조합은 자동 계약 테스트로 유지하고, 승인 운영 구성이 바뀔 때 해당 실장비 증거를 추가한다.

## 5. Failure-path recovery — `recovery/`

각 항목마다 "촬영이 막혔다"는 부스 화면과 host snapshot을 함께 남긴다.

- viewer reload → 새 `viewerEpoch`, 이전 epoch report 폐기, 재수렴
- listener loss / 창 닫힘 → `viewer-window-closed`, 촬영 차단
- 세션 교체 직후 → `session-mismatch`, 재보고 후 복구
- heartbeat 중단(창 최소화/백그라운드) → `stale-report`, 촬영 차단

## 6. Read-only surface — `read-only/`

- 고객 모니터 전체 화면 사진: 조작 요소와 진단 표시가 없음
- 준비 실패 시 관람 화면이 아니라 booth control surface에만 안내가 뜨는 대비 사진

## 7. 판정 기록

수집이 끝나면 ledger의 Story 7.1 행에 다음을 기록한다.

- `evidence package path`: 이 디렉터리의 실제 경로
- `Go / No-Go result`
- `executedAt`, `validator`, `booth PC`, `camera model`, `helper identifier`
- No-Go면 `release blocker`와 `rerun prerequisite`를 갱신한다

## 참고

- 계약 문서: `docs/contracts/viewer-readiness.md`
- Story: `_bmad-output/implementation-artifacts/7-1-전용-관람-창-준비와-화면-크기-계약.md`
- Story 7.2/HV-13B가 소유하는 actual-present 계측과 frame transition 증거는 여기에 넣지 않는다.
