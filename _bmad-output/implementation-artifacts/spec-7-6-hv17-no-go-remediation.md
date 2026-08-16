---
title: 'Story 7.6 HV-17 No-Go 개선 및 실장비 재검증'
type: 'bugfix'
created: '2026-08-17'
status: 'done'
baseline_commit: 'c390f53e2079e9379cb36501517fa05e26beb41d'
context:
  - '_bmad-output/implementation-artifacts/7-6-raw-정밀본-무중단-교체.md'
  - '_bmad-output/implementation-artifacts/hardware-validation-ledger.md'
  - 'tests/hardware/raw-refined/hv-17/README.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 첫 실장비 회차는 EOS 700D RAW 5/5와 proxy 표시 5/5에는 성공했지만, 수동 노출이 F/10·1/100초·ISO 400이라 거의 검은 결과만 만들었다. 또한 정밀본은 `NotMeasured`인 동안 고객 게시가 막히므로, 고객 안전장치를 우회하지 않고도 `--hq false/true` 쌍을 먼저 측정할 독립 evidence 경로가 필요하다. 취소 시 실제 taskkill/process-tree 결과도 구조화되어 남지 않아 HV-17A를 닫을 수 없다.

**Approach:** 실제 고객 lane과 분리된 HV-17 evidence runner로 EOS RAW·세 preset의 proxy/refined 쌍을 만들고 기존 품질 지표로 판정한다. 동시에 실제 darktable 취소에서 taskkill 원문·종료 시각·orphan 수를 기록한 뒤 같은 PC·카메라·모니터에서 HV-17A/B를 다시 실행한다.

## Boundaries & Constraints

**Always:** descriptor가 실제 지원하는 카메라 값만 적용하고 readback한다. proxy/refined는 같은 RAW·XMP·크기·renderer를 쓰며 `--hq`만 달라야 한다. 측정·사람 검토·취소 회차가 없으면 실패로 남긴다. 기존 사용자 변경과 별도 `Boothy Selectroom` 프로세스를 보존한다.

**Ask First:** 제품 승인 기준, 관찰자 3명 요구, 기본 lane 상태를 증거 없이 완화해야만 진행 가능한 경우. 카메라의 비가역 설정이나 승인되지 않은 dependency가 필요한 경우.

**Never:** `NotMeasured`를 임의로 `Justified`로 바꾸기, synthetic 이미지를 실장비 corpus로 제출하기, taskkill/observer 결과를 생성해 내기, refined 측정 전에 고객 기본 lane을 켜기.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 정상 corpus | 정상 노출 EOS RAW 3장 이상, 세 preset, 지정 ROI | 9쌍 이상과 per-sample MTF50/ΔE00/clipping, 재현 가능한 verdict | 기준 통과 여부를 그대로 기록 |
| 저노출·평면 corpus | 검거나 유효 slanted edge 없음 | tier `not-measured`, 고객 게시 없음 | 재촬영 조건과 실패 사유 기록 |
| 실제 렌더 취소 | 네 취소 사유 중 실행 중 darktable 존재 | taskkill stdout/stderr, 종료 시각, fallback, orphan 수 기록 | probe 불가와 orphan 0을 구분 |
| RAW handoff 오류 | RAW+JPEG에서 RAW 미도착 또는 설정 불일치 | descriptor 승인 RAW-only 적용·readback 후 재시도 | 지원값 없으면 중단하고 실패 기록 |

</frozen-after-approval>

## Code Map

- `src-tauri/src/render/mod.rs` -- 실제 darktable 실행과 process-tree 종료 경계
- `src-tauri/src/display/raw_refined_publisher.rs` -- tier 승인 전 고객 게시 차단과 refined publication
- `src/quality-metrics/parity-metrics.ts` -- PPM decode, MTF50, ΔE00, clipping 측정
- `src/quality-metrics/tier-justification.ts` -- 3×3 corpus 판정
- `sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs` -- EOS descriptor/readback 기반 촬영 설정
- `tests/hardware/raw-refined/hv-17/` -- evidence runner, 기계식 gate와 실행 계약

## Tasks & Acceptance

**Execution:**
- [x] `tests/hardware/raw-refined/hv-17/` -- 실제 RAW에서 세 preset proxy/refined 쌍과 ROI 기반 지표·verdict를 생성하는 evidence runner 및 실패 검사를 추가한다.
- [x] `src-tauri/src/render/mod.rs` -- 명시적 HV-17 evidence root에서만 실제 taskkill stdout/stderr와 구조화된 process-tree 종료 결과를 보존하고 단위 테스트한다.
- [x] `tests/hardware/raw-refined/hv-17/tools/` -- descriptor 지원 RAW-only와 노출값을 선택·readback하는 카메라 preflight 도구를 추가하고 원래 노출 설정을 기록한다.
- [x] 실장비 재실행 -- 정상 노출 RAW 3장 × preset 3개를 완료하고 gate stop-rule까지 판정했다. 실측이 `tier-not-justified` 및 `refined-look-drift`로 끝나 publication gate가 닫혔으므로 refined 전환·네 P1 취소 회차·사람 관찰은 실행하지 않았다. 5-shot/cancel 증거도 이번 회차에는 미수집으로 명시했다.

