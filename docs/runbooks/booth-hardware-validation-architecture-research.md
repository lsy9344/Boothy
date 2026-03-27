# Booth Hardware Validation Architecture Research

## 목적

이 문서는 Boothy의 승인된 아키텍처를 기준으로, 실제 카메라 연결과 darktable 기반 사진 편집 경계를 어떻게 검증해야 하는지 연구한 상세 노트다.

현장 실행 절차는 [booth-hardware-validation-checklist.md](./booth-hardware-validation-checklist.md)가 담당한다. 이 문서는 그 체크리스트의 상위 근거 문서로서 아래 질문에 답한다.

- 무엇을 반드시 검증해야 하는가
- 왜 그 검증이 아키텍처상 필수인가
- 어떤 방법으로 검증해야 false-ready, false-complete, preset drift, 세션 누수 같은 제품 리스크를 줄일 수 있는가
- 어떤 실패는 단순 버그가 아니라 아키텍처 위반으로 봐야 하는가

## 문서 상태

- 작성 기준일: 2026-03-26
- 기준 코드베이스: 현재 `C:\Code\Project\Boothy`
- 목적: 제품 출시 전 실장비 검증의 방향과 방법을 고정
- 성격: 연구/운영 설계 문서

## 연구 범위

이번 연구는 아래 범위를 다룬다.

- 실제 카메라 연결과 helper 경계
- RAW 저장과 세션 durable truth
- darktable preset artifact 적용 경계
- preview/final truth 분리
- active session과 future session의 preset 분리
- post-end truth와 operator recovery
- release gate에서 어떤 증거를 남겨야 하는지

이번 연구는 반대로 아래를 직접 해결하지는 않는다.

- helper 구현 세부 코드 변경
- darktable preset 품질 자체를 새로 설계하는 일
- 배포 파이프라인 변경
- 현장 운영 정책 승인

## 연구 방법

이번 문서는 세 가지 층을 함께 사용했다.

1. 로컬 아키텍처 / PRD / 계약 문서 검토
2. 현재 구현 및 테스트 시나리오 검토
3. 공식 외부 문서 확인

외부 문서는 아래를 우선했다.

- darktable 공식 문서
- Microsoft 공식 문서

추론이 들어가는 부분은 이 문서에서 명시적으로 `추론`으로 표기한다.

## 사용한 근거

### 로컬 근거

- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/prd.md`
- `docs/contracts/session-manifest.md`
- `docs/contracts/preset-bundle.md`
- `docs/contracts/authoring-publication-payload.md`
- `docs/contracts/authoring-publication.md`
- `reference/darktable/README.md`
- `src-tauri/src/capture/normalized_state.rs`
- `src-tauri/src/capture/ingest_pipeline.rs`
- `src-tauri/src/diagnostics/mod.rs`
- `src-tauri/tests/capture_readiness.rs`
- `src-tauri/tests/operator_diagnostics.rs`

### 외부 근거

- darktable CLI manual: `https://docs.darktable.org/usermanual/4.8/en/special-topics/program-invocation/darktable-cli/`
- darktable sidecar files manual: `https://docs.darktable.org/usermanual/4.8/en/overview/sidecar-files/sidecar/`
- darktable supported file formats: `https://docs.darktable.org/usermanual/4.8/en/overview/supported-file-formats/`
- darktable-cltest manual: `https://docs.darktable.org/usermanual/4.0/en/special-topics/program-invocation/darktable-cltest/`
- darktable OpenCL manual: `https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/`
- Microsoft WinUSB power management: `https://learn.microsoft.com/en-us/windows-hardware/drivers/usbcon/winusb-power-management`

## 먼저 정리하는 핵심 결론

### 결론 1

Boothy의 실검증은 화면 확인이 아니라 경계 검증이다.

아키텍처는 Tauri/Rust host가 단일 정규화 지점이어야 하고, camera/helper truth, render truth, timing truth, post-end truth를 각각 분리해서 다루라고 요구한다. 따라서 실검증도 “버튼 눌렀더니 되더라”가 아니라 “어떤 truth가 언제 확정됐는가”를 증명해야 한다.

### 결론 2

카메라 연결과 darktable 적용을 실패 없이 운영하려면, 가장 중요한 것은 기능 수보다 경계 오염 방지다.

