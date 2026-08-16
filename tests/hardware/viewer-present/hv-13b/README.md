# HV-13B Evidence Package — Story 7.2

이 디렉터리는 Story 7.2 `Immutable sample 표시와 actual-present 계측`의 실장비 증거를 보존한다.

**현재 상태: `No-Go` (2026-08-11 수정 후 재검증).** 승인된 `DISPLAY3`와 Canon EOS 700D에서
두 변형을 각각 warm-up 5회 + 측정 30회 실행했다. 140개 immutable generation의 무결성과
120/120 actual-present 측정 행을 확인해 소프트웨어 계측 문제는 닫았다. 다만 120fps+ 촬영 장비가
없어 물리 프레임 판정을 완료하지 못했고, hidden-prewarm에서 15.024초 outlier도 관측됐다.

증거 패키지는 `../run-20260811-203500-hv13b-rerun/`에 있다. 기본 운용은 더 안정적인
`visible-standby`를 유지하며, AC 5에 따라 Story 7.2는 `in-progress`다.

2026-08-12에는 미검증 상태였던 hidden-prewarm `offscreen` 전략을 직접 추가 검증했다.
Canon 실촬영 5회에서 10/10 generation 무결성과 승인 DISPLAY3의 최종 B 표시를 확인했지만,
generation 1개의 terminal present 행이 누락돼 계측 완전성은 9/10이었다. 120fps+ 물리 프레임도
여전히 없어 `No-Go`를 유지한다. 증거는 `../run-20260812-172000-hv13b-offscreen-direct/`에 있다.

HV-13A(Story 7.1) 증거는 `../run-20260811-*`에 있다. **섞지 않는다.**

## 수집 대상 (ledger required evidence)

`immutable generations` / `pointer commits` / `decode·dimension·correlation checks` /
`timing spans` / `actual monitor frames` / `visible-standby vs hidden-prewarm comparison` /
`zero-transition-defect report`

## 사전 조건

- HV-13A `Go` (2026-08-11 충족)
- 승인된 고객 모니터 구성 (`BOOTHY_APPROVED_CUSTOMER_MONITOR`가 가리키는 승인 1080p 디스플레이)
- **120fps 이상 촬영 장비.** 소프트웨어 present는 compositor→photon 구간을 측정할 수 없으므로
  이 장비 없이는 AC 3의 "물리 모니터 qualifying frame"을 닫을 수 없다

## 실행 방법

```powershell
# 계측 lane을 켠다. 기본은 off이며, 켜져 있을 때 표시되는 이미지는 계측용 fixture다.
$env:BOOTHY_DISPLAY_SAMPLE_MODE = 'visible-standby'   # 또는 'hidden-prewarm'
$env:BOOTHY_DISPLAY_SAMPLE_GAP_MS = '800'
$env:BOOTHY_DISPLAY_HIDDEN_PREWARM_STRATEGY = 'invisible'   # hidden 변형에서만
$env:BOOTHY_APPROVED_CUSTOMER_MONITOR = '<승인 모니터 이름>'

pnpm dev:desktop
```

세션을 시작하고 승인된 촬영 컨트롤로 실제 촬영을 수행한다.
한 request마다 sample A가 게시되고 `BOOTHY_DISPLAY_SAMPLE_GAP_MS` 후 sample B가 게시된다.

**측정이 끝나면 반드시 `BOOTHY_DISPLAY_SAMPLE_MODE`를 지운다.** 켠 채로 두면 실제 고객이
fixture 사진을 본다.

## 1. 환경 fingerprint — `environment.md`

- 승인 booth PC 모델, CPU/RAM/GPU/driver
- 고객 모니터 모델, 물리 해상도, Windows 배율(%), 실제 `devicePixelRatio`, **주사율(Hz)**
- WebView2 Runtime 버전, ICC 프로파일, HDR on/off, vsync 설정
- 앱 버전, 커밋 해시
- `BOOTHY_APPROVED_CUSTOMER_MONITOR`, `BOOTHY_DISPLAY_SAMPLE_MODE`,
  `BOOTHY_DISPLAY_SAMPLE_GAP_MS`, `BOOTHY_DISPLAY_HIDDEN_PREWARM_STRATEGY`
