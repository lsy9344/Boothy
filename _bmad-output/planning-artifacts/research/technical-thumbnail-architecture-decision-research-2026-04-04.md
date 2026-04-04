---
stepsCompleted: [1, 2, 3, 4, 5, 6]
inputDocuments:
  - C:/Code/Project/Boothy_thumbnail-latency-seam-reinstrumentation/_bmad-output/planning-artifacts/research/technical-thumbnail-architecture-decision-research-plan-2026-04-04.md
  - C:/Code/Project/Boothy_thumbnail-latency-seam-reinstrumentation/history/recent-session-thumbnail-speed-agent-context.md
  - C:/Code/Project/Boothy_thumbnail-latency-seam-reinstrumentation/history/recent-session-thumbnail-speed-brief.md
  - C:/Code/Project/Boothy_thumbnail-latency-seam-reinstrumentation/history/recent-session-thumbnail-speed-log-2026-04-04.md
workflowType: 'research'
lastStep: 6
research_type: 'technical'
research_topic: 'thumbnail latency next-phase architecture decision'
research_goals: '현재 히스토리와 최신 세션 근거를 바탕으로 truthful preset-applied preview close를 제품 목표 수준으로 줄일 수 있는 현실적인 구조 변화를 비교하고, Boothy에 가장 적합한 다음 단계 아키텍처를 추천한다.'
user_name: 'Noah Lee'
date: '2026-04-04'
web_research_enabled: true
source_verification: true
---

# Research Report: technical

**Date:** 2026-04-04
**Author:** Noah Lee
**Research Type:** technical

---

## Research Overview

본 리서치는 Boothy의 `same-capture first-visible`이 이미 약 `3초대`까지 내려온 반면, 고객이 실제로 기다리는 `preset-applied preview close`는 여전히 약 `6.4초 ~ 10초대`에 머무는 상황에서 시작되었다. 내부 세션 로그와 계약 문서, 그리고 현재 공개 기술 자료를 함께 검토한 결과, 남은 문제는 `같은 컷을 빨리 보이게 하는 것`이 아니라 `truthful close를 제품 목표 수준까지 더 줄일 수 있는 구조가 무엇인가`로 정리되었다.

이번 연구의 핵심 결론은 `현 darktable truth 계약을 유지한 채 host 뒤에 local dedicated renderer를 추가하는 방향`이 가장 현실적이라는 점이다. 이 방향은 현재 제품 계약을 가장 적게 깨면서도, canary 배포와 빠른 rollback, darktable fallback 공존, session 단위 seam 계측을 모두 유지할 수 있다. 반면 `watch-folder bridge`는 전술적 중간안, `edge appliance`는 장기 옵션으로는 의미가 있지만 지금 단계의 1차 권장안으로 보기에는 운영 부담이 크다.

이 문서는 위 판단을 뒷받침하는 기술 스택, 통합 패턴, 아키텍처 패턴, 구현 전략을 정리한 최종 의사결정 리서치다. 최종 결론과 실행 순서는 아래 `Executive Summary`와 `Research Synthesis`에서 한 번에 확인할 수 있다.

---

## Technical Research Scope Confirmation

**Research Topic:** thumbnail latency next-phase architecture decision
**Research Goals:** 현재 히스토리와 최신 세션 근거를 바탕으로 truthful preset-applied preview close를 제품 목표 수준으로 줄일 수 있는 현실적인 구조 변화를 비교하고, Boothy에 가장 적합한 다음 단계 아키텍처를 추천한다.

**Technical Research Scope:**

- Architecture Analysis - 현 darktable truth 계약을 유지하면서도 latency를 더 줄일 수 있는 구조 대안을 비교한다.
- Implementation Approaches - local dedicated renderer, watch-folder bridge, edge appliance 각각의 점진 전환 방식을 본다.
- Technology Stack - Rust/Tauri/React 기반 현재 앱 셸과, RAW 처리 엔진/sidecar/bridge/edge runtime 후보를 조사한다.
- Integration Patterns - session truth, same-slot replacement, diagnostics seam, fallback 호환성을 본다.
- Performance Considerations - latency 잠재력, fidelity 리스크, 운영 복잡도, 장애 복구 난이도를 함께 평가한다.

**Research Methodology:**

- Current web data with rigorous source verification
- Multi-source validation for critical technical claims
- Confidence level framework for uncertain information
- Comprehensive technical coverage with architecture-specific insights

**Scope Confirmed:** 2026-04-04

## Technology Stack Analysis

### Programming Languages

Boothy의 현재 제품 셸은 `Rust + Tauri + React/TypeScript` 조합을 유지하는 편이 가장 현실적이다. Rust 공식 사이트는 Rust를 성능 임계 서비스와 임베디드/저자원 환경에 적합한 언어로 설명하고 있으며, 메모리 안전성과 스레드 안전성을 강점으로 둔다. 이는 booth 하드웨어에서 sidecar, resident worker, file watcher, render orchestration을 다루는 현재 문제에 직접 맞는다. React 공식 문서는 최신 문서 기준 버전을 `19.2`로 안내하고 있어, 현재 리포의 React 19 계열 선택은 최신 메이저 흐름과도 어긋나지 않는다. 반면 RAW 처리 엔진 쪽은 여전히 C/C++ 기반 도구 생태계가 중심이다. darktable는 `darktable-cli`를 통해 헤드리스 export를 지원하고, LibRaw는 C++ API로 RAW decode를 제공하지만 공식 문서가 명시하듯 후처리 자체보다는 decode 중심이다. RawTherapee 역시 CLI를 제공해 배치 처리는 가능하지만, 제품 수준 저지연 recent-session preview용 전용 엔진으로 쓰기엔 프로파일/운영 경로 검증이 추가로 필요하다.

_Popular Languages: Rust, TypeScript, C/C++_
_Emerging Languages: 현재 이 문제에서는 새 언어 도입보다 Rust 집중이 더 현실적임_
_Language Evolution: UI는 React 19 계열 유지, host/worker는 Rust 강화, RAW 엔진은 기존 C/C++ 생태계 재사용이 자연스러움_
_Performance Characteristics: Rust는 저지연 orchestration에 적합하고, LibRaw는 decode 친화적이지만 full preset fidelity를 바로 대체하긴 어려움_
_Confidence: High for Rust/React current fit, Medium for replacing darktable with LibRaw-based custom processing_
_Source: https://rust-lang.org/_
_Source: https://react.dev/versions_
_Source: https://docs.darktable.org/usermanual/4.8/en/special-topics/program-invocation/darktable-cli/_
_Source: https://www.libraw.org/docs/API-overview.html_
_Source: https://rawpedia.rawtherapee.com/Command-Line_Options_

### Development Frameworks and Libraries

현재 코드베이스와 가장 잘 맞는 프레임워크 축은 `Tauri 2 + Rust sidecar/worker + React 19`다. Tauri 공식 문서는 외부 바이너리를 sidecar로 포함하고, capability 설정을 통해 `execute` 또는 `spawn` 권한을 명시적으로 부여하는 방식을 안내한다. 이는 Boothy가 이미 사용하는 helper/worker 모델과 정합적이다. 따라서 `local dedicated renderer` 후보는 현재 앱 셸을 버리기보다, Tauri 하에서 별도 저지연 렌더 바이너리를 sidecar로 승격하는 방식이 구현/배포 모두에서 가장 자연스럽다. 반대로 darktable의 Lua library mode는 공식 문서에서 `very experimental`이라고 경고하므로, 이를 next-phase 핵심 경로로 채택하는 것은 리스크가 크다. 파일 기반 bridge 옵션에서는 Rust의 `notify` crate처럼 플랫폼별 watcher를 추상화하고 poll watcher까지 제공하는 라이브러리가 실용적이며, watch-folder orchestration을 Rust 안에서 유지할 수 있다.

