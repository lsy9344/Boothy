---
stepsCompleted: [1, 2, 3, 4, 5, 6]
inputDocuments:
  - "docs/architecture-change-foundation-2026-04-11.md"
workflowType: 'research'
lastStep: 1
research_type: 'technical'
research_topic: 'Boothy GPU-first 렌더링 아키텍처'
research_goals: '고객이 보는 풀사이즈 결과와 최종 export 품질의 parity를 유지하면서 GPU를 최대 활용하는 새 렌더링 구조를 검토한다.'
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

이 리서치는 Boothy가 더 이상 `썸네일 속도`가 아니라 `고객이 실제로 보는 풀사이즈 결과의 표시 속도`와 `세션 종료 전후 대량 export 처리`를 동시에 만족해야 한다는 전제에서 시작했다. 출발 문서인 [architecture-change-foundation-2026-04-11.md](/C:/Code/Project/Boothy_thumbnail-latency-seam-reinstrumentation/docs/architecture-change-foundation-2026-04-11.md)을 기준으로, 현재 구조의 한계, darktable의 현실적 위치, 그리고 `GPU 최대 활용`이라는 새 목적을 중심에 두고 기술 스택, 통합 패턴, 아키텍처 패턴, 구현 전략을 단계적으로 검증했다.

조사 결과의 핵심은 명확하다. Boothy의 다음 주력 구조는 `GPU를 옵션으로 쓰는 구조`가 아니라 `always-warm resident GPU render service`를 중심으로 재설계되어야 한다. 동시에 기존 darktable 경로는 버릴 대상이 아니라 `baseline`, `fallback`, `parity oracle`로 유지하는 편이 제품 리스크를 크게 줄인다. 세부 근거와 권고안은 문서 하단의 `Research Synthesis`와 `Technical Research Conclusion`에 종합했다.

---

## Technical Research Scope Confirmation

**Research Topic:** Boothy GPU-first 렌더링 아키텍처  
**Research Goals:** 고객이 보는 풀사이즈 결과와 최종 export 품질의 parity를 유지하면서 GPU를 최대 활용하는 새 렌더링 구조를 검토한다.

**Technical Research Scope:**

- Architecture Analysis - GPU-first resident 구조, warm context, display/export 분리
- Implementation Approaches - darktable 재활용, 새 GPU 런타임, hybrid 구조
- Technology Stack - Windows 중심 언어, 프레임워크, GPU API, 도구, 플랫폼
- Integration Patterns - preset recipe canonicalization, parity, fallback, interoperability
- Performance Considerations - 2.5초 display SLA, RAW 200장 export throughput, cold-start 최소화

**Research Methodology:**

- Current web data with rigorous source verification
- Multi-source validation for critical technical claims
- Confidence level framework for uncertain information
- Comprehensive technical coverage with architecture-specific insights

**Scope Confirmed:** 2026-04-11

## Technology Stack Analysis

### Programming Languages

Boothy의 현재 운영 맥락과 새 아키텍처 후보를 같이 보면, 핵심 언어 축은 `Rust + GPU shader language + 제한적 UI language` 조합으로 정리된다.

- **주력 애플리케이션 언어는 Rust 유지가 유력**: Tauri는 Rust 백엔드와 WebView 프론트엔드를 결합하는 구조이며, 현재 프로젝트 구조와도 정합성이 높다. 런타임 제어, 큐 관리, 파일 I/O, 디바이스 라이프사이클, 오류 격리 같은 booth 핵심 책임은 계속 Rust에 두는 편이 자연스럽다.
- **GPU path는 API별 shader language가 사실상 결정 요소**:
  - `Direct3D 12` 경로는 HLSL 기반 compute pipeline과 PSO 관리가 기본이다.
  - `OpenCL` 경로는 OpenCL C와 OpenCL API를 사용하며, darktable 계열과 연결성이 좋다.
  - `Vulkan` 경로는 C/C++/Rust 호스트 코드와 SPIR-V 기반 shader toolchain을 전제로 하며, 보다 명시적 제어가 가능하다.
  - `CUDA` 경로는 CUDA C++ 중심이라 NVIDIA에서만 강하지만, Windows 범용 booth 제품에는 벤더 종속 리스크가 크다.
- **프론트엔드 언어는 부차적**: Tauri는 static web host처럼 동작하며 JS/TS 프론트는 UI 셸 역할에 적합하다. 다만 이번 아키텍처 리서치의 성패는 프론트 기술보다 백엔드 render service 구조에 달려 있다.
- **언어 선택 우선순위 판단**:
  - `Rust`: 현 코드베이스 연속성, 안정성, 패키징, 시스템 제어에 유리
  - `HLSL`: Windows GPU-first custom path에 가장 직접적
  - `OpenCL C`: darktable/OpenCL 재활용 검증에 유리
  - `CUDA C++`: 최고 성능 후보가 될 수 있으나 하드웨어 범용성에 불리
  - `Vulkan/SPIR-V 계열`: 교차 벤더성은 좋지만 Windows 단일 제품 기준에선 운영 복잡도 증가 가능

_Popular Languages:_ Rust, HLSL, OpenCL C, C++, JavaScript/TypeScript  
_Emerging Languages:_ Slang/SPIR-V 중심 shader toolchain은 성장 중이지만 Boothy의 즉시 의사결정 기준에서는 후보 보조축에 가깝다.  
_Language Evolution:_ 2025 Stack Overflow 설문은 Python 성장과 Rust 생태계 도구 강세를 보여주지만, Boothy의 핵심은 범용 인기보다 `시스템 제어 + GPU compute 적합성`이다.  
_Performance Characteristics:_ Rust는 orchestration에 적합하고, 실질적인 픽셀 처리 성능은 HLSL/OpenCL/CUDA/Vulkan compute 쪽 설계가 좌우한다.  
_Sources:_ https://tauri.app/concept/architecture/ ; https://v2.tauri.app/release/ ; https://learn.microsoft.com/en-us/windows/win32/direct3d12/pipelines-and-shaders-with-directx-12 ; https://registry.khronos.org/OpenCL/sdk/3.0/docs/man/html/intro.html ; https://github.khronos.org/Vulkan-Site/spec/latest/chapters/introduction.html ; https://docs.nvidia.com/cuda/cuda-programming-guide/index.html ; https://survey.stackoverflow.co/2025

### Development Frameworks and Libraries

프레임워크 관점에서는 `Tauri 앱 셸 + GPU 실행 엔진 + baseline/fallback 엔진`의 3계층으로 보는 것이 맞다.

- **앱 셸**: Tauri 2 계열은 Rust 백엔드와 WebView UI를 조합하는 구조로, 현재 Windows booth 앱 운영 모델과 잘 맞는다. 큰 구조 변경 없이 새 render service를 Rust 내부 서비스 또는 sidecar로 연결하기 쉽다.
- **baseline/fallback 엔진**: darktable는 OpenCL 지원, CLI, CPU fallback, profiling 훅을 이미 제공한다. 따라서 주력 엔진 여부와 별개로 `reference engine`, `preset parity oracle`, `fallback exporter` 역할은 여전히 유효하다.
- **custom GPU engine 후보**:
  - `Direct3D 12 compute`: Windows에서 가장 직접적인 custom GPU-first 경로. HLSL, descriptor heaps, PSO, resident resources, explicit queue 설계에 유리하다.
  - `OpenCL 3.0`: 크로스벤더 GPGPU 모델은 좋지만, darktable 재사용 이상의 차별화 구조가 없으면 아키텍처 전환 이익이 제한될 수 있다.
  - `Vulkan compute`: 명시적 제어와 교차 플랫폼성은 강점이나, 현재 제품 목표가 Windows booth 전용이라면 초기 복잡도가 높다.
  - `CUDA`: NVIDIA booth만 확정이라면 검토 가치가 있지만, AMD 대응을 버리게 되는 구조적 비용이 크다.
- **프론트 프레임워크**: Tauri 문서 기준 SPA/MPA/SSG가 정합적이며, Vite가 일반 권장값이다. 하지만 이번 리서치에서 UI 프레임워크는 병목 해법이 아니다.

_Major Frameworks:_ Tauri, darktable CLI/OpenCL, Direct3D 12, OpenCL 3.0, Vulkan, CUDA  
_Micro-frameworks:_ preset adapter, resident queue manager, local metadata store 같은 사내 경량 계층이 실제 제품 경쟁력을 좌우할 가능성이 높다.  
_Evolution Trends:_ Adobe는 GPU 적용 범위를 `Display`, `Image Processing`, `Export`, `Preview Generation`까지 넓히고 있다. 이는 Boothy도 단일 preview 해킹보다 서비스형 GPU 파이프라인으로 가야 한다는 간접 근거다.  
_Ecosystem Maturity:_ darktable와 D3D12는 성숙, Vulkan은 강력하지만 복잡, CUDA는 매우 성숙하나 벤더 제한, Tauri는 현 제품 셸로 충분히 성숙하다.  
_Sources:_ https://tauri.app/concept/architecture/ ; https://v2.tauri.app/start/frontend/ ; https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/ ; https://docs.darktable.org/usermanual/4.8/en/special-topics/program-invocation/darktable/ ; https://registry.khronos.org/OpenCL/ ; https://learn.microsoft.com/en-us/windows/win32/direct3d12/core-feature-levels ; https://docs.vulkan.org/tutorial/latest/00_Introduction.html ; https://docs.nvidia.com/cuda/cuda-programming-guide/index.html ; https://helpx.adobe.com/si/lightroom-classic/kb/lightroom-gpu-faq.html ; https://helpx.adobe.com/si/lightroom-classic/kb/gpu-preview-generation.html

