---
stepsCompleted: [1, 2, 3, 4, 5, 6]
inputDocuments: []
workflowType: 'research'
lastStep: 1
research_type: 'technical'
research_topic: 'Boothy preset-applied preview architecture decision'
research_goals: 'Choose the architecture with the highest probability of achieving original-visible to preset-applied-visible <= 2.5 seconds on real booth hardware, without breaking same-capture guarantees, preset fidelity, or preview/final truth contracts.'
user_name: 'Noah Lee'
date: '2026-04-09'
web_research_enabled: true
source_verification: true
---

# Research Report: technical

**Date:** 2026-04-09
**Author:** Noah Lee
**Research Type:** technical

---

## Research Overview

This research was conducted to choose Boothy's next architecture for reducing `preset-applied preview close` on real booth hardware. The decision target was not "show any image quickly" but "show the same capture's preset-applied result quickly enough to matter in product terms", with an ideal target of `original visible -> preset-applied visible <= 2.5s`.

Across technology, integration, architecture, and implementation analysis, one conclusion remained consistent: the highest-probability path is a **local dedicated renderer** combined with an explicit **different close topology** that separates `same-capture first-visible` from `preset-applied truthful close`. This path removes the most direct bottlenecks without weakening same-capture guarantees, preset fidelity, or preview/final truth contracts.

The rest of this document records the evidence behind that decision, compares the main alternatives, and provides a practical validation plan. For the final recommendation and execution framing, see the **Research Synthesis** section below.

## Technical Research Scope Confirmation

**Research Topic:** Boothy preset-applied preview architecture decision
**Research Goals:** Choose the architecture with the highest probability of achieving original-visible to preset-applied-visible <= 2.5 seconds on real booth hardware, without breaking same-capture guarantees, preset fidelity, or preview/final truth contracts.

**Technical Research Scope:**

- Architecture Analysis - design patterns, worker boundaries, queue topology, and system architecture for low-latency preset-applied preview
- Implementation Approaches - development methodologies, rendering paths, preset-application approaches, and bottleneck removal strategies
- Technology Stack - local render engines, watch-folder interoperability, edge deployment options, and supporting runtime tools
- Integration Patterns - APIs, sidecar protocols, filesystem handoff, event propagation, and interoperability boundaries
- Performance Considerations - scalability, warm-state retention, caching, hardware acceleration, and latency-sensitive patterns

**Research Methodology:**

- Current web data with rigorous source verification
- Multi-source validation for critical technical claims
- Confidence level framework for uncertain information
- Comprehensive technical coverage with architecture-specific insights

**Scope Confirmed:** 2026-04-09

---

<!-- Content will be appended sequentially through research workflow steps -->

## Technology Stack Analysis

### Programming Languages

Boothy의 `preset-applied preview` 문제에서 핵심 hot path는 여전히 네이티브 계층이 유리하다. Canon camera control은 Canon의 EDSDK/CCAPI 계열 API를 전제로 하고, RAW decode 및 preset-accurate render는 darktable, LibRaw, RawSpeed처럼 C/C++ 기반 스택이 주도한다. Canon은 2026년 4월 기준 EDSDK를 Windows, Mac, Raspberry Pi OS, Ubuntu에서 공통 코드로 제어 가능하다고 안내하고, CCAPI는 무선과 다중 플랫폼 제어를 확장 경로로 제시한다. 이는 카메라 제어 계층을 네이티브 또는 sidecar 경계로 유지하는 현재 Boothy 방향과 맞는다.

현재 저장소 계약도 같은 결론을 지지한다. 카메라 truth는 `canon-helper.exe` sidecar가 소유하고, render truth는 `darktableVersion + xmpTemplatePath + previewProfile/finalProfile` 조합으로 닫히도록 설계되어 있다. 따라서 상위 앱은 Rust/Tauri host와 React/TypeScript shell로 유지하되, 실제 지연을 좌우하는 경로는 C/C++ 엔진 또는 그 wrapper를 resident worker로 다루는 편이 현실적이다.

_Popular Languages:_ C/C++(camera SDK, RAW decode, render engine), Rust(host orchestration), TypeScript/React(UI shell)  
_Emerging Languages:_ Rust는 orchestration과 안정성 측면에서 매력적이지만, RAW truth engine 자체를 곧바로 대체할 주류는 아님  
_Language Evolution:_ 제품 shell은 생산성 언어로, hot path는 네이티브 엔진으로 분리하는 경향이 강함  
_Performance Characteristics:_ LibRaw는 한 processor 인스턴스가 한 번에 한 source만 처리하지만 다중 인스턴스 병렬은 가능하다고 문서화한다. 대신 메모리 비용이 커서 resident worker 설계가 더 중요하다.  
_Source:_ https://www.usa.canon.com/support/sdk, https://www.libraw.org/docs/API-overview.html, https://www.libraw.org/docs

### Development Frameworks and Libraries

Boothy 후보 구조를 실제로 받쳐줄 프레임워크는 다섯 갈래로 정리된다. 첫째, **truth-preserving full apply engine**으로는 darktable이 가장 직접적이다. darktable 문서는 XMP sidecar에 전체 편집 이력을 저장하는 비파괴 구조와 `darktable-cli`의 headless export 경로를 공식 지원한다. 둘째, **fast decode layer**로는 LibRaw/RawSpeed 계열이 있다. LibRaw는 `unpack_thumb`, `unpack`, dcraw-style postprocessing을 제공하고, RawSpeed는 "가장 빠른 decoding speed"를 목표로 하지만 color correction, demosaic, viewable thumbnail까지는 담당하지 않는다고 밝힌다. 이 조합은 `lighter truthful renderer` 후보가 어디까지 truthful할 수 있는지 경계를 분명히 해준다.

셋째, 앱 통합 프레임워크로는 Tauri sidecar 구조가 Boothy와 매우 잘 맞는다. Tauri v2 문서는 외부 바이너리를 sidecar로 번들하고 Rust/JavaScript 양쪽에서 실행하는 경로를 공식화한다. 즉, `local dedicated renderer`나 `canon-helper.exe` 같은 보조 프로세스를 앱 번들 안의 정식 런타임 컴포넌트로 관리하기 쉽다. 넷째, **watch-folder bridge**는 Adobe Lightroom Classic의 Auto Import가 공식 지원하는 상호운용 패턴이다. watched folder를 감시해 자동 import하고, Develop settings를 적용하며, `Standard` initial previews를 렌더할 수 있다. 다만 watched folder는 비어 있어야 하고 subfolder를 감시하지 않는 제약이 있다. 다섯째, Capture One은 tethered capture에서 `Immediately`와 `When ready`를 별도 정책으로 노출한다. 이 점은 제품들이 이미 “빠른 프리뷰”와 “조정이 완료된 고품질 프리뷰”를 정책적으로 분리한다는 강한 증거다.

_Major Frameworks:_ darktable/darktable-cli, LibRaw, RawSpeed, Tauri sidecar, Canon EDSDK/CCAPI  
_Micro-frameworks:_ File watcher, stdio/JSON-lines sidecar protocol, local job queue, filesystem handoff  
_Evolution Trends:_ 단일 blocking render보다 dual-lane preview 정책과 sidecar 분리가 강화되는 흐름  
_Ecosystem Maturity:_ darktable/LibRaw/Tauri는 문서와 유지보수 기반이 충분하고, watched-folder는 상용 제품 호환성 면에서 성숙함  
_Source:_ https://docs.darktable.org/usermanual/development/en/special-topics/program-invocation/darktable-cli/, https://docs.darktable.org/usermanual/development/en/overview/sidecar-files/sidecar/, https://www.libraw.org/docs/API-C.html, https://github.com/darktable-org/rawspeed, https://v2.tauri.app/ko/develop/sidecar/, https://helpx.adobe.com/in/lightroom-classic/help/import-photos-automatically.html, https://support.captureone.com/hc/en-us/articles/360002556797-Selecting-the-appropriate-capture-preview

### Database and Storage Technologies

이 문제의 핵심 저장소는 전통적인 관계형 DB보다 **filesystem artifact + sidecar metadata + durable manifest**다. darktable은 원본 RAW를 건드리지 않고 XMP sidecar에 processing steps를 저장하며, 동시에 빠른 접근을 위해 자체 library DB를 유지한다. 이 구조는 preset truth를 artifact로 고정하고, preview/final render를 파일로 분리해 관리하려는 Boothy의 현재 session-manifest 계약과 정합성이 높다.

