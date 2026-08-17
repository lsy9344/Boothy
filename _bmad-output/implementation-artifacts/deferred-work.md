# Deferred Work

## Deferred from: 7-7-완전한-installer와-clean-offline-재현 (2026-08-17)

이 Story 가 열어 놓고 **닫지 못한** 것들이다. 코드로 풀 수 없거나, 이 Story 의 범위 밖이다.

### 외부 종속 — 코드로 풀 수 없다

- **darktable GPL-3.0 재배포 고지 문안이 승인 대기다.** 의무 항목과 대응 소스 경로는 문서로
  고정했다. 운영자에게 나가는 문안 자체는 승인자의 몫이다. (`release/licenses/darktable-5.4.1.md`)

### 닫힌 것 (여기 적었다가 실제로 해결된 항목)

- **코드 서명 인증서와 Canon EDSDK 재배포 증거는 2026-08-17 Noah Lee 승인으로 MVP 내부
  HV-18A에서 면제됐다.** 해결 또는 통과가 아니라 `waived-by-owner`다. 설치본은 `unsigned`,
  재배포 권리는 `not-evidenced`로 계속 기록하며 공개 배포 적합성을 주장하지 않는다.

- ~~helper self-contained publish 실행 검증~~ **닫힘.** 이 머신에 EDSDK 페이로드가 있어
  `dotnet publish -r win-x64 --self-contained true` → `--version` → `--self-check` 가 전부
  실제로 통과했다 (311 파일 / 300.7 MB, `camera-ready`, 카메라 1대 인식).
  EDSDK 런타임 트리 해시는 실측값으로 명세에 핀으로 기록했다.

### 미결 결정 — 이 Story 가 기본안을 만들었지만 확정은 다른 사람 몫이다

- **CI 벤더 페이로드 조달 경로는 이번 내부 검증에서 제외됐다.** 현재 PC의 승인된 로컬
  페이로드로 전체 설치본을 만들고 검증했다. 공개/자동 릴리스로 전환할 때만 self-hosted runner
  고정 캐시 또는 보안 아티팩트 저장소를 결정한다. (`.github/workflows/release-windows.yml`)
- **darktable 5.4.1 staged-tree 핀은 실측값으로 채워졌다.** 공식 배포본의 별도 source archive
  파일은 이번 경로에 없으므로 source-archive 해시는 계속 비워 두되, strict 검증은 실제 staged
  tree digest로 통과한다. EDSDK 런타임도 source/staged 핀이 채워졌고 camera-helper는 저장소가
  직접 빌드하므로 `requiresPin: false`다. (`release/inventory-spec.json`)

### 현재 PC에서 실측해 닫힌 값

- **lifecycle에 사용한 darktable 포함 설치본은 405,257,006바이트**였고 설치된 전체 트리
  self-check를 통과했다. 이후 촬영 준비 갱신 수정을 포함해 다시 만든 최신 설치본은
  **405,260,278바이트**, SHA-256
  `202518faf04be56d4b47fc1a7ba7e180366e08be551700bc637885e93b94140c`이며 strict inventory
  검증을 통과했다. 현재 PC의 WebView2 런타임은 `151.0.4129.86`으로 기록했다. 다른 PC에서
  실행할 때에는 고정값으로 가정하지 않고 self-check가 다시 읽는다.

### 범위 밖 (다른 Story 소유)

- 100-shot 성능, cold/idle/reconnect 회복 → **Story 7.8 / HV-18B**
- 120fps+ 물리 frame 과 compositor→photon 오프셋 → **HV-18B 단독**
- 지점 승급과 운영 롤백 검증 → **Story 7.9 / HV-18C**. 이 Story 의 롤백은 **같은 PC 재설치**다.

## Deferred from: approved HV-17 Partial disposition (2026-08-17)

- RAW-refined tier는 detail 이득이 없어 기본 lane을 껐다. 따라서 HV-17B coverage/observer 항목은
  현재 출시 조건이 아니다. 향후 tier를 다시 켤 때 모든 capture의
  `proxy commit → refined commit → refined present` 완결성과 결손 테스트를 먼저 보강한다.
- HV-17A의 네 가지 실제 Windows cancellation/process-tree 시험은 통과했다. 더 강한 경쟁 작업
  dequeue 순서 schema는 RAW-refined P1 lane을 다시 켤 때 재검토한다.

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
