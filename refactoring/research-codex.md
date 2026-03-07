# Boothy Greenfield Rebuild Research

작성일: 2026-03-07
작성자: Codex  
상태: Primary redesign basis for future greenfield work
문서 우선순위: `refactoring/research-codex.md`가 현재 기준 문서이며, `refactoring/research.md`는 superseded historical notes로만 본다.

입력 문서:

- `refactoring/research-redesign-workorder.md`
- `docs/prd-2026-03-07-boothy-greenfield-mvp.md`
- `docs/research-checklist-2026-03-07-boothy-greenfield.md`
- `reference/uxui_presetfunction/*`
- `reference/camerafunction/digiCamControl-2.0.0/*`

전제:

- RapidRAW 재사용 가능성과 Canon/EDSDK 배포 이슈는 곧 합법적으로 해결된다고 가정한다.
- 따라서 레퍼런스 채택 여부의 1차 기준은 법적 가능성보다 기술 적합성, 전달 속도, 유지보수 비용, 경계 설계 용이성이다.

## 0. Decision Lock

이 문서의 잠금 결론은 다음과 같다.

> **Most efficient path: `RapidRAW Host/UI selective reuse + new Canon-focused Camera Engine Boundary`**

이 문장이 의미하는 바는 다음과 같다.

- 제품의 핵심 가치는 Host/UI와 booth result pipeline 쪽에 남긴다.
- RapidRAW는 Host/UI 아이디어와 일부 코드 자산을 선택적으로 재사용하는 대상으로 본다.
- digiCamControl은 카메라 흐름과 Canon seam을 학습하는 연구 대상으로 본다.
- Canon 우선 지원 전략이라면 `Canon.Eos.Framework`, `CanonSDKBase`, `CameraControl.Devices.Example/Form1.cs`, `CameraControlCmd/Program.cs`를 1차 추출 후보로 본다.
- `CameraDeviceManager` 전체나 digiCamControl 전체 솔루션은 제품 베이스로 채택하지 않는다.

## 1. Product Alignment From the PRD

현재 PRD와 discovery checklist를 기준으로 보면, 고객과 운영자에게 가장 중요한 순간은 카메라 내부가 아니라 Host/UI 쪽에 몰려 있다.

- 고객은 촬영 시작 가능 여부를 즉시 이해해야 한다.
- 고객은 프리셋 선택, 최신 사진 확인, 진행 중 결과 누적, 종료 후 인계 흐름을 자연스럽게 경험해야 한다.
- 고객은 `내보내기 중`, `전화 필요`, `세션명 인계` 같은 결과 중심 상태만 이해하면 된다.
- 운영자는 복잡한 SDK 상태 전체가 아니라 `촬영 가능 / 준비 중 / 전화 필요` 같은 압축된 운영 신호와 최근 원인만 빠르게 파악하면 된다.

즉, 지금 제품에서 가장 값비싼 자산은 "카메라를 어떻게 직접 만지느냐"보다 "고객 여정과 운영 여정을 어떻게 안정적으로 보여주느냐"다.
그래서 가장 효율적인 재사용 대상은 레거시 카메라 허브가 아니라 Host/UI 쪽 구조와 흐름이다.

## 2. Most Efficient Path

### 2.1 Locked Recommendation

권고안은 단순하다.

- **Host/UI는 RapidRAW 계열에서 선택적으로 가져오고 줄인다.**
- **Camera 쪽은 새 boundary 뒤로 격리한다.**
- **Canon 흐름은 digiCamControl의 Canon seam에서 배우되, 솔루션 전체는 들고 오지 않는다.**

### 2.2 Why This Is More Efficient Than the Alternatives

| 후보 경로 | 왜 덜 효율적인가 | 판단 |
| --- | --- | --- |
| 현재 sidecar 안정화 중심 | 기존 React/Rust/C# 분리와 복구 heuristic를 중심 계획으로 유지한다 | Reject as primary path |
| Photino-first 또는 shell replacement | 셸을 다시 쓰는 비용을 먼저 지불하면서도 Camera Boundary 문제는 그대로 남긴다 | Demote |
| 전체 WPF/WinUI 네이티브 재작성 | PRD가 요구하는 Host/UI 자산과 웹 UI 강점을 버리고 다시 시작한다 | Reject |
| digiCamControl 전체 재사용 | WPF UI, .NET Framework 4.0 체인, 다기종 장치 허브까지 함께 떠안게 된다 | Reject |
| Host도 완전 신규 작성 | 가능은 하지만 이미 있는 Host/UI 자산을 버려 delivery 속도를 늦춘다 | Concern |
| RapidRAW Host selective reuse + 새 Camera Boundary | 가장 높은 가치 자산을 재사용하면서 카메라 위험을 격리한다 | Strong Recommend |

