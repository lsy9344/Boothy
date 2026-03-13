---
stepsCompleted:
  - 1
  - 2
  - 3
  - 4
  - 5
  - 6
inputDocuments:
  - 'docs/business_context/context.md'
  - '_bmad-output/planning-artifacts/prd.md'
  - 'docs/research-checklist-2026-03-07-boothy-greenfield.md'
  - 'docs/refactoring/research-codex.md'
workflowType: 'research'
lastStep: 6
research_type: 'technical'
research_topic: 'Boothy wrapping strategy efficiency for Host/UI reuse and camera boundary design'
research_goals: 'Determine whether wrapping existing assets is the most efficient implementation path for Boothy, identify where wrapper or adapter reuse is appropriate versus where a new boundary is required, and produce a recommendation aligned with the PRD and operating constraints.'
user_name: 'Noah Lee'
date: '2026-03-08'
web_research_enabled: true
source_verification: true
---

# Research Report: technical

**Date:** 2026-03-08
**Author:** Noah Lee
**Research Type:** technical

---

## Research Overview

This research evaluated whether wrapping existing assets is the most efficient implementation path for Boothy by testing that question against the project PRD, business-context material, and current primary-source technical documentation. The scope covered technology stack choices, local integration patterns, architectural boundaries, and implementation sequencing for a Windows booth runtime where customer reassurance, operator recovery, and session-folder handoff matter more than general-purpose desktop abstraction.

Across the analysis, the strongest pattern was selective modernization. The Host/UI donor path based on modern `React + Tauri + Rust` remains technically current and efficient to reuse, while the camera-facing reference material remains useful mainly as extraction input, not as a product base. Official guidance on Tauri sidecars and capabilities, Azure sidecar isolation, AWS anti-corruption and branch-by-abstraction patterns, and Adobe tethered capture behavior all converged on the same conclusion: keep customer and operator state in the host, keep device truth behind a narrow adapter boundary, and use the filesystem as the durable handoff layer.

The detailed evidence is captured in the step-by-step analysis below. The decision-ready summary, final recommendation, phased roadmap, and source-verification summary are consolidated in `## Research Synthesis`.

---

<!-- Content will be appended sequentially through research workflow steps -->

## Technical Research Scope Confirmation

**Research Topic:** Boothy wrapping strategy efficiency for Host/UI reuse and camera boundary design
**Research Goals:** Determine whether wrapping existing assets is the most efficient implementation path for Boothy, identify where wrapper or adapter reuse is appropriate versus where a new boundary is required, and produce a recommendation aligned with the PRD and operating constraints.

**Technical Research Scope:**

- Architecture Analysis - design patterns, frameworks, system architecture
- Implementation Approaches - development methodologies, coding patterns
- Technology Stack - languages, frameworks, tools, platforms
- Integration Patterns - APIs, protocols, interoperability
- Performance Considerations - scalability, optimization, patterns

**Research Methodology:**

- Current web data with rigorous source verification
- Multi-source validation for critical technical claims
- Confidence level framework for uncertain information
- Comprehensive technical coverage with architecture-specific insights

**Scope Confirmed:** 2026-03-08

## Technology Stack Analysis

### Programming Languages

The local RapidRAW fork already uses a modern Host/UI stack: TypeScript in the web layer and Rust in the Tauri backend (`reference/uxui_presetfunction/package.json`). React's official docs currently treat 19.2 as the latest documented version, and the React 19 release is stable. Rust 2024 is also an official edition, released with Rust 1.85.0. That means the Host/UI donor stack is current enough to wrap and selectively reuse without first paying a modernization tax.

The camera reference tells a different story. The local digiCamControl source still targets .NET Framework 4.0 across most projects, while the official digiCamControl download page currently lists Windows plus .NET Framework 4.8 as runtime requirements. For Boothy, that makes C# a reasonable language only for an isolated Windows camera adapter or extraction spike, not for the whole product surface.

_Popular Languages: TypeScript for Host/UI, Rust for local desktop backend and file pipeline, C# only for camera-adapter extraction._
_Emerging Languages: None are required to validate the wrapping path; adding another language would increase coordination cost._
_Language Evolution: React/Rust are current; the camera reference is operationally useful but technologically legacy._
_Performance Characteristics: Rust fits async file and pipeline work; C# is acceptable for a contained Windows camera service; TypeScript should remain the UI and orchestration layer only._
_Source: https://react.dev/versions ; https://react.dev/blog/2024/12/05/react-19 ; https://doc.rust-lang.org/edition-guide/rust-2024/index.html ; https://learn.microsoft.com/en-us/dotnet/core/porting/ ; https://digicamcontrol.com/download_

### Development Frameworks and Libraries

The strongest framework signal still favors wrapping on the Host/UI side. Tauri 2 officially supports calling Rust commands from the frontend, async commands for heavy work, event emission back to the webview, and channels for streamed data. It also documents a first-class sidecar model, but that model is specific: external binaries are executed through `@tauri-apps/plugin-shell` with explicit capabilities such as `shell:allow-execute` or `shell:allow-spawn`. That matters for Boothy because a wrapped camera engine is technically aligned with Tauri, but not with an ad hoc, unpermissioned process launch model.

WPF remains a capable Windows desktop framework, but Microsoft still describes it as Windows-only and XAML/code-behind centric. That keeps it viable as a legacy extraction environment, yet weak as the main UI direction if Boothy wants to preserve the existing web UI leverage from RapidRAW.

_Major Frameworks: React 19 plus Tauri 2 are the best fit for the booth Host/UI; WPF is acceptable only for legacy camera seams or temporary spikes._
_Micro-frameworks: The official Tauri plugin surface already covers shell, filesystem, SQL, store, and event patterns without introducing another shell._
_Evolution Trends: Tauri 2 is converging on capability-based desktop permissions; React 19 improves async UI workflows that are useful for state-driven booth UX._
_Ecosystem Maturity: The maintained plugin and IPC surface is broad enough for Boothy's wrapper scenario._
_Source: https://v2.tauri.app/develop/calling-rust/ ; https://v2.tauri.app/develop/calling-frontend/ ; https://v2.tauri.app/develop/sidecar/ ; https://react.dev/blog/2024/12/05/react-19 ; https://learn.microsoft.com/en-us/dotnet/desktop/wpf/overview/_