반면 watch-folder bridge는 저장소 경계를 더 불안정하게 만든다. Lightroom Auto Import는 watched folder가 비어 있어야 하고 subfolder를 감시하지 않으며, 감지 후 destination으로 파일을 이동한다. 즉, same-capture correlation과 canonical preview path를 Boothy가 직접 소유하기 어렵고, 브리지 상대 제품의 catalog/storage 정책에 일부 종속된다. 따라서 Boothy의 1차 저장 전략은 로컬 세션 루트 아래 `originals/previews/finals`를 분리하고, preview truth close를 file existence + correlation으로 닫는 현재 방향이 맞다.

_Relational Databases:_ hot path에는 우선순위가 낮음. 운영 메타데이터나 catalog 관리에는 유용하지만 preview close를 직접 줄이지는 못함  
_NoSQL Databases:_ 이벤트 로그/진단 저장소로는 가능하나, 핵심 preview truth는 여전히 파일 artifact가 기준  
_In-Memory Databases:_ 캐시/큐에는 유효할 수 있으나 reboot/rollback 내구성을 보완해야 함  
_Data Warehousing:_ 이번 결정에는 비핵심. 다만 hardware validation 측정 데이터 집계에는 별도 분석 저장소가 유용  
_Source:_ https://docs.darktable.org/usermanual/development/en/overview/sidecar-files/sidecar/, https://helpx.adobe.com/in/lightroom-classic/help/import-photos-automatically.html

### Development Tools and Platforms

운영 가능한 구조를 만들려면 개발 도구보다 **런타임 운영 도구**가 더 중요하다. darktable은 OpenCL 활성화, `-d opencl -d perf` 기반 profiling, CPU fallback 동작을 공식 문서로 제공한다. 이는 `dedicated renderer` 후보에서 warm state와 GPU 가속을 검증하는 데 직접 도움이 된다. Microsoft의 `FileSystemWatcher` 문서는 watch-folder 기반 통합이 생성 직후 이벤트를 올리고, 복사 중에도 `Created`가 먼저 발생할 수 있으며, 일반적인 작업에서 여러 이벤트가 중복 발생할 수 있고, 버퍼 overflow 시 변경 추적을 잃을 수 있다고 명시한다. 즉, watched-folder bridge는 제품화하려면 file-ready polling, dedupe, overflow recovery가 필수다.

Boothy는 이미 sidecar protocol, session manifest, render worker contract를 갖추고 있다. 여기에 필요한 도구는 새 프레임워크보다 end-to-end 계측과 하드웨어 반복 측정이다. 제품 의사결정 관점에서 중요한 것은 “어떤 코드가 더 예쁜가”가 아니라 “resident worker warm-up 이후 close latency의 p50/p95를 반복 측정할 수 있는가”다.

_IDE and Editors:_ 이번 의사결정의 차별점은 아님  
_Version Control:_ preset artifact version pin과 rollback 안전성이 더 중요  
_Build Systems:_ sidecar binary packaging, native dependency pinning, booth hardware 재현 빌드가 중요  
_Testing Frameworks:_ synthetic benchmark보다 real booth hardware latency trace가 우선  
_Source:_ https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/, https://learn.microsoft.com/en-us/dotnet/api/system.io.filesystemwatcher.created?view=net-9.0, https://learn.microsoft.com/en-us/dotnet/api/system.io.filesystemwatcher.changed?view=net-9.0, https://learn.microsoft.com/en-us/dotnet/api/system.io.filesystemwatcher.internalbuffersize?view=net-9.0

### Cloud Infrastructure and Deployment

이번 주제는 cloud-first보다 **local-first deployability**가 훨씬 중요하다. 가장 현실적인 배포 기본값은 Windows booth PC 안에서 Canon helper, render worker, Tauri host가 함께 동작하는 형태다. Canon EDSDK도 Windows 경로가 가장 직접적이고, Tauri sidecar도 로컬 번들 배포를 전제로 한다. 이 조합은 same-capture correlation을 프로세스 내부 또는 같은 장비의 filesystem 경계에서 유지할 수 있어 truth 계약 보전에 유리하다.

`edge appliance` 후보는 별도 장비에 렌더 파이프라인을 상주시켜 booth 본체의 CPU/GPU 압박을 줄일 수 있다는 점이 장점이다. 다만 이 경우에도 네트워크 hop, 장비 관리, 재배포, 현장 장애 복구가 추가된다. Azure IoT Edge 문서는 모듈이 로컬에서 오프라인으로 계속 동작할 수 있다고 설명하고, NVIDIA Holoscan은 GPU-resident graph로 CPU orchestration overhead를 줄이는 패턴을 제시한다. 그러나 이는 Boothy의 현 단계에서 “가능한 패턴”을 보여주는 자료이지, 당장 가장 단순하고 성공 확률 높은 배포 기본값은 아니다. 실질적으로는 dedicated local renderer가 먼저이고, edge appliance는 local GPU/CPU headroom이 구조적으로 부족할 때의 2차 카드다.

_Major Cloud Providers:_ 이번 결정의 1순위는 아님  
_Container Technologies:_ edge appliance 운영 시 유용하지만 booth 단일 PC에는 필수 아님  
_Serverless Platforms:_ preview close 문제와 거리가 멀고 cold start 리스크가 큼  
_CDN and Edge Computing:_ 이 문제에서의 “edge”는 CDN이 아니라 booth 근처의 로컬 또는 근거리 appliance를 뜻함  
_Source:_ https://v2.tauri.app/ko/develop/sidecar/, https://www.usa.canon.com/support/sdk, https://learn.microsoft.com/en-us/azure/iot-edge/about-iot-edge, https://learn.microsoft.com/en-us/azure/iot-edge/offline-capabilities, https://docs.nvidia.com/holoscan/sdk-user-guide/gpu_resident.html

### Technology Adoption Trends

현재 기술 채택 흐름은 세 가지로 요약된다. 첫째, **preview policy의 명시화**다. Capture One은 `Immediately`와 `When ready`를 제품 옵션으로 분리해 보여준다. 이는 “빠른 first-visible”과 “조정이 끝난 truthful preview”를 같은 것으로 취급하지 않는다는 뜻이다. 둘째, **compatibility bridge의 지속적 존재**다. Lightroom Auto Import는 tethered import가 직접 지원되지 않을 때 watched folder를 공식 우회 경로로 둔다. 이는 watch-folder bridge가 산업적으로 실재하는 패턴임을 보여주지만, 동시에 direct integration이 안 될 때 쓰는 보조 경로라는 신호이기도 하다. 셋째, **GPU/low-overhead resident execution에 대한 관심 증가**다. darktable의 OpenCL, NVIDIA의 GPU-resident graphs는 공통적으로 warm 상태 유지와 orchestration overhead 감소를 통해 latency를 줄이려는 방향을 보여준다.

이 흐름을 Boothy에 적용하면, 단순 미세 튜닝보다 `resident local renderer + explicit dual-truth preview topology`가 기술 채택 방향과 가장 잘 맞는다. 반대로 watch-folder bridge는 실무적으로 쓸 수는 있지만, 장기 코어 아키텍처라기보다 상용 엔진을 빠르게 검증하거나 임시로 붙이는 interoperability 레이어에 가깝다. 이 판단은 일부 상용 제품 내부 구현이 비공개라는 점에서 부분적으로 추정이지만, 공식 문서에 드러난 제품 정책과 공개 엔진 특성으로 볼 때 신뢰도는 높다.

_Migration Patterns:_ monolith/inline render에서 worker/sidecar/resident path로 이동  
_Emerging Technologies:_ GPU-resident execution, dual preview policy, richer intermediate preview  
_Legacy Technology:_ blocking single-path full render, UI thread-close coupling  
_Community Trends:_ open-source decode/render 엔진 + product shell 분리  
_Source:_ https://support.captureone.com/hc/en-us/articles/360002556797-Selecting-the-appropriate-capture-preview, https://helpx.adobe.com/in/lightroom-classic/help/import-photos-automatically.html, https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/, https://docs.nvidia.com/holoscan/sdk-user-guide/gpu_resident.html

## Integration Patterns Analysis

### API Design Patterns

Boothy의 핵심 통합 문제는 일반적인 public API 설계보다, **로컬 장비 안에서 capture truth와 render truth를 어떻게 분리하면서도 강하게 연결하느냐**에 있다. 현재 저장소 계약은 이 점에서 이미 좋은 방향을 잡고 있다. 카메라 helper는 `request-capture -> capture-accepted -> file-arrived` 식의 command/response/event 흐름을 쓰고, render worker는 file existence 기반으로 `previewReady`와 `finalReady`를 닫는다. 이 구조는 local dedicated renderer와 가장 잘 맞는다.

