---
documentType: rerun-runbook
status: active
date: 2026-04-19
scope: old-resident-first-visible-validation-lane
---

# Old Resident First-Visible CPU Baseline Rerun

## 목적

- 이 runbook은 old `resident first-visible` line의 `one-session revalidation evidence`를 다시 닫기 위한 실행 기준이다.
- 이번 rerun은 release winner 선발이나 route promotion이 아니다.
- 이번 rerun의 목적은 current contract 아래에서 historical feel candidate가 실제로 다시 재현되는지 확인하는 것이다.

## 해석 경계

- official release gate는 그대로 `sameCaptureFullScreenVisibleMs <= 3000ms`와 `originalVisibleToPresetAppliedVisibleMs <= 3000ms`다.
- 이번 rerun의 1차 성공은 `one-session package` 안에서 first-visible seam, truthful close seam, `Preview Waiting` truth, same-slot replacement, wrong-capture 0을 다시 설명할 수 있는지다.
- 과거 better run은 rerun priority를 높이는 comparison evidence일 뿐 current release-proof가 아니다.
- GPU는 이번 문서에서 decision input이 아니라 future comparison hypothesis다.

## CPU Baseline Interpretation

- 이번 rerun은 `GPU off` 실험이 아니다.
- current approved booth-safe profile을 그대로 사용한다.
  - `disable_opencl: false`
  - `allow_fast_preview_raster: true`
- 따라서 이번 rerun의 timing은 `current approved profile baseline`으로만 읽는다.
- 이번 회차에서는 GPU/OpenCL capability와 observed state를 메타데이터로만 기록한다.
- `GPU가 켜져 있었으니 빨라졌다/느려졌다` 같은 비교 해석은 금지한다.
- GPU 비교는 동일 evidence package 형식의 matched rerun pair가 준비된 뒤에만 별도 gate로 연다.

## Final Evidence Package

### Package Root

