---
stepsCompleted: [1, 2, 3, 4, 5, 6]
inputDocuments:
  - '_bmad-output/planning-artifacts/research/domain-raw-photo-preset-gpu-first-research-2026-04-11.md'
  - '_bmad-output/planning-artifacts/research/technical-boothy-gpu-first-rendering-architecture-research-2026-04-11.md'
workflowType: 'research'
lastStep: 6
research_type: 'technical'
research_topic: 'Boothy GPU-first 렌더링 아키텍처 결론 검증'
research_goals: '2026-04-11에 작성된 최근 도메인/기술 리서치를 기준선으로 삼아, 최신 공개 기술 자료와 비교했을 때 핵심 결론이 유지되는지 검증한다. 특히 resident GPU-first 구조, darktable의 baseline/fallback 역할, canonical preset recipe, display/export 분리의 타당성을 재확인한다.'
user_name: 'Noah Lee'
date: '2026-04-11'
web_research_enabled: true
source_verification: true
---

# Research Report: technical

**Date:** 2026-04-11
**Author:** Noah Lee
**Research Type:** technical

---

## Research Overview

이번 문서는 2026-04-11에 작성된 기존 도메인/기술 리서치를 기준선으로 삼아, 최신 공개 자료를 다시 확인했을 때 핵심 결론이 유지되는지 검증하기 위한 기술 리서치다.
검증 대상은 단순한 GPU 활용 여부가 아니라, `품질`, `현재 프로덕트 목표인 프리셋 적용 속도`, `display/export parity`, `fallback 운영성`까지 포함한다.

방법론:

- 기존 두 문서를 기준 가설로 사용
- Adobe, Microsoft, Khronos, Tauri, darktable 등 1차 출처 우선 확인
- 기존 결론 유지/수정/보류 항목을 분리 평가
- 제품 관점의 판단 기준은 `사용자가 불편 없이 프리셋을 빠르게 적용하고, 본 화면 품질과 최종 결과가 어긋나지 않는가`로 고정

---

<!-- Content will be appended sequentially through research workflow steps -->

## Technical Research Scope Confirmation

**Research Topic:** Boothy GPU-first 렌더링 아키텍처 결론 검증
**Research Goals:** 2026-04-11에 작성된 최근 도메인/기술 리서치를 기준선으로 삼아, 최신 공개 기술 자료와 비교했을 때 핵심 결론이 유지되는지 검증한다. 특히 resident GPU-first 구조, darktable의 baseline/fallback 역할, canonical preset recipe, display/export 분리의 타당성을 재확인한다. 또한 품질과 현재 프로덕트 목표인 프리셋 적용 속도까지 함께 충족하는지 확인한다.

**Technical Research Scope:**

- Architecture Analysis - resident GPU-first 구조와 display/export 분리의 지속 타당성
- Implementation Approaches - darktable baseline/fallback 유지, hybrid, custom runtime 검토
- Technology Stack - Windows, Rust/Tauri, D3D12/OpenCL, 저장소 및 계측 도구 검증
- Integration Patterns - canonical preset recipe, parity, fallback, interoperability 재검증
- Performance Considerations - 프리셋 적용 속도, 풀사이즈 표시, batch export throughput, 품질 parity

**Research Methodology:**

- Current web data with rigorous source verification
- Multi-source validation for critical technical claims
- Confidence level framework for uncertain information
- Product-centered evaluation using quality, preset application speed, parity, and operational stability

**Scope Confirmed:** 2026-04-11

## Technology Stack Analysis

### Programming Languages

최신 공식 자료를 다시 확인해도, Boothy의 주력 언어 축을 `Rust + Windows GPU shader language`로 두는 기존 결론은 유지된다. Tauri 2 아키텍처 문서는 2025-02-22 기준으로도 Tauri가 `Rust tools + HTML rendered in a Webview` 조합이며, Webview와 Rust backend를 메시지 전달로 연결한다고 설명한다. 따라서 앱 셸과 시스템 제어는 여전히 Rust 유지가 자연스럽다.

Windows GPU-first 경로에 대해서도 기존 판단은 유지된다. Microsoft Learn은 Direct3D 12에서 pipeline state object와 HLSL 기반 shader object 구성이 핵심이라고 설명한다. 또한 Adobe Lightroom Classic GPU FAQ는 Windows에서 GPU display 가속 조건으로 `DirectX 12` 지원을 명시한다. 이는 Lightroom 계열 워크플로와 유사한 문제를 푸는 Boothy가 Windows 전용 GPU 경로를 검토할 때 `D3D12 + HLSL`을 우선 후보로 두는 판단을 뒷받침한다.

기존 리서치 이후 새로 생긴 중요한 변화는 Adobe가 2025-08-13 업데이트 문서에서 `GPU for Preview Generation`을 별도 기능으로 공개했다는 점이다. Lightroom Classic 14.5부터 preview generation에도 GPU를 적용할 수 있고, Auto 기본 조건은 `16 GB 이상 VRAM`과 `full acceleration` 지원이다. 이 문서는 직접적으로 `프리셋 적용 속도`를 말하지는 않지만, preview generation을 GPU 가속 범위로 확장했다는 사실 자체가 업계가 `사용자가 체감하는 반응 속도`를 GPU로 더 끌어오고 있음을 보여준다. 이 해석은 공식 문서에 근거한 추론이다.

_Popular Languages:_ Rust는 앱 코어와 시스템 제어에 계속 적합하며, Windows GPU 경로는 HLSL이 가장 직접적이다.  
_Emerging Languages:_ 범용 언어 트렌드에서는 Python 채택이 계속 커지고 있지만, Boothy의 hot path 결정에는 결정적이지 않다.  
_Language Evolution:_ 기존 `Rust 유지 + GPU 전용 언어 분리` 판단은 그대로 유효하며, 업계는 preview/processing/export에 GPU 경로를 더 넓히는 쪽으로 이동 중이다.  
_Performance Characteristics:_ 시스템 orchestration은 Rust, 픽셀 처리 성능은 D3D12/HLSL 또는 OpenCL 계열 설계가 좌우한다.  
_Source:_ https://v2.tauri.app/concept/architecture/  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/direct3d12/pipelines-and-shaders-with-directx-12  
_Source:_ https://helpx.adobe.com/ee/lightroom-classic/kb/lightroom-gpu-faq.html  
_Source:_ https://helpx.adobe.com/si/lightroom-classic/kb/gpu-preview-generation.html  
_Source:_ https://survey.stackoverflow.co/2025

### Development Frameworks and Libraries

프레임워크 측면에서도 기존 결론은 거의 그대로 유지된다. Tauri는 여전히 작은 앱 셸과 Rust backend 중심 구조를 제공하므로, Boothy가 UI 셸을 유지한 채 렌더링 구조만 교체하기에 적합하다. 즉 `앱 셸 유지 + 렌더 엔진 교체/병행` 전략은 최신 공식 문서와도 모순되지 않는다.

darktable 쪽 근거도 살아 있다. darktable 공식 문서는 OpenCL 활성화 시 processing context 초기화, calculation pipeline 시작, `.cl` 커널 읽기/컴파일/준비가 필요하다고 설명한다. 이 사실은 darktable가 GPU를 쓸 수 있다는 장점과 함께, cold start 및 warm-state 운영이 중요하다는 기존 리서치의 논리를 강화한다. 또한 `darktable-cli`는 GUI 없이 export를 수행하고 XMP sidecar의 history stack을 적용할 수 있다고 명시한다. 따라서 `darktable = baseline / fallback / export oracle` 역할은 여전히 유효하다.

반대로, latest official signal 중 기존 판단을 바꾸게 만드는 자료는 아직 보이지 않는다. 공식 자료 어디에도 `darktable-only가 resident GPU service보다 더 유리하다`는 신호는 없다. 오히려 Adobe가 preview generation까지 GPU 범위를 확장한 점을 고려하면, 제품의 핵심 목표가 `품질을 해치지 않으면서 프리셋 적용 직후 빠르게 보여주는 것`이라면 custom GPU lane의 필요성은 이전보다 더 강해졌다.

_Major Frameworks:_ Tauri, Direct3D 12, darktable OpenCL/CLI 조합이 여전히 가장 현실적인 축이다.  
_Micro-frameworks:_ canonical preset adapter, resident queue manager, parity comparator 같은 사내 경량 계층의 중요성은 여전하다.  
_Evolution Trends:_ Adobe는 GPU 활용 범위를 display/image processing/export에서 preview generation까지 넓혔다.  
_Ecosystem Maturity:_ Tauri와 D3D12는 성숙했고, darktable는 baseline/fallback 용도로 충분히 성숙하다.  
_Source:_ https://v2.tauri.app/concept/architecture/  
_Source:_ https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/  
_Source:_ https://docs.darktable.org/usermanual/development/en/special-topics/program-invocation/darktable-cli/  
_Source:_ https://helpx.adobe.com/si/lightroom-classic/kb/gpu-preview-generation.html