### Database and Storage Technologies

For this topic, storage is more important than databases. Adobe's tethered capture flow is explicitly session-folder based: the operator chooses a session name and destination folder, and Lightroom stores captured photos into that local structure. That matches Boothy's PRD and local research, which are both organized around session folders, latest-photo reassurance, and handoff by session identity.

Tauri's maintained plugins reinforce a filesystem-first conclusion. The official file system plugin covers file and directory access, the store plugin offers a persistent key-value file, and the SQL plugin exposes SQLite/MySQL/PostgreSQL through `sqlx`. The efficient wrapping implication is clear: keep the capture and result pipeline on local session folders first, use a small persistent store for booth state if needed, and add SQLite only when operator logs or KPI queries truly require relational queries.

_Relational Databases: SQLite is a sensible later addition for logs and metrics, not a prerequisite for wrapper validation._
_NoSQL Databases: Not justified by the current booth-local scope._
_In-Memory Databases: Not needed for the camera or handoff path._
_Data Warehousing: A future analytics export concern, not part of the first wrapping decision._
_Source: https://helpx.adobe.com/lightroom-classic/help/import-photos-tethered-camera.html ; https://v2.tauri.app/plugin/file-system/ ; https://v2.tauri.app/plugin/store/ ; https://v2.tauri.app/plugin/sql/_

### Development Tools and Platforms

The toolchain split should be intentional. The vendored RapidRAW fork already uses Vite, TypeScript, React 19, and Tauri packages, which gives Boothy a modern desktop-web toolchain with fast iteration. The official Tauri documentation also includes test guidance, plugin permissions, and a bundled external binary story, so the Host/UI side has a maintained developer experience.

The camera reference is a multi-project Windows toolchain. The public digiCamControl repository exposes many projects, including `CameraControl.Devices.Example`, `CameraControlCmd`, `Canon.Eos.Framework`, and `PhotoBooth`, which confirms the local documentation's claim that the reference is not a single narrow camera engine. Combined with the official Windows + .NET Framework runtime requirement, that means Boothy should localize Visual Studio/.NET Framework friction to a small adapter boundary instead of importing it into the whole build.

_IDE and Editors: Modern web/Rust tooling for Host/UI, Visual Studio only where camera extraction requires it._
_Version Control: A mono-repo can hold a modern host plus a small Windows adapter without architectural ambiguity._
_Build Systems: Vite/Tauri for the host; a separate C# build for the camera adapter if retained._
_Testing Frameworks: Contract tests and filesystem-pipeline tests matter more than framework-specific UI tests for the wrapper question._
_Source: https://github.com/dukus/digiCamControl ; https://digicamcontrol.com/download ; https://v2.tauri.app/develop/calling-rust/ ; https://v2.tauri.app/develop/sidecar/_

### Cloud Infrastructure and Deployment

Boothy's wrapping decision is fundamentally edge-desktop, not cloud-first. Adobe's tethered workflow requires a supported camera connected to the computer and writes into a local session destination. The PRD also fixes the initial runtime as a Windows booth PC. That means the critical deployment platform is the booth machine itself, where USB camera control, filesystem watching, preset application, and export handoff all occur locally.

Cloud services remain optional and secondary: logs, remote diagnostics, updates, and analytics can be added later, but they should stay out of the booth's capture-critical path. In other words, the cloud is a future control plane; the data plane stays local.

_Major Cloud Providers: Not a first-order choice for validating wrapper efficiency._
_Container Technologies: Poor fit for direct USB camera control and desktop shell integration._
_Serverless Platforms: Plausible only for later telemetry or reporting workflows._
_CDN and Edge Computing: The actual edge runtime is the booth PC, not a browser-only client._
_Source: https://helpx.adobe.com/lightroom-classic/help/import-photos-tethered-camera.html ; https://learn.microsoft.com/en-us/dotnet/desktop/wpf/overview/ ; https://v2.tauri.app/develop/sidecar/_

### Technology Adoption Trends

The adoption pattern that best fits the evidence is selective modernization. React 19 and Tauri 2 are current and maintained, so the RapidRAW-derived host stack can be reused without a speculative rewrite. By contrast, Microsoft's own guidance frames migration from .NET Framework to modern .NET as a distinct modernization effort, which reinforces the local conclusion that digiCamControl's value lies in extracted camera seams and example flows, not in wholesale adoption.

This leads to a concrete wrapping trend for Boothy: modernize where the product value already exists, and quarantine older vendor-facing code behind a narrow boundary. That is a stronger fit than either fully inheriting the old Windows stack or rebuilding every layer from zero.

_Migration Patterns: Modern host plus isolated legacy adapter is the highest-leverage path._
_Emerging Technologies: No additional technology is needed beyond the current Tauri/React/Rust surface._
_Legacy Technology: Full .NET Framework/WPF product adoption should be avoided._
_Community Trends: Official ecosystems are favoring permissioned plugins, async boundaries, and incremental modernization rather than monolithic rewrites._
_Source: https://react.dev/versions ; https://react.dev/blog/2024/12/05/react-19 ; https://learn.microsoft.com/en-us/dotnet/core/porting/ ; https://github.com/dukus/digiCamControl ; https://v2.tauri.app/develop/calling-rust/_

## Integration Patterns Analysis

### API Design Patterns

Boothy's efficient wrapper path is command-oriented, not service-mesh oriented. Tauri's official model already gives a clean split: frontend-to-backend calls should use commands, heavy work should be async, and streamed progress should use channels rather than generic events. That aligns directly with a booth contract such as `configureSession`, `capture`, `getDiagnostics`, `restart`, and `exportStatus`. The important implication is that the wrapper boundary should expose a small RPC-like surface without pretending the webview is the system of record.

gRPC remains a strong general-purpose RPC option. Official gRPC documentation emphasizes proto-defined services, binary transport, and high-performance point-to-point communication, while Microsoft positions it primarily for backend services. That makes gRPC a viable second-stage option only if Boothy ends up with a separate long-lived local camera service that needs a stricter typed contract across languages. For the first wrapping pass, it is heavier than necessary.

