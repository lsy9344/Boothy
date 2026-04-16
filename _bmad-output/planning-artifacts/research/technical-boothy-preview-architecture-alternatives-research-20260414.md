---
stepsCompleted: [1, 2, 3, 4, 5, 6]
inputDocuments:
  - "_bmad-output/planning-artifacts/preview-architecture-reassessment-report-20260414.md"
  - "_bmad-output/planning-artifacts/architecture.md"
  - "docs/contracts/local-dedicated-renderer.md"
workflowType: 'research'
lastStep: 6
research_type: 'technical'
research_topic: 'Boothy 촬영 후 same-capture preset-applied 결과물을 24인치 가로 풀화면 기준 2.5초 이내에 표시하기 위한 대체 preview architecture'
research_goals: '사용자가 촬영 직후 같은 사진의 프리셋 적용 결과물을 24인치 모니터 가로 풀화면으로 2.5초 안에 보게 하는 현실적인 기술/아키텍처 후보를 찾고, booth-safe waiting, same-capture 정합성, preset fidelity를 유지하는 도입 경로를 비교한다.'
user_name: 'Noah Lee'
date: '2026-04-14'
web_research_enabled: true
source_verification: true
---

# Research Report: technical

**Date:** 2026-04-14
**Author:** Noah Lee
**Research Type:** technical

---

## Research Overview

이번 리서치는 기존 `local dedicated renderer` 경로가 실제로 활성화된 뒤에도, 제품 합격선인 `same-capture preset-applied full-screen visible <= 2500ms`를 반복적으로 만족하지 못한다는 판단에서 시작되었다. 따라서 목적은 현 구조를 더 미세조정하는 것이 아니라, **촬영 직후 같은 사진의 프리셋 적용 결과를 24인치 가로 풀화면으로 2.5초 안에 보여줄 수 있는 대체 architecture를 찾는 것**으로 재정의되었다. 썸네일, RAW first-visible, 작은 close preview, recent strip 업데이트는 모두 성공으로 간주하지 않는다.

공개 기술 문서와 내부 계약/검증 자료를 함께 검토한 결과, 가장 설득력 있는 방향은 `local native/GPU resident full-screen lane + display-sized preset-applied truthful artifact`를 별도 경로로 두고, `same-capture 정합성`, `preset fidelity`, `booth-safe fallback`, `evidence completeness`를 유지하는 구조였다. 반대로, 현 경로의 추가 미세조정만으로 목표를 닫는 전략이나, 대규모 마이크로서비스 재구성, broker-first hot path는 제품 적합성이 낮았다.

아래 `Research Synthesis` 섹션에는 이 판단을 실행 의사결정용으로 다시 통합한 `Executive Summary`, 권장 아키텍처, 구현 로드맵, 리스크, source verification이 정리되어 있다.

---

## Technical Research Scope Confirmation

**Research Topic:** Boothy 촬영 후 same-capture preset-applied 결과물을 24인치 가로 풀화면 기준 2.5초 이내에 표시하기 위한 대체 preview architecture  
**Research Goals:** 사용자가 촬영 직후 같은 사진의 프리셋 적용 결과물을 24인치 모니터 가로 풀화면으로 2.5초 안에 보게 하는 현실적인 기술/아키텍처 후보를 찾고, booth-safe waiting, same-capture 정합성, preset fidelity를 유지하는 도입 경로를 비교한다.

**Technical Research Scope:**

- Architecture Analysis - full-screen preset-applied close를 줄일 수 있는 ownership 모델, dual-lane topology, render graph 경계
- Implementation Approaches - staged preview, cached truthful artifact, GPU resident worker, raw/decode/render 분리
- Technology Stack - 언어, 프레임워크, 디코더, GPU API, 캐시/스토리지, 로컬/엣지 배포 옵션
- Integration Patterns - current host/session truth를 유지하면서 대안 스택을 붙일 수 있는 방식
- Performance Considerations - `capture -> preset-applied full-screen visible <= 2500ms` 달성 가능성, warm-state 유지, fallback 부담

**Research Methodology:**

- 최신 공개 자료 기반 web verification
- 중요한 기술 주장에 대한 다중 출처 교차 확인
- 내부 문서와 외부 자료를 함께 사용한 product-fit 평가
- 불확실성은 inference로 명시

**Success Metric Clarification:**

- **정확한 제품 의미:** 성공은 same-capture preset-applied image가 24인치 모니터의 가로 풀화면을 채우는 상태로 2.5초 내에 보이는 것이다.
- **정확한 제외 항목:** tiny preview, first-visible raster, raw thumbnail, recent-strip 업데이트는 목표 달성으로 세지 않는다.
- **정확한 기술 방향:** full-resolution final export가 아니라도 되지만, full-screen에서는 "프리셋이 제대로 적용된 결과"로 받아들여질 만큼 truthful해야 한다.

**Scope Confirmed:** 2026-04-14

---

<!-- Content will be appended sequentially through research workflow steps -->

## Technology Stack Analysis

### Web Search Analysis

이번 단계에서는 네 개의 기술 축을 병렬로 조사했다.

- **Windows/native imaging stack:** WIC RAW guidelines, Direct2D custom effects, Windows-local imaging path
- **RAW/preset processing engines:** darktable OpenCL, LibRaw, RawTherapee, Adobe DNG preview/fast-load patterns
- **Caching and storage stack:** OpenImageIO ImageCache, embedded preview and prerendered preview patterns
- **Deployment/platform options:** Tauri sidecar, on-device Windows runtime, low-latency edge/on-prem deployment 패턴

조사 결과, full-screen preset-applied close를 빠르게 만드는 데 유효한 공통 방향은 크게 세 가지로 모였다.

1. **critical path를 JS가 아니라 native/GPU 경계에 둔다.**
2. **preview/save/final을 하나의 blocking path로 묶지 않고 분리한다.**
3. **cached preview 또는 resident worker를 통해 cold path를 줄인다.**

반대로, current sources만으로는 "darktable truth와 완전히 같은 룩을 Windows 전용 custom shader path 하나로 곧바로 대체할 수 있다"는 근거는 충분하지 않았다. 따라서 이번 단계의 기술 스택 결론은 **native Windows full-screen lane을 강화하되, fidelity oracle 또는 parity reference는 별도 진실 경로로 남기는 편이 더 현실적**이라는 쪽에 가깝다.

### Programming Languages

현재 목표는 UI 언어 선택이 아니라 **same-capture preset-applied full-screen artifact를 2.5초 안에 만드는 hot path**다. 그 관점에서 critical path는 여전히 네이티브 언어가 우세하다. Tauri는 외부 바이너리를 sidecar로 번들하고 Rust에서 직접 spawn/IPC 할 수 있게 지원하므로, 현재 Rust host를 유지하면서 별도 네이티브 렌더러를 붙이는 구조가 자연스럽다. 동시에 Windows Direct2D custom effects는 GPU에서 실행되는 image operation graph를 HLSL로 작성할 수 있게 제공한다. 이는 Windows 전용 full-screen render lane을 HLSL/Direct2D/Direct3D 기반으로 구성하는 선택지를 열어 둔다.

LibRaw는 한 `libraw_data_t`/processor 인스턴스가 동시에 하나의 소스만 처리하지만, 여러 인스턴스를 병렬로 돌릴 수 있다고 문서화한다. 이는 decode engine을 per-capture spawn보다 resident pool 또는 warm worker 쪽으로 설계해야 한다는 뜻이다. 요약하면, **Rust는 orchestration에 적합하고, decode/render hot path는 C/C++ 및 GPU shader 언어가 더 직접적**이다. TypeScript/React는 부스 shell에 유지해도 되지만, 목표 SLA를 닫을 주력 언어는 아니다.

_Popular Languages:_ Rust(host orchestration), C/C++(RAW decode, image libraries), HLSL/OpenCL/CUDA 계열(GPU kernels)  
_Emerging Languages:_ Rust-native imaging stack은 orchestration에는 강하지만, preset-fidelity가 필요한 RAW truth engine 대체재로는 아직 직접 증거가 약함  
_Language Evolution:_ product shell은 생산성 언어로 두고, latency-critical lane만 네이티브 worker로 빼는 방향이 더 설득력 있음  
_Performance Characteristics:_ decode/render worker는 resident or pooled 형태가 유리하고, per-capture 프로세스 생성 비용은 가능한 한 제거해야 함  
_Source:_ https://v2.tauri.app/ko/develop/sidecar/, https://www.libraw.org/docs/API-overview.html, https://learn.microsoft.com/en-us/windows/win32/direct2d/custom-effects

### Development Frameworks and Libraries

공개 자료 기준으로 가장 중요한 기술군은 네 가지다. 첫째, **truth-preserving RAW apply engine**이다. darktable는 OpenCL이 interactive work와 export 모두에서 속도 향상을 줄 수 있고, GPU 실패 시 CPU로 안전하게 fallback 한다고 설명한다. 즉, quality-preserving truth path로는 여전히 강력하다. 둘째, **fast/decode/cache layer**다. Windows WIC RAW guidelines는 빠른 thumbnail/preview를 위해 `GetThumbnail`/`GetPreview`와 prerendered preview cache를 권장하며, responsive UX를 위해 `200ms` 이하 반환을 매우 바람직한 목표로 제시한다. Adobe 쪽도 DNG에서 `Embed Fast Load Data`와 larger JPEG preview를 계속 제공한다. 이는 industry가 오래전부터 "**빠르게 보여주는 artifact**"를 별도로 다루고 있음을 뜻한다.

셋째, **tile/cache library**로는 OpenImageIO가 유력하다. OpenImageIO `ImageCache`는 스레드 안전하고, 필요한 tile만 읽고, 자동으로 열린 파일 수와 메모리를 제한하며, 수천 개의 큰 이미지를 적은 메모리로 다룰 수 있다고 문서화한다. full-screen display-sized artifact를 same-path replacement로 관리하려는 구조에는 잘 맞는다. 넷째, **pipeline 분리형 오픈소스 편집기 패턴**이다. RawTherapee 문서는 하나의 이미지가 화면에 표시될 때와 저장될 때가 서로 다른 predetermined pipeline을 지난다고 설명하고, preferences에서 preview와 save에 다른 demosaicing/denoise policy를 둘 수 있게 한다. 이는 preview/full-save 분리를 제품적으로 숨기더라도 기술적으로는 매우 일반적인 전략임을 보여준다.

_Major Frameworks:_ darktable, WIC RAW codec interfaces, OpenImageIO, Tauri sidecar, Windows Direct2D custom effects  
_Micro-frameworks:_ embedded preview cache, display-sized truthful preview artifact, tile cache, preset preload/warm worker  
_Evolution Trends:_ preview path와 full save path를 분리하고, cached preview 또는 fast-load artifact를 활용하는 쪽으로 수렴  
_Ecosystem Maturity:_ darktable/WIC/OpenImageIO/Tauri는 모두 성숙도가 높고 문서가 분명함  
_Source:_ https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/, https://learn.microsoft.com/en-us/windows/win32/wic/-wic-rawguidelines-thumbnail-previews, https://helpx.adobe.com/photoshop-elements/using/process-camera-raw-image-files.html, https://openimageio.readthedocs.io/en/stable/imagecache.html, https://rawpedia.rawtherapee.com/Toolchain_Pipeline

### Database and Storage Technologies

이번 문제의 핵심 저장소는 관계형 DB가 아니라 **file artifact + cache + manifest**다. Microsoft WIC는 preview를 빨리 보여주기 위해 **prerendered preview를 image file에 캐시하는 것을 강하게 권장**한다. Adobe DNG 쪽 역시 fast load data와 embedded preview를 유지한다. 이 조합은 "full-screen에 바로 올릴 수 있는 display-sized preset-applied artifact를 별도로 들고 가는 전략"이 기술적으로 정당하다는 뜻이다. 즉, 최종 full-resolution export와 별개로, booth UI가 바로 올릴 수 있는 truthful display artifact를 먼저 만들고 교체하는 접근이 합리적이다.