### Database and Storage Technologies

이번 주제에서 데이터 계층은 대규모 서버 DB보다 `온디바이스 메타데이터, 캐시, preset truth, artifact index`가 핵심이다.

- **핫패스 저장소 우선순위**:
  - RAW/JPEG/TIFF 결과물: 파일 시스템 기반
  - preset 및 version truth: sidecar/XMP 또는 내부 canonical recipe 저장소
  - 작업 큐/세션 메타데이터/캐시 인덱스: SQLite가 가장 현실적
- **SQLite가 유력한 이유**:
  - Windows 앱 내장형 로컬 DB로 가볍고 신뢰성이 높다.
  - 단일 파일 구조라 booth 운영/백업/이동이 쉽다.
  - WAL 모드는 reader/writer 동시성에 유리해 render queue metadata에 적합하다.
- **주의점**:
  - WAL은 reader gap이 없으면 checkpoint starvation으로 파일이 커질 수 있다.
  - 네트워크 파일시스템 또는 다중 프로세스 과다 공유를 전제로 설계하면 운영 리스크가 생긴다.
  - canonical preset recipe를 SQLite에 저장하더라도 최종 아티팩트는 여전히 파일 시스템과 sidecar 호환성을 유지하는 편이 안전하다.
- **판단**:
  - 서버형 RDBMS, NoSQL, data warehouse는 현재 display/export hot path와 거리가 멀다.
  - 이번 아키텍처에서 저장소 핵심은 `SQLite + filesystem + sidecar adapter` 조합이다.

_Relational Databases:_ SQLite가 가장 적합. PostgreSQL 등 서버 DB는 현 booth hot path에 과하다.  
_NoSQL Databases:_ 현재 요구 범위에서는 우선순위 낮음.  
_In-Memory Databases:_ 별도 Redis보다 프로세스 메모리 캐시와 GPU resource cache가 더 적합.  
_Data Warehousing:_ 제품 운영 분석용으로는 가능하지만 render SLA 구조 결정에는 비핵심.  
_Sources:_ https://learn.microsoft.com/en-us/windows/apps/develop/data-access/sqlite-data-access ; https://sqlite.org/serverless.html ; https://sqlite.org/wal.html ; https://www.sqlite.org/appfileformat.html

### Development Tools and Platforms

새 아키텍처는 구현 자체보다 `계측과 검증 도구 체계`가 중요하다.

- **기본 개발 플랫폼**:
  - Windows desktop + Tauri + Rust/Cargo 조합이 현행 운영과 가장 자연스럽다.
  - Cargo는 repeatable build와 dependency 관리에 강점이 있어 native Rust 서비스 계층 유지에 적합하다.
- **GPU 계측 도구**:
  - `PIX on Windows`: Direct3D 12 앱의 GPU capture, timing capture, frame analysis에 적합하다.
  - `NVIDIA Nsight Graphics`: D3D11/12, Vulkan, OpenGL 계열 디버깅/프로파일링 가능.
  - AMD 경로는 Radeon GPU Profiler 등 별도 벤더 도구 전략을 병행해야 한다.
  - darktable/OpenCL 경로는 `-d opencl -d perf` 같은 자체 프로파일링 훅을 활용 가능하다.
- **테스트/검증 도구 우선순위**:
  - GPU trace
  - capture-to-visible latency telemetry
  - display/export parity 비교 도구
  - booth-safe fallback 검증
- **판단**:
  - 이번 아키텍처의 개발도구 핵심은 IDE보다 `GPU profiling + pipeline telemetry + reproducible build`다.
  - 특히 Direct3D 12 custom path를 택한다면 PIX 도입은 선택이 아니라 필수에 가깝다.

_IDE and Editors:_ Rust/Cargo, Tauri, WebView 기반 개발환경이면 일반 IDE 선택은 자유도가 높다.  
_Version Control:_ Git 기반 운영이 전제되며, telemetry schema와 preset recipe versioning을 함께 관리해야 한다.  
_Build Systems:_ Cargo가 네이티브 백엔드 기본축, 프론트는 Vite 계열이 무난하다.  
_Testing Frameworks:_ 일반 단위 테스트보다 GPU capture 분석, parity diff, booth smoke가 중요하다.  
_Sources:_ https://doc.rust-lang.org/cargo/guide/why-cargo-exists.html ; https://v2.tauri.app/start/frontend/ ; https://learn.microsoft.com/en-us/windows/win32/direct3dtools/pix/articles/general/pix-overview ; https://developer.nvidia.com/rdp/nsight-graphics-registered-developer-portal ; https://developer.nvidia.com/nsight-graphics/get-started ; https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/

### Cloud Infrastructure and Deployment

Boothy의 주 SLA는 booth 현장 장비에서 닫혀야 하므로, 배포 관점의 중심은 cloud가 아니라 `on-device Windows deployment`다.

- **1차 배포 플랫폼**:
  - Windows + Tauri 패키징 + 로컬 GPU 실행환경
  - booth 현장의 드라이버, WebView2, GPU capability, warm-up orchestration이 핵심 운영 변수
- **Cloud의 역할은 제한적**:
  - display 2.5초 SLA와 세션 종료 직전 batch export는 네트워크 의존 설계와 맞지 않는다.
  - cloud GPU는 원격 백오피스 재처리, 장기 실험, 대규모 품질 비교용으로는 의미가 있지만 hot path 대체로는 부적합하다.
- **선택지 자체는 존재**:
  - AWS는 Deadline Cloud에서 GPU 가속 EC2 fleets를 지원한다.
  - Azure는 graphics-intensive workload용 NV 계열 GPU VM을 제공한다.
  - Google Cloud도 Windows GPU workstation 구성이 가능하다.
- **판단**:
  - Boothy 핵심 구조는 edge/local first가 맞다.
  - cloud는 `실험용 benchmark farm`, `비핫패스 재처리`, `장기 분석` 용도로만 2차 검토하는 것이 적절하다.

_Major Cloud Providers:_ AWS, Azure, Google Cloud는 모두 GPU 옵션을 제공하지만 주 아키텍처 중심축은 아님.  
_Container Technologies:_ 현재 hot path는 로컬 디바이스 제어와 GPU 드라이버 의존성이 커서 우선순위 낮음.  
_Serverless Platforms:_ 네트워크/콜드스타트 특성상 이번 SLA와 부정합.  
_CDN and Edge Computing:_ booth 자체가 edge이므로 외부 CDN/edge는 본 문제의 핵심이 아님.  
_Sources:_ https://v2.tauri.app/start/prerequisites/ ; https://v2.tauri.app/concept/architecture/ ; https://aws.amazon.com/about-aws/whats-new/2024/11/aws-deadline-cloud-gpu-accelerated-ec2-instance-types/ ; https://learn.microsoft.com/en-us/azure/virtual-machines/sizes/gpu-accelerated/nv-family ; https://docs.cloud.google.com/compute/docs/virtual-workstation/windows-gpu

### Technology Adoption Trends

현재 시장 흐름은 Boothy의 방향을 대체로 지지하지만, 그대로 베끼면 안 된다.

- **GPU 적용 범위 확대 추세**: Adobe Lightroom Classic은 이미 GPU를 display, image processing, export에 더해 preview generation까지 확장하고 있다. 이는 `GPU는 옵션`이 아니라 `파이프라인 수준 자원`이라는 방향을 뒷받침한다.
- **Windows booth 제품 기준 현실성**:
  - cross-platform 범용성보다 Windows 최적화가 우선이다.
  - 따라서 `D3D12 custom path`는 설계 우선순위가 높다.
  - 동시에 baseline/fallback/reference로 `darktable/OpenCL`을 유지하는 이중구조가 현실적이다.
- **도구 생태계 추세**:
  - Tauri 2 계열은 계속 성숙 중이며 현행 앱 셸 유지에 무리가 없다.
  - Rust/Cargo는 재현 가능한 빌드와 네이티브 서비스 계층 운영에 유리하다.
  - GPU profiling 생태계는 Microsoft PIX, NVIDIA Nsight처럼 벤더/플랫폼 특화 툴을 중심으로 굳어져 있다.
- **최종 판단**:
  - 단기적으로는 `Rust/Tauri 유지 + D3D12-first 실험 + darktable baseline 유지`가 가장 현실적이다.
  - 중장기적으로는 `canonical preset recipe`를 제품 내부 형식으로 승격해야 엔진 교체 비용을 낮출 수 있다.