_RESTful APIs: Not the preferred local pattern; same-machine desktop wrapping does not benefit from introducing HTTP-first orchestration._
_GraphQL APIs: No fit for the booth boundary because the contract is command-and-state, not client-shaped data aggregation._
_RPC and gRPC: Strong option for a later dedicated camera service, but excessive for the first local wrapper iteration._
_Webhook Patterns: Not relevant inside a single booth runtime._
_Source: https://v2.tauri.app/develop/calling-rust/ ; https://v2.tauri.app/develop/calling-frontend/ ; https://grpc.io/docs/what-is-grpc/introduction/ ; https://learn.microsoft.com/en-us/dotnet/architecture/cloud-native/grpc_

### Communication Protocols

The official sources point to a clear protocol ladder. Inside the Tauri app, commands are the request-response path. For progress and ordered streaming, Tauri channels are explicitly recommended, while the event system is documented as unsuitable for low-latency or high-throughput use and limited to JSON-string payloads. That means a wrapped camera flow should not push high-frequency photo-transfer or preview bytes through ordinary events.

For an external binary, Tauri's sidecar model already supports spawning a child process, reading stdout-like events, and writing to stdin. This is the lightest practical protocol if Boothy keeps the camera adapter as a child process of the host. If the adapter becomes a long-lived Windows-local service, Microsoft documents named pipes as duplex, message-based, and multi-client capable, whereas anonymous pipes are lower overhead but one-way and mainly suited to parent-child communication. So the choice is straightforward: sidecar stdio first, named pipes only if the camera process must outgrow the parent-child shape.

_HTTP/HTTPS Protocols: Unnecessary for the first local wrapper because Tauri already provides in-app IPC and sidecar execution._
_WebSocket Protocols: Also unnecessary unless the camera adapter evolves into a standalone local daemon with multiple subscribers._
_Message Queue Protocols: Overbuilt for a single booth PC and would add durability complexity where filesystem contracts already exist._
_gRPC and Protocol Buffers: Valuable only when a dedicated typed service boundary is justified enough to pay the service-hosting cost._
_Source: https://v2.tauri.app/develop/calling-rust/ ; https://v2.tauri.app/develop/calling-frontend/ ; https://v2.tauri.app/develop/sidecar/ ; https://learn.microsoft.com/en-us/dotnet/standard/io/pipe-operations ; https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipes ; https://learn.microsoft.com/en-us/dotnet/architecture/cloud-native/grpc_

### Data Formats and Standards

The integration boundary should keep control data and image data separate. Tauri events are JSON-string based, which is acceptable for small state changes but not for bulky or high-rate payloads. Channels are better for ordered streamed bytes. gRPC uses protobuf as its native IDL and payload format, which is efficient but only worthwhile if Boothy intentionally adopts a service definition workflow.

The durable truth for captured photos should stay on disk. Adobe's tethered capture flow is explicit that a capture session is centered on a session folder and destination. That is the natural interoperability contract for Boothy too: commands carry small identifiers such as `sessionId` and `requestId`, while image files arrive through a filesystem handoff. The efficient pattern is to keep photo bytes out of JSON IPC and correlate captures through filenames or a small manifest.

_JSON and XML: JSON is acceptable for control messages; XML adds no value here._
_Protobuf and MessagePack: Protobuf is useful only if Boothy formalizes a dedicated RPC service boundary._
_CSV and Flat Files: Not relevant for live capture control, though exports and logs can still use flat files later._
_Custom Data Formats: A tiny manifest or metadata file beside the session folder is justified to correlate capture requests with file arrival._
_Source: https://v2.tauri.app/develop/calling-frontend/ ; https://v2.tauri.app/develop/calling-rust/ ; https://grpc.io/docs/what-is-grpc/core-concepts/ ; https://helpx.adobe.com/lightroom-classic/help/import-photos-tethered-camera.html_

### System Interoperability Approaches

The most efficient interoperability model is point-to-point with a hard boundary. The host should talk to exactly one camera adapter boundary, and the adapter should own camera truth. Azure's API gateway guidance is useful here as a contrast case: gateways are valuable when clients must navigate many services and cross-cutting concerns. Boothy does not have that problem inside one booth. A thin local façade inside the Tauri backend is enough.

Adobe's tethered documentation reinforces the same conclusion from the opposite side. Lightroom's capture flow depends on supported cameras, a detection step, and UI-driven tether controls, but still centers around session and destination folders. That means wrapping Lightroom or Canon behavior is most stable when the integration boundary is placed at the process and filesystem level, not by letting booth UI reason about Lightroom window state.

_Point-to-Point Integration: Best fit for Boothy; one host, one camera adapter, one filesystem contract._
_API Gateway Patterns: Useful only as a conceptual reminder to keep one façade, not as a literal extra service._
_Service Mesh: No fit for the booth-local runtime._
_Enterprise Service Bus: No fit for the booth-local runtime._
_Source: https://learn.microsoft.com/en-us/azure/architecture/microservices/design/gateway ; https://helpx.adobe.com/lightroom-classic/help/import-photos-tethered-camera.html ; https://helpx.adobe.com/lightroom-classic/kb/tethered-camera-support.html ; https://v2.tauri.app/develop/sidecar/_

### Microservices Integration Patterns

Most microservices patterns should be borrowed selectively, not installed wholesale. The useful idea is not "split Boothy into services," but "treat the camera adapter as a fault-prone dependency with an explicit boundary." Microsoft's circuit breaker guidance is directly relevant: when a dependent service or resource fails repeatedly, block repeated attempts until recovery is likely. That maps cleanly to Boothy's `준비 중` and `전화 필요` operator states.

By contrast, patterns such as full API gateway stacks, service discovery, or saga orchestration are signs of overreach for a single-machine booth runtime. CQRS is mildly relevant only in the sense that Boothy's write path (`capture`, `delete`, `restart`) and read path (`latest photo`, `diagnostics`, `operator state`) should be modeled separately. That separation can be achieved inside one app without a distributed system.