OpenImageIO `ImageCache`는 자동 tile loading, file-handle 관리, invalidation을 제공하므로, current same-capture replacement model에서 full-screen raster 또는 타일 기반 이미지 접근 계층으로 사용할 가치가 있다. 반면 SQLite나 다른 DB는 여전히 audit, rollout, session lifecycle에는 중요하지만 **2.5초 close 자체를 직접 줄이지는 못한다.** 이 주제에서 storage technology의 핵심은 "무엇을 DB에 넣을까"보다 "어떤 preset-applied display artifact를 어떤 캐시 정책으로 곧바로 재사용할까"에 있다.

_Relational Databases:_ 운영 감사, preset catalog, rollout state에는 유효하지만 preset-applied close의 직접 해결책은 아님  
_NoSQL Databases:_ 이벤트/진단 적재는 가능하나 user-visible full-screen artifact를 대체하지는 못함  
_In-Memory Databases:_ 큐와 warm-state 관리 보조용으로는 가능하나, 최종 truth artifact는 파일/manifest가 더 적합  
_Data Warehousing:_ hardware validation 집계에는 유효하지만 hot path에는 비핵심  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/wic/-wic-rawguidelines-thumbnail-previews, https://helpx.adobe.com/photoshop-elements/using/process-camera-raw-image-files.html, https://openimageio.readthedocs.io/en/stable/imagecache.html

### Development Tools and Platforms

이번 목표에서 중요한 도구는 일반 IDE보다 **실제 full-screen close 비용을 해부할 수 있는 런타임 도구**다. darktable는 `-d opencl -d perf`로 module별 processing time과 OpenCL kernel profiling을 볼 수 있다고 명시한다. 이건 현재처럼 "activation은 맞지만 여전히 느리다"는 상황에서, 병목이 module stack인지, GPU warm-state인지, queue contention인지 분리하는 데 직접 쓸 수 있다. Windows Direct2D custom effects는 transform graph와 shader를 조합한 effect pipeline을 공식 지원하므로, Windows-only full-screen lane 프로토타입을 만들 때 플랫폼 적합성이 높다.

플랫폼 관점에서는 Tauri sidecar model이 계속 유효하다. Rust host가 local sidecar를 직접 실행하고 stdout/stderr 이벤트를 읽을 수 있으므로, 별도 서비스 매니저 없이 resident renderer를 앱 번들에 포함시키는 경로가 단순하다. Microsoft WIC performance 문서 역시 RAW 처리 성능 목표를 "업계 최고 수준 도구와 동급 또는 그 이상"으로 잡아야 한다고 설명한다. 즉, synthetic benchmark보다 **실제 booth hardware에서 product-visible full-screen close를 반복 측정하는 도구 체계**가 필수다.

_IDE and Editors:_ 이번 결정의 핵심 차별점은 아님  
_Version Control:_ preset bundle version pin, render policy versioning, rollback trace가 더 중요  
_Build Systems:_ native sidecar packaging, GPU dependency pinning, booth hardware 재현 빌드가 중요  
_Testing Frameworks:_ module profiling + hardware-in-loop latency trace가 우선  
_Source:_ https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/, https://learn.microsoft.com/en-us/windows/win32/direct2d/custom-effects, https://v2.tauri.app/ko/develop/sidecar/, https://learn.microsoft.com/en-us/windows/win32/wic/-wic-rawguidelines-performance

### Cloud Infrastructure and Deployment

현재 목표는 public cloud로 보내서 처리하는 문제가 아니라, **촬영 직후 booth 앞 이용자에게 full-screen preset-applied 결과를 2.5초 안에 보여주는 문제**다. 따라서 1차 배포 기본값은 여전히 **on-device local runtime**이어야 한다. 다만 booth PC 자체의 GPU/CPU headroom이 구조적으로 부족할 경우를 대비해, low-latency on-prem/edge 패턴은 reserve option으로 검토할 가치가 있다. AWS Outposts는 low-latency access to on-prem systems, local data processing, local interdependencies가 있는 workload에 적합하다고 설명하고, Google Distributed Cloud connected도 low-latency local data processing과 edge execution을 전면에 둔다.

이 자료들은 "엣지나 온프렘 분산 장비 자체가 나쁘지 않다"는 점은 보여준다. 하지만 Boothy의 현 단계에서 이 선택은 **2차 카드**다. booth 한 대에서 same-capture correlation을 유지하며 full-screen close를 줄이는 데는 local resident renderer가 먼저이고, edge appliance는 local lane으로도 목표를 못 닫을 때만 고려하는 편이 더 맞다. public cloud/serverless는 cold start, 네트워크 hop, 현장 장애 복구 측면에서 이번 KPI와 방향이 맞지 않는다.

_Major Cloud Providers:_ low-latency edge/on-prem 옵션은 존재하지만 이번 문제의 1순위는 아님  
_Container Technologies:_ appliance 운영 시 유용하지만 single-booth on-device 기본값에는 과할 수 있음  
_Serverless Platforms:_ cold start와 network dependency 때문에 이번 SLA와 부적합  
_CDN and Edge Computing:_ 여기서의 edge는 CDN이 아니라 booth 근처 on-prem compute를 의미  
_Source:_ https://aws.amazon.com/outposts/rack/, https://cloud.google.com/distributed-cloud-connected

### Technology Adoption Trends

이번 단계에서 읽힌 기술 채택 흐름은 비교적 선명하다. 첫째, **빠른 preview artifact를 별도로 가진다.** WIC는 prerendered preview cache를 권장하고, Adobe는 DNG fast load data와 embedded preview 옵션을 유지한다. 둘째, **preview path와 save/final path를 분리한다.** RawTherapee는 화면 표시와 저장 경로를 같은 효과 체인 안에서도 다른 performance policy로 운용할 수 있게 하고, 기술적으로는 main preview와 saved image가 서로 다른 pipeline 지점을 가진다고 설명한다. 셋째, **local GPU 또는 low-overhead resident execution을 활용한다.** darktable의 OpenCL 가속과 Windows custom effects stack은 모두 이 방향에 있다.

이 흐름을 Boothy에 적용하면, 가장 설득력 있는 기술 방향은 다음과 같다.

- JS/React shell은 유지하되, full-screen preset-applied close는 native/GPU lane으로 옮긴다.
- final export truth와 분리된 **display-sized preset-applied truthful artifact**를 first-class artifact로 다룬다.
- cache, embedded preview, tile cache, resident worker를 통해 cold 비용을 줄인다.
- darktable-compatible truth path 또는 parity oracle은 별도 reference lane으로 남긴다.

**Inference:** current public sources는 "Boothy가 당장 어떤 한 라이브러리만 넣으면 2.5초를 확정 달성한다"까지는 보장하지 않는다. 그러나 **full-screen close를 final export와 분리하고, cached truthful preview artifact를 native/GPU resident path에서 생성하는 방향**이 지금 목표와 가장 일치한다는 점은 강하게 뒷받침한다.

_Migration Patterns:_ single blocking render에서 preview/save 분리형 resident pipeline으로 이동  
_Emerging Technologies:_ embedded fast-load data, display-sized truthful preview artifact, GPU effect graph, edge reserve deployment  
_Legacy Technology:_ full-resolution render를 사용자 full-screen feedback의 유일한 닫힘 조건으로 두는 구조  
_Community Trends:_ cache-first preview, native/GPU hot path, explicit preview/save separation  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/wic/-wic-rawguidelines-thumbnail-previews, https://helpx.adobe.com/photoshop-elements/using/process-camera-raw-image-files.html, https://rawpedia.rawtherapee.com/Toolchain_Pipeline, https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/

## Integration Patterns Analysis

### Web Search Analysis

이번 단계에서는 통합 경계를 다섯 축으로 나눠 확인했다.

- **로컬 프로세스 통신:** Tauri sidecar, Windows anonymous pipes, Windows named pipes
- **원격 서비스 통신:** gRPC, Protocol Buffers, service discovery, health checking
- **웹/API 스타일:** REST, GraphQL, webhook
- **이벤트/메시징:** publish-subscribe, event-driven, AMQP, MQTT
- **보안 경계:** TLS, mutual TLS, OAuth 2.0, JWT

외부 자료와 현재 저장소 계약을 함께 보면, Boothy의 통합 패턴은 세 층으로 구분하는 것이 가장 자연스럽다.

1. **same-PC hot path**는 host-owned local IPC 또는 파일 경계가 우선이다.  
2. **reserve remote path**는 gRPC/Protobuf + TLS 계열이 우선이다.  
3. **운영 알림/비핫패스 통합**은 webhook 또는 pub/sub가 맞다.

이 구분이 중요한 이유는, `24인치 가로 풀화면 preset-applied close <= 2.5초` 목표는 API 유연성보다 **추가 네트워크 hop, serialization overhead, discovery complexity를 얼마나 피하느냐**에 더 민감하기 때문이다.

### API Design Patterns

Boothy의 현재 경계는 일반 public web API보다 **host-owned command boundary**에 가깝다. Microsoft Azure API design guidance는 REST가 HTTP 동사, 상태코드, idempotency 의미가 잘 정리되어 있고 폭넓은 상호운용성을 제공한다고 설명한다. 반면 RPC는 local method call처럼 보이기 쉬워 잘못 설계하면 chatty interface가 되기 쉽지만, 속도와 binary serialization 면에서는 더 유리할 수 있다고 정리한다. 이 차이는 Boothy에도 그대로 적용된다. booth UI나 operator surface가 host command를 coarse-grained request로 부르는 현재 구조에는 REST적 사고, 즉 "큰 동작 단위의 안정된 계약"이 잘 맞는다. 하지만 renderer를 원격 appliance로 떼는 순간에는 RPC가 더 자연스럽다.

GraphQL 공식 문서는 GraphQL 서버가 보통 단일 endpoint(`/graphql`)에서 동작하고, 인증은 GraphQL 실행 전에 middleware에서 처리하며, field-level authorization은 resolver/business logic에서 수행한다고 설명한다. 이 구조는 read-heavy 다목적 데이터 조회에는 강점이 있다. 그러나 Boothy의 핵심 흐름은 `request-capture`, `file-arrived`, `submit-preview-job`, `preview-ready`처럼 **명시적 상태 전이와 correlation이 중요한 command/event 흐름**이다. 따라서 GraphQL은 booth hot path의 기본 API 스타일로는 이점이 적고, 있다면 operator diagnostics나 rich read-model 조회에 한정된다.

gRPC는 서비스 인터페이스를 `.proto`로 정의하고, Protocol Buffers를 기본 IDL과 메시지 포맷으로 사용하며, 다른 머신의 서비스를 로컬 객체처럼 호출하게 해 준다고 공식 문서가 설명한다. 이건 edge appliance나 원격 renderer 후보에는 잘 맞는다. 하지만 same-PC 단일 부스 구조에서는 discovery, channel lifecycle, TLS 관리까지 새 복잡도를 가져온다. 현재 repo도 camera helper는 `camera-helper-requests.jsonl`/`camera-helper-events.jsonl` 파일 경계로, dedicated renderer는 request/result JSON artifact와 sidecar launch로 닫히고 있다. 이 현 상태를 보면 기본 선택은 여전히 **local command boundary**고, gRPC는 reserve remote route일 때만 가치가 커진다.

Webhook는 HTTPS endpoint로 비동기 event를 전달하는 패턴이다. Azure Event Grid 문서는 어떤 webhook이든 event handler가 될 수 있지만 HTTPS webhook endpoint만 지원한다고 밝힌다. 이는 operator alerting, evidence upload trigger, remote diagnostics notification에는 유용하지만, booth hot path에는 맞지 않는다. same-capture close는 synchronous 또는 near-synchronous local control이 필요하기 때문이다.

