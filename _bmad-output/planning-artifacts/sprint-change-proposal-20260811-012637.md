---
workflow: correct-course
project: Boothy
date: 2026-08-11 01:26:37 +09:00
user: Noah Lee
communication_language: Korean
document_output_language: Korean
user_skill_level: intermediate
mode: incremental
approval_status: approved
approved_at: 2026-08-11 01:41:49 +09:00
approval_decision: yes
trigger_reference: _bmad-output/planning-artifacts/research/technical-preset-image-fast-display-research-2026-08-10.md
individual_proposals_approved:
  - PRD
  - UX
  - Architecture
  - Epic
  - Sprint
---

# Sprint Change Proposal - 저지연 프리셋 관람 경험

## 0. 워크플로 프레이밍

- 변경 트리거는 `technical-preset-image-fast-display-research-2026-08-10.md`의 종합 결론이다.
- 검토 방식은 점진적 검토이며 PRD, UX, Architecture, Epic, Sprint 변경안은 각각 사용자 승인을 받았다.
- 이번 변경은 문서 보정만이 아니다. 승인 범위의 모든 Story는 구현, 관련 문서와 계약 수정, 자동 검증, 실장비 검증을 포함한다.
- Story 7.1에서는 렌더러를 교체하지 않는다. 촬영 전에 준비되는 viewer, 화면 크기 계약, 버튼부터 실제 화면 표시까지의 계측, immutable 이미지의 무중단 교체 기반을 먼저 증명한다.
- 필수 검토 문서인 PRD, Epics, Architecture, UX Design을 모두 확인했다. `project-context.md`와 `docs/index.md`는 존재하지 않았다.

## 1. Issue Summary

### 문제 진술

현재 Boothy는 사용자가 요구한 제품 경험인 **촬영 전에 준비된 전용 관람 창에 화면 크기의 프리셋 적용 사진이 빠르게 표시되는 경험**을 충족하지 못한다.

현재 `main`은 정확한 RAW 기반 렌더와 truthful waiting 기반을 제공하지만 다음 한계가 있다.

- 전용 관람 창이 없고 조작 화면의 작은 사진 레일이 주요 확인 수단이다.
- 프리셋 적용 프리뷰는 RAW 전송 뒤 매 촬영마다 새 darktable 프로세스를 거친다.
- 빠른 same-capture 이미지 경로인 Story 1.9는 실장비에서 고객 가시 결과를 만들지 못했다.
- 파일 준비나 `<img>` 로드와 실제 모니터 표시가 구분되지 않는다.
- 빠른 이미지와 정식 이미지를 같은 경로에 덮어써 cache, stale result, 부분 파일 노출 위험이 있다.
- 현재 설치·CI는 실제 촬영과 렌더에 필요한 전체 런타임을 하나의 재현 가능한 설치 단위로 증명하지 않는다.

### 발견 맥락과 증거

- 현재 `main`의 RAW preset render는 약 `4.84~5.77초`가 관측됐다.
- EOS 700D RAW 전송은 약 `1.38~1.49초`였다.
- direct camera thumbnail은 조사 표본에서 `0/2`였고, Story 1.9의 최신 실장비 증거도 fast preview 생성에 실패했다.
- CR2 두 샘플에는 `5184×3456` embedded JPEG가 있었고 LibRaw 추출은 cold 약 `50ms`, warm 약 `12ms`였다. 다만 표본 부족으로 production proof는 아니다.
- 1920px darktable one-shot은 CPU 약 `3.53초`, OpenCL 약 `4.99초`였지만 실제 pixelpipe 계산은 약 `0.3초`였다. process와 engine 초기화 비용이 유력한 병목이다.
- 과거 Story 1.26/1.9의 `2.4~2.9초` 기록은 button→full-view monitor paint와 동일한 지표가 아니므로 출시 근거로 사용할 수 없다.
- Lightroom Classic `3~4초`는 Adobe 공식 SLA가 아니라 사용자 관측 기반 비교 목표다.