### Database and Storage Technologies

저장소 선택에 대해서도 기존 판단을 바꿀 근거는 없다. Microsoft Learn은 Windows 앱에서 SQLite를 로컬 단일 파일 DB로 바로 사용할 수 있고, Windows에 포함된 SQLite를 활용할 수 있다고 설명한다. Boothy처럼 on-device 세션 메타데이터, preset truth, 캐시 인덱스를 관리하는 제품에는 여전히 `SQLite + filesystem` 조합이 가장 현실적이다.

다만 검증 관점에서 기존 리서치의 주의사항은 더 명확해졌다. SQLite 공식 WAL 문서는 reader와 writer의 동시성을 높일 수 있지만, reader gap이 없으면 checkpoint가 끝나지 않아 WAL 파일이 계속 커질 수 있다고 설명한다. Boothy에서 긴 읽기 트랜잭션이나 과도한 concurrent read를 방치하면 preview/export metadata hot path에도 역효과가 날 수 있으므로, `SQLite는 맞지만 WAL 운영 정책까지 같이 설계해야 한다`는 점은 유지된다.

XMP 관련 판단도 유지된다. Adobe Developer 문서는 XMP가 metadata data model, serialization, core properties를 표준화하고 다양한 파일 형식에 embed하는 가이드를 제공한다고 설명한다. 따라서 canonical preset recipe를 내부 truth로 두더라도, 외부 호환 계층으로 XMP를 계속 유지하는 전략은 여전히 타당하다.

_Relational Databases:_ SQLite가 여전히 1순위다.  
_NoSQL Databases:_ 현재 제품 목표에는 우선순위가 낮다.  
_In-Memory Databases:_ 별도 서버형 메모리 DB보다 프로세스 메모리/GPU resource cache가 더 중요하다.  
_Data Warehousing:_ 현장 hot path보다는 사후 분석 영역이다.  
_Source:_ https://learn.microsoft.com/en-us/windows/apps/develop/data-access/sqlite-data-access  
_Source:_ https://sqlite.org/wal.html  
_Source:_ https://developer.adobe.com/xmp/docs/xmp-specifications/

### Development Tools and Platforms

도구 체계도 기존 리서치와 같은 방향이다. Microsoft PIX 문서는 Direct3D 12 애플리케이션의 GPU capture와 timing capture를 통한 렌더링 문제 분석과 프레임 성능 분석을 지원한다고 설명한다. Boothy가 D3D12-first prototype을 검토한다면 PIX는 여전히 핵심 도구다.

운영 계측 측면에서도 ETW는 유효하다. Microsoft Learn은 ETW가 kernel-level tracing facility이며, production 환경에서 재시작 없이 동적으로 tracing을 켜고 끌 수 있다고 설명한다. 이는 `프리셋 적용 속도`, `latest full-size visible latency`, `fallback 전환`, `export 처리량`을 현장 장비에서 낮은 오버헤드로 수집해야 하는 Boothy 요구에 잘 맞는다.

로컬 IPC 후보로 제안했던 named pipe 역시 여전히 유효하다. Microsoft의 overlapped I/O 예제는 하나의 pipe server가 여러 pipe instance를 만들어 동시 연결을 처리할 수 있음을 보여준다. 따라서 `UI/core ↔ resident GPU service` 분리를 한다면 local IPC는 여전히 실용적인 선택지다.

_IDE and Editors:_ 핵심 차별화는 IDE보다 GPU/latency 계측 도구에 있다.  
_Version Control:_ 렌더 계약, preset schema, telemetry schema의 버전 관리가 중요하다.  
_Build Systems:_ Rust/Cargo 기반 repeatable build 판단은 유지된다.  
_Testing Frameworks:_ 단위 테스트보다 PIX/ETW/parity diff/booth smoke가 더 중요하다.  
_Source:_ https://learn.microsoft.com/hu-hu/windows/win32/direct3dtools/pix/articles/general/pix-overview  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/etw/about-event-tracing  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-server-using-overlapped-i-o

### Cloud Infrastructure and Deployment

클라우드 GPU 인프라의 존재 자체는 분명하다. Azure는 NV 계열 GPU VM을 graphics applications와 virtual desktop용으로 제공하고, Google Cloud는 Windows Server 2022 기반 display-capable GPU workstation 구성을 제공한다. AWS도 2024-11-14 기준 Deadline Cloud에서 NVIDIA GPU accelerated EC2 fleets를 지원한다고 발표했다.

하지만 이 사실이 Boothy의 핵심 결론을 바꾸지는 않는다. 이들 서비스는 remote rendering, benchmark farm, batch experimentation에는 유용하지만, booth 현장에서 사용자가 체감하는 `프리셋 적용 속도`와 `풀사이즈 표시 반응성`을 닫는 1차 해법은 아니다. 공식 자료도 이들을 원격 워크스테이션/렌더 팜/graphics workload 인프라로 설명할 뿐, 로컬 인터랙티브 booth UX 대체재로 말하지 않는다. 따라서 `cloud는 보조, on-device GPU는 주력`이라는 기존 결론은 그대로 유지된다.

_Major Cloud Providers:_ AWS, Azure, GCP 모두 GPU 옵션을 제공한다.  
_Container Technologies:_ 이번 문제의 핵심은 컨테이너보다 로컬 GPU/드라이버/interactive latency다.  
_Serverless Platforms:_ 프리셋 적용 직후 반응성 요구와는 맞지 않는다.  
_CDN and Edge Computing:_ Boothy의 edge는 외부 edge node가 아니라 현장 Windows 장비 자체다.  
_Source:_ https://learn.microsoft.com/en-us/azure/virtual-machines/sizes/gpu-accelerated/nv-family  
_Source:_ https://docs.cloud.google.com/compute/docs/virtual-workstation/windows-gpu  
_Source:_ https://aws.amazon.com/about-aws/whats-new/2024/11/aws-deadline-cloud-gpu-accelerated-ec2-instance-types/

### Technology Adoption Trends

이번 검증에서 가장 중요한 최신 변화는 `업계가 GPU를 더 깊게 preview path로 끌어오고 있다`는 점이다. 2026-04-11 기준으로 확인 가능한 Adobe 공식 자료에서, Lightroom Classic은 이미 display/image processing/export에 GPU를 쓰고 있었고, 2025-08-13에는 preview generation까지 GPU 적용 범위를 확장했다. 이는 기존 기술 리서치의 핵심 명제였던 `GPU-first 방향`을 약화시키지 않고 오히려 강화한다.

동시에 `darktable는 버리기보다 baseline/fallback로 둔다`는 결론도 유지된다. 최신 공식 자료에서도 darktable는 OpenCL acceleration과 CLI export, XMP sidecar 활용을 제공한다. 다만 resident warm-state와 프리셋 적용 직후의 체감 반응성을 제품 핵심으로 삼는다면, darktable만으로 주력 UX를 닫기보다는 baseline/fallback/reference로 두는 편이 여전히 더 타당하다. 이 역시 공식 문서 기반 해석이다.

종합하면, 기술 스택 차원에서 기존 결론은 `대체로 동일`하다. 다만 하나의 수정이 있다면, 이제 `GPU-first가 좋을 것 같다` 수준이 아니라 `preview/preset responsiveness까지 포함해 업계도 GPU를 더 깊게 쓰기 시작했다`는 근거가 더 강해졌다는 점이다.

_Migration Patterns:_ CPU 중심/엔진 단일 경로에서 resident GPU preview path + separated export path로 이동하는 흐름이 강화되고 있다.  
_Emerging Technologies:_ GPU preview generation, richer D3D12 tooling, stronger local telemetry 운영이 중요해지고 있다.  
_Legacy Technology:_ darktable-only 단일 주력 경로 가정은 여전히 약하다.  
_Community Trends:_ 범용 인기도보다, 제품 목표에 맞는 GPU/runtime/tooling 조합이 실제 선택을 좌우한다.  
_Source:_ https://helpx.adobe.com/ee/lightroom-classic/kb/lightroom-gpu-faq.html  
_Source:_ https://helpx.adobe.com/si/lightroom-classic/kb/gpu-preview-generation.html  
_Source:_ https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/  
_Source:_ https://docs.darktable.org/usermanual/development/en/special-topics/program-invocation/darktable-cli/  
_Source:_ https://v2.tauri.app/concept/architecture/

## Integration Patterns Analysis

### API Design Patterns