API 패턴 관점에서 보면, local-first 1순위는 **point-to-point local RPC + event callback**이다. Tauri는 sidecar 바이너리를 번들링하고 실행하는 경로를 공식 지원하므로, Boothy host가 `canon-helper.exe`와 renderer worker를 직접 child process로 관리할 수 있다. 이 경우 API surface는 크지 않아도 된다. `warm(renderer)`, `submitPreviewJob(captureId, presetVersion, rawPath)`, `previewReady(captureId, path, readyAtMs)` 정도면 충분하다. 반면 REST API는 이 문제에 맞지 않는다. HTTP path parsing, local port 관리, auth surface, retry semantics까지 모두 새 부담이 생기지만 same-capture close에는 도움이 적다.

gRPC는 edge appliance나 별도 로컬 서비스 프로세스로 확장할 때는 의미가 있다. gRPC는 HTTP/2 기반의 양방향 스트리밍과 Protobuf 기반 IDL을 제공하고, 저지연 서비스 간 통신에 적합하다고 문서화한다. 다만 booth 단일 PC 안에서는 gRPC가 과한 경우가 많다. 포트, lifecycle, 인증, 버전 협상까지 운영 복잡도를 늘리기 때문이다. 따라서 현재 시점의 기본 선택은 `Tauri host -> local sidecar renderer` direct RPC이고, gRPC는 edge appliance 후보에서만 유의미한 2차 옵션이다.

_RESTful APIs:_ booth 내부 hot path에는 과함. same-capture close보다 운영 surface만 커짐  
_GraphQL APIs:_ 이번 문제와 부적합. query flexibility보다 deterministic command path가 중요  
_RPC and gRPC:_ edge appliance 또는 독립 로컬 서비스에 적합. 단일 PC 기본 경로로는 복잡도 증가  
_Webhook Patterns:_ 로컬 제품에서는 부적합. 내부 event callback이나 local IPC가 더 자연스러움  
_Source:_ https://v2.tauri.app/ko/develop/sidecar/, https://grpc.io/docs/what-is-grpc/introduction/, https://grpc.io/about/

### Communication Protocols

가장 유력한 통신 프로토콜은 **stdio JSON Lines 기반 sidecar IPC**다. 현재 Boothy 계약도 이 방식을 canonical protocol로 채택하고 있다. 이는 프로세스 부모-자식 관계가 뚜렷하고, 별도 포트를 열 필요가 없으며, requestId/captureId/sessionId correlation을 application layer에서 명확히 유지할 수 있다는 장점이 있다. Windows IPC 관점에서도 parent-child 로컬 통신에는 pipe 계열이 가장 직접적이다. Microsoft 문서는 anonymous pipe가 parent-child 간 로컬 통신에 적합하고 네트워크를 타지 않는다고 설명한다.

대안으로 **named pipe**도 가능하다. named pipe는 단방향 또는 duplex 통신을 지원하고 같은 이름 아래 여러 인스턴스를 열 수 있으며, 보안 검사 대상이 된다. renderer를 host child process가 아니라 상주 로컬 서비스로 바꾸고 싶다면 named pipe가 stdio보다 나은 선택이 될 수 있다. 하지만 현재 Boothy처럼 host가 sidecar lifecycle을 직접 소유하는 구조에서는 stdio가 더 단순하다.

**gRPC/HTTP2**는 edge appliance에 적합하다. 장점은 양방향 streaming, health check, metadata, TLS/mTLS 확장성이다. 단점은 네트워크 hop과 서비스 배포 복잡도다. **watch-folder bridge**는 엄밀히 말하면 protocol이라기보다 file-based integration이다. Adobe 문서는 watched folder가 비어 있어야 하고, subfolder를 감시하지 않으며, 감지한 파일을 destination으로 이동하고, Develop settings와 Standard previews를 적용할 수 있다고 설명한다. 이 패턴은 interoperability에는 좋지만, `same-capture canonical preview path`를 Boothy가 직접 소유하기 어렵게 만든다.

_HTTP/HTTPS Protocols:_ edge appliance control plane에서는 가능하지만 booth hot path 기본값으로는 비효율  
_WebSocket Protocols:_ 실시간 상태 스트림 용도는 가능하나, 로컬 sidecar에서는 stdio가 더 단순  
_Message Queue Protocols:_ 외부 broker는 과함. 로컬 in-process queue나 worker queue로 충분  
_grpc and Protocol Buffers:_ edge appliance 통신에서만 적극 검토 가치가 큼  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/ipc/anonymous-pipes, https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipes, https://grpc.io/docs/what-is-grpc/core-concepts/, https://helpx.adobe.com/lightroom-classic/help/import-photos-automatically.html

### Data Formats and Standards

데이터 포맷은 크게 네 층으로 나뉜다. 첫째, **control plane**은 JSON 또는 JSON Lines가 적합하다. 현재 Boothy의 sidecar protocol처럼 line-delimited JSON은 사람과 기계가 함께 읽기 쉽고, 진단과 replay에 유리하다. 둘째, **preset truth plane**은 darktable XMP sidecar가 가장 중요하다. darktable 공식 문서는 XMP sidecar에 full editing history와 processing steps를 저장한다고 명시한다. 이것이 현재 Boothy가 말하는 preset fidelity와 preview/final truth 계약의 핵심 근거다.

셋째, **artifact plane**은 RAW/JPEG 파일 그 자체다. RAW는 source truth, preview/final JPEG는 user-visible truth close artifact다. 넷째, **high-performance remote serialization**은 Protobuf가 적합하다. gRPC는 Protocol Buffers를 기본 IDL 및 payload 형식으로 사용한다. 따라서 edge appliance로 갈 경우에는 JSON보다 Protobuf가 더 적절할 가능성이 높다. 다만 booth 단일 PC에서는 그 이점을 크게 체감하기 어렵다.

watch-folder bridge는 flat file 기반이라 직관적이지만, file readiness와 ownership semantics가 약하다. Microsoft `FileSystemWatcher` 문서는 변경이 짧은 시간에 많이 일어나면 버퍼가 overflow되어 변화를 놓칠 수 있고, 일반적인 파일 복사/이동에서도 생성 이벤트만으로 완료를 보장하지 않는다고 설명한다. 즉, file drop을 canonical truth plane으로 삼으려면 추가 polling과 completion 확인이 필수다.

_JSON and XML:_ control plane은 JSON, preset truth는 XMP(XML 기반)로 역할이 다름  
_Protobuf and MessagePack:_ edge appliance용 remote RPC에는 Protobuf 우세  
_CSV and Flat Files:_ watch-folder bridge에서는 유효하지만 truth ownership이 약함  
_Custom Data Formats:_ `session.json`, line-delimited helper events, preset manifest는 도메인 특화 형식으로 유지 가치가 큼  
_Source:_ https://docs.darktable.org/usermanual/development/en/overview/sidecar-files/sidecar/, https://grpc.io/docs/what-is-grpc/introduction/, https://learn.microsoft.com/en-us/dotnet/fundamentals/runtime-libraries/system-io-filesystemwatcher

### System Interoperability Approaches

후보별 상호운용 방식은 명확히 세 종류로 갈린다. **local dedicated renderer**는 point-to-point direct integration이다. host가 camera helper와 renderer를 직접 소유하고, filesystem artifact와 IPC event를 조합해 truth를 닫는다. 이 방식은 same-capture guarantee를 가장 단단하게 유지한다. **watch-folder bridge**는 asynchronous loose coupling이다. 파일 드롭과 외부 엔진 감시를 이용하므로 엔진 교체는 쉽지만 correlation과 rollback 통제가 약해진다. **edge appliance**는 service-to-service distributed integration이다. 독립 장비와 프로토콜 경계가 생기는 만큼 운영/보안/재시도 설계가 커진다.

API gateway, service mesh, ESB는 이번 문제에 과하다. booth 한 대와 로컬 renderer 또는 소수의 edge node를 다루는 구조에서 중앙 gateway나 mesh를 넣는 것은 얻는 것보다 잃는 것이 크다. 반면 **strangler-style interoperability**는 적합하다. 즉, 기존 preview close 경로를 유지한 채 새 renderer lane을 동일 manifest 계약 아래 추가하고, 성능이 입증되면 점진적으로 canonical close owner를 옮기는 방식이다. 이는 Microsoft가 설명하는 Strangler Fig 패턴과도 잘 맞는다.