특히 아래 네 가지가 출시 리스크의 중심이다.

- helper live state가 customer `Ready`에 섞여 false-ready를 만드는 것
- RAW 저장과 preview/final 준비를 한 성공으로 뭉개는 것
- active session binding이 publish/rollback 때문에 흔들리는 것
- darktable runtime state가 authoring 또는 다른 render mode와 섞이는 것

### 결론 3

성공-path 검증만으로는 부족하다.

실제 제품 안정성은 아래를 반드시 포함해야 증명된다.

- unplug / reconnect
- idle 이후 복귀
- render fail 격리
- active session catalog drift 방지
- 반복 촬영과 장시간 soak

### 결론 4

현재 repo는 체크리스트는 갖췄지만, 아키텍처가 기대한 일부 문서 경계는 아직 문서로 완전히 고정되어 있지 않다.

특히 architecture는 sidecar protocol과 error envelope의 별도 동결 문서를 기대하지만, 현재 `docs/contracts/`에는 그 문서가 없다. 지금은 코드와 테스트가 사실상 계약 역할을 일부 대신하고 있다. 따라서 실장비 검증에서는 DTO / manifest / diagnostics evidence를 더 엄격히 남겨야 한다.

## 아키텍처가 요구하는 검증 모델

아키텍처 문서는 아래를 강하게 고정한다.

- active booth truth는 session-scoped filesystem root가 durable source of truth다.
- Rust host는 camera/helper truth, timing truth, post-end truth의 single normalization point다.
- active session은 preset version을 명시적으로 참조해야 한다.
- `darktable-cli` render worker는 RAW ingest 이후에만 동작하고, capture success와 render success는 분리된다.
- operator diagnostics는 같은 host truth를 바탕으로 bounded recovery만 허용해야 한다.

이 요구를 검증 가능한 구조로 바꾸면 아래 9개 경계가 나온다.

| 경계 | 소유자 | 진실의 원천 | 절대 일어나면 안 되는 일 | 검증 핵심 |
| --- | --- | --- | --- | --- |
| Customer copy boundary | booth UI | host-normalized DTO | 고객 화면이 내부 기술 용어를 노출 | copy audit + blocked state drill |
| Host normalization boundary | Rust host | normalized readiness/post-end | UI가 helper/raw 상태를 직접 해석 | readiness/post-end DTO 순서 검증 |
| Camera live boundary | helper + host | live device state | 카메라 준비 전 `Ready` 노출 | unplug/reconnect/idle 검증 |
| Session durability boundary | session root | `session.json` + asset paths | capture success 전에 RAW 없음 | RAW-first persistence 검증 |
| Render boundary | darktable worker | preview/final assets + render status | preview/final 미완료인데 성공처럼 보임 | waiting/ready/fail 분리 검증 |
| Preset publication boundary | authoring + catalog | immutable published bundle + catalog state | active session binding drift | publish/rollback during session 검증 |
| Timing/post-end boundary | host timing/handoff | `postEnd` + lifecycle stage | end 이후 generic ready로 회귀 | exact-end / export-waiting / completed 검증 |
| Operator boundary | diagnostics + actions | same normalized truth | 고객용 안내와 운영자 진단 혼선 | operator summary와 blocked category 검증 |
| Release/branch boundary | rollout + local config | approved build + preset stack | 업데이트나 catalog drift가 active session 침범 | staged rollout / future-session-only 검증 |

## 외부 연구에서 얻은 중요한 사실

### 1. darktable는 XMP sidecar와 library DB를 함께 사용한다

darktable 공식 문서는 darktable가 non-destructive editor이고, 메타데이터와 편집 이력을 `.XMP` sidecar에 저장한다고 설명한다. 동시에 image-related data를 library database에도 유지하며, import 이후에는 database entries가 XMP보다 우선한다고 설명한다.

Boothy에 주는 의미:

- runtime apply truth는 shared GUI database가 아니라 immutable artifact여야 한다.
- authoring에서 사용한 darktable state를 booth runtime의 business truth로 삼으면 안 된다.
- runtime은 isolated `configdir` / `library`를 써야 한다.
- XMP 기반 artifact를 주 경로로 두고 style 기반 경로는 운영 편의 또는 예외 경로로 제한하는 편이 안전하다.

