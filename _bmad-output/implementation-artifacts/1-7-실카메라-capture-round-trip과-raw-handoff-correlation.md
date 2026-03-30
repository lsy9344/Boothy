# Story 1.7: 실카메라 capture round-trip과 RAW handoff correlation

Status: done

Correct Course Note: Story 1.6은 `canon-helper.exe` baseline, host spawn/health, `helper-ready`와 `camera-status` 기반 readiness truth까지만 닫는다. Story 1.7은 실제 `request-capture -> capture-accepted -> file-arrived -> session persistence` round-trip을 닫는 별도 story이며, placeholder capture flow나 Story 1.6 상태를 근거로 완료 처리하면 안 된다. 현재 제품 기준 supported success path는 booth 앱의 `사진 찍기` 버튼이 시작한 host-owned `request-capture` 경로이며, 카메라 본체 셔터 직접 입력은 이 story의 성공 경로나 closure evidence로 간주하지 않는다.

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

booth customer로서,
실제 촬영이 내 세션으로 올바른 파일이 돌아왔을 때만 끝났다고 믿고 싶다.
그래서 셔터 수락이나 중간 handoff를 저장 성공으로 오해하지 않는다.

## Acceptance Criteria

1. 승인된 booth hardware의 fresh `Ready` 상태에서 고객이 booth 앱의 `사진 찍기` 버튼으로 촬영을 시작해 host가 bundled helper로 `request-capture`를 보낼 때, helper는 한 번에 하나의 correlated in-flight capture만 accept 또는 reject해야 한다. host도 같은 요청에 대해 단일 in-flight guard를 유지해야 하며, second capture나 stale request가 parallel로 열리면 안 된다.
2. `capture-accepted`는 capture success가 아니다. host는 correlated `file-arrived`와 실제 파일 존재를 active session root 아래에서 함께 확인한 뒤에만 `capture-saved`를 확정해야 한다. 그때 생성되는 capture record는 최소 `sessionId`, `requestId`, `captureId`, `activePresetVersion`, RAW asset path를 포함해야 하고 `session.json`과 shared contract가 같은 의미를 유지해야 한다.
3. 카메라 본체 셔터 직접 입력처럼 host의 `request-capture` 없이 발생한 out-of-band 촬영은 현재 제품의 supported booth capture success path가 아니다. 이런 입력은 active session 성공, `Preview Waiting`, `Preview Ready`, story closure evidence로 자동 승격되면 안 되며, 현장 검증에서도 앱의 `사진 찍기` 버튼 경로와 구분해 기록해야 한다.
4. duplicate arrival, wrong session correlation, wrong request correlation, missing file, partial file, timeout, helper/session mismatch, 또는 in-flight 중 second capture 시도는 false success를 막아야 한다. booth는 unsafe parallel capture를 허용하지 않고, customer-safe wait 또는 recovery guidance로만 내려가야 하며 cross-session leakage가 생기면 안 된다.
5. preview/render 후속 처리와 customer-facing `Preview Waiting`/`Preview Ready` 흐름은 실제 RAW persistence 뒤에만 시작돼야 한다. helper가 셔터를 수락했거나 내부적으로 다운로드 중이라는 사실만으로 preview thread, success copy, latest-photo confirmation이 먼저 열리면 안 된다.
6. Story 1.7은 approved booth hardware에서 real capture round-trip evidence가 확보되기 전까지 닫히면 안 된다. closure evidence는 최소 실제 `request-capture`, `session.json` capture record, `captures/originals/` RAW, helper/correlation 근거, 그리고 `Preview Waiting`이 persistence 뒤에 truthful하게 이어지는 supporting proof를 포함해야 한다. 카메라 본체 셔터 직접 입력만으로 생긴 결과는 이 closure evidence에 포함하지 않는다.

## Tasks / Subtasks

- [x] helper capture round-trip contract와 correlation ownership을 닫는다. (AC: 1, 2, 3, 4)
  - [x] `src-tauri/src/capture/sidecar_client.rs`가 `capture-accepted`, `file-arrived`, `recovery-status`, `helper-error`를 실제 round-trip에서 읽고 검증할 수 있게 확장한다.
  - [x] `requestId`와 `captureId`의 소유 주체를 host/helper 경계에서 명확히 고정하고, `docs/contracts/camera-helper-sidecar-protocol.md`, Rust DTO, TypeScript schema가 같은 의미를 유지하게 맞춘다.
  - [x] helper와 host 모두 동시에 하나의 in-flight capture만 허용하도록 guard를 건다.
  - [x] 지원되는 촬영 트리거는 booth 앱의 `사진 찍기` 버튼이 시작한 host `request-capture`라는 점을 문서 경계에 고정하고, 카메라 본체 셔터 직접 입력은 supported success path로 간주하지 않는다고 명시한다.