## 2. Change Analysis Checklist

### 2.1 Trigger and Context

- [x] 1.1 Trigger identified
  - 직접 트리거: 2026-08-10~11 종합 기술 조사
  - 관련 Story: 1.8과 1.9
- [x] 1.2 Core problem defined
  - 유형: 구현 중 확인된 기술 제약 + 제품 경험 기준 불일치
  - 핵심: 현재 프리뷰 경로와 측정 기준이 full-view preset-applied experience를 증명하지 못한다.
- [x] 1.3 Evidence gathered
  - 현재 코드·실장비·과거 branch·공식 기술 문서 기반의 정량·정성 근거가 확보됐다.

### 2.2 Epic Impact Assessment

- [x] 2.1 Current epic viability assessed
  - Epic 1은 기존 capture truth와 RAW fallback 책임을 유지할 수 있다.
  - 그러나 새로운 제품 목표를 Epic 1에 계속 덧붙이는 방식은 범위와 완료 기준을 왜곡한다.
- [x] 2.2 Epic-level change identified
  - 신규 `Epic 7: 저지연 프리셋 관람 경험의 검증과 출시`를 추가한다.
- [x] 2.3 Remaining epics reviewed
  - Epic 2의 사진 레일은 현재 세션 검토 수단으로 유지한다.
  - Epic 3의 final truth와 completion 경계는 유지한다.
  - Epic 4의 게시 artifact에는 proxy 호환성과 검증 metadata가 필요하다.
  - Epic 5의 진단에는 source/render/publish/present 구간 분리가 필요하다.
  - Epic 6의 출시 gate에는 완전한 installer, 100-shot, canary, rollback 증거가 필요하다.
- [x] 2.4 Future epic invalidation checked
  - 폐기되는 기존 Epic은 없다.
  - 기존 완료 Story는 되돌리지 않지만 새 MVP 출시 증거로 자동 인정하지 않는다.
- [x] 2.5 Epic order and priority reviewed
  - Story 7.1부터 7.6까지 증거 기반 순차 실행을 강제한다.

### 2.3 Artifact Conflict and Impact Analysis

- [x] 3.1 PRD conflict reviewed
  - 기존 NFR-003은 `RAW 저장 후 p95 5초` 중심이어서 실제 고객 경험의 시작점과 종료점이 다르다.
  - 전용 관람 창과 qualifying preset-applied frame을 제품 요구로 추가해야 한다.
- [x] 3.2 Architecture conflict reviewed
  - 기존 same-path replacement, 동기 capture, 일반 event, one-shot render 중심 경로는 목표와 충돌한다.
  - viewer, async capture, ordered progress, immutable generation, source adapter, resident renderer spike, deadline scheduler, 실제 present 계측이 필요하다.
  - 현재 구현 사실에 맞춰 filesystem manifest/journal을 기본 진실로 정리하고 SQLite는 후속 index로 제한한다.
- [x] 3.3 UX conflict reviewed
  - 현재 UX는 최신 사진 레일을 중심으로 하며 전용 관람 화면의 readiness, physical fit, swap contract가 없다.
  - read-only viewer surface와 무중단 단계 전환 규칙이 필요하다.
- [x] 3.4 Other artifacts reviewed
  - session/preset/render/camera 계약
  - 성능 계측과 hardware validation ledger
  - 설치·서명·clean VM·CI
  - 운영 진단과 canary/rollback runbook

### 2.4 Path Forward Evaluation

- [x] 4.1 Direct Adjustment
  - 판정: 부분적으로 viable하지만 단독으로 부족
  - Effort: High
  - Risk: High
  - 기존 truth/fallback을 유지하고 새 Epic에서 target path를 추가하는 방식은 필요하다.
- [x] 4.2 Potential Rollback
  - 판정: Not viable
  - Effort: High
  - Risk: High
  - Story 1.8의 정확한 RAW truth를 되돌리면 false-ready와 false-complete 위험이 재발한다.