- 촬영 장비 모델과 프레임레이트

## 2. Immutable generation 증거 — `generations/`

- 두 generation의 확정 파일 경로, `sourceHash`, `byteSize`, 실제 픽셀 크기
- `pointer.json` 전/후 사본 (revision이 단조 증가함을 보인다)
- `generations.jsonl` 전체
- **확정 파일이 존재한 뒤에 pointer가 갱신되었음**을 보이는 host 로그 구간
  (`display_sample_committed`가 `viewer-display-update` emit보다 먼저 나온다)
- 이전 generation 파일이 덮어써지지 않고 그대로 남아 있음

## 3. 거부 경로 증거 — `rejections/`

각 항목마다 host 로그(`display_sample_rejected reason=...`)와 그때의 `pointer.json`을 남긴다.
**거부되었는데 화면이 바뀌지 않았음**을 함께 보인다.

- `partial-file` — staging 파일을 중간에 잘라 재현
- `insufficient-dimensions` — 4K photo rect + 저해상도 fixture
- `session-mismatch` — 세션 교체 직후 이전 세션 request 게시
- `stale-epoch` — sample A와 B 사이에 관람 창 강제 종료 → 재생성
- `older-request` — 두 촬영을 겹쳐 이전 request의 B가 늦게 도착하게 함

## 4. Actual monitor frame 증거 — `frames/` — **HV-18B로 이관됨 (2026-08-12)**

> **이 절은 HV-13B의 통과 조건이 아니다.** 2026-08-12 correct-course로 물리 프레임 증거의
> 소유권이 **Story 7.8 / HV-18B**로 넘어갔다. 절차는 여기 그대로 두어 HV-18B 회차에서 재사용한다.
>
> 이관 근거: 물리 프레임이 증명하는 compositor→photon 구간은 60Hz에서 최대 0.017초인데,
> 실제 고객 체감 지연은 3.7~4.5초이고 그 병목은 source(7.3)·renderer(7.4~7.6) 소유다.
> 그리고 촬영할 가치가 있는 대상 — 실제 preset 적용 사진 — 은 Story 7.4 이후에야 화면에 올라간다.
> 지금 계측용 fixture의 A→B 전환을 찍는 것은 출시 경험을 증명하지 않는다.
> 전문: `_bmad-output/planning-artifacts/sprint-change-proposal-20260812-183435.md`
>
> **HV-18B에서 표시 종단점 귀책의 전환 결함이 나오면 Story 7.2는 `review`로 되돌아간다.**

HV-18B 회차에서 수집한다 (fixture가 아니라 **실제 preset 적용 사진과 RAW 정밀본 교체**가 대상이다):

- 고객 모니터를 **120fps 이상**으로 촬영한 원본 영상
- 전환 구간의 프레임 단위 검토 결과. 다음이 전부 0이어야 한다:
  blank, spinner, stale image, 이전 촬영, crop jump, scale jump, tier downgrade
- 판정 근거 프레임 캡처 (계측 lane을 쓸 때는 fixture의 격자와 코너 registration 마크가 기준선이다)
- 소프트웨어 `actualPresent`와 영상에서 읽은 물리 프레임 시각의 **오프셋 측정값**,
  모니터 주사율과 vsync

## 5. Timing span 증거 — `timing/`