추론:

Boothy architecture가 `XMP sidecar template`를 1차 artifact로 두고 style 경로를 비권장으로 둔 판단은 이 공식 동작과 잘 맞는다.

### 2. darktable-cli는 console mode이며 XMP 파일을 직접 적용할 수 있다

darktable 공식 문서는 `darktable-cli`가 GUI 없이 console mode로 export를 수행하며, optional XMP sidecar file을 받아 history stack data를 적용할 수 있다고 설명한다. 또 `--style`을 쓸 때는 `--configdir` 지정이 필요하고, `--apply-custom-presets`는 `data.db`를 로드하는 옵션이며, 이를 꺼야 multiple instances 실행이 쉬워진다고 설명한다.

Boothy에 주는 의미:

- background render worker 경계는 아키텍처적으로 타당하다.
- preview/final 경로는 XMP 기반 경로를 우선 검증해야 한다.
- style 기반 경로는 `data.db` 의존성이 커서 출시 게이트에서는 더 높은 위험 경로로 취급해야 한다.
- multiple instance 또는 queue 운영을 하려면 shared config/library/data.db 의존성을 줄이는 방향이 맞다.

실무 검증 포인트:

- preview path와 final path는 서로 다른 `configdir` / `library`를 사용해야 한다.
- `--style` 기반 apply를 주 경로로 둘 생각이라면 별도 risk review가 필요하다.
- concurrency 또는 queue backlog 검증은 darktable state 공유 여부와 함께 봐야 한다.

### 3. darktable-cltest와 OpenCL은 “정확성”보다 “성능 프로필” 문제다

darktable 공식 문서는 `darktable-cltest`가 usable OpenCL environment를 확인하는 용도라고 설명한다. 또 OpenCL이 성공하면 speed-up을 기대할 수 있지만, CPU와 GPU 결과는 rounding error를 제외하고 동일하도록 설계돼 있고, GPU 계산 실패 시 자동으로 CPU fallback을 시도한다고 설명한다.

Boothy에 주는 의미:

- GPU 부재 또는 OpenCL 비활성은 correctness failure가 아니라 우선 performance risk다.
- 따라서 검증은 두 층으로 나눠야 한다.
  - correctness gate: 결과가 truthful하게 나온다
  - performance gate: preview SLA를 만족한다

실무 검증 포인트:

- `darktable-cltest` 결과를 반드시 증거로 남긴다.
- GPU off 또는 fallback 환경에서도 preview/final correctness가 유지되는지 확인한다.
- 다만 preview latency가 SLA를 넘으면 release 관점에서는 여전히 `No-Go`가 될 수 있다.

### 4. darktable는 많은 RAW 포맷을 지원하지만, 실제 카메라 파일 포맷을 현장에서 확인해야 한다

darktable 공식 문서는 CR2, CR3를 포함한 다수의 RAW 확장자를 지원한다고 설명한다.

Boothy에 주는 의미:

- 지원 목록만으로는 충분하지 않다.
- 실제 사용하는 카메라가 어떤 확장자와 전송 경로를 쓰는지 현장에서 확인해야 한다.
- helper가 파일을 어떻게 넘기고, darktable가 그 파일을 실제로 수용하는지 end-to-end로 확인해야 한다.

실무 검증 포인트:

- 카메라 모델별 실제 생성 RAW 확장자를 기록한다.
- 샘플 RAW만이 아니라 실촬영 RAW로 preview/final을 만들어 본다.

### 5. Windows USB selective suspend는 실카메라 안정성 검증을 요구한다

Microsoft 공식 문서는 WinUSB power management에서 selective suspend가 존재하며, idle 시 장치를 power down할 수 있고, read requests가 device를 wake 할 수 있다고 설명한다. 또 단일 설정만으로 이를 강제할 수 없고, device idle 관련 정책이 동작에 영향을 준다고 설명한다.

Boothy에 주는 의미:

- 실카메라 검증은 단순 연결 테스트 1회로 끝나면 안 된다.
- idle 이후 복귀, 장시간 대기, 분리/재연결, 포트 변경 같은 시나리오가 필요하다.