_Point-to-Point Integration:_ local dedicated renderer에 최적  
_API Gateway Patterns:_ 현재 범위에서는 불필요  
_Service Mesh:_ booth/edge 규모에서는 과도함  
_Enterprise Service Bus:_ 전통 엔터프라이즈 통합에는 유효하지만 Boothy 문제와 거리가 멂  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig, https://v2.tauri.app/ko/develop/sidecar/, https://helpx.adobe.com/lightroom-classic/help/import-photos-automatically.html

### Microservices Integration Patterns

Boothy에 가장 필요한 microservices 패턴은 사실 대규모 분산 시스템 패턴이 아니라 **bounded local service patterns**다. 첫째, **single-writer ownership**이 중요하다. 현재 계약대로 camera helper는 capture boundary를, render worker는 preview/final close를 각각 소유해야 한다. 둘째, **queue + backpressure**가 필요하다. resident worker가 saturation 상태에 들어가면 false-ready 없이 `Preview Waiting`으로 내려가야 한다는 현재 render-worker 계약은 매우 타당하다. 셋째, **circuit breaker / fallback**이 필요하다. dedicated renderer warm state가 깨지면 canonical truthful path로 자동 강등되어야 한다.

Saga 패턴은 edge appliance 또는 다단계 처리에 부분적으로 유효하다. `capture accepted -> raw arrived -> fast visible -> preset-applied visible -> final ready`는 결국 장기 트랜잭션에 가깝다. 하지만 분산 보상 트랜잭션 자체가 목표는 아니다. 핵심은 단계별 truth를 명확히 나누는 것이다. 따라서 Boothy에는 교과서적 microservices보다 `local saga with explicit states`가 더 맞다.

_API Gateway Pattern:_ edge appliance fleet가 커지지 않는 한 불필요  
_Service Discovery:_ edge appliance 다수화 전까지 불필요  
_Circuit Breaker Pattern:_ 필수. warm-state loss, queue saturation, invalid output 시 truthful fallback 필요  
_Saga Pattern:_ capture부터 preview/final까지의 명시적 단계 관리에 유효  
_Source:_ https://grpc.io/about/, https://learn.microsoft.com/en-us/azure/iot-edge/about-iot-edge, https://learn.microsoft.com/en-us/azure/iot-edge/offline-capabilities

### Event-Driven Integration

현재 문제는 event-driven 설계와 매우 잘 맞는다. `request-capture`는 command, `file-arrived`는 event, `previewReady`는 truth-closing event, `recent-session-visible`은 UI projection event로 분리하는 방식이 가장 자연스럽다. 이 분리는 same-capture guarantee를 지키는 데도 유리하다. 왜냐하면 UI는 file-arrived만으로 close를 선언하지 않고, renderer가 실제 preset-applied artifact를 생성했을 때만 close를 올릴 수 있기 때문이다.

중요한 점은 외부 broker가 꼭 필요하지 않다는 것이다. booth 단일 PC 또는 한 대의 edge appliance까지는 local event bus와 durable manifest만으로 충분하다. 다만 `watch-folder bridge`는 이벤트 대신 filesystem polling과 watcher callback에 의존하게 되므로 event semantics가 약해진다. Microsoft 문서가 watcher overflow와 blanket notification 가능성을 명시하는 만큼, 이 후보는 event-driven 정확성 면에서도 불리하다.

_Publish-Subscribe Patterns:_ 내부 이벤트 fan-out에는 유효하지만 외부 broker는 당장 불필요  
_Event Sourcing:_ 전체 도입은 과하지만 capture/render 진단 로그에는 유용  
_Message Broker Patterns:_ Kafka/RabbitMQ는 규모 대비 과함  
_CQRS Patterns:_ command(capture/request)와 query(manifest/session rail) 분리는 유효  
_Source:_ https://learn.microsoft.com/en-us/dotnet/fundamentals/runtime-libraries/system-io-filesystemwatcher, https://v2.tauri.app/ko/develop/sidecar/, https://grpc.io/docs/what-is-grpc/core-concepts/

### Integration Security Patterns

보안 패턴은 local-first일수록 오히려 단순해야 한다. local dedicated renderer는 별도 네트워크 포트를 열지 않고 host-owned child process로 묶는 편이 안전하다. Tauri sidecar 번들 방식은 이 점에서 유리하다. 또한 Boothy의 도메인 특성상 가장 중요한 보안은 OAuth보다 **artifact integrity와 boundary integrity**다. 즉, `captureId/sessionId/requestId` correlation, allowed-path 검증, preset bundle version pin, wrong-session/wrong-capture 차단이 실제 보안이자 correctness다.

named pipe는 보안 검사 대상이므로 로컬 서비스화할 때 ACL 제어를 넣을 수 있다. gRPC는 TLS와 optional mutual authentication을 공식 지원하므로 edge appliance 경로에서는 mTLS가 자연스럽다. 반면 watch-folder bridge는 파일 시스템 권한과 폴더 감시 정책에 크게 의존하므로, path spoofing과 stale file pickup을 막기 위한 별도 방어가 더 많이 필요하다.

_OAuth 2.0 and JWT:_ booth 단일 장비 hot path에는 우선순위 낮음  
_API Key Management:_ edge appliance 원격 호출 시에만 필요 가능  
_Mutual TLS:_ edge appliance gRPC에서는 권장  
_Data Encryption:_ 로컬 단일 장비보다 artifact path validation과 bounded exposure가 더 중요  
_Source:_ https://v2.tauri.app/ko/develop/sidecar/, https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipes, https://grpc.io/docs/guides/auth/

## Architectural Patterns and Design

### System Architecture Patterns

Boothy의 현재 문제는 전형적인 monolith 대 microservices 선택 문제가 아니다. 더 정확히는 `capture success`, `first-visible`, `preset-applied truthful close`, `final export truth`를 같은 단계로 묶어 버린 현재 경로를 분리해야 한다는 문제다. Azure Architecture Center는 아키텍처 스타일이 특정 제약을 부여해 원하는 속성을 만든다고 설명한다. 이 관점에서 Boothy에 필요한 제약은 명확하다. `camera boundary`는 capture truth만 소유하고, `renderer boundary`는 preset-applied truth만 소유하며, UI는 이를 소비만 해야 한다.

이 제약을 가장 잘 만족하는 기본 패턴은 **local modular monolith + dedicated render sidecar**다. 이는 대규모 microservices처럼 복잡한 분산 운영을 강요하지 않으면서도, hot path를 별도 프로세스로 격리하고, preset-applied close owner를 분명히 만든다. Microsoft의 microservices 가이드도 너무 잘게 나뉜 서비스는 복잡도와 성능 비용을 만든다고 경고한다. Boothy 규모에서는 서비스 수를 늘리는 것이 아니라, 병목 구간 하나를 분리하는 것이 맞다.

후보별 시스템 패턴 적합도는 다음과 같다.

- **local dedicated renderer**: 가장 좋은 정합성. booth PC 안에서 capture helper, host, resident renderer를 나누고, same-capture artifact를 local filesystem과 local IPC로 닫는다.
- **different close topology**: 거의 필수 보완 패턴. 어떤 엔진을 쓰더라도 `original visible`과 `preset-applied truthful close`를 별도 상태로 분리해야 한다.
- **edge appliance**: 독립 서비스 아키텍처. local PC 자원이 구조적으로 부족할 때 유효하나, 기본값으로는 과하다.
- **watch-folder bridge**: 외부 상용 엔진과의 interoperability façade. core architecture보다는 migration/experiment path에 가깝다.
- **lighter truthful renderer**: 독자 low-latency 엔진 패턴. 성공하면 가장 빠를 수 있지만, truthful contract 재구축 난도가 높다.

_Source:_ https://learn.microsoft.com/en-gb/azure/architecture/guide/architecture-styles/, https://learn.microsoft.com/en-us/azure/architecture/microservices/, https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig

### Design Principles and Best Practices

이번 결정에서 중요한 설계 원칙은 다섯 가지다. 첫째, **single-writer truth**다. capture truth와 render truth를 같은 코드 경로가 함께 닫지 말아야 한다. 둘째, **explicit dual-truth topology**다. Capture One이 `Immediately`와 `When ready`를 분리하듯, Boothy도 `same-capture first-visible`과 `preset-applied truthful visible`을 별도 제품 상태로 분리해야 한다. 셋째, **artifact-based truth**다. preview/final truth는 실제 artifact 파일 존재와 correlation으로만 닫혀야 한다. 넷째, **incremental migration**이다. Strangler Fig 패턴처럼 기존 경로를 남긴 채 새 renderer lane을 점진 치환해야 한다. 다섯째, **bounded fallback**이다. warm-state loss, queue saturation, invalid output 시 거짓으로 빠르게 보이는 대신 truthful waiting으로 내려가야 한다.

