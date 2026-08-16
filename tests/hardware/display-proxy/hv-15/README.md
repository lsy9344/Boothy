# HV-15 — 화면 적합 immutable preset proxy 실장비 검증

Story 7.4 (`7-4-화면-적합-immutable-preset-proxy`) 종료 게이트.

> ## ⛔ 선행 조건: HV-14 route 결정
>
> Epic 7 evidence dependency는 `7.3 approved route decision → 7.4 Go`다.
> **HV-14는 2026-08-14 현재 `No-Go`다** (3 회차 중단, 저장된 17 captures의 51 route 행 전부 거부).
> 이 게이트는 HV-14가 route 결정을 기록하기 전까지 `Go`로 닫을 수 없다.
>
> 구현과 자동 검증은 route와 무관하게 완결되어 있고, **이미 게시된 세 preset이 정확 경로로
> 동작한다.** 회차는 기본 source(`raw-original`)로 지금도 실행할 수 있지만,
> **그 결과만으로 HV-15 `Go`를 기록하지 않는다.**

## 이 게이트가 증명하는 것

| 항목 | 증명 대상 |
| --- | --- |
| display profile | 목표 크기가 **실제 photoRect × DPR**에서 왔다 |
| proxy bundle metadata | 화면에 오른 룩이 **승인된** preset이다 |
| generation manifest | 자산이 immutable하게 확정됐다 |
| publish/decode evidence | 프로세스 종료 → 구조 decode → 크기 → provenance → commit → notify 순서를 지켰다 |
| actual-present spans | 한 clock으로 측정된 종단점이 기록됐다 |
| zero-wrong-frame | 잘못된 프레임이 한 장도 없었다 |

## 이 게이트가 증명하지 않는 것

- **warm 100-shot 성능 판정** → Story 7.8 / HV-18B
- **120fps+ 물리 모니터 프레임과 compositor→photon 오프셋** → Story 7.8 / HV-18B (2026-08-12 이관)
- **RAW 정밀본 무중단 교체** → Story 7.6 / HV-17
- **상주 renderer 채택 결정** → Story 7.5 / HV-16

## 속도는 합격 조건이 아니다

AC 5가 명시한다. 현재 one-shot 렌더 기준선은 display-fit 1920px에서 **CPU 약 3.53초**이고
실제 pixelpipe 계산은 약 0.3초다. 비용 대부분이 매 촬영 프로세스·core 초기화이며,
그것을 없애는 것은 Story 7.5의 일이다.

**5초 예산을 넘으면 넘은 대로 기록한다.** 임계값을 올리거나 느린 표본을 제외하면
Story 7.5의 판단 근거가 사라진다.

## 회차 준비

### 1. 환경 확정 (`environment.md`)

- 승인 부스 PC, EOS 700D 펌웨어/렌즈/카드/케이블/USB 포트
- EDSDK·helper 버전, 앱 버전과 커밋 해시
- **고객 모니터** 모델/해상도/DPR/주사율, 승인 display profile ID
- WebView2 runtime, GPU/driver, ICC, HDR on/off
- darktable 버전 — pinned `5.4.1`과 일치해야 한다
- 환경 변수 상태
  - `BOOTHY_DISPLAY_PROXY_MODE=on`
  - `BOOTHY_DISPLAY_SAMPLE_MODE` — **반드시 미설정/off.** 켜져 있으면 fixture와 실제 결과가
    같은 pointer를 다투고 lane 충돌로 proxy가 시작조차 하지 않는다
  - `BOOTHY_SOURCE_COMPARE_MODE` — **반드시 미설정/off.** 표본이 서로 오염된다

### 2. preset 확보 — 추가 작업 없음

**이미 게시된 세 preset이 그대로 쓰인다** (`preset_soft-glow`, `preset_mono-pop`,
`preset_daylight` @ 2026.03.27). RAW 원본에서 렌더하는 정확 경로는 오늘의 preview/final과
엔진·recipe·source가 같고 출력 크기만 화면에 맞춘 것이라 별도 시각 승인 대상이 아니다.
자산에는 `approvalBasis: exact-reference-renderer`가 기록된다.

`proxyPublication` 시각 승인은 **HV-14가 fast source route를 승인한 뒤** 그 route를 쓸 때
필요해진다. 그때의 승인 절차는 `docs/contracts/preset-bundle.md`의 화질 승인 기준을 따른다.