**Acceptance Criteria:**
- Given 승인된 EOS 700D와 정상 노출 slanted-edge RAW 3장 이상, when evidence runner를 실행하면, then 세 preset의 최소 9쌍이 실제 production-equivalent 인자로 렌더되고 모든 pair의 원자료와 verdict가 남는다.
- Given 실행 중인 display darktable 작업, when delete/session replace/viewer epoch/newer capture로 취소하면, then 네 회차 각각 실제 taskkill 원문·완료 시각·orphan count가 기록되고 final 결과는 보존된다.
- Given 새 evidence package, when HV-17 gate를 실행하면, then 각 sub-gate가 독립 판정되며 미수집 사람 증거는 통과로 간주되지 않는다.

## Spec Change Log

- 2026-08-17: 자동화와 카메라 preflight를 구현하고 실제 EOS 700D 정상 노출 RAW 3장 × preset 3개를 재측정했다. median MTF50 비율 0.99947, 5개 역행, median ΔE00 4.5213, median p95 ΔE00 15.0056으로 채택 기준을 넘지 못해 제품 lane은 off로 유지한다. 회차: `tests/hardware/raw-refined/run-20260817-023031-hv17-rerun/`.

## Design Notes

측정 runner는 고객 pointer를 전진시키지 않는다. 먼저 offline pair를 측정하고 verdict가 `justified`일 때만 근거를 코드와 evidence에 연결해 실제 refined publication 회차를 연다. `not-justified`면 lane을 끈 채 Story의 승인된 Partial 규칙으로만 심사한다.

## Verification

**2026-08-17 result:** Vitest 44/44, Rust lib 211/211, Canon helper 46/46, HV-17 gate self-test 19/19 통과. 실제 gate는 HV-17A/HV-17B 각각 No-Go이며, `tier-not-justified`, `refined-look-drift`, 누락 증거를 독립적으로 보고한다.

**Commands:**
- `pnpm exec vitest run src/quality-metrics` -- 지표와 tier 판정 회귀 없음
- `cargo test --lib render::` -- scheduler/process-tree/evidence 기록 통과
- `dotnet test sidecar/canon-helper/tests/CanonHelper.Tests/CanonHelper.Tests.csproj` -- 카메라 설정 선택·복구 회귀 없음
- `tests/hardware/raw-refined/hv-17/test-check-raw-refined-evidence.ps1` -- gate 자체 검증 전부 통과
- `tests/hardware/raw-refined/hv-17/check-raw-refined-evidence.ps1` -- 새 실장비 패키지의 독립 판정 출력

## Suggested Review Order

**실측 진입점과 판정**

- 실제 RAW·preset을 패키지 내부에 고정하고 3×3 비교를 수행한다.
  [`run-tier-evidence.ts:213`](../../tests/hardware/raw-refined/hv-17/run-tier-evidence.ts#L213)

- 실측 실패를 제품 publication 차단 상태로 명시한다.
  [`raw_refined_publisher.rs:113`](../../src-tauri/src/display/raw_refined_publisher.rs#L113)

**실장비 안전 경계**

- 카메라 값은 descriptor exact member만 적용하고 readback한다.
  [`Program.cs:137`](../../tests/hardware/raw-refined/hv-17/tools/camera-preflight/Program.cs#L137)

- RAW-only 선택은 camera descriptor 밖 값을 합성하지 않는다.
  [`ImageQualityCapability.cs:91`](../../sidecar/canon-helper/src/CanonHelper/Runtime/ImageQualityCapability.cs#L91)

- 명시적 HV-17 root에만 taskkill 원문과 process-tree 결과를 남긴다.
  [`mod.rs:1867`](../../src-tauri/src/render/mod.rs#L1867)

**독립 gate와 증거**

- `tier-not-justified`를 HV-17B 통과로 오인하지 않는다.
  [`check-raw-refined-evidence.ps1:342`](../../tests/hardware/raw-refined/hv-17/check-raw-refined-evidence.ps1#L342)

- 실장비 결론과 미수집 항목을 한 곳에 기록한다.
  [`result.md:1`](../../tests/hardware/raw-refined/run-20260817-023031-hv17-rerun/result.md#L1)

**회귀 검증과 기록**

- 측정 후 부적합 gate 회귀를 독립 케이스로 고정한다.
  [`test-check-raw-refined-evidence.ps1:308`](../../tests/hardware/raw-refined/hv-17/test-check-raw-refined-evidence.ps1#L308)

- ledger가 route rejected와 남은 HV-17A 증거를 함께 표시한다.
  [`hardware-validation-ledger.md:76`](hardware-validation-ledger.md#L76)