_Migration Patterns:_ preview-only 최적화에서 resident GPU service 중심 구조로 이동하는 것이 타당하다.  
_Emerging Technologies:_ GPU preview generation, explicit compute pipeline, richer shader toolchains가 강화되고 있다.  
_Legacy Technology:_ darktable 단독 주력 런타임 가정은 약해지고 있다.  
_Community Trends:_ 범용 언어 인기보다, 검증 가능한 GPU 툴체인과 운영 안정성이 실제 선택을 좌우한다.  
_Sources:_ https://helpx.adobe.com/si/lightroom-classic/kb/lightroom-gpu-faq.html ; https://helpx.adobe.com/si/lightroom-classic/kb/gpu-preview-generation.html ; https://helpx.adobe.com/si/lightroom-classic/help/lightroom-smart-previews.html ; https://v2.tauri.app/release/ ; https://doc.rust-lang.org/cargo/guide/why-cargo-exists.html ; https://survey.stackoverflow.co/2025

## Integration Patterns Analysis

### API Design Patterns

Boothy의 새 렌더링 구조에서 API 설계는 외부 공개 API보다 `앱 셸과 렌더 서비스 사이의 내부 계약`이 핵심이다.

- **UI ↔ 앱 코어 1차 패턴은 Tauri Commands + Events가 적합**:
  - Tauri 공식 문서 기준 `Commands`는 프론트가 Rust 함수를 호출하고 JSON 직렬화 가능한 요청/응답을 주고받는 방식이다.
  - `Events`는 fire-and-forget, one-way 메시지라 render progress, readiness, lane state 변화 전파에 맞다.
  - 따라서 `명령형 제어`와 `상태 이벤트`를 분리하는 것이 자연스럽다.
- **앱 코어 ↔ GPU render service 2차 패턴은 JSON보다 typed RPC가 유리**:
  - 렌더 요청, preset recipe version, quality mode, target artifact 같은 제어면은 Protobuf/gRPC 또는 자체 binary contract가 더 안정적이다.
  - 단, gRPC는 HTTP/2 기반이라 로컬 booth 제품에서는 네트워크 스택과 packaging 제약을 같이 끌고 온다.
  - 따라서 booth hot path에서는 `named pipe + typed messages`가 더 실용적일 가능성이 높다.
- **REST/GraphQL은 우선순위 낮음**:
  - 본 문제는 browser-client 대규모 API가 아니라 동일 장비 내 렌더 제어다.
  - REST/GraphQL은 디버깅, 원격 진단, 백오피스 관리 API에는 의미가 있으나 render hot path에는 과하다.
- **Webhook 패턴도 비핵심**:
  - booth 로컬 구조에서는 외부 HTTP callback보다 내부 이벤트 버스가 더 단순하고 신뢰성이 높다.

_RESTful APIs:_ 원격 진단/운영 API에는 가능하지만 display/export hot path에는 부적합하다.  
_GraphQL APIs:_ UI 조합성은 좋지만 렌더 서비스 제어에는 이점이 작다.  
_RPC and gRPC:_ typed contract와 code generation 장점은 크지만, 로컬 booth 환경에서는 named pipe 계열과 비교 검토가 필요하다.  
_Webhook Patterns:_ 외부 시스템 연동에는 가능하나 핵심 렌더 구조에는 비우선이다.  
_Sources:_ https://v2.tauri.app/concept/inter-process-communication/ ; https://grpc.io/docs/what-is-grpc/introduction/ ; https://protobuf.dev/ ; https://datatracker.ietf.org/doc/html/rfc9113

### Communication Protocols

실제 통신 프로토콜은 `제어면`과 `데이터면`을 분리해서 봐야 한다.

- **제어면(Control Plane)**:
  - UI에서 코어로는 Tauri IPC가 가장 자연스럽다.
  - 코어에서 sidecar/render daemon으로는 Windows named pipe가 강력한 후보다.
  - Microsoft 문서 기준 named pipe는 overlapped I/O로 다중 연결을 처리할 수 있고, 패키지 앱에서도 명명 규칙과 제약이 명확하다.
- **데이터면(Data Plane)**:
  - 큰 이미지 버퍼까지 JSON/RPC로 옮기면 손해가 크다.
  - Microsoft는 shared memory/file mapping이 많은 양의 데이터를 효율적으로 공유하는 데 적합하다고 설명한다.
  - 따라서 `제어는 message`, `대용량 raster/intermediate buffer는 shared memory 또는 GPU resident resource`로 나누는 설계가 타당하다.
- **GPU 내부 동기화**:
  - Direct3D 12는 compute queue와 3D queue 사이 작업 의존성을 fence로 명시적으로 관리한다.
  - 이는 display lane과 export lane, 또는 preprocess와 present 단계 분리에 직접 연결된다.
- **loopback HTTP/gRPC는 제약 존재**:
  - Microsoft 문서상 packaged app의 loopback IPC는 기본 차단되며, capability/manifest 또는 예외 설정이 필요하다.
  - 따라서 Tauri 패키징 모델에서 localhost 기반 구조를 택하면 운영 복잡도가 늘 수 있다.

_HTTP/HTTPS Protocols:_ 원격 API와 진단에는 적합하지만 로컬 render hot path 기본 프로토콜로는 비효율 가능성이 크다.  
_WebSocket Protocols:_ 양방향 스트리밍은 가능하지만 same-machine local render service 기본값으로는 named pipe보다 이점이 분명하지 않다.  
_Message Queue Protocols:_ 외부 브로커보다 in-process async events와 local queue가 맞다.  
_gRPC and Protocol Buffers:_ 강한 타입 계약에는 유리하지만 loopback/HTTP2 제약을 같이 검토해야 한다.  
_Sources:_ https://v2.tauri.app/concept/inter-process-communication/ ; https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-server-using-overlapped-i-o ; https://learn.microsoft.com/en-us/windows/uwp/communication/interprocess-communication ; https://learn.microsoft.com/en-us/windows/win32/direct3d12/executing-and-synchronizing-command-lists ; https://grpc.io/docs/what-is-grpc/introduction/ ; https://datatracker.ietf.org/doc/html/rfc9113

### Data Formats and Standards

Boothy의 통합 포맷은 `사람이 읽는 포맷`, `엔진 교환 포맷`, `성능 지향 포맷`으로 분리하는 것이 맞다.

- **JSON은 UI/control 용으로 유지**:
  - Tauri Commands는 JSON 직렬화 제약이 있으므로, UI 명령/상태 조회/진단에는 JSON이 적합하다.
  - 하지만 큰 render recipe나 픽셀 버퍼까지 JSON에 넣는 것은 피해야 한다.
- **Protobuf는 typed service contract 후보**:
  - 구조화된 메시지를 언어중립으로 정의하고 codegen할 수 있어 sidecar/service 분리에 유리하다.
  - 특히 preset recipe version, artifact request, telemetry envelope 같은 계약 정의에 적합하다.
- **XMP는 현재 interoperability 기준선**:
  - Adobe XMP는 메타데이터 데이터 모델과 직렬화 포맷을 표준화하며, 파일 내 저장과 sidecar 저장 양쪽을 지원한다.
  - darktable는 XMP sidecar와 내부 DB를 함께 사용하며, 재임포트 시 XMP로부터 DB를 갱신할 수 있다.
  - 따라서 새 구조에서도 `XMP를 버리기보다 adapter 대상`으로 두는 쪽이 현실적이다.
- **권장 포맷 전략**:
  - UI/control: JSON
  - service contract: Protobuf
  - canonical preset truth: 내부 recipe schema
  - legacy interop: XMP adapter
  - bulk pixel transfer: shared memory / GPU resource / 파일 기반 intermediate

_JSON and XML:_ JSON은 UI 제어에 적합, XML 계열 XMP는 외부 엔진 호환성에 중요하다.  
_Protobuf and MessagePack:_ Protobuf가 schema governance와 codegen 측면에서 더 강하다.  
_CSV and Flat Files:_ 대량 결과 목록, telemetry export에는 가능하지만 hot path에는 비핵심이다.  
_Custom Data Formats:_ 내부 canonical preset recipe는 사실상 필요하다.  
_Sources:_ https://v2.tauri.app/concept/inter-process-communication/ ; https://protobuf.dev/ ; https://protobuf.dev/best-practices/ ; https://developer.adobe.com/xmp/docs/xmp-specifications/ ; https://docs.darktable.org/usermanual/4.8/en/overview/sidecar-files/sidecar/

### System Interoperability Approaches

Boothy는 enterprise integration이 아니라 `단일 장비 내부의 신뢰 가능한 다중 구성요소 조합` 문제다.

- **Point-to-point integration이 기본값**:
  - UI shell ↔ app core
  - app core ↔ render service
  - render service ↔ baseline/fallback engine
  - 이 3개 경계만 명확히 잡는 편이 가장 단순하다.