_Major Frameworks: Tauri 2, React 19, existing Rust host runtime_
_Micro-frameworks: Rust `notify`, dedicated sidecar binaries, CLI-based render tools_
_Evolution Trends: embedded desktop shell은 유지하고, render hot path만 sidecar/worker로 분리하는 방향이 가장 설득력 있음_
_Ecosystem Maturity: Tauri sidecar와 Rust watcher 생태계는 성숙, darktable Lua library mode는 실험적_
_Confidence: High for Tauri sidecar fit, Medium-High for Rust watcher fit, Low for darktable Lua embedding as production path_
_Source: https://v2.tauri.app/develop/sidecar/_
_Source: https://docs.rs/notify/latest/notify/_
_Source: https://docs.darktable.org/usermanual/4.0/en/lua/darktable-from-lua/_
_Source: https://react.dev/blog/2024/12/05/react-19_

### Database and Storage Technologies

Boothy의 canonical truth는 여전히 `session.json + session-scoped filesystem root`로 유지하는 편이 맞다. 이는 현재 계약과도 일치하고, preview/final asset의 same-path replacement를 가장 직접적으로 보장한다. 다만 다음 단계 구조가 `watch-folder bridge` 또는 `edge appliance`로 커질 경우, 파일만으로 job queue와 correlation을 모두 관리하면 분석성과 복구성이 떨어질 수 있다. SQLite 공식 문서는 application file format으로서 cross-platform, single-file, queryable 구조를 제공한다고 설명한다. 따라서 `current session truth` 자체를 DB로 옮기기보다는, bridge/appliance 내부의 `job queue / correlation index / retry ledger`를 SQLite로 두는 선택이 현실적이다. Docker bind mount 공식 문서는 host 파일을 컨테이너 안에 직접 노출할 수 있지만, host 경로 구조에 강하게 결합되고 write access 리스크가 있음을 분명히 한다. 따라서 watch-folder bridge를 컨테이너화할 경우에도 session asset root는 read-only 중심, write path는 제한된 handoff 디렉터리로 좁히는 설계가 필요하다.

_Relational Databases: SQLite는 bridge/appliance 내부 상태 저장에 적합_
_NoSQL Databases: 현재 문제에서는 우선순위가 낮음_
_In-Memory Databases: 현재 booth 단일 노드 문제에서는 필수성 낮음_
_Data Warehousing: 분석용으로는 별도 가치가 있으나 hot path 결정에는 비핵심_
_Confidence: High for filesystem truth retention, Medium-High for SQLite as auxiliary control-plane store, High for Docker bind-mount caution_
_Source: https://www.sqlite.org/appfileformat.html_
_Source: https://docs.docker.com/engine/storage/bind-mounts/_

### Development Tools and Platforms

현재 조사 범위에서 중요한 도구는 `darktable-cli`, `rawtherapee-cli`, Rust watcher, Tauri sidecar, 그리고 session diagnostics 체계다. darktable 공식 문서는 `darktable-cli`가 GUI 없이 순수 콘솔 모드에서 export를 수행하며 width/height, custom presets, output format 등의 옵션을 받는다고 설명한다. 즉 darktable는 여전히 fallback truth 엔진으로서 유지 가치가 있다. RawTherapee CLI도 헤드리스 batch 처리가 가능하고 Windows에서 콘솔을 띄우지 않는 `-w` 옵션까지 제공해 sidecar화 자체는 어렵지 않다. 다만 Boothy 관점에서는 둘 중 무엇이 더 빠른지가 아니라, 어떤 도구가 `preset fidelity`, `same-slot replacement`, `truth ownership`을 유지한 채 low-latency path를 형성할 수 있는지가 중요하다. 운영 플랫폼 측면에서는 Rust survey 2025 결과가 stable compiler와 release 추종이 중심이라고 확인해 주므로, production booth runtime도 nightly 의존보다 stable Rust 중심으로 두는 편이 맞다.

_IDE and Editors: Rust/TypeScript 양쪽 모두 표준 툴체인 성숙_
_Version Control: 현재 리포 구조와 BMAD artifact 흐름 유지 가능_
_Build Systems: Cargo + pnpm/Vite + Tauri 조합 유지가 자연스러움_
_Testing Frameworks: Rust integration tests와 Vitest 기반 UI/provider 회귀가 계속 핵심_
_Confidence: High for current toolchain continuity, Medium for RawTherapee as optional experimental tool_
_Source: https://docs.darktable.org/usermanual/4.8/en/special-topics/program-invocation/darktable-cli/_
_Source: https://rawpedia.rawtherapee.com/Command-Line_Options_
_Source: https://blog.rust-lang.org/2026/03/02/2025-State-Of-Rust-Survey-results/_

### Cloud Infrastructure and Deployment

이번 의사결정은 전형적인 cloud 확장보다 `booth-local` 또는 `booth-adjacent edge` 배치가 핵심이다. `Local dedicated renderer`는 현재 Windows booth 노드 내부에 sidecar 바이너리를 추가하는 방식이므로 추가 인프라가 가장 적다. `Watch-folder external render bridge`는 Docker 기반 패키징이 가능하지만, bind mount 제약 때문에 host 경로/권한/장애 전파를 세심히 설계해야 한다. `Edge render appliance`로 가면 K3s 같은 경량 Kubernetes 배포가 후보가 될 수 있다. K3s 공식 문서는 이를 edge, IoT, air-gapped 환경에 적합한 lightweight Kubernetes로 소개하며, 단일 binary와 sqlite3 기본 datastore를 제공한다고 설명한다. 다만 Boothy의 단일 booth 문맥에서는 이 옵션이 latency 잠재력은 크지만 운영 복잡도도 가장 크게 올린다. 따라서 현 단계에서 edge appliance는 `장기 옵션`으로 남기되, 즉시 실행 후보는 local sidecar 또는 constrained bridge가 더 현실적이다.

_Major Cloud Providers: 현재 리서치 주제에서는 비핵심_
_Container Technologies: Docker는 bridge 격리에 유용하지만 host coupling과 write-risk 관리가 필요_
_Serverless Platforms: booth 오프라인/저지연 요구와 맞지 않아 우선순위 낮음_
_CDN and Edge Computing: edge는 의미 있으나 현재는 CDN보다 booth-adjacent appliance 해석이 적절_
_Confidence: High for local-first priority, Medium for Docker bridge viability, Medium for K3s appliance as long-term option_
_Source: https://docs.docker.com/engine/storage/bind-mounts/_
_Source: https://docs.k3s.io/_

### Technology Adoption Trends

현재 기술 흐름은 Boothy에 두 가지 시사를 준다. 첫째, 프런트엔드/데스크톱 셸 쪽은 React 19와 Tauri 2처럼 최신 메이저 버전을 유지하면서도 hot path는 네이티브 sidecar로 분리하는 구조가 자연스럽다. 둘째, RAW 처리 쪽은 여전히 대체로 mature CLI/네이티브 라이브러리 생태계가 중심이며, 완전한 브라우저/JS 중심 처리로 이동한 상황이 아니다. darktable는 2026년 2월 기준 `5.4.1` bug-fix release가 나와 있고, 프로젝트 문서가 아직 5.4 기준으로 완전하지 않다고 명시하고 있다. 이는 darktable를 truth fallback로 유지하되, 그 위에 더 빠른 전용 recent-session path를 얹는 전략이 여전히 현실적이라는 신호다. Rust survey는 stable compiler 선호와 기업 채용 증가를 함께 보여 주므로, next-phase에서 Rust 기반 control plane을 더 강화하는 선택은 시장/운영 양쪽에서 무리가 적다.

