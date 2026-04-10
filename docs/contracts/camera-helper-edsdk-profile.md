# Canon EDSDK Helper Implementation Profile

## 목적

이 문서는 `docs/contracts/camera-helper-sidecar-protocol.md`의 generic helper 계약 위에,
Boothy가 현재 채택한 **Windows 전용 Canon EDSDK helper exe** 구현 기준선을 덧씌우는
구체 프로파일 문서다.

generic protocol이 host-helper 경계의 최소 메시지 의미를 고정한다면,
이 문서는 현재 제품이 실제로 만들 helper의 런타임, 패키징, 책임 분리, 진단 기준을 고정한다.

## 문서 관계

- cross-boundary 메시지 의미와 canonical contract: `docs/contracts/camera-helper-sidecar-protocol.md`
- 현재 채택 방향과 비교 검토 근거: `_bmad-output/planning-artifacts/research/technical-canon-camera-helper-research-20260328.md`
- operator projection 후속 맥락: `_bmad-output/implementation-artifacts/5-4-운영자용-카메라-연결-상태-전용-항목과-helper-readiness-가시화.md`
- hardware evidence 맥락: `docs/runbooks/booth-hardware-validation-checklist.md`

이 문서는 새로운 제품 요구사항을 만드는 문서가 아니라,
채택된 helper 방향을 구현 가능한 수준으로 더 구체화하는 문서다.

## Canon 공개 기준선

2026-03-28 기준 Canon 공개 CAP 문서에서 확인 가능한 사실은 아래와 같다.

- EDSDK는 Canon의 **USB wired control** 경로다.
- CAP 공개 페이지는 EDSDK 지원 OS로 Windows 10/11, macOS, Linux를 함께 소개한다.
- Canon 공개 페이지는 EDSDK 주요 기능으로 remote shooting, live view monitor, image transfer, camera settings를 설명한다.
- Canon 공개 페이지는 Windows sample program 언어로 `VB`, `C++`, `C#`를 적는다.
- Canon 공개 release note는 2025-09-24 기준 `Ver.13.20.10`을 표기한다.
- 같은 release note는 2021-03-31 `Ver.13.13.20`에서 Windows 7 지원 종료를 적는다.
- 같은 release note는 2018-12-13 `Ver.13.9.10`에서 `CSharp` sample 추가를 적는다.

Boothy에 대한 구현 해석은 아래와 같다.

- **추론:** Canon 공개 기준만 보면 Linux도 가능하지만, Boothy는 승인된 booth PC와 운영 복잡도 축소를 위해 **Windows 전용**으로 고정하는 편이 맞다.
- **추론:** Canon 공개 CAP가 Windows에서 `C#` sample을 인정하므로, helper를 C# self-contained exe로 두는 선택은 제품 운영 관점에서 자연스럽다.

## 제품 고정 결정

- helper 런타임은 **Windows 전용**이다.
- booth 현장 표준은 Windows 10/11 **x64**를 기본으로 본다.
- helper 프로세스 이름은 `canon-helper.exe`를 기준으로 한다.
- helper target framework는 `.NET 8 / net8.0` baseline으로 고정한다.
- helper는 app instance당 **하나의 장수명 프로세스**로 두는 것을 기본으로 한다.
- booth runtime은 **동시에 1대의 활성 카메라**만 소유하는 것을 기본으로 한다.
- helper는 **동시에 1개의 in-flight capture**만 허용하는 것을 기본으로 한다.
- 고객/운영자 UI는 helper raw 상태를 직접 보지 않는다.
- darktable render worker와 camera helper는 계속 **별개 프로세스 경계**다.

## 권장 패키징 기준

- 소스 기준 경계는 `sidecar/canon-helper/`다.
- 배포 기준으로는 `canon-helper.exe`와 Canon EDSDK DLL들이 **같은 sidecar 경계**에 함께 배치되어야 한다.
- helper가 의존하는 Canon SDK payload는 공개 저장소에 무심코 커밋하지 않는다.
- **추론:** Canon SDK zip, DLL, header는 라이선스 검토가 끝난 private build input 또는 승인된 내부 artifact로 다루는 편이 안전하다.
- 현장 booth PC가 Java/Node/Python 런타임을 추가로 요구하지 않도록 유지한다.
- helper는 시작 시점에 최소 아래 정보를 로그 또는 `helper-ready` 진단 정보로 남길 수 있어야 한다.
  - helper version
  - protocol version
  - runtime platform
  - sdk family = `canon-edsdk`
  - sdk package version 또는 helper가 링크한 SDK 기준 버전

## 책임 분리

### helper가 소유하는 것