이 원칙 아래에서 `different close topology`는 후보가 아니라 사실상 모든 후보 위에 올라가는 설계 원칙이다. 즉, 가장 추천되는 조합은 `local dedicated renderer`와 `different close topology`를 같이 채택하는 형태다. 반대로 `lighter truthful renderer`는 원칙을 지키기 어렵다. fast path를 만들기는 쉬워도, darktable/XMP 기반 truth source와의 fidelity 계약을 유지하는 것이 어렵기 때문이다. 이 평가는 일부 구현 세부가 비공개인 상용 제품과 달리, Boothy가 현재 darktable truth path를 이미 계약으로 잡고 있다는 점에 근거한 구조적 판단이다.

_Source:_ https://support.captureone.com/hc/en-us/articles/360002556797-Selecting-the-appropriate-capture-preview, https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig, https://docs.darktable.org/usermanual/development/en/overview/sidecar-files/sidecar/

### Scalability and Performance Patterns

성능 관점에서 핵심은 “얼마나 많은 이미지를 동시에 처리할 수 있나”보다 **한 장의 same-capture preset-applied preview를 얼마나 빨리 닫을 수 있나**다. Azure의 Cache-Aside 패턴 문서는 캐시가 성능을 높일 수 있지만 일관성 전략이 필요하다고 설명한다. 이 원칙을 Boothy에 옮기면, preview warm cache와 preset preload는 유효하지만, truth source보다 앞서는 캐시를 canonical close로 승격하면 안 된다. 즉, cache는 시간을 줄이는 보조 수단이고, truth close owner는 여전히 renderer여야 한다.

후보별로 제거하는 병목은 다르다.

- **local dedicated renderer**는 per-capture process spawn, cold OpenCL init, preset load, queue handoff를 줄인다.
- **different close topology**는 UI가 full render close를 기다리면서 체감과 실제 close가 뒤엉키는 병목을 분리한다.
- **edge appliance**는 booth PC의 CPU/GPU contention을 다른 장비로 밀어낸다.
- **watch-folder bridge**는 renderer 자체를 바꿔 병목을 우회하려 하지만, file detection과 external catalog latency를 새로 들여온다.
- **lighter truthful renderer**는 compute weight 자체를 줄이려 하지만, truthfulness 재현 병목이 새로 생긴다.

실장비 성공 확률 기준으로는 `resident local renderer + warm state + cache priming + dual close topology` 조합이 가장 현실적이다. 엔진을 바꾸지 않고도 병목 대부분을 직접 겨냥하고, 네트워크와 외부 앱 의존성을 늘리지 않기 때문이다.

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/cache-aside, https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/, https://docs.nvidia.com/holoscan/sdk-user-guide/gpu_resident.html

### Integration and Communication Patterns

아키텍처 차원에서 가장 중요한 통신 패턴은 `command -> event -> truth-close event`의 분리다. Azure의 event-driven architecture 문서는 producer와 consumer를 느슨하게 결합하면서 near real-time 반응을 가능하게 하는 구조를 설명한다. Boothy에 이를 적용하면, `capture request`는 command, `file-arrived`는 ingestion event, `preset-preview-ready`는 canonical close event, `recent-session-visible`은 projection event가 된다.

이 구조는 `local dedicated renderer`와 `edge appliance` 둘 다에 적용 가능하지만, 전자는 local IPC로, 후자는 gRPC 또는 유사 RPC로 구현된다. `watch-folder bridge`는 이 체인에서 `file-arrived -> external engine ingest -> watched destination detect`로 바뀌기 때문에, close ownership이 약해지고 timing variance가 커진다. 따라서 아키텍처 관점에서도 bridge는 1차 코어 구조보다는 migration adapter에 가깝다.

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/event-driven, https://grpc.io/docs/what-is-grpc/core-concepts/, https://helpx.adobe.com/lightroom-classic/help/import-photos-automatically.html

### Security Architecture Patterns

이번 문제에서 보안은 traditional web auth보다 **correctness-preserving isolation**에 가깝다. 잘못된 캡처, 잘못된 세션, 잘못된 preset version이 섞이지 않는 것이 가장 중요하다. 그러므로 가장 좋은 구조는 경계 수가 적고, ownership이 명확하며, network attack surface가 작은 구조다. 이 기준에서는 local dedicated renderer가 가장 낫다. child process 또는 local service로 제한하고, allowed paths, version pin, correlation IDs만 강하게 검증하면 된다.

edge appliance는 mTLS, device identity, retry/idempotency, network partition 처리가 필요하므로 운영 보안과 correctness 모두 복잡해진다. watch-folder bridge는 외부 앱 폴더, moved file semantics, stale pickup을 막아야 하므로 파일 시스템 차원의 방어가 더 많이 필요하다. 즉, security/correctness 차원에서도 local-first가 우세하다.

_Source:_ https://grpc.io/docs/guides/auth/, https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipes, https://learn.microsoft.com/en-us/azure/iot-edge/offline-capabilities

### Data Architecture Patterns

데이터 아키텍처는 `write model = capture/render truth`, `read model = UI projection`으로 나누는 것이 맞다. Azure의 CQRS 패턴 문서는 read와 write를 분리하면 각각을 독립 최적화할 수 있다고 설명한다. Boothy에선 이 분리가 특히 중요하다. write model은 capture correlation, preset bundle pin, preview/final artifact existence를 엄격히 관리해야 하고, read model은 최근 세션 rail과 화면 상태만 빠르게 읽으면 된다.

이 관점에서 `different close topology`는 사실상 CQRS를 제품 상태에 적용한 것이다. `fastPreviewVisibleAtMs`와 `xmpPreviewReadyAtMs`를 분리해 기록하는 현재 session-manifest 방향은 매우 타당하다. 이 구조가 있어야 `원본은 빨리 보였지만 preset-applied close는 아직`이라는 truth를 왜곡 없이 표현할 수 있다.

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/cqrs, https://docs.darktable.org/usermanual/development/en/overview/sidecar-files/sidecar/

### Deployment and Operations Architecture

배포/운영 관점에서 가장 좋은 기본값은 **single booth PC local-first deployment**다. Tauri host, Canon helper, dedicated renderer, preset bundle, local session storage가 한 장비 안에 있고, 필요 시에만 별도 appliance를 붙이는 방식이다. Azure IoT Edge와 NVIDIA Holoscan 자료는 edge 장치가 오프라인에서도 계속 동작할 수 있고, GPU-resident execution으로 overhead를 줄일 수 있음을 보여준다. 하지만 이는 “가능한 패턴”일 뿐, Boothy 1차 선택으로 바로 뛰어들 근거는 아니다.

운영 난이도와 rollback 용이성까지 고려하면, 추천되는 배포 진화 순서는 아래와 같다.

1. **현재 앱 유지 + local dedicated renderer lane 추가**
2. **same-capture / preset-applied dual close topology 정착**
3. **실장비에서 목표 미달 시 edge appliance 실험**
4. **watch-folder bridge는 외부 엔진 검증용으로만 제한**

이 순서는 Strangler Fig 패턴과도 맞고, 실패했을 때 기존 경로로 되돌리기 쉽다. 반대로 edge appliance를 먼저 택하면, 성능이 개선되지 않았을 때 무엇이 병목인지 분리해서 보기 어려워진다. 네트워크, 장비, RPC, 원격 큐까지 동시에 변수로 들어오기 때문이다.

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig, https://learn.microsoft.com/en-us/azure/iot-edge/about-iot-edge, https://learn.microsoft.com/en-us/azure/iot-edge/offline-capabilities, https://docs.nvidia.com/holoscan/sdk-user-guide/gpu_resident.html

## Implementation Approaches and Technology Adoption

### Technology Adoption Strategies

가장 안전한 도입 전략은 `big bang` 교체가 아니라 **Strangler Fig 방식의 점진 치환**이다. Microsoft는 Strangler Fig를 기존 시스템을 유지한 채 일부 기능을 새 구조로 점진 치환하는 패턴으로 설명한다. Boothy에선 이 패턴이 특히 적합하다. 기존 capture/render path를 그대로 둔 상태에서 `local dedicated renderer lane`만 새로 추가하고, 동일한 session manifest와 동일한 UI rail 아래서 close latency와 correctness를 비교할 수 있기 때문이다.