_RESTful APIs:_ UI-host, operator-host, rollout tooling 같은 coarse-grained control plane에 적합. broad interoperability와 명확한 idempotency 의미가 강점이다.  
_GraphQL APIs:_ read-heavy query surface에는 유용하지만 booth hot path의 command/event 흐름을 단순화하지는 못한다.  
_RPC and gRPC:_ edge appliance 또는 분리된 renderer service에 적합. binary serialization, typed IDL, health checking이 강점이다.  
_Webhook Patterns:_ operator-facing alerts, evidence forwarding, async integrations에는 유효하지만 booth capture-close path에는 부적합하다.  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/microservices/design/api-design, https://graphql.org/learn/serving-over-http/, https://grpc.io/docs/what-is-grpc/introduction/, https://learn.microsoft.com/en-us/azure/event-grid/handler-webhooks, https://v2.tauri.app/ko/develop/sidecar/

### Communication Protocols

현재 Boothy에 가장 가까운 통신 기준은 **local process IPC**다. Microsoft는 anonymous pipe가 보통 parent-child process 사이에서 쓰이는 one-way local pipe라고 설명하고, child process의 stdin/stdout redirection 예시도 공식적으로 제공한다. named pipe는 one-way 또는 duplex이고, 여러 클라이언트 인스턴스를 동시에 받을 수 있으며, related 또는 unrelated process 모두 접근 가능하다고 설명한다. 이 차이는 꽤 실무적이다. 지금처럼 Tauri host가 sidecar binary를 직접 띄우는 구조라면, anonymous pipe/stdin-stdout 계열이 가장 단순하다. 반대로 dedicated renderer를 부스 내 상주 service로 빼고 싶다면 named pipe가 더 자연스럽다.

gRPC는 원격 서비스 통신용으로는 더 강력하다. gRPC는 protocol buffers를 기본으로 사용하고, health checking, metadata, retries, graceful shutdown 같은 운영 요소를 공식 가이드로 제공한다. custom name resolution은 service discovery의 핵심이며, DNS를 watch-based resolver로 대체하거나 보강할 수 있다고 설명한다. 따라서 edge appliance를 실제로 도입할 때는 `gRPC + Protobuf + health check + service discovery`가 기본 조합이 된다.

HTTP/HTTPS는 여전히 broadest interoperability를 제공한다. 다만 booth hot path에서는 너무 범용적이다. GraphQL 공식 문서도 결국 GraphQL over HTTP의 transport는 HTTP와 JSON 위에 얹는다고 설명한다. 즉, HTTP는 operator API, remote management API, diagnostics export API에는 적합하지만, same-PC same-capture close에는 너무 두껍다. WebSocket은 RFC 6455 기준으로 browser와 remote host 사이의 양방향 통신을 제공하지만, 이 역시 booth 내부 hot path보다는 operator live dashboard나 remote monitoring 채널에 더 어울린다.

메시지 브로커 계열은 더 비핫패스에 가깝다. MQTT는 OASIS 표준으로, 매우 lightweight한 pub/sub transport이며 작은 코드 footprint와 minimal network bandwidth를 장점으로 내세운다. AMQP 1.0은 비동기 메시지 전송을 위한 표준화된 프로토콜이다. 이런 프로토콜은 booth hardware telemetry fan-out이나 remote appliance fleet control에는 의미가 있지만, 단일 booth의 same-capture full-screen close에는 과한 경우가 많다.

_HTTP/HTTPS Protocols:_ operator/control plane과 external integrations에는 적합하지만, booth hot path 기본값으로는 과하다.  
_WebSocket Protocols:_ 양방향 push에는 유효하지만 same-PC preset-applied close owner 경계로는 우선순위가 낮다.  
_Message Queue Protocols:_ MQTT/AMQP는 remote telemetry, fleet orchestration, async fan-out에는 유용하지만 booth close path와는 거리가 있다.  
_gRPC and Protocol Buffers:_ edge appliance 또는 remote renderer 후보에 가장 설득력 있는 조합이다.  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/ipc/anonymous-pipe-operations, https://learn.microsoft.com/en-us/windows/win32/procthread/creating-a-child-process-with-redirected-input-and-output, https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipes, https://grpc.io/docs/what-is-grpc/introduction/, https://grpc.io/docs/guides/health-checking/, https://grpc.io/docs/guides/custom-name-resolution/, https://mqtt.org/, https://docs.oasis-open.org/amqp/core/v1.0/csprd01/amqp-core-complete-v1.0-csprd01.pdf, https://www.rfc-editor.org/rfc/rfc6455

### Data Formats and Standards

데이터 포맷은 "어떤 게 더 예쁘냐"가 아니라 **same-capture correlation, schema evolution, payload cost, diagnostics readability**를 얼마나 잘 맞추느냐가 중요하다. GraphQL 공식 문서는 GraphQL over HTTP가 JSON 요청/응답을 기본으로 다룬다고 설명한다. 현재 Boothy camera helper 계약도 BOM 없는 UTF-8 JSON Lines와 `schemaVersion` 필드를 canonical framing으로 고정한다. 이는 로컬 진단과 재현성 측면에서 매우 실용적이다. 사람이 바로 읽을 수 있고, line-by-line append가 쉽고, requestId/captureId/sessionId correlation을 남기기 좋기 때문이다.

반면 remote renderer나 appliance 통신은 JSON보다 Protobuf가 더 나을 수 있다. Protocol Buffers 공식 문서는 protobuf가 language-neutral, platform-neutral, extensible serialization이며 JSON보다 더 작고 빠르다고 설명한다. 또한 `.proto` 기반 코드 생성과 backward-compatible schema evolution이 강점이다. 따라서 local diagnostics와 evidence는 계속 JSON/JSONL로 두고, remote high-frequency RPC만 Protobuf로 분리하는 이중 포맷 전략이 가장 현실적이다.

MessagePack도 binary serialization 대안으로 의미는 있다. 공식 사이트는 MessagePack을 "JSON 같지만 더 빠르고 더 작다"고 설명한다. 다만 Boothy처럼 명시적 계약 문서, cross-language stub generation, remote RPC method typing이 중요한 경우에는 Protobuf 쪽이 더 구조적이다. MessagePack은 로컬 캐시 blob, 내부 binary cache, 또는 네트워크 메시지 최적화 보조안 정도가 적절하다.

CSV/flat file는 이번 주제에서 우선순위가 낮다. 현재 repo는 diagnostics와 request/event transport에 JSONL을 사용한다. 이 선택은 맞다. CSV는 capture-bound structured state, optional fields, schema versioning, nested payload를 다루기 불편하다. flat file 자체는 계속 중요하지만, **포맷은 CSV보다 JSON/JSONL 또는 binary artifact metadata가 맞다.**

_JSON and XML:_ local diagnostics, request/result artifact, operator-readable evidence에는 JSON/JSONL이 가장 적합하다. XML은 XMP adapter compatibility 경계에서만 의미가 크다.  
_Protobuf and MessagePack:_ remote renderer, edge appliance, high-frequency binary RPC에는 protobuf가 우선이고, MessagePack은 경량 보조안이다.  
_CSV and Flat Files:_ flat file boundary는 유효하지만 CSV는 현재 문제의 구조화된 correlation payload에 적합하지 않다.  
_Custom Data Formats:_ 현재 repo의 `schemaVersion + UTF-8 JSONL + filesystem handoff` 패턴은 product-fit이 높다.  
_Source:_ https://graphql.org/learn/serving-over-http/, https://protobuf.dev/overview/, https://msgpack.org/, https://www.libraw.org/docs/API-overview.html, https://v2.tauri.app/ko/develop/sidecar/

### System Interoperability Approaches

Boothy는 현재도 이미 **point-to-point integration**을 택하고 있다. camera helper는 session diagnostics file boundary를 canonical contract로 고정했고, dedicated renderer는 host-owned sidecar 실행과 capture-bound request/result artifact를 사용한다. 이건 장점이 분명하다. same-session, same-capture, same-preset-version correlation을 강하게 유지할 수 있고, diagnostics evidence를 capture 단위로 바로 묶을 수 있다. 이 주제에서 point-to-point는 "임시방편"이 아니라 오히려 제품 목표에 맞는 1차 선택이다.

API gateway나 service mesh는 이 단계에선 과하다. Azure API gateway 문서는 gateway가 routing, aggregation, offloading을 도와주지만, 단일 operation이 여러 서비스 호출을 요구할 경우 latency가 늘 수 있다고 설명한다. 이는 booth close path 관점에서 불리한 신호다. service mesh도 원격 다서비스 환경에서는 유용하지만 single-booth same-PC 구조에는 이득보다 표면적 복잡도가 더 크다.

그렇다고 point-to-point만 영원히 유지하자는 뜻은 아니다. 원격 appliance나 multi-booth fleet 운영으로 가면 `gateway + service discovery + health check + circuit breaker`가 필요해진다. Microsoft의 microservices guidance는 service registry/service discovery를 핵심 요소로 보고, gRPC custom name resolution도 discovery의 실제 기제로 설명한다. 따라서 **현재 1차안은 point-to-point local integration, 2차 reserve안은 service-style remote integration**으로 계층을 나누는 것이 맞다.

Enterprise Service Bus는 더더욱 과하다. pub/sub, broker, transformation, routing 중심 구조는 여러 독립 시스템을 느슨하게 연결할 때 장점이 있지만, booth 한 대의 same-capture close에는 오히려 실패 지점을 늘릴 뿐이다.

_Point-to-Point Integration:_ 현재 목표에 가장 적합하다. capture correlation과 booth-safe fallback 유지가 쉽다.  
_API Gateway Patterns:_ remote multi-service 환경에서는 유용하지만 booth hot path에는 latency와 complexity를 더할 가능성이 크다.  
_Service Mesh:_ fleet-scale remote renderer가 아니면 과하다.  
_Enterprise Service Bus:_ 현 단계 product-fit이 낮다.  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/microservices/design/gateway, https://learn.microsoft.com/en-us/dotnet/architecture/microservices/architect-microservice-container-applications/microservices-addressability-service-registry, https://grpc.io/docs/guides/custom-name-resolution/

### Microservices Integration Patterns

이번 주제에서 microservices pattern은 "당장 써야 할 것"과 "reserve option으로만 볼 것"을 분리해야 한다. API gateway는 public entrypoint를 정리하고 aggregation을 제공하지만, Azure 문서가 직접 말하듯 multiple network round trips 때문에 latency를 늘릴 수 있다. 따라서 booth hot path에는 비권장이고, remote operator plane이나 multi-appliance management plane에만 적합하다.

service discovery는 edge appliance를 실제로 도입할 때 필수다. gRPC custom name resolution은 service discovery를 위해 DNS를 보강하거나 대체할 수 있다고 설명한다. health checking도 gRPC가 표준 `health/v1` API를 제공하므로, renderer appliance 상태를 watch 기반으로 반영할 수 있다. 하지만 single-booth local path에서는 이 레벨까지 갈 이유가 적다.

Circuit breaker는 remote dependency가 생기는 순간 중요해진다. Microsoft는 circuit breaker가 고장난 remote service에 대한 반복 접근을 일시적으로 막아 시스템 회복성을 높인다고 설명한다. 따라서 edge renderer가 도입되면, booth host는 remote renderer 오류가 반복될 때 즉시 local truthful waiting/fallback으로 내릴 수 있어야 한다.

Saga는 이 문제에서 고전적 분산 트랜잭션으로 이해하기보다, **capture-close workflow의 보상 흐름**으로 해석하는 편이 낫다. Microsoft의 compensating transaction 문서는 원래 작업의 각 단계를 되돌리는 보상 단계와 그 진행 상태 기록을 강조한다. 이는 `remote renderer submit -> timeout -> fallback waiting 유지 -> evidence 기록 -> retry 또는 rollback` 같은 흐름에 잘 맞는다. 다만 single-PC local lane에서는 full microservice saga를 구현할 이유는 크지 않다.