- [x] 4.3 PRD MVP Review
  - 판정: Viable and required
  - Effort: Medium
  - Risk: Medium
  - MVP 성공 지표와 출시 gate를 실제 관람 화면 기준으로 바꿔야 한다.
- [x] 4.4 Recommended path selected
  - 선택: Hybrid - `MVP Review + Direct Adjustment + 신규 Epic 7`
  - 변경 등급: Major
  - 이유: 제품 목표는 유지되지만 고객 화면, KPI, 렌더 critical path, 배포와 Sprint 우선순위를 함께 재설계해야 한다.

### 2.5 Proposal and Handoff Readiness

- [x] 5.1 Issue summary prepared
- [x] 5.2 Epic and artifact impacts documented
- [x] 5.3 Recommended path and alternatives documented
- [x] 5.4 MVP impact and high-level action plan documented
- [x] 5.5 Handoff roles defined
- [x] 6.1 Applicable checklist sections reviewed
- [x] 6.2 Proposal consistency reviewed
- [x] 6.3 Complete proposal final approval obtained
- [x] 6.4 Sprint status updated with Epic 7 and Story 7.1 ready-for-dev
- [x] 6.5 Major-change implementation handoff recorded

## 3. Impact Analysis

### Epic Impact

- Epic 1
  - Story 1.8은 정확한 RAW preview/final/fallback 경로의 소유자로 유지한다.
  - Story 1.9는 빠른 source와 진행 피드백의 참고 구현으로 유지한다.
  - 두 Story의 기존 No-Go 상태는 Epic 7 검증 전까지 유지한다.
- Epic 2
  - 사진 레일은 현재 세션 검토 기능으로 유지한다.
  - 사진 레일 표시를 full-view 출시 성능 성공으로 계산하지 않는다.
- Epic 3
  - RAW final과 truthful completion 경계를 유지한다.
  - background final 작업이 first-visible display를 막지 않도록 우선순위만 정렬한다.
- Epic 4
  - 게시 preset이 proxy 호환성, versioned recipe, reference renderer, visual approval evidence를 제공하도록 확장한다.
- Epic 5
  - source ready, render, publish, viewer receive, decode, present, RAW swap을 분리 진단한다.
- Epic 6
  - signed complete installer, clean offline VM, 100-shot, cold/idle/reconnect, canary/rollback gate를 추가한다.
- Epic 7
  - 새 관람 경험을 구현부터 출시 증거까지 순차적으로 소유한다.

### Story Impact

- 기존 완료 Story는 되돌리지 않는다.
- Story 1.8, 1.9는 `review`를 유지하며 Epic 7의 성공을 대신하지 않는다.
- Story 7.1만 최초 착수 가능 상태로 둔다.
- Story 7.2~7.6은 바로 앞 Story의 증거를 통과해야 활성화한다.
- 모든 Epic 7 Story는 구현·문서/계약 수정·자동화·실장비 검증을 완료 조건으로 가진다.

### Product and MVP Impact

- MVP의 핵심 고객 약속에 전용 관람 화면이 추가된다.
- 공식 성능 지표가 RAW 저장 이후 프리뷰 준비에서 button→monitor present로 변경된다.
- `cameraSource`는 진행/내부 입력일 뿐 고객 성공 화면으로 세지 않는다.
- 첫 성공 화면은 `displayFitPresetProxy` 또는 이를 건너뛴 `rawRefinedDisplay`여야 한다.
- Epic 7 Story 7.6을 통과하기 전까지 새로운 MVP 출시 상태는 No-Go다.

### Technical and Operational Impact

- Tauri multi-window와 viewer lifecycle
- capture request correlation과 ordered progress
- immutable display artifacts와 revisioned manifest
- LibRaw와 RAW+JPEG source 후보 검증
- display-fit proxy publication과 renderer spike
- RAW refinement와 deadline scheduler
- QPC/DXGI actual-present measurement
- complete installer, signing, clean VM, hardware runner, canary/rollback