- [x] placeholder RAW 저장 경로를 real helper handoff 기반 ingest로 교체한다. (AC: 1, 2, 3, 4)
  - [x] `src-tauri/src/capture/normalized_state.rs`와 `src-tauri/src/capture/ingest_pipeline.rs`에서 helper correlation이 닫히기 전 placeholder RAW를 만들어 `capture-saved`를 선언하는 흐름을 제거한다.
  - [x] RAW 파일은 `captures/originals/` 아래 active session root에만 저장하고, host는 `file-arrived` correlation과 실제 파일 존재를 둘 다 확인한 뒤 manifest를 쓴다.
  - [x] capture record는 기존 `session-capture/v1` shape를 유지하되, real helper round-trip 기준 `requestId`, `captureId`, preset binding, timing metric이 계속 채워지게 한다.

- [x] booth/customer truth를 in-flight round-trip에도 truthful하게 유지한다. (AC: 2, 3, 4)
  - [x] `src-tauri/src/commands/capture_commands.rs`는 `capture-accepted`만으로 preview completion thread를 시작하면 안 된다.
  - [x] 실제 round-trip이 현재 sync `request_capture` 반환 시점보다 길어지면, host-owned in-flight projection 또는 동등한 typed update path를 추가해 "action acknowledged"와 "capture saved"를 구분한다.
  - [x] booth surface에는 helper, SDK, USB, raw stderr/stdout 같은 내부 진단어를 노출하지 않는다.

- [x] mismatch / duplicate / timeout / recovery 경계를 잠근다. (AC: 1, 4, 5)
  - [x] wrong `sessionId`, wrong `requestId`, duplicate `file-arrived`, missing RAW, timeout, reconnect-before-file-close, helper restart mid-flight를 각각 bounded host error와 safe readiness로 정규화한다.
  - [x] once-ready 이후 helper/camera truth가 흔들리면 Story 1.6 규칙대로 false-ready를 막되, Story 1.7은 그 상태에서 false capture success도 함께 막아야 한다.
  - [x] render failure와 capture failure를 섞지 않고, RAW가 남아 있으면 render failure는 별도 isolation path로 유지한다.

- [x] 테스트와 hardware evidence를 준비한다. (AC: 1, 2, 3, 4, 5, 6)
  - [x] `src-tauri/tests/capture_readiness.rs` 또는 동등한 integration test에 happy path, accepted-but-no-file, duplicate arrival, wrong session/request correlation, timeout, in-flight second capture block을 추가한다.
  - [x] `src/shared-contracts/schemas/capture-readiness.ts`, `src/shared-contracts/schemas/session-capture.ts`, `src/capture-adapter/services/capture-runtime.ts` 관련 contract/service test가 schema 변경 후에도 same-session guard를 유지하는지 검증한다.
  - [x] 실장비 세션 `session_000000000018a138ef5c96c18c`에서 booth 앱의 `사진 찍기` 버튼 경로로 actual capture round-trip evidence 1건을 확보했다.
    - [x] `diagnostics/camera-helper-requests.jsonl`에 `request_000000000018a138f3793d7020` 기록 확인
    - [x] `diagnostics/camera-helper-events.jsonl`에서 같은 `requestId`로 `capture-accepted -> file-arrived` 순서와 `captureId` `capture_20260329053227617_23c7b14960` 확인
    - [x] `captures/originals/capture_20260329053227617_23c7b14960.CR2` 실제 파일 존재 확인
    - [x] `session.json` capture record에 같은 `requestId`, `captureId`, `raw.assetPath`, `activePresetVersion` 기록 확인
    - [x] `renders/previews/capture_20260329053227617_23c7b14960.jpg` 생성과 `preview.readyAtMs > raw.persistedAtMs` 확인