- **API gateway/service mesh/ESB는 과도**:
  - booth 앱은 분산 시스템이 아니며, 서비스 디스커버리와 중앙 게이트웨이 레이어는 운영 복잡도만 키울 가능성이 높다.
- **host object / native bridge는 예외적 도구**:
  - WebView2는 native host object를 JS에 노출하는 방식을 제공한다.
  - 다만 보안 경계와 테스트 복잡도를 키울 수 있으므로, 일반 제어는 Tauri IPC로 두고 host object는 하드웨어 특수 기능이나 저수준 브리지에 제한하는 편이 낫다.
- **엔진 interoperability 원칙**:
  - canonical recipe를 중심으로 engine adapter를 붙인다.
  - darktable, custom GPU engine, fallback exporter가 같은 capture binding과 version truth를 공유해야 한다.

_Point-to-Point Integration:_ 현재 제품 목표에 가장 적합하다.  
_API Gateway Patterns:_ 원격 관리 plane에는 가능하지만 core runtime에는 불필요하다.  
_Service Mesh:_ 현 단계에서는 도입 가치가 거의 없다.  
_Enterprise Service Bus:_ 제품 규모와 목적에 비해 과하다.  
_Sources:_ https://v2.tauri.app/concept/inter-process-communication/ ; https://learn.microsoft.com/en-us/microsoft-edge/webview2/how-to/hostobject ; https://docs.darktable.org/usermanual/4.8/en/overview/sidecar-files/sidecar/

### Microservices Integration Patterns

새 구조를 microservice처럼 생각하더라도, cloud-native 방식이 아니라 `local service partitioning` 정도로 해석하는 것이 맞다.

- **권장 패턴은 local render service 분리**:
  - UI/app shell과 render engine lifecycle을 분리하면 warm GPU context 유지와 장애 격리가 쉬워진다.
  - resident daemon 모델은 first capture cold-start를 booth startup 시점으로 이동시키기 쉽다.
- **Service discovery는 불필요**:
  - 단일 장비, 고정 프로세스 구성에서는 정적 엔드포인트가 낫다.
- **Circuit breaker는 필요**:
  - GPU failure, driver reset, queue stall 시 baseline/fallback engine으로 degrade해야 한다.
  - 이는 기술적으로는 microservice resilience pattern이지만, Boothy에선 `booth-safe fallback`으로 해석하는 것이 더 정확하다.
- **Saga pattern은 비핵심**:
  - 분산 트랜잭션보다 capture-bound version truth와 idempotent job 처리 설계가 더 중요하다.

_API Gateway Pattern:_ local service 분리에는 필요 없고, 원격 관리 plane에만 제한적으로 의미 있다.  
_Service Discovery:_ 정적 배치 제품에선 불필요하다.  
_Circuit Breaker Pattern:_ 실제로 중요하다. GPU lane 실패 시 fallback 전환 기준이 필요하다.  
_Saga Pattern:_ 장비 내부 렌더링 흐름에서는 과하다.  
_Sources:_ https://learn.microsoft.com/en-us/windows/win32/direct3d12/executing-and-synchronizing-command-lists ; https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-server-using-overlapped-i-o ; https://docs.darktable.org/usermanual/4.8/en/overview/sidecar-files/sidecar/

### Event-Driven Integration

이번 아키텍처의 이벤트 설계는 외부 메시지 브로커가 아니라 `내부 render lifecycle event model`로 가야 한다.

- **권장 이벤트 종류**:
  - capture accepted
  - recipe resolved
  - GPU warm ready
  - fast display ready
  - truthful display ready
  - export queued / export completed
  - fallback entered / recovered
- **Tauri Events는 UI 상태 전파에 적합**:
  - 공식 문서상 fire-and-forget, one-way 성격이라 진행 상태 브로드캐스트에 잘 맞는다.
- **Event sourcing는 제한적 도입만 권장**:
  - 전체 시스템을 event-sourced로 만들 필요는 없다.
  - 다만 capture-bound version truth, preset revision history, failure audit trail에는 append-only event log가 유용하다.
- **외부 message broker는 비권장**:
  - RabbitMQ, Kafka 같은 중앙 브로커는 booth 장비 운영에 과하다.
  - local queue + durable log면 충분하다.

_Publish-Subscribe Patterns:_ 내부 상태 전파용으로 적합하다.  
_Event Sourcing:_ 전체 구조보다는 audit/versioning 용도로만 제한하는 편이 좋다.  
_Message Broker Patterns:_ 외부 브로커는 과도하다.  
_CQRS Patterns:_ 읽기용 telemetry 조회와 쓰기용 render command 분리는 부분적으로 가치가 있다.  
_Sources:_ https://v2.tauri.app/concept/inter-process-communication/ ; https://learn.microsoft.com/en-us/windows/win32/direct3d12/executing-and-synchronizing-command-lists

### Integration Security Patterns

보안은 외부 인증보다 `WebView 노출 최소화`, `로컬 IPC 권한 제한`, `렌더 엔진 격리`가 우선이다.

- **Tauri capability 기반 최소 권한 노출이 중요**:
  - 공식 문서 기준 capability는 어떤 window/webview에 어떤 권한을 줄지 제한한다.
  - 고권한 기능은 별도 capability로 좁혀야 한다.
- **named pipe는 ACL과 세션 범위 제한이 필요**:
  - Microsoft 문서 기준 named pipe는 security descriptor로 접근 권한을 제어할 수 있다.
  - 기본 DACL 동작에 기대지 말고, 익명/원격 접근을 막는 명시적 제한이 필요하다.
  - 다른 터미널 세션 접근 방지를 위해 logon SID 사용도 권장된다.
- **localhost 기반 IPC는 packaging/보안 제약 동반**:
  - packaged app에서 loopback은 기본 허용이 아니므로, 운영 리스크를 감수할 이유가 있는지 먼저 따져야 한다.
- **host object 사용은 제한적으로**:
  - WebView2는 native API를 JS로 노출할 수 있지만, 이 방식은 노출면을 넓힐 수 있다.
  - 일반 명령은 Tauri command/event로 유지하고, host object는 꼭 필요한 경우만 사용해야 한다.

_OAuth 2.0 and JWT:_ booth 로컬 프로세스 경계에는 기본 해법이 아니다.  
_API Key Management:_ 내부 same-machine IPC에는 부적합하다.  
_Mutual TLS:_ 원격 서비스 연동에는 가능하나 핵심 hot path엔 과하다.  
_Data Encryption:_ 로컬 저장 artifact와 세션 메타데이터는 필요 수준에 맞게 적용하되, 우선순위는 권한 경계와 노출 최소화다.  
_Sources:_ https://v2.tauri.app/security/capabilities/ ; https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights ; https://learn.microsoft.com/en-us/windows/uwp/communication/interprocess-communication ; https://learn.microsoft.com/en-us/microsoft-edge/webview2/how-to/hostobject

## Architectural Patterns and Design

### System Architecture Patterns

Boothy의 새 구조는 일반적인 `웹 앱 아키텍처 스타일`로 고르면 안 되고, `단일 장비 내부의 고성능 워크로드 분리`로 봐야 한다.

- **배제해야 할 구조**:
  - 전통적 N-tier 단일 프로세스 확장형 구조는 계층 분리는 쉬워도, 이번 문제의 핵심인 `GPU warm state 유지`, `렌더 장애 격리`, `display/export 독립 스케줄링`에 불리하다.
  - Azure Architecture Center도 N-tier는 기존 layered app 이전에는 적합하지만, 변화 민첩성에는 제약이 있다고 설명한다.
- **과한 구조**:
  - full microservices는 분산 운영, 데이터 일관성, 관측성, 서비스 디스커버리 복잡도를 가져오므로 booth 제품에 과하다.
  - Microsoft는 microservices가 높은 독립성과 확장성을 주지만, 그만큼 운영 복잡성이 크다고 정리한다.
- **가장 맞는 구조**:
  - `modular monolith + local worker/service` 패턴이 가장 적합하다.
  - 즉 Tauri 앱 셸은 얇게 유지하고, heavy render는 resident local GPU service로 분리한다.
  - 이 패턴은 Azure의 Web-Queue-Worker 스타일과 유사하게 `사용자 상호작용`과 `resource-intensive background processing`를 분리하되, cloud queue 대신 local queue를 쓰는 제품형 변형으로 해석할 수 있다.
- **제품 기준 추천 아키텍처**:
  - `Presentation`: Tauri/WebView UI
  - `Application Core`: capture binding, session state, preset truth, queue coordination
  - `GPU Render Service`: always-warm display/export engine
  - `Baseline/Fallback Engine`: darktable/OpenCL/CLI 계열

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/ ; https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/microservices ; https://v2.tauri.app/concept/architecture/

### Design Principles and Best Practices

이번 구조의 설계 원칙은 범용 SOLID 설명보다 `truth, isolation, fallback, measurability` 네 축으로 정리하는 것이 맞다.

- **Single source of truth for presets**:
  - display와 export가 각각 다른 해석을 하면 parity를 잃는다.
  - 따라서 `canonical preset recipe`를 한 곳에 두고, 각 엔진은 adapter로 붙어야 한다.