## 4. Recommended Approach

### Chosen Path

`Hybrid: MVP Review + Direct Adjustment + 신규 Epic 7`

### Rationale

- 현재 RAW 기반 경로는 정확성 fallback으로 가치가 있으므로 폐기하지 않는다.
- 새로운 고객 경험은 기존 사진 레일 개선만으로 충족할 수 없다.
- renderer를 먼저 교체하면 실제 표시 기준과 화면 계약이 없어 성능 개선을 잘못 판단할 위험이 있다.
- viewer와 actual-present KPI를 먼저 닫으면 이후 source, proxy, renderer, swap 개선을 같은 기준으로 비교할 수 있다.
- 모든 후보 기술은 shadow 또는 spike로 검증한 뒤 production 경로로 승격한다.

### Effort and Timeline

| Story | 예상 |
| --- | ---: |
| 7.1 전용 관람 창과 실제 화면 표시 계측 | 2~4일 |
| 7.2 빠른 JPEG source 비교 | 1~6일 |
| 7.3 화면 적합 immutable preset proxy | 2~4일 |
| 7.4 상주 renderer 검증 spike | 3~6일 |
| 7.5 RAW 정밀본 무중단 교체 | 2~4일 |
| 7.6 완전한 installer와 100-shot 출시 gate | 2~3일 + soak |

- 총 범위: 약 `12~27 엔지니어링 일 + soak`
- production-like canary 목표: 약 `2~3주`
- resident renderer가 목표에 실패하고 전체 preset 호환 엔진 연구가 필요하면 별도 `2~6주` 재계획이 필요하다.

### Risk Assessment

| 위험 | 영향 | 대응 |
| --- | --- | --- |
| 빠른 JPEG가 없거나 늦음 | 첫 화면 SLA 미달 | LibRaw와 RAW+JPEG를 비교하고 승인된 fallback 유지 |
| proxy와 RAW look 차이 | 고객 신뢰 하락 | preset별 compatibility와 자동·blind 화질 gate |
| viewer가 준비되지 않음 | 첫 촬영 미표시 | pre-capture readiness handshake와 snapshot 복구 |
| stale result가 최신 화면을 덮음 | 잘못된 고객 사진 노출 | immutable generation, full correlation, latest-wins cancellation |
| swap 중 blank/flicker | 프리미엄 경험 훼손 | 완전 decode 후 double-buffer 교체와 frame-level 검증 |
| installer가 일부 런타임 누락 | 현장 재현 실패 | signed inventory와 clean offline VM gate |
| 파일-ready 지표 오용 | 잘못된 Go 판단 | QPC/DXGI actual-present를 공식 KPI로 고정 |

## 5. Detailed Change Proposals

### 5.1 PRD

#### KPI Table / NFR-003

OLD:

- RAW source persistence 이후 preset-applied preview를 p95 5초 이내 표시한다.

NEW:

- 공식 E2E는 `trusted capture input → pre-opened viewer의 첫 qualifying preset-applied monitor present`다.
- warm p50≤3초, p95≤4초, hard max≤5초를 적용한다.
- release 100-shot 성공률은 99% 이상이어야 하며 실패와 timeout을 percentile에서 제외하지 않는다.
- wrong session/request/capture/preset/version, unfiltered qualifying frame, blank, stale image, tier downgrade, upscale는 0건이어야 한다.

Rationale:

- 기존 지표는 고객이 실제로 보는 시점을 측정하지 않는다.

#### FR-004 / 신규 FR-010

OLD:

- 저장 성공, Preview Waiting, preset-applied preview readiness를 구분한다.

NEW:

- 기존 truth separation을 유지한다.
- 신규 FR-010에서 촬영 전에 준비된 전용 관람 화면, 화면 적합 프리셋 적용 첫 화면, 무중단 RAW 정밀본 교체를 제품 요구로 추가한다.
- 무보정 camera JPEG와 thumbnail rail은 qualifying 성공 화면으로 인정하지 않는다.