_API Gateway Pattern:_ remote control plane에만 유효, booth hot path에는 비권장  
_Service Discovery:_ edge appliance 도입 시 필요, local single-booth 기본값에는 불필요  
_Circuit Breaker Pattern:_ remote renderer 실패 시 즉시 local fallback으로 내려가기 위한 핵심 패턴  
_Saga Pattern:_ 엄밀히는 compensating transaction에 가까운 rollback/retry/evidence workflow로 해석하는 편이 맞다  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/microservices/design/gateway, https://grpc.io/docs/guides/custom-name-resolution/, https://grpc.io/docs/guides/health-checking/, https://learn.microsoft.com/en-us/azure/architecture/patterns/circuit-breaker, https://learn.microsoft.com/en-us/azure/architecture/patterns/compensating-transaction

### Event-Driven Integration

이벤트 구동은 Boothy에 이미 일부 들어와 있다. camera helper는 `capture-accepted`, `fast-preview-ready`, `file-arrived`, `helper-error` 같은 event를 남긴다. dedicated renderer도 `preview-promotion-evidence.jsonl`과 timing events를 남긴다. 즉, 현재 구조는 완전한 message broker는 아니지만 **event-sourced diagnostics seam**은 갖고 있다.

Azure의 event-driven architecture 문서는 event producer, consumer, event channel로 구성되고 producer와 consumer가 decoupled된다고 설명한다. publisher-subscriber pattern 문서는 correlation ID, schema evolution, idempotency를 고려해야 하며, near real-time synchronous response가 필요하면 request-reply가 더 적합하다고 말한다. 이건 Boothy에 중요하다. booth hot path는 완전한 pub/sub보다는 request-reply + append-only event trace의 혼합형이 더 맞다. event broker를 앞세우면 지연과 eventual consistency가 늘기 때문이다.

따라서 event-driven 전략은 두 층으로 쓰는 것이 좋다.

- **hot path:** request/reply 중심, event는 trace와 state projection용 보조물
- **non-hot path:** publish-subscribe 중심, operator notification, fleet telemetry, validation analytics fan-out

CQRS도 같은 맥락에서 제한적으로 유효하다. Microsoft는 CQRS가 read/write concerns를 분리해 읽기 성능 최적화와 독립 발전을 돕는다고 설명한다. Boothy에선 booth runtime truth(write model)와 operator analytics/read model을 분리하는 데는 도움이 되지만, same-capture close owner를 더 빠르게 하지는 못한다.

_Publish-Subscribe Patterns:_ operator notifications, telemetry fan-out, analytics에는 유효하지만 booth close path 기본값은 아니다.  
_Event Sourcing:_ full event store보다는 capture-bound diagnostics/evidence append log 형태가 더 product-fit이 높다.  
_Message Broker Patterns:_ fleet/remote orchestration에는 유효, single-booth hot path에는 과할 수 있다.  
_CQRS Patterns:_ booth truth와 operator read-model 분리에는 유효하지만 close latency 직접 해결책은 아니다.  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/event-driven, https://learn.microsoft.com/en-us/azure/architecture/patterns/publisher-subscriber, https://learn.microsoft.com/en-us/azure/architecture/patterns/cqrs, https://learn.microsoft.com/en-us/azure/event-grid/handler-webhooks

### Integration Security Patterns

보안 패턴도 같은 기준으로 봐야 한다. same-PC 단일 부스 hot path에서는 public OAuth dance보다 **process boundary 최소화와 local path validation**이 더 중요하다. 그러나 remote renderer, remote diagnostics API, webhook integration을 열면 보안 경계가 달라진다.

OAuth 2.0은 제3자 애플리케이션이 HTTP 서비스에 제한된 접근을 얻기 위한 표준 authorization framework다. JWT는 compact한 claims 표현 형식으로, 서명되거나 암호화될 수 있다. 이 두 기술은 operator portal이나 remote management API에는 유용하지만, booth local renderer sidecar에 기본 적용할 대상은 아니다. 불필요한 토큰 검증과 관리 복잡도만 늘 수 있기 때문이다.

원격 renderer service가 생기면 mTLS와 TLS가 우선이다. gRPC 인증 가이드는 SSL/TLS를 기본 transport security로 두고, 상호 인증을 위해 client certificates를 사용할 수 있다고 설명한다. RFC 8705는 OAuth 2.0 mutual TLS client authentication과 certificate-bound access token을 정의한다. 따라서 edge appliance가 현실화되면 **channel-level mTLS + optional token auth** 조합이 가장 타당하다.

Webhook는 Azure Event Grid 기준 HTTPS endpoint만 지원하므로, 최소한 TLS가 전제다. local-only booth path는 filesystem boundary와 signed release artifacts, path validation, same-session correlation이 더 중요하고, remote path는 TLS/mTLS 및 explicit authz가 중요하다.

_OAuth 2.0 and JWT:_ remote operator API나 external service integration에는 유효하지만 local booth hot path 기본값은 아님  
_API Key Management:_ internal tooling에선 가능하지만 rotation과 leakage risk를 고려하면 remote production path 기본값으로는 약하다  
_Mutual TLS:_ edge appliance 또는 remote renderer에 가장 적합한 기본 transport security다  
_Data Encryption:_ webhook/remote API는 TLS가 전제, local booth는 process/file boundary 보호와 path validation이 우선이다  
_Source:_ https://www.rfc-editor.org/rfc/rfc6749, https://www.rfc-editor.org/rfc/rfc7519, https://grpc.io/docs/guides/auth/, https://datatracker.ietf.org/doc/html/rfc8705, https://learn.microsoft.com/en-us/azure/event-grid/handler-webhooks

## Architectural Patterns and Design

### System Architecture Patterns

이번 주제에서 가장 중요한 구조 선택은 `full microservices`가 아니라 **local-first modular core + sidecar worker architecture**다. Azure architecture styles guidance는 아키텍처 스타일이 품질 속성, 배포, 운영 방식을 크게 좌우하므로 문제 성격에 따라 선택해야 한다고 본다. AWS의 branch-by-abstraction pattern은 기존 구현과 새 구현을 동시에 두고 쉽게 되돌릴 수 있는 전환 방식을 제공한다고 설명한다. Azure의 anti-corruption layer pattern은 외부 시스템의 의미 체계가 내부 설계를 오염시키지 않게 adapter/façade를 두라고 권장한다. Azure의 ambassador pattern은 같은 호스트에 colocated proxy를 둬 연결성, 라우팅, TLS, 모니터링을 오프로드하는 방식을 제시한다.

이 네 패턴을 Boothy에 맞춰 합치면 가장 적합한 기본 형태는 다음과 같다.

- **제품 코어는 modular monolith**로 유지한다.
- **camera SDK / darktable-compatible truth path / dedicated renderer**는 sidecar 또는 adapter boundary로 분리한다.
- **새 full-screen close lane 도입은 branch-by-abstraction + route policy**로 점진 전환한다.
- **darktable/XMP semantics는 anti-corruption layer 안에 가둔다.**

이 구조가 맞는 이유는 목표가 "여러 팀이 독립 배포하는 클라우드 서비스"가 아니라, **한 부스에서 same-capture preset-applied full-screen artifact를 2.5초 내에 보여주는 것**이기 때문이다. full microservices는 팀 분리와 독립 배포에는 유리하지만, 현재 문제에는 네트워크 hop, discovery, distributed failure surface를 추가한다. 반대로 sidecar pattern은 같은 호스트에서 낮은 지연과 분리된 실패 경계를 함께 얻을 수 있다. 원격 appliance가 필요해지는 경우에도, 첫 단계는 microservices replatforming보다 **현재 modular core 바깥에 remote renderer cell 하나를 추가하는 형태**가 더 안전하다.

**Inference:** 현재 목표와 코드베이스 상태를 보면, 권장 1순위 시스템 패턴은 `modular monolith + dedicated sidecar lane + anti-corruption adapters + branch-by-abstraction rollout`이다. `full microservices`는 과설계 가능성이 높다.

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/, https://docs.aws.amazon.com/prescriptive-guidance/latest/modernization-decomposing-monoliths/branch-by-abstraction.html, https://learn.microsoft.com/en-us/azure/architecture/patterns/anti-corruption-layer, https://learn.microsoft.com/en-us/azure/architecture/patterns/ambassador

### Design Principles and Best Practices

이번 문제에 맞는 설계 원칙은 일반적인 SOLID 교과서보다 더 구체적이다. Microsoft의 design principles 문서는 운영되는 클라우드 애플리케이션이 self-healing, redundancy, partitioning, asynchronous messaging, evolution, operation visibility를 고려해야 한다고 설명한다. .NET microservices guidance는 domain-driven design에서 bounded context가 모델 경계를 분리하는 핵심이라고 본다. AWS의 hexagonal architecture guidance는 domain logic을 외부 인프라 세부사항에서 분리하되, 모든 것을 과도하게 ports-and-adapters로 만들면 복잡도가 커질 수 있다고 본다.

Boothy에 적용하면 설계 원칙은 아래처럼 좁혀진다.

- **bounded context를 분명히 나눈다:** capture, preset publication, preview close, final export, operator diagnostics
- **ports & adapters는 외부 seam에만 쓴다:** camera SDK, darktable/XMP, dedicated renderer, remote appliance
- **도메인 진실과 사용자 피드백을 분리하되 혼동하지 않는다:** first-visible은 advisory, preset-applied full-screen visible만 성공 close
- **새 lane은 기존 truth를 오염시키지 않게 추가한다:** anti-corruption rule 유지
- **설계는 교체 가능성과 rollback을 전제한다:** branch-by-abstraction, feature/route policy

즉, clean/hexagonal 사고는 유효하지만, 전 범위에 동일 강도로 적용하는 것이 아니라 **외부 의존 경계에 집중 적용**하는 편이 맞다. Boothy가 가장 피해야 하는 것은 계층을 아름답게 나누느라 hot path를 길게 만드는 일이다. 설계 원칙은 purity보다 product latency를 먼저 지켜야 한다.

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/guide/design-principles/, https://learn.microsoft.com/en-us/azure/architecture/patterns/anti-corruption-layer, https://learn.microsoft.com/en-us/dotnet/architecture/microservices/microservice-ddd-cqrs-patterns/ddd-oriented-microservice, https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/hexagonal-architecture.html

### Scalability and Performance Patterns

이번 문제에서 scaling pattern은 cloud-scale throughput보다 **booth-local latency budgeting**이 우선이다. Azure queue-based load leveling pattern은 비동기 큐로 burst를 흡수하는 데 유용하지만, 서비스가 minimal latency 응답을 기대하는 경우에는 적합하지 않다고 설명한다. Priority Queue pattern은 urgency가 다른 작업을 우선순위별로 처리할 수 있게 하고, 서로 다른 SLA를 가진 작업을 분리하는 데 유용하다고 말한다. Bulkhead pattern은 자원을 격리해 한 부분의 실패가 전체로 번지는 것을 막는다. Autoscaling guidance는 instrumentation, decision logic, scaling action이 workload 코드와 분리되어야 한다고 설명한다.

이를 Boothy에 맞춰 해석하면 다음이 맞다.

- **current capture full-screen close는 queue-based load leveling의 주 대상이 아니다.**
- **대신 priority queue가 맞다:** current capture close > current session visible sync > background backfill > final export > analytics
- **bulkhead가 중요하다:** full-screen close lane은 backfill, parity diff, diagnostics export, final render와 자원을 분리해야 한다.
- **throttling/degradation이 필요하다:** 과부하 시 nonessential work를 늦추고 booth-safe waiting을 유지해야 한다.
- **autoscaling은 remote appliance 도입 때만 본격 의미가 있다.**

즉, 이 제품에서 성능 패턴의 핵심은 "어떻게 많이 처리할까"보다 "무엇을 먼저 처리하고 무엇을 늦출까"다. 같은 캡처의 풀화면 결과를 2.5초 안에 보여줘야 하므로, latency-critical lane은 짧고 독립적이어야 한다.

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/queue-based-load-leveling, https://learn.microsoft.com/en-us/azure/architecture/patterns/priority-queue, https://learn.microsoft.com/en-us/azure/architecture/patterns/bulkhead, https://learn.microsoft.com/en-us/azure/architecture/patterns/throttling, https://learn.microsoft.com/en-us/azure/architecture/best-practices/auto-scaling