### 2.3 Why It Is Beginner-Friendly to Implement

이 경로는 구현 순서도 가장 단순하다.

1. Host-only pipeline부터 만든다.
   `Raw/` 감시, 안정화, ingest, preset 적용, 결과 표시, 종료 인계 흐름을 먼저 검증한다.
2. Canon spike를 별도로 만든다.
   `configureSession -> capture(requestId) -> transfer -> Raw arrival`만 증명한다.
3. 두 영역을 얇은 boundary로 연결한다.
   이때도 Host는 camera truth를 다시 계산하지 않는다.

즉, 제품의 핵심 경험을 먼저 만들고, 가장 불안정한 하드웨어 문제는 뒤에서 격리된 형태로 붙인다.

### 2.4 What "Independent Product" Means

이 문서에서 "독립 제품"은 "모든 것을 0부터 다시 짠다"는 뜻이 아니다.

- 새 제품 구조를 가진다.
- 새 Camera Boundary를 가진다.
- 자체 public contract를 가진다.
- 내부 구현에서만 선택적 재사용을 허용한다.
- Host/UI를 0부터 다시 쓰는 것을 목표로 삼지 않는다.

즉, 독립성은 소스 코드 출처보다 **경계와 계약의 주도권**에 관한 말이다.

## 3. Facts From the Local References

### 3.1 Reference A: `reference/uxui_presetfunction`

로컬 확인 결과:

- Boothy용으로 수정된 RapidRAW fork다.
- 스택은 `React 19 + Tauri 2 + Rust 2024`다.
- `src/App.tsx`는 약 3,707 lines다.
- `invoke()`와 `listen()` 호출이 `App.tsx`와 핵심 훅에 깊게 섞여 있다.
- 따라서 이 코드는 "중립 UI 라이브러리"가 아니라 "Tauri backend에 결합된 Host/editor 앱"에 가깝다.

이 레퍼런스에서 가치가 큰 것:

- 라이브러리/에디터 분리 UX
- 프리셋 선택, 썸네일 브라우징, 폴더 기반 세션 탐색
- 결과 중심의 화면 배치
- Host shell을 고객용/운영자용으로 나누기 쉬운 출발점
- 일부 Host/UI 코드의 선택적 재사용 가능성

이 레퍼런스에서 직접 가져오면 안 되는 것:

- Tauri `invoke/listen` 기반 contract
- 전체 RAW editor 기능 집합
- 거대한 단일 `App.tsx` 중심 구조
- Host와 editor를 통째로 포크하는 결정

결론:

> RapidRAW는 **전체 베이스 앱**이 아니라
> **Host/UI selective reuse 후보 + UX reference**다.

### 3.2 Reference B: `reference/camerafunction/digiCamControl-2.0.0`

로컬 확인 결과:

- 단일 camera engine이 아니라 여러 프로젝트 묶음이다.
- `PhotoBooth`는 WPF/MahApps.Metro 기반 UI다.
- `CameraControlCmd`와 `CameraControl.Devices.Example`에는 이미 `connect -> capture -> transfer -> session folder` 흐름이 드러나 있다.
- `Canon.Eos.Framework`와 `CameraControl.Devices/Canon/CanonSDKBase.cs`는 Canon 제어 seam에 더 가깝다.
- `CameraDeviceManager.cs`는 Canon뿐 아니라 Nikon, WIA, MTP, PTP-IP 등을 모두 다루는 장치 허브다.

이 레퍼런스에서 가치가 큰 것:

- 장치 연결, 캡처, 전송 흐름
- session folder와 filename 정책
- Canon wrapper 계층
- headless 제어 예시
- out-of-process camera service로 재구성할 수 있는 기능 단위

이 레퍼런스에서 직접 가져오면 안 되는 것:

- 전체 솔루션
- WPF `PhotoBooth` UI
- .NET Framework 4.0 체인 전체
- `CameraDeviceManager` 전체

결론:

> digiCamControl은 **제품 코어**가 아니라
> **Canon 흐름과 camera seam을 추출하기 위한 연구 대상**이다.

### 3.3 The Canon Extraction Seam, Concretely

다음 파일들이 Canon-focused extraction 전략의 근거다.

- `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Devices.Example/Form1.cs`
  - `CameraDeviceManager.PhotoCaptured` 이벤트를 받아 `TransferFile()`로 파일을 저장한다.
  - 가장 얇은 `capture -> photo event -> transfer` 연결 예제로 볼 수 있다.