- [x] approved booth hardware에서 최소 HV-04 성격의 actual RAW persistence evidence와, HV-05 성격의 truthful `Preview Waiting -> Preview Ready` supporting evidence를 묶어 close package를 남긴다.
- [x] 이번 회차 evidence package에는 `Preview Waiting` 화면 검증과 timing supporting proof를 함께 남긴다.
- [x] 현장 evidence에는 앱의 `사진 찍기` 버튼 경로를 이번 story closure 근거로 고정하고, direct shutter는 후속 별도 story에서 다루는 범위라고 남긴다.

### Review Findings

- [x] [Review][Patch] direct shutter만 관찰된 HV-04/HV-05 회차는 `Fail`로 닫히도록 runbook과 결과 표를 정렬해야 함 [docs/runbooks/booth-hardware-validation-checklist.md:578]
- [x] [Review][Patch] HV-04/HV-05 소유권과 적용 범위가 Story 1.5/1.7 사이에서 충돌함 [_bmad-output/implementation-artifacts/1-7-실카메라-capture-round-trip과-raw-handoff-correlation.md:157]
- [x] [Review][Patch] Story 1.7 closure에 필요한 helper correlation 근거가 HV-04/HV-05 증거 목록에 반영되지 않음 [docs/runbooks/booth-hardware-validation-checklist.md:347]
- [x] [Review][Patch] AC 재번호 부여 뒤에도 mismatch/timeout task의 traceability 라벨이 이전 번호를 가리킴 [_bmad-output/implementation-artifacts/1-7-실카메라-capture-round-trip과-raw-handoff-correlation.md:42]

## Dev Notes

### 스토리 범위와 목적

- 이번 스토리는 helper readiness baseline이 아니라, 실제 촬영 요청이 올바른 세션 자산으로 닫히는 round-trip을 소유한다.
- 제품 관점의 핵심은 "셔터가 눌렸다"가 아니라 "내 세션에 source photo가 안전하게 저장됐다"는 사실이다.
- 따라서 Story 1.7은 success 판정 기준을 helper accept가 아니라 host-confirmed file persistence로 잠그는 것이 목적이다.

### 스토리 기반 요구사항

- Epic 1 Story 1.7은 `request-capture`, single in-flight guard, `capture-accepted != success`, correlated `file-arrived`, timeout/mismatch/duplicate 방어, real hardware evidence를 직접 요구한다. [Source: _bmad-output/planning-artifacts/epics.md#Story 1.7: 실카메라 capture round-trip과 RAW handoff correlation]
- PRD는 successful capture를 "active session 아래 source photo가 persisted 된 상태"로 정의하고, preview readiness와 completion truth를 별도 책임으로 분리한다. [Source: _bmad-output/planning-artifacts/prd.md#FR-004 Current-Session Capture Persistence and Truthful Preview Confidence]
- PRD responsiveness/NFR은 source-photo persistence 뒤 5초 내 preview confirmation을 요구하므로, preview pipeline은 RAW persistence 이후에만 시작돼야 한다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness]
- session isolation/NFR은 cross-session leak을 0으로 요구하므로, wrong-session file handoff는 success가 아니라 blocker다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-004 Session Isolation and Privacy]
- active session capture binding은 immutable published preset과 session-local `catalogSnapshot`을 기준으로 유지돼야 하며, publish/rollback이나 live catalog 변경이 이번 세션 capture record를 조용히 바꾸면 안 된다. [Source: docs/contracts/preset-bundle.md] [Source: docs/contracts/authoring-publication.md]

### 선행 의존성과 구현 순서

- 직접 선행 흐름은 Story 1.5의 truthful preview waiting, Story 1.6의 live readiness truth다.
- 안전한 구현 순서는 다음과 같다.
  - sidecar message handling과 in-flight guard부터 닫는다.
  - placeholder ingest를 helper-backed ingest로 교체한다.
  - preview/render kickoff 시점을 RAW persistence 뒤로 다시 잠근다.
  - 마지막에 mismatch/timeout/retry tests와 hardware evidence 수집 경로를 닫는다.

### 현재 워크스페이스 상태

