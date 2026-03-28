---
stepsCompleted:
  - 1
  - 2
  - 3
  - 4
  - 5
  - 6
inputDocuments:
  - docs/contracts/camera-helper-sidecar-protocol.md
  - docs/runbooks/booth-hardware-validation-checklist.md
  - docs/runbooks/booth-hardware-validation-architecture-research.md
workflowType: research
lastStep: 6
research_type: technical
research_topic: Canon EOS 700D Windows booth camera helper architecture
research_goals: 현재 프로젝트 구조와 Windows 부스 PC 환경에 맞춰 Canon EOS 700D 제어 방식을 재검증하고, 운영 안정성과 구현 속도를 함께 고려한 기술 방향을 선택한다.
user_name: Noah Lee
date: 2026-03-28
web_research_enabled: true
source_verification: true
---

# Research Report: technical

**Date:** 2026-03-28
**Author:** Noah Lee
**Research Type:** technical

---

## Research Overview

이번 검토는 Boothy의 현재 sidecar 계약과 Windows 부스 운영 전제를 기준으로, Canon EOS 700D를 실카메라 경계에 연결하는 가장 현실적인 기술 방향을 다시 검증하는 데 목적이 있다.

검증 기준은 아래 다섯 가지로 고정했다.

1. Windows 부스 운영 적합성
2. Canon EOS 700D 실제 지원 여부
3. capture + 다운로드 + live view 가능 범위
4. USB 재연결/복구 시 운영 리스크
5. Boothy의 helper sidecar 구조 적합성

핵심 결론은 다음과 같다.

- **최종 채택 방향:** Canon **EDSDK 기반 전용 Windows helper exe**
- **빠른 검증용 차선책:** **digiCamControl 기반 helper 프로토타입**
- **비교/참고 후보:** **libgphoto2/gphoto2**
- **제외:** **CCAPI**

## 로컬 아키텍처 적합성

Boothy는 이미 카메라를 앱 내부가 아니라 **외부 helper 경계**로 다루도록 설계돼 있다.

- `docs/contracts/camera-helper-sidecar-protocol.md`는 `sidecar/canon-helper/`와 Tauri host의 분리를 전제로 한다.
- helper는 `camera-status`, `file-arrived`, `recovery-status`를 JSON Lines로 넘기고, host가 최종 readiness truth를 정규화한다.
- `docs/runbooks/booth-hardware-validation-checklist.md`도 `camera helper`와 render worker를 **별도 프로세스 경계**로 운영하라고 고정한다.

따라서 이번 선택은 "어떤 카메라 기술을 쓰느냐"보다, "**어떤 helper를 sidecar 계약 안에 넣느냐**"의 문제다.

이 관점에서 보면 앱 내부 직접 연동보다 외부 helper exe가 구조적으로 맞다.

## 외부 검증 요약

### 1. Canon EDSDK

Canon 공식 SDK 목록 기준으로 2026-03-28 현재:

- **ED-SDK 최신 표기:** `V13.20.10`
- **공개 CAP 페이지 기준 지원 OS:** Windows 10/11, macOS, Linux
- **EOS 700D가 지원 모델 목록에 포함**
- Canon 설명상 카메라 설정, 촬영 제어, 이미지 전송 API를 제공
- Canon 공개 CAP는 remote shooting, live view monitor, image transfer, camera settings를 주요 기능으로 제시한다.
- Canon 공개 CAP는 Windows sample program 언어로 `VB`, `C++`, `C#`를 함께 제시한다.

또한 Canon 공식 release note 기준:

- **2025-09-24:** `Ver.13.20.10`

해석:

- 700D 같은 구형 EOS DSLR을 Windows 호스트에서 제어하는 공식 경로는 여전히 EDSDK다.
- 700D는 CCAPI 세대가 아니라 USB host-PC 제어 세대에 가깝기 때문에, 운영 안정성 관점에서는 공식 SDK가 가장 자연스럽다.

### 2. Canon CCAPI

Canon 공식 SDK 목록 기준으로 2026-03-28 현재:

- CCAPI는 HTTP/Wi-Fi 기반으로 live view, 촬영, 이미지 조회를 제공한다.
- 하지만 **지원 모델 목록에 EOS 700D는 없다**.
- 지원 모델은 EOS R 계열, EOS RP, 일부 신형 DSLR/미러리스 중심이다.