채택 우선순위는 `shadow lane -> limited booth canary -> default 승격` 순서가 적절하다. 이 방식은 실패 시 rollback이 쉽고, 새 경로가 정말로 `preset-applied close`를 줄였는지 기존 경로와 직접 비교할 수 있다. 반면 edge appliance를 먼저 채택하면 네트워크와 원격 장비 변수까지 동시에 들어와 병목 진단이 어려워진다. watch-folder bridge는 core adoption strategy가 아니라 외부 엔진 검증용 adapter로 제한하는 편이 안전하다.

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig

### Development Workflows and Tooling

개발 워크플로는 세 층으로 나누는 것이 맞다. 첫째, **계약 중심 개발**이다. `requestId/sessionId/captureId` correlation, allowed-path 검증, preset version pin, preview/final truth close 규칙을 contract test로 고정해야 한다. 둘째, **하드웨어 통합 검증**이다. 실제 Canon body와 booth PC에서 `capture -> raw arrived -> original visible -> preset-applied visible`를 end-to-end로 재현해야 한다. 셋째, **UI trace 기반 회귀 추적**이다. Playwright는 trace viewer와 retries를 공식 지원하므로, close 시점의 UI 상태를 재생 가능한 증거로 남기기에 적합하다.

관측 계층은 OpenTelemetry를 쓰는 편이 가장 현실적이다. OpenTelemetry는 traces, metrics, logs를 표준화된 방식으로 수집하는 프레임워크이며 .NET에서도 stable signal을 제공한다. Boothy에선 host/helper/renderer 전체에 동일한 correlation ID를 흘려 보내 `capture accepted`, `file arrived`, `fast visible`, `preset-applied ready`, `recent-session-visible`을 하나의 trace로 묶어야 한다.

_Source:_ https://playwright.dev/docs/trace-viewer, https://playwright.dev/docs/test-retries, https://opentelemetry.io/docs/, https://opentelemetry.io/docs/languages/net/

### Testing and Quality Assurance

이번 의사결정에서 테스트의 목적은 기능 확인이 아니라 **성능과 정확성을 동시에 보증하는 것**이다. Microsoft Well-Architected 운영 우수성 가이드는 production-like testing, observability, safe deployment를 강조한다. Boothy에는 아래 네 층 테스트가 필요하다.

- `unit/contract`: manifest timing field, renderer protocol, fallback state machine
- `integration`: helper -> host -> renderer -> manifest truth close
- `e2e`: 실제 촬영 버튼부터 recent session rail 교체까지
- `hardware-in-loop performance`: 실장비 기준 `original visible -> preset-applied visible`

특히 성공 기준은 평균값이 아니라 `p50/p95`, cold/warm 분리, preset별 worst-case, saturation rate를 함께 보는 것이다. 목표 SLA가 2.5초라면 warm-only 평균으로 통과시키면 안 된다. `wrong-capture 0`, `preset mismatch 0`, `preview/final drift 0`이 성능 KPI와 같은 급의 품질 KPI가 되어야 한다.

_Source:_ https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/, https://learn.microsoft.com/en-us/training/modules/azure-well-architected-operational-excellence/

### Deployment and Operations Practices

운영 방식은 `safe deployment + strong observability + fast truthful fallback`이 핵심이다. 새 renderer lane은 feature flag로 감싸고, 1개 booth 또는 1개 preset 그룹에만 먼저 적용해야 한다. renderer warm-state loss, queue saturation, invalid output, fallback rate를 별도 운영 지표로 잡고, 목표 미달 또는 mismatch 발생 시 즉시 기존 truthful path로 강등할 수 있어야 한다.

중요한 점은 “실패해도 빨라 보이게”가 아니라 “실패하면 정직하게 waiting으로 내려가는 것”이다. 이는 현재 render worker 계약과도 일치한다. 운영 관점에서는 false-ready를 줄이는 것이, 잘못된 preview를 빠르게 보여주는 것보다 더 중요하다.

_Source:_ https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/, https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig

### Team Organization and Skills

필요한 역량은 대규모 조직이 아니라 **경계별 책임이 분명한 소규모 실행 팀**이다. 한 축은 camera/helper/host correlation, 한 축은 renderer/native/OpenCL/warm-state, 한 축은 UI projection/session rail/fallback, 마지막 한 축은 hardware validation/measurement를 맡는 구성이 적절하다. 기술적으로는 darktable/XMP artifact 이해, Windows sidecar 운영, OpenTelemetry 계측, 실장비 benchmark 운영 역량이 필요하다.

`lighter truthful renderer`를 1순위로 선택하면 팀 요구 역량이 급격히 올라간다. RAW color pipeline, preset fidelity 재현, preview/final equivalence 검증까지 별도 전문성이 더 필요하기 때문이다. 따라서 현재 팀 리스크까지 감안하면 dedicated local renderer가 더 현실적이다.

_Source:_ https://docs.darktable.org/usermanual/development/en/overview/sidecar-files/sidecar/, https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/, https://opentelemetry.io/docs/

### Cost Optimization and Resource Management

비용과 자원 효율 관점의 1순위도 local dedicated renderer다. 기존 booth PC와 앱 구조를 유지하면서 hot path만 바꾸기 때문이다. edge appliance는 성능 상한을 높일 수 있지만 booth당 추가 장비, 현장 관리, 장애 대응 비용이 붙는다. watch-folder bridge는 개발 착수 비용이 낮아 보일 수 있지만 외부 앱 의존, 파일 감시 불안정성, correlation 보완 로직 때문에 숨은 운영 비용이 커질 수 있다. lighter truthful renderer는 런타임 비용은 낮출 수 있어도 연구개발 비용과 검증 비용이 가장 크다.

즉, “실행 비용 대비 성공 확률” 기준으로는 dedicated local renderer가 가장 유리하다. 이는 단순히 싸기 때문이 아니라, 기존 truth source와 계약을 유지한 채 병목을 직접 제거할 수 있기 때문이다.

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig, https://learn.microsoft.com/en-us/azure/iot-edge/about-iot-edge, https://learn.microsoft.com/en-us/azure/iot-edge/offline-capabilities

### Risk Assessment and Mitigation

후보별 리스크와 완화책은 다음처럼 정리된다.

- **local dedicated renderer**
  - 리스크: warm-state 유지 실패, OpenCL variance, queue saturation
  - 완화: resident worker, preset preload, cache priming, truthful fallback waiting
- **different close topology**
  - 리스크: UI/state complexity 증가
  - 완화: manifest timing field 분리, single-writer truth 고수
- **edge appliance**
  - 리스크: 네트워크 variance, 운영 복잡도, 원격 장비 장애
  - 완화: local fallback lane 유지, gRPC health check, offline-first, mTLS
- **watch-folder bridge**
  - 리스크: file-ready ambiguity, stale pickup, wrong-capture correlation 약화
  - 완화: file completion polling, quarantine folder, dedupe, strict import validation
- **lighter truthful renderer**
  - 리스크: preset fidelity와 preview/final truth 계약 붕괴 가능성
  - 완화: darktable diff-validation, preset subset 제한, 진실값 역할 축소

결론적으로, 리스크 대비 완화 가능성까지 포함하면 `local dedicated renderer + different close topology`가 가장 방어 가능한 조합이다.

_Source:_ https://learn.microsoft.com/en-us/dotnet/fundamentals/runtime-libraries/system-io-filesystemwatcher, https://grpc.io/docs/guides/auth/, https://docs.darktable.org/usermanual/development/en/overview/sidecar-files/sidecar/

## Technical Research Recommendations

### Implementation Roadmap

1. `different close topology`를 제품 상태와 계측에 먼저 확정
2. `local dedicated renderer` 프로토타입을 sidecar lane으로 추가
3. warm-up, preset preload, cache priming 도입
4. 실장비에서 p50/p95 및 mismatch 0 기준으로 측정
5. 목표 미달 시에만 `edge appliance` 실험
6. `watch-folder bridge`는 외부 엔진 비교 실험으로만 제한

### Technology Stack Recommendations

- 기본 권장: `Tauri host + Canon helper sidecar + resident local renderer + darktable truth path + dual-close session manifest`
- 2차안: `local host + remote edge renderer + gRPC + local truthful fallback lane`
- 비권장 기본안: watch-folder 중심 코어 구조, 독자 lightweight truthful renderer 선행 개발

### Skill Development Requirements

- darktable/XMP artifact 이해
- Windows native sidecar 운영
- OpenTelemetry 기반 end-to-end trace 계측
- hardware-in-loop benchmark 운영