_Migration Patterns: JS/desktop shell 유지 + native worker 강화_
_Emerging Technologies: edge-local orchestration, Rust-based control planes, containerized adjunct render services_
_Legacy Technology: per-capture cold CLI spawn 중심 구조는 점점 한계가 분명해짐_
_Community Trends: React 19 최신화, Rust stable 중심 사용, darktable는 지속 유지되지만 production embedding은 신중해야 함_
_Confidence: Medium-High overall_
_Source: https://react.dev/versions_
_Source: https://www.darktable.org/2026/02/darktable-5.4.1-released/_
_Source: https://blog.rust-lang.org/2026/03/02/2025-State-Of-Rust-Survey-results/_

## Integration Patterns Analysis

### API Design Patterns

Boothy의 현재 제품 경계에서는 `Tauri command + channel + sidecar process` 조합이 기본 API 패턴으로 가장 잘 맞는다. Tauri 공식 문서는 frontend에서 Rust를 호출할 때 typed `command`를 기본 원시 연산으로 제공하고, Rust에서 frontend로 갈 때는 event, channel, JavaScript evaluation을 제공한다고 설명한다. 또한 sidecar 실행은 별도 capability 허용이 필요해, local dedicated renderer를 도입하더라도 제어면을 현재 앱 권한 체계 안에 두기 쉽다. 반면 renderer가 별도 프로세스나 별도 노드로 분리될 경우에는 gRPC/protobuf 조합이 더 적합하다. gRPC 공식 문서는 proto로 서비스와 메시지를 정의하고, 동일 정의에서 client/server 코드를 생성할 수 있다고 설명한다. 따라서 `local dedicated renderer`는 Tauri command에서 sidecar를 제어하고, sidecar 내부 API는 얇은 local RPC 또는 stdio contract로 두는 방식이 적합하다. `edge appliance`는 명시적인 proto 계약 기반 gRPC API가 가장 자연스럽다. `watch-folder bridge`는 전통적 API보다 `drop request -> file watcher -> result handoff` 패턴이 핵심이므로 API richness보다 correlation discipline이 더 중요하다.

_RESTful APIs: 현재 주제에서는 1차 선택지가 아님. HTTP REST는 단순하지만 local hot path typed contract 측면에서는 이점이 제한적임_
_GraphQL APIs: 최근 booth thumbnail hot path에는 부적합_
_RPC and gRPC: cross-process 또는 edge 분리 시 가장 강한 typed contract 후보_
_Webhook Patterns: booth 내부보다는 edge integration 후 외부 상태 알림 용도에 더 적합_
_Confidence: High for Tauri command + sidecar as present fit, Medium-High for gRPC in appliance path_
_Source: https://v2.tauri.app/develop/calling-rust/_
_Source: https://v2.tauri.app/develop/calling-frontend/_
_Source: https://v2.tauri.app/develop/sidecar/_
_Source: https://grpc.io/docs/what-is-grpc/introduction/_

### Communication Protocols

통신 프로토콜 관점에서 세 옵션은 서로 다른 강점을 가진다. `local dedicated renderer`는 네트워크 프로토콜까지 끌어오지 않고, Tauri sidecar의 `spawn`과 stdin/stdout 이벤트 또는 Windows named pipe 같은 로컬 IPC로 닫는 편이 가장 가볍다. Microsoft 문서는 Windows IPC에서 named pipe를 공식 메커니즘으로 안내하며, 같은 컴퓨터 내 프로세스 간 통신에도 직접 사용할 수 있음을 보여 준다. `edge appliance`는 gRPC처럼 typed RPC와 TLS를 함께 가져갈 수 있는 프로토콜이 유리하다. gRPC 공식 문서는 인증 API와 SSL/TLS, 선택적 mutual auth를 기본 메커니즘으로 제공한다고 설명한다. `watch-folder bridge`는 사실상 파일시스템 이벤트가 프로토콜 역할을 맡는다. Rust `notify`는 플랫폼별 최적 watcher와 poll watcher를 함께 제공하므로, request/ready/error 상태 파일을 감시하는 bridge를 Rust 안에서 구현하기에 적합하다. 다만 파일 이벤트는 low-level transport일 뿐이라, 상위 레벨에서는 상태 전이 규약과 timeout 규약을 별도로 정해야 한다.

_HTTP/HTTPS Protocols: edge appliance 제어면에는 가능하지만 booth-local hot path에서는 우선순위 낮음_
_WebSocket Protocols: 지속 연결 UI stream에는 가능하지만 current thumbnail truth path의 1차 후보는 아님_
_Message Queue Protocols: booth 단일 노드에는 과하고, edge 다중 구성에서는 후보가 될 수 있음_
_gRPC and Protocol Buffers: appliance 또는 richer split-process 경계에서 가장 명확한 선택지_
_Confidence: High for local IPC/file-watch suitability, Medium-High for gRPC over TLS on edge path_
_Source: https://learn.microsoft.com/en-us/windows/win32/ipc/interprocess-communications_
_Source: https://learn.microsoft.com/en-us/dotnet/standard/io/how-to-use-named-pipes-for-network-interprocess-communication_
_Source: https://grpc.io/docs/guides/auth/_
_Source: https://docs.rs/notify/latest/notify/_

### Data Formats and Standards

데이터 포맷은 `customer/session truth`와 `worker/appliance contract`를 분리하는 편이 낫다. 현재 truth는 계속 `session.json`과 canonical preview/final asset path가 소유해야 한다. 이 계약은 내부적으로도 타당하다. 반면 renderer 제어 메시지까지 JSON 파일로만 밀어 넣으면 schema drift와 상관키 누락 위험이 커진다. Protocol Buffers 공식 문서는 이를 language-neutral, platform-neutral한 구조화 직렬화 메커니즘으로 설명하며, compact storage, fast parsing, generated bindings를 장점으로 든다. 따라서 `edge appliance` 또는 `local dedicated renderer`가 독립 프로세스 API를 가진다면, control-plane message는 protobuf가 더 적절하다. `watch-folder bridge`는 flat file 기반이 본질이므로 JSON manifest 또는 작은 request/result envelope가 현실적이다. 이벤트 표준이 필요할 경우 CloudEvents는 공통 이벤트 서술 포맷을 제공하므로 diagnostics relay나 edge event bus에 유용할 수 있다. 다만 booth 내부 hot path 전체를 CloudEvents로 바꾸는 것은 현재 문제에 비해 과하다.

_JSON and XML: current session truth와 file bridge metadata에는 JSON이 가장 실용적_
_Protobuf and MessagePack: typed control-plane contract에는 protobuf가 더 적합_
_CSV and Flat Files: watch-folder bridge에서는 파일 자체가 handoff carrier가 됨_
_Custom Data Formats: capture/request correlation과 replacement ownership을 위한 domain envelope는 필요_
_Confidence: High for JSON as truth/file metadata, High for protobuf as remote contract option_
_Source: https://protobuf.dev/overview/_
_Source: https://cloudevents.io/_
_Source: C:/Code/Project/Boothy_thumbnail-latency-seam-reinstrumentation/docs/contracts/session-manifest.md