결론:

- **700D에는 CCAPI를 선택지로 두면 안 된다.**

### 3. digiCamControl

digiCamControl 공식 문서 기준:

- **Windows용 tethered shooting 솔루션**
- 지원 카메라 목록에 **Canon EOS 700D 포함**
- `CameraControlRemoteCmd.exe`로 **CLI capture** 가능
- live view 창 제어와 **LiveView capture** 예시 제공
- `CameraControl.Devices` **standalone library** 제공
- 연결/분리 이벤트와 `CapturePhoto()` 같은 라이브러리 API 제공

추가로 GitHub 기준:

- 저장소는 2025-05-26까지 push 이력이 있고, 2026-03-11까지 업데이트 메타데이터가 갱신되어 있다.
- 다만 GitHub Releases는 오래된 편이라, 배포 채널/문서 최신성이 일정하지 않다.
- Canon 관련 이슈에서 파일 전송 문제, 멀티카메라 handle 문제, 재시작 후 정상화 같은 사례가 관찰된다.

해석:

- **프로토타입 속도는 매우 좋다.**
- 하지만 장기 운영에서 helper의 authoritative source로 삼기에는, Canon SDK 위에 한 단계 더 감싼 구조라 진단/복구 제어력이 낮다.

### 4. libgphoto2 / gphoto2

gPhoto 지원 목록과 문서 기준:

- Canon EOS 700D는 **Image Capture / Trigger Capture / Liveview / Configuration** 지원 대상으로 보인다.
- 원격 제어 문서에는 Canon EOS 계열의 `eosremoterelease`, `wait-event-and-download` 기반 촬영 흐름이 보인다.
- 별도 카메라 목록 미러에는 Canon EOS 700D가 **capture support = Yes**, **Liveview = Yes**, 각종 설정 제어 가능으로 표시된다.

다만 photobooth 오픈소스 사례를 보면:

- **Windows에서는 digiCamControl**
- **Linux에서는 gphoto2**

로 분기한다.

해석:

- 기능 가능성 자체는 충분하다.
- 하지만 Windows 부스 PC에서 Canon DSLR을 상시 운영하는 주 경로로는 생태계 무게중심이 상대적으로 약하다.
- Boothy의 주 운영 환경이 Windows라는 점을 고려하면 1순위로 올리기 어렵다.

## 선택지별 평가

### A. Canon EDSDK 기반 전용 helper

장점:

- Canon 공식 지원 경로다.
- 700D 지원이 공식 목록에 있다.
- 촬영 제어, 설정 제어, 이미지 전송 범위가 명확하다.
- helper가 카메라 상태를 직접 소유하므로 `camera-status` / `recovering` / `degraded` 같은 sidecar 상태 모델과 잘 맞는다.
- 장애 원인을 Boothy 운영 진단 관점으로 다시 포장하기 쉽다.

단점:

- SDK가 오픈소스가 아니다.
- Canon 개발자 리소스 신청/다운로드 경로가 필요하다.
- Windows 배포 시 Canon DLL 포함 규칙을 확인해야 한다.

판정:

- **운영 안정성 1위**
- **Boothy 최종 구조 적합성 1위**

### B. digiCamControl 기반 helper

장점:

- 붙여보기 가장 빠르다.
- CLI와 library가 모두 있어 실험 비용이 낮다.
- 700D 지원 문서가 명시돼 있다.
- Windows 사례가 많다.

단점:

- Canon EDSDK 위에 다시 추상화가 올라간 형태라, Boothy가 원하는 bounded diagnostics를 세밀하게 만들기 어렵다.
- Canon 관련 파일 전송/멀티카메라/복구성 이슈가 공개 이력에 있다.
- 최종 authoritative helper로 삼기보다는 prototype adapter에 가깝다.

판정:

- **빠른 검증 1위**
- **최종 제품 구조 2위**

### C. libgphoto2 / gphoto2

장점:

- 오픈소스다.
- 700D에서 capture/liveview/configuration 가능성이 확인된다.
- Linux, SBC, CLI 자동화 생태계가 좋다.

단점:

- Windows 주력 운영 레일이 아니다.
- Canon Windows 부스 운영 기준으로는 문제 분석과 복구 경험치를 쌓기가 상대적으로 불리하다.
- 제품 운영팀 입장에서 장애를 설명하고 좁히는 데 경로가 길어진다.

판정:

- **비교 연구용으로 유효**
- **현재 Boothy의 Windows 기준 최종 후보로는 비권장**

### D. CCAPI

판정:

- **EOS 700D 미지원**
- **현 시점 제외**

## 최종 기술 방향

### 선택

**Boothy의 최종 카메라 기술 방향은 `Windows 전용 Canon EDSDK helper exe + Tauri sidecar contract`로 선택하는 것이 가장 적절하다.**

### 이유

1. 현재 제품 문서가 이미 helper boundary를 전제로 설계돼 있다.
2. Canon EOS 700D는 공식 EDSDK 지원 목록에 있다.
3. 700D는 CCAPI 세대가 아니라 USB host-PC 제어가 더 자연스러운 모델이다.
4. 운영 리스크의 핵심은 촬영 기능 수보다 **재연결/회복/false-ready 방지**인데, 이 부분은 공식 SDK 직접 제어가 유리하다.
5. helper 상태를 `disconnected/connecting/ready/capturing/recovering/degraded/error`로 모델링하기에 EDSDK 직접 제어가 가장 깔끔하다.

## 구현 권장안

### 권장 아키텍처

`Boothy(Tauri/Rust host) -> canon-helper.exe -> Canon EOS 700D`

helper 책임:

- 카메라 탐지/세션 open
- live 상태 감시
- 촬영 요청 수락/거절
- 파일 다운로드 완료 확인
- 연결 끊김/재연결 감지
- host 계약에 맞춘 bounded 상태 이벤트 송신

host 책임:

- freshness 검사
- active session/preset 바인딩 확인
- booth/operator DTO 정규화
- false-ready 차단

### 구현 기술 권장

초기 판단에서는 `digiCamControl -> 전용 helper` 2단계 접근이 현실적이라고 보였다.

하지만 이후 Boothy의 실제 요구 범위를 다시 확인한 결과, 현재 제품은 카메라 쪽에서 많은 기능을 요구하지 않는다. 핵심은 아래다.

- 연결/미연결/복구 상태
- 촬영 가능 여부
- 촬영 트리거
- 파일 다운로드 완료
- stale status 방지
- 필요 시 소수의 설정값 읽기/쓰기

따라서 **현재 범위만 기준으로 하면 `digiCamControl`을 거치지 않고 바로 전용 EDSDK helper로 가는 것이 더 단순하고 더 맞다.**

정리:

1. **최종 제품 기본안:** Canon EDSDK 직접 붙인 전용 helper
2. **예외적 빠른 검증안:** 정말 하드웨어 smoke만 빨리 보고 싶을 때만 digiCamControl spike 사용

이 판단이 바뀐 이유:

- Boothy는 카메라를 "풍부하게 제어"하는 제품이 아니라, **truthful readiness와 capture boundary를 안정적으로 소유**하는 제품이다.
- 지금 필요한 기능은 적기 때문에 큰 도구를 우회해서 쓰는 편이 오히려 무겁다.
- 공식 EDSDK를 이미 확보한 상태라면, 필요한 기능만 얇게 감싼 helper가 구현 복잡도와 운영 복잡도 모두에서 유리하다.

## 언어/런타임 선택 의견

### 권장

**전용 helper는 C#/.NET 또는 C++ 중 하나로 구현하되, 실무적으로는 C# self-contained exe가 가장 빠르다.**

근거:

- digiCamControl과 EDSDK.NET 같은 참고 자산이 모두 C# 진영에 있다.
- Windows 전용 helper exe로 묶기 쉽다.
- 연결/분리 이벤트, 로그, 상태 머신 구현 속도가 빠르다.

주의:

- `EDSDK.NET` 저장소는 Canon 최신 SDK와 동기화돼 있지 않으므로 **참고용**으로만 보고, 최종 의존성은 Canon 최신 EDSDK 기준으로 직접 맞추는 편이 안전하다.

추론:

- 제품 관점에서 가장 중요한 것은 helper의 구현 언어보다, Canon SDK 상태를 host 계약으로 안정적으로 재포장하는 것이다.