추론:

Boothy helper가 WinUSB를 직접 쓰는지 여부는 repo에서 문서화돼 있지 않다. 하지만 Windows USB power management가 실제 장치 연결 안정성에 영향을 주는 구조라는 점은 분명하므로, 실장비 검증에 idle/reconnect/soak를 넣는 것은 합리적이다.

## 아키텍처 기반 검증 방향

### 방향 1. 기능 단위보다 truth 전환 단위로 검증한다

검증 질문은 “촬영이 됐나?”가 아니라 아래처럼 바뀌어야 한다.

- 언제 RAW persistence truth가 생겼는가
- 언제 preview truth가 생겼는가
- 언제 final 또는 post-end truth가 생겼는가
- 어느 시점까지 capture가 허용되어야 하고, 어느 시점부터 막혀야 하는가

이 방향이 중요한 이유는 PRD가 preview/final truth를 미리 암시하면 안 된다고 명시하기 때문이다.

### 방향 2. success path와 degraded path를 같은 무게로 본다

성공 경로만 맞고 실패 경로에서 false-ready가 나오면 제품 신뢰는 무너진다.

따라서 최소 동일한 우선순위로 검증해야 할 degraded path는 아래다.

- helper preparing
- camera disconnected
- stale preset binding
- render failed
- exact-end reached
- post-end phone-required
- publish/rollback during active session

### 방향 3. active session과 future session을 엄격히 분리해 검증한다

이 제품의 가장 큰 아키텍처 규칙 중 하나는 active session이 future catalog change에 영향을 받지 않는 것이다.

따라서 publish / rollback 검증은 단순히 catalog UI 변경 확인이 아니라 아래를 같이 봐야 한다.

- 기존 세션 `catalogSnapshot` 유지
- 기존 capture binding 유지
- 새 세션에만 새 live version 반영

### 방향 4. correctness와 performance를 분리해 평가한다

darktable OpenCL 공식 문서를 근거로 보면 GPU는 주로 speed-up 문제다.

따라서 평가도 분리한다.

- correctness fail: wrong state, wrong asset, wrong preset, missing evidence
- performance fail: preview SLA 미달, queue backlog 장기화

이 구분이 없으면 GPU fallback을 불필요하게 correctness bug로 오인하거나, 반대로 속도 문제를 사소하게 넘기게 된다.

### 방향 5. operator path는 고객 path의 그림자가 아니라 별도 경계로 본다

아키텍처는 operator diagnostics를 same normalized truth 위에서 bounded recovery만 허용하는 별도 경계로 본다.

따라서 operator 검증은 단순 “화면이 보인다”가 아니라 아래를 봐야 한다.

- blocked category가 올바르게 분류되는가
- capture-blocked와 preview-render-blocked를 혼동하지 않는가
- recovery action이 고객 화면의 의미를 뒤집지 않는가

## 아키텍처 기반 검증 방법

아래 순서로 실행하는 것이 가장 안전하다.

## Phase 0. 계약 및 문서 동결 확인

목표:
검증 전에 무엇이 truth인지부터 잠근다.

확인할 것:

- `session-manifest.md`의 `catalogSnapshot`, `activePreset`, `postEnd`, session-root asset path 규칙
- `preset-bundle.md`의 immutable bundle, `catalog-state.json`, future-session-only 규칙
- `authoring-publication*.md`의 pinned darktable version, duplicate version reject, active session 불변 규칙
- darktable runtime pin과 path convention

출력:

- 이번 회차에서 사용할 build version
- darktable version pin
- helper version 또는 식별값
- presetId / publishedVersion baseline

실패 의미:

- 이 단계가 잠기지 않으면 이후의 실장비 검증 결과는 재현성이 떨어진다.

## Phase 1. 환경 선행 점검

목표:
장비와 런타임이 검증 가능한 상태인지 먼저 확인한다.

방법:

- `darktable --version`
- `darktable-cltest`
- preview/final/style별 `configdir` / `library` 경로 기록
- booth runtime과 authoring runtime state 분리 확인
- 세션 루트 쓰기 가능 여부 확인
- published bundle과 `catalog-state.json` live pointer 확인

통과 기준:

- darktable pin 일치
- state 공유 없음
- session root / diagnostics writable
- published bundle과 live pointer 일치

## Phase 2. 단일 happy path 증명

목표:
가장 기본적인 end-to-end truth 분리를 증명한다.

방법:

1. 새 세션 시작
2. published preset 선택
3. camera ready 진입
4. 실제 촬영
5. RAW 저장 확인
6. `Preview Waiting`
7. preview ready
8. 필요 시 final ready
9. end 이후 `Export Waiting` 또는 `Completed`

반드시 남길 증거:

- `session.json`
- RAW / preview / final 경로
- `timing-events.log`
- `bundle.json`
- 화면 캡처

핵심 판정:

- capture acceptance 전 RAW가 없으면 안 된다.
- preview 파일 전에는 preview ready를 암시하면 안 된다.
- post-end truth 전에는 completed를 암시하면 안 된다.

## Phase 3. 경계 교란 테스트

목표:
경계가 흔들릴 때도 truth가 무너지지 않는지 확인한다.

필수 시나리오:

- camera unplug while ready
- reconnect and recover
- idle after ready, then capture
- stale preset binding
- publish new version during active session
- rollback during active session
- render fail before preview ready
- render fail after end

판정 기준:

- degraded path에서 false-ready / false-complete가 없어야 한다.
- evidence가 남아야 한다.
- 고객 안내는 계속 booth-safe여야 한다.

## Phase 4. 반복 / soak / 성능 검증

목표:
1회 성공이 아니라 운용 안정성을 본다.

권장 방법:

- 10회 연속 capture smoke
- 30회 capture soak
- session restart 반복
- idle 10분 / 20분 대기 후 복귀 capture
- GPU on/off 또는 fallback 비교

측정 포인트:

- preview latency 분포
- render backlog 여부
- helper reconnect 안정성
- 세션 파일 증가에 따른 성능 저하
- memory / disk pressure 징후

판정 방법:

- correctness가 모두 통과하더라도 preview SLA를 지속적으로 넘기면 `performance No-Go`
- helper recovery가 일정하지 않으면 `operational No-Go`

## Phase 5. operator / release gate 검증

목표:
실패가 나도 현장이 통제 가능한지 확인한다.

검증 항목:

- operator summary의 blocked category 정확성
- customer-safe guidance 유지
- allowed action만 노출되는지
- active session 중 forced update 없음
- publish/rollback이 future sessions only로 동작하는지

## 경계별 상세 체크 포인트

### A. Customer copy boundary

왜 중요한가:

PRD는 고객 화면에 darktable, XMP, module, style, library 같은 내부 용어가 보이면 안 된다고 고정한다.

체크 방법:

- ready
- preview waiting
- export waiting
- phone required
- reconnect 후 blocked state

각 화면에서 내부 용어 노출 여부를 점검한다.

실패 의미:

- 단순 카피 버그가 아니라 product boundary leak이다.

### B. Host normalization boundary

왜 중요한가:

아키텍처는 Rust host를 single normalization point로 고정한다.

체크 방법:

- 같은 상황에서 booth 화면과 operator 화면이 서로 다른 truth를 말하지 않는지 확인한다.
- helper error raw text가 customer UI에 그대로 드러나지 않는지 본다.
- post-end 상태가 local fallback 때문에 `ready` 또는 generic preparing으로 되돌아가지 않는지 본다.

실패 의미:

- UI가 host truth를 우회하거나 덮어쓰는 구조 문제일 수 있다.

### C. Camera live boundary

왜 중요한가:

실카메라가 준비되기 전 `Ready`가 보이면 고객 신뢰가 즉시 무너진다.

체크 방법:

- cold boot 후 첫 ready 진입
- cable reseat
- USB port change
- idle 후 첫 capture
- helper restart 후 재동기화

권장 증거:

- 화면 캡처
- helper 상태 확인 메모
- `session.json`

실패 의미:

- readiness normalization 또는 helper liveness 경계가 흔들리는 것

### D. Session durability boundary

왜 중요한가:

이 제품은 session root가 durable truth다.

체크 방법:

- capture 직후 RAW 생성 확인
- `captures[*].preview.assetPath`, `final.assetPath`가 current session root 아래 절대경로인지 확인
- cross-session reopening 시 다른 세션 자산이 보이지 않는지 확인