- `reference/camerafunction/digiCamControl-2.0.0/CameraControlCmd/Program.cs`
  - UI 없이 세션, 파일명, 임시 파일, 전송 완료 후 저장까지 처리한다.
  - headless flow와 session-folder 중심 운영을 이해하기 좋다.
- `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Devices/Canon/CanonSDKBase.cs`
  - `CapturePhoto()`, `CapturePhotoNoAf()`, `TransferFile()`이 Canon wrapper 레벨에 모여 있다.
  - live view pause/resume와 transfer progress 같은 Canon-specific concern이 여기서 드러난다.
- `reference/camerafunction/digiCamControl-2.0.0/Canon.Eos.Framework/*`
  - 특히 `EosCamera.cs`는 session open, save-to-host, event handler registration, `TakePicture()`, shutter reset 같은 더 낮은 레벨의 Canon EDSDK seam을 보여준다.
- `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Devices/CameraDeviceManager.cs`
  - Canon, Nikon, WIA, MTP, PTP-IP 등 다수 장치 계층을 한 허브에 모은다.
  - 장치 탐색과 선택 허브로는 유용하지만, Canon 전용 최소 엔진의 직접 베이스로는 너무 넓다.

정리하면, 추출 포인트는 `CameraDeviceManager` 전체가 아니라 **Canon wrapper와 예제 흐름이 만나는 좁은 구간**이다.

## 4. Product Direction vs Prototype Path

### 4.1 Final Product Direction

최종 제품 방향은 아래 구조다.

```text
[Customer/Admin UI]
        |
        v
[Editor Host]
  - check-in
  - preset selection
  - latest-photo reassurance
  - exporting / handoff
  - operator diagnostics surface
        |
        v
[Camera Adapter Boundary]
        |
        v
[Camera Engine Process]
  - detect
  - capture
  - transfer
  - SDK/session recovery
        |
        v
[Session Filesystem: Raw/, Jpg/]
```

역할 분리는 다음과 같다.

- Editor Host
  - 세션 생성/선택
  - `Raw/` 감시와 파일 안정화
  - ingest / preset / render
  - 고객 흐름과 운영자 흐름 표시
  - PRD 기준의 `내보내기 중`, `세션명 인계`, `전화 필요` 화면 관리
- Camera Adapter Boundary
  - 최소 command/result/event contract 제공
  - diagnostics와 customer-visible state 분리
  - camera truth를 Host가 다시 추론하지 않게 차단
- Camera Engine
  - 장치 연결 감지
  - capture 요청 수락/거절
  - 파일 전송
  - SDK/session recovery
  - hardware truth 단독 소유

### 4.2 Short Spike Path

짧은 spike는 최종 제품 구조와 구분해서 본다.

- 1차 spike 목표는 `configureSession -> capture(requestId) -> Raw arrival`을 단일 Canon 모델에서 증명하는 것이다.
- 이때 `Form1.cs`와 `Program.cs`의 최소 흐름을 재현하거나, `CanonSDKBase`와 `EosCamera`를 얇게 감싼 throwaway 실험 코드를 쓸 수 있다.
- spike의 산출물은 "작동했다"보다 "어디까지가 Canon seam이고 어디부터가 제품 contract인가"를 분리해내는 것이다.

### 4.3 Temporary Learning Scaffolds

아래는 허용되는 임시 학습 발판이다.

- `Canon.Eos.Framework`를 직접 물고 동작 검증하기
- `CanonSDKBase` 주변을 얇게 잘라 capture/transfer를 확인하기
- `CameraControlCmd` 흐름을 따라 session folder 저장까지 headless로 검증하기

하지만 이것이 자동으로 최종 아키텍처를 의미하지는 않는다.
Spike foundation과 product architecture는 분리해서 판단해야 한다.

## 5. Architecture Guardrails

### 5.1 Truth Domains

- **camera truth**: Camera Engine only
- **process truth**: Adapter/Host backend
- **file pipeline truth**: Editor Host
- **customer-visible state**: UI translation layer

이 원칙은 다시 깨면 안 된다.
Host와 UI는 camera internals를 재판정하는 계층이 아니라, 이미 정규화된 신호를 번역하는 계층이다.

### 5.2 Success Semantics

성공은 세 단계로 나눈다.

1. `Command Accepted`
   - engine이 `capture(requestId)`를 수락했다.
   - 아직 셔터 동작, 파일 전송, 결과 표시는 보장하지 않는다.
2. `Raw Arrived`
   - 상관관계가 있는 새 파일이 `Raw/`에 안전하게 도착했다.
   - 파일 크기와 잠금 상태가 안정화되었다.