## 운영 리스크 메모

### EDSDK helper에서 꼭 먼저 잠가야 할 것

- 카메라 전원 on/off
- USB 케이블 분리/재연결
- 세션 open 직후 첫 ready 지연
- 촬영 요청 중 중복 캡처 차단
- RAW 다운로드 완료 전 success 확정 금지
- reconnect 후 stale ready 차단

### digiCamControl prototype에서 예상해야 할 것

- 일부 Canon 전송 경계에서 temp file/transfer 이슈 가능성
- 복수 장치 또는 재시작 시 handle 문제 가능성
- CLI 성공과 product truth 확정 시점을 분리해야 함

## 범위 재검토 추가 메모

2026-03-28 추가 검토에서 로컬 계약과 호스트 코드를 다시 확인했다.

핵심 확인 사항:

- `docs/contracts/camera-helper-sidecar-protocol.md`는 helper에 풍부한 카메라 UI가 아니라 `camera-status`, `capture`, `file-arrived`, `recovery-status` 수준의 bounded contract를 요구한다.
- `src-tauri/src/capture/normalized_state.rs`는 helper가 제공한 상태를 booth/operator truth로 정규화하는 역할이 중심이며, 복잡한 카메라 기능 노출이 중심이 아니다.
- PRD와 shared contracts는 고객에게 필요한 최종 결과를 `Preparing`, `Ready`, `Preview Waiting`, `Export Waiting`, `Phone Required` 같은 제품 상태로 한정한다.

따라서 제품이 정말 원하는 것은 아래 두 가지다.

1. 카메라를 많이 제어하는 것
2. 카메라 상태를 거짓 없이 안정적으로 소유하는 것

이 중 Boothy는 명확히 **2번**에 더 가깝다.

이 점 때문에 오픈소스 완성형 도구보다, **필요한 기능만 가진 얇은 helper가 제품 적합성이 더 높다.**

## 구현 구체화 보강 메모

2026-03-28 추가 검증에서 Canon 공개 CAP 페이지를 다시 보면,
Boothy에 중요한 구현 shaping point는 아래처럼 정리된다.

### 1. EDSDK는 여전히 USB-wired helper 경계에 잘 맞는다

Canon 공개 CAP는 EDSDK를 USB wired control 경로로 설명한다.
이 점은 Boothy의 `Tauri host -> sidecar helper -> Canon camera` 구조와 잘 맞는다.

해석:

- 카메라 연결과 이벤트 처리는 helper가 소유하고
- host는 freshness와 product truth 정규화만 소유하는 분리가 자연스럽다.

### 2. 공식 공개 기능 범위가 현재 제품 요구보다 넓다

Canon 공개 CAP는 remote shooting, live view, image transfer, camera settings를 모두 언급한다.
하지만 Boothy의 현재 핵심 요구는 그 전체가 아니라 아래다.

- 연결/미연결/복구 상태
- 촬영 가능 여부
- 촬영 트리거
- RAW 다운로드 완료
- stale status 차단

해석:

- helper는 "풍부한 카메라 앱"이 아니라
- **필요한 기능만 가진 얇은 상태 머신**으로 두는 편이 맞다.

### 3. C# helper 선택은 공식 공개 생태계와도 충돌하지 않는다

Canon 공개 CAP는 Windows sample program 언어로 `VB`, `C++`, `C#`를 함께 제시하고,
release note는 2018-12-13 `Ver.13.9.10`에서 `CSharp` sample 추가를 적는다.

해석:

- Boothy가 C# self-contained exe를 helper 기본안으로 보는 것은 무리한 우회가 아니다.
- 다만 최종 의존성은 unofficial wrapper보다 Canon 최신 EDSDK 기준으로 직접 맞추는 편이 안전하다.

### 4. 공개 OS 범위와 제품 선택은 분리해서 봐야 한다

Canon 공개 CAP는 현재 Linux도 지원 OS에 넣는다.
하지만 Boothy는 승인된 booth PC, Tauri packaging, 현장 운영 단순화를 기준으로 여전히 Windows 전용 helper를 택하는 편이 맞다.

이 문장은 Canon 제약이 아니라 **제품 운영 선택**에 대한 해석이다.