- **Isolate what is expensive**:
  - GPU context, shader cache, large intermediate resources는 앱 셸과 분리된 수명주기로 관리해야 한다.
  - 그렇지 않으면 UI 흐름과 렌더 수명이 얽혀 cold-start와 장애 전파가 커진다.
- **Separate fast path and truthful path, but share intent**:
  - Adobe의 Smart Preview/원본 출력 분리처럼, 표시와 최종 산출물은 같은 intent를 공유하되 비용 구조는 다르게 둘 수 있다.
  - Boothy에서는 `display lane`과 `export lane`을 나누되 같은 recipe/version truth를 공유하는 것이 핵심이다.
- **Design contracts before engines**:
  - 엔진 후보를 바꾸더라도 capture id, preset version, artifact type, parity metadata 계약은 유지돼야 한다.
  - 그래야 darktable 기준선, custom GPU engine, fallback 경로를 비교 가능하게 유지할 수 있다.

_Source:_ https://helpx.adobe.com/si/lightroom-classic/help/lightroom-smart-previews.html ; https://helpx.adobe.com/si/lightroom-classic/kb/lightroom-gpu-faq.html ; https://protobuf.dev/best-practices/ ; https://docs.darktable.org/usermanual/4.8/en/overview/sidecar-files/sidecar/

### Scalability and Performance Patterns

Boothy에서 scalability는 서버 수평 확장이 아니라 `장비 한 대에서 throughput과 latency를 둘 다 관리하는 구조`를 뜻한다.

- **Resident warm service가 1순위**:
  - first capture 비용을 요청 시점이 아니라 앱/booth startup 시점으로 이동해야 한다.
  - warm GPU context, shader pipeline state, resource pools, decode/cache priming이 핵심이다.
- **Queue 분리**:
  - display lane과 export lane을 같은 작업 큐에 넣으면 긴 배치 작업이 즉시 표시 SLA를 망친다.
  - 최소한 `interactive queue`와 `batch queue`는 분리해야 한다.
- **Cache-aside + warm priming 조합**:
  - Azure cache-aside 문서는 on-demand load와 priming의 trade-off를 설명한다.
  - Boothy에선 preset recipe, lens profile, LUT, shader state, 최근 capture metadata는 warm 시점 pre-prime하고, 나머지는 cache-aside로 가져가는 하이브리드가 적절하다.
- **GPU memory reuse 설계**:
  - Direct3D 12는 placed/reserved resource와 heap 재사용, tiled mapping 같은 명시적 메모리 전략을 제공한다.
  - 이는 풀사이즈 display와 batch export가 겹칠 때 VRAM을 효율적으로 회전시키는 기반이 된다.
- **판단**:
  - 성능 병목이 이미 render body로 이동한 만큼, 추가 미세 튜닝보다 `queue architecture + warm state + GPU memory strategy` 전환이 수익이 크다.

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/cache-aside ; https://learn.microsoft.com/en-us/windows/win32/direct3d12/memory-management-strategies ; https://learn.microsoft.com/en-us/windows/win32/api/d3d12/nf-d3d12-id3d12device-createplacedresource ; https://learn.microsoft.com/en-us/windows/win32/api/d3d12/ne-d3d12-d3d12_tiled_resources_tier ; https://learn.microsoft.com/en-us/windows/win32/direct3d12/executing-and-synchronizing-command-lists

### Integration and Communication Patterns

이 구조의 통신 패턴은 `thin UI IPC + typed local control plane + explicit GPU synchronization`의 조합이 가장 맞다.

- **UI와 코어는 Tauri IPC**:
  - command는 요청/응답, event는 상태 전파용으로 분리한다.
- **코어와 렌더 서비스는 local control plane**:
  - named pipe 또는 동급 로컬 IPC가 기본값이다.
  - localhost/gRPC는 타입 장점이 있지만 packaging 제약과 운영 복잡도를 가져온다.
- **데이터 이동은 최소화**:
  - control plane은 작은 메시지로 유지하고, 대형 raster/intermediate는 shared memory 또는 GPU resident 자원으로 다뤄야 한다.
- **GPU 작업 의존성은 explicit sync**:
  - D3D12 fence/command queue 설계는 architecture concern이지 단순 구현 detail이 아니다.
  - display present, export encode, intermediate preprocess를 구조적으로 분리할 수 있는 기반이 된다.

_Source:_ https://v2.tauri.app/concept/inter-process-communication/ ; https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-server-using-overlapped-i-o ; https://learn.microsoft.com/en-us/windows/win32/direct3d12/executing-and-synchronizing-command-lists ; https://learn.microsoft.com/en-us/windows/uwp/communication/interprocess-communication

### Security Architecture Patterns

보안 구조는 `same-machine trusted runtime` 전제에서 과도한 네트워크 보안보다 `권한 경계`와 `공격면 축소`를 우선해야 한다.

- **Least privilege UI**:
  - Tauri capability로 window/webview별 권한을 제한해야 한다.
  - 렌더 제어, 파일 접근, shell/OS 기능을 한 capability에 몰아넣으면 위험하다.
- **Process boundary as safety boundary**:
  - GPU render service를 분리하면 crash containment뿐 아니라 권한 격리에도 유리하다.
  - 앱 셸이 고장나도 render service를 재시작하거나 반대로 격리하기 쉽다.
- **Local IPC ACL**:
  - named pipe는 ACL과 logon SID 범위 제한으로 세션 외 접근을 줄여야 한다.
- **Avoid unnecessary localhost surface**:
  - packaged 환경의 loopback 예외 설정은 운영 복잡도와 설정 drift를 부를 수 있다.
  - 특별한 이유가 없으면 network listener보다 로컬 IPC가 낫다.

_Source:_ https://v2.tauri.app/security/capabilities/ ; https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights ; https://learn.microsoft.com/en-us/windows/uwp/communication/interprocess-communication ; https://learn.microsoft.com/en-us/microsoft-edge/webview2/how-to/hostobject

### Data Architecture Patterns

데이터 아키텍처의 핵심은 `원본 파일`, `canonical recipe`, `운영 메타데이터`, `캐시`를 분리하는 것이다.

- **원본/산출물 계층**:
  - RAW와 최종 raster는 파일 시스템에 둔다.
  - 이는 batch export, 외부 전달, baseline engine 비교에 가장 단순하다.
- **canonical recipe 계층**:
  - 장기적으로는 내부 schema가 필요하다.
  - XMP는 interoperability adapter와 회귀 기준선으로 남긴다.
- **운영 메타데이터 계층**:
  - session, capture binding, version truth, queue state, artifact index는 SQLite가 적합하다.
  - 단일 파일형 저장소라 booth 운영과 장애 복구가 쉽다.
- **캐시 계층**:
  - 최근 recipe 해석 결과, 렌즈 보정 파라미터, shader/pipeline metadata, decode metadata는 memory 또는 local persistent cache로 둘 수 있다.
  - 다만 cache-aside 문서가 지적하듯 일관성 보장이 약하므로 truth 계층과 혼동하면 안 된다.

_Source:_ https://learn.microsoft.com/en-us/windows/apps/develop/data-access/sqlite-data-access ; https://sqlite.org/wal.html ; https://www.sqlite.org/appfileformat.html ; https://developer.adobe.com/xmp/docs/xmp-specifications/ ; https://learn.microsoft.com/en-us/azure/architecture/patterns/cache-aside

### Deployment and Operations Architecture

배포/운영 아키텍처는 cloud rollout이 아니라 `현장 장비에서 cold-start 없이 반복적으로 안정 동작`하는 구조로 정의해야 한다.

- **Booth startup phase를 아키텍처에 포함해야 한다**:
  - 앱 시작 후 즉시 GPU context 생성
  - shader/pipeline warm-up
  - baseline engine health check
  - preset/cache priming
  - 이 단계가 architecture의 일부여야 first capture SLA를 닫을 수 있다.
- **Observability is mandatory**:
  - ETW는 Windows의 고성능 저오버헤드 tracing 메커니즘이다.
  - WPR/WPA, PIX, vendor profiler를 조합해 startup, capture, display, export 경로를 측정해야 한다.
- **Failure handling architecture**:
  - DXGI device removed 같은 GPU 장애는 실제 운영 이벤트로 간주해야 한다.
  - render service restart, fallback engine 전환, UI 상태 고지까지 포함한 recovery flow가 필요하다.
- **운영 판단**:
- 이 구조는 “빠른 렌더 엔진”만으로 성립하지 않는다.
- `warm-up orchestration + telemetry + fallback recovery`가 제품 아키텍처의 일부여야 한다.

_Source:_ https://learn.microsoft.com/en-us/windows/win32/etw/about-event-tracing ; https://learn.microsoft.com/en-us/windows-hardware/test/wpt/windows-performance-recorder ; https://learn.microsoft.com/en-us/windows/win32/direct3dtools/pix/articles/general/pix-overview ; https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/d3d10-graphics-programming-guide-dxgi ; https://v2.tauri.app/start/prerequisites/