### Integration and Communication Patterns

아키텍처 관점에서도 통합 기본값은 변하지 않는다. same-host에서는 **sidecar + local IPC/file contract**, remote에서는 **service + gRPC/Protobuf**다. Azure ambassador pattern은 client 옆의 out-of-process proxy가 routing, resiliency, security, monitoring을 오프로드할 수 있다고 설명한다. 이는 renderer를 별도 프로세스로 두고 route policy, fallback, observability를 바깥으로 빼는 현재 방향과 잘 맞는다. 반면 gateway aggregation/offloading/routing은 원격 다서비스 환경에선 유효하지만, 단일 부스 hot path에는 추가 네트워크 단계와 제어면을 만든다.

따라서 구조 수준에서의 권장은 아래와 같다.

- **로컬 기본형:** host-owned sidecar + local point-to-point contracts
- **원격 예비형:** renderer appliance cell + gRPC/Protobuf + health checks + circuit breaker
- **비권장 기본형:** API gateway 중심, broker 중심, service mesh 중심 hot path

이 판단은 microservices 일반론이 아니라, `same-capture correlation`과 `fallback`을 제품 acceptance에 포함시키는 현재 목표에서 나온 것이다.

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/ambassador, https://learn.microsoft.com/en-us/azure/architecture/microservices/design/gateway, https://grpc.io/docs/what-is-grpc/introduction/, https://grpc.io/docs/guides/health-checking/, https://grpc.io/docs/guides/custom-name-resolution/

### Security Architecture Patterns

보안 구조는 local booth와 remote extension을 분리해 봐야 한다. Azure의 architecture guidance는 security가 별도 부가 기능이 아니라 workload boundary 설계의 일부여야 한다고 본다. remote path에서는 TLS/mTLS, health/auth separation, least-privilege identity가 핵심이다. local path에서는 public API auth보다 **권한 분리, 파일 경로 검증, sidecar 실행 경계, signed artifact, operator/authoring 분리**가 더 중요하다.

아키텍처 수준에서 보면 다음이 맞다.

- **local booth path:** process isolation, file path allow-list, schema validation, capability gating
- **remote renderer path:** mTLS 기본, optional token auth, explicit health/auth separation
- **authoring/operator separation:** 같은 앱이어도 capability-gated surface로 분리
- **trust minimization:** fast preview advisory path는 truth 승격 권한이 없다

즉, 이번 주제의 보안 패턴은 "OAuth를 쓸까?"가 아니라 **어떤 경계가 결과를 진실로 승격할 수 있느냐**를 통제하는 데 더 가깝다.

_Source:_ https://grpc.io/docs/guides/auth/, https://datatracker.ietf.org/doc/html/rfc8705, https://www.rfc-editor.org/rfc/rfc6749, https://learn.microsoft.com/en-us/azure/architecture/patterns/anti-corruption-layer, https://learn.microsoft.com/en-us/azure/well-architected/security/

### Data Architecture Patterns

이번 목표에 맞는 데이터 구조는 관계형 중심이 아니라 **capture-bound file truth + manifest projection + append-only evidence**다. Azure external configuration store pattern은 구성과 코드 분리를 통해 versioned configuration과 safe rollout을 돕는다고 설명한다. Cache-Aside pattern은 자주 읽는 데이터를 cache에 두되 source of truth를 유지하는 방식이다. Event-driven/CQRS guidance는 write model과 read model의 분리를 가능하게 하지만, 결국 authoritative write path가 분명해야 한다고 본다.

Boothy에 가장 맞는 데이터 아키텍처는 다음과 같다.

- **authoritative write model:** session-scoped capture record + capture-bound preset version + canonical preview/final artifact
- **cache/read model:** display-sized preset-applied artifact, recent session projection, operator summary
- **append-only evidence:** timing log, preview promotion evidence, route policy snapshot
- **externalized config artifact:** `preview-renderer-policy.json` 같은 route policy를 코드 밖 rollout artifact로 취급
- **anti-corruption data adapter:** XMP/darktable-specific metadata는 adapter layer에 격리

이 구조에서 cache는 truth를 대체하지 않는다. display-sized full-screen artifact가 중요하다고 해도, 그것은 "빠른 truth projection"이지 별도 무책임한 미리보기가 아니다. 또한 full event sourcing 전체 도입은 필요 없어 보인다. 현재 목적엔 capture-bound audit/evidence seam 정도면 충분하다.

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/external-configuration-store, https://learn.microsoft.com/en-us/azure/architecture/patterns/cache-aside, https://learn.microsoft.com/en-us/azure/architecture/patterns/cqrs, https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/event-driven

### Deployment and Operations Architecture

운영 구조는 현재 Boothy가 이미 일부 채택한 방향과 잘 맞는다. Azure Well-Architected operational excellence guidance는 safe deployment, incremental evolution, observability를 지원하는 패턴을 강조한다. Deployment Stamps pattern은 같은 unit을 제어된 배포 단위로 관리하는 접근을 설명한다. Branch by abstraction은 rollback 가능한 병행 구현을 가능하게 하고, health endpoint monitoring은 시스템 상태를 표준화해 triage를 쉽게 한다.

이걸 Boothy 제품 운영으로 번역하면 아래가 권장 구조다.

- **local route policy canary/default/rollback**를 deployment primitive로 유지
- **renderer lane은 branch-by-abstraction으로 shadow -> canary -> default** 순으로 노출
- **health / warm-state / fallback ratio / replacementMs**를 sign-off 기준에 포함
- **원격 appliance가 생기면 deployment stamp/cell 단위로 운영**하고 부스별 격리를 유지
- **release decision은 correctness + latency + fallback stability를 함께 본다**

즉, 운영 구조도 마찬가지로 "대형 플랫폼식 중앙화"보다 **부스 단위 안전 전개와 즉시 롤백 가능성**이 우선이다.

_Source:_ https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/design-patterns, https://learn.microsoft.com/en-us/azure/architecture/patterns/deployment-stamp, https://docs.aws.amazon.com/prescriptive-guidance/latest/modernization-decomposing-monoliths/branch-by-abstraction.html, https://learn.microsoft.com/en-us/azure/architecture/patterns/health-endpoint-monitoring

## Implementation Approaches and Technology Adoption

### Technology Adoption Strategies

도입 전략의 핵심은 `빅뱅 전환`이 아니라 **작은 변화 + 점진 노출 + 즉시 종료 가능성**이다. Azure Strangler Fig pattern은 큰 시스템이나 핵심 기능을 교체할 때 위험을 줄이기 위해 점진적 전환이 유효하다고 설명한다. Azure DevOps의 feature flags guidance는 deployment와 exposure를 분리하고, 필요 시 빠른 off switch를 제공한다고 설명한다. Azure Well-Architected safe deployment guidance 역시 small, incremental, quality-gated release와 progressive exposure를 권장한다.

Boothy에 맞는 채택 전략은 명확하다.

- **1차:** 기존 local path를 유지한 채 새 full-screen close lane을 `route policy / feature flag` 뒤에 둔다.
- **2차:** shadow evidence를 먼저 수집하고, 이후 canary -> default로 넓힌다.
- **3차:** health gate를 통과하지 못하면 즉시 이전 trusted behavior로 내린다.
- **4차:** local lane으로도 목표를 못 닫을 때만 remote appliance를 별도 실험한다.

즉, adoption strategy의 목표는 새 기술을 빨리 켜는 것이 아니라 **사용자가 느끼는 위험을 최소화하면서 product acceptance를 빠르게 검증하는 것**이다.

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig, https://learn.microsoft.com/en-us/devops/operate/progressive-experimentation-feature-flags, https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/safe-deployments

### Development Workflows and Tooling

개발 흐름은 `작은 변경`, `반복 가능한 자동 검증`, `증적 보존` 중심이어야 한다. GitHub Actions workflow syntax 문서는 job dependency, 최소 권한 `permissions`, 환경별 제어를 제공한다. Deployment environments는 환경별 승인, 브랜치 제한, custom protection rules, secrets 제한에 유용하다. Dependency caching은 반복 실행 속도를 줄여 비용과 시간 둘 다 절약하게 해 준다. Workflow artifacts는 빌드 결과물, 테스트 로그, crash/debug 증적을 보관하고 검증하는 표준 경로를 제공한다.

Boothy에는 아래 개발 흐름이 맞다.

- **PR 단계:** schema/contracts, unit/integration tests, sidecar build, UI regression tests
- **Windows package 단계:** booth 실행본, sidecar binary, route policy artifact를 함께 빌드
- **evidence 단계:** timing logs, Playwright traces, preview evidence bundle, hardware-validation logs를 artifact로 남긴다
- **environment gate 단계:** dev -> canary -> booth validation 환경별 보호 규칙을 둔다

즉, tooling의 목적은 편의가 아니라 **same-capture full-screen 목표를 재현 가능한 방식으로 증명하고 되돌릴 수 있게 만드는 것**이다.

_Source:_ https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax, https://docs.github.com/en/actions/concepts/workflows-and-actions/deployment-environments, https://docs.github.com/en/actions/concepts/workflows-and-actions/dependency-caching, https://docs.github.com/en/actions/tutorials/store-and-share-data

### Testing and Quality Assurance

테스트 전략은 기능 수보다 **제품 acceptance를 얼마나 직접 검증하느냐**가 중요하다. Azure testing guidance는 test pyramid를 기본으로 삼고, E2E는 business-critical flow에만 선택적으로 쓰라고 권장한다. 또한 production-like 환경을 유지하고 configuration drift를 자동 검증하라고 설명한다. Playwright best practices는 사용자에게 보이는 동작을 테스트하고 implementation detail에 의존하지 말라고 권장한다. Trace Viewer는 retry 시 trace를 남겨 실패 시점의 UI/네트워크/콘솔 상태를 복기하게 해 준다.

Boothy의 테스트 계층은 아래처럼 잡는 것이 맞다.

- **계약 테스트:** session, preset, sidecar request/result, route policy schema
- **호스트/렌더러 테스트:** same-capture 보장, wrong-session 차단, fallback/rollback 규칙
- **E2E 테스트:** booth에서 사용자가 보는 `full-screen preset-applied visible`만 검증
- **하드웨어 검증:** 실제 카메라, 실제 preset, 실제 모니터에서 `<= 2500ms`를 측정
- **production-like canary:** 낮은 노출 상태에서 controlled testing in production

즉, 테스트 합격선은 `모든 테스트가 녹색`이 아니라 **사용자가 같은 사진의 프리셋 적용 결과를 2.5초 내 풀화면으로 보는가**다.

_Source:_ https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/testing, https://playwright.dev/docs/best-practices, https://playwright.dev/docs/trace-viewer-intro

### Deployment and Operations Practices

운영 실천은 safe deployment와 observability가 중심이다. Microsoft의 safe deployment practices 문서는 tier-based rollout, bake time, quality signals, rollback readiness를 강조한다. Azure Well-Architected incident response guidance는 end-to-end traceability, structured telemetry, containment, blameless postmortem을 권장한다. OpenTelemetry는 traces, metrics, logs를 표준 방식으로 수집/전송하는 관측성 프레임워크를 제공한다.

Boothy 운영 기본값은 아래가 적절하다.

- **점진 rollout:** shadow -> canary -> broader canary -> default
- **health gate:** `replacementMs`, `fallback ratio`, `wrong-capture`, `fidelity drift`, `warm-state stability`
- **observability:** capture request부터 full-screen visible까지 하나의 trace 또는 최소한 강한 correlation chain 유지
- **incident posture:** 문제 발생 시 rollout 중단, trusted path로 복귀, evidence 보존, 회고 반영

즉, 운영은 “새 경로가 있긴 하다”가 아니라 **새 경로가 health model 안에서 계속 안전한가**를 반복 확인하는 과정이어야 한다.

_Source:_ https://learn.microsoft.com/en-us/devops/operate/safe-deployment-practices, https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/safe-deployments, https://learn.microsoft.com/en-au/azure/well-architected/operational-excellence/incident-response, https://opentelemetry.io/docs/concepts/