### System Interoperability Approaches

시스템 상호운용성 측면에서 Boothy는 `point-to-point + bounded fallback`이 가장 중요하다. 현재 계약은 `session.json` truth와 same-slot replacement correctness를 중심으로 서 있으므로, 어떤 옵션이든 여러 중간 시스템이 truth owner가 되면 안 된다. `local dedicated renderer`는 현재 host가 orchestration을 계속 소유하면서 renderer를 단일 point-to-point sidecar로 붙이는 방식이라 가장 충돌이 적다. `watch-folder bridge`는 direct API coupling을 줄이는 대신, 파일 경로와 디렉터리 구조에 대한 강한 결합이 생긴다. Docker 문서가 bind mount가 기본적으로 host write access를 갖고 host directory structure에 강하게 묶인다고 경고하는 이유가 바로 이 부분이다. `edge appliance`는 booth와 별도 노드 사이의 명시적 service boundary를 만들 수 있어 관측성과 독립 배포에는 유리하지만, 분산 경계가 늘어나는 만큼 correlation과 failure ownership을 더 엄격히 설계해야 한다.

_Point-to-Point Integration: local dedicated renderer에 가장 적합_
_API Gateway Patterns: edge appliance로 갈 때 host를 thin client gateway로 둘 수 있음_
_Service Mesh: 현재 booth 단일 제품에는 과도함_
_Enterprise Service Bus: 현재 문제에는 부적합_
_Confidence: High for point-to-point local fit, Medium for bridge/appliance interoperability cost_
_Source: https://docs.docker.com/engine/storage/bind-mounts/_
_Source: https://v2.tauri.app/develop/sidecar/_

### Microservices Integration Patterns

엄밀히 말하면 현재 Boothy는 마이크로서비스 제품이 아니지만, `edge appliance` 옵션을 평가하려면 최소한의 분산 패턴은 검토해야 한다. 이 경우 필요한 것은 full microservices catalog가 아니라 `single renderer service + host orchestrator` 패턴이다. API gateway는 현재 booth host가 담당하고, renderer service는 내부 전용 서비스로 두는 구조가 적절하다. circuit breaker와 bounded retry는 특히 중요하다. 현재도 제품 계약상 renderer miss가 나와도 false-ready 없이 fallback 해야 하므로, edge path에서도 동일하게 `service fail -> darktable fallback -> truth preserved` 구조를 강제해야 한다. saga나 service mesh는 이 시점엔 과하다. K3s는 edge, IoT, air-gapped 환경을 겨냥한 lightweight Kubernetes이므로 장기적으로 appliance 운영 기반이 될 수 있지만, 지금 당장 필요한 것은 orchestration sophistication보다 latency와 운영 단순성이다.

_API Gateway Pattern: host orchestrator가 가장 자연스러운 gateway_
_Service Discovery: edge appliance에서만 의미가 크고 local renderer에는 불필요_
_Circuit Breaker Pattern: 모든 옵션에서 truth-preserving fallback과 직접 연결됨_
_Saga Pattern: 현재 single preview close 문제에는 과도함_
_Confidence: Medium-High_
_Source: https://docs.k3s.io/_
_Source: https://grpc.io/docs/guides/auth/_
_Source: C:/Code/Project/Boothy_thumbnail-latency-seam-reinstrumentation/docs/contracts/render-worker.md

### Event-Driven Integration

이벤트 주도 통합은 이번 문제에서 특히 중요하다. 이유는 단순 성능 개선보다도 `어느 경로가 close owner였는지`를 한 세션 안에서 닫아야 하기 때문이다. OpenTelemetry는 traces, metrics, logs를 공통 context로 상관시킬 수 있다고 설명하며, context propagation을 통해 프로세스 경계를 넘어 causal chain을 유지할 수 있다고 안내한다. 이는 Boothy의 `request-capture -> file-arrived -> fast-preview-visible -> preview-render-start -> capture_preview_ready -> recent-session-visible` seam과 잘 맞는다. 다만 current booth는 local file diagnostics 가치를 유지해야 하므로, OpenTelemetry full stack 도입보다 `trace/correlation 개념만 차용한 session-scoped event envelope`가 더 현실적이다. CloudEvents는 이벤트 데이터의 공통 서술 포맷을 제공하므로 edge path에서 event relay가 늘어날 경우 가치가 있다. `watch-folder bridge`는 사실상 filesystem event-driven model이고, `notify`는 이를 Rust 쪽에서 직접 처리할 수 있다.

_Publish-Subscribe Patterns: edge appliance나 diagnostics fan-out에서만 부분적으로 유효_
_Event Sourcing: 전체 세션 truth를 event sourcing으로 바꿀 필요는 없음_
_Message Broker Patterns: booth 단일 노드에는 과하고 edge 다중 booth 운영에서만 재검토 가치_
_CQRS Patterns: 현재 문제 해결에는 과도하지만 read-model diagnostics 분리에는 일부 아이디어 제공 가능_
_Confidence: High for correlation-first event design, Medium for broker-based expansion_
_Source: https://opentelemetry.io/docs/concepts/context-propagation/_
_Source: https://opentelemetry.io/docs/concepts/signals/_
_Source: https://cloudevents.io/_
_Source: https://docs.rs/notify/latest/notify/_

### Integration Security Patterns

보안 패턴은 옵션마다 다르게 중요해진다. `local dedicated renderer`에서는 Tauri capability와 sidecar allowlist가 핵심이다. 공식 문서는 sidecar 실행에 `shell:allow-execute` 또는 `shell:allow-spawn` 권한이 필요하다고 명시하므로, local renderer는 제품 내부 allowlist로 강하게 제한하는 편이 맞다. `edge appliance`에서는 gRPC의 TLS와 선택적 mutual auth가 직접적인 기본 선택지가 된다. gRPC 공식 문서는 SSL/TLS를 기본 내장 인증 메커니즘으로 두고, 클라이언트 인증서를 통한 상호 인증도 가능하다고 설명한다. 관측성 쪽에서는 OpenTelemetry가 외부 서비스로 context를 전달할 때 forged trace header, baggage 내 민감정보 전파 위험을 경고한다. 즉 Booth 내부 상관키는 유지하되, 외부 경계로 나갈 때는 민감한 session/customer 정보가 baggage나 이벤트 payload에 실리지 않도록 분리해야 한다.

_OAuth 2.0 and JWT: public multi-tenant API가 아니므로 우선순위 낮음_
_API Key Management: edge appliance 원격 제어면이 열릴 때만 제한적으로 검토_
_Mutual TLS: edge appliance 경계에서 가장 유효_
_Data Encryption: local file truth는 OS/volume 보안, remote control-plane은 TLS 강제_
_Confidence: High_
_Source: https://v2.tauri.app/develop/sidecar/_
_Source: https://grpc.io/docs/guides/auth/_
_Source: https://opentelemetry.io/docs/concepts/context-propagation/_

## Architectural Patterns and Design

### System Architecture Patterns