최신 공식 자료를 기준으로 보면, Boothy의 핵심 API 문제는 외부 공개 API 설계가 아니라 `같은 Windows 장비 안에서 UI, 앱 코어, resident GPU service가 어떤 계약으로 통신하느냐`다. Tauri IPC 문서는 Tauri가 비동기 메시지 전달 방식을 사용하며, `Events`와 `Commands`라는 두 가지 IPC primitive를 제공한다고 설명한다. 이벤트는 단방향 상태 전달에 적합하고, 명령은 프론트엔드가 Rust 함수를 호출하고 인수를 전달해 응답을 받는 구조다. 또한 명령 호출은 내부적으로 JSON-RPC류 직렬화를 사용하므로 인수와 반환값은 JSON 직렬화 가능해야 한다고 설명한다.

이 점을 Boothy에 대입하면, `UI → 앱 코어` 구간은 여전히 `Tauri Commands + Events` 조합이 가장 자연스럽다. 즉 preset 선택, 렌더 요청, export 시작 같은 제어는 command, 진행률/가용성/상태 변경은 event가 맞다. 반면 `앱 코어 ↔ resident GPU service`는 다른 판단이 필요하다. gRPC 공식 문서는 gRPC가 서비스 메서드 정의와 Protocol Buffers를 함께 사용하는 RPC 프레임워크이며, HTTP/2 기반 분산 애플리케이션에 적합하다고 설명한다. 그러나 Boothy의 주 hot path는 네트워크 분산 서비스가 아니라 로컬 프로세스 경계다. 따라서 gRPC는 `typed contract`와 code generation 장점은 있지만, 로컬 booth 앱에서 HTTP/2 스택을 함께 끌고 오는 비용이 있다.

결론적으로 기존 리서치의 판단은 유지된다. `UI ↔ core는 Tauri IPC`, `core ↔ render service는 더 얇은 로컬 IPC`가 맞다. gRPC/protobuf는 장기적으로 sidecar나 원격 진단 경로에 적용 가능하지만, 현재 프로덕트 목표인 `프리셋 적용 속도`를 직접 닫는 1차 제어면으로는 과할 가능성이 높다. 이 마지막 판단은 공식 문서에 근거한 추론이다.

_RESTful APIs:_ 원격 관리나 진단용에는 가능하지만, 로컬 렌더 hot path에는 부적합하다.  
_GraphQL APIs:_ 현재 문제에서는 실질 이점이 작다.  
_RPC and gRPC:_ typed contract 장점은 크지만, 로컬 same-machine render control의 1차 선택으로는 무겁다.  
_Webhook Patterns:_ 외부 연동이 아닌 현장 단일 장비 구조에는 핵심이 아니다.  
_Source:_ https://v2.tauri.app/concept/inter-process-communication/  
_Source:_ https://grpc.io/docs/what-is-grpc/introduction/  
_Source:_ https://protobuf.dev/overview/  
_Source:_ https://www.rfc-editor.org/rfc/rfc9113.html

### Communication Protocols

통신 프로토콜은 `제어면(control plane)`과 `데이터면(data plane)`을 분리해서 보는 기존 결론이 그대로 맞다. 제어면에서는 Tauri IPC와 Windows named pipe가 여전히 가장 실용적이다. Microsoft Learn은 overlapped I/O를 사용하는 named pipe server가 고정 개수의 pipe instance를 만들고 여러 client 연결을 동시에 처리할 수 있다고 설명한다. 또한 named pipe 보안 문서는 pipe 생성 시 security descriptor를 지정해 client/server 끝점 접근을 제어할 수 있다고 명시한다.

데이터면에서는 shared memory/file mapping의 타당성이 더 분명하다. Microsoft의 `CreateFileMapping` 문서는 여러 프로세스가 같은 file mapping object의 view를 공유할 수 있으며, 동일 파일을 기반으로 한 mapped view는 같은 시점에 coherent하다고 설명한다. `Creating Named Shared Memory` 문서는 첫 번째 프로세스가 paging file-backed mapping을 만들고, 두 번째 프로세스가 같은 이름으로 열어 같은 메모리를 읽는 구조를 예시로 보여준다. 즉 `제어는 메시지`, `대용량 raster/intermediate buffer는 shared memory`라는 기존 방향은 여전히 타당하다.

여기서 중요한 제품적 해석은, 프리셋 적용 직후 반응성을 높이려면 JSON 기반 대용량 payload 왕복보다 `작은 제어 메시지 + 공유 버퍼 참조`가 유리하다는 점이다. 이 해석은 위 공식 문서가 직접 Boothy를 언급하는 것은 아니므로, 공식 자료 기반 추론이다.

_HTTP/HTTPS Protocols:_ gRPC/HTTP/2는 원격 서비스에는 강하지만, 로컬 same-machine hot path엔 우선순위가 낮다.  
_WebSocket Protocols:_ 지속 연결 장점은 있지만 로컬 렌더 제어에는 named pipe보다 직접성이 낮다.  
_Message Queue Protocols:_ RabbitMQ/Kafka/MQTT는 현장 단일 장비 구조에 과하다.  
_grpc and Protocol Buffers:_ 로컬 제어면에 쓸 수는 있으나, 현재는 lightweight local IPC가 더 실용적이다.  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-server-using-overlapped-i-o  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createfilemappinga  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/memory/creating-named-shared-memory  
_Source:_ https://www.rfc-editor.org/rfc/rfc9113.html

### Data Formats and Standards

데이터 형식은 `외부 호환용`과 `내부 실행용`을 분리하는 판단이 계속 맞다. Tauri command는 JSON 직렬화 가능 데이터를 전제로 하므로, UI와 앱 코어 사이의 명령/상태 교환은 JSON 계열이 자연스럽다. 반면 resident GPU service와의 내부 계약은 binary-friendly 형식이 더 맞다. Protocol Buffers 공식 문서는 protobuf가 JSON보다 더 작고 빠르며, 구조화된 typed data를 언어 중립적으로 직렬화할 수 있다고 설명한다. 또한 wire format의 backward compatibility 설계 원칙도 유지된다고 명시한다.

이 점은 기존 결론인 `canonical preset recipe`의 필요성을 강화한다. 내부 truth는 버전 진화가 가능한 명시적 schema여야 하고, 외부 호환은 XMP로 풀어야 한다. Adobe XMP 문서는 XMP가 metadata data model과 serialization, file embedding 가이드를 표준화한다고 설명한다. 따라서 `내부 canonical preset recipe + XMP adapter` 구조는 여전히 가장 현실적이다.

주의할 점도 있다. protobuf version support 문서는 binary wire format 안정성은 강하지만 JSON/textproto는 동일한 안정성 보장을 주지 않는다고 명시한다. 따라서 preset truth를 장기 호환 대상으로 둘 때는 human-readable JSON 파일 하나에만 의존하기보다, schema-driven contract를 두는 편이 안전하다.

_JSON and XML:_ UI-facing command/state와 XMP 호환 계층에는 계속 유효하다.  
_Protobuf and MessagePack:_ 내부 typed contract 후보로 적합하며, 특히 schema evolution이 장점이다.  
_CSV and Flat Files:_ 현재 렌더 hot path에는 비핵심이다.  
_Custom Data Formats:_ canonical preset recipe는 사실상 제품 핵심의 custom domain format이 된다.  
_Source:_ https://v2.tauri.app/concept/inter-process-communication/  
_Source:_ https://protobuf.dev/overview/  
_Source:_ https://protobuf.dev/support/version-support/  
_Source:_ https://developer.adobe.com/xmp/docs/xmp-specifications/

### System Interoperability Approaches

상호운용성 관점에서 기존 결론은 더 강해졌다. Boothy가 실제로 유지해야 할 상호운용성은 `다양한 원격 시스템 연동`이 아니라 `darktable baseline`, `새 GPU path`, `XMP sidecar`, `로컬 세션 저장소` 사이의 일관성이다. 따라서 point-to-point integration이 여전히 가장 맞는다. 즉 하나의 중심 truth를 두고 각 엔진이 adapter로 붙는 구조가 맞다.

API gateway, service mesh, ESB 같은 패턴은 여기서 실익이 거의 없다. 이 패턴들은 복수 네트워크 서비스의 traffic management와 observability에 강점이 있지만, Boothy의 주 문제는 한 대의 Windows 장비 안에서 프리셋 적용 속도와 품질 parity를 맞추는 것이다. 공식 자료를 다시 봐도 이 문제를 마이크로서비스 계층으로 올릴 근거는 없다. 오히려 Tauri IPC, named pipe, shared memory처럼 로컬 경계에 맞는 메커니즘이 더 직접적이다.

_Point-to-Point Integration:_ 현재 제품 구조에 가장 잘 맞는다.  
_API Gateway Patterns:_ 원격 운영 API가 생기기 전까지는 우선순위가 낮다.  
_Service Mesh:_ 현 단계에서는 과도하다.  
_Enterprise Service Bus:_ booth 제품 구조에는 맞지 않는다.  
_Source:_ https://v2.tauri.app/concept/inter-process-communication/  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-server-using-overlapped-i-o  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/memory/creating-named-shared-memory