_API Gateway Pattern: Keep only the thin façade idea inside the host backend._
_Service Discovery: Unnecessary because process endpoints are fixed and local._
_Circuit Breaker Pattern: Strong fit for repeated camera faults and recovery windows._
_Saga Pattern: Not justified for the first design because the booth workflow is local and sequential, not a distributed transaction._
_Source: https://learn.microsoft.com/en-us/azure/architecture/patterns/circuit-breaker ; https://learn.microsoft.com/en-us/azure/architecture/patterns/cqrs ; https://learn.microsoft.com/en-us/azure/architecture/microservices/design/gateway ; https://learn.microsoft.com/en-us/dotnet/architecture/cloud-native/distributed-data_

### Event-Driven Integration

Event-driven thinking helps, but a brokered event architecture does not. Microsoft's event-driven architecture guidance emphasizes decoupled producers and consumers with near-real-time response. Tauri also distinguishes between low-volume events and higher-throughput channels. For Boothy, that suggests a narrow internal event model: the camera boundary emits `capture_accepted`, `transfer_started`, `transfer_completed`, `camera_fault`; the host consumes those and translates them into booth UI state.

The crucial limit is scope. Boothy should use event-driven integration inside the app boundary, not as justification for installing a broker, queue, or event bus. The local workflow has one producer and a few consumers. That is small enough to keep eventing in-process and let the filesystem remain the durable handoff layer.

_Publish-Subscribe Patterns: Useful inside the host for decoupling UI updates from camera status signals._
_Event Sourcing: Too heavy for the first implementation path; append-only operational logs are enough._
_Message Broker Patterns: Not justified for a single booth machine._
_CQRS Patterns: Helpful as a local modeling principle, not as a distributed platform choice._
_Source: https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/event-driven ; https://learn.microsoft.com/en-us/azure/architecture/patterns/cqrs ; https://v2.tauri.app/develop/calling-frontend/ ; https://v2.tauri.app/develop/calling-rust/_

### Integration Security Patterns

The cheapest correct security model is local capability control plus process boundary restrictions. Tauri's runtime authority enforces which window can access which command and injects scopes at runtime. That is a better fit for Boothy than introducing OAuth or JWT into a same-machine capture path. If the app spawns a sidecar, permissions must be explicitly granted through Tauri capabilities. That keeps the camera wrapper from becoming an uncontrolled shell escape hatch.

If Boothy uses named pipes on Windows, Microsoft documents that pipe access is controlled by security descriptors and ACLs. In other words, even the fallback local-service path can remain OS-level secured without importing web auth machinery. Network-oriented measures like mTLS or token-based auth become relevant only if the camera service stops being local.

_OAuth 2.0 and JWT: Not recommended for the booth-local wrapper boundary._
_API Key Management: Also unnecessary inside one packaged desktop runtime._
_Mutual TLS: Reserve for a future networked service, not the first local adapter._
_Data Encryption: Keep sensitive operator configuration at rest and rely on OS/process boundaries for local transport; image payloads should flow by filesystem rather than ad hoc shell strings._
_Source: https://v2.tauri.app/security/runtime-authority/ ; https://v2.tauri.app/develop/sidecar/ ; https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights ; https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-open-modes_

## Architectural Patterns and Design

### System Architecture Patterns

The official architecture guidance keeps pointing to the same seam: Boothy should not become a distributed system, but it also should not collapse camera control, booth UI, and filesystem processing into one undifferentiated runtime. Microsoft's architecture style guidance describes monoliths as operationally simpler but also warns that architectural style choices determine scalability, deployment shape, and coupling boundaries. For Boothy, the best reading is a modular desktop host with one isolated sidecar-style camera boundary. Azure's sidecar pattern is especially relevant because it is explicitly designed to isolate cross-cutting or platform-specific capabilities in a colocated helper process with low-latency communication.

That gives Boothy a concrete target shape: a state-driven desktop host as the primary runtime, an isolated camera adapter process for Canon or Lightroom-facing behavior, and a filesystem-based result handoff. This is not microservices. It is a bounded modular monolith plus a narrowly scoped sidecar.

_Source: https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/ ; https://learn.microsoft.com/en-us/azure/architecture/patterns/sidecar ; https://learn.microsoft.com/en-us/azure/architecture/guide/design-principles/_

### Design Principles and Best Practices

The strongest design principle here is "translate, do not leak." AWS's anti-corruption layer pattern describes a translation layer that prevents a legacy model from polluting a new domain model. That is almost a literal description of what Boothy needs between old Lightroom or camera-control behavior and the new booth state model from the PRD. The host should speak in customer-visible states, session IDs, export progress, and operator diagnostics. The camera boundary can keep device details, SDK failure semantics, and transport quirks hidden behind that translation layer.

The other principle that matters is simplicity under change. Azure's design principles explicitly emphasize keeping things simple, minimizing coordination, and designing for evolution. For Boothy that means the UI should not learn window-focus heuristics, popup states, or Canon SDK object graphs. A small translation boundary is simpler and far easier to replace later.

_Source: https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/anti-corruption-layer.html ; https://learn.microsoft.com/en-us/azure/architecture/guide/design-principles/_

### Scalability and Performance Patterns

Boothy's real scaling problem is not cloud throughput. It is local fault isolation, deterministic booth responsiveness, and predictable recovery. Azure's reliability and performance guidance frames architecture quality around failure containment and resource efficiency. That maps directly to Boothy's needs: if camera control stalls, the host should remain responsive; if export is delayed, the UI should still render accurate state; if a capture is accepted but file transfer lags, the filesystem pipeline should absorb that gap without freezing the customer flow.

This is another reason a sidecar-style camera process is attractive. It creates a natural blast-radius boundary. The host can restart, trip a circuit-breaker-like guard, or switch to `전화 필요` without bringing down the entire booth runtime.

_Source: https://learn.microsoft.com/en-us/azure/well-architected/reliability/ ; https://learn.microsoft.com/en-us/azure/well-architected/performance-efficiency/ ; https://learn.microsoft.com/en-us/azure/architecture/patterns/sidecar ; https://learn.microsoft.com/en-us/azure/architecture/patterns/circuit-breaker_