현재 Boothy의 next-phase에 가장 잘 맞는 시스템 패턴은 `modular monolith shell + dedicated renderer sidecar`다. Alistair Cockburn의 원문은 hexagonal architecture를 안쪽 애플리케이션과 바깥 장치/데이터베이스/인터페이스를 포트로 분리하는 방식으로 설명한다. 이는 Boothy가 지금 지키고 있는 `session truth`, `render truth`, `UI shell` 경계를 그대로 유지하면서, 느린 preview close 경로만 새 adapter로 치환하는 데 잘 맞는다. 반대로 full microservice 또는 appliance first 구조는 현 시점에 너무 큰 분산 경계를 도입한다. 점진 교체 전략으로는 Strangler Fig 패턴이 더 적절하다. AWS Prescriptive Guidance는 proxy layer를 두고 기존 시스템과 새 시스템 사이를 단계적으로 치환하라고 설명하며, 필요 시 anti-corruption layer를 둬서 semantics 오염을 막으라고 권장한다. Boothy에 그대로 옮기면, host가 proxy/orchestrator 역할을 유지하고, 기존 darktable truth lane과 새 renderer lane을 동시에 관리하는 구조가 된다.

_Architectural reading: `로컬 전용 렌더러`는 hexagonal + strangler 조합에 가장 잘 맞고, `watch-folder bridge`는 tactical bridge, `edge appliance`는 장기 분산 옵션에 가깝다._
_Confidence: High_
_Source: https://alistair.cockburn.us/hexagonal-architecture_
_Source: https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/strangler-fig.html_
_Source: https://learn.microsoft.com/en-us/azure/architecture/patterns/anti-corruption-layer_

### Design Principles and Best Practices

설계 원칙 측면에서 핵심은 `truth owner를 늘리지 말 것`, `새 경로가 기존 semantics를 오염시키지 않게 할 것`, `교체 가능성보다 관찰 가능성을 먼저 보장할 것`이다. Azure의 anti-corruption layer 패턴은 서로 다른 semantic을 가진 두 시스템 사이에 번역층을 두어 새 시스템의 설계가 legacy 제약에 오염되지 않도록 하라고 설명한다. 이는 Boothy에서 새 renderer를 도입하더라도 `previewReady`, `previewVisibleAtMs`, same-slot replacement contract를 host가 계속 canonical semantics로 유지해야 한다는 뜻이다. hexagonal 관점에서도 renderer는 port 뒤의 adapter여야지, session truth를 직접 소유하는 core가 되면 안 된다. 즉 새로운 구조가 필요하더라도 제품 핵심 규칙은 `core invariant 고정 + adapter 교체 가능` 쪽으로 가져가는 것이 맞다.

_Best-practice reading: renderer replacement는 가능하지만 session/capture truth semantics는 host core에 고정해야 한다._
_Confidence: High_
_Source: https://alistair.cockburn.us/hexagonal-architecture_
_Source: https://learn.microsoft.com/en-us/azure/architecture/patterns/anti-corruption-layer_
_Source: C:/Code/Project/Boothy_thumbnail-latency-seam-reinstrumentation/docs/contracts/render-worker.md

### Scalability and Performance Patterns

이번 문제에서 scalability는 전형적인 수평 확장이 아니라 `cold spawn 비용 제거`, `close ownership 단순화`, `duplicate render 방지`에 더 가깝다. circuit breaker 패턴은 remote service나 flaky renderer가 반복 실패할 때 빠르게 차단해 연쇄 비용을 막는 구조로 설명된다. 이는 edge appliance뿐 아니라 local dedicated renderer에도 적용 가능하다. 예를 들어 새 renderer가 일정 횟수 이상 실패하면 그 세션 또는 일정 기간 동안은 곧바로 darktable fallback으로 내려가게 할 수 있다. 또한 Strangler Fig 가이드가 말하듯 synchronous call은 timeout이 thread/resource 소모를 일으킬 수 있으므로, 시간이 오래 걸리는 close path를 새 경로로 옮길 때는 bounded retry와 immediate fallback을 함께 설계해야 한다. 즉 `빠른 경로가 이기면 채택, 아니면 즉시 truth fallback`이 next-phase 성능 패턴의 기본 형태다.

_Performance reading: 추가 단축의 핵심은 scale-out보다 hot path 단순화와 bounded failure control에 있다._
_Confidence: High_
_Source: https://microservices.io/patterns/cn/reliability/circuit-breaker.html_
_Source: https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/strangler-fig.html_
_Source: https://docs.aws.amazon.com/wellarchitected/2023-10-03/framework/rel_prevent_interaction_failure_idempotent.html_

### Integration and Communication Patterns

아키텍처 레벨에서 통합 패턴은 `host-proxy orchestrator`가 유지돼야 한다. Strangler Fig 문서가 설명하듯 proxy layer는 기존 시스템과 새 시스템 사이에서 요청을 적절한 대상으로 라우팅한다. Boothy에서는 바로 이 proxy가 booth host다. 이 host는 capture accepted 이후 어떤 close path가 owner가 될지 결정하고, 결과가 안전할 때만 canonical preview path에 반영해야 한다. new renderer가 local sidecar든 edge service든 host는 계속 `route + validate + promote + fallback`을 담당해야 한다. 따라서 next-phase 구조는 `renderer service 직접 호출형 UI`가 아니라 `UI -> host -> renderer adapter`를 유지하는 것이 맞다. 이 점에서 local dedicated renderer는 현재 구조와 가장 가까우며, edge appliance는 host가 사실상 API gateway + ACL 역할까지 함께 맡는 구조가 된다.

_Architecture reading: proxy/orchestrator는 유지하고 renderer만 뒤에서 교체하는 구조가 가장 안전하다._
_Confidence: High_
_Source: https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/strangler-fig.html_
_Source: https://learn.microsoft.com/en-us/azure/architecture/patterns/anti-corruption-layer_
_Source: https://v2.tauri.app/develop/sidecar/_

### Security Architecture Patterns

보안 아키텍처는 경계 수가 늘수록 더 중요해진다. local dedicated renderer는 현재 Tauri capability와 allowlist 경계 안에서 통제할 수 있어 가장 단순하다. edge appliance는 네트워크 경계를 새로 열기 때문에 TLS, 선택적 mTLS, 인증서 수명주기, trace header sanitization 같은 추가 책임이 생긴다. OpenTelemetry overview는 distributed trace가 프로세스, 네트워크, 보안 경계를 넘는 사건 집합이라고 설명한다. 즉 appliance 방향으로 갈수록 관측성은 좋아질 수 있지만, 동시에 그 컨텍스트를 안전하게 다루는 보안 설계가 필수다. 제품 관점에서는 booth 단일 노드 문제를 해결하기 위해 그 복잡도를 지금 당장 감수할 이유가 아직 약하다.

_Security reading: local sidecar가 가장 간단하고, appliance는 보안 설계 비용이 크게 늘어난다._
_Confidence: High_
_Source: https://grpc.io/docs/guides/auth/_
_Source: https://opentelemetry.io/docs/reference/specification/overview/_
_Source: https://v2.tauri.app/develop/sidecar/_

### Data Architecture Patterns

데이터 아키텍처는 `database per service` 같은 분산 데이터 패턴을 지금 바로 가져오기보다, `single source of truth + adjunct control plane`으로 유지하는 편이 낫다. AWS strangler guidance는 새 서비스가 자기 저장소를 가질 수 있지만, migration 중 동기화는 tactical solution일 뿐 eventual consistency를 만든다고 경고한다. Boothy에서 preview close truth는 eventual consistency가 되면 안 된다. 따라서 `session.json`과 canonical preview path는 계속 단일 소스로 두고, 새 renderer는 결과 후보를 제출하는 쪽에 머물러야 한다. `watch-folder bridge`가 특히 위험한 이유도 여기 있다. bridge가 자체 queue DB와 file state를 갖는 순간 truth가 이원화되기 쉽다. 따라서 bridge를 쓰더라도 그것은 control plane 보조 상태까지만 허용하고, customer truth는 계속 session root가 소유해야 한다.