Rationale:

- 새로운 경험은 단순 NFR 보강이 아니라 고객이 사용하는 별도 제품 surface다.

#### Published Preset Artifact / Release Gates

OLD:

- published preset은 booth-safe preview와 final behavior를 제공한다.

NEW:

- preset은 `proxyCompatible`, supported operation set, versioned compiled recipe, reference renderer/version, visual approval evidence를 포함한다.
- signed complete installer, clean VM, approved EOS 700D/display evidence 없이는 release하지 않는다.

Rationale:

- 빠른 proxy는 preset별 화질 승인을 제품 publication 경계에서 보장해야 한다.

### 5.2 UX Design

#### Customer Viewing Surface

OLD:

- 최신 사진 레일이 촬영 확인의 중심이다.

NEW:

- 세션 시작 시 대상 모니터에 read-only viewer surface를 미리 준비한다.
- viewer는 촬영 전에 visible, layout-ready, listener-ready 상태를 증명한다.
- 사진 레일은 현재 세션 검토용으로 유지한다.

Rationale:

- 고객이 실제로 보는 full-view surface와 조작/검토 surface의 역할을 분리해야 한다.

#### Progressive Display and Swap

OLD:

- `Preview Waiting` 중 pending image를 사진 레일에 먼저 표시할 수 있다.

NEW:

- viewer는 standby → preset proxy → RAW refined display 순서로 전환한다.
- 다음 이미지가 완전히 decode되고 qualifying 조건을 만족할 때만 double-buffer로 교체한다.
- blank, spinner, 이전 촬영, 잘못된 preset, crop/scale jump, tier downgrade를 허용하지 않는다.

Rationale:

- 빠른 표시와 truthful quality progression을 동시에 지켜야 한다.

#### Story 7.1 UX Boundary

NEW:

- Story 7.1은 renderer를 교체하지 않는다.
- immutable sample asset으로 viewer readiness, display fit, double-buffer, actual paint evidence만 닫는다.
- visible standby와 hidden prewarm을 동일 장비에서 비교해 기본 동작을 정한다.

Rationale:

- 화면과 계측 기반이 없으면 이후 renderer 개선을 제품 관점에서 판단할 수 없다.

### 5.3 Architecture

#### Viewer and Capture Communication

OLD:

- booth route와 사진 레일, 동기 capture command, generic event 중심이다.

NEW:

- app-lifetime viewer window와 read-only snapshot을 추가한다.
- capture는 즉시 request ID를 반환하고 background에서 진행한다.
- ordered progress와 monotonic snapshot을 함께 사용한다.

Rationale:

- 긴 촬영/렌더 작업이 viewer 준비와 화면 표시를 막지 않아야 한다.

#### Immutable Display Artifacts

OLD:

- fast preview와 rendered preview가 같은 canonical path를 덮어쓴다.

NEW:

- `cameraSource`, `displayFitPresetProxy`, `rawRefinedDisplay`, `final`을 별도 immutable generation으로 저장한다.
- write→close→decode/dimension validation→atomic publish→manifest pointer commit→notification 순서를 고정한다.

Rationale:

- WebView cache, stale worker, partial file, wrong-generation 문제를 구조적으로 차단한다.

#### Source, Renderer, Scheduler

OLD:

- RAW 도착 뒤 darktable one-shot이 preview/final을 만든다.

NEW:

- LibRaw embedded JPEG와 capability-gated RAW+JPEG를 비교한다.
- darktable은 RAW refined/final/parity oracle/fallback으로 유지한다.
- 상주 display renderer는 구현 spike와 화질 gate 후에만 승격한다.
- 우선순위를 P0 current preset proxy, P1 RAW refine, P2 final/warmup으로 고정한다.

Rationale:

