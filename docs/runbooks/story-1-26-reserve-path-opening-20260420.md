---
documentType: opening-note
status: active
date: 2026-04-20
scope: preview-track
---

# Story 1.26 Reserve Path Opening

## 왜 이 문서가 필요한가

- Story `1.10` old `resident first-visible` line은 최신 승인 하드웨어 재검증까지 반영한 결과, baseline evidence는 다시 닫았지만 official `preset-applied visible <= 3000ms` gate는 닫지 못했다.
- 따라서 old line은 더 이상 primary execution lane이 아니라 closed `No-Go` baseline으로 읽어야 한다.
- 이 문서는 그 판단 위에서 Story `1.26`을 공식 오픈하고, 다음 실험 경로의 범위를 좁게 고정하기 위해 만든 current opening note다.

## Opening Decision

- opening date: `2026-04-20`
- frozen baseline: Story `1.10`
- active reserve path: Story `1.26`
- official verdict owner: `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`

제품 해석:

- `1.30`은 actual-primary-lane의 bounded `No-Go` evidence다.
- `1.10`은 old line의 closed `No-Go` baseline이다.
- `1.31`은 여전히 unopened success-side default/rollback gate다.
- 지금 active하게 진행할 preview route는 Story `1.26` 하나다.

## Story 1.26의 목표

Story `1.26`의 목표는 old line을 더 깎는 것이 아니다.

목표는 승인 하드웨어에서 official release gate인 `preset-applied visible <= 3000ms`, 즉 `originalVisibleToPresetAppliedVisibleMs <= 3000ms`를 닫을 수 있는 새 reserve topology를 검증하는 것이다.

현재 문서 집합 기준으로 그 새 reserve topology는 아래로 읽는다.

- `host-owned local native/GPU resident full-screen lane`
- `display-sized preset-applied truthful artifact`

즉 per-capture `darktable-cli` hot path를 계속 미세조정하는 대신, host가 소유하는 local native/GPU resident path에서 booth-visible hot path를 줄이고, 고객에게 보이는 truthful close artifact는 display-sized preset-applied 결과로 다시 닫는 방향이다.

## Scope In

- host-owned local native/GPU resident full-screen preview lane 정의
- display-sized preset-applied truthful artifact를 `previewReady` truth owner로 다시 닫는 방법 정의
- same-session, same-capture correctness 유지
- truthful `Preview Waiting` 유지
- wrong-capture 0 / cross-session leakage 0 유지
- 승인 하드웨어 one-session package로 official gate를 직접 판정하는 검증 패키지 정의
- darktable path를 parity reference, fallback, final/export truth로 유지하는 경계 정의

## Scope Out

- old line CPU/GPU rerun을 primary execution path로 되돌리는 일
- `darktable-cli` 옵션, wait-budget, preview size 같은 미세 조정 반복
- Story `1.31` 재오픈
- remote renderer / edge appliance / watch-folder bridge 실험
- booth copy나 화면 문구의 제품 카피 리라이트
- final/export pipeline 전체 교체

## 성공 기준

- official gate:
  - `originalVisibleToPresetAppliedVisibleMs <= 3000ms`
  - product wording: `preset-applied visible <= 3000ms`
- reference/comparison metrics:
  - `sameCaptureFullScreenVisibleMs`
  - first-visible feel

성공 판정 원칙:

- first-visible가 빨라도 official gate를 못 닫으면 release success가 아니다.
- display-sized preset-applied truthful artifact가 실제 customer-visible close owner여야 한다.
- ledger에 `Go`가 기록되기 전까지는 route success를 주장하지 않는다.

## 진행 순서

1. Story `1.10`을 closed `No-Go` baseline으로 고정한다.
2. Story `1.26` story artifact를 만들고 sprint tracking에서 `ready-for-dev`로 연다.
3. reserve topology를 `host-owned local native/GPU resident full-screen lane + display-sized preset-applied truthful artifact` 범위로만 구현한다.
4. per-session seam instrumentation과 official gate logging을 그대로 유지한다.
5. 승인 하드웨어 one-session package를 수집해 ledger에 `Go / No-Go`를 기록한다.
6. `Go`면 해당 lane을 release candidate로 승격 검토하고, `No-Go`면 이 reserve path도 bounded failure로 닫는다.

## 실행 가드레일

- `sameCaptureFullScreenVisibleMs`를 공식 합격선으로 되돌리지 않는다.
- first-visible source를 truth owner로 승격하지 않는다.
- old line better-run 숫자를 release proof처럼 재해석하지 않는다.
- `darktable-cli` hot path를 이번 story의 primary owner로 되돌리지 않는다.
- story note만으로 성공을 선언하지 않는다. 공식 판정은 ledger row가 소유한다.

## Canonical Reading Order

1. `docs/README.md`
2. `docs/runbooks/story-1-26-reserve-path-opening-20260420.md`
3. `docs/runbooks/current-preview-gpu-direction-20260419.md`
4. `docs/runbooks/current-actual-lane-handoff-20260419.md`
5. `docs/runbooks/preview-track-route-decision-20260418.md`
6. `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`

## 관련 문서

- [Current Preview GPU Direction](./current-preview-gpu-direction-20260419.md)
- [Current Actual-Lane Handoff](./current-actual-lane-handoff-20260419.md)
- [Preview Track Route Decision](./preview-track-route-decision-20260418.md)
- [Release Baseline](../release-baseline.md)
- [Preview Architecture History And Agent Guide](../preview-architecture-history-and-agent-guide.md)

## Latest Route Note - 2026-04-24 11:36 +09:00

- Latest fast-preview cached XMP `iop_order_list` trimming produced a stable 5/5 hardware validation run on `session_000000000018a92a6c02e7f2d4`.
- The `Kim4821` prompt still records the session as `Kim 4821`.
- Official `preset-applied visible <= 3000ms` is now closed: latest readings were `2956ms`, `2951ms`, `2961ms`, `2954ms`, and `2960ms`.
- Story `1.26` now has ledger `Go` evidence. The next product step is not more tail chasing; it is visual acceptability review for the trimmed preview XMP and deciding whether Story `1.31` should open as the success-side default / rollback gate.