### 3. 시나리오

| # | 시나리오 | 확인 |
| --- | --- | --- |
| S1 | 단일 촬영 | 첫 성공 화면이 preset 적용 display-fit 이미지다 |
| S2 | 연속 촬영 3회 | 항상 **최신** 촬영이 화면에 남는다 |
| S3 | 촬영 중 preset 변경 후 촬영 | 이전 preset의 늦은 결과가 화면을 덮지 않는다 |
| S4 | 촬영 직후 삭제 | pointer가 정직하게 비워진다 |
| S5 | 참조 렌더러 pin 불일치 번들 | proxy lane이 시작조차 하지 않고 진단에 사유가 남는다 |
| S6 | 세션 종료 후 새 세션 | 이전 세션 자산이 한 프레임도 남지 않는다 |

## 증거 패키지 구조

```
run-<timestamp>-hv15/
├── result.md                     # Go / No-Go와 근거
├── environment.md                # 위 1번 확정값
├── bundle/                       # 사용된 preset의 proxyPublication 원문과 승인 근거
├── display-profile/summary.json  # photoRect와 requiredSource 실측값
├── generations/                  # pointer.json 전/후, generations.jsonl, 확정 파일 해시
├── timing/summary.json           # 표본별 raw + p50/p95/max/성공률
├── wrong-frame/summary.json      # zero-wrong-frame 보고
└── frames/README.md              # 물리 프레임은 HV-18B 소유임을 명시
```

## 통과 필수 게이트

1. **계측 완결성** — `tests/hardware/viewer-present/hv-13b/check-telemetry-completeness.ps1`
   commit된 generation 하나당 terminal 행이 **정확히 하나**여야 한다.
   Story 7.2가 이 결함으로 한 회차를 잃었다.
2. **approvalBasis** — 모든 proxy generation이 `exact-reference-renderer` 또는
   `visual-approval` 중 하나를 기록해야 하고, 정확 경로를 주장하는 프레임의 `sourceRoute`는
   반드시 `raw-original`이어야 한다.
3. **display fit** — 모든 accepted generation이 `object-fit: contain`에서 확대 없이 표시되어야 한다.
   즉 `sourceWidthPx >= requiredSourceWidthPx` 또는 `sourceHeightPx >= requiredSourceHeightPx`이고,
   `targetWidthPx/targetHeightPx`가 그 회차의 실측 photoRect와 일치해야 한다.
   세로 사진은 높이 기준 letterbox를 허용하지만 임의 crop과 upscale은 허용하지 않는다.
4. **provenance** — 모든 proxy generation이 `presetId`, `presetVersion`, `sourceRoute`,
   `sourceAssetHash`, `referenceRendererVersion`을 싣고, preset은 capture record의 결속과 같아야 한다.
5. **zero-wrong-frame** — wrong-session / wrong-request / wrong-capture / **wrong-preset** /
   unfiltered qualifying / blank / stale / **upscaled** / crop jump / scale jump / tier downgrade가 전부 0.
6. **route 결정** — HV-14가 승인한 route가 기록되어 있어야 한다.

## 실행

```powershell
# 표본 완결성 (Story 7.2 게이트를 그대로 태운다)
pwsh tests/hardware/viewer-present/hv-13b/check-telemetry-completeness.ps1 `
  -SessionRoot "<session_root>"

# Story 7.4 전용 게이트
pwsh tests/hardware/display-proxy/hv-15/check-proxy-evidence.ps1 `
  -SessionRoot "<session_root>" `
  -ExpectedPresetId "<preset id>" `
  -ExpectedPresetVersion "<published version>" `
  -RequiredSourceWidthPx <w> -RequiredSourceHeightPx <h>
```

**스크립트 PASS는 기계적 완결성만 증명한다.** display profile 확정, bundle 승인 근거,
품질 corpus, zero-wrong-frame 사람 검토, HV-14 route 결정이 모두 갖춰지기 전에는
HV-15 `Go`가 아니다.

## 종료 처리

- `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`의 Story 7.4 행을 갱신한다.
- `Go`가 기록된 뒤에만 story status를 `done`으로 전환한다.
  **자동 테스트 통과만으로 `done` 처리하지 않는다.**
- `Go` 이후 `BOOTHY_DISPLAY_PROXY_MODE`의 기본값을 `on`으로 전환하고, 그 기본값을 테스트로 고정한다.