- Canon EDSDK initialize / terminate
- 카메라 탐지와 선택
- Canon session open / close
- Canon 이벤트 수신 루프 또는 동등 감시 경계
- 촬영 요청 수락/거절
- RAW 다운로드와 파일 handoff
- USB 분리/재연결 감지
- bounded helper diagnostics 생성
- `camera-status`, `file-arrived`, `recovery-status`, `helper-error` 송신

### Tauri host가 소유하는 것

- helper spawn / stop / health timeout
- runtime root, diagnostics path, session binding 전달
- freshness 판정
- active session / active preset correlation
- booth `Ready`와 operator `카메라 연결 상태` 정규화
- 파일 존재, session 경계, capture success 최종 확정
- audit / recovery policy / UI projection

### React가 소유하지 않는 것

- Canon SDK 상태 해석
- helper recovery decision
- capture success 확정
- camera truth와 booth truth의 직접 결합

## helper 내부 상태 기계 권장안

helper 내부의 더 세밀한 상태는 구현 자유를 허용하되,
host로 보낼 raw 상태 의미는 아래 집합에 안정적으로 접혀야 한다.

| helper 내부 phase | host에 보이는 raw 상태 | 의미 |
| --- | --- | --- |
| booting | `connecting` | 프로세스 시작 직후, diagnostics 경계와 SDK 진입 준비 중 |
| sdk-initializing | `connecting` | EDSDK initialize 및 런타임 검증 중 |
| scanning | `connecting` | 카메라 탐지 또는 첫 연결 대기 중 |
| session-opening | `connecting` | 카메라를 찾았고 session open 시도 중 |
| connected-idle | `connected-idle` | 장비는 연결됐지만 아직 촬영 readiness를 열기 전 |
| ready | `ready` | 촬영 시작 가능한 상태 |
| capture-triggered | `capturing` | shutter trigger 수락 후 in-flight capture 진행 중 |
| download-in-progress | `capturing` | 파일 다운로드/정리 중이라 추가 촬영 금지 |
| recovering | `recovering` | 재연결, SDK 재초기화, session reopen 중 |
| degraded | `degraded` | previously-ready 이후 false-ready를 막기 위해 강제 차단된 상태 |
| fatal-error | `error` | 승인된 recovery path 밖의 실패 |

핵심은 helper 내부 phase가 더 많아도,
host에 보이는 메시지 vocabulary는 계속 bounded하게 유지하는 것이다.

## 권장 부팅 시퀀스

1. Tauri host가 `canon-helper.exe`를 시작한다.
2. helper는 diagnostics path 준비, runtime self-check, SDK payload 확인을 수행한다.
3. helper는 `helper-ready`를 보낼 수 있지만, 이 메시지는 **camera ready**를 뜻하지 않는다.
4. helper는 Canon SDK initialize 후 카메라 탐지를 시작한다.
5. 카메라가 발견되면 session open을 시도하고, 결과를 `camera-status`로 보낸다.
6. host는 first fresh `camera-status`를 받기 전까지 booth `Ready`를 만들지 않는다.

권장 원칙:

- `helper-ready`는 "helper process가 protocol conversation을 시작할 수 있다"는 뜻이다.
- `camera-status=ready`는 "카메라가 촬영 경계까지 준비됐다"는 뜻이다.
- 두 신호는 반드시 분리한다.

## capture / download 시퀀스 권장안

1. host가 `request-capture`를 보낸다.
2. helper는 session 바인딩, in-flight 여부, camera state를 보고 수락/거절한다.
3. helper는 host가 보낸 `requestId`를 그대로 담아 `capture-accepted`를 보낸다.
4. helper는 Canon capture trigger를 호출한다.
5. helper는 object/file arrival 이벤트를 기다린다.
6. helper는 다운로드를 **임시 경로**에 먼저 기록한다.
7. 파일 close가 끝난 뒤 session-scoped 최종 경로로 이동 또는 rename 한다.
8. helper는 최종 경로가 준비된 뒤에만 helper-owned `captureId`와 함께 `file-arrived`를 보낸다.
9. host는 파일 존재와 correlation을 확인한 뒤 capture success를 확정한다.

권장 원칙:

- `capture-accepted`는 success가 아니다.
- `shutter command 성공`도 success가 아니다.
- `file-arrived`는 최종 경로가 닫힌 뒤에만 나가야 한다.
- host는 `captures/originals/` 아래 active session root에 실제 파일이 존재하고 비어 있지 않을 때만 success를 확정해야 한다.
- host는 여전히 실제 파일 존재를 다시 확인해야 한다.

## recovery 시퀀스 권장안

- USB 분리, 카메라 전원 off, session loss는 즉시 `recovering` 또는 `degraded`로 내려간다.
- once-ready 이후 연결이 흔들리면 stale `ready`를 유지하지 않는다.
- 재연결 직후에도 새 `camera-status` freshness가 닫히기 전까지 `ready`를 주장하지 않는다.
- helper 재시작이나 SDK 재초기화는 host가 승인한 `request-recovery` 경로로만 수행한다.
- 반복 실패가 누적되면 `helper-error`와 bounded operator action으로 승격한다.

