# Deferred Work

## Deferred from: review of HV-17 No-Go remediation (2026-08-17)

- HV-17B의 generation gate가 최소 한 건의 refined 존재와 각 refined의 선행 proxy만 확인한다. 모든 대상 capture가 `proxy commit → refined commit → refined present`를 완결했는지 검증하는 coverage 계약과 결손 테스트가 별도 보강되어야 한다. (`tests/hardware/raw-refined/hv-17/check-raw-refined-evidence.ps1`, pre-existing)
- HV-17A gate는 telemetry의 `P0`/`P1` 라벨을 확인하지만 실제 경쟁 작업의 dequeue/실행 순서를 비교하지 않는다. 우선순위 라벨 오기재와 실제 역순 실행을 구분할 scheduler-order evidence schema가 필요하다. (`tests/hardware/raw-refined/hv-17/check-raw-refined-evidence.ps1`, pre-existing)

## Deferred from: code review of 7-5-상주-renderer-검증-spike (2026-08-16)

**proxy 경로 미결 6건은 Story 7.6이 같은 파일을 열어 놓고 전부 닫았다 (2026-08-16).**
같은 자리를 세 번째로 보지 않기 위해, 무엇을 어떻게 닫았는지 아래에 남긴다.

- ~~기존 proxy 경로가 렌더 시작 시점 viewer epoch를 게시 시점에 재사용해 stale 게시를 놓칠 수 있다.~~
  **닫힘.** `publish_rendered_proxy_in_dir` / `publish_rendered_raw_refined_in_dir`이
  `current_viewer_epoch`를 인자로 받는다. job에 실린 렌더 시작 시점 값을 쓸 수 없게 구조로 막았고,
  호출자는 게시 직전에 다시 읽은 `fresh_viewer.viewer_epoch`를 넘긴다.
- ~~기존 proxy staging 경로가 중복 작업 사이에서 충돌할 수 있다.~~
  **닫힘.** `proxy_render_staging_path`가 `<requestId>-<captureId>-<tier>-render.jpg`로
  generation 좌표를 싣는다. P0와 P1이 함께 도는 지금 이 충돌 확률이 올라갔다.
  `staging_paths_never_collide_between_concurrent_jobs`가 고정한다.
- ~~기존 proxy render/read 실패 경로가 staging 파일을 정리하지 않는다.~~
  **닫힘.** 재읽기 실패·구조 decode 실패·크기 불일치·취소 경로가 전부 staging 파일을 지운다.
  중간에 죽은 파일이 다음 시도의 "성공"으로 오인되지 않는다.
- ~~기존 actual-present 수신 경로가 report viewer epoch와 generation epoch를 대조하지 않는다.~~
  **닫힘.** `GenerationContext`가 `viewer_epoch`를 들고 있고, 불일치 보고는 `stale-epoch` 거부로
  확정된다. **행을 없애지 않는다** — 조용히 버리면 계측 완결성이 깨진다.
  한 촬영에 generation이 둘이 되면서 잘못된 짝짓기 위험이 실제로 커졌다.
- ~~기존 terminal present evidence 쓰기 실패가 호출자에게 성공으로 반환된다.~~
  **닫힘.** `persist_present_record`가 실패를 그대로 반환하고 `report_display_present`가
  그 값을 돌려준다. HV-17B의 완결성 판정이 이 반환값 위에 선다.
- ~~기존 display telemetry map이 terminal/session 전환 뒤에도 generation metadata를 보유한다.~~
  **닫힘.** 세션 교체 시 이전 세션 tombstone을 지우고, `MAX_RETAINED_GENERATION_CONTEXTS`(512)
  상한을 두어 오래된 것부터 버린다. **미결 generation은 절대 버리지 않는다.**
  촬영당 generation이 2개가 되어 누수량이 두 배가 됐다.

아래 세 건은 이 Story가 손대는 코드가 아니라 **문서/ledger 서술**이며 그대로 남는다.

- Story 7.2/HV-13B의 `done`과 `No-Go` 기록이 ledger 안에서 충돌한다. (`hardware-validation-ledger.md:28`)
- Story 7.4/HV-15의 예외 승인과 품질 미통과 표현이 일반 `Go`와 구분되지 않는다. (`hardware-validation-ledger.md:30`)
- viewer present 계약의 일부가 실제 하드웨어 재검증보다 앞서 확정 상태로 서술되어 있다. (`docs/contracts/viewer-display.md`)

## Resolved follow-up (2026-08-16)

- 전체 `pnpm build`의 기존 TypeScript 오류 20건을 해소했고 production build가 통과한다.
- Story 1.4/1.5 gate 문구를 현재 상태와 맞추고 보존 worktree를 테스트 수집에서 제외해 전체 Vitest가 통과한다.
- render queue 상태를 runtime root별로 격리해 capture readiness 통합 테스트 60건이 기본 병렬 실행에서 통과한다.

## Deferred from: code review of 7-3-libraw-embedded-jpeg와-raw-jpeg-source-비교 (2026-08-14)

- helper `SourceObjectRejected` → host sample reason 매핑과 paired JPEG 5s timeout 경계에 대한 자동 검증이 약하다. 합성 게이트/단위 테스트로 reject 사유 전파와 timeout 이후 late-arrival을 고정할 후속 작업이 필요하다.

## Deferred from: code review of 7-3-libraw-embedded-jpeg와-raw-jpeg-source-비교 (2026-08-13)

- Hardware validation ledger에서 Story 7.2의 re-scoped `Go`/`done` 설명과 요약 gateboard의 `No-Go` 표시가 상충한다. Story 7.2/HV-13B 상태 모델을 별도 정리해야 한다. (`hardware-validation-ledger.md:30`, pre-existing)

## Deferred from: code review of 7-6-raw-정밀본-무중단-교체 (2026-08-16)

- 단일 모니터 환경에서 windowed viewer fallback 없이 생성이 중단되는 누적 변경을 Story 7.1 범위에서 재검토한다. (`src-tauri/src/commands/viewer_commands.rs:329`, pre-existing)
- 초기 viewer 생성도 trusted input 이후 창 이벤트로 집계될 수 있는 누적 telemetry 변경을 별도 검증한다. (`src-tauri/src/commands/viewer_commands.rs:456`, pre-existing)
- reveal 성공 전에 epoch를 소비해 같은 epoch의 재시도가 막힐 수 있는 누적 viewer 변경을 별도 수정한다. (`src-tauri/src/commands/viewer_commands.rs:506`, pre-existing)
- 구형 generation에 안정적인 capture 순서가 없을 때 commit 시각으로 오래된 촬영을 추정하는 fallback의 재시작·혼합 버전 정책을 별도로 정한다. (`src/display-generation/services/display-guard.ts:145`, pre-existing)
- viewer present span에서 decode·swap·actual-present의 시간 선후관계를 검증하지 않는 누적 telemetry 계약을 별도로 보강한다. (`src/shared-contracts/schemas/viewer-display.ts:598`, pre-existing)
- clock calibration이 장시간 성립하지 않을 때 서로 다른 generation의 pending present 보고가 제한 없이 누적되는 기존 telemetry 정책에 보존 상한 또는 durable spool 정책을 정한다. (`src/viewer-surface/pending-present-report.ts:8`, pre-existing)