### Microservices Integration Patterns

기존 기술 리서치에서 `modular monolith + local GPU service`를 권장했던 판단은 유지된다. 이번 검증에서도 microservices standard patterns를 다시 봤지만, API gateway, service discovery, saga, circuit breaker 같은 분산 시스템 패턴은 현 문제의 본질과 거리가 있다. Boothy는 네트워크 hop 여러 개를 거치는 서비스 묶음이 아니라, 현장 장비 내 프로세스/스레드/디바이스 자원 조율 문제를 풀고 있다.

다만 `circuit breaker`에 해당하는 운영 개념은 제품적으로 남는다. 즉 GPU path 실패 시 바로 baseline/fallback path로 전환하고, 세션을 망치지 않아야 한다. 이것은 엄밀한 마이크로서비스 패턴 구현이라기보다, 기존 리서치의 `fallback-first operational design`을 다시 지지하는 해석이다.

_API Gateway Pattern:_ 현 구조에서는 비우선이다.  
_Service Discovery:_ 로컬 고정 프로세스 구조에서는 필요성이 작다.  
_Circuit Breaker Pattern:_ 개념적으로는 GPU path fast-fail과 fallback 전환에 유용하다.  
_Saga Pattern:_ 분산 트랜잭션 문제가 아니라서 우선순위가 낮다.  
_Source:_ https://grpc.io/docs/what-is-grpc/introduction/  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-server-using-overlapped-i-o  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights

### Event-Driven Integration

이벤트 기반 통합은 여전히 중요하지만, 그 범위는 `로컬 상태 변화 브로드캐스트`로 한정하는 편이 맞다. Tauri IPC 문서는 events가 lifecycle과 state change 전달에 최적이라고 설명한다. 따라서 render service readiness, preset apply queued/started/completed, fallback entered, export progress 같은 신호는 event-driven으로 흘리는 것이 적합하다.

반면 Kafka/RabbitMQ 같은 외부 broker 기반 pub-sub는 현 구조에 과하다. Boothy가 필요한 것은 고가용성 분산 event bus가 아니라, same-machine에서 UI와 core와 render service가 낮은 지연으로 상태를 공유하는 구조다. CQRS나 event sourcing도 현재 hot path에 직접 필요한 것은 아니다. 다만 preset mutation history와 parity audit trail을 남기는 데는 제한적으로 참고할 가치가 있다. 이 역시 공식 문서와 제품 요구를 결합한 추론이다.

_Publish-Subscribe Patterns:_ 로컬 상태 변화 전파에는 적합하다.  
_Event Sourcing:_ 전체 제품 상태 저장 전략으로는 과할 수 있다.  
_Message Broker Patterns:_ 현장 단일 장비 구조에는 과도하다.  
_CQRS Patterns:_ 명령/상태 조회 분리는 유용하지만, full CQRS 도입 필요성은 낮다.  
_Source:_ https://v2.tauri.app/concept/inter-process-communication/  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/etw/about-event-tracing  
_Source:_ https://learn.microsoft.com/en-us/windows-hardware/test/weg/instrumenting-your-code-with-etw

### Integration Security Patterns

보안 패턴도 기존 판단과 같다. Boothy의 주 hot path는 외부 API가 아니라 로컬 desktop app이므로 OAuth 2.0, JWT, mTLS는 핵심이 아니다. 대신 Tauri capability system과 Windows object security가 더 중요하다. Tauri capability 문서는 capability file을 `src-tauri/capabilities`에 정의하고, window별 permissions와 platform 범위를 제한할 수 있다고 설명한다. 또한 기본적으로 API는 번들된 코드에만 접근 가능하며, remote source가 특정 command에 접근하려면 capability에서 별도 정의해야 한다고 명시한다.

Windows 측면에서는 named pipe security descriptor와 ACL로 client/server 접근을 통제할 수 있다. 따라서 제품적으로 필요한 보안 패턴은 `권한 최소화`, `UI가 호출할 수 있는 command 최소화`, `pipe/shared memory 접근 제한`, `remote API 비활성 기본값`이다. 이 방향은 `프리셋 적용 속도`를 해치지 않으면서도 로컬 공격면을 줄이는 가장 현실적인 구조다.

_OAuth 2.0 and JWT:_ 현재 로컬 렌더 hot path에는 비핵심이다.  
_API Key Management:_ 외부 서비스 연동이 커질 때 검토 대상이다.  
_Mutual TLS:_ same-machine local IPC에는 우선순위가 낮다.  
_Data Encryption:_ 외부 전송보다 로컬 권한 경계와 최소 권한 설계가 더 중요하다.  
_Source:_ https://v2.tauri.app/security/capabilities/  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createfilemappinga

## Architectural Patterns and Design

### System Architecture Patterns

이번 단계에서 가장 중요한 공식 근거는 `아키텍처 스타일은 제약의 집합`이라는 점이다. Azure Architecture Center는 아키텍처 스타일이 설계의 형태를 제한하고, 그 제약을 지킬 때 특정한 바람직한 속성이 생긴다고 설명한다. 또한 어떤 스타일을 선택할지는 구현 유행보다 `비기능 요구와 비즈니스 우선순위`를 기준으로 해야 하며, 때로는 순수성보다 실용성이 더 중요하다고 명시한다.

이 기준으로 Boothy를 다시 보면, 기존 결론인 `modular monolith + local GPU render service`가 여전히 가장 맞다. 마이크로서비스는 독립 배포, 데이터 독립성, 자율 팀 운영 같은 장점이 있지만, 그 제약은 네트워크 분산 시스템과 팀 구조를 전제로 할 때 가장 큰 가치를 낸다. 반면 Boothy의 주 문제는 한 대의 Windows 장비 안에서 `프리셋 적용 속도`, `품질`, `display/export parity`, `fallback 운영성`을 동시에 만족시키는 것이다. 따라서 마이크로서비스 제약을 억지로 적용하는 것보다, 로컬 애플리케이션 코어 위에 GPU 서비스를 분리하는 제한된 구조가 더 적합하다.

Direct3D 12 문서도 이 판단을 지지한다. D3D12는 이전 버전의 즉시 모드와 달리 command queue, command list, fence를 통해 개발자가 concurrency와 synchronization을 직접 관리하도록 만든다. 즉 GPU 자원과 작업 순서를 적극적으로 조절해야 성능 이점을 얻는 구조다. 이런 모델은 `resident GPU-first service`처럼 GPU 상태를 오래 유지하며 제어하는 아키텍처와 더 잘 맞는다.

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/microservices/migrate-monolith  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/direct3d12/executing-and-synchronizing-command-lists

### Design Principles and Best Practices

설계 원칙 관점에서도 기존 결론은 유지된다. 아키텍처 스타일 문서는 어떤 스타일이든 그 스타일의 제약과 이유를 먼저 이해해야 하고, 그렇지 않으면 겉모양만 닮은 설계가 된다고 경고한다. Boothy에 이 원칙을 적용하면, 핵심은 `로컬 interactive image system`이라는 문제 정의를 흔들지 않는 것이다.

따라서 현재 가장 타당한 설계 원칙은 아래로 정리된다.

- 앱 셸과 제품 플로우는 단순하게 유지
- GPU 계산과 상태 관리는 별도 서비스 경계로 격리
- preset truth는 엔진별 표현이 아니라 canonical schema에 고정
- display와 export는 같은 truth를 공유하되 다른 비용 구조로 실행
- fallback은 예외 처리 기능이 아니라 아키텍처의 일부로 설계

이 원칙은 `기술 선택`보다 `제품 목표에 맞는 제약을 먼저 고정`하는 접근이다. 기존 리서치의 방향과 완전히 일치하며, 이번 검증에서도 이를 뒤집는 최신 근거는 없었다.

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/  
_Source:_ https://v2.tauri.app/concept/architecture/  
_Source:_ https://developer.adobe.com/xmp/docs/xmp-specifications/

### Scalability and Performance Patterns

Boothy의 scalability는 일반적인 웹 서비스처럼 `수평 확장`이 아니라 `한 대의 장비 안에서 latency와 throughput을 동시에 관리`하는 문제다. 이 점에서 기존 리서치의 `display lane / export lane 분리`는 여전히 타당하다. Azure Cache-Aside 패턴 문서는 자주 읽는 데이터에 대해 cache miss 시 원본에서 읽고, 캐시에 넣고, 만료 정책과 priming 전략을 함께 설계해야 한다고 설명한다. 또한 원본과 캐시 사이의 완전한 일관성이 자동 보장되지는 않는다고 명시한다.