### Integration and Communication Patterns

From an architectural perspective, integration should be asymmetric. The host owns the product workflow. The sidecar owns device truth. The filesystem owns durable capture results. That separation is more important than the exact transport. Azure's sidecar pattern and the anti-corruption layer pattern together imply the right shape: colocated communication for low latency, plus semantic translation before state reaches the product layer.

This means Boothy should avoid letting the customer UI become an orchestration peer of the camera system. The Tauri backend or an equivalent host layer should be the only façade that the UI talks to. Internally, that façade can call commands, sidecar stdin/stdout, channels, or named pipes as needed.

_Source: https://learn.microsoft.com/en-us/azure/architecture/patterns/sidecar ; https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/anti-corruption-layer.html ; https://v2.tauri.app/develop/calling-rust/ ; https://v2.tauri.app/develop/sidecar/_

### Security Architecture Patterns

The right security posture is local least privilege, not network-grade ceremony. Tauri's runtime authority model and capability-based permissions are already a strong architectural fit because they limit which commands and shell privileges a given window can access. This matters more than OAuth-style machinery inside a same-machine booth runtime. If the host needs to launch a sidecar, the capability should be explicit. If the camera process listens over named pipes later, pipe ACLs should restrict access to the local packaged app context.

The architectural point is that security belongs at the boundary. Once the UI can arbitrarily spawn or control system-level processes, the wrapper design has already failed.

_Source: https://v2.tauri.app/security/runtime-authority/ ; https://v2.tauri.app/develop/sidecar/ ; https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights_

### Data Architecture Patterns

The PRD and Adobe tethering model both support the same data architecture: session-centric local folders as the primary source of truth for capture artifacts. That suggests a layered data model rather than a generalized data platform. Layer 1 is ephemeral control state in memory. Layer 2 is durable session artifacts on disk. Layer 3 is optional operator logs or KPIs in a small local store such as SQLite.

This is architecturally important because it avoids coupling booth success to a relational database or a remote service. The customer outcome can complete even if analytics logging is degraded.

_Source: https://helpx.adobe.com/lightroom-classic/help/import-photos-tethered-camera.html ; https://v2.tauri.app/plugin/store/ ; https://v2.tauri.app/plugin/sql/_

### Deployment and Operations Architecture

The operationally efficient deployment pattern is one packaged booth app plus one optional colocated camera helper. Azure's sidecar pattern again fits because it keeps the helper deployed with the parent application and localized to the same host. That preserves low coordination cost and matches Boothy's single-PC store model. It also creates a clean path for incremental replacement: the host package can remain stable while the camera helper changes.

AWS's branch by abstraction pattern strengthens the same conclusion from a modernization angle. It recommends introducing an abstraction layer so old and new implementations can coexist behind the same contract, enabling gradual switchover and rollback. For Boothy, that means the host contract should be fixed early, while the implementation behind the camera boundary can move from wrapped legacy behavior to a cleaner Canon-focused adapter over time.

_Source: https://learn.microsoft.com/en-us/azure/architecture/patterns/sidecar ; https://docs.aws.amazon.com/prescriptive-guidance/latest/modernization-decomposing-monoliths/branch-by-abstraction.html ; https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/anti-corruption-layer.html_

## Implementation Approaches and Technology Adoption

### Technology Adoption Strategies

The most efficient adoption strategy is incremental replacement behind a stable boundary, not a whole-stack migration. AWS's branch by abstraction pattern is the closest formal match because it is meant for deep internal components that must be replaced gradually while callers remain stable. That is a better fit than strangler-fig style perimeter interception because Boothy's problem is inside the booth runtime: camera control and Lightroom-facing behavior sit behind internal seams, not external entry points.

Microsoft's .NET porting guidance points the same way indirectly. Modernization is treated as a deliberate migration program with assessment, path selection, and validation. Since the local camera reference is still heavily .NET Framework based, the cheapest move is not "modernize all of that now," but "confine it behind a replaceable contract and modernize only if the boundary proves durable."

_Source: https://docs.aws.amazon.com/prescriptive-guidance/latest/modernization-decomposing-monoliths/branch-by-abstraction.html ; https://docs.aws.amazon.com/prescriptive-guidance/latest/modernization-decomposing-monoliths/strangler-fig.html ; https://learn.microsoft.com/en-us/dotnet/core/porting/_

### Development Workflows and Tooling

Tauri's official guidance makes the host-side adoption path straightforward. The brownfield approach is recommended when embedding or reusing existing frontend assets, and the external-binaries/sidecar guidance shows how to package helper executables with the app. That aligns almost exactly with the Boothy plan: reuse Host/UI selectively, keep the camera helper separate, and let the packaged desktop app remain the primary operator-facing runtime.

This yields a practical workflow split. Host/UI development happens in the modern React/Tauri/Rust toolchain. Camera-boundary work is isolated in a small Windows helper and validated against the contract rather than through broad product entanglement.

_Source: https://v2.tauri.app/start/migrate/from-electron/ ; https://v2.tauri.app/develop/embedding-external-binaries/ ; https://v2.tauri.app/develop/sidecar/_

### Testing and Quality Assurance

The testing shape should mirror the boundary shape. Tauri's official testing docs support `mockIPC`, a mock runtime, and WebDriver-based E2E. That means Boothy can verify most booth logic without a live camera: the UI can be tested against mocked commands and streamed sidecar-like events, and only a small smoke layer needs real Windows desktop execution.

This matters because wrapper decisions fail when teams over-test hardware too early or under-test contracts entirely. The right order is contract-first tests, then filesystem-pipeline tests, then one or two Windows smoke paths with the packaged app.

_Source: https://v2.tauri.app/develop/tests/ ; https://v2.tauri.app/develop/calling-rust/ ; https://v2.tauri.app/develop/sidecar/ ; https://playwright.dev/docs/intro_

### Deployment and Operations Practices

Operationally, the sidecar path is still the cheapest deployable shape. Tauri's distribute and CI documentation supports GitHub Actions-based builds, platform packaging, updater integration, and signing workflows. For Boothy, this means the host and bundled camera helper can ship as one managed booth package, instead of requiring field operators to install or maintain a separate service manually.