### Team Organization and Skills

팀 구조는 도구보다 중요하다. Azure DevOps culture guidance는 workload team이 end-to-end ownership을 가져야 하고, cross-functional team이 각자의 전문성을 유지하면서도 전체 흐름을 이해해야 한다고 설명한다. 또한 safe experimentation과 continuous learning을 위한 enablement가 필요하다고 본다.

Boothy에는 아래 조직 원칙이 맞다.

- **한 셀의 end-to-end ownership:** booth UI, host, renderer, hardware validation, rollout 판단
- **지원 조직은 enablement 역할:** platform/review/support는 가이드와 표준 제공, 최종 책임은 workload team
- **필수 역량:** Windows native runtime, GPU/render profiling, contract testing, booth hardware validation, observability
- **의사결정 방식:** evidence-based final call, blameless review

즉, 이 과제는 프런트만, 렌더러만, 운영만 따로 보는 구조보다 **한 팀이 제품 경계 전체를 책임지는 구조**에서 성공 확률이 더 높다.

_Source:_ https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/devops-culture

### Cost Optimization and Resource Management

비용 최적화는 “가장 싼 기술”이 아니라 **요구사항을 만족하면서 낭비를 줄이는 구조 선택**이다. Azure cost optimization principles는 요구사항을 만족하는 범위 안에서 tradeoff를 하라고 강조한다. GitHub dependency caching은 반복 빌드 시간을 줄이고, artifacts는 필요한 증적만 보존하게 해 준다. production-like environment는 중요하지만, 모든 단계에서 풀복제를 유지하면 비용이 커질 수 있다고 Azure testing guidance도 설명한다.

Boothy에는 아래 비용 원칙이 맞다.

- **우선순위 1:** local lane 최적화에 먼저 투자
- **우선순위 2:** expensive test는 hardware gate와 production-like canary에 집중
- **우선순위 3:** CI에서는 caching과 artifact retention으로 반복 비용을 줄인다
- **우선순위 4:** edge appliance는 local lane 목표 미달이 확인된 뒤에만 검토

즉, 비용 최적화의 핵심은 remote infrastructure를 빨리 늘리는 것이 아니라 **목표 달성에 가장 직접적인 경로에만 비용을 쓰는 것**이다.

_Source:_ https://learn.microsoft.com/en-us/azure/well-architected/cost-optimization/principles, https://docs.github.com/en/actions/concepts/workflows-and-actions/dependency-caching, https://docs.github.com/en/actions/tutorials/store-and-share-data, https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/testing

### Risk Assessment and Mitigation

구현 리스크는 이미 꽤 선명하다.

- **리스크 1:** same-capture는 맞지만 full-screen close가 여전히 느릴 수 있다
- **리스크 2:** fidelity 확보를 위해 truth path를 무겁게 남기면 목표 시간을 못 닫을 수 있다
- **리스크 3:** feature flag/route policy가 있어도 health gate가 약하면 문제를 늦게 잡을 수 있다
- **리스크 4:** CI는 녹색인데 실장비/실모니터에서는 제품 실패가 날 수 있다
- **리스크 5:** 팀이 경계별로 분리되면 문제 triage 속도가 느려진다

권장 완화는 아래다.

- **점진 노출:** progressive exposure + bake time
- **빠른 종료:** feature flag / route rollback / trusted fallback
- **강한 관측성:** trace, metrics, logs, evidence bundle
- **실장비 우선 검증:** hardware gate를 acceptance 중심에 둔다
- **사후 학습:** incident response, postmortem, route tuning

즉, 리스크 관리의 목적은 실패를 완전히 없애는 것이 아니라 **실패를 작게 만들고 빨리 되돌리는 것**이다.

_Source:_ https://learn.microsoft.com/en-us/devops/operate/safe-deployment-practices, https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/safe-deployments, https://learn.microsoft.com/en-au/azure/well-architected/operational-excellence/incident-response

## Technical Research Recommendations

### Implementation Roadmap

1. **Metric reset:** 제품 합격 기준을 `same-capture preset-applied full-screen visible <= 2500ms`로 고정한다.
2. **Trace reset:** capture request부터 full-screen visible까지 correlation chain과 evidence bundle을 재정렬한다.
3. **Local lane prototype:** display-sized truthful artifact를 만드는 resident local lane을 feature flag 뒤에 둔다.
4. **Canary validation:** hardware canary에서 `replacementMs`, `fallback ratio`, `wrong-capture`, `fidelity drift`를 측정한다.
5. **Default decision:** local lane이 health gate를 통과하면 점진 확장하고, 미통과면 route를 내린다.
6. **Reserve experiment:** local lane으로도 목표를 닫지 못할 때만 edge appliance/remote renderer POC를 시작한다.

### Technology Stack Recommendations

- **기본 권장:** Rust host + Tauri sidecar + native/GPU full-screen close lane + capture-bound file truth
- **계속 유지할 것:** darktable-compatible truth/parity reference, route policy rollout artifact, evidence bundle
- **추가 권장:** OpenTelemetry-style traces/metrics/logs, GitHub Actions environments/artifacts
- **예비안:** gRPC/Protobuf remote renderer only if local route fails
- **비권장 기본안:** full microservices replatforming, broker-first hot path, darktable-only blocking close ownership

### Skill Development Requirements

- Windows native runtime and packaging
- GPU/render profiling and display-sized pipeline tuning
- contract/schema testing and sidecar boundary validation
- hardware-in-loop measurement and booth-safe diagnostics
- rollout operations, feature flags, incident response

### Success Metrics and KPIs

- **Primary KPI:** same-capture preset-applied full-screen visible `<= 2500ms`
- **Correctness KPIs:** wrong-capture `0`, wrong-session `0`, preset mismatch `0`
- **Stability KPIs:** fallback ratio, warm-state retention, route rollback success
- **Operational KPIs:** trace completeness, incident triage time, canary halt/rollback speed
- **Decision rule:** latency, correctness, fallback 안정성을 함께 만족해야 `Go`

---

## Research Synthesis

### Executive Summary

이번 종합 리서치의 결론은 명확하다. Boothy의 현재 과제는 `preview가 보이느냐`가 아니라, **촬영 직후 같은 캡처의 프리셋 적용 결과물을 24인치 가로 풀화면으로 2.5초 안에 보여주느냐**다. 이 기준으로 다시 보면, 지금 필요한 것은 현 blocking path의 추가 미세조정보다 `full-screen close`를 전용 경로로 분리하는 대체 architecture다. 공개 자료 기준으로 `WIC`의 preview/cache guidance, `darktable OpenCL`의 accelerated truth path, `OpenImageIO ImageCache`, `Tauri sidecar`, `Azure`의 점진 전환/안전 배포 패턴은 모두 이 방향을 뒷받침한다.

가장 유력한 해법은 `local native/GPU resident full-screen lane + display-sized preset-applied truthful artifact + capture-bound truth/evidence contract` 조합이다. 즉, 사용자가 봐야 하는 `full-screen close artifact`를 빠르게 만들고 올리는 경로를 짧게 유지하되, `darktable-compatible truth/parity path`는 fidelity oracle로 남겨 same-capture 정합성과 preset truth를 지키는 구조다. 이 판단은 공개 자료에 exact benchmark가 있는 것이 아니라, 여러 공식 문서가 공통적으로 권장하는 `preview cache`, `warm worker`, `path separation`, `progressive exposure`, `health-gated rollout` 패턴을 현재 제품 조건에 맞게 결합한 inference다.

전략적으로는 `로컬 경로를 끝까지 최적화한 뒤에도 목표를 못 닫을 때만 remote renderer/edge appliance를 연다`가 맞다. 따라서 제품 의사결정은 `local resident lane`을 feature flag 뒤에서 검증하고, `same-capture`, `preset fidelity`, `fallback stability`, `evidence completeness`를 모두 만족할 때만 확대하는 순서여야 한다.

**Key Technical Findings:**

- 현재 문제는 renderer 활성화 여부가 아니라 `same-capture preset-applied full-screen <= 2500ms` 미달이다.
- 가장 적합한 기본 구조는 `modular monolith + dedicated sidecar/native lane + anti-corruption adapter + branch-by-abstraction rollout`이다.
- `display-sized truthful artifact`와 `truth/parity path`를 분리해야 full-screen close와 fidelity를 동시에 지킬 가능성이 높다.
- `local IPC/file contract`가 hot path 기본값이며, `gRPC/Protobuf + health/mTLS`는 remote 예비안에 가깝다.
- `priority queue`, `bulkhead`, `throttling`, `feature flag`, `safe deployment`, `observability`가 이 과제의 실질적인 성공 패턴이다.

**Technical Recommendations:**

- 1순위는 `local native/GPU resident full-screen lane` 도입이다.
- `same-capture preset-applied full-screen visible <= 2500ms`를 유일한 제품 합격선으로 고정한다.
- `darktable-compatible truth/parity`와 `route policy/evidence bundle`은 계속 유지한다.
- rollout은 `shadow -> canary -> default` 순서로 진행하고, health gate 미통과 시 즉시 이전 trusted path로 복귀한다.
- local lane 실패가 반복 검증된 뒤에만 `remote renderer / edge appliance` POC를 시작한다.

### Table of Contents

1. Technical Research Introduction and Methodology
2. Technical Landscape and Architecture Analysis
3. Implementation Approaches and Best Practices
4. Technology Stack Evolution and Current Trends
5. Integration and Interoperability Patterns
6. Performance and Scalability Analysis
7. Security and Compliance Considerations
8. Strategic Technical Recommendations
9. Implementation Roadmap and Risk Assessment
10. Future Technical Outlook and Innovation Opportunities
11. Technical Research Methodology and Source Verification
12. Technical Appendices and Reference Materials

### 1. Technical Research Introduction and Methodology

#### Technical Research Significance

이 연구가 중요한 이유는 현재 병목이 단순한 엔진 성능 문제가 아니라, **사용자가 실제로 보는 결과의 도착 시간** 문제이기 때문이다. Microsoft `WIC` 문서는 preview/thumbnail과 prerendered preview cache를 별도 고려하라고 안내하고, Azure safe deployment guidance는 성능 목표가 명확한 기능을 점진적으로 노출하고 health model로 통제할 것을 권장한다. 이를 현재 Boothy 상황에 대입하면, 이번 과제는 기술 스택 비교보다 `제품 합격선을 만족하는 전용 close lane을 만들 수 있느냐`의 문제로 보는 편이 정확하다.  
_Technical Importance:_ `same-capture`, `preset-applied`, `full-screen`, `<= 2500ms`라는 네 조건을 동시에 만족해야 한다.  
_Business Impact:_ 이 지표를 닫지 못하면 촬영 직후 경험의 핵심 가치가 무너진다.  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/wic/-wic-rawguidelines-thumbnail-previews, https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/safe-deployments

#### Technical Research Methodology

이번 리서치는 내부 문서와 최신 공개 문서를 함께 사용했다. 내부에서는 current route policy, dedicated renderer contract, camera/session truth, hardware validation history를 기준선으로 삼았고, 외부에서는 `WIC`, `darktable`, `OpenImageIO`, `Tauri`, `Azure Architecture Center`, `GitHub Actions`, `Playwright`, `OpenTelemetry`, `gRPC` 공식 문서를 사용해 구조적 타당성을 교차 검증했다. 직접적인 public benchmark가 없는 제품별 SLA에 대해서는 **공식 패턴의 교차 해석 + 내부 검증 맥락**을 근거로 inference를 명시했다.  
_Technical Scope:_ architecture, integration, implementation, rollout, observability, performance, risk  
_Data Sources:_ official docs, open-source project docs, internal product contracts and validation artifacts  
_Analysis Framework:_ product acceptance 중심의 architecture fit analysis  
_Time Period:_ 2026-04-14 기준 최신 공개 자료  
_Technical Depth:_ 설계 결정을 내릴 수 있을 정도의 product-fit 수준  