_Data reading: 분산 데이터 소유보다 single truth 유지가 더 중요하다._
_Confidence: High_
_Source: https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/strangler-fig.html_
_Source: C:/Code/Project/Boothy_thumbnail-latency-seam-reinstrumentation/docs/contracts/session-manifest.md

### Deployment and Operations Architecture

운영 아키텍처 측면에서는 `single-booth deployable unit`을 유지할 수 있는지가 중요하다. local dedicated renderer는 현재 앱 배포 단위 안에 sidecar binary 하나를 추가하는 구조라 현장 운영 부담이 가장 적다. watch-folder bridge는 Docker나 별도 서비스로 격리할 수 있지만, host path coupling과 mount 설정 실패가 그대로 booth failure로 이어질 수 있다. edge appliance는 K3s 같은 lightweight Kubernetes로 구성할 수 있으나, 그만큼 separate node lifecycle, network health, certificate management, remote update, failover까지 운영 책임이 커진다. K3s가 edge/IoT/air-gapped 환경에 적합하다는 점은 장점이지만, 그것이 곧 Boothy 현재 문제에 대한 최적 해답이라는 뜻은 아니다. 현 시점에서 운영 아키텍처는 `local-first, strangler-friendly, rollback-easy`가 가장 좋은 기준이다.

_Operations reading: local sidecar > constrained bridge > edge appliance 순으로 운영 단순성이 높다._
_Confidence: High_
_Source: https://docs.k3s.io/_
_Source: https://docs.docker.com/engine/storage/bind-mounts/_
_Source: https://v2.tauri.app/develop/sidecar/_

## Implementation Approaches and Technology Adoption

### Technology Adoption Strategies

도입 전략은 `big bang 전환`보다 `progressive displacement`가 맞다. Google SRE의 canarying 가이드는 부분적이고 시간 제한된 배포를 통해 작은 blast radius로 새 릴리스를 검증하라고 설명한다. AWS blue/green 가이드도 병렬 환경을 두고 작은 비율의 트래픽으로 새 경로를 검증한 뒤 전환하는 방식을 권장한다. Boothy에서는 이 개념을 booth traffic 비율이 아니라 `preview close ownership 비율`에 적용하는 것이 자연스럽다. 예를 들어 일부 세션, 일부 preset, 일부 booth에서만 새 renderer를 먼저 켜고, 실패 시 즉시 darktable truth lane으로 rollback 하는 방식이다. 이 해석은 현재 제품의 truth contract와도 잘 맞는다. 따라서 `local dedicated renderer`는 feature-gated canary adoption이 가능하고, `watch-folder bridge`도 limited session routing으로 실험 가능하다. `edge appliance`는 canary 자체보다 먼저 운영 경계 구축 비용이 커서 초기 adoption 전략으로는 불리하다.

_Adoption reading: local dedicated renderer는 canary/progressive rollout에 가장 잘 맞고, edge appliance는 초기 adoption 비용이 큼._
_Confidence: High_
_Source: https://sre.google/workbook/canarying-releases/_
_Source: https://docs.aws.amazon.com/whitepapers/latest/blue-green-deployments/introduction.html_
_Source: https://openfeature.dev/docs/reference/intro/_

### Development Workflows and Tooling

개발 워크플로우는 `host core 유지 + adapter별 계약 검증 + rollout flag` 조합으로 가져가는 것이 맞다. OpenFeature는 vendor-neutral feature flagging API를 제공하고, hooks와 metadata를 통해 telemetry와 결합할 수 있다고 설명한다. 이 철학을 그대로 쓰지 않더라도, Boothy에서는 renderer route 선택을 feature flag처럼 다루는 것이 유용하다. 즉 `old darktable lane`, `new local renderer lane`, `forced fallback lane`을 운영 중에도 제어 가능하게 두는 편이 좋다. 또한 sidecar나 edge service를 도입할 경우 소비자 관점의 계약 테스트가 중요해진다. Pact 문서는 consumer-driven contract testing을 통해 consumer가 provider에 기대하는 상호작용을 JSON 계약으로 남긴다고 설명한다. Boothy에서는 UI가 아니라 host가 consumer 역할을 하게 되고, renderer adapter가 provider가 된다. 따라서 구현 흐름은 `contract -> adapter integration -> booth hardware canary` 순으로 가는 것이 적절하다.

_Workflow reading: feature-gated routing + contract testing + hardware canary의 3단계가 적절하다._
_Confidence: High_
_Source: https://openfeature.dev/docs/reference/intro/_
_Source: https://openfeature.dev/specification/sections/flag-evaluation_
_Source: https://docs.pact.io/_
_Source: https://docs.pact.io/pact_nirvana_

### Testing and Quality Assurance

테스트 전략은 `unit-only`도 `UI-only`도 아니다. 새 renderer 도입에서는 host-core correctness와 adapter 계약이 가장 중요하므로, 통합 테스트 비중이 커지는 것이 자연스럽다. Pact는 provider/consumer 간 계약 오해를 CI에서 미리 드러내는 데 적합하고, Google SRE canary 가이드는 실제 production-like 입력이 들어오기 전까지 모든 결함을 찾을 수 없다고 분명히 말한다. 따라서 Boothy의 검증 구조는 세 층으로 정리하는 편이 맞다.

- host truth regression: `previewReady`, `same-slot replacement`, `fallback ownership`, seam event chain
- adapter contract regression: request/result/error envelope, timeout, invalid output, idempotent retry
- booth hardware canary: 실제 세션 기준 `first-visible`, `preview close`, owner 분류, rollback 기준

OpenTelemetry는 trace/log correlation을 공식적으로 지원하므로, 구현 테스트에서도 `trace_id`, `capture_id`, `session_id` 같은 correlation key를 일관되게 남기는 것이 품질 게이트에 도움이 된다.

_Testing reading: 새 구조일수록 contract/integration/hardware canary 비중이 커져야 한다._
_Confidence: High_
_Source: https://docs.pact.io/implementation_guides/javascript/docs/consumer_
_Source: https://sre.google/workbook/canarying-releases/_
_Source: https://opentelemetry.io/docs/concepts/context-propagation/_
_Source: https://opentelemetry.io/docs/specs/otel/compatibility/logging_trace_context/_

### Deployment and Operations Practices

운영 실천 관점에서는 `canary -> validate -> rollback` 자동화가 핵심이다. Google SRE는 canarying이 defect를 작은 비용으로 빨리 찾게 해 주는 배포 보강책이라고 설명하고, rollback 가능성이 incident duration을 줄이는 핵심이라고 강조한다. AWS blue/green 가이드 역시 병렬 환경을 두고 트래픽을 되돌릴 수 있는 점을 주요 장점으로 든다. Boothy에 적용하면, 새 renderer 배포는 다음과 같은 운영 흐름을 갖는 편이 좋다.

1. binary/sidecar 병렬 포함
2. feature flag로 라우팅 비율 제어
3. approved booth session package로 자동 검증
4. threshold 미달 시 즉시 rollback 또는 forced fallback

이 패턴은 local dedicated renderer에 가장 잘 맞고, watch-folder bridge도 비슷하게 적용 가능하다. edge appliance는 node/network/dependency까지 함께 롤백 설계를 해야 하므로 초기 운영 난이도가 더 높다.