For runtime diagnostics, OpenTelemetry is useful as a vendor-neutral observability layer for logs, metrics, and traces. The key architectural point is not to put it in the capture-critical path. Instrumentation should observe the booth flow, not become a dependency of it.

_Source: https://v2.tauri.app/distribute/ ; https://github.com/tauri-apps/tauri-action ; https://opentelemetry.io/docs/what-is-opentelemetry/ ; https://learn.microsoft.com/en-us/azure/architecture/framework/devops/overview_

### Team Organization and Skills

Skill concentration matters here. The main product team should optimize around React, Tauri, Rust, and booth-state UX. C# and legacy camera-control expertise should be treated as a specialized adapter concern, not as the center of gravity for the whole product. That reduces coordination load and keeps the long-term product architecture aligned with the strongest reusable asset, which is the Host/UI flow.

This is consistent with operational excellence guidance that emphasizes standardized tooling, automation, and repeatable delivery. The broader the product's core stack becomes, the harder those qualities are to maintain.

_Source: https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/ ; https://learn.microsoft.com/en-us/azure/architecture/framework/devops/overview_

### Cost Optimization and Resource Management

The lowest-cost path is the one that reuses modern value and quarantines legacy risk. Host/UI selective reuse avoids a full rewrite. A bundled camera helper avoids a full platform migration of the old camera reference. Capability-based sidecar packaging also reduces field-install complexity compared with introducing a separately deployed Windows service too early.

The expensive alternatives are easy to identify: modernizing the full .NET Framework camera stack up front, rewriting the entire UI natively, or building a service-heavy local architecture before the booth contract is stable.

_Source: https://v2.tauri.app/develop/embedding-external-binaries/ ; https://learn.microsoft.com/en-us/dotnet/core/porting/ ; https://docs.aws.amazon.com/prescriptive-guidance/latest/modernization-decomposing-monoliths/branch-by-abstraction.html_

### Risk Assessment and Mitigation

The primary implementation risks are architectural leakage, adapter sprawl, and insufficient contract verification. Architectural leakage happens when the host begins to understand camera internals again. Adapter sprawl happens when the wrapper grows into a second product with no stable contract. Verification failure happens when the first real integration test occurs only in the booth.

The mitigation set is correspondingly simple:

- fix the host-facing camera contract early
- keep camera truth inside the adapter
- correlate captures by filesystem artifacts and request IDs
- use mockIPC and sidecar-contract tests before live hardware
- package the helper with the app to control runtime drift

_Source: https://v2.tauri.app/develop/tests/ ; https://v2.tauri.app/develop/sidecar/ ; https://learn.microsoft.com/en-us/azure/architecture/patterns/circuit-breaker ; https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/anti-corruption-layer.html_

## Technical Research Recommendations

### Implementation Roadmap

1. Define the host-facing camera contract first: `configureSession`, `capture`, `health`, `getDiagnostics`, `restart`, and result correlation rules.
2. Stand up a Host-only path that proves session creation, filesystem watching, preset application, export state, and operator translation without real camera integration.
3. Add a bundled Windows camera helper behind the contract using the thinnest viable sidecar protocol.
4. Validate `capture -> file arrival -> booth result ready` with one Canon-focused spike or wrapped legacy seam.
5. Only after the contract is stable, decide whether the helper remains wrapped legacy code, a narrowed Canon adapter, or a more formal local service.

### Technology Stack Recommendations

- Keep the primary product surface in `React + Tauri + Rust`.
- Use Tauri capabilities and sidecar packaging for any external camera helper.
- Keep durable booth truth on the filesystem first; add SQLite only for logs and KPI queries if needed.
- Avoid HTTP-first or gRPC-first local architecture until there is clear evidence that sidecar-plus-filesystem is insufficient.

### Skill Development Requirements

- Strong state-modeling and asynchronous UI discipline in the Host/UI team
- Rust ownership over filesystem pipeline and contract enforcement
- Narrow C# or camera-SDK expertise limited to the adapter seam
- Test discipline around mocked IPC, sidecar process handling, and Windows smoke validation

### Success Metrics and KPIs

- Time to first Host-only end-to-end booth simulation
- Time to first successful `capture -> file arrival -> booth result ready` spike
- Number of host modules that remain ignorant of camera internals
- Number of failure cases surfaced as normalized booth/operator states instead of raw device errors
- Deployment simplicity on a single booth PC with no manual helper installation

## Research Synthesis

### Executive Summary

Boothy should not treat "wrapping" as a blanket strategy. Wrapping is efficient only when it preserves the highest-value reusable asset and pushes low-trust legacy behavior behind a strict boundary. The evidence in this research shows that the highest-value reusable asset is the Host/UI flow: guided session entry, reassurance through latest-photo updates, export-state visibility, and operator-friendly status translation. That layer aligns well with a modern `React + Tauri + Rust` stack and can be reused or adapted without first paying a platform-migration penalty.

The camera side is different. Current local references and current public documentation point to legacy, Windows-bound, and camera-specific constraints. That makes wholesale reuse of old camera solutions inefficient. The right move is to isolate camera and Lightroom-facing behavior inside a small local adapter or sidecar, translate its outputs into Boothy's booth state model, and keep durable capture truth in the filesystem. In practical terms, the most efficient path is:

**`RapidRAW Host/UI selective reuse + bundled Windows camera sidecar/adapter + filesystem handoff + branch-by-abstraction migration`**

This recommendation is stronger than the alternatives because it minimizes coordination cost, contains failures, preserves the best modern asset, and allows gradual camera-boundary replacement without destabilizing the customer-facing product. It also maps directly to the PRD's emphasis on stable booth flow, low operator burden, and state-centered recovery instead of UI-level troubleshooting.

**Key Technical Findings:**

- The modern Host/UI donor path remains current and maintainable.
- The camera reference remains useful as an extraction seam, not as a full product base.
- The best local contract is command-oriented with sidecar execution and filesystem result handoff.
- A bounded modular host plus narrow sidecar is a better fit than either a full monolith with leaked device logic or a service-heavy local architecture.
- Incremental replacement behind a stable abstraction is safer than up-front modernization of the entire legacy camera stack.

