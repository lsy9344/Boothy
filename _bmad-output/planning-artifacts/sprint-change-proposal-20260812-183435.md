# Sprint Change Proposal — HV-13B 물리 프레임 게이트 이관

- 작성 시각: `2026-08-12T18:34:35+09:00`
- 요청자: Noah Lee
- 유형: Epic 7 evidence dependency 재조정 (scope 축소 아님, 게이트 이관)
- 영향 Story: 7.2 (closure), 7.3 (착수 해제), 7.8 (증거 인수)

## 문제

Story 7.2는 자동 검증과 실장비 소프트웨어 증거를 모두 통과했지만, **120fps 이상 외부 촬영 rig가
없다는 단 하나의 이유로** HV-13B가 `No-Go`에 묶여 있었다. Epic 7 dependency가
`7.2 Go -> 7.3`이므로 7.3~7.6 전체가 그 뒤에서 대기 중이었다.

측정된 사실:

- 표시 종단점의 소프트웨어 계측은 통과했다. 140/140 immutable generation 무결성,
  120/120 actual-present, 입력 이후 창 이벤트 0건.
- 물리 프레임 게이트가 증명하려는 것은 **compositor → photon 구간**이다. 60Hz 모니터에서
  최대 16.7ms, 즉 **0.017초**다.
- 같은 회차에서 관측된 실제 고객 체감 지연은 **3.7~4.5초**이고, 미리보기는 5초 예산을
  6.8~24.0초로 초과했다. 이 병목은 camera source(7.3), preset renderer(7.4~7.6) 소유다.

**0.017초를 증명하지 못해 4초짜리 문제의 해결이 막혀 있다.** 순서가 뒤바뀌었다.

## 결정

**HV-13B의 물리 프레임 증거 항목을 Story 7.8 / HV-18B로 이관한다.**

HV-13B는 자기가 실제로 증명한 범위로 재정의하고 그 범위에서 `Go`를 기록한다. 이관된 항목은
삭제되지 않으며, HV-18B의 required evidence로 이동해 그대로 남는다.

| 항목 | 이전 소유 | 이후 소유 |
| --- | --- | --- |
| immutable generation 무결성, pointer commit 순서, decode/크기/correlation 검증 | HV-13B | HV-13B (유지) |
| one-clock actual-present timing span, 계측 완결성 | HV-13B | HV-13B (유지) |
| visible-standby vs hidden-prewarm A/B 결정 | HV-13B | HV-13B (유지) |
| AC 4 실패 조건 (입력 이후 창 생성/navigation) | HV-13B | HV-13B (유지) |
| **120fps+ 물리 모니터 프레임 촬영** | HV-13B | **HV-18B** |
| **compositor → photon 오프셋 측정** | HV-13B | **HV-18B** |
| **frame-level zero-transition-defect 판정** | HV-13B | **HV-18B** |

### 왜 7.10이 아니라 7.8인가

- **7.10 / HV-18D는 증거를 집계하는 판정이지 생산하는 게이트가 아니다.** 거기에 rig 요구를 두면
  최종 출시 판정 직전까지 아무도 실행하지 않고, 그때 실패하면 되돌릴 여지가 없다.
- **7.8 / HV-18B는 이미 같은 성질의 주장을 소유한다.** 현재 AC가 100회 측정에서
  `blank, stale, upscaled, crop/scale jump, tier downgrade = 0`을 요구한다. 이것이 곧
  frame-level zero-transition-defect이며, 물리 프레임 촬영이 그 유일한 판정 수단이다.
- **7.8은 7.4/7.6 이후에 실행된다.** 그 시점에는 fixture가 아니라 **실제 preset 적용 사진과
  RAW 정밀본 교체**가 화면에 올라간다. 물리 프레임으로 검증할 가치가 있는 대상은 그쪽이다.
  지금 rig를 구해 계측용 fixture의 A→B 전환을 찍는 것은 출시 경험을 증명하지 않는다.
- rig 확보를 한 번만 하면 되므로 준비 비용도 한 번이다.

## 위험과 방어선

- **위험:** 표시 종단점의 물리적 결함이 7.8까지 발견되지 않을 수 있다.
- **방어선 1:** HV-13B row는 `Go`이되 **무엇을 증명하지 않았는지**를 row 안에 명시한다.
  "물리 프레임 미검증"이 ledger에서 계속 보인다.
- **방어선 2:** HV-18B가 표시 종단점 귀책의 전환 결함을 발견하면 **Story 7.2를 `review`로
  되돌린다.** 이 재개 조건을 HV-13B row와 Story 7.2에 명시한다. 기존 ledger 정책
  ("hardware evidence가 No-Go면 story는 review로 돌아간다")의 적용이다.
- **방어선 3:** Story 7.2가 만든 계측 완결성 게이트
  (`check-telemetry-completeness.ps1`)는 7.4/7.6/7.8 회차에서 계속 실행된다. 표시 종단점의
  회귀는 물리 프레임 없이도 계측 누락/`present-unreported` 행으로 먼저 드러난다.
- **방어선 4:** Epic 7과 MVP release는 여전히 HV-18D Go 없이는 `done`이 될 수 없다.
  이관은 최종 출시 기준을 낮추지 않는다.

## 적용 대상 문서

- `_bmad-output/planning-artifacts/epics.md` — Epic 7 execution rule, Story 7.2 closure AC, Story 7.8 AC
- `_bmad-output/implementation-artifacts/hardware-validation-ledger.md` — policy summary, canonical table, HV-13B row, HV-18B row
- `_bmad-output/implementation-artifacts/7-2-immutable-sample과-actual-present-계측.md` — Correct Course Note, T6/T8, Status
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — 7.2 close state, 7.3 착수
- `tests/hardware/viewer-present/hv-13b/README.md` — 물리 프레임 섹션의 이관 표기

## 승인

- 2026-08-12 Noah Lee 승인. 근거: "0.017초 때문에 4초짜리 문제 해결이 막혀 있는 게 순서가 뒤바뀐 것"이며,
  소프트웨어 기준선으로 7.3~7.6을 진행하고 실제 사진이 뜨는 7.4 이후에 물리 검증을 한 번에 수행한다.