- source 확보와 renderer lifetime을 분리해 가장 큰 지연 원인을 각각 검증해야 한다.

#### Measurement, Storage, Deployment

OLD:

- file/onLoad 중심 timing, SQLite audit 가정, shell 중심 packaging이다.

NEW:

- QPC 기반 button time과 DXGI actual-present를 공식 KPI로 사용한다.
- session manifest와 versioned JSONL journal을 현재 MVP의 durable truth로 정리하고 SQLite는 필요 시 후속 index로 제한한다.
- app/helper/EDSDK/renderer/profile/recipe/darktable을 하나의 signed inventory로 배포한다.

Rationale:

- 현재 구현 사실과 출시 재현성을 아키텍처 문서에 일치시켜야 한다.

### 5.4 Epic and Stories

#### 신규 Epic 7

`Epic 7: 저지연 프리셋 관람 경험의 검증과 출시`

##### Story 7.1: 촬영 전 전용 관람 창과 실제 화면 표시 계측

- renderer를 교체하지 않는다.
- viewer pre-create/readiness, viewport×DPR 계약, immutable sample display, double-buffer swap, button→monitor paint evidence를 구현·검증한다.

##### Story 7.2: LibRaw embedded JPEG와 RAW+JPEG source 비교

- 실제 EOS 700D 30회 이상에서 속도, 존재율, orientation, decode, corruption, 화질, request correlation을 비교한다.
- 승인된 winner와 fallback을 기록한다.

##### Story 7.3: 화면 적합 immutable preset proxy

- viewport profile에 맞는 preset-applied proxy를 unique generation으로 생성·검증·게시한다.
- same capture/preset/version만 qualifying display로 승격한다.

##### Story 7.4: 상주 renderer 검증 spike

- 승인 preset 3개를 지원하는 실제 prototype을 구현한다.
- one-shot darktable 기준선과 latency, color parity, 안정성, unsupported fallback을 비교한다.
- Go/No-Go 결과를 남기며 No-Go 후보는 제품 경로로 승격하지 않는다.

##### Story 7.5: RAW 정밀본 무중단 교체

- RAW refined display를 완전 decode 후 교체한다.
- blank/flicker/crop-scale jump/tier downgrade/stale overwrite가 0임을 frame-level evidence로 증명한다.

##### Story 7.6: 완전한 installer와 100-shot 출시 gate

- clean offline Windows 환경에 전체 runtime을 설치한다.
- warm 100-shot, cold 5회, 10분 idle/reconnect 10회, quality, privacy, rollback evidence를 수집한다.
- lab→pilot→10~20%→default canary와 즉시 forced fallback을 검증한다.

#### Existing Story Notes

- Story 1.8: exact RAW preview/final/fallback responsibility 유지
- Story 1.9: camera source/progress feedback 참고 구현으로 유지, release performance proof에서는 제외
- 기존 완료 Story: rollback 없음, 새 MVP release proof로 자동 인정하지 않음

### 5.5 Sprint Tracking

OLD:

- Epic 1~6만 존재하며 viewer와 actual-present workstream이 없다.

NEW:

```yaml
epic-7: in-progress
7-1-전용-관람-창과-실제-화면-표시-계측: ready-for-dev
7-2-libraw-embedded-jpeg와-raw-jpeg-source-비교: backlog
7-3-화면-적합-immutable-preset-proxy: backlog
7-4-상주-renderer-검증-spike: backlog
7-5-raw-정밀본-무중단-교체: backlog
7-6-완전한-installer와-100-shot-출시-gate: backlog
```

Rationale:

- Story 7.1의 actual-present 기반이 모든 후속 성능 판단의 전제다.
- 순차 gate는 잘못된 KPI나 검증되지 않은 기술을 production 기본 경로로 승격하는 것을 막는다.

## 6. Implementation Handoff

### Scope Classification

`Major`

### Handoff Recipients

- Product Manager / Product Owner
  - PRD MVP 기준과 출시 No-Go/Go 경계 소유