_Operations reading: binary 병렬 포함 + flag routing + instant fallback이 가장 현실적이다._
_Confidence: High_
_Source: https://sre.google/workbook/canarying-releases/_
_Source: https://docs.aws.amazon.com/whitepapers/latest/blue-green-deployments/introduction.html_
_Source: https://sre.google/workbook/configuration-design/_

### Team Organization and Skills

팀 역량 측면에서는 `renderer algorithm 역량`, `Rust/Tauri host 역량`, `hardware validation 역량` 세 가지가 필요하다. local dedicated renderer는 현재 팀의 Rust/Tauri 기반을 재사용하기 때문에 가장 skill-adjacent 하다. watch-folder bridge는 Rust watcher/ops/docker 능력이 더 필요하다. edge appliance는 여기에 더해 gRPC, TLS/mTLS, K3s or edge ops, remote incident handling까지 요구한다. 즉 현재 팀이 가장 빠르게 실험할 수 있는 것은 local sidecar path다. 또한 anti-corruption layer를 제대로 유지하려면 제품 계약을 이해하는 host owner가 반드시 계속 중심에 있어야 한다. renderer를 외부 팀이나 별도 프로세스로 분리하더라도 truth semantics ownership은 제품 코어 쪽에서 놓으면 안 된다.

_Skill reading: 현 팀 역량과 가장 가까운 선택지는 local dedicated renderer다._
_Confidence: Medium-High_
_Source: https://v2.tauri.app/develop/sidecar/_
_Source: https://grpc.io/docs/what-is-grpc/introduction/_
_Source: https://docs.k3s.io/_

### Cost Optimization and Resource Management

비용 관점에서 지금 중요한 것은 클라우드 청구보다 `현장 운영비 + 디버깅 비용 + rollback 비용`이다. local dedicated renderer는 새 바이너리 개발 비용은 들지만 추가 노드나 원격 제어면이 없어 운영비가 가장 낮다. watch-folder bridge는 컨테이너나 보조 프로세스를 더 두게 되므로 운영 복잡도 비용이 올라간다. edge appliance는 하드웨어, 네트워크, 원격 업데이트, 인증서, 장애 대응까지 더해 총비용이 가장 높다. 또한 Google SRE는 rollback과 hermetic configuration의 중요성을 강조하는데, 이는 비용 측면에서도 맞다. 되돌리기 어려운 구조일수록 incident 비용이 커지기 때문이다. 따라서 초기 비용-효율 관점에서도 local dedicated renderer가 우선이다.

_Cost reading: 초기 총비용은 local dedicated renderer가 가장 낮고, appliance가 가장 높다._
_Confidence: Medium-High_
_Source: https://sre.google/workbook/configuration-design/_
_Source: https://docs.docker.com/engine/storage/bind-mounts/_
_Source: https://docs.k3s.io/_

### Risk Assessment and Mitigation

리스크를 정리하면 다음과 같다.

- `local dedicated renderer`
  - 리스크: preset fidelity 차이, 새 adapter 버그, close owner 혼선
  - 완화: host promotion gate, forced darktable fallback, contract tests, booth canary
- `watch-folder bridge`
  - 리스크: file correlation 누락, mount drift, truth 이원화, timeout ambiguity
  - 완화: read-only mount, explicit request/result envelopes, queue DB는 보조 상태만 허용
- `edge appliance`
  - 리스크: 네트워크/인증서/노드 장애, 운영 복잡도 급증, incident blast radius 확대
  - 완화: mTLS, circuit breaker, local fallback 유지, staged rollout

이 중 현재 제품 리스크 대비 완화 비용이 가장 좋은 쪽은 local dedicated renderer다. 특히 canarying과 rollback을 빠르게 붙일 수 있다는 점이 크다.

_Risk reading: 리스크 대비 완화 효율은 local dedicated renderer가 가장 좋다._
_Confidence: High_
_Source: https://sre.google/workbook/canarying-releases/_
_Source: https://microservices.io/patterns/cn/reliability/circuit-breaker.html_
_Source: https://docs.aws.amazon.com/whitepapers/latest/blue-green-deployments/introduction.html_

## Technical Research Recommendations

### Implementation Roadmap

1. `Phase 0`
   session seam 계측을 완성하고, current darktable lane에서 cold-start와 steady-state baseline을 다시 고정한다.
2. `Phase 1`
   local dedicated renderer adapter를 Boothy host 뒤에 sidecar로 추가한다.
   이 단계에서 truth owner는 여전히 host이며, 새 renderer는 candidate result만 제출한다.
3. `Phase 2`
   feature-gated canary rollout을 preset/session/booth 단위로 시작한다.
   실패 시 강제 darktable fallback으로 즉시 되돌린다.
4. `Phase 3`
   booth hardware package에서 `preview close 50% 추가 단축` 가능성이 검증되면 점진 확대한다.
5. `Phase 4`
   local path가 목표를 못 맞추면 그때 `watch-folder bridge` 또는 `edge appliance`를 2차 후보로 재검토한다.

### Technology Stack Recommendations

- 1차 권장: `Rust/Tauri host 유지 + local dedicated renderer sidecar + darktable fallback`
- 보조 권장: `OpenTelemetry-style correlation discipline`을 session diagnostics에 적용
- 선택적 권장: renderer route를 feature flag처럼 제어할 수 있는 configuration layer 추가
- 보류 권장: `watch-folder bridge`, `edge appliance`는 1차 local path 검증 후 재평가

### Skill Development Requirements

- Rust host와 adapter contract 설계 역량
- renderer fidelity 평가와 booth-safe quality 기준 정리 역량
- booth hardware canary 운영과 rollback 판단 역량
- 필요 시 2단계에서 gRPC/protobuf, container/edge ops 역량 확장

### Success Metrics and KPIs

- same-capture `first-visible`이 아니라 `truthful preset-applied preview close`가 주 KPI
- `preview close p50/p95`
- first capture cold-start vs steady-state split
- close owner 비율: `new renderer` vs `darktable fallback`
- false-ready 0건
- cross-session leakage 0건
- session seam completeness 100%
- canary rollback mean time

## Executive Summary

현재 Boothy의 최근 세션 썸네일 문제는 더 이상 `first-visible` 확보 문제가 아니다. 내부 실측 기준으로 같은 컷을 rail에 처음 보이는 시간은 이미 약 `3초대`까지 내려왔지만, 고객이 실제로 기다리는 `truthful preset-applied preview close`는 여전히 `약 6.4초 ~ 10초대`에 남아 있다. 따라서 다음 결정은 미세 튜닝이 아니라, 어떤 구조 변화가 `truth 유지`, `same-slot replacement`, `fallback 안전성`을 깨지 않으면서 이 구간을 실질적으로 더 줄일 수 있는지에 대한 선택이어야 한다.

이번 리서치의 최종 권장안은 `Rust/Tauri host 유지 + local dedicated renderer sidecar + darktable fallback`이다. 이 방향은 현재 제품 코어가 `session truth`, `preview truth`, `promotion gate`를 계속 소유하면서도, 느린 close hot path만 새 renderer adapter로 분리할 수 있다. 또한 canary rollout, feature-gated routing, 계약 테스트, booth hardware rollback이 모두 가능해 실제 현장 검증과 실패 대응이 가장 쉽다.