## Implementation Approaches and Technology Adoption

### Technology Adoption Strategies

Boothy에 맞는 채택 전략은 `big bang 교체`가 아니라 `기준선 유지 + 병행 검증 + 점진 승격`이다.

- **Strangler / Branch by Abstraction 계열 접근이 적합**:
  - Martin Fowler의 legacy displacement 정리는 abstraction layer를 유지한 채 새 구현을 붙여 교체 범위를 점진적으로 넓히는 방식이 실용적임을 보여준다.
  - Boothy에서는 `canonical preset contract`, `artifact contract`, `queue contract`를 abstraction layer로 두는 해석이 맞다.
- **권장 채택 순서**:
  - 1단계: darktable baseline 유지
  - 2단계: display lane용 GPU-first prototype 추가
  - 3단계: parity 검증 자동화
  - 4단계: export lane prototype 추가
  - 5단계: 조건부 트래픽 승격
- **빅뱅 교체가 위험한 이유**:
  - 현행 fallback을 잃으면 부스 현장 실패 복구 수단이 약해진다.
  - GPU path 장애, 드라이버 이슈, 품질 회귀를 초기에 모두 감당해야 한다.
- **최종 판단**:
  - 새 엔진은 처음부터 `주력 대체재`가 아니라 `shadow/parallel path`로 들어와야 한다.
  - 승격 기준은 속도뿐 아니라 parity, 안정성, 장애 복구율까지 포함해야 한다.

_Source:_ https://martinfowler.com/articles/patterns-legacy-displacement/ ; https://martinfowler.com/articles/strangler-fig-mobile-apps.html ; https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/checklist

### Development Workflows and Tooling

구현 워크플로는 일반 기능 개발 흐름보다 `계측 가능한 실험 개발`에 가깝게 설계해야 한다.

- **CI는 빠른 기본 검증에 집중**:
  - Azure Well-Architected는 CI를 모든 커밋에서 빌드와 테스트를 자동 실행하는 heartbeat로 설명한다.
  - Boothy에서는 최소한 `build + unit tests + contract tests + smoke-ready checks`가 매 커밋에 돌아야 한다.
- **실험 브랜치 전략**:
  - GPU 서비스, preset canonicalization, parity tooling 같은 큰 변화는 abstraction layer 뒤에서 분리 개발하는 편이 안전하다.
  - 코드 리뷰도 일반 스타일보다 `제품 SLA 영향`, `fallback 유지`, `telemetry 누락` 중심으로 봐야 한다.
- **자동화 원칙**:
  - Microsoft는 반복적이고 절차적인 작업은 자동화하고 CI/CD 도구로 파이프라인을 정의하라고 권장한다.
  - 따라서 benchmark run, ETW 수집, parity diff, artifact 보관은 수동이 아니라 스크립트화해야 한다.
- **도구 조합 권장**:
  - Cargo / Tauri build
  - Windows Performance Toolkit
  - PIX / vendor GPU profiler
  - benchmark orchestration scripts
  - 결과 아카이브 및 비교 대시보드

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/framework/devops/automation-tasks ; https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/release-engineering-performance ; https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/tools-processes ; https://doc.rust-lang.org/cargo/guide/why-cargo-exists.html

### Testing and Quality Assurance

이 프로젝트의 테스트 전략은 `정확성 테스트`만으로 부족하고, `관측 기반 성능/품질 테스트`가 기본이어야 한다.

- **Shift-left 테스트**:
  - Microsoft는 가능한 가장 낮은 레벨에서 자주 테스트하고, 테스트 가능성을 설계 요구로 삼으라고 권장한다.
  - 따라서 canonical recipe parser, artifact contract, queue scheduler, fallback policy는 단위/계약 테스트가 먼저 필요하다.
- **테스트 환경 분리**:
  - Azure 가이드는 각 테스트 단계 목적에 맞는 환경을 의도적으로 설계하라고 한다.
  - Boothy에서는 최소한
    - 로컬 빠른 단위 테스트
    - 실제 GPU 장비 통합 테스트
    - benchmark regression 환경
    - booth-safe smoke 환경
    가 분리되어야 한다.
- **관측을 테스트 프레임워크에 통합**:
  - Azure는 observability를 테스트 프레임워크에 통합하라고 권장한다.
  - ETW 문서도 중요 상태 변화와 시작/종료 이벤트를 계측하라고 설명한다.
  - 따라서 모든 핵심 실험은 `latency trace + GPU trace + artifact diff`를 함께 남겨야 한다.
- **핵심 검증 세트**:
  - preset parity diff
  - first capture / later capture SLA
  - GPU failure fallback
  - wrong-shot / cross-session leakage 0 검증
  - batch export throughput

_Source:_ https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/testing ; https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/tools-processes ; https://learn.microsoft.com/en-us/windows-hardware/test/weg/instrumenting-your-code-with-etw

### Deployment and Operations Practices

배포는 코드 배포가 아니라 `현장 실패를 막는 운영 절차`까지 포함해야 한다.

- **점진 배포 원칙**:
  - Windows의 gradual rollout 문서는 일부 사용자 비율에 먼저 배포하고, 분석 데이터를 보며 확대하거나 중단하라고 안내한다.
  - 부스 앱도 같은 원칙으로, 일부 장비/파일럿 부스에서 먼저 새 GPU path를 활성화해야 한다.
- **채널 구분**:
  - Windows App SDK도 Stable / Preview / Experimental 채널을 분리한다.
  - Boothy도 `stable booth`, `pilot booth`, `experimental bench` 같은 운영 채널 구분이 필요하다.
- **운영 자동화**:
  - startup health check
  - shader/cache priming
  - benchmark snapshot
  - crash/fallback trace 수집
  - 이 항목들은 반복적이므로 자동화가 맞다.
- **롤백과 중단 기준**:
  - Windows driver safe deployment 가이드는 versioning, rollback planning, monitoring, ring strategy를 강조한다.
  - Boothy도 신규 GPU path는 `중단 버튼`, `기존 baseline 즉시 복귀`, `telemetry 근거 기반 판단`이 있어야 한다.

_Source:_ https://learn.microsoft.com/en-us/windows/apps/publish/gradual-package-rollout ; https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/release-channels ; https://learn.microsoft.com/en-us/windows-hardware/drivers/develop/safe-deployment-best-practices-for-drivers ; https://learn.microsoft.com/en-us/azure/architecture/framework/devops/automation-tasks

### Team Organization and Skills

이 작업은 일반 프론트/백엔드 분업보다 `시스템 엔지니어링 + GPU 성능 + 제품 품질`이 같이 필요한 일이다.

- **필수 역할 축**:
  - runtime/system engineer
  - GPU/performance engineer
  - quality/parity owner
  - booth operations owner
- **핵심 역량**:
  - Rust/Tauri 유지보수
  - Windows GPU API 개념
  - ETW/WPR/PIX 기반 성능 분석
  - 이미지 품질 비교와 회귀 판단
  - 장애 재현과 운영 복구 절차
- **현실적 팀 구성 제안**:
  - 대규모 전담 조직보다, 핵심 2~3명이 `contract + prototype + measurement`를 빠르게 돌리는 소수 정예 구조가 유리하다.
  - 단, 품질 승인과 현장 운영 판단은 구현자와 분리하는 편이 안전하다.

_Source:_ https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/checklist ; https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/tools-processes ; https://learn.microsoft.com/en-us/windows-hardware/test/weg/instrumenting-your-code-with-etw

### Cost Optimization and Resource Management

이번 과제의 비용 최적화는 cloud 비용보다 `개발 낭비를 줄이고 현장 GPU 자산을 제대로 쓰는 것`이 우선이다.

- **가장 큰 낭비**:
  - 현재 고성능 GPU 장비를 두고 CPU 중심/비상시적 경로에 머무는 것
  - 측정 없는 구현 반복
  - parity 기준 없이 엔진을 바꾸는 것
- **비용 절감 원칙**:
  - baseline을 유지해 실패 비용을 낮춘다.
  - prototype scope를 display lane 하나로 제한해 학습비를 줄인다.
  - batch export는 2단계로 미뤄 리스크를 분리한다.
  - cloud GPU는 실험실 benchmark 용도로만 제한한다.
- **자원 관리 관점**:
  - GPU, SSD, RAM, CPU를 모두 보는 균형이 필요하지만, 제품 목표상 가장 미활용된 자원은 GPU다.
  - Adobe도 GPU 외 CPU, RAM, SSD의 중요성을 같이 언급한다.

_Source:_ https://helpx.adobe.com/si/lightroom-classic/kb/lightroom-gpu-faq.html ; https://helpx.adobe.com/si/lightroom-classic/kb/optimize-performance-lightroom.html ; https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/checklist

### Risk Assessment and Mitigation

핵심 리스크는 기술 선택보다 `승격 기준 없이 구조를 바꾸는 것`이다.