이 원리는 Boothy의 preview/cache에도 그대로 적용된다. 즉, preview cache나 intermediate artifact cache는 유효하지만, 만료 정책과 invalidation이 preset mutation과 맞물려야 한다. `빠른 프리셋 적용 반응성`을 위해 캐시를 적극적으로 쓰되, 품질 parity를 깨지 않도록 canonical recipe와 동기화해야 한다는 기존 결론이 유지된다.

또한 D3D12 메모리 관리 문서는 추천 전략을 `classify, budget and stream`이라고 명시한다. D3D12 residency starter library도 메모리 압박 상황에서 성능 저하를 줄이기 위해 명시적 residency 관리가 필요하다고 설명한다. 이는 Boothy가 resident GPU architecture를 택할 경우, 단순히 셰이더를 옮기는 것이 아니라 `VRAM budget`, `resource residency`, `streaming`을 설계 수준에서 포함해야 함을 보여준다.

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/cache-aside  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/direct3d12/memory-management  
_Source:_ https://learn.microsoft.com/en-us/samples/microsoft/directx-graphics-samples/d3d12-residency-starter-library-uwp/  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/direct3d12/executing-and-synchronizing-command-lists

### Integration and Communication Patterns

아키텍처 관점에서 통합 패턴 역시 그대로 유지된다. Tauri는 프론트엔드와 Rust backend를 IPC로 연결하고, D3D12는 command queue와 fence를 통해 GPU 작업을 명시적으로 조율한다. 이 두 층을 합치면, Boothy에 맞는 구조는 `UI ↔ app core ↔ resident render service`의 선명한 3계층이다.

여기서 중요한 설계 판단은 `제어 경로`와 `데이터 경로`를 분리하는 것이다. 제어 경로는 command/event와 local IPC로 작게 유지하고, 데이터 경로는 shared memory와 GPU resident resource 중심으로 최적화해야 한다. 이 구조는 복잡한 분산 메시징 패턴보다, 프리셋 적용 직후 사용자가 느끼는 latency를 줄이는 데 직접적이다.

_Source:_ https://v2.tauri.app/concept/inter-process-communication/  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-server-using-overlapped-i-o  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/memory/creating-named-shared-memory  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/direct3d12/executing-and-synchronizing-command-lists

### Security Architecture Patterns

보안 아키텍처는 이번 제품에서 네트워크 perimeter보다 로컬 권한 경계가 더 중요하다. Tauri capability 시스템은 window와 WebView별로 노출할 permission을 제한하고, 기본적으로 번들된 코드만 API에 접근하도록 설계되어 있다. 이는 Boothy에서 UI 계층이 render service나 시스템 자원에 과도하게 접근하지 않도록 막는 기본 장치가 된다.

여기에 Windows named pipe security와 shared memory object 권한 제어를 결합하면, 현재 제품에 맞는 보안 아키텍처는 `minimum privilege local system`이다. 다시 말해, 외부 인증 체계보다 `누가 어떤 local command와 메모리 영역에 접근할 수 있는가`를 통제하는 것이 핵심이다. 이 방향은 기존 리서치와 동일하다.

_Source:_ https://v2.tauri.app/security/capabilities/  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createfilemappinga

### Data Architecture Patterns

데이터 아키텍처에서는 `single source of truth`가 더 중요해졌다. XMP는 호환 계층으로 충분히 유효하지만, 제품 내부의 핵심 진실은 여전히 `canonical preset recipe`로 분리하는 쪽이 맞다. 이유는 명확하다. preview cache, export queue, fallback engine, baseline oracle이 모두 같은 의도를 재현해야 하기 때문이다.

또한 Cache-Aside 패턴 문서가 지적하듯, 캐시는 원본과 자동으로 완전 일치하지 않는다. 따라서 cache나 precomputed preview를 공격적으로 쓰더라도, 원본 truth가 recipe라는 점을 흔들면 안 된다. SQLite + filesystem + XMP adapter + canonical recipe 조합은 이 요구를 가장 잘 충족한다.

_Source:_ https://developer.adobe.com/xmp/docs/xmp-specifications/  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/cache-aside  
_Source:_ https://learn.microsoft.com/en-us/windows/apps/develop/data-access/sqlite-data-access  
_Source:_ https://sqlite.org/wal.html

### Deployment and Operations Architecture

운영 아키텍처 측면에서도 기존 결론은 유지되며, 일부는 더 강해졌다. Windows App SDK release channels 문서는 `Stable`, `Preview`, `Experimental` 채널을 분리해 운영한다고 설명하고, stable은 프로덕션용, preview와 experimental은 탐색용이라고 명시한다. 또한 gradual package rollout 문서는 Windows 앱 업데이트를 기존 설치 고객의 일부 비율에만 먼저 배포하고, 분석을 본 뒤 확대하거나 중단할 수 있다고 설명한다.

이 두 문서는 Boothy의 운영 전략에 직접적인 힌트를 준다. 즉 새 GPU path를 한 번에 주력으로 바꾸기보다, `stable / pilot / experimental`에 가까운 운영 링을 두고 점진적으로 승격하는 방식이 맞다. 문제 발생 시 halt하고 이전 패키지 경로를 유지할 수 있는 배포 구조는 `fallback-first` 아키텍처와 자연스럽게 맞물린다.

결론적으로, 아키텍처 패턴 관점에서도 기존 판단은 거의 그대로 유지된다. `resident GPU-first + canonical preset truth + separated display/export lanes + darktable baseline/fallback + staged rollout`이 여전히 가장 제품 목표에 잘 맞는 구조다.

_Source:_ https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/release-channels  
_Source:_ https://learn.microsoft.com/en-us/windows/apps/publish/gradual-package-rollout  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/etw/about-event-tracing

## Implementation Approaches and Technology Adoption

### Technology Adoption Strategies

구현 전략 측면에서 가장 강한 최신 근거는 `big bang보다 gradual displacement가 안전하다`는 점이다. Martin Fowler의 `Transitional Architecture` 문서는 성공적인 legacy displacement의 핵심이 점진적 교체이며, 이렇게 해야 이점을 일찍 얻고 big bang의 위험을 피할 수 있다고 설명한다. 또한 교체 기간 동안에는 old와 new가 동시에 동작하며 역할 분담이 계속 바뀐다고 명시한다. 이 설명은 Boothy의 현재 상황과 거의 정확히 맞아떨어진다.

즉 기존 경로를 당장 버리고 새 GPU path만 남기는 방식은 이번 검증 기준에서도 맞지 않는다. 대신 `darktable baseline/fallback 유지 + 새 resident GPU lane 추가 + traffic/환경별 점진 승격`이 가장 현실적인 도입 방식이다. `Strangler Fig`와 `Legacy Seam`도 같은 맥락을 지지한다. 기존 시스템에 seam을 만들고, 작은 단위로 행동을 새 구조로 우회시키는 접근이 필요하다는 점이다.

_Source:_ https://martinfowler.com/articles/patterns-legacy-displacement/transitional-architecture.html  
_Source:_ https://martinfowler.com/articles/2024-strangler-fig-rewrite.html  
_Source:_ https://martinfowler.com/bliki/LegacySeam.html

### Development Workflows and Tooling

개발 워크플로는 `기능 개발`보다 `측정 가능한 엔진 교체`에 맞게 설계되어야 한다. Microsoft는 WPR이 ETW 기반 recording tool이며, WPA가 그 결과를 그래프와 데이터 테이블로 분석하는 도구라고 설명한다. 기존 리서치에서 제안한 `ETW + WPR/WPA + PIX` 조합은 이번 단계에서도 그대로 유효하다. 즉 구현 워크플로는 코드 작성보다 먼저 `측정 프로파일`, `latency trace`, `fallback 전환 추적`, `VRAM 압박 관찰`이 준비돼야 한다.

또한 Operational Excellence 체크리스트는 표준화된 도구, 소스 제어, 품질 게이트, 자동 파이프라인, telemetry 기반 검증을 강조한다. Boothy에 맞게 해석하면, 새 GPU 경로는 일반 기능 브랜치처럼 다루기보다 `측정 스크립트`, `비교 샘플셋`, `자동 parity 검사`, `운영 채널 구분`을 기본으로 갖춘 전용 개발 흐름이 맞다.

_Source:_ https://learn.microsoft.com/en-us/windows-hardware/test/wpt/windows-performance-recorder  
_Source:_ https://learn.microsoft.com/en-us/windows-hardware/test/wpt/windows-performance-analyzer  
_Source:_ https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/checklist  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/etw/about-event-tracing

### Testing and Quality Assurance

테스트 전략은 일반 unit test보다 `성능 + 품질 parity + 운영 복구`에 맞춰져야 한다. 이번 검증 기준에서 가장 중요한 QA 기준은 아래 세 가지다.

