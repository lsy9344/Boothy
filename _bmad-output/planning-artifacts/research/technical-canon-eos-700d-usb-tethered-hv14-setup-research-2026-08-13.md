---
stepsCompleted: [1, 2, 3, 4, 5]
inputDocuments: []
workflowType: 'research'
lastStep: 5
research_type: 'technical'
research_topic: 'Canon EOS 700D 카드 없는 USB 테더 촬영 기반 HV-14 측정 환경'
research_goals: '공식 근거로 카드 없는 촬영 가능 조건, 권장 전원·USB 연결·Image Quality·장면 구성을 확정하고 안전하게 HV-14 실행 준비에 반영'
user_name: 'Noah Lee'
date: '2026-08-13'
web_research_enabled: true
source_verification: true
---

# Research Report: technical

**Date:** 2026-08-13
**Author:** Noah Lee
**Research Type:** technical

---

## Research Overview

[Research overview and methodology will be appended here]

---

<!-- Content will be appended sequentially through research workflow steps -->

## Technical Research Scope Confirmation

**Research Topic:** Canon EOS 700D 카드 없는 USB 테더 촬영 기반 HV-14 측정 환경
**Research Goals:** 공식 근거로 카드 없는 촬영 가능 조건, 권장 전원·USB 연결·Image Quality·장면 구성을 확정하고 안전하게 HV-14 실행 준비에 반영

**Technical Research Scope:**

- 카드 없는 USB 테더 촬영의 지원 조건과 제한
- EOS 700D·EDSDK·Boothy 사이의 촬영 및 파일 전달 구조
- 안정적인 전원, USB 연결, 카메라 설정 권장안
- Route A/B/C 비교에 적합한 장면과 반복 측정 조건
- 펌웨어 미확인 상태의 위험 평가와 안전한 진행 기준

**Research Methodology:**

- Canon 공식 문서와 로컬 EDSDK 실측을 우선 사용
- 핵심 주장에 대한 다중 근거 검증
- 물리적으로 확인하지 못한 값은 추정하지 않고 불확실성 표시
- 조사 결과를 HV-14 실행 템플릿과 안전장치에 반영

**Scope Confirmed:** 2026-08-13

## Technology Stack Analysis

### Camera and Firmware Platform

EOS 700D와 EOS Rebel T5i는 동일 계열이며, 18MP APS-C 센서에서 CR2 RAW와 JPEG를 생성하고 RAW+JPEG 동시 기록을 지원한다. 사용 중인 EF-S 18-55mm 번들 렌즈는 공식 구성품이자 지원 렌즈다. Canon이 공개한 최신 T5i 펌웨어는 1.1.5지만 현재 장비 버전은 확인되지 않았다. 1.1.5의 공개 수정 범위는 18-135mm 및 70-300mm 일부 렌즈의 렌즈 수차 보정 문제이므로, 18-55mm를 쓰는 이번 source latency 비교의 직접 차단 사유로 보지 않는다. 다만 환경 fingerprint에는 `unknown`과 공식 최신 버전을 함께 기록한다.