- **리스크 1. 품질 회귀**
  - 대응: canonical recipe 정의, parity diff 자동화, darktable baseline 비교 유지
- **리스크 2. GPU/드라이버 불안정**
  - 대응: resident service 격리, fallback engine, rollout ring, 중단 가능 배포
- **리스크 3. first capture는 빨라졌는데 later/export가 무너짐**
  - 대응: display lane과 export lane KPI를 분리 추적
- **리스크 4. 구현 복잡도 과증가**
  - 대응: D3D12-first prototype 범위를 display lane에 한정
- **리스크 5. 운영 불가**
  - 대응: ETW/telemetry, startup health check, 현장 복구 절차 자동화

_Source:_ https://learn.microsoft.com/en-us/windows-hardware/drivers/develop/safe-deployment-best-practices-for-drivers ; https://learn.microsoft.com/en-us/windows/apps/publish/gradual-package-rollout ; https://learn.microsoft.com/en-us/windows-hardware/test/weg/instrumenting-your-code-with-etw

## Technical Research Recommendations

### Implementation Roadmap

권장 구현 로드맵은 아래 순서다.

1. canonical preset recipe 최소 스키마 정의
2. darktable adapter를 baseline oracle로 유지
3. resident GPU display lane prototype 구현
4. ETW + PIX + latency telemetry 연결
5. parity diff 자동화
6. pilot booth에서 조건부 활성화
7. export lane prototype 착수

### Technology Stack Recommendations

- 앱 셸: 기존 Tauri/Rust 유지
- 1차 주력 실험: Windows D3D12-first GPU service
- 기준선/복구: darktable OpenCL/CLI 유지
- 저장소: SQLite + filesystem + XMP adapter
- 계측: ETW/WPR + PIX + booth telemetry

### Skill Development Requirements

- Rust 기반 시스템 제어
- D3D12/HLSL 또는 동급 GPU compute 이해
- ETW/WPR/PIX 분석
- 이미지 parity 비교 체계
- 부스 현장 복구 운영

### Success Metrics and KPIs

- latest full-size display `<= 2500ms`
- first capture / later capture 각각의 SLA 분리 추적
- preset parity 승인율
- GPU path 실패 시 fallback 성공률
- RAW 200장 batch export 총 시간
- wrong-shot / cross-session leakage `0`

## Research Synthesis

# Boothy GPU-first Rendering Architecture: Comprehensive Technical Research

## Executive Summary

이번 리서치는 Boothy의 다음 주력 렌더링 구조를 `GPU 최대 활용` 관점에서 다시 정의하기 위해 수행되었다. 조사 범위는 Windows 기반 현행 제품 구조, darktable 및 XMP 기반 기준선, Adobe Lightroom Classic의 GPU 활용 방향, Direct3D 12/OpenCL/Vulkan/CUDA 후보군, 그리고 실제 운영에 필요한 warm-up, fallback, parity, telemetry 체계까지 포함했다.

핵심 결론은 하나다. Boothy는 더 이상 `first-visible을 조금 더 줄이는 미세 튜닝`으로 목표를 닫기 어렵다. 제품이 실제로 요구하는 것은 `고객이 보는 풀사이즈 결과물 <= 2.5초`, `최종 결과와의 시각적 parity`, `RAW 200장 batch export 처리`이며, 이를 만족하려면 `resident GPU-first render service + shared canonical preset recipe + display/export queue 분리`가 중심이 되는 새 구조가 필요하다.

**Key Technical Findings:**

- 현재 구조는 안정성 연구로는 의미가 있지만, 주력 성능 해법으로는 한계가 뚜렷하다.
- darktable는 주력 런타임보다 `baseline`, `fallback`, `parity oracle` 역할로 두는 편이 현실적이다.
- Windows booth 제품 기준 1차 실험 우선순위는 `D3D12-first custom GPU path`가 가장 높다.
- 제품 성공의 핵심은 API 선택보다 `resident warm state`, `queue architecture`, `shared recipe truth`, `telemetry`, `fallback`이다.

**Technical Recommendations:**

- `darktable baseline 유지 + D3D12-first display prototype` 전략으로 시작한다.
- `canonical preset recipe`를 정의하고 XMP는 adapter 대상으로 남긴다.
- `stable / pilot / experimental` 운영 채널을 분리해 점진 승격한다.
- 승인 기준을 `2.5초 display`, `parity`, `fallback`, `200장 export throughput`으로 고정한다.

## Table of Contents

1. Technical Research Introduction and Methodology
2. Technical Landscape and Architecture Analysis
3. Implementation Approaches and Best Practices
4. Technology Stack Evolution and Current Trends
5. Integration and Interoperability Patterns
6. Performance and Scalability Analysis
7. Security and Operational Considerations
8. Strategic Technical Recommendations
9. Implementation Roadmap and Risk Assessment
10. Future Technical Outlook
11. Methodology and Source Verification
12. Reference Materials

## 1. Technical Research Introduction and Methodology

### Technical Research Significance

이 리서치가 중요한 이유는 제품의 성공 기준이 바뀌었기 때문이다. 이제 Boothy의 핵심 artifact는 rail thumbnail이 아니라 `고객이 크게 보는 풀사이즈 결과물`이며, 이 artifact에 대한 SLA와 최종 export 품질이 동시에 중요해졌다. Adobe가 2025년 기준 Lightroom Classic에서 `Display`, `Image Processing`, `Export`, `Preview Generation`까지 GPU 범위를 확장하고 있는 점은, Boothy 역시 GPU를 보조 자원이 아니라 파이프라인 중심 자원으로 다뤄야 함을 보여준다.

_Technical Importance:_ GPU는 선택이 아니라 주력 구조의 중심이다.  
_Business Impact:_ 현재 구조를 유지한 채 미세 튜닝만 반복하면 제품 목표와 현장 체감 품질 사이의 격차가 계속 남는다.  
_Sources:_ https://helpx.adobe.com/si/lightroom-classic/kb/lightroom-gpu-faq.html ; https://helpx.adobe.com/si/lightroom-classic/kb/gpu-preview-generation.html

### Technical Research Methodology

- **Technical Scope**: 기술 스택, 통합, 아키텍처, 구현, 운영 전략
- **Primary Inputs**: 로컬 구조 문서, 현재 코드 근거, 공식 기술 문서
- **Verification Approach**: Microsoft, Adobe, Khronos, NVIDIA, Tauri, SQLite 등 1차 출처 우선
- **Analysis Framework**: 제품 목표 적합성, 운영 리스크, 점진 도입 가능성, fallback 유지 가능성
- **Time Focus**: 2026-04-11 기준 최신 공개 자료 확인

### Technical Research Goals and Objectives

**Original Technical Goals:** 고객이 보는 풀사이즈 결과와 최종 export 품질의 parity를 유지하면서 GPU를 최대 활용하는 새 렌더링 구조를 검토한다.

**Achieved Technical Objectives:**

- 새 구조의 중심이 `resident GPU-first architecture`여야 한다는 결론 도출
- darktable를 `baseline/fallback/reference`로 재정의
- D3D12-first prototype이 가장 현실적인 출발점이라는 우선순위 확정
- 제품 승인 기준과 운영 채널 전략 정리

## 2. Technical Landscape and Architecture Analysis

### Current Technical Landscape

현재 기술 환경은 Boothy에 유리하다. Windows 단일 제품이라는 전제는 cross-platform 복잡도를 줄이고, D3D12 같은 Windows-native GPU 경로를 더 직접적으로 활용할 수 있게 한다. 동시에 Tauri/Rust 기반 현행 앱 셸은 유지 가능성이 높아 전체 재작성 없이 새 render service를 붙일 수 있다.

### Architecture Synthesis

아키텍처 관점에서 가장 맞는 형태는 `modular monolith + local GPU service`다. full microservices는 과하고, 단일 프로세스 강화형 구조는 warm GPU state 유지와 장애 격리에 불리하다. 따라서 제품 관점의 권장 구조는 아래 네 계층으로 정리된다.

- `Tauri UI`
- `Application Core`
- `Resident GPU Render Service`
- `darktable baseline/fallback`

_Sources:_ https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/ ; https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/microservices ; https://v2.tauri.app/concept/architecture/

## 3. Implementation Approaches and Best Practices

### Implementation Method

도입 방식은 `big bang replacement`가 아니라 `baseline 유지 + 병행 검증 + 점진 승격`이 맞다. 이는 제품 리스크를 낮추고, 성능과 품질을 동시에 측정 가능한 방식으로 관리하게 해준다. 새 엔진은 처음부터 대체재가 아니라 shadow path로 들어와야 하며, parity와 fallback이 확인된 뒤에만 승격해야 한다.

### Operational Best Practice

실험 개발은 일반 기능 개발과 다르다. benchmark, telemetry, parity diff, fallback recovery 검증이 파이프라인의 기본이 되어야 한다. 운영적으로도 `stable`, `pilot`, `experimental` 채널 구분이 필요하다.

_Sources:_ https://martinfowler.com/articles/patterns-legacy-displacement/ ; https://learn.microsoft.com/en-us/windows/apps/publish/gradual-package-rollout ; https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/release-channels