**Technical Recommendations:**

- Reuse and reshape Host/UI assets first.
- Freeze the host-facing camera contract before expanding integration work.
- Keep camera truth inside the adapter and booth truth inside the host.
- Test contract and filesystem behavior before relying on live camera hardware.
- Delay HTTP-first, gRPC-first, or whole-stack .NET modernization until the boundary proves insufficient.

### Table of Contents

1. Technical Research Introduction and Methodology
2. Boothy Technical Landscape and Architecture Analysis
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

This research was driven by a concrete product question: for Boothy's unmanned photo-booth workflow, where is wrapping an efficient accelerator and where does it become architecture debt? The analysis used the local PRD and operating documents as the product truth, then validated the technical claims against current primary sources for Tauri, React, Rust, Adobe Lightroom Classic tethering, Microsoft architecture guidance, and AWS modernization patterns.

The methodology was intentionally boundary-first. Instead of starting from frameworks, it started from booth outcomes: customer-visible state, operator diagnosability, session-folder handoff, and failure containment. That made it possible to judge technologies not by popularity, but by how well they preserve those outcomes while limiting coupling and recovery cost.

_Source: https://learn.microsoft.com/en-us/azure/architecture/guide/design-principles/ ; https://helpx.adobe.com/lightroom-classic/help/import-photos-tethered-camera.html ; https://v2.tauri.app/develop/calling-rust/_

### 2. Boothy Technical Landscape and Architecture Analysis

The technical landscape splits cleanly in two. On the Host/UI side, the local RapidRAW fork already sits on a modern stack and demonstrates reusable booth-adjacent strengths such as local desktop packaging, GPU-backed image workflows, and folder-centric state. On the camera side, the local digiCamControl reference remains valuable because it exposes a Canon seam and working capture-transfer examples, but it also remains deeply tied to a legacy Windows/.NET Framework shape.

That asymmetry is why the recommended architecture is not "wrap everything" and not "rewrite everything." It is a bounded modular host with one isolated camera boundary. The host owns product flow, the adapter owns device truth, and the filesystem owns durable image handoff.

_Source: https://react.dev/versions ; https://doc.rust-lang.org/edition-guide/rust-2024/index.html ; https://digicamcontrol.com/download ; https://learn.microsoft.com/en-us/azure/architecture/patterns/sidecar_

### 3. Implementation Approaches and Best Practices

The most defensible implementation approach is incremental replacement behind a stable contract. AWS's branch-by-abstraction guidance is especially relevant because Boothy's camera and Lightroom seams are internal dependencies, not external system edges. That means the host-facing contract should be introduced first, and old versus new camera implementations should compete behind it without forcing repeated changes in the Host/UI layer.

The practical best practice set follows directly: keep the first integration path narrow, validate booth behavior without live hardware where possible, and reserve deeper modernization work for the point where the boundary has already proven useful.

_Source: https://docs.aws.amazon.com/prescriptive-guidance/latest/modernization-decomposing-monoliths/branch-by-abstraction.html ; https://learn.microsoft.com/en-us/dotnet/core/porting/ ; https://v2.tauri.app/develop/tests/_

### 4. Technology Stack Evolution and Current Trends

The current technology trend relevant to Boothy is not constant framework churn but selective modernization. React 19 and Rust 2024 are current and stable. Tauri 2 formalizes capability-based permissions, structured commands, events, channels, and sidecar execution. By contrast, migrating older .NET Framework code is still a distinct effort with its own cost model. That means the modern stack should remain the center of gravity, while legacy Windows camera logic should be treated as a specialized compatibility concern.

In other words, Boothy gains more by modernizing where product value already lives than by dragging the whole product toward the oldest dependency.

_Source: https://react.dev/blog/2024/12/05/react-19 ; https://doc.rust-lang.org/edition-guide/rust-2024/index.html ; https://v2.tauri.app/develop/sidecar/ ; https://learn.microsoft.com/en-us/dotnet/core/porting/_

### 5. Integration and Interoperability Patterns

The most efficient interoperability model is local, point-to-point, and asymmetric. The UI should call a single host façade. The host should use a small command contract to speak to the camera helper. The helper should emit normalized status and leave durable photo transfer to the filesystem. Tauri's official guidance strengthens this model by distinguishing request-response commands from streamed channels and by supporting packaged sidecars as first-class external binaries.

This makes `command + sidecar + filesystem handoff` the lowest-friction contract shape for the first implementation path. Named pipes or gRPC only become attractive if the helper grows into a more independent local service than the current product evidence justifies.

_Source: https://v2.tauri.app/develop/calling-rust/ ; https://v2.tauri.app/develop/calling-frontend/ ; https://v2.tauri.app/develop/sidecar/ ; https://learn.microsoft.com/en-us/dotnet/standard/io/pipe-operations_

### 6. Performance and Scalability Analysis

Boothy's critical performance axis is booth responsiveness under fault, not horizontal cloud scale. The system must keep the customer flow legible while camera detection, file transfer, or export work is delayed or degraded. Azure's reliability and performance guidance supports this framing by emphasizing containment of failure domains and efficient use of local resources.

An isolated camera helper improves this immediately. If the helper stalls or fails, the host can keep rendering booth state, trip operator-visible fault handling, and preserve session data. That is a much stronger operational profile than a single runtime where device failures leak directly into the UI layer.

_Source: https://learn.microsoft.com/en-us/azure/well-architected/reliability/ ; https://learn.microsoft.com/en-us/azure/well-architected/performance-efficiency/ ; https://learn.microsoft.com/en-us/azure/architecture/patterns/circuit-breaker_

### 7. Security and Compliance Considerations

The right security model is local least privilege. Tauri capabilities and runtime authority make it possible to restrict which windows can access which commands and which shell permissions are granted to sidecars. That is a better fit for Boothy than adding network-style authentication concepts inside a same-machine booth runtime.