## 지금 시점의 추천 의사결정

### 제품 결정

- **최종 채택:** Canon EDSDK 기반 전용 helper

### 실행 결정

- **기본 실행안:** 바로 Canon EDSDK 전용 helper 설계 및 구현 시작
- **보조 실행안:** 하드웨어 smoke나 USB 특성 확인만 매우 빨리 보고 싶을 때만 digiCamControl spike 사용

### 보류 결정

- CCAPI
- Windows 주력 경로로서의 gphoto2
- digiCamControl UI 자체 포크
- Java/Node/Python 런타임을 helper 기본 런타임으로 채택하는 결정

## 다음 액션 제안

1. `sidecar/canon-helper/`를 별도 Windows helper 프로젝트로 만든다.
2. Canon에서 받은 EDSDK DLL/헤더를 helper 빌드 입력으로 정리한다.
3. 먼저 아래 최소 명령/이벤트만 구현한다.
   - `helper-ready`
   - `camera-status`
   - `request-capture`
   - `file-arrived`
   - `request-recovery`
4. helper 내부 최소 책임을 아래로 제한한다.
   - 카메라 탐지
   - 세션 open/close
   - 촬영 트리거
   - host 저장 대상으로 파일 다운로드
   - 연결 끊김/복구 상태 송신
   - 필요 시 ISO/조리개 같은 소수 속성 읽기/쓰기
5. host 쪽에서는 freshness 기반 false-ready 차단을 먼저 붙인다.
6. 그다음 unplug/reconnect/idle 복귀를 실장비 기준으로 검증한다.

## 초보자용 실행 해석

이 문서에서 말하는 "전용 EDSDK helper 구현"은 새 카메라 앱을 크게 만드는 뜻이 아니다.

의미:

- Canon에서 받은 공식 EDSDK를 이용해
- Boothy 전용 작은 `camera-helper.exe`를 만들고
- Boothy는 그 exe와만 통신한다

이 helper가 하는 일은 아래 정도로 매우 제한된다.

- "카메라 연결됨/안 됨" 알려주기
- "지금 촬영 가능함/아직 안 됨" 알려주기
- "사진 찍어" 요청 받기
- 실제 사진 파일 다운로드하기
- "파일 도착했음" 알려주기
- 선이 뽑히면 "복구 중" 알려주기

즉, 남이 만든 큰 앱을 제품 안에 끼워 넣는 것이 아니라, **Boothy가 필요한 작은 기능만 가진 카메라 전용 백그라운드 프로그램을 만드는 것**이다.

## Source Links

- Boothy sidecar contract: `docs/contracts/camera-helper-sidecar-protocol.md`
- Boothy EDSDK helper profile: `docs/contracts/camera-helper-edsdk-profile.md`
- Boothy hardware validation checklist: `docs/runbooks/booth-hardware-validation-checklist.md`
- Canon CAP overview: https://asia.canon/en/campaign/developerresources/camera/cap
- Canon SDK list: https://asia.canon/en/campaign/developerresources/sdk
- Canon EDSDK release note: https://asia.canon/en/campaign/developerresources/camera/cap/edsdk-eos-digital-camera-sdk-release-note
- digiCamControl supported cameras: https://digicamcontrol.com/cameras
- digiCamControl command utility: https://digicamcontrol.com/doc/userguide/cmd
- digiCamControl remote utility: https://digicamcontrol.com/doc/userguide/remoteutil
- digiCamControl standalone library: https://digicamcontrol.com/doc/development/lib
- digiCamControl GitHub: https://github.com/dukus/digiCamControl
- digiCamControl issue example 1: https://github.com/dukus/digiCamControl/issues/160
- digiCamControl issue example 2: https://github.com/dukus/digiCamControl/issues/362
- EDSDK.NET wrapper reference: https://github.com/thrixton/EDSDK.NET
- gphoto support list: https://www.gphoto.org/proj/libgphoto2/support.php
- gphoto remote control docs: https://www.gphoto.org/doc/remote/
- gphoto camera list mirror: https://github-wiki-see.page/m/gphoto/libgphoto2/wiki/List-of-cameras
- photobooth Windows/Linux split example: https://github.com/vdubuk/photobooth-1