- 카메라: Canon EOS 700D / Rebel T5i
- 렌즈: EF-S 18-55mm 번들 렌즈
- 출력: CR2 RAW, JPEG, RAW+JPEG
- 펌웨어: 장비값 미확인, Canon 공개 최신 버전 1.1.5
- 근거: [Canon EOS Rebel T5i 제품 사양](https://www.cla.canon.com/en/p/eos-rebel-t5i), [Canon 펌웨어 1.1.5 공지](https://www.usa.canon.com/support/canon-product-advisories/firmware-notice-eos-rebel-t5i-firmware-version-1-1-5)

### USB Tether and Host Storage

Canon은 EOS 700D를 USB 인터페이스 케이블로 PC에 연결한 원격 촬영을 공식 지원하며, 촬영한 정지 이미지는 컴퓨터로 직접 저장되고 카드 동시 저장은 선택 옵션이라고 설명한다. 따라서 카드가 없는 현재 구성도 PC 저장 목적지에서는 유효하다. Boothy helper도 카메라 세션을 열 때 EDSDK 저장 목적지를 `Host`로 설정하고 PC 용량을 광고하므로 기존 RAW→필터→JPEG 운영 경로와 일치한다.

- 연결: PC 직접 USB, Windows 장치 위치 Port 6 / Root Hub 3.0
- 저장 목적지: PC host
- 메모리 카드: 없음 — 이번 PC tether 회차의 차단 사유 아님
- 케이블 권장: Canon은 정품 인터페이스 케이블을 권장한다. 현재 케이블의 정품 여부는 미확인이므로 연결 안정성은 회차 중 오류율로 함께 판단한다.
- 근거: [EOS 700D 원격 촬영과 PC 직접 저장](https://sg.canon/en/support/8201805300), [Canon USB 연결 안내](https://sg.canon/en/support/8201785800), [EOS Utility 연결 지침](https://cam.start.canon/en/S003/manual/html/UG-00_Before_0060.html)

### Power Platform

사용자는 이미 상시전원을 사용 중이다. Canon은 EOS 700D의 PC USB 연결에 ACK-E8 AC Adapter Kit 사용을 권장하며, 제품 사양에서도 ACK-E8을 통한 AC 전원을 지원한다. 상시전원 장치의 모델명은 미확인이므로 `continuous AC power, model unknown`으로 기록하되, 35회 동안 전원 경고·재연결이 없다는 실행 증거로 안정성을 검증한다.

- 권장 기준: Canon ACK-E8 또는 동등한 안정적 AC coupler
- 현재 상태: 상시전원 사용, 모델 미확인
- 판정: 진행 가능; 전원 중단이나 USB 재연결 발생 시 회차 폐기 후 새 세션 재실행
- 근거: [EOS 700D USB 연결 시 ACK-E8 권장](https://sg.canon/en/support/8201785800), [EOS 700D 전원 사양](https://sg.canon/en/support/6200160400)

### Application and SDK Stack

Boothy의 촬영 경로는 Canon EDSDK 13.19.0, .NET 8 C# helper, Rust/Tauri host, TypeScript UI로 구성된다. helper가 CR2 원본을 PC에 저장하고, host가 별도 source 비교 lane에서 CR2 내장 JPEG·카메라 paired JPEG·Windows Shell thumbnail을 측정한다. 최종 제품 경로는 기존처럼 RAW에 필터를 적용해 JPEG를 생성하며, HV-14 후보 JPEG는 최종 산출물을 대체하지 않는다.

- EDSDK/helper: 카메라 제어, host 저장, RAW+JPEG capability 설정·복원
- Rust host: 세 route 판정과 telemetry 기록
- 제품 truth: CR2 RAW 원본 → 필터 적용 → 최종 JPEG
- 실험 source: 빠른 중간 표시 후보일 뿐 preset 적용 최종본이 아님
- 로컬 검증: EDSDK 초기화 성공, EOS 700D 1대 `camera-ready`, helper protocol v2

### Evidence Storage

데이터베이스나 클라우드는 사용하지 않는다. 한 로컬 세션의 CR2, source JPEG, capability/correlation 이벤트와 JSONL telemetry를 immutable evidence package로 복사한다. 이 구조는 네트워크 상태를 변수에서 제거하고, request당 세 route 행과 35회 분모를 기계적으로 검사할 수 있게 한다.

- runtime: `C:\Users\dltnd\Pictures\dabi_shoot\sessions\<sessionId>`
- evidence: `tests/hardware/capture-source/run-20260813-115426-hv14/`
- completeness: 35 requests × 3 routes = 105 rows
- cloud/database: 사용하지 않음

### Technology Adoption Decision

이번 회차에 새 기술을 도입하지 않는다. 카드 구매, EOS Utility 설치, 펌웨어 업데이트는 측정 전제에 추가하지 않는다. 이미 검증된 EDSDK host-save와 현재 RAW 후처리 경로를 유지하고, RAW+JPEG 설정은 비교 lane 안에서 descriptor가 허용한 경우에만 일시 적용한 뒤 원복한다. 이는 측정 변수를 최소화하면서 카드 없는 현행 운영 환경을 그대로 검증하는 선택이다.

**신뢰도:** 카드 없는 PC 직접 저장과 RAW+JPEG 지원은 높음. 현재 펌웨어 버전과 상시전원 장치 모델은 미확인이므로 중간 신뢰도로 기록하며, source route 채택 판단에는 사용하지 않는다.

## Integration Patterns Analysis

### Camera Control API

Canon EDSDK는 카메라를 애플리케이션에 직접 통합해 원격 제어와 즉시 파일 전달을 수행하는 API다. Canon의 v13.19.0 호환표에는 EOS Kiss X7i / EOS 700D / Rebel T5i가 포함되어 있고, 현재 부스에서도 같은 SDK가 카메라 1대를 초기화해 `camera-ready`를 반환했다. 따라서 최신 EOS Utility UI의 지원 목록이 아니라 **EDSDK 호환표 + 실제 helper self-check**를 이번 통합의 truth로 사용한다.

- 제어 방식: Windows 프로세스에서 EDSDK native API 직접 호출
- 세션: 카메라 1대와 전용 세션 1개
- 명령: Image Quality descriptor 조회·일시 변경, 셔터 명령, 원복
- 결과: object arrival event를 통해 PC로 파일 전달
- 근거: [Canon SDK 개요](https://www.usa.canon.com/support/sdk), [EDSDK v13.19.0 호환표](https://developercommunity.usa.canon.com/resource/1681570481000/CDC_EDSDK_Compat_List)

### USB and Storage Protocol

물리 계층은 EOS 700D의 Hi-Speed USB 인터페이스이고, 연결은 Port 6의 PC root hub에 직접 구성되어 있다. Canon은 PC와의 케이블 연결에 정품 인터페이스 케이블을 권장하고, Windows AutoPlay는 연결 카메라에 대해 `아무 작업도 하지 않음`으로 두어 다른 프로그램의 자동 점유를 피하도록 안내한다. 현재 EOS Utility/Canon 경쟁 프로세스는 0개다.

Boothy helper는 세션을 열 때 저장 목적지를 `Host`로 설정하고 `EdsSetCapacity`로 PC 저장 용량을 제공한다. 이 구조에서는 SD 카드가 데이터 경로에 참여하지 않는다. Canon의 EOS 700D 원격 촬영 안내도 촬영 이미지를 PC에 직접 저장하고 카드 동시 저장을 선택 옵션으로 설명한다.

- 권장: 현재 직접 USB 연결 유지, 중간 허브 추가 금지
- 권장: EOS Utility 자동 실행 및 동시 실행 금지
- 비권장: USB selective suspend를 시스템 전체에서 끄는 조치. Microsoft는 이를 강하게 비권장하므로 현재 설정을 바꾸지 않고 실제 disconnect 여부를 측정한다.
- 근거: [EOS Utility 연결 지침](https://cam.start.canon/en/S003/manual/html/UG-00_Before_0060.html), [EOS 700D PC 직접 저장](https://sg.canon/en/support/8201805300), [Microsoft USB selective suspend](https://learn.microsoft.com/en-us/windows-hardware/drivers/usbcon/usb-selective-suspend)

### Event-Driven Capture Flow

통합은 동기식 “셔터 명령의 반환값”만으로 성공을 판정하지 않는다. EDSDK object event가 RAW와 JPEG transfer object를 비동기로 전달하고, helper가 이를 request 단위로 묶은 뒤 host에 JSON event를 기록한다.

1. Boothy가 새 `requestId`와 고정된 `captureId`를 만든다.
2. paired/AB lane에서만 Image Quality descriptor를 읽는다.
3. 지원 조합이 있으면 RAW+JPEG를 일시 설정한다.
4. 셔터 명령을 한 번 보낸다.
5. 도착한 object를 `groupId` 우선, 제한된 fallback correlation 보조로 같은 request에 묶는다.
6. RAW는 `captures/originals/`에 저장해 제품 truth를 닫는다.
7. paired JPEG는 `renders/sources/`에만 저장하고 비교 후보로 기록한다.
8. 종료·실패 여부와 무관하게 원래 Image Quality를 복원한다.

이 방식은 JPEG가 RAW보다 먼저 도착해도 동일 capture로 추적하며, paired JPEG 실패가 RAW 원본 성공을 뒤집지 않는다.

### Data Formats and Evidence Contracts

경계별 데이터 형식은 역할이 분리되어 있다.

- CR2: 변하지 않는 원본과 최종 RAW 후처리 입력
- JPEG: 내장 JPEG, paired JPEG, Windows Shell thumbnail 비교 산출물
- JSONL: request·route별 source telemetry; append-only
- Markdown/JSON: 환경, capability, correlation, aggregate, 품질 승인, route 결정
- JSON file protocol v2: helper와 host 사이의 상태·도착·거부 event

source telemetry는 35개 request 각각에 세 route 행이 정확히 하나씩 있어야 하며 총 105행이다. 손상된 JSONL, mixed session, route 누락, 잘못된 warm-up 또는 AB/BA order는 모두 회차 실패다.

### Product Interoperability Boundary

사용자가 설명한 현행 제품은 `CR2 RAW 생성 → 필터 적용 → JPEG 출력`이다. HV-14는 이 경로를 교체하지 않는다. 비교 후보는 고객이 기다리는 동안 먼저 보여줄 수 있는 source의 속도와 품질을 판단하기 위한 것이며 `isPresetApplied=false`로 고정된다. 최종 JPEG는 기존 RAW 후처리만이 만든다.

- 제품 truth: RAW 저장 성공
- 빠른 후보 실패: 해당 route의 실패 표본으로 기록
- 최종 JPEG 실패: 별도 기존 render 경로의 문제
- 채택 전제: 빠른 후보가 source gate를 통과하고 HV-14 route 결정이 승인돼도, Story 7.4 이후에만 preset proxy로 연결 가능

### Fault Isolation and Recovery

중요한 실패 경계는 다음과 같다.

- Image Quality 변경 실패: paired route를 `unsupported/set-failed`로 기록하고 RAW 촬영은 유지
- JPEG 누락/timeout: paired route 실패, RAW 원본 유지
- unknown/duplicate/group mismatch object: 명시적 거부 행 기록
- USB disconnect 또는 helper recovery: qualifying 회차를 이어 붙이지 않고 보존 후 새 세션에서 35회 재실행
- Image Quality 원복 실패: HV-14 No-Go 및 수동 복원 확인 전 증거 수집 차단
- source/display lane 동시 활성화: 표본 오염이므로 실행 금지

### Integration Security and Scope

외부 네트워크 API, 인증 토큰, 클라우드 전송은 없다. 카메라와 로컬 PC 사이의 직접 USB, 로컬 프로세스 간 파일 protocol, session-scoped 디렉터리만 사용한다. 보안의 핵심은 네트워크 인증보다 세션 경로 격리, 다른 session artifact 재사용 금지, 경쟁 카메라 프로세스 배제, 측정 환경변수 종료 후 원복이다.

**통합 결론:** 카드 없는 host-save 구성은 지원되고 현재 helper와 실장비에서 연결이 확인됐다. 추가 설정 변경 없이 진행하되, EOS Utility 경쟁 점유·USB disconnect·Image Quality 원복 실패를 회차 중단 조건으로 둔다. 신뢰도는 높음이다.

**운영자 확인 (2026-08-13):** 현재 카메라의 USB→PC RAW 전송과 기존 RAW→필터→JPEG 생성은 이미 정상 운영 중이다. 이후 단계에서 이 경로의 재검증은 생략하고, HV-14 신규 비교 lane만 검증한다.

## Architectural Patterns and Design

### System Architecture Pattern

가장 안전한 구조는 현행 로컬 촬영 파이프라인을 중심에 두고, source 비교를 **기본 off인 측정 전용 side lane**으로 붙이는 방식이다. EOS 700D와 EDSDK는 이미 정상 운영 중이므로 전송 경로를 재설계하지 않는다.

```text
EOS 700D shutter
  └─ CR2 RAW ──> immutable original ──> 기존 필터 처리 ──> final JPEG
       ├─> embedded JPEG candidate
       ├─> paired JPEG candidate (capability가 허용할 때만)
       └─> Windows Shell thumbnail baseline
                    └─> source telemetry + isolated evidence
```

이 패턴은 실험 실패가 제품 truth를 오염시키지 않는 branch-by-abstraction 구조다. Canon은 EDSDK를 애플리케이션에 카메라 제어와 즉시 전송을 통합하는 API로 설명하며, EOS 700D는 RAW+JPEG 동시 생성을 지원한다. [Canon SDK](https://www.usa.canon.com/support/sdk), [EOS 700D 사양](https://www.cla.canon.com/en/p/eos-rebel-t5i)

### Design Principles and Boundaries

- **RAW truth 우선:** CR2 저장 성공이 촬영 성공의 기준이다.
- **후보 격리:** source JPEG는 `renders/sources/`에만 두고 최종 JPEG 경로에 쓰지 않는다.
- **capability before mutation:** descriptor에 있는 RAW+JPEG 값만 임시 적용한다.
- **restore on every exit:** 성공, timeout, cancellation, 예외 모두 Image Quality 원복을 거친다.
- **explicit failure:** 누락 object나 correlation 실패도 한 행으로 남겨 분모가 줄지 않게 한다.
- **single-session evidence:** 35회 회차를 이어 붙이거나 여러 session을 합치지 않는다.
- **known-good baseline preservation:** 사용자가 확인한 USB→PC RAW와 RAW→필터→JPEG 운영 경로는 재검증 범위에서 제외한다.

### Experimental Architecture

route마다 별도 셔터를 누르는 구조보다 **한 번의 촬영에서 세 route를 모두 관측**하는 구조가 우월하다. 장면, 초점, 노출, 카메라 처리 시점이라는 nuisance factor가 같은 request 안에서 공유되므로 route 차이를 더 직접 비교할 수 있다. NIST는 통제 가능한 nuisance factor는 block으로 묶고 나머지는 randomize하라고 권고한다. [NIST randomized block design](https://www.itl.nist.gov/div898/handbook/pri/section3/pri332.htm)

- block: 한 번의 shutter/request
- treatment: embedded / paired / shell 세 route
- warm-up: 앞 5 request, 집계 제외
- measured: 뒤 30 request
- route order: seed로 AB/BA를 섞되 세 route 모두 같은 request에 존재
- success denominator: route당 35, 성능 집계 denominator는 measured 30

### Scene Blocking Recommendation

품질과 장면 복잡도 영향을 분리하기 위해 measured 30회를 다음 네 scene block으로 배분한다. 각 block 안에서는 삼각대, 초점, focal length, 조명, 노출, white balance를 고정하고 scene 전환만 명시적으로 기록한다.

| 구간 | 장면 | 횟수 | 목적 |
| --- | --- | ---: | --- |
| warm-up | 일반 실내 장면 | 5 | SDK·카메라·filesystem 예열 |
| block A | 밝고 균일한 장면 | 8 | 낮은 노이즈, 밝은 톤, 파일 크기 기준 |
| block B | 어두운 장면 | 8 | shadow/noise와 낮은 광량 품질 |
| block C | 인물 또는 실제 피부톤 대상 | 7 | 피부색·선명도·노출 품질 |
| block D | 창가/검정·흰 물체가 공존하는 고대비 | 7 | highlight/shadow clipping과 artifact |

총 셔터는 정확히 35회다. route별로 다시 촬영하지 않는다. 장면 순서는 사전에 고정하고 `blockIndex`와 별도의 scene 메모로 남긴다. NIST는 실험 계획을 실행 전에 정하고, 장비 warm-up 조건을 재현할 것을 권고한다. [NIST DOE 개요](https://itl.nist.gov/div898/handbook/pri/section1/pri11.htm), [NIST confirmatory run 원칙](https://www.itl.nist.gov/div898/handbook/pri/section4/pri46.htm)

### Camera Setting Architecture

HV-14의 primary factor는 route이므로 카메라 자동 기능이 불필요한 변동을 만들지 않게 한다.

- 카메라 고정: 삼각대 또는 현재 부스 고정 마운트
- focal length: 현재 부스에서 쓰는 값 유지; 회차 중 zoom 금지
- focus: block 시작 시 AF로 맞춘 뒤 가능하면 MF 고정; 인물 block에서 위치가 바뀌면 다시 맞추고 기록
- exposure: block별 Manual 권장; block 안에서는 shutter/aperture/ISO 고정
- white balance: Auto 대신 현재 부스의 고정 WB 또는 Kelvin 값 권장
- drive: single shot
- flash: 현재 제품이 쓰지 않으면 off; 쓰면 모든 block에서 동일 조건
- Image Quality: 시작값은 현재 제품의 RAW 설정 그대로 기록한다. paired/AB lane이 descriptor에서 지원값을 선택하므로 사람이 RAW+JPEG로 미리 바꾸지 않는다.

이 권고는 최종 필터 결과를 평가하려는 것이 아니라 source route 사이의 비교 가능성을 높이기 위한 것이다. EOS 700D가 RAW+JPEG를 지원한다는 사실과 실제 descriptor가 어떤 조합을 제공하는지는 구분하며, 후자는 회차 증거로 남긴다.

### Performance and Scalability

확장성 목표는 동시 카메라 수가 아니라 한 카메라에서의 안정적인 35회 반복이다. 별도 서버, queue, database를 추가하지 않고 session-scoped append-only 파일을 사용한다. 측정할 값은 route별 trigger→ready p50/p95/max, 성공률, 거부 사유, extraction cost다.

- p95가 소수 outlier를 숨기지 않도록 max도 함께 본다.
- timeout과 실패를 latency 집계에서 조용히 제거하지 않고 성공률 분모에 남긴다.
- 30 measured sample은 기술 채택의 최소 evidence이며 보편적 성능 보증은 아니다.
- scene block별 결과도 함께 봐 route가 특정 장면에서만 유리한지 확인한다.

### Event and Data Architecture

비동기 object event에는 항상 `sessionId`, `requestId`, `captureId`를 붙여 end-to-end trace를 만든다. event-driven 구조는 공유 call stack이 없으므로 correlation ID와 전송 중 데이터 보존이 observability의 핵심이라는 일반 원칙과 일치한다. [Microsoft event-driven architecture](https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/event-driven)

- helper event log: capability, object arrived/rejected, file arrived
- host telemetry: route별 accepted/rejected와 timing
- evidence copy: 원본 session 전체를 새 run 폴더에 한 번만 복사
- mutation rule: 복사 후 원본과 evidence의 telemetry 행 삭제·수정 금지

### Security and Operations Architecture

네트워크 공격면은 없으며 핵심 운영 위험은 카메라 독점 점유, 잘못된 session 선택, 측정 mode 누수다. 따라서 시작 스크립트가 EDSDK self-check, 새 session 요구, display lane off를 강제하고 종료 시 환경변수를 원복한다. 증거 collector는 기존 run을 덮어쓰지 않는다.

### Architecture Decision

**채택:** 현행 RAW 제품 경로 + 격리된 3-route measurement lane + request 단위 randomized block.

**거절:** route별 독립 촬영, 카드 도입, EOS Utility 병행, USB 절전 전역 해제, 실험 JPEG의 최종본 대체.

**남은 물리 조건:** 장면 4종 준비와 현재 부스의 focal length·노출·WB 값을 운영자가 회차 전에 기록해야 한다. 펌웨어 및 상시전원 모델 미확인은 `unknown`으로 기록하되 실행 차단 사유가 아니다.

## Implementation and Adoption Analysis

### Implementation Baseline

HV-14는 새로운 촬영 제품 경로를 배포하는 작업이 아니라, 기존 경로 옆에서 빠른 source 후보를 평가하는 한정된 하드웨어 측정이다. 현재 USB→PC RAW 수신과 RAW→필터→JPEG 생성은 운영자가 정상 동작을 확인했으므로 다시 검증하지 않는다. 구현의 기준선은 다음과 같다.

- EOS 700D 한 대, 기존 직접 USB 연결과 상시전원 유지
- 메모리카드 없이 EDSDK `SaveTo Host` 사용
- 현행 RAW Image Quality를 운영자가 미리 변경하지 않음
- source comparison은 `ab`, display sampling은 `off`
- 한 개의 새 session에서 정확히 35회 촬영
- 제품 RAW와 최종 JPEG는 기존 경로에 남기고 비교 후보만 별도 evidence로 격리

Canon은 원격 촬영 이미지를 PC로 직접 저장하고 메모리카드 동시 저장을 선택 사항으로 설명한다. 또한 EOS Utility 문서는 카메라 연결 시 다른 자동 실행 동작을 피하도록 안내한다. [Canon EOS 700D 원격 촬영](https://sg.canon/en/support/8201805300), [Canon EOS Utility 연결 지침](https://cam.start.canon/en/S003/manual/html/UG-00_Before_0060.html)

### Operator Workflow

1. 네 장면과 고정 구도·초점거리·화이트밸런스를 준비한다.
2. EOS Utility 등 경쟁 카메라 프로세스를 종료한다.
3. `environment.md`에서 장비와 시작 Image Quality 기록을 확인한다.
4. 운영자 준비 확인 스위치로 앱을 실행하고 새 session을 만든다.
5. 일반 실내 warm-up 5회, 밝은 장면 8회, 어두운 장면 8회, 피부톤 7회, 고대비 7회를 순서대로 촬영한다.
6. 앱을 정상 종료한 뒤 Image Quality가 기존 RAW 값으로 복원됐는지 확인한다.
7. 새 session을 evidence 폴더로 복사하고 자동 완결성 검사를 실행한다.
8. capability, correlation, quality, aggregate, route decision을 채운 뒤 최종 HV-14 gate를 실행한다.

한 번의 셔터에서 세 route를 모두 관측하므로 총 셔터 수는 35회이고 telemetry 예상치는 105행이다. 중단된 session을 다른 session과 합치지 않으며 새 session에서 35회를 다시 시작한다.

### Measurement and Quality Assurance

측정 안정성은 평균 속도 하나가 아니라 완결성, 상관관계, 복원, 품질을 함께 확인한다. NIST는 측정 프로세스가 안정 상태인지 확인한 뒤 미래 성능을 추론해야 한다고 설명한다. [NIST process stability](https://www.itl.nist.gov/div898/handbook/pri/section2/pri22.htm)

- **완결성:** route별 35행, 전체 105행, warm-up 5와 measured 30의 정확한 분리
- **상관관계:** 모든 행이 단일 session의 request/capture에 연결되고 duplicate·unknown object가 없음
- **성능:** measured 30개를 대상으로 route별 trigger→ready p50·p95·max와 성공률 산출
- **품질:** 네 scene block별 노출, 색, 선명도, clipping 및 육안 수용성 기록
- **복원:** 성공·실패와 무관하게 카메라 Image Quality가 시작값으로 돌아옴
- **격리:** source 후보가 기존 최종 JPEG 또는 display lane을 대체하지 않음

### Operational Safeguards

실행 패키지는 잘못된 상태에서 측정이 시작되거나 불완전한 결과가 승인되는 것을 방지한다.

- SDK 초기화와 카메라 정확히 1대 연결을 실행 직전 검사
- 운영자 확인 없이는 앱 실행 차단
- `environment.md` 누락 또는 미확정 placeholder가 있으면 실행 차단
- 촬영 후 Image Quality 복원값이 미확정이면 evidence 수집 차단
- 기존 evidence 폴더를 덮어쓰지 않음
- 실행 종료 시 source/display 환경변수 원복
- USB disconnect, 카메라 전원 중단, session 혼합 시 해당 회차 폐기

USB selective suspend는 측정 전에 시스템 전체에서 비활성화하지 않는다. Microsoft는 전역 비활성화를 강하게 권장하지 않으므로 실제 연결 끊김이 관측될 때에만 별도 원인 분석 대상으로 둔다. [Microsoft USB selective suspend](https://learn.microsoft.com/en-us/windows-hardware/drivers/usbcon/usb-selective-suspend)

### Risk Assessment

| 위험 | 영향 | 대응 |
| --- | --- | --- |
| 촬영 횟수 오기 | 분모와 telemetry 불일치 | 사전 고정된 1–35 scene plan 사용, 초과/누락 시 새 session 재실행 |
| 장면 블록 내 자동 설정 변동 | route 차이와 카메라 변동 혼합 | block별 Manual 노출, 고정 WB, 고정 zoom·focus 권장 |
| USB 또는 전원 중단 | object 누락과 잘못된 latency | 현재 직접 USB·상시전원 유지, 회차 보존 후 전면 재실행 |
| Image Quality 원복 실패 | 이후 제품 촬영 설정 오염 | 수동 복원 확인 전 evidence 수집과 Go 판정 차단 |
| paired JPEG 미지원 | 특정 route 측정 불가 | capability evidence에 명시하고 RAW truth를 유지한 채 No-Go/대안 판정 |
| firmware·전원 모델 미상 | 재현성 fingerprint 약화 | `unknown`으로 정직하게 기록; 관측된 안정성과 route 결과에는 혼입하지 않음 |

### Adoption Roadmap

1. **Qualifying run:** 현재 준비된 회차에서 35회 촬영 및 evidence 수집
2. **Evidence gate:** completeness, capability, correlation, quality, aggregate 검사
3. **Route decision:** 품질과 안정성 조건을 통과한 후보 중 지연시간이 가장 좋은 route를 Go/No-Go로 결정
4. **Controlled integration:** Go인 경우에만 후속 Story에서 빠른 preview/preset proxy 후보로 연결
5. **Rollback:** 문제가 발견되면 source lane을 기본 off로 유지하고 기존 RAW→필터→JPEG만 사용

### Success Criteria and Decision Rule

성공은 단순히 JPEG가 생성되는 것이 아니라 **재현 가능한 35회 evidence가 완전하고, 카메라 상태를 오염시키지 않으며, 품질 승인과 route 결정을 지원하는 것**이다.

- 정확히 한 session, 35 requests, route별 35행
- measured 30개에 대한 성능 집계 가능
- 누락·중복·mixed-session·correlation defect 없음
- 네 장면 블록의 품질 검토 완료
- Image Quality 시작값과 종료값 일치
- 기존 제품 RAW 및 최종 JPEG 경로 변화 없음
- 최종 route decision에 근거, 제한, rollback 조건 포함

### Implementation Decision

현재 환경은 HV-14 실행에 적합하다. 카드 구매, 펌웨어 업데이트, USB 절전 전역 변경, 기존 전송 경로 재검증은 선행 조건이 아니다. 남은 작업은 운영자가 네 장면을 준비하고 `-OperatorReady`로 한 번의 qualifying session을 시작하는 것이다. 실행 후에는 자동 완결성 통과만으로 Go를 선언하지 않고, 품질·상관관계·복원·aggregate 증거를 모두 닫는다.