- Solution Architect
  - viewer, immutable tier, source/renderer adapter, scheduler, packaging 계약 소유
- Scrum Master
  - Epic 7 순차 gate와 sprint-status 운영
- Development Team
  - Story 7.1부터 순서대로 구현하고 각 Story의 문서·계약·자동화까지 완료
- QA / Release / Operations
  - DXGI actual-present, 화질 parity, 30/100-shot, clean VM, canary/rollback evidence 소유
- Preset / Imaging Owner
  - proxy compatibility와 visual approval 판정

### Success Criteria

- 촬영 전에 viewer가 준비돼 있다.
- 첫 qualifying 화면은 화면 크기에 적합한 같은 촬영·같은 프리셋의 결과다.
- warm p50≤3초, p95≤4초, hard max≤5초다.
- 100-shot 성공률은 99% 이상이다.
- wrong capture/preset/session, blank, stale frame, crop/scale jump, tier downgrade는 0건이다.
- RAW 정밀본 교체가 고객에게 거슬리지 않는다.
- 전체 runtime이 signed installer 하나로 clean offline Windows에서 재현된다.
- 승인된 canary와 rollback evidence가 있다.

## 7. Final Approval and Application Log

개별 PRD, UX, Architecture, Epic, Sprint 변경안과 통합 Sprint Change Proposal이 모두 승인됐다.

- Final approval status: `approved`
- Approval decision: `yes`
- Approved at: `2026-08-11 01:41:49 +09:00`
- Change scope: `Major`

Applied artifacts:

- `prd.md`: pre-opened viewer, FR-010, actual-present NFR/KPI, proxy compatibility, complete installer and 100-shot release gates
- `ux-design-specification.md`: dedicated viewer surface, physical display fit, double-buffer transition, actual-present testing
- `architecture.md`: authoritative correct-course baseline, immutable generations, async/ordered state, source and renderer boundaries, release inventory
- `epics.md`: Epic 7 and Story 7.1~7.6, mandatory implementation/documentation/automation/hardware gates
- `7-1-전용-관람-창과-실제-화면-표시-계측.md`: first ready-for-dev implementation story
- `sprint-status.yaml`: Epic 7 in progress, Story 7.1 ready-for-dev, Story 7.2~7.6 backlog
- `hardware-validation-ledger.md`: HV-13~HV-18 canonical No-Go rows and evidence requirements

Implementation handoff:

- Product Manager / Architect: Major change governance and contract review
- Scrum Master: sequential Epic 7 activation and status control
- Development team: begin with Story 7.1 and do not replace the renderer in that Story
- QA / Release / Operations: actual monitor-present, source comparison, parity, seamless swap, 100-shot, clean installer, canary, and rollback evidence

The product release remains No-Go until Story 7.6 and HV-18 record Go.

## 8. Workflow Completion

- Issue addressed: 현재 `main`이 전용 관람 화면, actual-present KPI, physical display-fit preset proxy, seamless RAW upgrade, complete installer evidence 없이 file/thumbnail 중심 프리뷰를 제공하는 문제
- Change scope: `Major`
- Artifacts modified: PRD, UX Design, Architecture, Epics, Story 7.1, Sprint Status, Hardware Validation Ledger, Sprint Change Proposal
- Routed to: Product Manager / Architect / Scrum Master / Development / QA / Release / Operations / Imaging owner
- First implementation item: Story 7.1 - renderer 교체 없는 pre-opened viewer와 actual-present measurement
- Release position: `No-Go` until Story 7.6 and HV-18 record Go

Verification completed:

- Artifact cross-reference checks: pass
- Epic 7 Story 7.1~7.6 sequence and Sprint status alignment: pass
- HV-13~HV-18 gate alignment: pass
- `git diff --check`: pass
- `pnpm vitest run src/governance/hardware-validation-governance.test.ts`: 8/8 pass

Correct Course workflow complete, Noah Lee!