- 프리셋 적용 후 사용자가 보는 풀사이즈 반응 속도
- 최종 export와의 visual parity
- GPU path 실패 시 fallback이 세션을 망치지 않는지

`Transitional Architecture`와 `Canary Release` 패턴은 새 경로를 일부 사용자 또는 일부 흐름에만 먼저 붙여 검증하는 접근을 지지한다. 또한 Windows gradual rollout 문서는 일부 고객 비율에만 먼저 배포하고, 분석을 보며 확대하거나 중단할 수 있다고 설명한다. 이 점을 제품 QA에 대입하면, 테스트는 단순히 로컬 bench pass가 아니라 `pilot cohort`, `thin slice`, `fallback rehearse`, `package halt 가능성`까지 포함해야 한다.

_Source:_ https://martinfowler.com/articles/patterns-legacy-displacement/transitional-architecture.html  
_Source:_ https://martinfowler.com/articles/patterns-legacy-displacement/canary-release.html  
_Source:_ https://learn.microsoft.com/en-us/windows/apps/publish/gradual-package-rollout  
_Source:_ https://learn.microsoft.com/en-us/windows-hardware/drivers/develop/safe-deployment-best-practices-for-drivers

### Deployment and Operations Practices

배포와 운영은 이번 구조 변화의 성패를 좌우한다. Windows 앱 gradual rollout은 업데이트를 설치 고객의 일부에게만 먼저 배포하고, 문제가 보이면 halt할 수 있다고 명시한다. Safe deployment best practices 문서도 배포를 pre-deployment, distribution, post-deployment monitoring으로 나누고, telemetry를 바탕으로 pause, rollback, patch를 수행하라고 권고한다.

이 점을 Boothy에 적용하면, 새 GPU path는 아래 형태로 배포하는 것이 맞다.

- stable에서는 기존 baseline 우선
- pilot에서는 GPU path opt-in 또는 조건부 활성화
- experimental에서는 더 넓은 기능 범위 검증
- 문제 발생 시 즉시 halt 또는 fallback 강제

즉 운영 방식까지 포함해 보면, 기존 리서치의 `staged rollout + fallback-first` 전략은 그대로 유지될 뿐 아니라 더 강해졌다.

_Source:_ https://learn.microsoft.com/en-us/windows/apps/publish/gradual-package-rollout  
_Source:_ https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/release-channels  
_Source:_ https://learn.microsoft.com/en-us/windows-hardware/drivers/develop/safe-deployment-best-practices-for-drivers

### Team Organization and Skills

팀 구성 역시 기존 판단이 유지된다. Operational Excellence는 명확한 역할, 표준화된 프로세스, incident 관리, telemetry 검증을 강조한다. 따라서 이 작업은 단순히 렌더 코드를 잘 짜는 개발자 한 명의 문제가 아니라, 최소한 아래 역량이 같이 필요하다.

- Rust/Tauri 제품 코어 담당
- GPU/D3D12 성능 담당
- 품질 parity 판단 담당
- 운영/배포 및 incident 대응 담당

다만 팀 규모를 크게 늘리는 것보다, 소수 인원이 `contract + prototype + measurement + rollout`을 짧게 반복하는 구조가 더 맞다. 구현 난이도는 높지만 문제 정의가 선명하므로, 큰 조직보다 작은 핵심팀이 더 효율적이라는 판단은 유지된다.

_Source:_ https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/checklist  
_Source:_ https://learn.microsoft.com/en-us/windows-hardware/test/weg/instrumenting-your-code-with-etw

### Cost Optimization and Resource Management

비용 최적화 측면에서도 기존 결론은 바뀌지 않는다. 가장 큰 낭비는 `고성능 GPU 장비를 두고 CPU 중심 경로를 계속 미세조정하는 것`이다. 반면 resident GPU path를 도입하더라도 VRAM/driver/initialization 비용을 관리하지 않으면 새 낭비가 생긴다. D3D12 memory management와 residency starter library는 메모리 budget과 residency 관리가 없으면 실제 성능 저하가 클 수 있음을 보여준다.

따라서 비용 최적화는 클라우드 비용 절감보다 `현장 장비 자원 활용 최적화`에 가깝다. 즉,

- preview lane부터 범위를 제한해 실험
- baseline 유지로 실패 비용 절감
- export lane은 두 번째 단계로 분리
- telemetry 없는 구현 반복 금지

가 더 맞다.

_Source:_ https://learn.microsoft.com/en-us/windows/win32/direct3d12/memory-management  
_Source:_ https://learn.microsoft.com/en-us/samples/microsoft/directx-graphics-samples/d3d12-residency-starter-library-uwp/  
_Source:_ https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/checklist

### Risk Assessment and Mitigation

리스크와 완화 전략은 이번 단계에서 더 선명해졌다.

- **품질 회귀 리스크**
  - 완화: canonical preset recipe 유지, baseline parity oracle 유지, pilot cohort 검증
- **GPU 불안정/드라이버 리스크**
  - 완화: resident service 격리, startup health check, fallback 강제 경로
- **속도는 빨라졌지만 운영이 무너지는 리스크**
  - 완화: gradual rollout, halt 가능 배포, ETW/WPR/WPA telemetry
- **구현 범위가 과도해지는 리스크**
  - 완화: 첫 단계는 display/preset apply lane에 한정

이 리스크 모델은 driver safe deployment 문서와 operational excellence 문서가 강조하는 `small release`, `quality gate`, `telemetry`, `rollback planning`과 일치한다.

_Source:_ https://learn.microsoft.com/en-us/windows-hardware/drivers/develop/safe-deployment-best-practices-for-drivers  
_Source:_ https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/checklist  
_Source:_ https://learn.microsoft.com/en-us/windows/apps/publish/gradual-package-rollout

## Technical Research Recommendations

### Implementation Roadmap

가장 현실적인 구현 순서는 아래다.

1. `canonical preset recipe` 최소 스키마를 먼저 고정
2. 기존 darktable 경로를 `baseline / fallback / parity oracle`로 명시 재정의
3. `display + preset apply` 전용 resident GPU prototype 구현
4. ETW + WPR/WPA + PIX 기반 측정 체계 연결
5. visual parity diff와 latency KPI 자동 비교
6. `pilot` 환경에만 조건부 활성화
7. 검증 후 `export lane`을 별도 단계로 구현

### Technology Stack Recommendations

- 앱 셸: `Tauri + Rust` 유지
- 1차 GPU 경로: `D3D12 + HLSL`
- 기준선/복구: `darktable OpenCL/CLI`
- 데이터 진실: `canonical preset recipe + XMP adapter`
- 저장소: `SQLite + filesystem`
- 계측: `ETW + WPR/WPA + PIX`
- 로컬 통신: `Tauri IPC + named pipe + shared memory`

### Skill Development Requirements

- Rust/Tauri 제품 코어 운용
- D3D12/HLSL 및 GPU 메모리 관리
- ETW/WPR/WPA/PIX 계측
- visual parity 평가 체계
- 단계적 배포와 incident 대응

### Success Metrics and KPIs

- `프리셋 적용 후 풀사이즈 반응 속도`
- `최종 export와 display의 visual parity`
- `GPU path 실패 시 fallback 성공률`
- `pilot 환경에서의 세션 안정성`
- `RAW 200장 export 총 시간`
- `잘못된 세션 간 오염 / 잘못된 샷 반영 = 0`

## Research Synthesis

# Boothy GPU-first Rendering Architecture Validation: Comprehensive Technical Research

## Executive Summary

이번 검증의 결론은 명확하다. 2026-04-11에 작성된 기존 도메인 리서치와 기술 리서치의 핵심 방향은 최신 공개 자료를 다시 확인해도 `대체로 동일`하게 유지된다. 즉 Boothy의 다음 주력 구조는 여전히 `resident GPU-first architecture`가 맞고, 기존 darktable 경로는 버릴 대상이 아니라 `baseline`, `fallback`, `parity oracle`로 재정의하는 편이 가장 현실적이다. 또한 `canonical preset recipe`, `display lane / export lane 분리`, `점진 승격`이라는 기존 판단도 그대로 유효하다.

다만 이번 검증을 통해 한 가지는 더 강해졌다. 이제 GPU-first는 단순한 처리량 선택이 아니라, `품질을 유지한 채 사용자가 체감하는 프리셋 적용 속도`를 만족시키기 위한 더 직접적인 수단이라는 점이다. Adobe는 `2025-08-13`에 Lightroom Classic `14.5`부터 Preview Generation에도 GPU를 사용할 수 있다고 공식화했고, 이는 업계가 display/image processing/export를 넘어 preview responsiveness 자체를 GPU 쪽으로 더 끌어오고 있음을 보여준다.