## detailCode 권장 어휘

아래 detail code는 예시지만, 구현 초기에 이 수준으로 vocabulary를 좁히는 편이 좋다.

- `camera-not-found`
- `sdk-payload-missing`
- `sdk-init-failed`
- `session-opening`
- `session-opened`
- `camera-ready`
- `capture-in-flight`
- `download-in-progress`
- `camera-busy`
- `degraded-after-ready`
- `usb-disconnected`
- `reconnect-pending`
- `session-mismatch`
- `unsupported-camera`
- `recovery-restart-sdk`
- `recovery-reopen-session`

이 값들은 helper/operator diagnostics용 machine vocabulary다.
customer copy로 직접 내려가지 않는다.

## operator projection 매핑 힌트

Story 5.4 기준으로 operator용 `카메라 연결 상태`는 helper raw 어휘를 직접 보여 주지 않고,
아래처럼 접는 편이 안전하다.

- `camera-not-found`, `usb-disconnected`, `unsupported-camera` -> `미연결`
- `sdk-initializing`, `session-opening`, first fresh truth 대기 -> `연결 중`
- `connected-idle`, `camera-ready` -> `연결됨`
- `reconnect-pending`, `sdk-init-failed`, `degraded-after-ready`, 반복 recovery -> `복구 필요`

이 매핑의 최종 권위는 host normalization에 있다.
React는 raw detailCode를 보고 독자적으로 최종 상태를 만들면 안 된다.

## 구현 우선순위

1. `helper-ready`
2. `camera-status`
3. `request-capture` -> `capture-accepted`
4. `file-arrived`
5. `request-recovery` -> `recovery-status`
6. `helper-error`

초기 MVP helper는 아래 기능만 먼저 닫아도 된다.

- 카메라 1대 탐지
- session open / close
- 촬영 트리거
- RAW 다운로드
- 분리/재연결 감지
- freshness 가능한 status 송신

초기 MVP helper에서 굳이 먼저 하지 않아도 되는 것:

- 멀티카메라 동시 제어
- customer-facing live view UI
- 광범위한 카메라 설정 편집
- 네트워크 기반 camera control
- darktable와의 직접 결합

## 운영/검증에서 꼭 남길 것

- helper version
- sdk package version 또는 링크 기준 버전
- camera model
- helper-ready 시각
- last fresh camera-status 시각과 sequence
- capture request / file-arrived correlation
- recovery 진입/종료 시각

durable example baseline:

- `sidecar/protocol/examples/helper-ready.json`
- `sidecar/protocol/examples/camera-status.json`
- `sidecar/protocol/examples/file-arrived.json`
- `sidecar/protocol/examples/recovery-status.json`
- `sidecar/protocol/examples/helper-error.json`

HV-00, HV-03, HV-10 evidence에는 최소한 위 항목 일부가 남아야,
문제가 생겼을 때 false-ready와 helper fault를 분리해서 볼 수 있다.

## 권장 개발 편의 기능

아래는 제품 요구사항은 아니지만 개발 생산성에 큰 도움이 된다.

- `canon-helper.exe --version`
- `canon-helper.exe --self-check`
- helper diagnostics를 지정 경로에 평문 또는 JSON line으로 남기는 옵션
- mock camera 또는 fixture status를 내보내는 개발용 profile

단, booth production profile에서는 fixture path가 실제 `Ready`를 합성하지 못하게 해야 한다.

## 비목표

- 범용 Canon 관리 앱 만들기
- booth UI에서 live view나 camera settings를 직접 노출하기
- React가 helper stderr/stdout를 직접 읽게 하기
- helper가 preset, timing, completion truth까지 소유하기
- Java/Node/Python 런타임을 booth PC 필수 의존성으로 만들기
- 멀티카메라와 다중 booth를 같은 helper 프로세스에서 동시에 오케스트레이션하기

## Source Links

- Generic sidecar contract: `docs/contracts/camera-helper-sidecar-protocol.md`
- Canon helper research: `_bmad-output/planning-artifacts/research/technical-canon-camera-helper-research-20260328.md`
- Hardware validation checklist: `docs/runbooks/booth-hardware-validation-checklist.md`
- Canon CAP overview: https://asia.canon/en/campaign/developerresources/camera/cap
- Canon Digital Camera SDK overview: https://asia.canon/en/campaign/developerresources/camera/digital-camera-software-development-kit
- Canon EDSDK release note: https://asia.canon/en/campaign/developerresources/camera/cap/edsdk-eos-digital-camera-sdk-release-note