3. `Booth Result Ready`
   - ingest 성공
   - preset/filter 적용 성공
   - 고객에게 보여줄 결과가 준비됨

PRD 기준의 고객 경험 성공은 3단계다.
따라서 `capture accepted`를 곧바로 고객 성공으로 번역하면 안 된다.

### 5.3 Contract v0

최소 command:

- `configureSession({ sessionId, rawDir, jpgDir, namingPolicy })`
- `capture({ requestId })`
- `health()`
- `getDiagnostics()`
- `restart()`

최소 event:

- `engine_state_changed`
- `capture_request_accepted`
- `capture_request_rejected`
- `transfer_started`
- `transfer_completed`
- `camera_fault`

filesystem contract:

- engine은 전송 중 파일과 최종 파일을 구분할 수 있어야 한다.
- 가능하면 atomic rename 또는 그에 준하는 finalize 규칙을 둔다.
- Host는 파일 안정화 규칙을 통과한 뒤에만 ingest 한다.
- 가능하면 `requestId`를 manifest 또는 metadata로 남겨 correlation을 보조한다.

UI translation:

- 고객 상태는 작게 유지한다: `ready`, `capturing`, `importing`, `error`
- 운영자 표시는 PRD에 맞춰 `촬영 가능 / 준비 중 / 전화 필요` 같은 압축 신호로 번역할 수 있다.
- 그러나 그 뒤에는 더 풍부한 diagnostics 모델이 별도로 존재해야 한다.

## 6. Anti-Patterns and Non-Goals

다음은 명시적으로 금지한다.

- digiCamControl 전체 솔루션을 제품 베이스로 삼는 것
- `CameraDeviceManager` 전체를 Canon 엔진 베이스로 삼는 것
- Host가 Canon SDK 복잡도를 직접 흡수하는 것
- 옛 IPC 안정화를 다시 중심 계획으로 되살리는 것
- Photino-first 또는 HTTP-sidecar-first를 현재 주 경로로 놓는 것
- RapidRAW UI나 digiCamControl camera code를 거의 그대로 채택하는 것
- engine, host, UI가 camera 상태를 동시에 추론하는 것
- customer state와 admin diagnostics를 섞는 것

이 문서의 비목표도 분명하다.

- 제품 코드 구현
- 현재 앱 재작성
- 새 camera engine 실제 개발
- sidecar transport 현대화
- reference source code 수정

## 7. Reading Order for Future Work

다음 세션은 아래 순서로 읽는 것이 좋다.

1. `refactoring/research-redesign-workorder.md`
2. `refactoring/research-codex.md`
3. `refactoring/research.md`
4. `docs/prd-2026-03-07-boothy-greenfield-mvp.md`
5. `docs/research-checklist-2026-03-07-boothy-greenfield.md`
6. `reference/uxui_presetfunction/README.md`
7. `reference/uxui_presetfunction/src/App.tsx`
8. `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Devices.Example/Form1.cs`
9. `reference/camerafunction/digiCamControl-2.0.0/CameraControlCmd/Program.cs`
10. `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Devices/Canon/CanonSDKBase.cs`
11. `reference/camerafunction/digiCamControl-2.0.0/CameraControl.Devices/CameraDeviceManager.cs`
12. `reference/camerafunction/digiCamControl-2.0.0/Canon.Eos.Framework/EosCamera.cs`

중요:

- 현재 체크아웃에는 예전 문서가 언급하던 `apps/boothy/*`, `apps/camera-sidecar/*`, `work/*` 경로가 없다.
- 따라서 future LLM은 로컬 reference와 현재 PRD를 먼저 기준으로 잡고, 레거시 코드가 복원된 경우에만 비교 읽기를 추가해야 한다.

## 8. Final Conclusion

Boothy의 현재 greenfield 재설계는 "현재 sidecar를 어떻게 고칠까"가 아니라, "제품 가치가 있는 Host/UI를 얼마나 빨리 살리고 camera risk를 얼마나 작게 격리할까"의 문제다.

최종 권고는 바뀌지 않는다.

> **새 제품은 `RapidRAW Host/UI selective reuse + new Canon-focused Camera Engine Boundary`를 기본축으로 삼아야 한다.**

그리고 이 문장을 해석할 때는 아래 두 가지를 같이 기억해야 한다.

- RapidRAW는 제품 가치를 빠르게 회복하는 Host/UI donor다.
- digiCamControl은 Canon seam을 배우는 extraction reference다.

즉, 앞으로의 설계는 "무엇을 통째로 가져올까"가 아니라, "어떤 경계와 계약 아래에서 무엇을 작게 재사용할까"를 기준으로 진행해야 한다.