**Key Technical Findings:**

- 기존 핵심 결론은 유지된다: `resident GPU-first + darktable baseline/fallback + canonical preset recipe + lane 분리`
- 가장 중요한 최신 강화 근거는 `preview generation`까지 GPU 적용 범위를 확대한 Adobe 공식 흐름이다.
- Boothy의 문제는 웹서비스 분산 설계가 아니라 `한 대의 Windows 장비 안에서 품질과 프리셋 반응성을 같이 닫는 구조`다.
- 따라서 `modular monolith + local GPU service`가 여전히 가장 적합하다.
- 구현 전략은 `big bang replacement`가 아니라 `display + preset apply lane`부터 시작하는 점진 전환이 맞다.

**Technical Recommendations:**

- `canonical preset recipe`를 먼저 고정한다.
- `darktable`를 주력 엔진이 아니라 `baseline / fallback / parity oracle`로 명확히 재정의한다.
- 첫 구현 타겟은 `display + preset apply` resident GPU prototype으로 제한한다.
- `ETW + WPR/WPA + PIX + visual parity diff`를 먼저 갖춘다.
- `stable / pilot / experimental`에 가까운 점진 승격 구조로 운영한다.

## Table of Contents

1. Technical Research Introduction and Methodology
2. Technical Landscape and Validation Thesis
3. Technology Stack Validation
4. Integration and Interoperability Validation
5. Architecture Validation
6. Implementation Strategy Validation
7. Performance, Quality, and Product-Fit Interpretation
8. Strategic Technical Recommendations
9. Implementation Roadmap and Risk Assessment
10. Future Technical Outlook
11. Technical Research Methodology and Source Verification
12. Reference Materials

## 1. Technical Research Introduction and Methodology

### Technical Research Significance

이번 검증 리서치가 중요한 이유는, Boothy의 현재 과제가 단순한 렌더 최적화가 아니라 `품질`, `프리셋 적용 속도`, `display/export parity`, `운영 안정성`을 동시에 만족해야 하는 구조 문제이기 때문이다. 기존 리서치는 이미 GPU-first 방향을 제안했지만, 실제 제품 판단에는 `정말 그 결론이 최신 공개 자료를 다시 봐도 유지되는가`를 확인하는 절차가 필요했다.

_Technical Importance:_ 기존 구조 개선이 아니라 주력 아키텍처 방향 자체를 검증하는 작업이었다.  
_Business Impact:_ 잘못된 구조 선택은 사용자 체감 속도와 품질을 동시에 해칠 수 있다.  
_Source:_ https://helpx.adobe.com/ee/lightroom-classic/kb/lightroom-gpu-faq.html  
_Source:_ https://helpx.adobe.com/si/lightroom-classic/kb/gpu-preview-generation.html

### Technical Research Methodology

- **Technical Scope**: 기술 스택, 통합 패턴, 아키텍처, 구현 전략, 운영 방식
- **Primary Inputs**: 기존 도메인 리서치, 기존 기술 리서치, 최신 공식 문서
- **Verification Approach**: Adobe, Microsoft, Tauri, darktable, SQLite, Martin Fowler 자료 우선
- **Analysis Framework**: `결론 유지 / 강화 / 수정 / 보류` 기준으로 평가
- **Time Focus**: `2026-04-11` 기준 최신 공개 자료 확인

### Technical Research Goals and Objectives

**Original Technical Goals:** 2026-04-11에 작성된 최근 도메인/기술 리서치를 기준선으로 삼아, 최신 공개 기술 자료와 비교했을 때 핵심 결론이 유지되는지 검증한다. 특히 resident GPU-first 구조, darktable의 baseline/fallback 역할, canonical preset recipe, display/export 분리의 타당성을 재확인한다.

**Achieved Technical Objectives:**

- 기존 결론이 `대체로 동일`하게 유지됨을 확인
- `프리셋 적용 속도`까지 포함하면 GPU-first의 필요성이 오히려 더 커졌음을 확인
- darktable의 최적 역할이 `주력 UX 엔진`보다 `baseline/fallback/reference`에 가깝다는 판단을 재확인
- 실행 우선순위가 `display + preset apply lane`이라는 점을 더 명확히 정리

## 2. Technical Landscape and Validation Thesis

### Validation Thesis

이번 검증의 핵심 명제는 아래 한 문장으로 요약된다.

`Boothy는 GPU를 많이 쓰는 제품이 되어야 하는 것이 아니라, 품질과 프리셋 반응성을 함께 만족시키기 위해 GPU를 중심 자원으로 재배치한 제품이 되어야 한다.`

이 명제는 기존 리서치의 결론과 동일하며, 최신 Adobe 공식 자료가 preview generation까지 GPU 적용을 확장한 점 때문에 더 강해졌다.

### What Stayed the Same

- `resident GPU-first`가 주력 방향이라는 판단
- `darktable`를 유지하되 역할을 재정의해야 한다는 판단
- `canonical preset recipe`가 필요하다는 판단
- `display lane / export lane 분리`가 필요하다는 판단
- `staged rollout`과 `fallback-first`가 필요하다는 판단

### What Became Stronger

- GPU는 단순 가속이 아니라 preview responsiveness까지 포함한 제품 구조 자원이라는 점
- local IPC, shared memory, resident state 유지가 product-fit이 높다는 점
- full rewrite보다 점진 전환이 더 안전하다는 구현 전략

## 3. Technology Stack Validation

### Current Technology Stack Landscape

최신 문서 기준으로도 Boothy의 가장 현실적인 스택은 크게 변하지 않았다.

- 앱 셸: `Tauri + Rust`
- 1차 GPU 후보: `Direct3D 12 + HLSL`
- 기준선/복구: `darktable OpenCL/CLI`
- 저장소: `SQLite + filesystem`
- 메타데이터 호환: `XMP`
- 계측: `ETW + WPR/WPA + PIX`

이 구성은 Windows 현장 장비라는 제품 제약과 가장 잘 맞는다.

### Stack-Level Interpretation

중요한 변화는 스택이 바뀐 것이 아니라, 각 스택 요소의 우선순위가 더 명확해졌다는 점이다. 특히 Adobe의 `GPU for Preview Generation` 공개는 Boothy가 GPU를 `display/export 보조자원`이 아니라 `프리셋 적용 직후 체감 반응성`까지 닫는 핵심 자원으로 봐야 함을 뒷받침한다.

_Source:_ https://v2.tauri.app/concept/architecture/  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/direct3d12/pipelines-and-shaders-with-directx-12  
_Source:_ https://helpx.adobe.com/si/lightroom-classic/kb/gpu-preview-generation.html  
_Source:_ https://docs.darktable.org/usermanual/development/en/special-topics/program-invocation/darktable-cli/  
_Source:_ https://learn.microsoft.com/en-us/windows/apps/develop/data-access/sqlite-data-access

## 4. Integration and Interoperability Validation

### Current Integration Approach

통합 구조는 여전히 `same-machine local integration`이 맞다.

- `UI ↔ core`: Tauri Commands + Events
- `core ↔ resident render service`: local IPC
- 대용량 데이터: shared memory / file mapping
- 외부 호환: XMP adapter
- 내부 진실: canonical preset recipe

이 구조는 프리셋 적용 속도와 품질 parity 요구를 동시에 충족시키는 데 가장 직접적이다.

### Interoperability Principle

Boothy의 상호운용성 핵심은 `외부 SaaS 연동`이 아니라 `darktable baseline`, `GPU path`, `preview cache`, `export path`가 같은 의도를 재현하는 것이다. 따라서 `single source of truth`를 canonical recipe로 두는 판단은 이번 검증에서도 유지된다.

_Source:_ https://v2.tauri.app/concept/inter-process-communication/  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-server-using-overlapped-i-o  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/memory/creating-named-shared-memory  
_Source:_ https://developer.adobe.com/xmp/docs/xmp-specifications/

## 5. Architecture Validation

### Current Architecture Decision

아키텍처 관점에서 가장 적합한 형태는 계속 `modular monolith + local GPU service`다. 마이크로서비스의 장점은 분명하지만, Boothy의 핵심 문제는 네트워크 분산 시스템 설계가 아니라 `한 대의 Windows 장비 안에서 빠르고 정확하고 안전한 렌더링 구조를 만드는 것`이다.

### Why the Existing Recommendation Still Holds

- D3D12는 명시적 queue/synchronization/residency 관리를 전제로 한다.
- preview와 export는 같은 recipe를 공유하되 다른 비용 구조가 필요하다.
- fallback은 optional feature가 아니라 아키텍처 일부로 설계해야 한다.
- cache는 강하게 쓰되, truth는 recipe 쪽에 남겨야 한다.