실패 의미:

- filesystem truth와 UI truth가 분리된 것
- privacy / isolation release blocker 가능성

### E. Render boundary

왜 중요한가:

capture success와 render success는 분리되어야 한다.

체크 방법:

- preview waiting 전환 확인
- preview 파일 생성 시점과 화면 전환 순서 비교
- render fail 시 `phone-required` 또는 safe blocked state 진입
- end 이후 final/post-end와 preview state가 섞이지 않는지 확인

실패 의미:

- false-complete 또는 false-latest-photo

### F. Preset publication boundary

왜 중요한가:

active session 불변성과 future-session-only 규칙은 branch consistency의 핵심이다.

체크 방법:

- `bundle.json` immutable 여부 확인
- duplicate version publish reject
- active session 중 publish / rollback 후 기존 세션 binding 유지
- 새 세션에만 new live version 반영

실패 의미:

- preset drift
- branch inconsistency
- rollback safety 훼손

### G. Timing/post-end boundary

왜 중요한가:

end 이후 상태는 고객 신뢰와 운영 안내를 좌우한다.

체크 방법:

- exact-end 후 capture 차단
- explicit `Export Waiting`, `Completed`, `Phone Required`
- `postEnd` truth와 화면 의미 일치
- completed 전 final 또는 handoff truth 실제 존재 여부

실패 의미:

- lifecycle truth corruption

### H. Operator boundary

왜 중요한가:

운영자는 diagnosis를 해야 하지만 customer-safe boundary를 깨면 안 된다.

체크 방법:

- capture-blocked / preview-render-blocked / completion-blocked 분류
- recent failure summary가 실제 경계와 맞는지
- recovery action이 과도하지 않은지

실패 의미:

- operator tooling이 bounded recovery를 넘는 것

### I. Release/branch boundary

왜 중요한가:

현장에서는 소프트웨어와 preset stack 변화가 실세션을 흔들 수 있다.

체크 방법:

- active session 중 forced update 없음
- preset stack rollout이 current session을 건드리지 않음
- rollback이 audit와 live pointer만 바꾸고 existing capture binding은 유지

실패 의미:

- release governance 위반

## 카메라 연결 안정성을 높이기 위한 권장 검증법

### 권장 1. idle/reconnect를 기본 시나리오로 승격한다

이유:

Windows USB power management 특성상, 연결 직후 1회 성공만으로는 안정성을 증명할 수 없다.

방법:

- ready 상태 10분 대기 후 capture
- unplug / replug 후 ready 복귀
- helper 재시작 후 recover

### 권장 2. “첫 촬영 성공”보다 “복구 후 다음 촬영 성공”을 더 중요하게 본다

이유:

현장 장애는 대부분 steady-state보다 recovery path에서 신뢰를 잃는다.

방법:

- reconnect 후 연속 2회 capture
- failed state 이후 새 session 또는 기존 session의 복구 경로 비교

### 권장 3. helper 건강성 증거를 남긴다

이유:

현재 repo는 sidecar protocol의 별도 문서 동결이 부족하다.

방법:

- helper 실행 방식
- helper restart 여부
- ready 복귀까지 걸린 시간
- 실패 시 표면 상태와 내부 관찰 메모

## darktable 적용 안정성을 높이기 위한 권장 검증법

### 권장 1. XMP 경로를 주 검증 경로로 고정한다

이유:

style 경로는 `data.db` 의존성이 커서 다중 인스턴스와 격리 검증이 더 복잡하다.

방법:

- published artifact의 `xmpTemplatePath` 근거 확보
- preview/final 모두 XMP 기반 적용 확인

### 권장 2. preview와 final을 별도 runtime mode로 취급한다

이유:

아키텍처와 reference 문서는 preview/final profile 분리를 허용한다.

방법:

- preview `configdir` / `library`
- final `configdir` / `library`
- style `configdir` / `library`

세 경로를 각각 기록한다.

### 권장 3. correctness와 latency를 따로 본다

이유:

OpenCL fallback은 correctness를 반드시 깨는 것이 아니다.

방법:

- GPU available 환경
- GPU unavailable 또는 fallback 환경