`watch-folder based external render bridge`는 tactical bridge로는 검토 가치가 있지만, file correlation, mount drift, truth 이원화 리스크가 크다. `edge render appliance + thin client`는 장기적 확장성과 typed RPC, 보안, 분산 관측성 측면에서는 매력적이지만, 현재 문제를 해결하기 전에 운영·네트워크·인증서 관리 복잡도를 크게 올린다. 따라서 1차 실행안으로는 부적합하고, local path가 목표를 못 닫을 때의 2차 옵션으로 두는 편이 맞다.

**Key Findings**

- 현재 병목의 중심은 `pending first-visible -> truthful close` 구간이다.
- 현재 계약을 가장 적게 깨면서 구조 전환 효과를 얻는 방법은 `local dedicated renderer sidecar`다.
- 새 renderer는 truth owner가 아니라 `candidate result producer`여야 한다.
- 성공 기준은 `first-visible`이 아니라 `truthful preview close p50/p95`다.
- `watch-folder bridge`와 `edge appliance`는 즉시 실행안보다 2차 후보로 관리하는 편이 낫다.

**Top Recommendations**

- `local dedicated renderer`를 1차 구조 실험안으로 승인한다.
- 새 경로는 host 뒤에서만 동작하게 하고, host가 promotion/fallback/truth ownership을 계속 유지한다.
- renderer routing을 feature flag처럼 제어해 preset/session/booth 단위 canary를 가능하게 한다.
- hardware validation의 주 KPI를 `truthful preset-applied preview close`로 고정한다.
- local path 검증 실패 전에는 `watch-folder bridge`와 `edge appliance`로 점프하지 않는다.

## Table of Contents

1. Research Overview
2. Technical Research Scope Confirmation
3. Technology Stack Analysis
4. Integration Patterns Analysis
5. Architectural Patterns and Design
6. Implementation Approaches and Technology Adoption
7. Research Synthesis
8. Source Verification Summary
9. Conclusion

## Research Synthesis

### Decision Framing

이번 리서치의 질문은 단순했다.

- 현 구조를 더 다듬으면 목표를 닫을 수 있는가
- 아니면 구조 전환 승인이 필요한가

최신 내부 근거와 외부 기술 자료를 함께 보면, 현재 답은 `현 구조를 그대로 둔 미세 조정`이 아니라 `host는 유지하되 close hot path를 별도 renderer로 분리하는 구조 전환` 쪽에 가깝다. 다만 그 전환은 `full platform pivot`이 아니라 `same product contract, different renderer topology`여야 한다.

### Option Comparison

| Option | Latency Potential | Contract Fit | Rollout Safety | Ops Complexity | Overall Reading |
| --- | --- | --- | --- | --- | --- |
| Local dedicated renderer | 높음 | 매우 높음 | 매우 높음 | 낮음 | 1차 권장 |
| Watch-folder external bridge | 중간 | 중간 | 중간 | 중간~높음 | 2차 전술안 |
| Edge render appliance | 잠재력 높음 | 중간 | 낮음 | 매우 높음 | 장기 옵션 |

### Recommended Direction

권장 방향은 `Local dedicated renderer`다.

이유는 다음과 같다.

- 현재 `Rust/Tauri host + session truth + same-slot replacement` 구조를 유지할 수 있다.
- 새 renderer를 `host 뒤의 adapter`로 붙일 수 있어 semantic 오염을 줄일 수 있다.
- darktable fallback을 계속 남긴 채 canary와 rollback이 가능하다.
- session seam diagnostics를 유지한 상태로 close owner를 비교할 수 있다.
- 팀 역량과 현장 운영 방식에 가장 가깝다.

### Why The Other Options Are Deferred

`Watch-folder bridge`는 file-based handoff라 진입 장벽은 낮지만, customer truth와 control plane 상태가 섞이기 쉬워진다. mount 구조, timeout, correlation 누락 같은 운영 리스크도 상대적으로 크다.

`Edge appliance`는 분산 경계 설계, gRPC/protobuf, TLS/mTLS, remote update, node lifecycle까지 한 번에 열어야 한다. 장기적으로는 검토 가치가 있지만, 현재 Boothy의 immediate product problem을 해결하는 첫 실험으로는 비용이 크다.

### 30 / 60 / 90 Day Roadmap

**30일**

- per-session seam instrumentation을 완결한다.
- cold-start와 steady-state baseline을 다시 수집한다.
- local renderer adapter contract를 설계한다.
- darktable fallback promotion gate를 고정한다.

**60일**

- local dedicated renderer sidecar 프로토타입을 host 뒤에 붙인다.
- feature-gated renderer routing을 도입한다.
- contract test + host integration test + hardware canary 패키지를 연동한다.
- 일부 preset/session/booth에서 제한 rollout을 시작한다.

**90일**

- `truthful preview close p50/p95`가 목표에 근접하면 점진 확대한다.
- close owner 비율과 rollback 빈도를 함께 본다.
- 목표 미달 시 `watch-folder bridge` 또는 `edge appliance`의 2차 탐색 여부를 결정한다.

### Decision Gates

다음 단계 승인 기준은 아래처럼 두는 것이 적절하다.

- Gate 1: session seam completeness 100%
- Gate 2: false-ready 0건, cross-session leakage 0건
- Gate 3: local renderer canary에서 darktable fallback 대비 의미 있는 `preview close` 단축
- Gate 4: preset fidelity와 customer-facing quality 허용 범위 유지
- Gate 5: rollback이 수 분이 아니라 즉시 가능한 수준일 것

### Future Outlook

local dedicated renderer가 목표를 닫는다면, Boothy는 기존 앱 셸과 진실값 계약을 유지한 채 latency 구조만 개선하는 가장 비용 효율적인 경로를 확보하게 된다. 반대로 local path로도 목표를 못 닫는다면, 그 시점에는 `renderer topology` 자체가 아니라 `execution locality`를 바꿔야 한다는 증거가 되므로, 그때 edge appliance 검토가 더 설득력을 얻게 된다.

## Source Verification Summary

이번 리서치는 아래 종류의 근거를 함께 사용했다.

- 내부 제품 증거
  - recent session brief / agent context / speed log
  - render-worker contract
  - session-manifest contract
- 공식 기술 문서
  - Tauri sidecar and app boundary docs
  - darktable CLI / Lua docs
  - gRPC / protobuf docs
  - Docker bind mounts docs
  - K3s docs
  - OpenTelemetry docs
  - AWS / Azure architecture guidance
  - Google SRE canary guidance

Confidence summary:

- `local dedicated renderer` 1차 권장안: High
- `watch-folder bridge` tactical fallback candidate: Medium
- `edge appliance` long-term candidate: Medium
- `darktable를 즉시 대체하는 full engine swap`: Low-Medium

## Conclusion

이번 리서치의 최종 판단은 명확하다.

Boothy는 지금 `엔진을 당장 버릴지`를 결정할 단계가 아니라, `현재 제품 계약을 유지한 채 close hot path를 local dedicated renderer로 분리할지`를 결정할 단계에 있다. 이 방향은 현재 목표와 가장 가깝고, 실패했을 때도 가장 빨리 되돌릴 수 있다.

따라서 제품 차원의 다음 액션은 다음 한 줄로 요약된다.

`Local dedicated renderer sidecar를 1차 구조 실험안으로 승인하고, darktable fallback을 유지한 상태에서 session-scoped canary로 truthful preview close 단축을 검증한다.`

---

**Technical Research Completion Date:** 2026-04-04
**Research Period:** current comprehensive technical analysis
**Source Verification:** current public technical sources + current Boothy internal evidence
**Technical Confidence Level:** High for the primary recommendation