- approved hardware에서 실행한 latest single session folder 1개를 canonical package root로 사용한다.
- package root 예시:
  - `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_<id>\`

### Required Files

| Item | Required path | Pass condition |
| --- | --- | --- |
| Session truth | `session.json` | latest capture record와 timing fields를 읽을 수 있어야 한다 |
| Per-session seam log | `diagnostics/timing-events.log` | required seam chain과 request correlation을 읽을 수 있어야 한다 |
| Helper correlation | `diagnostics/camera-helper-events.jsonl` | same request/capture의 helper-side correlation을 읽을 수 있어야 한다 |
| Latest canonical preview | `renders/previews/{captureId}.jpg` | latest capture canonical preview path가 same-slot replacement 대상이어야 한다 |
| Same capture RAW | `captures/originals/{captureFile}` | same capture 원본 RAW가 남아 있어야 한다 |
| Active preset bundle | `preset-catalog/published/.../bundle.json` | capture가 바인딩한 active preset/version과 일치해야 한다 |
| Catalog snapshot | `preset-catalog/catalog-state.json` | run 시점 catalog snapshot을 읽을 수 있어야 한다 |

### Required Metadata

- `sessionId`
- `requestId`
- `captureId`
- darktable pin
- helper identifier
- booth PC
- camera model
- observed OpenCL/GPU capability state

### Package Validity Rule

- 위 파일이 모두 있어야 `successful revalidation package`로 읽을 수 있다.
- expected preview file이 없거나 seam event가 빠지면 rerun을 성공으로 닫지 않는다.
- 단, 실패 회차라도 이미 생성된 session folder와 diagnostics는 보존하고 `No-Go` evidence로 남긴다. 누락을 덮기 위해 파일을 재생성하거나 수동 수정하면 안 된다.

## CPU Baseline Prep Checklist

### A. Pre-run Environment

- [ ] approved booth hardware를 사용한다.
- [ ] darktable pin을 기록한다.
- [ ] booth PC 이름과 camera model을 기록한다.
- [ ] helper identifier를 기록한다.
- [ ] current profile이 approved booth-safe baseline인지 확인한다.
  - expected: `disable_opencl=false`, `allow_fast_preview_raster=true`
- [ ] 이번 회차를 `GPU comparison closed` 상태로 시작한다고 기록한다.
  - meaning: GPU/OpenCL capability는 메타만 남기고 성능 비교 판단은 하지 않는다.

### B. Seam Chain Closure

- [ ] `request-capture`가 same session `timing-events.log`에 남는다.
- [ ] `file-arrived`가 same `requestId`와 same `captureId`로 닫힌다.
- [ ] `fast-preview-visible` 또는 동등 first-visible event가 same capture에 연결된다.
- [ ] `preview-render-start`가 same capture later truth lane으로 이어진다.
- [ ] `capture_preview_ready`가 same capture later truthful close를 닫는다.
- [ ] `recent-session-visible`이 same capture latest slot replacement를 다시 기록한다.

### C. Correlation Continuity

- [ ] `sessionId`가 session folder와 `session.json`에서 일치한다.
- [ ] `requestId`가 `request-capture -> file-arrived -> fast-preview-visible -> recent-session-visible`까지 끊기지 않는다.
- [ ] `captureId`가 helper correlation, `session.json`, canonical preview path, UI visible event에서 일치한다.

### D. Preview Truth And Replacement

- [ ] first-visible이 먼저 떠도 `Preview Waiting`은 유지된다.
- [ ] `preview.readyAtMs`가 `null`인 동안 false-ready가 발생하지 않는다.
- [ ] same latest slot에서 pending preview가 later truthful preview로 교체된다.
- [ ] canonical preview path를 잃지 않고 same-slot replacement가 완료된다.

### E. Owner Readout

- [ ] resident/speculative lane이 actual `first-visible owner`인지 설명할 수 있다.
- [ ] later render worker가 actual `truthful close owner`인지 설명할 수 있다.
- [ ] raw/direct fallback이 winning close owner로 승격되지 않았음을 설명할 수 있다.

### F. Story 1.10 Pending To Rerun Checks

- [ ] per-session seam instrumentation pending item:
  - same session `timing-events.log`만으로 required seam chain을 닫을 수 있어야 한다.
- [ ] correlation pending item:
  - `requestId`, `captureId`, `sessionId` continuity를 one-session package에서 직접 설명할 수 있어야 한다.
- [ ] hardware package pending item:
  - 위 required files와 metadata가 session 1개에 묶여 있어야 한다.
- [ ] correctness pending item:
  - `Preview Waiting` truth, same-slot replacement, cross-session isolation, wrong-capture 0을 한 회차에서 같이 설명할 수 있어야 한다.

### G. Suggested Local Verification Commands

```powershell
$session = 'C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_<id>'
Get-Item "$session\session.json"
Get-Item "$session\diagnostics\timing-events.log"
Get-Item "$session\diagnostics\camera-helper-events.jsonl"
Get-ChildItem "$session\renders\previews"
Get-ChildItem "$session\captures\originals"
```

```powershell
$timing = Get-Content "$session\diagnostics\timing-events.log"
$timing | Select-String 'event=request-capture|event=file-arrived|event=fast-preview-visible|event=preview-render-start|event=capture_preview_ready|event=recent-session-visible'
```

```powershell
$timing | Select-String 'request='
$timing | Select-String 'sourceAsset=raw-original'
```

## CPU Rerun Plan

### Step 1. Baseline Freeze

- approved hardware에서 active preset을 고정한다.
- 이번 회차는 `historical feel candidate revalidation`이라고 기록한다.
- release challenge가 아니라는 점을 시작 메모에 명시한다.
- GPU/OpenCL capability는 기록만 하고 비교 해석은 잠근다.

### Step 2. Single Session Capture

- booth 앱의 supported `사진 찍기` 버튼으로 capture를 실행한다.
- 같은 session 안에서 아래를 순서대로 관찰한다.
  - `Preview Waiting` 진입
  - same-capture first-visible arrival
  - same latest slot later truthful replacement
- 관찰 포인트는 숫자 하나가 아니라 owner다.
  - resident/speculative lane이 first-visible owner인지
  - render worker가 truthful close owner인지
  - raw/direct fallback이 뒤로 물러났는지

### Step 3. Session Readout

- session folder 하나만으로 evidence package를 수집한다.
- one-session package에서 아래를 읽는다.
  - seam chain close 여부
  - request/capture/session continuity
  - `Preview Waiting` truth 유지 여부
  - same-slot replacement 유지 여부
  - wrong-capture 0 여부
- dual 3000ms gate는 별도 release column으로만 읽는다.
- 이 회차의 결론은 `revalidation success / revalidation fail / release gate fail`로 분리해서 적는다.

## GPU Comparison Gate

아래 조건이 모두 충족될 때만 GPU comparison을 연다.

- CPU baseline one-session package가 complete하다.
- same-capture first-visible이 안정적으로 재현된다.
- `Preview Waiting` truth가 유지된다.
- same-slot replacement가 유지된다.
- truthful close owner가 later render worker로 읽힌다.
- raw/direct fallback이 winning close owner가 아니다.
- wrong-capture, stale replacement, false-ready, cross-session leakage가 0이다.

GPU comparison을 열더라도 규칙은 같다.

- evidence package format은 CPU baseline과 동일해야 한다.
- GPU 결과만으로 route promotion을 판단하지 않는다.
- GPU 결과만으로 release success를 주장하지 않는다.

## Success Criteria

### Revalidation Success

- same-capture first-visible이 one-session package에서 안정적으로 재현된다.
- `Preview Waiting` truth가 끝까지 유지된다.
- same-slot replacement가 latest slot에서 자연스럽게 닫힌다.
- `wrong-capture = 0`
- resident/speculative lane이 actual first-visible owner로 읽힌다.
- later render worker가 truthful close owner로 읽힌다.
- raw/direct fallback은 winner가 아니다.

### Release Success

아래는 separate gate다.

- `sameCaptureFullScreenVisibleMs <= 3000ms`
- `originalVisibleToPresetAppliedVisibleMs <= 3000ms`

revalidation success와 release success는 같은 뜻이 아니다.

## Stop Criteria

- required seam chain이 one-session package에서 닫히지 않으면 즉시 중단한다.
- `requestId / captureId / sessionId` continuity가 설명되지 않으면 중단한다.
- truthful close owner가 fallback/raw로 계속 읽히면 중단한다.
- raw/direct fallback이 winning close owner처럼 남으면 중단한다.
- `Preview Waiting` 중 false-ready가 나오면 즉시 중단한다.
- wrong-capture, stale replacement, cross-session leakage가 한 번이라도 나오면 즉시 중단한다.
- required files가 누락되면 회차를 `No-Go evidence`로만 보존하고 성공 판단은 닫지 않는다.

## Ready For Rerun

- repo/document 기준 준비 상태: `Yes`
- meaning:
  - big code change 없이도 rerun 직전 운영 기준은 고정됐다.
  - 남은 것은 approved hardware access와 run-time metadata 기록 같은 execution-side 준비다.

## Remaining Blockers

- approved hardware가 실제로 사용 가능해야 한다.
- run operator가 booth PC / camera model / helper identifier / observed GPU capability를 현장에서 즉시 기록해야 한다.
- session folder를 확보한 직후 evidence package를 보존해야 한다. evidence 확보 전에 재시도하거나 수동 수정하면 이번 lane의 판단 근거가 무너진다.

## Cross References

- general hardware checklist: `docs/runbooks/booth-hardware-validation-checklist.md`
- route interpretation: `docs/runbooks/current-actual-lane-handoff-20260419.md`
- route decision boundary: `docs/runbooks/preview-track-route-decision-20260418.md`
- validation candidate spec: `_bmad-output/implementation-artifacts/1-10-known-good-preview-lane-복구와-상주형-first-visible-worker-도입.md`
- canonical ledger row: `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`