#### Technical Research Goals and Objectives

**Original Technical Goals:** 사용자가 촬영 직후 같은 사진의 프리셋 적용 결과물을 24인치 모니터 가로 풀화면으로 2.5초 안에 보게 하는 현실적인 기술/아키텍처 후보를 찾고, booth-safe waiting, same-capture 정합성, preset fidelity를 유지하는 도입 경로를 비교한다.

**Achieved Technical Objectives:**

- `preview가 빠르다`와 `제품 합격`을 명확히 분리했다.
- local-first architecture와 remote reserve architecture의 경계를 정리했다.
- rollout, evidence, fallback, observability를 포함한 구현 프레임을 완성했다.
- public exact benchmark 부재 영역과 inference 영역을 분리했다.

### 2. Technical Landscape and Architecture Analysis

#### Current Technical Architecture Patterns

가장 적합한 기본 패턴은 `modular monolith + dedicated sidecar/native lane + anti-corruption adapter + branch-by-abstraction rollout`이다. Azure guidance는 모든 문제를 microservices로 풀 필요가 없다고 보고, anti-corruption layer는 외부 의미 체계를 코어로부터 격리하는 데 적합하다. AWS의 branch-by-abstraction은 새 경로를 병행 운영하면서 즉시 rollback 가능한 전환에 적합하다. 이를 Boothy에 적용하면, core booth flow는 짧게 유지하고 camera SDK, darktable/XMP, sidecar render semantics는 경계 바깥에 고립시키는 편이 맞다.  
_Dominant Patterns:_ modular monolith, sidecar boundary, anti-corruption adapter, progressive rollout  
_Architectural Evolution:_ blocking truth path 중심에서 dual-lane close path 중심으로 이동  
_Architectural Trade-offs:_ 구조 단순성보다 hot path isolation이 더 중요하다.  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/, https://learn.microsoft.com/en-us/azure/architecture/patterns/anti-corruption-layer, https://docs.aws.amazon.com/prescriptive-guidance/latest/modernization-decomposing-monoliths/branch-by-abstraction.html

#### System Design Principles and Best Practices

핵심 설계 원칙은 네 가지다. 첫째, `same-capture truth`를 절대 우선으로 둔다. 둘째, `full-screen close lane`은 background backfill, final export, diagnostics와 자원을 분리한다. 셋째, route policy와 feature flag를 통해 점진 노출과 즉시 복귀가 가능해야 한다. 넷째, 모든 promotion에는 evidence가 남아야 한다. Azure의 priority queue, bulkhead, throttling 패턴은 바로 이런 hot path 분리에 맞는다.  
_Design Principles:_ capture-bound truth, hot path isolation, progressive exposure, evidence-first operation  
_Best Practice Patterns:_ priority queue, bulkhead, throttling, health-gated rollout  
_Architectural Quality Attributes:_ latency, correctness, recoverability, traceability  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/priority-queue, https://learn.microsoft.com/en-us/azure/architecture/patterns/bulkhead, https://learn.microsoft.com/en-us/azure/architecture/patterns/throttling

### 3. Implementation Approaches and Best Practices

#### Current Implementation Methodologies

도입 방식은 `strangler + feature flags + safe deployment` 조합이 가장 적합하다. Strangler Fig는 기존 기능을 유지한 채 일부 기능만 새 경로로 대체하는 점진 전환을 설명하고, feature flags는 배포와 노출을 분리한다. 이 조합은 현재 trusted path를 유지하면서 새로운 full-screen lane을 `shadow -> canary -> default`로 키우는 운영 모델과 정확히 맞다.  
_Development Approaches:_ strangler rollout, branch-by-abstraction, feature-gated exposure  
_Code Organization Patterns:_ host-owned contracts, sidecar-native lane, anti-corruption boundary  
_Quality Assurance Practices:_ contract tests, user-visible E2E, hardware validation, evidence review  
_Deployment Strategies:_ progressive exposure with immediate rollback  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig, https://learn.microsoft.com/en-us/devops/operate/progressive-experimentation-feature-flags, https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/safe-deployments

#### Implementation Framework and Tooling

구현 도구는 제품 합격 증명을 지원해야 한다. `GitHub Actions`의 environments와 artifacts는 환경 게이트와 증적 보관에 적합하고, `Playwright`는 user-visible acceptance를 검증하는 데 적합하다. `OpenTelemetry`는 capture부터 visible까지 correlation chain을 정리하는 관측성 기반으로 쓸 수 있다.  
_Development Frameworks:_ Tauri host shell, native sidecar, GitHub Actions, Playwright  
_Tool Ecosystem:_ schema validation, evidence artifacts, traces/metrics/logs  
_Build and Deployment Systems:_ environment-gated CI, artifact retention, health-gated rollout  
_Source:_ https://docs.github.com/en/actions/concepts/workflows-and-actions/deployment-environments, https://docs.github.com/en/actions/tutorials/store-and-share-data, https://playwright.dev/docs/best-practices, https://opentelemetry.io/docs/concepts/

### 4. Technology Stack Evolution and Current Trends

#### Current Technology Stack Landscape

이 목표에서 핵심은 UI 프레임워크가 아니라 `decode/apply/render hot path`다. `Tauri sidecar`는 Rust host가 외부 네이티브 바이너리를 자연스럽게 붙일 수 있게 해 주고, `WIC`는 preview와 prerendered preview cache 패턴을, `OpenImageIO`는 image cache/tile access를, `darktable OpenCL`은 accelerated truth path를, `RawTherapee`는 preview/save path 분리의 일반성을 보여준다.  
_Programming Languages:_ Rust orchestration, C/C++ decode, GPU shader/OpenCL render  
_Frameworks and Libraries:_ Tauri sidecar, WIC, OpenImageIO, darktable-compatible path  
_Database and Storage Technologies:_ capture-bound artifacts, manifest, cache, evidence bundle  
_API and Communication Technologies:_ local IPC/file contract, optional gRPC/Protobuf reserve path  
_Source:_ https://v2.tauri.app/develop/sidecar/, https://learn.microsoft.com/en-us/windows/win32/wic/-wic-rawguidelines-thumbnail-previews, https://openimageio.readthedocs.io/en/stable/imagecache.html, https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/, https://rawpedia.rawtherapee.com/Toolchain_Pipeline

#### Technology Adoption Patterns

공개 자료가 공통적으로 가리키는 방향은 `preview cache`, `warm worker`, `separated path`, `incremental rollout`이다. 정확히 같은 제품을 위한 public benchmark는 없지만, 이 패턴 조합은 현재 Boothy의 목표와 가장 잘 맞는다. 따라서 adoption priority는 `local resident lane`이 가장 높고, `remote appliance`는 local lane 실패가 검증될 때만 의미가 있다. 이는 공식 문서의 운영/배포 패턴에 기반한 inference다.  
_Adoption Trends:_ local acceleration first, artifact-based preview, health-gated rollout  
_Migration Patterns:_ parallel path, shadow evidence, canary expansion, fallback rollback  
_Emerging Technologies:_ remote appliance, edge execution, richer observability correlation  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig, https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/safe-deployments, https://learn.microsoft.com/en-us/devops/operate/progressive-experimentation-feature-flags

### 5. Integration and Interoperability Patterns

#### Current Integration Approaches

hot path 기본값은 `host-owned local IPC + capture-bound file/evidence contract`다. same-host에서는 추가 네트워크 hop 없이 session/capture correlation을 유지하는 것이 가장 중요하기 때문이다. `gRPC/Protobuf`는 remote path가 필요한 경우에만 적합하며, health checking과 auth boundary를 함께 설계해야 한다.  
_API Design Patterns:_ local point-to-point contracts first, remote service contracts second  
_Service Integration:_ sidecar boundary locally, gRPC service boundary remotely  
_Data Integration:_ capture-bound manifest, append-only evidence, route policy artifacts  
_Source:_ https://grpc.io/docs/what-is-grpc/introduction/, https://grpc.io/docs/guides/health-checking/, https://learn.microsoft.com/en-us/azure/architecture/patterns/ambassador

#### Interoperability Standards and Protocols

프로토콜 선택 기준은 유연성이 아니라 `latency`, `correlation`, `operational simplicity`다. local lane에서는 named pipe/stdin-stdout/file contract가 충분하며, remote lane에서는 `gRPC + Protobuf + mTLS`가 가장 설득력 있다. 반대로 GraphQL, API gateway, service mesh, broker-first integration은 현재 부스 hot path에 비해 제어면과 복잡도가 과하다.  
_Standards Compliance:_ schema-validated contracts, explicit health model, secured remote transport  
_Protocol Selection:_ local IPC first, gRPC remote reserve, broker/event only for side workloads  
_Integration Challenges:_ wrong-capture 방지, truth promotion 권한 통제, fallback consistency  
_Source:_ https://grpc.io/docs/guides/auth/, https://datatracker.ietf.org/doc/html/rfc8705, https://learn.microsoft.com/en-us/azure/architecture/microservices/design/gateway

### 6. Performance and Scalability Analysis

#### Performance Characteristics and Optimization

공개 자료는 exact `same-capture preset-applied full-screen <= 2500ms` benchmark를 제공하지 않는다. 대신 `WIC`는 빠른 preview와 prerendered preview cache를, `darktable`은 accelerated interactive path와 fallback을, `OpenImageIO`는 efficient cached image access를 설명한다. 이를 종합하면, 목표에 가까워질 가능성이 가장 높은 방식은 `display-sized truthful artifact`를 빠르게 만들고 올리는 전용 resident lane을 두는 것이다. 이는 공식 문서의 공통 패턴을 현재 조건에 적용한 inference다.  
_Performance Benchmarks:_ 공용 exact benchmark는 부재하며 내부 hardware validation이 최종 판정 근거다.  
_Optimization Strategies:_ warm-state, cached artifact, full-screen lane isolation, fast promotion path  
_Monitoring and Measurement:_ capture -> decode -> apply -> visible correlation chain, evidence bundle, trace completeness  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/wic/-wic-rawguidelines-thumbnail-previews, https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/, https://openimageio.readthedocs.io/en/stable/imagecache.html

#### Scalability Patterns and Approaches

현재 단계에서 확장은 `많이 처리하기`보다 `먼저 처리하기` 문제다. priority queue와 bulkhead는 current capture close를 최우선으로 만들고, background work가 hot path를 잡아먹지 못하게 한다. booth fleet 차원의 scale-out은 remote appliance가 실제로 필요해졌을 때만 고려하면 된다.  
_Scalability Patterns:_ priority-based work separation, bulkhead isolation, throttled degradation  
_Capacity Planning:_ local warm-state 우선, expensive hardware validation 집중  
_Elasticity and Auto-scaling:_ remote appliance 도입 시점 이후에만 본격 고려  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/priority-queue, https://learn.microsoft.com/en-us/azure/architecture/patterns/bulkhead, https://learn.microsoft.com/en-us/azure/architecture/patterns/throttling

### 7. Security and Compliance Considerations

#### Security Best Practices and Frameworks

이번 과제의 보안 핵심은 외부 인증 체계보다 `누가 결과를 truth로 승격할 수 있는가`를 통제하는 데 있다. local lane에서는 process isolation, schema validation, allow-list path, capability gating이 중요하고, remote lane에서는 mTLS와 명시적 health/auth 경계가 필요하다.  
_Security Frameworks:_ least privilege, boundary validation, transport security for remote extension  
_Threat Landscape:_ wrong-session promotion, unsafe file boundary, unauthorized route changes  
_Secure Development Practices:_ signed artifacts, schema validation, route policy control, observability-backed audit  
_Source:_ https://learn.microsoft.com/en-us/azure/well-architected/security/, https://grpc.io/docs/guides/auth/, https://datatracker.ietf.org/doc/html/rfc8705

#### Compliance and Governance Considerations