두 조건에서 같은 preset / 같은 RAW로 결과를 비교하고, 동시에 preview latency를 따로 기록한다.

### 권장 4. published artifact drift를 방지한다

이유:

same `presetId`라도 `publishedVersion`이 바뀌면 결과가 달라질 수 있다.

방법:

- 각 capture record의 `activePresetVersion`
- 해당 version의 `bundle.json`
- 비교 결과 이미지

세 가지를 한 묶음으로 보관한다.

## 출시 전 반드시 추가해야 할 검증 관점

### 관점 1. soak test

현재 체크리스트는 기능 확인에는 충분하지만, 장시간 운용 안정성은 별도 회차가 필요하다.

권장:

- 30 capture soak
- session end / restart 반복
- idle 후 복귀

### 관점 2. fault injection

render fail과 reconnect fail을 staging 장비에서 의도적으로 한 번은 만들어 봐야 한다.

권장:

- invalid or missing XMP reference
- unavailable render path
- helper restart during ready

### 관점 3. documentation gap closure

아키텍처가 기대한 아래 문서는 별도 문서화가 추가되면 좋다.

- sidecar protocol contract
- error envelope contract
- operator recovery playbook

지금도 구현은 가능하지만, 실검증 결과를 여러 사람이 재사용하려면 이 문서들이 있으면 더 안전하다.

## 현재 repo 기준의 문서 / 구조 갭

2026-03-26 기준으로 확인한 갭은 아래와 같다.

- `docs/contracts/`에는 session manifest, preset bundle, authoring publication 문서는 있으나, architecture가 기대한 sidecar protocol / error envelope 문서는 없다.
- architecture 예시 구조에는 `docs/architecture/`, `sidecar/` 등이 나오지만 현재 repo에는 해당 경로가 그대로 존재하지 않는다.
- 따라서 helper protocol과 일부 error semantics는 현재 코드와 테스트에서 사실상 읽어야 한다.

이 갭이 의미하는 것:

- 하드웨어 검증 시 evidence를 더 엄격히 남겨야 한다.
- 운영자나 후속 개발자가 문제를 재현하려면 문서보다 runtime evidence가 더 중요해진다.

## 실무 권장 우선순위

출시 직전 우선순위는 아래가 가장 합리적이다.

1. HV-00 환경 잠금
2. happy path single proof
3. unplug / reconnect
4. render fail isolation
5. active session publish / rollback drift
6. exact-end / post-end truth
7. 10회 smoke
8. 30회 soak

## 이 연구를 체크리스트에 반영하는 방법

체크리스트는 현장 실행용으로 유지하고, 아래 원칙을 항상 같이 적용한다.

- 각 항목은 하나의 UI screen이 아니라 하나의 truth transition을 증명해야 한다.
- 모든 pass는 evidence package를 남겨야 한다.
- 모든 fail은 “어느 경계가 깨졌는가”로 분류해야 한다.
- capture, render, timing, publication은 서로 다른 실패 축으로 기록해야 한다.

## 최종 판단 기준

아래 조건이 모두 맞아야 제품 관점에서 “카메라 연결 및 darktable 사진 편집 기능이 아키텍처에 맞게 검증되었다”고 볼 수 있다.

- 카메라 준비 전에는 절대 `Ready`가 나오지 않는다.
- RAW persistence 없이 capture success가 나오지 않는다.
- preview/final/post-end truth 없이 완료를 암시하지 않는다.
- render fail은 safe blocked state로 내려간다.
- active session binding은 publish/rollback에 흔들리지 않는다.
- customer copy는 끝까지 booth-safe다.
- operator는 same truth를 더 자세히 볼 뿐, 다른 truth를 만들지 않는다.
- repeated run과 idle/reconnect 이후에도 같은 규칙이 유지된다.

## 참고 메모

이 문서는 “실패가 절대 없다”는 보장을 만들지 않는다. 대신 아키텍처가 약속한 안전 경계가 실제 장비에서 유지되는지 증명하는 기준을 만든다. 제품 운영에서 중요한 것은 무오류 선언이 아니라, 실패가 나더라도 false-ready, false-complete, cross-session leak, preset drift 같은 더 큰 신뢰 붕괴로 번지지 않게 막는 것이다.