## 4. Technology Stack Evolution and Current Trends

### Recommended Stack Direction

- 앱 셸: 기존 `Tauri + Rust`
- 1차 GPU 실험: `Direct3D 12 + HLSL`
- 기준선/복구: `darktable OpenCL/CLI`
- 저장소: `SQLite + filesystem + XMP adapter`
- 계측: `ETW/WPR + PIX + booth telemetry`

### Trend Interpretation

Adobe의 최근 GPU 활용 확대와 Microsoft GPU 도구 체계의 성숙도를 보면, Windows 현장 장비에서 GPU를 적극 활용하는 제품 구조는 충분히 현실적이다. 반면 Vulkan/CUDA는 각각 복잡도와 벤더 종속 리스크가 있어 1차 선택으로는 우선순위가 낮다.

_Sources:_ https://helpx.adobe.com/si/lightroom-classic/kb/lightroom-gpu-faq.html ; https://learn.microsoft.com/en-us/windows/win32/direct3d12/pipelines-and-shaders-with-directx-12 ; https://learn.microsoft.com/en-us/windows/win32/direct3dtools/pix/articles/general/pix-overview

## 5. Integration and Interoperability Patterns

### Integration Summary

Boothy의 통합 구조는 복잡한 분산 API가 아니라 `same-machine control plane` 중심이어야 한다. UI와 코어는 Tauri IPC, 코어와 렌더 서비스는 local IPC, 대용량 데이터는 shared memory 또는 GPU resident resource로 분리하는 쪽이 적절하다.

### Interoperability Principle

엔진 간 상호운용성의 핵심은 `canonical preset recipe`다. XMP는 현재와의 호환성을 위해 유지하되, 장기적으로는 내부 canonical recipe를 중심에 두고 darktable와 새 GPU 엔진이 모두 adapter로 붙는 구조가 유리하다.

_Sources:_ https://v2.tauri.app/concept/inter-process-communication/ ; https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-server-using-overlapped-i-o ; https://developer.adobe.com/xmp/docs/xmp-specifications/

## 6. Performance and Scalability Analysis

### Performance Thesis

현재 문제는 UI가 아니라 render body다. 따라서 다음 성능 해법의 핵심은 아래 네 가지다.

- resident warm GPU context
- display/export queue 분리
- GPU memory reuse
- startup priming과 cache strategy

### Scalability Interpretation

Boothy에서 scalability는 서버 수평 확장이 아니라 `장비 한 대 안에서 latency와 throughput을 동시에 관리하는 능력`이다. 따라서 display와 export는 같은 truth를 공유하되 다른 queue와 비용 구조를 가지는 것이 맞다.

_Sources:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/cache-aside ; https://learn.microsoft.com/en-us/windows/win32/direct3d12/memory-management-strategies ; https://learn.microsoft.com/en-us/windows/win32/direct3d12/executing-and-synchronizing-command-lists

## 7. Security and Operational Considerations

### Security

보안은 외부 인증보다 로컬 권한 경계가 핵심이다. Tauri capability 최소화, local IPC ACL 제한, render service 격리가 기본 원칙이다.

### Operations

운영 성공의 핵심은 단순한 속도보다 `warm-up orchestration`, `telemetry`, `fallback recovery`를 제품 구조에 포함시키는 것이다. ETW/WPR/PIX를 조합한 측정 체계가 필수다.

_Sources:_ https://v2.tauri.app/security/capabilities/ ; https://learn.microsoft.com/en-us/windows/win32/etw/about-event-tracing ; https://learn.microsoft.com/en-us/windows-hardware/test/wpt/windows-performance-recorder

## 8. Strategic Technical Recommendations

### Primary Recommendation

가장 현실적인 전략은 아래 조합이다.

1. 기존 Tauri/Rust 앱 셸 유지
2. darktable baseline/fallback 유지
3. D3D12-first resident display prototype 추가
4. canonical preset recipe 정의
5. parity와 fallback을 기준으로 승격

### Strategic Implication

이 전략은 제품 리스크를 낮추면서도 GPU 자산을 가장 빠르게 성과로 전환할 수 있는 경로다. 즉 `지금 가진 제품을 버리지 않고`, `새 GPU 경로를 주력 후보로 검증`하는 방식이다.

## 9. Implementation Roadmap and Risk Assessment

### Recommended Roadmap

1. canonical preset recipe 최소 스키마 정의
2. darktable adapter를 baseline oracle로 유지
3. resident GPU display lane prototype 구현
4. ETW + PIX + latency telemetry 연결
5. parity diff 자동화
6. pilot booth에서 조건부 활성화
7. export lane prototype 착수

### Risk Assessment

- 품질 회귀: baseline parity 자동 비교로 완화
- GPU/드라이버 불안정: resident service + fallback engine으로 완화
- 운영 불가: rollout ring + telemetry + rollback으로 완화
- 구현 복잡도 과증가: 1차 prototype 범위를 display lane으로 제한

_Sources:_ https://learn.microsoft.com/en-us/windows-hardware/drivers/develop/safe-deployment-best-practices-for-drivers ; https://learn.microsoft.com/en-us/windows/apps/publish/gradual-package-rollout

## 10. Future Technical Outlook

향후 1~2년 관점에서는 `GPU preview generation`, `stronger warm-state orchestration`, `shared recipe governance`의 중요성이 더 커질 가능성이 높다. 중기적으로는 canonical preset recipe를 중심으로 display/export/baseline을 묶는 구조가 Boothy의 제품 유연성을 크게 높일 것이다.

## 11. Methodology and Source Verification

### Primary Source Families

- Microsoft Learn
- Adobe HelpX / Adobe Developer
- Tauri 공식 문서
- Khronos 공식 문서
- NVIDIA 공식 문서
- SQLite 공식 문서
- Martin Fowler 아키텍처 글

### Verification Note

시간 변화 가능성이 큰 항목은 공식 문서 기준으로 확인했다. 특히 Lightroom Classic GPU 활용, Windows 배포 채널, gradual rollout, Direct3D 12 관련 내용은 최신 공개 문서를 우선 사용했다.

## 12. Reference Materials

### Local References

- [architecture-change-foundation-2026-04-11.md](/C:/Code/Project/Boothy_thumbnail-latency-seam-reinstrumentation/docs/architecture-change-foundation-2026-04-11.md)
- [mod.rs](/C:/Code/Project/Boothy_thumbnail-latency-seam-reinstrumentation/src-tauri/src/render/mod.rs)
- [capture_readiness.rs](/C:/Code/Project/Boothy_thumbnail-latency-seam-reinstrumentation/src-tauri/tests/capture_readiness.rs)

### External References

- https://helpx.adobe.com/si/lightroom-classic/kb/lightroom-gpu-faq.html
- https://helpx.adobe.com/si/lightroom-classic/kb/gpu-preview-generation.html
- https://helpx.adobe.com/si/lightroom-classic/help/lightroom-smart-previews.html
- https://learn.microsoft.com/en-us/windows/win32/direct3d12/pipelines-and-shaders-with-directx-12
- https://learn.microsoft.com/en-us/windows/win32/direct3dtools/pix/articles/general/pix-overview
- https://learn.microsoft.com/en-us/windows/apps/publish/gradual-package-rollout
- https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/release-channels
- https://v2.tauri.app/concept/architecture/
- https://v2.tauri.app/concept/inter-process-communication/
- https://developer.adobe.com/xmp/docs/xmp-specifications/

---

## Technical Research Conclusion

### Summary of Key Technical Findings

- Boothy의 핵심 문제는 더 이상 썸네일이 아니라 `풀사이즈 결과를 진실하게 빨리 보여주는 구조`다.
- 현재 구조의 추가 미세 튜닝보다 `resident GPU-first architecture` 전환이 수익이 크다.
- darktable는 계속 필요하지만, 역할은 `주력 엔진`이 아니라 `baseline/fallback/reference`가 맞다.
- 1차 실험 우선순위는 `D3D12-first display prototype`이다.

### Strategic Technical Impact Assessment

이 결론은 제품 방향을 바꾼다. Boothy의 다음 경쟁력은 단순 렌더 속도가 아니라, `빠른 표시`, `최종 결과 parity`, `안전한 fallback`, `현장 운영 안정성`을 한 구조로 묶는 데서 나온다.

### Next Steps Technical Recommendations

1. `canonical preset recipe` 정의를 바로 시작한다.
2. `display lane D3D12 prototype`을 가장 작은 범위로 구현한다.
3. `parity diff + telemetry`를 먼저 자동화한다.
4. `pilot booth` 채널을 분리하고 baseline과 병행 검증한다.

---

**Technical Research Completion Date:** 2026-04-11  
**Research Period:** current comprehensive technical analysis  
**Source Verification:** official technical sources prioritized  
**Technical Confidence Level:** High for architectural direction, Medium for final engine choice pending prototype benchmarks

<!-- Content will be appended sequentially through research workflow steps -->