이 과제에서의 governance는 규제 대응보다는 운영 통제에 가깝다. route policy, evidence bundle, promotion decision, rollback decision이 모두 재현 가능해야 한다. 즉, 어떤 경로가 어떤 기준으로 켜졌고 꺼졌는지를 문서와 증적으로 남기는 운영 거버넌스가 필요하다.  
_Industry Standards:_ 안전 배포, auditability, configuration control  
_Regulatory Compliance:_ 현재 공개 자료 기준으로 별도 산업 규제 요구보다는 운영 증적과 변경 통제가 핵심  
_Audit and Governance:_ route policy snapshots, evidence archives, incident/postmortem trail  
_Source:_ https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/safe-deployments, https://learn.microsoft.com/en-au/azure/well-architected/operational-excellence/incident-response

### 8. Strategic Technical Recommendations

#### Technical Strategy and Decision Framework

전략적 권고는 다섯 가지다. 첫째, 제품 합격선을 `same-capture preset-applied full-screen visible <= 2500ms`로 고정한다. 둘째, `local native/GPU resident full-screen lane`을 1순위로 구현한다. 셋째, `darktable-compatible truth/parity path`를 버리지 말고 fidelity oracle로 유지한다. 넷째, route policy와 feature flag로 progressive exposure를 운영한다. 다섯째, evidence와 observability 없이는 어떤 경로도 default로 승격하지 않는다.  
_Architecture Recommendations:_ local resident lane + truth/parity reference + evidence contract  
_Technology Selection:_ Rust host, native sidecar, GPU acceleration, image cache, contract-driven integration  
_Implementation Strategy:_ shadow, canary, default, rollback의 반복 검증  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig, https://learn.microsoft.com/en-us/devops/operate/progressive-experimentation-feature-flags, https://opentelemetry.io/docs/concepts/

#### Competitive Technical Advantage

이 방향의 경쟁력은 단순 성능이 아니라 `빠른데도 믿을 수 있는 결과`를 주는 데 있다. 썸네일이 빨리 뜨는 경험은 대체재가 많지만, 촬영 직후 같은 사진의 프리셋 적용 결과를 풀화면으로 빠르고 안정적으로 보여주는 경험은 차별화 포인트가 될 수 있다. 이 평가는 공개 자료의 preview/cache/fallback 패턴을 제품 경험 관점으로 해석한 inference다.  
_Technology Differentiation:_ truthful full-screen close with same-capture guarantees  
_Innovation Opportunities:_ resident lane tuning, preset-aware warm-state, stronger evidence UX  
_Strategic Technology Investments:_ local acceleration, hardware validation, rollout tooling  

### 9. Implementation Roadmap and Risk Assessment

#### Technical Implementation Framework

권장 구현 순서는 다음과 같다.

1. `Metric reset`: 제품 KPI와 증적 수집 기준을 `same-capture preset-applied full-screen visible <= 2500ms`로 재정렬한다.
2. `Trace reset`: capture부터 visible까지 correlation chain과 evidence bundle을 재정렬한다.
3. `Local lane prototype`: resident local lane을 feature flag 뒤에서 shadow 실행한다.
4. `Canary validation`: 실제 카메라와 실제 모니터에서 health gate를 검증한다.
5. `Default decision`: latency, correctness, fallback 안정성이 모두 맞을 때만 default 확대를 결정한다.
6. `Reserve experiment`: local lane이 반복적으로 실패할 때만 remote renderer/edge appliance POC를 시작한다.

_Implementation Phases:_ metric -> trace -> prototype -> canary -> default -> reserve  
_Technology Migration Strategy:_ branch-by-abstraction with fast rollback  
_Resource Planning:_ native/GPU, hardware validation, observability, rollout operations  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig, https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/safe-deployments

#### Technical Risk Management

주요 리스크는 `latency 미달`, `fidelity drift`, `wrong-capture`, `fallback instability`, `실장비-실운영 불일치`다. 완화책은 `progressive exposure`, `health gate`, `hardware-in-loop validation`, `strong telemetry`, `immediate rollback`이다. 특히 이번 과제에서는 CI 통과보다 실장비 기준 합격이 우선이다.  
_Technical Risks:_ local lane이 빨라도 truth를 잃을 수 있음  
_Implementation Risks:_ route policy/flag가 있어도 health signal이 약하면 잘못된 확대가 가능함  
_Business Impact Risks:_ 제품 신뢰 저하, 부스 운영 지연, 재촬영/응대 비용 증가  
_Source:_ https://learn.microsoft.com/en-us/devops/operate/safe-deployment-practices, https://learn.microsoft.com/en-au/azure/well-architected/operational-excellence/incident-response

### 10. Future Technical Outlook and Innovation Opportunities

#### Emerging Technology Trends

가까운 시기에는 `local acceleration + richer observability + stronger rollout control`이 가장 현실적인 발전 방향이다. 중기적으로는 booth fleet 규모와 운영 난이도에 따라 remote cell/appliance가 의미를 가질 수 있다. 장기적으로는 preset-aware precomputation, smarter warm-state retention, richer hardware profiling이 더 중요해질 가능성이 높다. 이 전망은 공식 문서가 직접 제시한 제품 roadmap이 아니라, 현재 패턴의 연장선에서 도출한 inference다.  
_Near-term Technical Evolution:_ local resident lane 완성도 향상  
_Medium-term Technology Trends:_ selective remote cell/appliance adoption  
_Long-term Technical Vision:_ stronger predictive warm-state and artifact orchestration  

#### Innovation and Research Opportunities

추가 연구 가치가 높은 영역은 세 가지다. `preset-aware artifact generation`, `GPU resident pipeline tuning`, `booth-safe operator diagnostics`다. 특히 local lane이 목표를 얼마나 안정적으로 닫는지에 따라 remote reserve 연구의 필요성이 결정된다.  
_Research Opportunities:_ truthful display artifact generation, fidelity drift detection, operator-facing evidence UX  
_Emerging Technology Adoption:_ remote renderer는 reserve track으로 유지  
_Innovation Framework:_ local-first hypothesis -> canary validation -> reserve expansion  

### 11. Technical Research Methodology and Source Verification

#### Comprehensive Technical Source Documentation

이번 리서치는 다음 유형의 자료를 사용했다.

- **내부 자료:** preview architecture reassessment, architecture draft, local dedicated renderer contract, camera/render worker protocol, hardware validation history
- **Primary Technical Sources:** Microsoft Learn (`WIC`, Azure patterns, safe deployments, security), Tauri docs, gRPC docs, Playwright docs, OpenTelemetry docs, GitHub Actions docs
- **Secondary Technical Sources:** OpenImageIO docs, darktable docs, RawTherapee docs, Adobe DNG/Camera Raw guidance
- **Representative Web Search Queries:** `WIC rawguidelines thumbnail previews`, `darktable OpenCL activate`, `OpenImageIO ImageCache`, `Tauri sidecar`, `Strangler Fig pattern`, `safe deployments`, `Playwright best practices`, `OpenTelemetry concepts`, `gRPC health checking`

#### Technical Research Quality Assurance

검증 기준은 `공식 문서 우선`, `중요 주장 다중 출처 확인`, `product-fit inference 명시`였다. exact public benchmark가 없는 부분은 문서에서 직접 증명된 사실과, 그 사실을 현재 제품 목표에 적용한 해석을 구분했다.  
_Technical Source Verification:_ 핵심 결론은 복수 공식 문서와 내부 계약 맥락을 함께 사용해 검증  
_Technical Confidence Levels:_ local resident lane 방향은 높음, remote appliance 필요성은 중간, exact achievable latency는 하드웨어 검증 전까지 중간  
_Technical Limitations:_ 제품별 exact benchmark와 vendor-neutral comparative test는 공개 자료에서 충분하지 않음  
_Methodology Transparency:_ conclusion은 명시적으로 inference와 sourced fact를 구분함  

### 12. Technical Appendices and Reference Materials

#### Detailed Technical Data Tables

| Option | Product Fit | Strength | Primary Risk | Recommendation |
| --- | --- | --- | --- | --- |
| Local native/GPU resident full-screen lane | High | 가장 짧은 hot path, same-host correlation 유지 | fidelity drift, local tuning 난이도 | Primary |
| Current truth-heavy blocking close path | Low | truth consistency 단순 | 2.5초 목표 미달 가능성 큼 | Do not extend as main strategy |
| Remote renderer / edge appliance | Medium | scale-out과 중앙 관리 가능성 | 운영 복잡도, network dependency | Reserve only |

| Decision Gate | Pass Condition | Fail Condition |
| --- | --- | --- |
| Latency | `<= 2500ms` 반복 충족 | 반복적으로 SLA 초과 |
| Correctness | wrong-capture `0`, preset mismatch `0` | any truth/capture mismatch |
| Stability | fallback controlled, warm-state stable | frequent fallback or unstable warm-state |
| Operations | trace/evidence complete | 증적 불충분, rollback slow |

#### Technical Resources and References

- Microsoft WIC RAW preview guidance: https://learn.microsoft.com/en-us/windows/win32/wic/-wic-rawguidelines-thumbnail-previews
- darktable OpenCL guidance: https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/
- OpenImageIO ImageCache: https://openimageio.readthedocs.io/en/stable/imagecache.html
- Tauri sidecar: https://v2.tauri.app/develop/sidecar/
- Azure Strangler Fig: https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig
- Azure priority queue: https://learn.microsoft.com/en-us/azure/architecture/patterns/priority-queue
- Azure bulkhead: https://learn.microsoft.com/en-us/azure/architecture/patterns/bulkhead
- Azure safe deployments: https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/safe-deployments
- Feature flags: https://learn.microsoft.com/en-us/devops/operate/progressive-experimentation-feature-flags
- GitHub Actions environments: https://docs.github.com/en/actions/concepts/workflows-and-actions/deployment-environments
- GitHub Actions artifacts: https://docs.github.com/en/actions/tutorials/store-and-share-data
- Playwright best practices: https://playwright.dev/docs/best-practices
- Playwright trace viewer: https://playwright.dev/docs/trace-viewer-intro
- OpenTelemetry concepts: https://opentelemetry.io/docs/concepts/
- gRPC health checking: https://grpc.io/docs/guides/health-checking/
- gRPC auth: https://grpc.io/docs/guides/auth/

## Technical Research Conclusion

### Summary of Key Technical Findings

이번 리서치가 내린 핵심 판단은 다음과 같다. Boothy의 현재 목표는 `빠른 preview`가 아니라 `same-capture preset-applied full-screen <= 2500ms`이며, 이를 위해서는 `local native/GPU resident full-screen lane`이 가장 적합하다. `truth/parity path`는 버리는 대상이 아니라 fidelity를 지키는 기준선으로 유지해야 하고, `remote renderer / edge appliance`는 local route가 반복적으로 실패한 뒤에만 검토하는 예비안이 맞다.

### Strategic Technical Impact Assessment

이 결론은 제품 전략에도 직접 연결된다. 앞으로의 평가는 기술 구성요소의 존재 여부가 아니라, 사용자가 촬영 직후 믿을 수 있는 결과를 2.5초 안에 보느냐로 수렴해야 한다. 따라서 roadmap, verification, rollout, incident handling 모두 이 단일 합격선에 맞춰 재정렬하는 것이 타당하다.

### Next Steps Technical Recommendations

1. `same-capture preset-applied full-screen <= 2500ms`를 제품의 유일한 sign-off KPI로 고정한다.
2. local resident lane prototype을 feature flag 뒤에서 shadow/canary로 검증한다.
3. health gate에 latency뿐 아니라 correctness, fallback stability, evidence completeness를 포함한다.
4. local lane이 실패할 때만 remote renderer/edge appliance reserve track을 연다.

---

**Technical Research Completion Date:** 2026-04-14
**Research Period:** 2026-04-14 current comprehensive technical analysis
**Document Status:** Complete
**Source Verification:** Official and primary technical sources prioritized
**Technical Confidence Level:** High for architecture direction, medium for exact latency attainment until hardware validation closes the loop

_This document is intended to guide product-level architectural decision-making for Boothy's post-capture preset-applied full-screen experience, not to justify the current path by default._