즉 `resident GPU-first + darktable baseline/fallback + separated lanes`는 여전히 가장 product-fit이 높은 구조다.

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/direct3d12/executing-and-synchronizing-command-lists  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/direct3d12/memory-management  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/cache-aside

## 6. Implementation Strategy Validation

### Recommended Migration Pattern

구현 전략은 `big bang replacement`가 아니라 `transitional architecture`가 맞다. 즉 기존 경로와 새 경로가 일정 기간 공존하며, 새 GPU lane이 작은 범위부터 점진적으로 승격되는 방식이 안전하다.

### Practical Implementation Thesis

첫 구현 타겟은 전체 파이프라인이 아니라 `display + preset apply`다. 이유는 이 구간이 현재 사용자 불편과 가장 직접적으로 연결되고, product value를 가장 빨리 확인할 수 있기 때문이다. export lane은 그 다음 단계로 분리하는 것이 맞다.

_Source:_ https://martinfowler.com/articles/patterns-legacy-displacement/transitional-architecture.html  
_Source:_ https://martinfowler.com/articles/2024-strangler-fig-rewrite.html  
_Source:_ https://martinfowler.com/bliki/LegacySeam.html

## 7. Performance, Quality, and Product-Fit Interpretation

### Product-Centered Reading of the Research

이번 검증은 단순히 `GPU를 더 써야 한다`는 결론으로 끝나지 않는다. 제품 관점의 진짜 결론은 아래다.

- 품질을 해치면 GPU-first는 의미가 없다.
- 프리셋 적용 직후 체감 반응성이 느리면 품질만 좋아도 제품 만족은 떨어진다.
- display와 export가 어긋나면 신뢰가 깨진다.
- fallback이 세션을 망치면 현장 제품으로 쓸 수 없다.

따라서 최종 판단 기준은 계속 `품질 + 프리셋 적용 속도 + parity + 운영 안정성`의 4축이다.

### KPI Interpretation

이번 리서치가 실무적으로 요구하는 KPI는 아래다.

- 프리셋 적용 후 풀사이즈 반응 시간
- display/export visual parity
- GPU path 실패 시 fallback 성공률
- pilot 세션 안정성
- batch export 처리 시간

이 다섯 개를 동시에 보지 않으면, 속도 개선만 보고 잘못된 의사결정을 내릴 수 있다.

## 8. Strategic Technical Recommendations

### Primary Recommendation

가장 현실적인 전략은 아래 조합이다.

1. `canonical preset recipe`를 먼저 고정한다.
2. `darktable`를 baseline/fallback/parity oracle로 명확히 남긴다.
3. `display + preset apply` resident GPU lane을 1차 구현 대상으로 잡는다.
4. `ETW + WPR/WPA + PIX + parity diff`를 함께 만든다.
5. `stable / pilot / experimental` 구조로 점진 승격한다.

### Competitive Technical Advantage

이 전략의 장점은 `지금 가진 제품을 버리지 않으면서`, `사용자가 바로 느끼는 프리셋 반응성`을 우선적으로 개선할 수 있다는 점이다. 즉 기술적으로도, 제품적으로도 가장 손실이 적고 학습이 빠른 경로다.

## 9. Implementation Roadmap and Risk Assessment

### Implementation Phases

1. canonical preset recipe 최소 스키마 정의
2. darktable 역할 재정의 및 baseline contract 확정
3. display + preset apply resident GPU prototype
4. ETW/WPR/WPA/PIX 및 parity diff 연결
5. pilot cohort 검증
6. export lane 확장

### Major Risks

- 품질 회귀
- GPU/driver 불안정
- preview는 빨라졌지만 export parity가 깨지는 문제
- 범위 과확장으로 인한 일정 붕괴

### Mitigation

- baseline oracle 유지
- staged rollout
- telemetry 중심 검증
- 첫 단계 범위 제한

## 10. Future Technical Outlook

향후 1~2년 관점에서는 `preview responsiveness`까지 포함한 GPU path 강화가 더 일반화될 가능성이 높다. 따라서 Boothy가 지금 GPU-first 구조를 product-fit 중심으로 재설계하는 방향은 시대 흐름과도 어긋나지 않는다.

중기적으로는 `canonical preset recipe`가 더 큰 전략 자산이 된다. 이 레이어가 있어야 engine 교체, preview/export 분리, cache 최적화, fallback 운영을 모두 유연하게 다룰 수 있다.

## 11. Technical Research Methodology and Source Verification

### Primary Source Families

- Adobe HelpX / Adobe Developer
- Microsoft Learn / Windows documentation
- Tauri 공식 문서
- darktable 공식 문서
- SQLite 공식 문서
- Martin Fowler 아키텍처 자료

### Confidence Assessment

- 아키텍처 방향: `High`
- 구현 우선순위: `High`
- 최종 엔진 선택 확정: `Medium`
  - 이유: 최종 확정은 prototype benchmark와 visual parity 측정이 필요

### Technical Limitations

이번 검증은 공개 공식 문서와 기존 리서치의 교차 검증이다. 실제 최종 엔진 채택 여부는 Boothy의 실샷 샘플셋, GPU 장비, 드라이버 환경, preset complexity를 반영한 내부 benchmark가 필요하다.

## 12. Reference Materials

### Local References

- [domain-raw-photo-preset-gpu-first-research-2026-04-11.md](/C:/Code/Project/Boothy_thumbnail-reset-at-2c89c40/_bmad-output/planning-artifacts/research/domain-raw-photo-preset-gpu-first-research-2026-04-11.md)
- [technical-boothy-gpu-first-rendering-architecture-research-2026-04-11.md](/C:/Code/Project/Boothy_thumbnail-reset-at-2c89c40/_bmad-output/planning-artifacts/research/technical-boothy-gpu-first-rendering-architecture-research-2026-04-11.md)

### External References

- https://helpx.adobe.com/ee/lightroom-classic/kb/lightroom-gpu-faq.html
- https://helpx.adobe.com/si/lightroom-classic/kb/gpu-preview-generation.html
- https://v2.tauri.app/concept/architecture/
- https://v2.tauri.app/concept/inter-process-communication/
- https://v2.tauri.app/security/capabilities/
- https://learn.microsoft.com/en-us/windows/win32/direct3d12/pipelines-and-shaders-with-directx-12
- https://learn.microsoft.com/en-us/windows/win32/direct3d12/executing-and-synchronizing-command-lists
- https://learn.microsoft.com/en-us/windows/win32/direct3d12/memory-management
- https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-server-using-overlapped-i-o
- https://learn.microsoft.com/en-us/windows/win32/memory/creating-named-shared-memory
- https://learn.microsoft.com/en-us/windows/apps/publish/gradual-package-rollout
- https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/release-channels
- https://learn.microsoft.com/en-us/windows-hardware/test/wpt/windows-performance-recorder
- https://learn.microsoft.com/en-us/windows-hardware/test/wpt/windows-performance-analyzer
- https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/
- https://docs.darktable.org/usermanual/development/en/special-topics/program-invocation/darktable-cli/
- https://developer.adobe.com/xmp/docs/xmp-specifications/
- https://sqlite.org/wal.html
- https://martinfowler.com/articles/patterns-legacy-displacement/transitional-architecture.html
- https://martinfowler.com/articles/2024-strangler-fig-rewrite.html
- https://martinfowler.com/bliki/LegacySeam.html

---

## Technical Research Conclusion

### Summary of Key Technical Findings

기존 리서치의 핵심 결론은 최신 공식 자료로 다시 봐도 유지된다. Boothy는 `resident GPU-first`가 맞고, `darktable baseline/fallback`, `canonical preset recipe`, `display/export 분리`, `staged rollout`이 모두 여전히 타당하다.

### Strategic Technical Impact Assessment

이번 검증으로 더 분명해진 것은, GPU-first가 단순 성능 선택이 아니라 `품질을 유지하면서 프리셋 적용 속도를 개선하기 위한 제품 구조 선택`이라는 점이다. 따라서 다음 제품 투자 우선순위는 전체 재작성보다 `display + preset apply` 경로를 빠르게 검증하는 쪽에 두는 것이 맞다.

### Next Steps Technical Recommendations

1. `canonical preset recipe` 정의를 바로 시작한다.
2. `darktable`의 역할을 baseline/fallback/parity oracle로 문서화한다.
3. `display + preset apply` resident GPU prototype을 착수한다.
4. `ETW/WPR/WPA/PIX + parity diff` 측정 체계를 먼저 만든다.
5. `pilot` 환경에서 점진 검증한다.

---

**Technical Research Completion Date:** 2026-04-11  
**Research Period:** current comprehensive technical validation  
**Source Verification:** official technical sources prioritized  
**Technical Confidence Level:** High for direction, Medium for final engine promotion pending prototype benchmarks