### Success Metrics and KPIs

- 핵심 KPI: `original visible -> preset-applied visible <= 2.5s`
- 필수 품질 KPI:
  - wrong-capture 0
  - preset mismatch 0
  - preview/final truth drift 0
  - warm/cold p95 분리 기록
  - renderer saturation rate
  - fallback rate
  - booth session failure rate

## Research Synthesis

# Boothy Preset-Applied Preview Decision: Comprehensive Technical Research

## Executive Summary

이번 리서치의 결론은 명확하다. Boothy가 실장비에서 `original visible -> preset-applied visible <= 2.5s`에 가장 가까이 갈 수 있는 1순위 구조는 **`local dedicated renderer + different close topology`**다. 이 조합은 현재 병목인 `preset-applied preview close`를 직접 겨냥하면서도, `same-capture 보장`, `preset fidelity`, `preview truth = final truth` 계약을 가장 적게 흔든다. 핵심은 원본 first-visible과 preset-applied truthful close를 분리하고, preset-applied close owner를 로컬 상주 렌더러로 명확히 만드는 것이다.

다른 후보들도 의미는 있다. `edge appliance`는 로컬 자원이 구조적으로 부족할 때의 강한 2차 카드이고, `watch-folder bridge`는 외부 엔진 비교 실험이나 이행 어댑터로는 유효하다. 그러나 1차 코어 구조로 보기에는 각각 운영 복잡도와 loose coupling 리스크가 크다. `lighter truthful renderer`는 성공 시 잠재력이 크지만, 현재 Boothy의 darktable/XMP 기반 truth source 계약과 fidelity 보장을 다시 입증해야 하므로, 실장비 목표 달성 확률보다 연구 리스크가 더 크다.

이 문서의 최종 권고는 제품 관점에서도 동일하다. 다음으로 바로 착수할 구조는 `local dedicated renderer + different close topology`다. `edge appliance`는 2순위 보류 실험으로 남기고, `watch-folder bridge`를 코어 구조로 채택하는 안과 `lighter truthful renderer` 선행 투자는 지금 시점에서는 버리는 것이 맞다.

**Key Technical Findings:**

- 상용 제품도 이미 `Immediately`와 `When ready`처럼 preview policy를 분리한다.
- darktable XMP sidecar와 `darktable-cli`는 truth-preserving preset apply path를 제공한다.
- local-first resident renderer는 cold start, preset load, warm-state loss 같은 직접 병목을 겨냥할 수 있다.
- watch-folder 기반 통합은 실제로 존재하지만 file-ready ambiguity와 correlation 약점이 크다.
- edge appliance는 가능하지만 첫 선택으로는 변수와 운영 부담이 크다.

**Technical Recommendations:**

- `local dedicated renderer + different close topology`를 바로 착수
- resident worker warm-up, preset preload, cache priming을 설계 기본값으로 채택
- same-capture / preset fidelity / preview-final truth drift를 성능 KPI와 같은 급으로 관리
- 실장비 p50/p95 중심의 hardware-in-loop 검증을 먼저 수행
- 목표 미달 시에만 edge appliance를 2차 실험으로 전환

## Table of Contents

1. Technical Research Introduction and Methodology
2. Candidate Comparison and Final Ranking
3. Recommended Architecture: Local Dedicated Renderer
4. Why Other Candidates Ranked Lower
5. Validation Plan and Success Criteria
6. Final Recommendation
7. Technical Research Methodology and Source Verification

## 1. Technical Research Introduction and Methodology

### Technical Research Significance

Boothy의 문제는 단순한 썸네일 속도 개선이 아니라 제품 신뢰와 직결된 preview close 문제다. 사용자가 실제로 기다리는 것은 "무언가 보인다"가 아니라 "선택한 프리셋이 적용된 같은 사진이 닫힌다"는 순간이다. Capture One이 tethered capture에서 `Immediately`와 `When ready`를 별도 정책으로 노출하는 것도 이 문제를 제품 차원에서 분리해서 다루기 때문이다. Lightroom Classic 역시 tethered import, auto-import, preview rendering, watched folder를 별도 기능으로 나누어 제공한다. 이는 preview pipeline이 단일 동작이 아니라 복수의 truth와 latency trade-off로 구성된다는 뜻이다.

Boothy는 이미 darktable/XMP 기반 preset artifact와 session manifest 기반 truth 모델을 채택하고 있으므로, 이번 결정은 "새 기능 추가"보다 "제품 핵심 성능 경계 재설계"에 가깝다. 따라서 리서치는 미세 튜닝이나 마케팅 지표가 아니라, 병목 제거 구조와 운영 가능성을 중심으로 수행했다.

_Technical Importance:_ preset-applied preview close는 제품 체감과 correctness가 동시에 걸린 종단간 아키텍처 문제다.  
_Business Impact:_ 같은 캡처, 같은 프리셋, 빠른 close가 안정적으로 보장되어야 포토부스 경험과 운영 신뢰를 동시에 유지할 수 있다.  
_Source:_ https://support.captureone.com/hc/en-us/articles/360002556797-Selecting-the-appropriate-capture-preview, https://helpx.adobe.com/lt/lightroom-classic/help/import-photos-tethered-camera.html

### Technical Research Methodology

- **Technical Scope**: local renderer, watch-folder interoperability, edge appliance, lightweight truthful rendering, close topology
- **Data Sources**: Canon SDK/CCAPI, darktable, LibRaw, RawSpeed, Tauri, Microsoft Learn, Adobe Lightroom Classic, Capture One, Azure, NVIDIA, OpenTelemetry, Playwright
- **Analysis Framework**: 병목 직접 제거 능력, same-capture 보장, preset fidelity, preview/final truth 계약, 운영 난이도, rollback 용이성
- **Time Period**: 2026-04-09 기준 최신 공개 자료
- **Technical Depth**: 제품 의사결정이 가능할 수준의 구조/운영/검증 관점 분석

### Technical Research Goals and Objectives

**Original Technical Goals:** Choose the architecture with the highest probability of achieving original-visible to preset-applied-visible <= 2.5 seconds on real booth hardware, without breaking same-capture guarantees, preset fidelity, or preview/final truth contracts.

**Achieved Technical Objectives:**

- 구조 후보 5개를 제품 KPI와 truth 계약 기준으로 비교
- 각 후보가 제거하는 직접 병목을 분리 설명
- 운영 난이도와 rollback 가능성을 포함한 우선순위 도출
- 바로 실행 가능한 최소 프로토타입과 측정 기준 제안

## 2. Candidate Comparison and Final Ranking

### One-Line Conclusion

`local dedicated renderer + different close topology`를 바로 착수 1순위로 추천한다.

### Candidate Comparison Table

| 후보명 | 왜 빨라질 수 있는지 | 무엇을 바꿔야 하는지 | 기대 성능 | 리스크 | 추천 순위 |
|---|---|---|---|---|---|
| local dedicated renderer | per-capture spawn, cold init, preset load, queue handoff를 줄이고 warm state 유지 | 로컬 상주 render sidecar, preload/warm-up, same-path truthful close | 가장 높음 | OpenCL variance, saturation | 1 |
| different close topology | 원본 first-visible과 preset-applied truthful close를 분리해 실제 병목만 직접 추적 | manifest timing, UI 상태, projection, 계측 재설계 | 필수 보완 효과 | 상태 복잡도 증가 | 1과 세트 |
| edge appliance | booth PC의 CPU/GPU contention을 분리 장비로 이동 | 원격 renderer, RPC, 장비 운영, health/fallback | 조건부 높음 | 네트워크/운영 부담 | 2 |
| watch-folder bridge | 외부 상용 엔진을 빠르게 붙여 우회 가능 | file drop/import quarantine/watcher/polling | 중간 | same-capture/correlation 약화 | 3 |
| lighter truthful renderer | 연산량 자체를 줄일 수 있음 | 자체 truthful preset engine과 diff-validation | 성공 시 높음 | fidelity/truth 재입증 | 4 |

### Why the Top Choice Wins

1순위는 단순히 빨라 보이는 방법이 아니라, **현재 병목을 가장 직접적으로 제거하면서 truth 계약을 가장 적게 손상시키는 방법**이기 때문이다. local dedicated renderer는 기존 darktable truth source를 유지한 채 warm-state, preset preload, same-capture canonical close를 최적화할 수 있다. different close topology는 어떤 엔진을 쓰더라도 필수다. 이 두 개를 함께 채택해야만 제품 KPI와 내부 truth를 동시에 맞출 수 있다.