- `src-tauri/src/capture/sidecar_client.rs`에는 이미 `CanonHelperCaptureAcceptedMessage`, `CanonHelperFileArrivedMessage`, `CanonHelperRecoveryStatusMessage` struct가 있지만, 실제 사용 경로는 아직 latest status snapshot read 중심이다.
- `src-tauri/src/capture/normalized_state.rs`의 `request_capture_in_dir(...)`는 현재 `persist_capture_in_dir(...)`가 끝나면 바로 `capture-saved`를 반환한다.
- `src-tauri/src/capture/ingest_pipeline.rs`의 `persist_capture_in_dir(...)`는 helper handoff 대신 placeholder RAW 파일(`{captureId}.jpg`)을 즉시 생성하고 manifest를 `preview-waiting`으로 전환한다.
- `src-tauri/src/commands/capture_commands.rs`는 `request_capture(...)` 직후 120ms sleep 기반으로 preview completion thread를 시작한다. 이 흐름은 real helper round-trip 전에는 유용한 placeholder지만, 1.7에서는 `capture-accepted`를 success처럼 오해하게 만들 수 있다.
- `src/shared-contracts/schemas/session-capture.ts`는 이미 `requestId`, `captureId`, active preset binding, RAW/preview/final asset path, timing metric을 포함하므로 session record shape 자체는 round-trip correlation을 담을 준비가 되어 있다.
- `src/shared-contracts/schemas/capture-readiness.ts`와 `src/capture-adapter/services/capture-runtime.ts`는 request/result를 typed하게 감싸고 same-session guard도 이미 갖고 있다. 따라서 1.7은 React가 아니라 host/contract 경계를 중심으로 바꾸는 편이 안전하다.

### 이전 스토리 인텔리전스