If the helper later uses named pipes, Windows ACLs can enforce local access boundaries there as well. This keeps security anchored to process and capability boundaries instead of mixing it into booth-state logic.

_Source: https://v2.tauri.app/security/runtime-authority/ ; https://v2.tauri.app/develop/sidecar/ ; https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights_

### 8. Strategic Technical Recommendations

The strategic recommendation is to keep the product core where the product value is strongest. For Boothy, that means preserving and simplifying the host-side user journey while isolating camera behavior behind an anti-corruption boundary. The result is not a pure rewrite and not a pure wrapper; it is selective Host/UI reuse plus a replaceable camera adapter.

Just as important are the explicit non-recommendations. Boothy should not adopt the full digiCamControl solution as a base, should not make the UI responsible for device semantics, and should not introduce service-heavy local infrastructure before the camera boundary proves that it needs it.

_Source: https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/anti-corruption-layer.html ; https://learn.microsoft.com/en-us/azure/architecture/patterns/sidecar ; https://learn.microsoft.com/en-us/azure/architecture/guide/design-principles/_

### 9. Implementation Roadmap and Risk Assessment

The phased roadmap is straightforward:

1. Freeze the host-facing camera contract and its success semantics.
2. Prove the Host-only booth flow without real camera integration.
3. Add a bundled camera helper using the thinnest viable sidecar path.
4. Validate one real capture pipeline from request to file arrival to booth-ready result.
5. Decide whether the helper stays wrapped, narrows into a Canon-focused adapter, or graduates into a more formal local service.

The primary risks are host leakage of camera internals, adapter sprawl, and weak contract verification. Each is manageable if the contract is fixed early, tested via mocks before hardware, and protected by filesystem-based correlation rather than UI-level heuristics.

_Source: https://v2.tauri.app/develop/tests/ ; https://v2.tauri.app/develop/embedding-external-binaries/ ; https://docs.aws.amazon.com/prescriptive-guidance/latest/modernization-decomposing-monoliths/branch-by-abstraction.html_

### 10. Future Technical Outlook and Innovation Opportunities

If the recommended boundary holds, Boothy gains optionality. The first version can ship with a bundled helper. A later version can replace wrapped legacy logic with a narrower Canon-first adapter. Beyond that, the team can decide whether there is real value in moving toward a longer-lived local service, richer observability, or more automated booth diagnostics. None of those future moves require reworking the booth-state UX if the contract remains stable.

That is the real opportunity in this design: it makes later innovation cheaper because it keeps today's compromise localized.

_Source: https://docs.aws.amazon.com/prescriptive-guidance/latest/modernization-decomposing-monoliths/branch-by-abstraction.html ; https://learn.microsoft.com/en-us/dotnet/core/porting/ ; https://opentelemetry.io/docs/what-is-opentelemetry/_

### 11. Technical Research Methodology and Source Verification

Primary source verification was applied to all time-sensitive or ecosystem-sensitive claims. The main official sources were:

- React official version and release documentation
- Rust edition guide
- Tauri 2 developer, security, testing, sidecar, packaging, and plugin documentation
- Adobe Lightroom Classic tethered capture and support documentation
- Microsoft architecture, .NET porting, WPF, IPC, and reliability guidance
- AWS modernization and anti-corruption guidance
- OpenTelemetry reference documentation

Confidence is high for the Host/UI, sidecar, integration, and modernization-pattern conclusions. Confidence is moderate for the exact long-term Canon SDK path because official public material is thinner there and much of the actionable detail still comes from local reference code.

_Source: https://react.dev/versions ; https://doc.rust-lang.org/edition-guide/rust-2024/index.html ; https://v2.tauri.app/develop/sidecar/ ; https://helpx.adobe.com/lightroom-classic/help/import-photos-tethered-camera.html ; https://learn.microsoft.com/en-us/dotnet/core/porting/ ; https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/anti-corruption-layer.html_

### 12. Technical Appendices and Reference Materials

**Decision Table**

| Topic | Recommended | Deferred / Rejected |
| --- | --- | --- |
| Host/UI base | Selective reuse of modern React/Tauri/Rust assets | Full native rewrite |
| Camera integration shape | Bundled Windows sidecar/adapter | Full digiCamControl product adoption |
| Durable booth truth | Session folders and filesystem handoff | Database-first capture architecture |
| Local IPC | Commands + sidecar stdio/channels | HTTP-first or gRPC-first local architecture |
| Migration strategy | Branch by abstraction | Big-bang modernization of the entire legacy camera stack |

**Primary Local Inputs**

- `_bmad-output/planning-artifacts/prd.md`
- `docs/business_context/context.md`
- `docs/research-checklist-2026-03-07-boothy-greenfield.md`
- `docs/refactoring/research-codex.md`

**Final Decision Statement**

Boothy should pursue selective wrapping, not universal wrapping. The most efficient path is to reuse and simplify modern Host/UI assets, isolate camera and Lightroom-facing behavior inside a bundled local adapter, translate that behavior through an anti-corruption boundary, and preserve session-folder handoff as the durable booth contract.

---

## Technical Research Conclusion

### Summary of Key Technical Findings

The research found that wrapping is efficient only where Boothy already has strong product leverage. That is the Host/UI layer, not the entire legacy camera stack. The camera side should be treated as a constrained compatibility problem, isolated behind a sidecar or adapter and gradually improved behind a fixed contract.

### Strategic Technical Impact Assessment

This reduces delivery risk, preserves product momentum, and prevents the customer-facing workflow from inheriting the worst traits of legacy Windows camera tooling. It also gives the team a path to improve camera integration over time without reopening the product architecture each time.

### Next Steps Technical Recommendations

- Write the host-facing camera contract and state model as an explicit design artifact.
- Prototype the Host-only booth loop with mocked camera events.
- Prove one real capture-to-filesystem spike through the adapter boundary.
- Keep all new product features above that boundary unless a strong reason appears to cross it.

---

**Technical Research Completion Date:** 2026-03-08
**Research Period:** 2026-03-08 current-state technical validation
**Source Verification:** Current primary sources plus local project references
**Technical Confidence Level:** High for architecture direction, medium for the exact long-term camera-helper implementation path