## 3. Recommended Architecture: Local Dedicated Renderer

### Structure in Plain Language

지금 앱을 버리지 않고, 사진이 도착하면 그 캡처와 선택된 프리셋만 빠르게 처리하는 **전용 로컬 렌더러**를 하나 붙이는 방식이다. 사용자는 먼저 같은 캡처의 원본을 보고, 그 뒤 같은 위치에서 프리셋 적용본으로 빠르게 교체되는 경험을 하게 된다. 중요한 것은 이 교체가 "추정"이나 "비슷한 그림"이 아니라, 실제 preset-applied truthful artifact가 생겼을 때만 닫힌다는 점이다.

### Data Flow / Processing Flow

1. booth가 `request-capture`를 보낸다.  
2. Canon helper가 RAW를 session-scoped path에 전달하고 `file-arrived`를 보낸다.  
3. UI는 same-capture 원본 first-visible을 표시할 수 있다.  
4. host가 `captureId + presetVersion + rawPath`를 local dedicated renderer에 전달한다.  
5. resident renderer가 warm 상태에서 preset-applied preview를 생성한다.  
6. 실제 preview artifact가 canonical preview path에 생기면 그 시점에만 `previewReady`와 `xmpPreviewReadyAtMs`를 닫는다.  
7. UI는 same capture의 원본 표시를 preset-applied truthful preview로 교체한다.

### Why It Directly Solves the Current Bottleneck

현재 느린 것은 "무언가 보여주는 일"이 아니라 `preset-applied preview close`다. local dedicated renderer는 이 close를 늦추는 직접 병목들, 즉 per-capture process start, OpenCL cold init, preset artifact load, 느슨한 queue handoff를 줄일 수 있다. 동시에 same-capture correlation을 로컬 IPC와 같은 session filesystem 안에서 유지하므로 correctness에도 유리하다. 즉, 빠르면서도 truth를 잃지 않는 경로다.

### Failure Conditions

- warm state가 유지되지 않을 때
- GPU/OpenCL 초기화 편차가 클 때
- render queue가 포화될 때
- 특정 preset 조합이 preview lane에서 과하게 무거울 때

이 경우에도 기존 truthful waiting path로 즉시 강등할 수 있어 rollback이 쉽다.

## 4. Why Other Candidates Ranked Lower

### 2nd Place: Edge Appliance

edge appliance는 로컬 장비 자원이 구조적으로 부족할 때 강한 카드다. 원격 GPU 또는 별도 장비로 render contention을 분리할 수 있기 때문이다. 하지만 1차 선택으로는 변수와 운영 리스크가 너무 많다. 네트워크 hop, 원격 장비 health, RPC, 배포, 현장 장애 복구까지 동시에 설계해야 하므로, 성능이 안 나왔을 때 진짜 병목이 renderer인지 네트워크인지 분리해서 보기 어렵다. 그래서 2순위다.

### 3rd Place: Watch-Folder Bridge

watch-folder bridge는 빠른 외부 엔진 비교 실험에는 유용하지만, 코어 구조로는 불리하다. Lightroom Classic Auto Import 문서가 보여주듯 watched folder는 비어 있어야 하고, subfolder를 감시하지 않으며, 감지 후 파일을 destination으로 이동한다. Microsoft `FileSystemWatcher` 역시 overflow, multiple events, completion ambiguity를 문서화한다. 즉, 이 구조는 본질적으로 loose coupling이다. same-capture guarantee와 canonical close ownership을 Boothy가 직접 잡기 어려워진다.

### Why Lighter Truthful Renderer Was Pushed Down

독자 경량 truthful renderer는 이론적으로 가장 빠를 수도 있다. 하지만 지금 Boothy의 핵심은 속도만이 아니라 darktable/XMP 기반 preset truth source와 preview/final truth 계약을 유지하는 것이다. 이 후보는 속도 개선과 동시에 truth-preserving equivalence를 새로 증명해야 하므로, 성공 확률보다 연구 리스크가 더 크다.

## 5. Validation Plan and Success Criteria

### Smallest Viable Prototype

가장 작은 프로토타입은 기존 앱을 유지한 채, `captureId + presetVersion + rawPath`를 받아 canonical preview path에 same-capture preset-applied artifact를 생성하는 resident local renderer lane 하나를 추가하는 것이다. UI 전체를 바꾸거나 전체 export path를 다시 쓰지 않아도 된다.

### What Must Be Measured

- `raw arrived -> original visible`
- `original visible -> preset-applied visible`
- warm p50 / warm p95
- cold p50 / cold p95
- wrong-capture count
- preset mismatch count
- preview/final truth drift count
- renderer saturation rate
- fallback rate

### Success Criteria

- 실장비에서 `original visible -> preset-applied visible <= 2.5s`
- wrong-capture 0
- preset mismatch 0
- preview/final truth drift 0
- warm p95 기준에서도 안정적인 close

### Failure Criteria

- SLA 미달이 반복될 때
- same-capture 보장이 흔들릴 때
- preset fidelity가 깨질 때
- fallback이 상시 발생해 구조적 개선이라 보기 어려울 때

## 6. Final Recommendation

### Next Structure to Start Immediately

`local dedicated renderer + different close topology`

### Structure to Hold in Reserve

`edge appliance`

### Structure to Avoid for Now

- `watch-folder bridge`를 코어 구조로 채택하는 안
- `lighter truthful renderer`를 선행 투자하는 안

### Product-Level Decision

이번 결정은 "작은 수정"이 아니라 "성공 확률이 높은 다음 구조"를 고르는 일이다. 그 기준에서 바로 착수해야 할 것은 로컬 전용 렌더러와 close topology 재정의다. 이 경로는 가장 현실적이고, 가장 local-first이며, 실패했을 때도 제품을 크게 흔들지 않고 되돌릴 수 있다.

## 7. Technical Research Methodology and Source Verification

### Primary Technical Sources

- Canon SDK / CCAPI: https://www.usa.canon.com/support/sdk
- darktable CLI / sidecar / OpenCL: https://docs.darktable.org/usermanual/development/en/special-topics/program-invocation/darktable-cli/, https://docs.darktable.org/usermanual/development/en/overview/sidecar-files/sidecar/, https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/
- LibRaw / RawSpeed: https://www.libraw.org/docs/API-overview.html, https://www.libraw.org/docs/API-C.html, https://github.com/darktable-org/rawspeed
- Capture One preview policy: https://support.captureone.com/hc/en-us/articles/360002556797-Selecting-the-appropriate-capture-preview
- Lightroom Classic watched folder / tethered capture: https://helpx.adobe.com/in/lightroom-classic/help/import-photos-automatically.html, https://helpx.adobe.com/lt/lightroom-classic/help/import-photos-tethered-camera.html
- Microsoft Learn: FileSystemWatcher, Strangler Fig, CQRS, Operational Excellence
- Azure IoT Edge / NVIDIA Holoscan / gRPC / OpenTelemetry / Playwright official docs

### Confidence Level

전체 결론의 신뢰도는 **높음**이다. Canon/camera SDK, darktable/XMP, Capture One preview policy, Lightroom watched-folder behavior, Microsoft watcher limitations 같은 핵심 전제는 공식 문서로 확인했다. 다만 상용 제품의 내부 렌더 엔진 구현 세부는 비공개이므로, 일부 비교는 제품 정책과 공개 동작을 바탕으로 한 추정이 포함된다. 그 추정은 최종 순위보다 세부 구현 상상에만 영향을 준다.

### Research Limitation

이 리서치는 최신 공개 자료와 현재 저장소 계약을 기반으로 수행되었다. 실제 booth 하드웨어 편차, 특정 preset 조합의 무게, GPU 드라이버 상태는 프로토타입 측정으로 최종 확정해야 한다. 따라서 최종 의사결정은 이미 가능하지만, 성능 수치 확정은 hardware-in-loop 검증이 필요하다.

---

## Technical Research Conclusion

Boothy의 다음 구조는 `local dedicated renderer + different close topology`가 맞다. 이것이 가장 local-first이고, same-capture와 truth 계약을 가장 안전하게 지키며, 목표 SLA에 도달할 확률이 가장 높다. `edge appliance`는 목표 미달 시의 2차 카드로 남기고, `watch-folder bridge`와 `lighter truthful renderer`는 코어 구조 우선순위에서 내리는 것이 맞다.

**Technical Research Completion Date:** 2026-04-09  
**Research Period:** current comprehensive technical analysis  
**Source Verification:** All major claims verified against current official or primary sources  
**Technical Confidence Level:** High