- Story 1.6은 readiness truth만 닫는 story로 재정의됐고, 실제 `request-capture`, `capture-accepted`, RAW download, `file-arrived`, in-flight capture guard는 명시적으로 Story 1.7 범위로 분리됐다. [Source: _bmad-output/implementation-artifacts/1-6-실카메라-helper-readiness-truth-연결과-false-ready-차단.md#스코프-분할-기준]
- Story 1.5와 관련 구현은 "저장 성공"과 `Preview Waiting`을 분리하는 UX/host contract를 이미 만들었다. 1.7은 이 분리를 깨지 말고, 그 앞단의 real RAW persistence를 truthful하게 바꾸는 작업이어야 한다. [Source: _bmad-output/implementation-artifacts/1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백.md]
- 최근 feature commit은 `d4a5c15` (`feat: implement story 1-5 truthful preview waiting flow`)이며, 이후 커밋은 generic chore snapshot 성격이 강하다. 즉, preview-waiting 제품 규칙은 이미 중요 패턴이지만 helper round-trip 구현 패턴은 아직 고정되지 않았다.

### 아키텍처 준수사항

- raw image bytes와 booth-safe derived files는 JSON IPC가 아니라 filesystem handoff로 이동해야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- host가 session/preset correlation, freshness, capture success 최종 확정을 소유하고, helper는 capture trigger/download/reconnect detection을 소유한다. [Source: docs/contracts/camera-helper-sidecar-protocol.md]
- React component는 helper를 직접 다루지 않고 typed adapter/service만 사용해야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#Frontend Architecture]
- `session.json`, `captures/originals/`, `renders/previews/`, `renders/finals/`, `handoff/`, `diagnostics/` 아래의 session-scoped structure를 깨면 안 된다. [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture] [Source: docs/contracts/session-manifest.md]
- capture correlation은 최소 `sessionId`, `captureId`, `requestId`, active preset version, file reference를 유지해야 한다. [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture]
- active session은 rollout, publish, rollback 중에도 이미 잠긴 baseline과 `catalogSnapshot`을 유지해야 한다. Story 1.7 구현은 capture 성공 판정 과정에서 branch/preset live truth를 다시 읽어 현재 세션 binding을 흔들면 안 된다. [Source: docs/contracts/preset-bundle.md] [Source: docs/contracts/branch-rollout.md] [Source: docs/release-baseline.md]

### helper / protocol 구현 기준선

- `helper-ready`는 boot 완료일 뿐 capture success와 무관하다.
- `request-capture`는 booth 앱의 `사진 찍기` 버튼이 시작한 supported success path를 뜻하며, 기본적으로 `sessionId`, `requestId`, active preset reference를 동반한다. `file-arrived`는 `sessionId`, `requestId`, `captureId`, `rawPath`를 포함해야 한다. [Source: docs/contracts/camera-helper-sidecar-protocol.md]
- helper는 한 번에 하나의 in-flight capture만 허용하는 보수적 경계를 기본값으로 본다. [Source: docs/contracts/camera-helper-sidecar-protocol.md] [Source: docs/contracts/camera-helper-edsdk-profile.md]
- `capture-accepted`는 success가 아니고, helper는 final path가 준비되고 file close가 끝난 뒤에만 `file-arrived`를 보내야 한다. host는 그 뒤에도 실제 파일 존재를 다시 확인해야 한다. [Source: docs/contracts/camera-helper-edsdk-profile.md]
- 카메라 본체 셔터 직접 입력은 현재 제품 문서 기준 supported booth capture path가 아니므로, active session success나 closure evidence로 자동 해석하면 안 된다.

### 프로젝트 구조 요구사항

- 우선 검토/수정 후보 경로:
  - `src-tauri/src/capture/sidecar_client.rs`
  - `src-tauri/src/capture/normalized_state.rs`
  - `src-tauri/src/capture/ingest_pipeline.rs`
  - `src-tauri/src/commands/capture_commands.rs`
  - `src-tauri/src/contracts/dto.rs`
  - `src-tauri/tests/capture_readiness.rs`
  - `src/shared-contracts/schemas/capture-readiness.ts`
  - `src/shared-contracts/schemas/session-capture.ts`
  - `src/shared-contracts/dto/capture.ts`
  - `src/capture-adapter/services/capture-runtime.ts`
  - `docs/contracts/camera-helper-sidecar-protocol.md`
  - `docs/contracts/camera-helper-edsdk-profile.md`
  - `docs/contracts/session-manifest.md`
- 새 파일이 꼭 필요하지 않다면 기존 `capture/`와 `shared-contracts/` 경계 안에서 닫는 편이 우선이다.

### UX 및 제품 가드레일

- 고객에게 중요한 문장은 "사진이 저장되었다"와 "확인용 사진을 준비 중"이다. `capture-accepted`, helper recovery, SDK detail은 고객 문구가 아니다. [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview Waiting 보호 흐름]
- real helper round-trip이 현재 placeholder보다 길어질 수 있어도, booth는 저장 성공 전 false-ready/false-complete를 보여주면 안 된다.
- `Preview Waiting`은 RAW persistence 이후에만 들어가야 한다. 저장이 아직 닫히지 않았는데 `Preview Waiting`을 띄우면 "이미 저장됐다"는 UX 약속을 깨게 된다.
- second capture를 막을 때도 고객에게는 technical diagnosis가 아니라 plain-language wait/call guidance만 보여야 한다.

### 테스트 요구사항

- 최소 필수 테스트:
  - helper가 `capture-accepted`를 보냈지만 `file-arrived`가 오지 않으면 success가 나오지 않는다.
  - wrong `sessionId` 또는 wrong `requestId`의 `file-arrived`는 무시 또는 error path로 떨어지고 현재 세션 success를 만들지 않는다.
  - duplicate `file-arrived`는 duplicate capture record를 만들지 않는다.
  - in-flight capture 중 second capture request는 block된다.
  - timeout 또는 helper restart mid-flight는 safe blocked/recovery path로 내려간다.
  - happy path에서는 실제 RAW file path와 `session.json` capture record가 일치한다.
  - preview thread 또는 render enqueue는 RAW persistence 이후에만 시작된다.
  - schema가 바뀌면 TS contract/service parsing과 same-session guard가 계속 통과한다.

### Hardware gate 및 운영 메모

- 실장비 close evidence는 단순 화면 확인이 아니라 truth transition 증명으로 남겨야 한다. 즉, `request-capture` 수락, RAW persistence, preview waiting, preview ready가 각각 언제 확정됐는지 구분되는 증거가 필요하다. [Source: docs/runbooks/booth-hardware-validation-architecture-research.md]
- Story split 이후 runbook 적용 범위는 Story 1.7을 명시적으로 포함하고, HV-04/HV-05는 아래처럼 해석한다.
  - HV-04: Story 1.7 primary closure evidence
  - HV-05: Story 1.7 supporting regression evidence이자 Story 1.5 truthfulness regression
- 따라서 Story 1.7은 최소 HV-04 성격의 actual RAW persistence evidence가 없으면 닫지 않는다. HV-05 성격의 `Preview Waiting -> Preview Ready` 증거는 supporting proof로 함께 남긴다.
- evidence package는 최소 `session.json`, RAW path, helper/camera freshness 또는 correlation 근거, capture 직후 화면, `Preview Waiting` 화면, preview file path를 연결해야 한다. 이때 supported closure path는 booth 앱의 `사진 찍기` 버튼 경로이며, 카메라 본체 셔터 직접 입력은 별도 관찰 메모로만 남긴다. [Source: docs/runbooks/booth-hardware-validation-checklist.md#HV-04-실제-촬영과-RAW-저장-확인] [Source: docs/runbooks/booth-hardware-validation-checklist.md#HV-05-Preview-Waiting---Preview-Ready-확인]

### 금지사항 / 안티패턴

- `capture-accepted`를 `capture-saved`처럼 UI나 manifest에 올리는 것 금지
- helper가 보낸 `rawPath` 문자열만 믿고 실제 파일 존재를 재검증하지 않는 것 금지
- placeholder `.jpg` RAW 생성 경로를 real helper handoff라고 포장하는 것 금지
- preview completion timer를 `capture-accepted` 직후 유지해 false success를 만드는 것 금지
- 카메라 본체 셔터 직접 입력 결과를 host `request-capture` 성공처럼 active session에 묶거나 HV closure evidence로 쓰는 것 금지
- React에서 helper correlation을 ad-hoc 해석하고 host truth를 우회하는 것 금지
- wrong-session file handoff를 "이번 세션 latest photo"로 읽는 것 금지
- customer 화면에 helper/SDK/USB/internal error vocabulary를 노출하는 것 금지

### 최신 기술 확인 메모

- 2026-03-28 기준 local research와 Canon official SDK pages를 다시 교차 확인한 결과, Canon 공개 SDK 목록은 여전히 `ED-SDK V13.20.10`을 제공하고 EOS 700D를 지원 모델에 포함한다.
- 같은 날짜 기준 Canon 공개 release note는 최신 EDSDK 계열에서 Windows/macOS 기반 host PC 제어와 image transfer 기능 축이 계속 유지된다는 점을 보여 준다.
- 반면 local research가 교차 검증한 Canon CCAPI 지원 범위에는 EOS 700D가 포함되지 않는다.
- 따라서 Story 1.7은 CCAPI/network pivot이 아니라, 현재 문서대로 Windows sidecar + Canon EDSDK helper profile을 유지하는 것이 맞다. [Source: _bmad-output/planning-artifacts/research/technical-canon-camera-helper-research-20260328.md]

### 참고 문서

- Epic 분해: `_bmad-output/planning-artifacts/epics.md`
- PRD: `_bmad-output/planning-artifacts/prd.md`
- 아키텍처: `_bmad-output/planning-artifacts/architecture.md`
- UX: `_bmad-output/planning-artifacts/ux-design-specification.md`
- sprint change proposal: `_bmad-output/planning-artifacts/sprint-change-proposal-20260328-023725.md`
- 이전 스토리: `_bmad-output/implementation-artifacts/1-6-실카메라-helper-readiness-truth-연결과-false-ready-차단.md`
- Canon helper sidecar 계약: `docs/contracts/camera-helper-sidecar-protocol.md`
- Canon EDSDK helper 프로파일: `docs/contracts/camera-helper-edsdk-profile.md`
- session manifest 계약: `docs/contracts/session-manifest.md`
- preset bundle 계약: `docs/contracts/preset-bundle.md`
- branch rollout 계약: `docs/contracts/branch-rollout.md`
- authoring publication 계약: `docs/contracts/authoring-publication.md`
- hardware validation checklist: `docs/runbooks/booth-hardware-validation-checklist.md`
- hardware validation architecture research: `docs/runbooks/booth-hardware-validation-architecture-research.md`
- release baseline: `docs/release-baseline.md`
- Canon helper 기술 연구: `_bmad-output/planning-artifacts/research/technical-canon-camera-helper-research-20260328.md`
- 프로젝트 컨텍스트: 없음

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.7: 실카메라 capture round-trip과 RAW handoff correlation]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-004 Current-Session Capture Persistence and Truthful Preview Confidence]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-004 Session Isolation and Privacy]
- [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Frontend Architecture]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Preview Waiting 보호 흐름]
- [Source: _bmad-output/planning-artifacts/sprint-change-proposal-20260328-023725.md]
- [Source: _bmad-output/implementation-artifacts/1-6-실카메라-helper-readiness-truth-연결과-false-ready-차단.md#스코프-분할-기준]
- [Source: docs/contracts/camera-helper-sidecar-protocol.md]
- [Source: docs/contracts/camera-helper-edsdk-profile.md]
- [Source: docs/contracts/session-manifest.md]
- [Source: docs/contracts/preset-bundle.md]
- [Source: docs/contracts/branch-rollout.md]
- [Source: docs/contracts/authoring-publication.md]
- [Source: docs/runbooks/booth-hardware-validation-checklist.md#HV-04-실제-촬영과-RAW-저장-확인]
- [Source: docs/runbooks/booth-hardware-validation-checklist.md#HV-05-Preview-Waiting---Preview-Ready-확인]
- [Source: docs/runbooks/booth-hardware-validation-architecture-research.md]
- [Source: docs/release-baseline.md]
- [Source: _bmad-output/planning-artifacts/research/technical-canon-camera-helper-research-20260328.md]
- [Source: https://asia.canon/en/campaign/developerresources/sdk]
- [Source: https://asia.canon/en/campaign/developerresources/camera/cap/edsdk-eos-digital-camera-sdk-release-note]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-03-28 02:48 +09:00 - Epic 1 Story 1.7, PRD, architecture, UX, 1.6 story, capture host code, shared contracts, hardware validation runbook, Canon helper research artifact를 교차 검토해 round-trip scope를 정리했다.
- 2026-03-28 02:48 +09:00 - 현재 구현이 placeholder RAW persistence와 timer-based preview completion에 의존하고 있음을 확인했고, 이를 real helper correlation 기준으로 교체해야 한다고 정리했다.
- 2026-03-28 03:27 +09:00 - helper request log, capture event parsing, file-arrived 검증, runtime-scoped single in-flight guard를 추가하고 placeholder RAW 생성 경로를 helper handoff 기반 persistence로 교체했다.
- 2026-03-28 03:27 +09:00 - capture round-trip happy path, accepted-but-no-file timeout, wrong session/request correlation, duplicate arrival, in-flight second capture block Rust integration test와 TypeScript contract/runtime test를 검증했다.
- 2026-03-28 03:58 +09:00 - `sidecar/canon-helper/src/CanonHelper`에 Windows용 `canon-helper.exe` 프로젝트를 추가하고 Canon EDSDK 13.19.0 vendor payload를 연결했다.
- 2026-03-28 03:58 +09:00 - `--version`, `--self-check`, session diagnostics status writer, single-session request watcher, actual EDSDK camera open/status heartbeat를 smoke 검증했다.

### Implementation Plan

- helper round-trip event handling과 single in-flight guard를 먼저 닫는다.
- placeholder ingest를 real helper `file-arrived` 기반 persistence로 교체한다.
- preview/render kickoff를 RAW persistence 뒤로 잠그고 mismatch/timeout/duplicate tests와 hardware evidence를 닫는다.

### Completion Notes List

- helper round-trip이 `capture-accepted`가 아니라 correlated `file-arrived`와 실제 RAW 파일 존재 확인 뒤에만 `capture-saved`로 닫히도록 변경했다.
- `requestId`는 host가 생성하고 `captureId`는 helper가 `file-arrived`에서 확정하도록 경계를 고정했다.
- runtime root 단위 single in-flight guard를 추가해 round-trip 중 second capture를 차단하고, preview kickoff가 RAW persistence 뒤에만 시작되도록 유지했다.
- `cargo test --test capture_readiness`, `cargo test --test operator_recovery`, `cargo test --test operator_audit`, `pnpm vitest run src/capture-adapter/services/capture-runtime.test.ts src/shared-contracts/contracts.test.ts`를 통과했다.
- Windows 전용 `.NET 8` 기반 `canon-helper.exe` 프로젝트를 추가했고, local Canon EDSDK 13.19.0 payload를 output/publish에 함께 싣도록 구성했다.
- `canon-helper.exe --self-check`가 현재 장비에서 `cameraCount=1`, `camera-ready`, `Canon EOS 700D` readiness smoke까지 통과했다.
- helper를 임시 runtime/session에 붙여 `camera-helper-status.json`이 실제 세션 diagnostics 아래에 생성되는 비파괴 smoke를 확인했다.
- 2026-03-29 실장비 세션 `session_000000000018a138ef5c96c18c`에서 booth 앱 `사진 찍기` 버튼으로 시작한 `request-capture -> capture-accepted -> file-arrived -> session persistence` round-trip을 확인했다.
- 같은 회차에서 `camera-helper-requests.jsonl`, `camera-helper-events.jsonl`, `session.json`, 실제 RAW 파일, preview 파일이 동일한 `requestId`/`captureId`로 닫히는 것을 확인했다.
- 2026-03-29 실장비 세션 `session_000000000018a157b0cfc8cea4`에서도 booth 앱 `사진 찍기` 버튼 경로로 actual capture round-trip을 다시 확인했다.
- 같은 회차에서 `request_000000000018a157b2a1a5e4ec`와 `capture_20260329145553949_656bbc58ff`가 `camera-helper-requests.jsonl`, `camera-helper-events.jsonl`, `session.json`, 실제 RAW, preview 파일에서 동일하게 correlation 되는 것을 직접 확인했다.
- 해당 세션 `session.json`의 timing 필드는 `captureAcknowledgedAtMs`와 `previewVisibleAtMs`를 함께 남기고 있고, 사용자는 같은 회차의 `Preview Waiting -> Preview Ready` booth 검증을 완료했다고 확인했다.
- 이번 story closure package는 booth 앱 `사진 찍기` 버튼 경로 증거를 기준으로 닫고, 카메라 본체 direct shutter는 후속 별도 story에서 구현/검증한다.

### Hardware Validation Evidence

- sessionId: `session_000000000018a157b0cfc8cea4`
- requestId: `request_000000000018a157b2a1a5e4ec`
- captureId: `capture_20260329145553949_656bbc58ff`
- helper request evidence: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a157b0cfc8cea4\diagnostics\camera-helper-requests.jsonl`
- helper event evidence: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a157b0cfc8cea4\diagnostics\camera-helper-events.jsonl`
- RAW evidence: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a157b0cfc8cea4\captures\originals\capture_20260329145553949_656bbc58ff.CR2`
- preview evidence: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a157b0cfc8cea4\renders\previews\capture_20260329145553949_656bbc58ff.jpg`
- session manifest evidence: `C:\Users\KimYS\Pictures\dabi_shoot\sessions\session_000000000018a157b0cfc8cea4\session.json`
- timing supporting proof: `session.json.captures[0].timing.captureAcknowledgedAtMs=1774796153069`, `previewVisibleAtMs=1774796155107`
- booth validation note: 사용자가 같은 회차의 `Preview Waiting -> Preview Ready` 검증 완료를 확인했다.

### File List

- .gitignore
- docs/contracts/camera-helper-sidecar-protocol.md
- docs/contracts/camera-helper-edsdk-profile.md
- sidecar/canon-helper/README.md
- sidecar/canon-helper/vendor/README.md
- sidecar/canon-helper/src/CanonHelper/CanonHelper.csproj
- sidecar/canon-helper/src/CanonHelper/HelperVersion.cs
- sidecar/canon-helper/src/CanonHelper/CanonHelperOptions.cs
- sidecar/canon-helper/src/CanonHelper/Program.cs
- sidecar/canon-helper/src/CanonHelper/Protocol/CanonHelperMessages.cs
- sidecar/canon-helper/src/CanonHelper/Runtime/SessionPaths.cs
- sidecar/canon-helper/src/CanonHelper/Runtime/JsonFileProtocol.cs
- sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs
- sidecar/canon-helper/src/CanonHelper/Runtime/CanonHelperService.cs
- src-tauri/src/capture/sidecar_client.rs
- src-tauri/src/capture/mod.rs
- src-tauri/src/capture/normalized_state.rs
- src-tauri/src/capture/ingest_pipeline.rs
- src-tauri/tests/capture_readiness.rs
- src-tauri/tests/operator_recovery.rs
- src-tauri/tests/operator_audit.rs

### Change Log

- 2026-03-28 - helper-backed capture round-trip, correlated RAW persistence validation, runtime-scoped in-flight guard, and round-trip regression tests를 추가했다.
- 2026-03-28 - Windows용 `canon-helper.exe` baseline project, Canon EDSDK payload wiring, `--version`/`--self-check`, session diagnostics status writer를 추가했다.
- 2026-03-29 - 실장비 세션 `session_000000000018a138ef5c96c18c`에서 actual capture round-trip, helper correlation, RAW persistence, manifest correlation, preview file 생성 evidence를 수집했다.