- `<session_root>/diagnostics/viewer-present.jsonl` 원본 (표본별 raw row)
- **계측 완결성 게이트 (통과 필수):** `generations.jsonl`의 committed generation 수와
  `viewer-present.jsonl`의 terminal 행 수가 **정확히 같아야 하고**, generation id가 1:1로 대응해야 한다.
  눈으로 세지 말고 아래를 돌린다. exit code가 0이 아니면 그 회차는 통과가 아니다.

  ```powershell
  ./check-telemetry-completeness.ps1 `
    -SessionEvidenceDir ../run-<timestamp>/session-evidence `
    -OutFile ../run-<timestamp>/timing/completeness.json
  ```

  - 짝이 없는 generation이 하나라도 있으면 그 회차는 **No-Go**다. 남은 표본만으로 집계하지 않는다
    (분모가 줄면 성공률이 실제보다 좋아 보인다).
  - `rejectReason: present-unreported` 행은 host가 유예 시간 안에 viewer 보고를 받지 못했다는 뜻이다.
    행이 존재하는 것 자체는 계측이 정상 동작한 것이고, **그 표본은 실패로 집계**한다.
    같은 행의 `viewerWindowEventsAfterInput`과 host 로그의 `display_present_after_terminal_record` /
    `display_present_report_invalid`를 함께 확인해 원인을 기록한다.
  - `confidence: unreported` 행은 KPI(p50/p95/max)에서 제외하되 **성공률 분모에는 포함**한다.
- 집계: p50 / p95 / max / 성공률. **실패와 timeout을 제외하지 않는다**
- `confidence: low-confidence` 표본을 별도 집계로 함께 보고
- `qualifyingLatencyMicros`가 보수적 구간(`present 상한 − input 하한`)임을 확인
- 진단 span(`fileReady`, `pointerCommitted`, `viewerReceipt`, `decodeEnd`, `imgOnLoad`,
  `elementTimingRender`)이 KPI 종료점으로 승격되지 않았음을 확인
- `isElementRenderTime` 분포 (cross-origin으로 false가 예상된다)

## 6. A/B 증거 — `ab/`

- 변형당 warm-up 5회 + 측정 30회, **randomized AB/BA 순서 기록**
- 동일한 PC·모니터·WebView2 runtime·display profile에서 실행했음을 fingerprint로 확인
- `hidden-prewarm`의 두 은닉 방식(`invisible`, `offscreen`) 각각의 결과
- **hidden 변형이 Story 7.1 readiness 계약을 유지하지 못했다면 그 관측을 그대로 남긴다.**
  `stale-report`로 촬영이 막혔거나 rAF가 멈춰 계측이 성립하지 않은 것도 유효한 결과다.
  `VIEWER_REPORT_STALE_AFTER_MS`를 늘려 통과시키지 않는다
- 선택된 기본값과 근거

## 7. AC 4 실패 조건 증거 — `window-events/`

- 표본별 `viewerWindowEventsAfterInput`가 0임을 보이는 집계
- host 로그에서 trusted input 이후 구간에 `viewer_window_opened`가 없음
- 0이 아닌 표본이 있다면 **실패 표본으로 보고**하고 원인을 기록한다
  (`get_capture_readiness` polling이 창을 재생성하는 경로가 실제로 존재한다)

## 8. Read-only 유지 — `read-only/`

- 사진이 표시된 상태의 고객 모니터 전체 화면 사진: 조작 요소와 진단 표시가 0
- 표시 실패(decode 실패, 크기 미달) 시 관람 화면이 아니라 booth control surface에만 안내가 뜨는 대비 사진

## 9. 판정 기록

수집이 끝나면 `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`의
Story 7.2 행에 다음을 기록한다.

- `evidence package path`: 이 회차 디렉터리의 실제 경로
- `Go / No-Go result`
- `executedAt`, `validator`, `booth PC`, `camera model`, `monitor`, `capture rig`
- No-Go면 `release blocker`와 `rerun prerequisite`를 갱신한다

`Go`를 기록한 뒤에만 story status를 `done`으로 전환한다.
**자동 테스트 통과만으로 `done` 처리하지 않는다.**

## 참고

- 계약 문서: `docs/contracts/viewer-display.md`
- Story 7.1 계약: `docs/contracts/viewer-readiness.md`
- Story: `_bmad-output/implementation-artifacts/7-2-immutable-sample과-actual-present-계측.md`
- fixture 생성기: `storage/fixtures/display-sample/generate-samples.ps1`
