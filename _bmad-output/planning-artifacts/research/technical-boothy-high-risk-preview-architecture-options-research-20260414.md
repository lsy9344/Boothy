---
stepsCompleted: [1, 2, 3, 4, 5, 6]
inputDocuments:
  - "_bmad-output/planning-artifacts/preview-architecture-reassessment-report-20260414.md"
  - "_bmad-output/planning-artifacts/research/technical-boothy-preview-architecture-alternatives-research-20260414.md"
  - "_bmad-output/planning-artifacts/research/technical-boothy-gpu-first-rendering-architecture-research-2026-04-11.md"
  - "_bmad-output/planning-artifacts/research/technical-boothy-gpu-first-rendering-architecture-validation-research-2026-04-11.md"
workflowType: 'research'
lastStep: 6
research_type: 'technical'
research_topic: 'Boothy 24인치 가로 풀화면 same-capture preset-applied 2.5초 목표를 위한 고위험 기술 옵션 및 전체 구조 재조정안'
research_goals: '지금까지 설계, 시도, 검증, 재설계 이력을 전제로 low-risk 경로가 사실상 소진된 상황에서 목표 달성을 위해 남아 있는 high-risk technical path를 조사하고, 이미 시도한 방법과 실제로 다른 옵션을 구분한다. 남아 있는 옵션이 없거나 전부 기존 시도의 변형이라면 시스템 전체 구조를 어디까지 재조정해야 목표를 현실적으로 닫을 수 있는지까지 연구하며, 각 구조/옵션의 성공 가능성, 구현 난이도, 검증 방법, 제품 리스크, 중단 기준을 정리한다.'
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

이번 리서치는 `촬영 후 raw 파일에 프리셋이 적용된 같은 사진이 24인치 가로 풀화면으로 2.5초 안에 보여야 한다`는 제품 목표를 기준으로 수행했다. 기존 `fast preview`, `recent-session`, `first-visible`, `local dedicated renderer`, `GPU-first canary`까지의 설계/시도/재설계 이력을 전제로, 저위험 경로는 사실상 소진되었다고 보고 남아 있는 `고위험 기술 옵션`과 `전체 구조 재조정안`만 다시 검토했다. 내부 이력 문서와 함께 Microsoft, GitHub, Playwright, OpenTelemetry, gRPC, Tauri, WIC, LibRaw, OpenImageIO 등의 공식 문서를 교차 검증했다.

종합 결론은 단순하다. `가능한 방법이 아예 없는 것`으로 결론 내릴 단계는 아니지만, `낮은 리스크로 목표를 달성할 길`은 이미 대부분 소진됐다. 지금 남아 있는 가장 설득력 있는 다음 베팅은 `local native/GPU coprocessor + dedicated full-screen lane + host-owned promotion authority + bounded CQRS/materialized display projection`이다. 이 경로가 반복적인 실장비 검증에서도 실패할 때만 `single off-box renderer cell/stamp`로 구조를 한 단계 더 올리는 편이 맞다.

아래 상세 섹션은 기술 스택, 통합 경계, 아키텍처 패턴, 구현 전략을 순서대로 정리하고 있으며, 문서 하단의 `Executive Summary`, `Research Synthesis`, `Technical Research Conclusion`에서 최종 권고와 중단 기준을 다시 요약한다.

---

## Executive Summary

이 리서치의 핵심 전제는 명확하다. 목표는 `썸네일`이나 `첫 표시`가 아니라, `same-capture preset-applied full-screen <= 2500ms`다. 그리고 내부 이력상 기존 `local dedicated renderer / GPU-first / route activation` 계열은 이미 제품 경로에 올라갔지만 목표를 닫지 못했다. 따라서 이번 문서는 `남아 있는 저위험 옵션`을 찾는 문서가 아니라, **고위험 기술 베팅과 구조 재조정의 현실성을 평가하는 문서**다.

최종 판단은 다음과 같다. 1순위는 `local native/GPU coprocessor + dedicated full-screen lane`이며, 이 lane은 `host-owned promotion authority`, `capture-bound evidence`, `current capture 전용 bulkhead`, `bounded CQRS/materialized display projection` 위에서 움직여야 한다. 즉, 24인치 가로 풀화면에 바로 올릴 결과물을 단순 임시 캐시가 아니라 read-optimized projection으로 승격하고, export/backfill/parity는 별도 lane으로 밀어내야 한다.

2순위는 `single off-box renderer cell/stamp`다. 다만 이는 로컬 경로가 반복 측정에서 실패한 뒤에만 열어야 한다. 반대로 `기존 dedicated renderer 계열의 추가 미세조정`, `full microservices`, `broker-first close owner`는 현재까지의 이력과 공식 패턴 근거를 합치면 우선순위가 낮다. 구현 전략도 `빅뱅 교체`가 아니라 `짧은 R&D POC -> shadow validation -> real hardware canary -> health-gated expansion` 순서를 따라야 한다.

**Key Technical Findings:**

- 저위험 경로는 사실상 소진됐고, 이제는 `구조적으로 다른 runtime`만 의미가 있다.
- 가장 유력한 다음 베팅은 `local native/GPU coprocessor`이며, 핵심은 `close authority`와 `critical flow`를 다시 자르는 것이다.
- `display-sized truthful artifact`는 캐시가 아니라 별도 read model로 취급하는 편이 맞다.
- off-box를 열더라도 `분산 시스템 전체`가 아니라 `single renderer cell/stamp` 단위로 제한해야 한다.
- 제품 노출은 반드시 `feature flag + health gate + quick off switch` 뒤에서만 허용해야 한다.

**Technical Recommendations:**

- `same-capture preset-applied full-screen <= 2500ms`를 유일한 합격 기준으로 고정한다.
- `local native/GPU coprocessor` POC를 time-box 방식으로 짧게 검증한다.
- 기존 truth/parity 경로는 제거하지 말고 shadow validation과 fidelity oracle로 유지한다.
- `wrong-capture = 0`, `fidelity mismatch = 0`, `fallback stability`를 속도와 동일한 승격 조건으로 둔다.
- 로컬 반복 실패가 확인될 때만 `single off-box renderer cell/stamp` POC로 올라간다.

## Table of Contents

1. Research Overview
2. Executive Summary
3. Technical Research Scope Confirmation
4. Technology Stack Analysis
5. Integration Patterns Analysis
6. Architectural Patterns and Design
7. Implementation Approaches and Technology Adoption
8. Research Synthesis
9. Technical Research Methodology and Source Verification
10. Technical Research Conclusion

<!-- Content will be appended sequentially through research workflow steps -->

## Technical Research Scope Confirmation

**Research Topic:** Boothy 24인치 가로 풀화면 same-capture preset-applied 2.5초 목표를 위한 고위험 기술 옵션 및 전체 구조 재조정안
**Research Goals:** 지금까지 설계, 시도, 검증, 재설계 이력을 전제로 low-risk 경로가 사실상 소진된 상황에서 목표 달성을 위해 남아 있는 high-risk technical path를 조사하고, 이미 시도한 방법과 실제로 다른 옵션을 구분한다. 남아 있는 옵션이 없거나 전부 기존 시도의 변형이라면 시스템 전체 구조를 어디까지 재조정해야 목표를 현실적으로 닫을 수 있는지까지 연구하며, 각 구조/옵션의 성공 가능성, 구현 난이도, 검증 방법, 제품 리스크, 중단 기준을 정리한다.

**Technical Research Scope:**

- Architecture Analysis - 기존 실패 구조와 실질적으로 다른 ownership, lane topology, runtime boundary, system reset candidate
- Implementation Approaches - full custom GPU runtime, off-box renderer, structural split, contract reset, hardware boundary reallocation
- Technology Stack - Windows native/GPU stack, RAW decode/apply/display stack, remote or distributed render stack, cache/artifact stack
- Integration Patterns - same-capture truth를 유지하는 로컬/원격 계약, promotion/evidence/rollback model, authority boundary
- Performance Considerations - `24인치 가로 풀화면 same-capture preset-applied <= 2500ms` 달성 가능성, 병목 이전 여부, 하드웨어 검증 가능성
- Structural Adjustment Analysis - 개별 high-risk option이 남지 않을 경우 목표 달성을 위해 제품 구조를 어디까지 바꿔야 하는지

**Research Methodology:**

- 최신 공개 자료 기반 web verification
- 중요한 기술 주장에 대한 다중 출처 교차 확인
- 기존 내부 실패 이력과 현재 공개 기술 패턴을 함께 사용한 product-fit 평가
- 불확실성은 sourced fact와 inference를 구분해 명시

**Trigger Conditions:**

- 기존 `local dedicated renderer + different close topology` 경로는 이미 1순위로 선택되어 실제 제품 경로에 올라갔다.
- 이후 `resident GPU-first primary lane` 계열도 activation/canary까지 진행되었다.
- 그럼에도 최신 재평가 문서는 `운영 경로 전환 성공`, `속도 목표 달성 실패`, `무기한 미세조정 비권장`으로 정리했다.
- 따라서 이번 리서치는 가능한 대안을 넓게 찾는 단계가 아니라, low-risk path 소진 이후에도 남아 있는 high-risk bet 또는 전체 구조 재조정안이 있는지 검토하는 단계다.

**Scope Confirmed:** 2026-04-14

## Technology Stack Analysis

### Web Search Analysis

이번 단계에서는 `고위험 기술 옵션`과 `전체 구조 재조정안`에 직접 연결되는 기술 스택만 다시 확인했다. 조사 축은 다섯 가지였다.

- **Windows native/GPU runtime**: Direct3D 12, Direct2D custom effects, shader linking, GPU queue/memory model
- **RAW decode / preset apply stack**: LibRaw, darktable OpenCL, RawTherapee pipeline, Adobe preview-generation and DNG fast-load pattern
- **Artifact/cache/storage stack**: WIC prerendered preview, OpenImageIO ImageCache, DNG preview / fast-load data
- **Host boundary / process model**: Tauri sidecar, Rust host + native worker 조합
- **Off-box / structural reset platform**: AWS Outposts, Azure Local, Google Distributed Cloud connected, gRPC

조사 결과, 현재 남아 있는 high-risk path는 기술적으로 크게 네 계열로 압축된다.

1. **Windows-native custom GPU runtime**: `D3D12/HLSL` 또는 `Direct2D custom effect graph`를 중심으로 full-screen close lane을 아예 새로 만드는 경로
2. **Hybrid runtime with decode/apply split**: `LibRaw + custom apply/display + darktable parity/fallback` 조합으로 blocking truth path를 깨는 경로
3. **Artifact-first structural reset**: `display-sized truthful artifact`를 primary close 대상으로 승격하고 truth/parity path를 뒤에 두는 경로
4. **Off-box / edge reset**: booth 본체와 render owner를 물리적으로 분리하는 경로

반대로, 현재 공개 자료를 다시 봐도 `DirectML 중심 ML upscaling`이나 `범용 serverless/cloud-only hot path`가 이 목표의 1차 주력안이라는 근거는 강하지 않았다. 특히 DirectML은 계속 지원되지만 신규 기능 중심은 `Windows ML` 쪽으로 이동했다고 명시돼 있어, 지금 시점에서는 core RAW truth engine보다는 보조 가속 계층으로 보는 편이 맞다.

**Research Coverage:**

- Windows-native GPU stack과 explicit scheduling model 확인
- RAW/preset engine의 역할 분리 가능성 확인
- artifact/cache 중심 구조의 기술 정당성 재확인
- off-box edge 플랫폼의 운영 가능성 확인
- profiling/diagnostics toolchain 확인

**Quality Assessment:**

- **높음:** Microsoft Learn, Adobe, Tauri, LibRaw, OpenImageIO, gRPC 공식 문서
- **중간:** edge/on-prem platform의 실제 booth 적합성
- **한계:** public 문서에는 `same-capture preset-applied 24인치 full-screen <= 2500ms`를 직접 증명하는 benchmark는 없음

### Programming Languages

이번 주제에서 언어 선택은 생산성보다 `누가 hot path를 담당할 수 있느냐`가 핵심이다. 공개 자료 기준으로 가장 직접적인 언어 조합은 여전히 `Rust + C/C++ + HLSL`이다. Tauri는 Webview와 Rust backend를 쉽게 연결하고, sidecar 실행도 정식 경로로 제공하므로 현재 host를 유지하면서 별도 네이티브 worker를 붙이기 쉽다. 반면 Direct3D 12는 HLSL을 pre-compiled shader object와 pipeline state object로 다루며, command queue/command list/fence 모델을 통해 동시성, 우선순위, 동기화를 앱이 직접 통제하게 만든다. 즉, 목표를 닫으려면 상위 런타임이 아니라 `C++/HLSL` 계열의 Windows-native control surface를 더 많이 떠안아야 한다.

`LibRaw`도 같은 방향을 뒷받침한다. LibRaw 인스턴스는 한 번에 하나의 source만 처리하지만, 여러 processor를 병렬로 둘 수 있다고 설명한다. 이는 decode stage를 resident pool 또는 per-capture multi-instance 구조로 설계할 수 있다는 뜻이다. 반면 LibRaw 자체는 RAW postprocessing의 주력 엔진이 아니라고 명시하므로, `decode는 LibRaw`, `apply/display는 custom path`, `truth/parity는 darktable` 같은 역할 분리가 현실적이다.

언어 관점에서 high-risk 후보를 나누면 이렇게 정리된다.

- **Rust**: host orchestration, session/evidence contract, sidecar supervision
- **C/C++**: RAW decode, low-level image runtime, GPU service core
- **HLSL**: Windows custom display lane, Direct2D/Direct3D shader graph
- **CUDA C++**: NVIDIA 전용 최고 성능 카드가 될 수 있으나 booth 하드웨어 종속성이 커진다
- **DirectML / Windows ML 계열**: denoise, upscaling, heuristic assist용 보조축은 가능하지만 core RAW truth path의 1차 답으로 보긴 이르다

_Popular Languages:_ Rust(host), C/C++(decode/runtime), HLSL(Windows GPU path)  
_Emerging Languages:_ DirectML/Windows ML은 ML 보조 연산용 후보지만 핵심 RAW truth runtime 주력으로 보기엔 근거가 약하다  
_Language Evolution:_ 제품 셸은 Rust/Tauri 유지, latency-critical lane은 점점 더 explicit native/GPU control 쪽으로 이동하는 편이 맞다  
_Performance Characteristics:_ D3D12는 동기화와 우선순위를 앱이 직접 통제할 수 있어 high-risk이지만, 제대로만 구현되면 현재 구조보다 더 직접적으로 병목을 제거할 수 있다  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/direct3d12/pipelines-and-shaders-with-directx-12 ; https://learn.microsoft.com/en-us/windows/win32/direct3d12/executing-and-synchronizing-command-lists ; https://www.libraw.org/docs/API-overview.html ; https://v2.tauri.app/concept/architecture/ ; https://learn.microsoft.com/en-us/windows/ai/directml/dml

### Development Frameworks and Libraries

가장 중요한 프레임워크 축은 네 가지다. 첫째, **Windows-native custom effect/runtime 계층**이다. Direct2D는 HLSL 기반 custom effect를 직접 작성할 수 있고, effect shader linking을 지원하는 경우 여러 effect graph pass를 하나의 pass로 줄이고 intermediate surface도 없앨 수 있다고 설명한다. 이건 `full-screen close lane`을 새로 만들 때 매우 중요한 신호다. 단, shader linking은 pixel shader/simple sampling에 유리하고 compute/vertex 중심 graph에는 제약이 있으므로, 모든 preset을 그대로 얹는 범용 해법은 아니다.

둘째, **RAW decode / light-processing 계층**이다. LibRaw는 RAW source data를 읽고 dcraw 수준의 처리 호출도 제공하지만 postprocessing 주력 라이브러리는 아니다. 따라서 `LibRaw만으로 truthful preset-applied result`를 닫는 것은 어렵고, decode stage 전용 부품으로 보는 편이 맞다. 셋째, **parity/fallback 계층**이다. darktable는 OpenCL 가속 경로와 CPU fallback이 동작하며 CPU/GPU 결과가 사실상 동일하도록 설계된다고 설명한다. 이것은 darktable가 주력 close owner로는 실패했더라도 `parity oracle`로는 계속 강력하다는 뜻이다. 넷째, **pipeline 분리형 편집기 패턴**이다. RawTherapee는 화면 표시와 저장 경로가 다른 pipeline을 탄다고 설명하므로, display/export truth 분리는 고위험 도박이 아니라 업계 공통 패턴에 가깝다.

프레임워크 관점에서 의미 있는 high-risk 조합은 아래와 같다.

- **D3D12 compute/render runtime + custom HLSL graph**
- **Direct2D custom effects + effect shader linking** 기반의 display lane
- **LibRaw decode + custom apply/display**
- **darktable baseline/fallback/parity**
- **Tauri sidecar + native worker process**

_Major Frameworks:_ Direct3D 12, Direct2D custom effects, Tauri sidecar, LibRaw, darktable, OpenImageIO  
_Micro-frameworks:_ shader linking, tiled image cache, preview artifact cache, darktable parity path  
_Evolution Trends:_ preview generation과 display lane을 GPU로 끌어오는 방향이 강해지고 있으며, display/save pipeline 분리도 여전히 일반적이다  
_Ecosystem Maturity:_ Windows-native GPU stack과 Tauri boundary는 성숙했고, LibRaw/OIIO/darktable도 충분히 안정적이다. 다만 이를 하나의 booth runtime으로 엮는 작업은 고위험 통합 과제다  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/direct2d/custom-effects ; https://learn.microsoft.com/en-us/windows/win32/direct2d/effect-shader-linking ; https://www.libraw.org/docs/API-overview.html ; https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/ ; https://rawpedia.rawtherapee.com/Toolchain_Pipeline ; https://v2.tauri.app/develop/sidecar/

### Database and Storage Technologies

이번 주제에서 핵심 저장소는 관계형 DB가 아니라 **artifact cache와 capture-bound file truth**다. Microsoft `WIC`는 fast thumbnail/preview를 위해 `GetThumbnail`, `GetPreview`, prerendered preview cache를 권장하고, 반응성 있는 경험을 위해 `200ms` 이하 반환을 매우 바람직한 목표로 제시한다. Adobe도 DNG에서 `JPEG Preview`와 `Embed Fast Load Data`를 통해 preview loading을 빠르게 하는 패턴을 노출한다. 즉, high-risk 구조라도 결국 성공하려면 `바로 올릴 수 있는 artifact`를 중심에 둬야 한다.

OpenImageIO `ImageCache`는 이번 리서치에서 특히 중요하다. 공식 문서는 thread-safe, automatic file-handle management, tile-based loading, very small memory footprint, thousands of files access를 설명한다. 이건 `full-screen truthful artifact`, `multi-capture backfill`, `same-path replacement`를 관리하는 쪽에서 매우 유리하다. 즉, 현재까지의 실패를 고려하면 storage stack의 1순위는 DB 변경이 아니라 **display artifact를 어떻게 캐시하고 promote하느냐**다.

이번 주제에서 데이터 계층은 아래처럼 보는 것이 적절하다.

- **Relational Databases:** SQLite 같은 audit/catalog/route state 용도는 유지 가능하지만 hot path 해결책은 아님
- **NoSQL Databases:** booth close hot path에서는 우선순위 낮음. 분산/원격 구조가 커질 때만 메타데이터 보조축으로 검토 가능
- **In-Memory Databases:** Redis류보다 process-local queue/cache가 더 직접적일 가능성이 큼
- **Data Warehousing / Object Storage:** off-box edge 구조에서만 의미가 커지며, 로컬 구조에서는 secondary concern

위 판단 중 `관계형/NoSQL` 우선순위는 `WIC/DNG/OIIO`가 모두 artifact cache 쪽을 직접 강조한다는 점에서 도출한 inference다.

_Relational Databases:_ control plane/audit에는 유효하지만 close latency의 직접 해법은 아님  
_NoSQL Databases:_ 현재 hot path보다는 원격 분리 구조의 메타데이터 보조 수단에 가깝다  
_In-Memory Databases:_ 일반 목적 DB보다 process-local queue/cache가 더 적합하다  
_Data Warehousing:_ off-box 구조에서 local object storage나 edge platform storage와 결합될 때 의미가 커진다  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/wic/-wic-rawguidelines-thumbnail-previews ; https://helpx.adobe.com/si/photoshop-elements/using/processing-camera-raw-image-files.html ; https://helpx.adobe.com/my_ms/lightroom-classic/help/export-files-disk-or-cd.html ; https://openimageio.readthedocs.io/en/stable/imagecache.html

### Development Tools and Platforms

이번 단계에서 도구 체계는 선택이 아니라 필수다. high-risk runtime으로 갈수록 `느리다`가 아니라 **왜 느린지, 어느 queue/barrier/resource에서 느린지**를 바로 볼 수 있어야 하기 때문이다. Microsoft `PIX`는 D3D12 앱의 GPU capture와 timing capture를 제공하고, GPU capability는 Direct3D 12를 쓰는 모든 앱에서 동작한다고 설명한다. `WPR/WPA`는 ETW 기반으로 시스템 및 앱 이벤트를 기록해 전체 자원 소비를 볼 수 있게 한다. NVIDIA `Nsight Graphics`는 D3D11/12, Vulkan, OpenGL, OpenXR까지 지원하는 debugging/profiling/analysis 도구다.

즉, 만약 이번에 구조를 더 과감하게 바꾼다면 개발 플랫폼도 같이 바뀌어야 한다.

- **IDE / Native toolchain:** Visual Studio + Windows SDK + Rust toolchain
- **GPU profiling:** PIX, Nsight Graphics
- **System profiling:** WPR/WPA, ETW
- **Build systems:** cargo + native C/C++ build + shader compilation pipeline
- **Testing frameworks:** 기존 UI E2E보다 hardware trace와 capture-level evidence가 더 중요

이 영역의 핵심은 새로운 엔진을 만들 수 있느냐보다, **만든 뒤 병목을 수치로 해부할 수 있느냐**다.

_IDE and Editors:_ Rust + Windows-native 개발을 함께 다룰 수 있는 Visual Studio / Rust toolchain 조합이 현실적이다  
_Version Control:_ Git 기반 운영은 그대로 가능하나, 이 과제의 차별점은 source control보다 binary/evidence reproducibility에 있다  
_Build Systems:_ Rust host, native worker, shader compilation이 함께 도는 다중 빌드 체계가 필요하다  
_Testing Frameworks:_ PIX/WPR/Nsight 기반 profiling과 hardware validation evidence가 중심이 된다  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/direct3dtools/pix/articles/general/pix-overview ; https://learn.microsoft.com/en-us/windows-hardware/test/wpt/windows-performance-recorder ; https://docs.nvidia.com/nsight-graphics/ ; https://v2.tauri.app/develop/sidecar/

### Cloud Infrastructure and Deployment

원격 또는 off-box 구조를 진지하게 열 경우, 일반 cloud region보다는 **on-prem / edge-local infrastructure**가 더 relevant하다. AWS Outposts는 on-prem 환경에서 EC2, EBS, S3, EKS/ECS 등 로컬 지원 서비스를 제공한다. Google Distributed Cloud connected는 edge와 data center로 Google Cloud 인프라를 확장하며 low latency local workloads에 적합하다고 설명한다. Azure Local도 Azure Arc 기반 분산 인프라로 VM, container, 일부 Azure 서비스를 로컬에서 실행하게 해 준다.

이건 중요한 의미를 갖는다. 지금까지 `edge appliance`는 예비 카드 수준이었지만, low-risk/local-first path가 소진된 상황에서는 **booth 본체와 render owner를 물리적으로 분리하는 구조 자체가 다시 주력 후보**가 될 수 있다. 다만 이 경우에도 일반 public-cloud serverless나 CDN이 아니라, 현장 근처에서 deterministic하게 돌 수 있는 edge-local compute가 필요하다.

_Major Cloud Providers:_ AWS Outposts, Azure Local, Google Distributed Cloud connected가 직접 relevant하다  
_Container Technologies:_ EKS/ECS on Outposts, AKS on Azure Local, managed edge Kubernetes가 off-box 구조 후보가 된다  
_Serverless Platforms:_ capture-close hot path의 1차 기본안으로 보기 어렵다. 이 판단은 low-latency deterministic requirement에 기반한 inference다  
_CDN and Edge Computing:_ CDN보다는 on-prem edge compute / local object storage / low-latency local processing이 더 중요하다  
_Source:_ https://docs.aws.amazon.com/outposts/latest/network-userguide/what-is-outposts.html ; https://cloud.google.com/distributed-cloud-connected ; https://learn.microsoft.com/en-us/azure/azure-local/ ; https://grpc.io/docs/what-is-grpc/introduction/

### Technology Adoption Trends

현재 공개 자료가 공통으로 가리키는 방향은 세 가지다. 첫째, **preview generation과 display path를 더 GPU 쪽으로 끌어오는 흐름**이다. Adobe는 2025-08-13 기준 Lightroom Classic 14.5부터 GPU preview generation 옵션을 공식 노출한다. 둘째, **artifact cache / fast-load data / prerendered preview**를 사용해 사용자에게 먼저 보여줄 결과를 따로 다루는 흐름이다. 셋째, **explicit control의 대가로 복잡성을 감수하는 방향**이다. D3D12는 command queue, fence, residency, barrier를 앱이 직접 관리하게 해 주고, Direct2D shader linking은 pass/intermediate를 줄이는 대신 effect authoring 제약을 요구한다.

반대로, 이번 조사 기준으로는 다음 두 가지는 약하다.

- **Legacy Technology:** `darktable-only blocking close owner`를 계속 주력으로 두는 방향
- **Community Trend:** `ML upscaling / inferencing`를 core RAW truth path의 즉시 대체재로 보는 방향

즉 adoption trend는 `조금 더 최적화된 기존 구조`가 아니라, **더 explicit한 native control + 더 분리된 artifact/display pipeline + 필요 시 off-box edge separation** 쪽으로 움직인다.

_Migration Patterns:_ blocking truth path에서 explicit display lane 또는 off-box render owner로 이동  
_Emerging Technologies:_ GPU preview generation, edge-local compute, artifact-first truthful close  
_Legacy Technology:_ darktable-only close owner, queue/warm-state tuning 중심 접근  
_Community Trends:_ GPU는 더 중심으로 오고 있지만, ML 보조 가속은 아직 core truth path보다 accessory에 가깝다  
_Source:_ https://helpx.adobe.com/si/lightroom-classic/kb/gpu-preview-generation.html ; https://learn.microsoft.com/en-us/windows/win32/wic/-wic-rawguidelines-thumbnail-previews ; https://learn.microsoft.com/en-us/windows/win32/direct3d12/memory-management-strategies ; https://learn.microsoft.com/en-us/windows/win32/direct2d/effect-shader-linking ; https://learn.microsoft.com/en-us/windows/ai/directml/dml

## Integration Patterns Analysis

### Web Search Analysis

이번 단계에서는 `고위험 옵션`과 `전체 구조 재조정안`이 실제로 요구하는 통합 경계를 다시 확인했다. 조사 축은 다섯 가지였다.

- **UI ↔ host 경계**: Tauri IPC, commands/events, JSON-serializable request/response
- **host ↔ local renderer 경계**: anonymous pipe, named pipe, overlapped I/O, shared memory/file mapping
- **host ↔ remote/off-box renderer 경계**: gRPC, protobuf, health checking, authentication
- **distributed control plane**: API gateway, ambassador, service mesh, event-driven style
- **security/governance**: mTLS, OAuth/JWT, certificate-bound token, local authority boundary

조사 결과, 이번 주제의 통합 패턴은 `범용 분산 시스템`처럼 보면 오히려 흐려진다. 제품 목표가 `same-capture preset-applied full-screen <= 2500ms`이기 때문에, 핵심은 기술 유행이 아니라 **어떤 통신 경계가 full-screen close를 가장 적게 지연시키고, 어떤 경계가 truth/evidence/fallback을 가장 명확히 유지하느냐**다.

이번 기준에서 유력한 통합 계열은 세 가지다.

1. **로컬 point-to-point contract**: `Tauri events/commands` + `host-owned native IPC` + `capture-bound artifact/evidence contract`
2. **로컬 zero/low-copy 강화형**: named pipe + overlapped I/O + shared memory/file mapping + 별도 artifact commit
3. **원격 service contract형**: gRPC + protobuf + health + mTLS + explicit promotion contract

반대로, `API gateway`, `service mesh`, `broker-first pub/sub`, `ESB 스타일 허브`는 booth hot path 기본값으로는 과도할 가능성이 높다. 이들은 remote/off-box 구조가 여러 node/cell로 확장될 때 통제면으로는 의미가 있지만, `current capture full-screen close`의 1차 해결책으로 쓰기에는 hop과 제어면이 늘어난다.

**Research Coverage:**

- UI/host/native worker의 로컬 IPC 패턴
- remote/off-box service contract 패턴
- event-driven / CQRS / event sourcing의 적합성
- security and governance boundary

**Quality Assessment:**

- **높음:** Tauri, Win32 IPC, gRPC, protobuf, Azure architecture patterns, RFC 6455, RFC 8705
- **중간:** service mesh/edge broker를 booth hot path에 적용할 실제 product fit
- **한계:** 공개 문서는 booth 같은 단일 hot path의 absolute latency를 직접 비교하지 않으므로 일부 product-fit 판단은 inference

### API Design Patterns

이번 주제에서 API 설계는 `공개 API를 예쁘게 만드는 법`이 아니라 **누가 close authority를 가지는가**에 더 가깝다. Tauri는 코어와 Webview 사이에 asynchronous message passing을 사용하고, `Events`와 `Commands`라는 두 IPC primitive를 제공한다. 이벤트는 상태 변화 통지에, 명령은 프런트엔드가 Rust 함수를 호출하고 응답을 받는 데 적합하다. 또한 이 메커니즘은 내부적으로 JSON-RPC와 유사한 직렬화를 사용하므로 인수와 반환값은 JSON 직렬화 가능해야 한다. 따라서 `UI ↔ host` 경계는 지금도 앞으로도 `command + event` 패턴이 맞다.

문제는 그 다음 경계다. local custom runtime이나 off-box renderer로 가면, `host ↔ renderer`는 더 이상 Tauri-style JSON IPC로 다루기 어려워진다. gRPC는 protocol buffers를 IDL과 interchange format으로 함께 사용할 수 있고, 서로 다른 머신에서도 로컬 객체처럼 메서드 호출을 조직할 수 있게 해 준다. 이건 remote/off-box 구조에는 강하지만, 같은 머신 안의 ultra-hot path에는 HTTP/2 stack과 serialization layer라는 비용도 같이 데려온다. 따라서 API 패턴 관점의 기본 판단은 아래와 같다.

- **UI ↔ host:** Tauri `Commands + Events`
- **host ↔ local renderer:** narrow binary/local IPC contract
- **host ↔ remote renderer:** gRPC/Protobuf service contract
- **artifact promotion:** API call이 아니라 capture-bound file/evidence commit으로 닫는 편이 안전

REST나 GraphQL은 이번 과제의 기본안이 아니다. API gateway 문서가 보여주듯 gateway는 다수 front-end service를 다룰 때 중앙 진입점, SSL termination, mTLS, auth, rate limiting 같은 cross-cutting concern을 정리하는 데 유리하다. 그러나 booth hot path는 다수의 client-facing front-end service를 갖는 구조가 아니므로, gateway는 remote/off-box fleet control plane이 생길 때만 relevant하다.

_RESTful APIs:_ control plane, operator tooling, remote management에는 가능하지만 close hot path 1차 기본안으로는 부적합  
_GraphQL APIs:_ 동적 query flexibility보다 deterministic request/response가 중요해 우선순위가 낮다  
_RPC and gRPC:_ remote/off-box renderer에는 가장 설득력 있는 typed contract다  
_Webhook Patterns:_ observability, incident, operator notification에는 유효하지만 close path 기본값은 아니다  
_Source:_ https://v2.tauri.app/ko/concept/inter-process-communication/ ; https://grpc.io/docs/what-is-grpc/introduction/ ; https://learn.microsoft.com/en-us/azure/architecture/microservices/design/gateway

### Communication Protocols

프로토콜 관점에서는 `같은 머신`과 `다른 머신`을 분리해서 봐야 한다. Win32 문서는 named pipe I/O가 overlapped mode를 켠 경우 비동기적으로 수행될 수 있고, completion port까지 연결해 높은 동시성을 다룰 수 있다고 설명한다. 반면 anonymous pipe는 비동기(overlapped) 읽기/쓰기를 지원하지 않는다. CreateProcess와 redirected stdin/stdout는 child process 제어에는 유용하지만, 고성능 long-lived worker 통신으로 갈수록 named pipe 쪽이 더 유리하다.

공유 메모리도 여전히 의미가 있다. Win32 named shared memory는 `CreateFileMapping`과 `MapViewOfFile`을 통해 여러 프로세스가 같은 메모리 뷰를 열 수 있게 한다. 이 패턴은 큰 raster/tile/tensor를 복사 없이 넘기고, control path만 별도 IPC로 유지하는 데 적합하다. 따라서 local high-risk runtime의 유력 조합은 `named pipe for control + shared memory for bulk data + file commit for truth promotion`이다.

remote/off-box 경계에서는 이야기가 달라진다. gRPC는 protobuf와 함께 typed RPC를 제공하고, health checking은 `health/v1` 표준 service API를 통해 서버가 healthy/unhealthy를 광고하게 해 준다. 또한 auth 가이드와 RFC 8705는 TLS/mTLS 기반 보호 모델을 제공한다. 즉 원격 구조를 연다면 `gRPC + protobuf + health + mTLS`가 기본 조합이다.

WebSocket은 브라우저와 서버 간 two-way communication을 위한 표준 프로토콜이다. 이것은 remote operator UI나 live progress push에는 유효하지만, booth와 renderer 사이의 1차 계약으로는 여전히 불리하다. MQTT는 Azure Event Grid처럼 pub/sub 장치 연결이나 telemetry fan-out에는 적합하지만, `same-capture close authority`를 직접 다루는 기본 프로토콜로 보긴 어렵다.

_HTTP/HTTPS Protocols:_ remote control plane, operator APIs, event hook에는 유효  
_WebSocket Protocols:_ remote UI/live status push에는 유효하지만 primary close contract는 아님  
_Message Queue Protocols:_ MQTT/pub-sub는 telemetry or fleet sync에는 가능하나 capture-close 1차 path로는 부적합  
_grpc and Protocol Buffers:_ remote/off-box renderer 주력 조합  
_Source:_ https://learn.microsoft.com/en-us/windows/win32/ipc/synchronous-and-overlapped-input-and-output ; https://learn.microsoft.com/en-us/windows/win32/ipc/anonymous-pipe-operations ; https://learn.microsoft.com/en-us/windows/win32/procthread/creating-a-child-process-with-redirected-input-and-output ; https://learn.microsoft.com/en-us/windows/win32/memory/creating-named-shared-memory ; https://grpc.io/docs/guides/health-checking/ ; https://www.rfc-editor.org/rfc/rfc6455 ; https://learn.microsoft.com/en-us/azure/event-grid/overview

### Data Formats and Standards

데이터 형식도 경계마다 다르게 잡는 편이 맞다. Tauri command는 JSON 직렬화 가능한 데이터가 기본이다. 이건 UI ↔ host에는 충분하다. 그러나 local renderer나 remote renderer로 가면 JSON은 구조 표현은 쉽지만, 큰 payload와 typed evolution 면에서 한계가 있다. gRPC는 protobuf를 IDL과 interchange format으로 함께 쓰고, protobuf 공식 문서는 proto3 guide, encoding, ProtoJSON, techniques를 별도로 제공한다. 따라서 typed service contract가 필요할수록 protobuf가 더 설득력 있다.

flat file와 artifact manifest는 오히려 이번 과제에서 더 중요하다. full-screen close는 메모리 안에서 끝나는 연산이 아니라, 결국 booth가 믿고 올릴 수 있는 artifact 승격 문제이기 때문이다. 따라서 `JSON control envelope + protobuf or binary payload + capture-bound file artifact + evidence manifest` 조합이 가장 현실적이다.

이 영역에서 중요한 점은 `event payload`도 지나치게 비대해지면 안 된다는 것이다. Azure event-driven guidance는 payload에 모든 속성을 다 넣는 방식과 key만 넣는 방식을 비교하면서, 전자는 consistency와 contract complexity 문제가 있고 후자는 추가 조회 비용이 있다고 설명한다. booth hot path에서는 large payload event보다 `key/correlation 중심 event + authoritative artifact lookup`이 더 맞다.

_JSON and XML:_ JSON은 UI/host control path에 적합하고 XML은 기존 XMP adapter 호환 영역에 남는다  
_Protobuf and MessagePack:_ protobuf가 remote service contract와 typed evolution 면에서 더 강하다  
_CSV and Flat Files:_ legacy bulk transfer보다 capture-bound artifact/evidence bundle 쪽이 중요하다  
_Custom Data Formats:_ canonical recipe, evidence manifest, promotion contract는 제품 전용 포맷이 필요할 가능성이 높다  
_Source:_ https://v2.tauri.app/ko/concept/inter-process-communication/ ; https://grpc.io/docs/what-is-grpc/introduction/ ; https://protobuf.dev/programming-guides/ ; https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/event-driven

### System Interoperability Approaches

상호운용성 관점에서 1순위는 여전히 **point-to-point integration**이다. current capture를 current renderer로 보내고, result를 same-path replacement로 승격하는 문제는 불특정 다수 시스템 통합이 아니기 때문이다. 따라서 local point-to-point contract가 가장 product-fit이 높다.

Ambassador pattern은 off-box 구조에서 의미가 커진다. Microsoft는 ambassador를 client와 colocated된 out-of-process proxy로 설명하며, noncontainerized 환경에서는 local process 또는 Windows service로 둘 수 있다고 설명한다. 즉, booth 본체 옆에 `network/routing/security/health/logging`을 전담하는 Windows service를 둘 수 있다. 이것은 remote renderer service를 붙일 때, booth core를 덜 건드리고 네트워크 concern을 밖으로 빼는 방식으로 유용하다.

API gateway는 여러 front-end service가 존재할 때 중앙 entry point로 유효하다. service mesh는 Istio가 설명하듯 traffic management, observability, mTLS, policy를 infra layer에서 제공한다. 하지만 이 둘은 booth 한 대의 hot path 1차 해법이라기보다, remote/off-box 구조가 여러 서비스나 여러 cell로 확장될 때 통제면으로 보는 편이 맞다. ESB 스타일 central bus는 current capture close 문제에는 너무 무겁다.

_Point-to-Point Integration:_ local GPU runtime / artifact promotion 문제에 가장 적합  
_API Gateway Patterns:_ remote fleet or multi-service control plane이 생길 때만 relevant  
_Service Mesh:_ multi-service edge cell이 생기면 고려 가능하지만 1차 기본안은 아니다  
_Enterprise Service Bus:_ hot path보다 enterprise integration control plane에 가까워 우선순위가 낮다  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/ambassador ; https://learn.microsoft.com/en-us/azure/architecture/microservices/design/gateway ; https://istio.io/latest/about/service-mesh/

### Microservices Integration Patterns

현재 주제는 full microservices 전환을 권장하지 않는다. 다만 off-box 구조를 진지하게 연다면 일부 패턴은 선택적으로 필요하다. API gateway는 외부 또는 operator-facing entry point를 정리하는 데 유용하고, health checking은 remote renderer의 gating에 필수다. service discovery는 여러 renderer cell이 생기면 필요해지지만, 단일 booth와 단일 appliance 조합에서는 정적 endpoint나 local discovery로 충분할 가능성이 높다.

Circuit breaker와 retry는 remote/off-box path에서 중요하다. Ambassador pattern은 routing, monitoring, TLS, resiliency를 out-of-process로 옮기는 예를 보여준다. booth hot path에서는 이것을 `renderer proxy service`로 번역할 수 있다. 반면 saga pattern이나 distributed transaction은 지금 기준으로 과하다. 이번 과제의 핵심은 다단계 business transaction이 아니라 `who can promote same-capture artifact`를 통제하는 것이다.

_API Gateway Pattern:_ remote control plane이나 multi-service edge cell에서만 의미가 커진다  
_Service Discovery:_ renderer cell이 동적으로 늘어날 때만 필요성이 커진다  
_Circuit Breaker Pattern:_ remote renderer를 열 경우 필수에 가깝다. 이 평가는 ambassador/gRPC health guidance에 기반한 inference다  
_Saga Pattern:_ 현재 booth capture-close에는 과도하다  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/microservices/design/gateway ; https://learn.microsoft.com/en-us/azure/architecture/patterns/ambassador ; https://grpc.io/docs/guides/health-checking/

### Event-Driven Integration

event-driven는 이번 과제에서 오해하기 쉬운 영역이다. Azure guidance는 event-driven architecture가 producer/consumer decoupling, near-real-time delivery, independent scalability에 강하다고 설명한다. Publisher-subscriber도 다수 consumer fan-out에는 좋다. Event sourcing은 append-only store와 background handling으로 write throughput과 auditability를 높이고, CQRS와 결합하면 read/write 최적화를 분리할 수 있다고 설명한다.

하지만 booth hot path에서 이걸 곧바로 기본 구조로 삼으면 문제가 생긴다. `current capture full-screen close`는 eventual consistency보다 deterministically promoted artifact가 중요하기 때문이다. 따라서 event-driven integration의 올바른 위치는 아래처럼 나누는 편이 맞다.

- **적합:** telemetry, hardware evidence, diagnostics, operator stream, backfill, analytics
- **조건부 적합:** event-sourced audit trail, CQRS read projection
- **부적합 기본안:** current capture full-screen close owner 자체를 broker/pub-sub에 맡기는 것

다만 structural reset이 커지면 CQRS는 의미가 생긴다. Azure CQRS 문서는 single data store를 공유하더라도 read model과 write model을 분리할 수 있다고 설명한다. 따라서 authoritative write는 capture/promotion/evidence stream, read model은 booth full-screen projection과 operator diagnostics projection으로 나누는 구조는 충분히 고려할 가치가 있다.

_Publish-Subscribe Patterns:_ telemetry and fan-out에는 적합, close owner 기본안으로는 부적합  
_Event Sourcing:_ append-only audit/evidence trail로는 강력하다  
_Message Broker Patterns:_ broker는 integration side-work에는 유효하지만 primary close path는 request/response 또는 promotion contract가 낫다  
_CQRS Patterns:_ booth projection과 authoritative write stream 분리에 유효하다  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/event-driven ; https://learn.microsoft.com/en-us/azure/architecture/patterns/publisher-subscriber ; https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing ; https://learn.microsoft.com/en-us/azure/architecture/patterns/cqrs

### Integration Security Patterns

보안 패턴도 local과 remote를 나눠야 한다. Tauri IPC는 메시지 전달 기반이라 shared memory/direct function access보다 안전한 방법이라고 설명한다. 즉 UI ↔ host에서는 capability-gated command/event 경계가 기본이다. local renderer 쪽에서는 pipe/shared-memory/file-commit 조합을 쓰더라도, truth 승격 권한은 host가 쥐고 renderer는 candidate artifact와 evidence만 제출하는 구조가 맞다.

remote/off-box 구조에서는 인증과 전송 보안이 기본으로 들어와야 한다. gRPC auth는 TLS/SSL을 기본 보호 수단으로 설명하고, RFC 8705는 mutual TLS 기반 client authentication과 certificate-bound token을 정의한다. 따라서 remote renderer service를 연다면 `mTLS + optional token/JWT + health/auth separation + promotion authority restriction`이 기본안이다.

Azure Event Grid도 MQTT/HTTP에서 X.509, Entra ID, OAuth 2.0 JWT, TLS 1.2/1.3 같은 인증/보호 모델을 보여준다. 이는 원격 장치/edge cell이 생길 때 event/control plane을 보호하는 참고 패턴이 된다.

_OAuth 2.0 and JWT:_ remote control plane이나 broker auth에는 유효하지만 local hot path 기본값은 아님  
_API Key Management:_ booth hot path보다는 관리용 integration에 가깝다  
_Mutual TLS:_ remote renderer / edge cell에는 사실상 기본  
_Data Encryption:_ local은 process and file boundary control, remote는 TLS and certificate-bound trust가 핵심  
_Source:_ https://v2.tauri.app/ko/concept/inter-process-communication/ ; https://grpc.io/docs/guides/auth/ ; https://www.rfc-editor.org/rfc/rfc8705 ; https://learn.microsoft.com/en-us/azure/event-grid/overview

## Architectural Patterns and Design

### Web Search Analysis

이번 단계에서는 `고위험 기술 옵션`과 `전체 구조 재조정안`을 실제 아키텍처 패턴으로 번역할 수 있는지 확인하기 위해, Microsoft의 아키텍처 스타일/패턴 문서와 Well-Architected 가이드를 중심으로 다시 검증했다. 조사 축은 여섯 가지였다.

- **Architecture style 선택**: `N-tier`, `Web-Queue-Worker`, `Microservices`, `Event-driven`
- **Core design discipline**: `Clean Architecture`, `Ports-and-Adapters`, `Anti-Corruption Layer`
- **Critical flow isolation**: `Bulkhead`, `Priority Queue`, `Queue-Based Load Leveling`, `Pipes and Filters`
- **Control/data plane separation**: control plane responsibilities, data plane runtime boundary
- **Data projection patterns**: `CQRS`, `Materialized View`, bounded `Event Sourcing`
- **Fleet/off-box expansion**: `Deployment Stamps`, safe deployment, progressive exposure

조사 결과, 현재 목표에 대해 구조적으로 의미 있는 후보는 아래 네 가지로 압축된다.

1. **Clean modular core + dedicated native coprocessor**  
   부스 애플리케이션의 product authority는 유지하되, RAW decode/apply/display의 latency-critical lane만 별도 native runtime으로 분리하는 구조
2. **Split-lane architecture with strict bulkheads**  
   `current capture full-screen close`와 `export/backfill/telemetry`를 같은 파이프라인으로 보지 않고, critical flow를 별도 자원/큐/소비자 풀로 격리하는 구조
3. **Bounded CQRS/materialized projection**  
   authoritative write path와 full-screen read projection을 분리해 `display-sized truthful artifact`를 전용 read model로 다루는 구조
4. **Off-box cell or deployment-stamp reset**  
   로컬 경로로는 물리적으로 답이 안 나올 때, booth와 renderer owner를 아예 다른 셀 또는 appliance로 나누는 구조

반대로, 현재 공개 자료를 다시 봐도 아래는 우선순위가 낮다.

- `full microservices decomposition`: 복잡성과 통신 오버헤드가 크고, 현재 팀/제품 구조 대비 얻는 이점이 불확실함
- `broker-first event-driven close owner`: same-capture full-screen close authority를 eventual consistency 쪽으로 밀어버릴 위험이 큼
- `generic sidecar dogma`: sidecar는 유용하지만, 통신 빈도가 매우 높고 독립 스케일이 필요한 경우 최종 해법이 아닐 수 있음
- `same family tuning`: 이미 실패한 local dedicated renderer 계열의 미세조정은 구조적으로 새로운 패턴이 아님

**Research Coverage:**

- architecture styles와 product-fit의 현재 적합성
- clean/hexagonal/anti-corruption 중심의 core boundary 설계
- critical-flow isolation과 background offload pattern의 경계
- data projection / evidence / authoritative write 분리 가능성
- off-box cell과 deployment stamp 운영 모델

**Quality Assessment:**

- **높음:** Azure Architecture Center, Microsoft Learn, Azure Well-Architected, .NET architecture guidance
- **중간:** off-box cell을 booth 제품에 적용했을 때의 실제 경제성
- **한계:** 공개 문서는 `same-capture preset-applied 24인치 full-screen <= 2500ms`를 직접 다루지 않으므로 일부 선택은 sourced fact 위의 architecture inference

### System Architecture Patterns

Microsoft는 architecture style을 `특정 특성을 공유하는 architecture family`로 설명하면서, 각 스타일은 제약과 trade-off를 함께 가져온다고 정리한다. 또한 `Microservices`는 복잡한 도메인과 잦은 변경에 맞지만, service discovery, data consistency, distributed system management 같은 상당한 복잡성을 동반하고 interservice communication overhead가 latency 문제를 만들 수 있다고 경고한다. 반면 .NET 아키텍처 가이드는 비즈니스 애플리케이션이 여전히 single deployment unit 안에서 논리적 분리를 통해 유지보수성을 얻을 수 있다고 설명한다. 이건 이번 과제에서 중요한 시사점을 준다. 지금 필요한 것은 `fleet-wide microservices`가 아니라, **부스 하나의 critical flow를 닫을 수 있는 더 직접적인 runtime topology**다.

따라서 1순위 시스템 패턴은 `all-in-one monolith`도 `full microservices`도 아니다. 가장 설득력 있는 형태는 **clean modular core + dedicated native coprocessor**다. 즉, product/session/promotion authority는 host core가 유지하고, high-risk 연산만 별도 native process 또는 dedicated runtime으로 분리한다. 이 구조는 기존의 `local dedicated renderer`와 비슷해 보일 수 있지만, 아키텍처 관점의 차이는 **renderer를 단순 보조 프로세스가 아니라 close-critical compute owner로 재설계하되, final promotion authority는 host가 유지하는 것**에 있다.

`Sidecar` 패턴은 여기서 부분적으로만 유효하다. Microsoft는 sidecar가 낮은 지연, lifecycle 공유, language independence에 강하다고 설명하지만, 동시에 통신이 매우 빈번할 때는 latency overhead 때문에 부적합할 수 있고, 독립적으로 스케일해야 하면 별도 서비스가 더 낫다고 명시한다. 따라서 renderer를 sidecar처럼 colocated process로 두는 것은 transition pattern이나 per-booth colocation pattern으로는 맞지만, **통신 밀도가 높고 독립 리소스 통제가 핵심이면 최종 형태는 `true sidecar`보다 `dedicated local service` 또는 `off-box cell`에 가까워질 수 있다.** 이 판단은 Sidecar 패턴 문서와 현재 목표의 latency 특성에 기반한 inference다.

오히려 off-box 구조를 연다면 `Deployment Stamps` 또는 cell-based architecture가 더 적합하다. Deployment stamp는 workload 전체를 독립적으로 운영되는 scale unit으로 두는 패턴이며, 각 stamp가 독립 failure domain이 된다고 설명한다. Boothy에 그대로 번역하면, booth 또는 renderer appliance 하나가 하나의 cell/stamp가 되고, same-capture 데이터와 runtime locality도 그 셀 안에 묶는 편이 맞다. `Geode` 같은 전면 active-anywhere 모델은 cross-cell consistency와 traffic routing complexity를 급격히 올리므로 booth capture-close 문제에는 과하다.

정리하면 시스템 패턴 관점의 결론은 아래와 같다.

- **권장 1순위:** `modular monolith + dedicated native coprocessor`
- **권장 2순위:** `off-box renderer cell / deployment stamp`
- **조건부 사용:** `sidecar`는 가까운 lifecycle/host 공유가 필요할 때
- **비권장 기본안:** `full microservices`, `broker-first event-driven close owner`

_Monolithic / N-tier:_ logical separation을 유지한 단일 workload unit은 여전히 유효하다  
_Microservices:_ 현재 booth hot path에는 복잡도와 통신 비용이 크다  
_Sidecar / Coprocessor:_ transition 또는 colocated process로는 유효하지만, 통신 밀도와 독립 스케일 요구가 높으면 최종 해법은 아닐 수 있다  
_Deployment Stamp / Cell:_ off-box reset을 연다면 가장 자연스러운 scale/failure unit이다  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/ ; https://learn.microsoft.com/en-us/dotnet/architecture/modern-web-apps-azure/common-web-application-architectures ; https://learn.microsoft.com/en-us/azure/architecture/patterns/sidecar ; https://learn.microsoft.com/en-us/azure/architecture/patterns/deployment-stamp ; https://learn.microsoft.com/en-us/azure/well-architected/reliability/redundancy

### Design Principles and Best Practices

설계 원칙 관점에서는 `무엇을 어디에 넣을지`보다 `무엇이 무엇에 의존하면 안 되는지`가 더 중요하다. .NET 아키텍처 가이드는 Clean Architecture를 `business logic와 application model을 중앙에 두고 infrastructure가 core에 의존하게 만드는 구조`로 설명한다. 이는 Boothy에도 그대로 적용된다. `capture`, `session`, `promotion authority`, `evidence contract` 같은 제품 핵심 의미는 Application Core 쪽에 남겨야 하고, camera SDK, darktable/XMP, native GPU worker, remote renderer는 모두 adapter/infrastructure concern으로 내려야 한다.

여기서 `Anti-Corruption Layer`는 선택이 아니라 필수에 가깝다. Microsoft는 서로 다른 semantics를 가진 subsystem 사이에 façade 또는 adapter layer를 두어 새 시스템 설계가 외부 시스템에 의해 오염되지 않게 하라고 설명한다. 이 원칙을 현재 상황에 대입하면, camera SDK semantics, darktable/XMP semantics, custom GPU runtime semantics를 제품 core에 직접 퍼뜨리면 또다시 같은 종류의 구조적 제약이 생긴다. 따라서 **camera adapter**, **preset/parity adapter**, **renderer adapter**는 명시적인 anti-corruption layer로 유지해야 한다.

또 하나의 중요한 설계 원칙은 `control plane`과 `data plane`의 분리다. Microsoft는 control plane이 provisioning, configuration, lifecycle, routing 같은 관리 책임을 맡고, control plane 리소스를 data plane과 분리해야 noisy neighbor와 privilege spread를 막을 수 있다고 설명한다. Boothy에선 이 원칙을 이렇게 번역하는 편이 맞다.

- **Data plane:** capture -> decode -> apply -> full-screen replacement -> promotion evidence
- **Control plane:** route policy, preset distribution, rollout, health gating, diagnostics policy, remote placement

이 분리는 단순 운영 편의가 아니라 제품 목표와 직결된다. `current capture full-screen close`는 data plane에서 닫혀야 하며, preset rollout이나 route policy 판단이 그 경로 안으로 깊게 들어오면 다시 latency와 coupling이 커진다. 따라서 architecture purity를 지키는 방식이 아니라, **critical path purity를 지키는 방식**으로 clean architecture를 적용해야 한다.

또한 Microsoft는 architecture style 선택에서 “architectural purity”를 쫓기보다 왜 그 스타일을 선택하는지가 중요하다고 설명한다. 이건 이번 과제의 핵심 경고이기도 하다. 예를 들어 sidecar, microservices, event-driven 모두 문서상으론 합리적이지만, `2.5초 same-capture full-screen`을 해치면 그 패턴은 좋은 패턴이 아니라 잘못 적용된 패턴이다.

_Clean Architecture / Ports-and-Adapters:_ core meaning을 infrastructure로부터 분리해야 한다  
_Anti-Corruption Layer:_ camera SDK, darktable/XMP, custom GPU runtime 모두 제품 core 앞에서 번역되어야 한다  
_Control Plane / Data Plane Separation:_ policy/rollout과 capture-close runtime을 분리해야 한다  
_Pragmatic Constraint Selection:_ 패턴 준수보다 critical path purity가 우선이다  
_Source:_ https://learn.microsoft.com/en-us/dotnet/architecture/modern-web-apps-azure/common-web-application-architectures ; https://learn.microsoft.com/en-us/shows/dotnetconf-2021/clean-architecture-with-aspnet-core-6 ; https://learn.microsoft.com/en-us/azure/architecture/patterns/anti-corruption-layer ; https://learn.microsoft.com/en-us/azure/architecture/guide/multitenant/considerations/control-planes ; https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/

### Scalability and Performance Patterns

성능 패턴 관점에서 가장 중요한 공식 근거는 `Bulkhead`, `Priority Queue`, `Queue-Based Load Leveling`, `Pipes and Filters`다. Bulkhead 문서는 애플리케이션 요소를 격리된 pool로 분리해 failure와 resource exhaustion의 전파를 막고, high-priority consumer에 별도 quality of service를 제공할 수 있다고 설명한다. 이건 Boothy의 현재 목표와 거의 직접적으로 맞닿아 있다. 즉, `current capture full-screen close`는 다른 일들과 같은 pool에 두면 안 된다. **현재 캡처는 전용 bulkhead 또는 최소한 전용 resource budget을 가져야 한다.**

만약 큐를 남겨야 한다면, `Priority Queue` 패턴이 기본이 된다. Microsoft는 priority queue가 높은 우선순위 작업을 먼저 처리하게 하고, multiple consumer pool을 두면 strict performance requirement와 fault isolation에 유리하다고 설명한다. 따라서 `current capture`와 `background export/backfill`이 동시에 존재한다면, 단일 FIFO보다 `high-priority queue + dedicated consumer pool` 쪽이 맞다.

반면 `Queue-Based Load Leveling`은 적용 범위를 조심해야 한다. 이 패턴은 service overload를 피하기 위해 요청을 비동기 큐로 넘기는 데 유용하지만, 문서상으로도 **minimal latency response를 기대하는 경우에는 적합하지 않다.** 따라서 export/backfill/parity verification에는 유효하지만, `2.5초 안에 풀화면 close를 보여줘야 하는 경로`의 기본 구조로 삼으면 안 된다.

`Pipes and Filters`도 비슷하다. Microsoft는 이 패턴이 복잡한 처리를 독립 단계로 나눠 서로 다른 하드웨어에 배치하거나 병렬화할 수 있다고 설명하지만, 동시에 request-response나 초기 요청 안에서 끝나야 하는 작업에는 맞지 않다고 한다. 이건 중요한 경계다. 즉, decode/apply/display/export를 개념적으로는 filter로 나눌 수 있지만, **current capture close path는 broker-separated filters가 아니라 shared-memory 또는 resident runtime 안의 staged execution**으로 구현하는 편이 낫다. 반대로 export/backfill/parity generation은 실제로 pipes-and-filters로 빼는 것이 맞다.

결론적으로 성능 패턴의 핵심은 아래와 같다.

- close path는 `bulkhead + dedicated resource budget`이 기본
- 큐가 있다면 `priority queue`, 없으면 적어도 scheduler priority가 필요
- `queue-based load leveling`은 hot path 뒤에서만 사용
- `pipes-and-filters`는 background lane에는 유효하지만 close authority 기본안은 아님

_Bulkhead / Cell-Based Architecture:_ current capture와 background 작업을 물리적/논리적으로 격리해야 한다  
_Priority Queue:_ 큐를 남긴다면 current capture 전용 우선순위와 소비자 풀이 필요하다  
_Queue-Based Load Leveling:_ export/backfill에는 유효하지만 minimal-latency close path에는 부적합  
_Pipes and Filters:_ stage 분리와 병렬화에는 유효하지만 request-response close path에는 제한적이다  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/bulkhead ; https://learn.microsoft.com/en-us/azure/architecture/patterns/priority-queue ; https://learn.microsoft.com/en-us/azure/architecture/patterns/queue-based-load-leveling ; https://learn.microsoft.com/en-us/azure/architecture/patterns/pipes-and-filters ; https://learn.microsoft.com/en-us/azure/architecture/guide/design-principles/self-healing

### Integration and Communication Patterns

통합 패턴은 이미 앞 단계에서 다뤘지만, 아키텍처 관점에서 다시 정리하면 **local point-to-point data plane + bounded control plane + selective async side-work**가 핵심이다. Architecture style 문서는 event-driven이 decoupling과 scalability에 강하지만 eventual consistency와 ordering 문제가 따른다고 설명한다. 따라서 same-capture close path는 direct authority contract로 유지하고, event-driven은 부가 흐름으로 내려야 한다.

`Gateway Routing`이나 `Gateway Offloading`은 remote/off-box 구조가 생길 때 control plane에 유용하다. Microsoft는 gateway routing이 backend availability와 intent에 따라 요청을 분기하고, advanced deployment models와 platform transition을 지원한다고 설명한다. 그러나 booth 안에서 촬영 직후 한 장의 결과물을 닫는 문제는 gateway가 아니라 **host-owned promotion contract**가 핵심이다. 즉 gateway는 local hot path의 해답이 아니라, `여러 off-box cell 중 어디로 보낼지`, `어떤 stamp를 활성화할지`, `원격 관리/API를 어디서 받는지` 같은 control plane 문제에 더 가깝다.

또한 `Claim-Check`는 이번 과제에서 통합 패턴이자 데이터 패턴이다. Microsoft는 큰 payload를 메시지 시스템 밖의 external store에 두고 token만 흘리는 방식이 메시징 성능과 보안을 모두 개선한다고 설명한다. 이건 메시지 브로커가 없어도 유효하다. Boothy에서는 `raw/full raster payload`를 모든 경계에서 직접 직렬화하기보다, **artifact handle 또는 claim-check**로 넘기고 authoritative file/evidence store에서 찾게 하는 구조가 product-fit이 높다.

따라서 integration architecture의 기본형은 아래와 같이 정리된다.

- **Data plane:** host-owned direct local contract
- **Large payload transfer:** claim-check / shared memory / file-backed artifact
- **Control plane:** gateway/routing/offloading 가능
- **Async side-work:** pub-sub, CQRS projection update, telemetry stream

_Direct Contract:_ same-capture close authority는 point-to-point가 가장 적합하다  
_Gateway Patterns:_ off-box routing과 control plane에서만 비중이 커진다  
_Claim-Check:_ 큰 raster/artifact를 메시지 본문에서 분리하는 기본 패턴이다  
_Async Eventing:_ telemetry, audit, projection update에만 선택적으로 사용해야 한다  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/ ; https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/design-patterns ; https://learn.microsoft.com/en-us/azure/architecture/patterns/claim-check ; https://learn.microsoft.com/en-us/azure/architecture/microservices/design/gateway

### Security Architecture Patterns

보안 패턴은 이번 과제에서 성능과 충돌하는 것이 아니라, 오히려 구조를 더 선명하게 만든다. Microsoft의 Security design patterns 문서는 `Bulkhead`, `Claim Check`, `Sidecar`, `Gatekeeper`, `Gateway Aggregation` 같은 패턴이 segmentation과 authorization을 강화한다고 설명한다. 이 관점에서 가장 중요한 것은 **renderer에게 권한을 너무 많이 주지 않는 것**이다.

local 구조에서는 host가 여전히 promotion authority를 쥐고, renderer는 candidate artifact와 evidence만 제출하는 구조가 least-privilege에 맞다. Microsoft는 least privilege를 `사용자와 애플리케이션이 업무 수행에 필요한 최소 권한만 가져야 한다`고 설명한다. 이 원칙을 적용하면 renderer는 session catalog, operator control, preset policy, final promotion을 모두 가질 이유가 없다. renderer가 가져야 하는 것은 `render`, `publish candidate`, `report evidence` 수준이다.

remote/off-box 구조에서는 security architecture가 더 커진다. control plane 문서는 control plane이 높은 권한을 가지므로 data plane과 분리해야 하고, secrets 접근과 deployment capability abuse를 위협 모델에 포함해야 한다고 설명한다. 따라서 off-box cell을 연다면 `global/stamp control plane`, `renderer data plane`, `promotion authority`를 같은 프로세스나 같은 권한 집합에 두면 안 된다. 여기에 gRPC auth와 mTLS, certificate-bound trust를 얹는 것이 기본형이다.

`Claim-Check`는 보안 측면에서도 직접적인 이점이 있다. 공식 문서는 claim-check가 민감 데이터를 메시지 본문에서 제거하고 tighter access control을 적용할 수 있게 해 준다고 설명한다. RAW/full artifact가 로컬이든 원격이든 민감한 원본이 될 수 있으므로, `metadata message`와 `artifact body`를 분리하는 방식이 맞다.

_Least Privilege:_ renderer는 full product authority를 가져선 안 된다  
_Bulkhead Segmentation:_ current capture lane, background lane, control plane의 blast radius를 분리해야 한다  
_Claim-Check for Sensitive Payloads:_ 큰 image payload와 control metadata를 분리해야 한다  
_Control Plane Isolation:_ off-box 구조에서는 privileged control path와 render data path를 분리해야 한다  
_Source:_ https://learn.microsoft.com/en-us/azure/well-architected/security/design-patterns ; https://learn.microsoft.com/en-us/entra/identity-platform/secure-least-privileged-access ; https://learn.microsoft.com/en-us/azure/architecture/guide/multitenant/considerations/control-planes ; https://learn.microsoft.com/en-us/azure/architecture/patterns/claim-check ; https://grpc.io/docs/guides/auth/ ; https://www.rfc-editor.org/rfc/rfc8705

### Data Architecture Patterns

데이터 아키텍처에서는 `full-screen display artifact`를 단순 캐시로 볼지, 별도 read model로 볼지가 핵심이다. CQRS 문서는 read와 write를 분리해 각각 독립적으로 최적화할 수 있으며, 전통적 CRUD 모델은 lock contention과 query complexity 문제를 만들 수 있다고 설명한다. Materialized view 문서는 미리 계산된 view가 특정 query에 맞춰 효율적인 읽기를 제공하고, specialized cache처럼 동작할 수 있다고 설명한다. 이 둘을 Boothy에 적용하면, `same-capture truthful promotion`은 write concern이고, `24인치 가로 full-screen display`는 read concern이다. 둘을 같은 모델로 억지로 묶을 이유가 없다.

따라서 high-risk이지만 가장 설득력 있는 데이터 패턴은 **bounded CQRS + materialized display projection**이다.

- **Write model:** capture identity, preset recipe, promotion decision, evidence, parity linkage
- **Read model:** current booth screen에 바로 올릴 display-sized truthful artifact와 상태 DTO

이 구조는 `display-sized truthful artifact`를 속임수 캐시가 아니라, **같은 캡처에 대해 의도적으로 만든 read-optimized projection**으로 승격한다는 뜻이다. Microsoft는 materialized view가 단일 또는 소수 query를 위해 tailor-made 되는 것이 자연스럽다고 설명한다. full-screen booth display는 바로 그런 종류의 query다.

반면 `Event Sourcing`은 범위를 제한해야 한다. 공식 문서는 event sourcing이 auditability와 write performance에 강하지만, 채택 비용이 높고 future design decisions를 제약하며 대부분의 시스템에는 전통적 데이터 관리가 충분하다고 경고한다. 따라서 Boothy 전체를 event-sourced system으로 재설계하는 것은 과하다. 대신 **capture evidence / promotion audit / hardware validation trail** 같이 append-only 이력이 직접 가치가 있는 영역에만 bounded event sourcing을 적용하는 편이 맞다.

또한 Deployment stamp 문서는 서로 다른 stamp 사이의 이동이 어렵고 별도 backplane이 필요할 수 있다고 설명한다. 이건 data locality 설계에도 영향을 준다. 만약 off-box cell을 연다면, current capture의 원본/중간산출물/authoritative display artifact는 가급적 한 stamp/cell 안에 머물게 해야 한다. cross-cell migration이 기본 경로가 되면 close latency와 정합성 관리가 동시에 악화된다.

_CQRS:_ authoritative write와 full-screen read projection을 분리하는 bounded 도입이 유효하다  
_Materialized View:_ display-sized truthful artifact는 specialized cache이자 read model로 볼 수 있다  
_Bounded Event Sourcing:_ audit/evidence 영역에는 적합하지만 시스템 전체 도입은 과하다  
_Data Locality:_ off-box로 가더라도 current capture는 한 cell/stamp 안에 머물게 해야 한다  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/cqrs ; https://learn.microsoft.com/en-us/azure/architecture/patterns/materialized-view ; https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing ; https://learn.microsoft.com/en-us/azure/architecture/patterns/deployment-stamp

### Deployment and Operations Architecture

운영 아키텍처는 이번 과제에서 부수적이지 않다. Microsoft의 safe deployment 가이드는 모든 production deployment를 위험으로 보고, progressive exposure, health model, issue detection, immediate halt/recovery를 기본 원칙으로 삼으라고 설명한다. 또한 deployment stamp는 controlled unit of deployment가 될 수 있고, one-stamp-at-a-time rollout으로 safe deployment를 지원할 수 있다고 명시한다. 따라서 구조를 바꾸더라도 운영 모델은 `big bang replacement`가 아니라 **stamp/cell/route 단위의 progressive exposure**가 되어야 한다.

로컬 구조만 유지하는 경우에도 이 원칙은 유효하다. renderer runtime, preset bundle, route policy, evidence rule, health threshold는 각각 따로 보는 것이 아니라 **같은 architecture release unit**으로 취급해야 한다. 그래야 canary와 rollback이 제품 의미를 유지한다. 특히 `full-screen <= 2500ms`, `same-capture`, `wrong-capture 0`, `fidelity drift 없음`, `fallback stability`를 health model에 넣어야 한다.

off-box 구조를 연다면 control plane 설계가 더 중요해진다. Microsoft는 control plane이 resource placement, configuration, lifecycle, long-running workflow orchestration을 담당한다고 설명하고, 복잡한 환경에서는 global control plane과 stamp control plane을 분리할 수 있다고 한다. 이건 renderer appliance 구조에 그대로 적용된다.

- **Global control plane:** booth를 어느 renderer cell/stamp에 붙일지 결정, capacity 관리, rollout/preset 버전 정책
- **Stamp control plane:** 해당 cell 안의 renderer health, cleanup, local placement, recovery
- **Data plane:** current capture render/promotion/runtime

단, Microsoft도 control plane 수를 최소화하라고 권고한다. 따라서 renderer cell이 몇 대 안 되는 단계에서는 다중 control plane을 성급히 도입하기보다, 단일 control plane + cell-local agent 정도로 시작하는 편이 낫다.

운영 아키텍처의 결론은 명확하다.

- **로컬 경로:** health-gated canary와 즉시 off switch가 기본
- **off-box 경로:** deployment stamp/cell 단위 rollout과 capacity planning이 기본
- **모든 경로:** route policy, preset package, runtime binary, evidence rules를 함께 versioned release unit으로 취급

_Progressive Exposure:_ 구조 변경은 한 번에 여는 것이 아니라 단계적으로 노출해야 한다  
_Health-Gated Rollout:_ latency와 same-capture correctness가 release gate가 되어야 한다  
_Deployment Stamps:_ off-box renderer가 생기면 cell/stamp가 배포와 장애 격리의 기본 단위다  
_Control Plane Minimization:_ 복잡성이 커지기 전까지는 control plane 수를 최소화해야 한다  
_Source:_ https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/safe-deployments ; https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/design-patterns ; https://learn.microsoft.com/en-us/azure/architecture/patterns/deployment-stamp ; https://learn.microsoft.com/en-us/azure/architecture/guide/multitenant/considerations/control-planes

## Implementation Approaches and Technology Adoption

### Technology Adoption Strategies

이번 과제의 구현 전략에서 가장 중요한 공식 근거는 `Strangler Fig`, `feature flag`, `safe deployment`, `workload supply chain`이다. Microsoft는 Strangler Fig를 대형 교체 대신 기능을 점진적으로 치환하는 패턴으로 설명하고, Safe Deployment는 작은 incremental release와 progressive exposure, health model, issue detection을 기본 원칙으로 둔다. 또한 feature flag 가이드는 deployment와 exposure를 분리하고 quick off switch를 제공한다고 명시한다. 이 네 가지를 합치면 이번 과제의 기술 도입 방식은 명확해진다. **고위험 기술 자체는 짧게 실험할 수 있지만, 제품 경로 승격은 점진 전환으로만 해야 한다.**

여기서 중요한 예외도 있다. workload supply chain 문서는 `code promotion chain`에 있는 환경에는 엄격한 자동화와 통제가 필요하지만, `sandbox or other exploratory and proof-of-concept environments require less rigor and structure`라고 설명한다. 즉, 이번 과제는 두 개의 다른 adoption 모드로 나뉘어야 한다.

- **R&D / POC 모드:** sandbox 또는 하드웨어 실험 환경에서 짧고 공격적인 탐색 허용
- **Product adoption 모드:** code promotion chain 안에서는 feature flag, 승인 게이트, 증적, 자동 테스트, rollback을 필수로 적용

따라서 adoption strategy의 기본형은 아래와 같다.

1. **짧은 local native/GPU coprocessor POC**
2. **shadow validation with parity/evidence**
3. **feature-flagged canary on real hardware**
4. **health-gated expansion or stop**
5. **로컬 실패가 반복될 때만 off-box cell/stamp POC**

이 전략은 현재 실패 이력과도 맞는다. 이미 낮은 리스크의 route/policy tuning은 소진됐으므로, 다음 실험은 구조적으로 다른 runtime이어야 한다. 하지만 그 실험이 바로 제품 기본 경로가 되어서는 안 된다.

_Gradual Adoption:_ 제품 승격은 점진 전환과 progressive exposure를 기본으로 해야 한다  
_R&D Exception:_ sandbox/POC 환경은 더 낮은 엄격도로 운영할 수 있다  
_Feature Flagging:_ deployment와 exposure를 분리하고 quick off switch를 제공한다  
_Migration Strategy:_ local PoC -> shadow validation -> canary -> expand/stop -> off-box fallback 순서가 가장 현실적이다  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig ; https://learn.microsoft.com/en-us/devops/operate/progressive-experimentation-feature-flags ; https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/safe-deployments ; https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/workload-supply-chain

### Development Workflows and Tooling

개발 워크플로는 실험 속도와 제품 통제를 동시에 가져가야 한다. workload supply chain 가이드는 예측 가능하고 자동화된 파이프라인으로 변경을 테스트하고 환경 간 promotion 해야 한다고 설명한다. GitHub Actions 문서는 `deployment environments`가 승인, branch 제한, protection rule, secret access 제한을 제공한다고 설명하고, workflow syntax 문서는 `GITHUB_TOKEN` 권한을 최소 권한으로 줄일 수 있다고 설명한다. dependency caching은 workflow 속도와 비용을 낮추고, artifacts는 워크플로 종료 후에도 build/test 산출물을 보관할 수 있다.

이걸 현재 과제에 적용하면, 개발 워크플로는 아래처럼 짜는 편이 맞다.

- **CI lane:** Windows build, unit/contract tests, packaging
- **evidence lane:** Playwright report, trace, renderer log, latency measurement bundle 업로드
- **promotion gate:** environment approval + hardware canary evidence 확인
- **policy discipline:** workflow/job별 최소 권한 `permissions`, branch gate, environment secret 분리

즉, 이번 과제에서 GitHub Actions는 단순 CI가 아니라 **증적과 승격을 묶는 통제면**이어야 한다. artifact는 `실패 원인 재구성`, `하드웨어 검증 추적`, `릴리즈 비교`를 위해 필수다. caching은 실험 속도를 높이되 제품 의미를 바꾸지는 않으므로, build latency만 줄이는 부차적 최적화로 봐야 한다.

개발 도구 측면에서 이미 step 2에서 정리한 `PIX`, `WPR/WPA`, `ETW`, `Nsight` 같은 프로파일링 체인은 여전히 중요하다. 구현 단계에서는 이 도구들이 단순 디버깅이 아니라, `왜 2500ms를 못 넘겼는지`를 증명하는 근거가 된다. 이 판단은 step 2의 기술 스택 조사와 이번 supply chain/evidence 조사에 기반한 inference다.

_CI/CD Pipelines:_ build/test/promotion을 자동화된 파이프라인으로 묶어야 한다  
_Deployment Environments:_ 승인, branch 제한, secret gating에 유효하다  
_Artifacts:_ build/test/trace/evidence를 저장하고 비교하는 데 필수다  
_Permissions Discipline:_ `GITHUB_TOKEN` 최소 권한 설정이 필요하다  
_Source:_ https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/workload-supply-chain ; https://docs.github.com/en/actions/concepts/workflows-and-actions/deployment-environments ; https://docs.github.com/en/actions/tutorials/store-and-share-data ; https://docs.github.com/en/actions/concepts/workflows-and-actions/dependency-caching ; https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax

### Testing and Quality Assurance

테스트는 이번 과제에서 구현 내부를 맞추는 수단이 아니라, **제품 합격 기준을 직접 검증하는 장치**여야 한다. Azure testing 가이드는 테스트 전략과 계획이 business objectives에 정렬돼야 하며, release별로 entry/exit criteria를 가져야 한다고 설명한다. Playwright는 user-visible behavior를 검증하라고 권고하고, trace viewer는 CI에서 `on-first-retry` trace를 남겨 실패를 시각적으로 재구성할 수 있게 한다.

따라서 품질 전략의 최상위 acceptance는 아래 한 줄이어야 한다.

- `same-capture preset-applied 24인치 가로 full-screen <= 2500ms`

그 아래에 필요한 테스트 계층은 다음과 같다.

- **계약 테스트:** capture identity, preset recipe, promotion evidence contract
- **렌더링 검증:** same-capture, wrong-capture 0, fidelity drift 없음
- **E2E:** Playwright로 실제 화면 교체와 사용자 가시 결과 검증
- **실장비 측정:** booth 모니터 기준 replacement latency 실측
- **실패 증적:** trace, logs, timings, artifact snapshot

이 구조의 핵심은 테스트를 많이 만드는 것이 아니라, **제품 실패를 빠르게 판정하는 것**이다. Playwright의 “user-visible behavior” 원칙은 이번 과제에서 특히 중요하다. 내부 함수나 intermediate state가 아니라, 최종 화면에 무엇이 언제 보였는지를 검증해야 하기 때문이다.

또한 trace는 단순 디버깅 편의가 아니다. Playwright는 첫 retry에서 trace를 자동 수집하도록 권장하므로, flaky하거나 간헐적인 close path 실패를 재구성하는 데 유효하다. 이를 artifacts와 같이 보관하면 canary 단계의 품질 게이트가 훨씬 더 강해진다.

_Testing Strategy:_ 테스트 전략과 계획은 business objective에 맞춰야 한다  
_Primary Acceptance:_ `same-capture full-screen <= 2500ms`가 유일한 합격 기준이다  
_User-Visible Validation:_ 테스트는 사용자에게 보이는 결과를 검증해야 한다  
_Trace Evidence:_ 실패 시 trace와 artifacts를 남겨 재구성을 가능하게 해야 한다  
_Source:_ https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/testing ; https://playwright.dev/docs/best-practices ; https://playwright.dev/docs/trace-viewer-intro ; https://docs.github.com/en/actions/tutorials/store-and-share-data

### Deployment and Operations Practices

운영 관점에서는 `고위험 실험 허용`과 `제품 노출 보수성`을 분리하는 것이 핵심이다. Safe Deployment 문서는 모든 production deployment를 위험으로 보고, 작은 변경, progressive exposure, health model, issue detection, immediate halt/recovery를 요구한다. feature flags 문서는 runtime에서 기능을 켜고 끄며, 문제가 생기면 redeploy 없이 trusted behavior로 즉시 되돌릴 수 있다고 설명한다.

따라서 운영 패턴의 기본형은 다음과 같다.

- **실험 환경:** fast iteration, less rigor
- **promotion chain:** quality-gated release only
- **runtime control:** feature flag / route policy / quick off switch
- **observability:** traces, metrics, logs, correlation IDs
- **incident process:** detection, containment, triage, RCA, postmortem, drills

OpenTelemetry는 signals로 traces, metrics, logs를 제공하고 collector 배치 패턴까지 문서화하고 있으므로, renderer/host/control plane의 관측 계층으로 적합하다. incident response 가이드는 detection, containment, triage, root cause analysis, postmortem, regular drills를 구조화하라고 권고한다. 이는 고위험 canary에서 특히 중요하다. 목표를 못 닫는 문제는 단순 버그가 아니라 product-critical incident로 다뤄야 하기 때문이다.

운영 실무 수준의 권고는 아래와 같다.

- canary health gate에 `<=2500ms`, `same-capture`, `wrong-capture 0`, `fallback stability`, `fidelity drift 없음`을 넣는다
- 실패 시 즉시 flag off 또는 route rollback이 가능해야 한다
- incident playbook을 `latency breach`, `wrong capture`, `renderer stall`, `fidelity mismatch` 별로 분리한다

_Safe Deployment:_ production 변경은 모두 위험으로 보고 quality gate를 통과해야 한다  
_Feature Flags:_ quick off switch와 runtime exposure control을 제공한다  
_Observability:_ traces, metrics, logs 기반의 correlated evidence가 필요하다  
_Incident Readiness:_ detection, containment, triage, RCA, drill이 필요하다  
_Source:_ https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/safe-deployments ; https://learn.microsoft.com/en-us/devops/operate/progressive-experimentation-feature-flags ; https://opentelemetry.io/docs/concepts/ ; https://learn.microsoft.com/en-au/azure/well-architected/operational-excellence/incident-response

### Team Organization and Skills

DevOps culture 가이드는 workload team이 운영을 end-to-end로 소유해야 하고, shared ownership과 accountability가 중요하다고 설명한다. 이번 과제는 특히 그 원칙이 강하게 적용된다. 이유는 성능 목표가 `frontend`, `host`, `renderer`, `preset truth`, `hardware validation`, `operations`를 동시에 걸치기 때문이다. 이 중 하나라도 외부 의존 부서로 던져지면 판단이 늦어지고, 증적 해석이 끊어진다.

따라서 팀 구조는 `전문화된 사일로`보다 **한 개의 cross-functional strike cell**이 맞다. 역할은 분리할 수 있지만 ownership은 분리하면 안 된다.

- rendering/runtime
- host/session/promotion contract
- hardware validation/evidence
- deployment/observability

의사결정 방식도 중요하다. DevOps culture 가이드는 역할과 decision authority를 분명히 하되, disagreement가 있을 때는 evidence 기반으로 final call을 내려야 한다고 설명한다. 이번 과제에서 이 원칙은 `예쁘다/빠르다`가 아니라 `실측 evidence가 있는가`로 바뀌어야 한다.

필요 역량은 다음 네 계열로 압축된다.

- Windows native/GPU runtime 역량
- contract/evidence/promotion 설계 역량
- CI/CD 및 관측/incident 운영 역량
- hardware-in-the-loop 검증 역량

_Shared Ownership:_ workload team이 end-to-end 운영 책임을 가져야 한다  
_Cross-Functional Cell:_ renderer, host, hardware, ops를 같은 셀 안에 묶는 편이 맞다  
_Evidence-Based Decisions:_ 최종 판단은 실측과 증적으로 해야 한다  
_Skill Profile:_ native/GPU, contract design, ops, hardware validation 역량이 동시에 필요하다  
_Source:_ https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/devops-culture ; https://learn.microsoft.com/en-au/azure/well-architected/operational-excellence/incident-response

### Cost Optimization and Resource Management

비용 최적화는 이번 과제에서 `최소 비용`이 아니라 `목표 대비 투자 정당성`으로 봐야 한다. Microsoft는 cost-optimized workload가 반드시 low-cost workload는 아니며, ROI와 trade-off를 함께 고려해야 한다고 설명한다. 즉, 로컬 경로를 끝까지 밀지 않고 너무 일찍 off-box나 appliance 구조로 가는 것은 비용과 복잡도를 동시에 끌어올릴 수 있다.

따라서 자원 관리 기본 원칙은 아래와 같다.

- 먼저 `로컬 경로`에서 병목 제거 가능성을 끝까지 검증
- artifact/cache/profiling으로 **무엇이 비용을 만든는지** 계측
- 반복 실패가 확인될 때만 off-box cell/stamp로 이동
- off-box를 열면 장비비, 운영비, 배포비, 장애 대응비까지 함께 계산

또한 cost principle 문서는 tactical cost cutting보다 continuous monitoring과 repeatable process가 중요하다고 설명한다. 이건 이번 과제에서도 같다. 빠른 실험을 위해 비공식 도구와 수동 절차를 많이 늘리면, 나중에 제품 승격 시 운영비가 급격히 커진다. 따라서 POC는 자유롭게 하되, product path에 가까워질수록 운영 구조를 표준화해야 한다.

_ROI Focus:_ low cost보다 목표 달성 대비 투자 효율이 중요하다  
_Local First Economics:_ 로컬 실패 근거가 쌓이기 전엔 off-box 투자가 이르다  
_Continuous Cost Monitoring:_ 비용은 반복적으로 관측하고 조정해야 한다  
_Resource Management:_ compute, storage, operator overhead를 함께 봐야 한다  
_Source:_ https://learn.microsoft.com/en-us/azure/well-architected/cost-optimization/principles ; https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/safe-deployments

### Risk Assessment and Mitigation

이번 과제의 구현 리스크는 기술 난이도보다 `잘못된 승격`에서 더 크게 터질 수 있다. 따라서 risk register는 아래처럼 잡는 편이 맞다.

- **R1. 목표 미달 지속:** local coprocessor가 여전히 `<=2500ms`를 못 맞춤
- **R2. Wrong capture:** 빠르지만 다른 사진이 올라감
- **R3. Fidelity drift:** 프리셋 적용 결과가 truth/parity 경로와 다름
- **R4. Ops fragility:** 새 runtime이 canary에서는 되지만 field에서 자주 꺼짐
- **R5. Off-box overreach:** 로컬 검증이 부족한 상태에서 분산 구조로 조기 확장

완화 전략은 각 리스크별로 분명해야 한다.

- `R1`: time-boxed POC와 stop rule 설정
- `R2`: capture identity contract와 promotion authority host 유지
- `R3`: shadow validation과 parity evidence 유지
- `R4`: health gate + trace/log/artifact bundle + incident playbook
- `R5`: off-box는 local repeated failure 이후에만 진입

이 중 `R2`와 `R3`는 속도보다 더 치명적이다. 이 판단은 기존 문서 이력과 현재 product objective에 기반한 inference다. 2.5초를 맞춰도 wrong-capture나 fidelity mismatch가 있으면 제품적으로는 실패다.

_Primary Risks:_ latency miss, wrong capture, fidelity drift, operational fragility, off-box overreach  
_Mitigation Pattern:_ stop rule, host authority, shadow validation, health gate, phased escalation  
_Decision Rule:_ 속도만이 아니라 correctness와 stability를 함께 봐야 한다  
_Source:_ https://learn.microsoft.com/en-us/azure/well-architected/operational-excellence/safe-deployments ; https://learn.microsoft.com/en-au/azure/well-architected/operational-excellence/incident-response ; https://learn.microsoft.com/en-us/devops/operate/progressive-experimentation-feature-flags

## Technical Research Recommendations

### Implementation Roadmap

권장 구현 로드맵은 아래와 같다.

1. **Phase 0 - Sandbox POC**
   `local native/GPU coprocessor`로 current capture close path만 별도 구현한다. 합격 기준은 처음부터 `same-capture preset-applied full-screen <= 2500ms`로 고정한다.
2. **Phase 1 - Shadow Validation**
   기존 truth/parity 경로는 유지하고, 새 lane의 결과를 같은 캡처에 대해 비교 수집한다. wrong-capture, fidelity drift, fallback behavior를 같이 본다.
3. **Phase 2 - Canary on Real Hardware**
   feature flag와 route policy 뒤에서 제한된 하드웨어/세션에만 노출한다. health gate를 넘지 못하면 즉시 중단한다.
4. **Phase 3 - Controlled Expansion**
   반복 측정에서 기준을 넘기면 범위를 확장한다. renderer binary, preset bundle, route policy, evidence rules를 같은 release unit으로 관리한다.
5. **Phase 4 - Off-box Fallback POC**
   local repeated failure가 확인된 뒤에만 single renderer cell/stamp POC로 간다.

### Technology Stack Recommendations

- **주력 실험 스택:** Rust host + native renderer + Windows GPU runtime + shared-memory/file-backed artifact
- **증적/운영 스택:** GitHub Actions environments/artifacts, Playwright, OpenTelemetry
- **유지 경로:** 기존 truth/parity lane은 fidelity oracle과 fallback으로 유지
- **예비 스택:** remote renderer는 gRPC/protobuf/mTLS + single cell/stamp로만 시작

### Skill Development Requirements

- Windows native/GPU profiling과 runtime 구현 역량
- capture identity / promotion authority / evidence contract 설계 역량
- CI/CD, observability, incident 대응 역량
- hardware-in-the-loop 측정 및 판정 역량

### Success Metrics and KPIs

- `same-capture preset-applied full-screen <= 2500ms`
- `wrong-capture = 0`
- `preset fidelity mismatch = 0`
- `fallback stability`가 canary 기준 통과
- `health gate pass rate`가 반복 측정에서 유지
- `artifact/trace/log evidence completeness` 확보

## Research Synthesis

### Strategic Technical Synthesis

이번 리서치 전체를 한 문장으로 요약하면 이렇다. **지금 필요한 것은 “더 빠른 preview”가 아니라, “같은 캡처의 프리셋 적용 결과를 2.5초 안에 full-screen으로 닫는 전용 구조”다.** 기존 이력상 이미 시도된 것은 주로 `preview 가시성`, `recent-session`, `local dedicated renderer`, `GPU-first activation` 계열이었고, 이들은 제품 경로 전환에는 성공했지만 최종 속도 목표를 닫지 못했다. 따라서 다음 선택은 `동일 계열 미세조정`이 아니라 `구조적으로 다른 close path`여야 한다.

공식 패턴과 내부 이력을 합친 최종 1순위 구조는 아래와 같다.

- `modular host core`
- `dedicated local native/GPU coprocessor`
- `current capture 전용 bulkhead`
- `host-owned promotion authority`
- `bounded CQRS + materialized display projection`
- `truth/parity lane retained as oracle + fallback`

이 구조의 장점은 제품 계약을 깨지 않으면서 병목을 가장 직접적으로 옮길 수 있다는 점이다. `display-sized truthful artifact`를 read-optimized projection으로 승격하면, full-screen close path는 export/backfill/parity와 분리된다. 반면 same-capture correctness와 preset fidelity는 host authority와 shadow validation으로 계속 통제할 수 있다.

### Final Architecture Decision Framework

최종 의사결정 프레임은 아래와 같다.

- **Go 1:** local native/GPU coprocessor가 반복 실장비 측정에서 `<=2500ms`를 달성한다
- **Go 2:** `wrong-capture = 0`, `preset fidelity mismatch = 0`, `fallback instability 없음`
- **Go 3:** canary health gate와 evidence completeness가 유지된다
- **No-Go 1:** 구조가 달라도 반복 측정에서 `<=2500ms`를 못 넘긴다
- **No-Go 2:** 속도는 개선되지만 correctness 또는 fidelity가 흔들린다
- **Next Escalation:** 위 No-Go가 반복될 때만 `single off-box renderer cell/stamp` POC로 이동

즉, 다음 단계는 여러 대안을 병렬로 더 넓게 여는 것이 아니라, **가장 가능성 있는 한 개의 구조적 베팅을 짧고 엄격하게 검증하는 것**이다.

### What Should Be Stopped

이번 리서치 기준으로 이제 우선순위를 낮춰야 할 항목은 아래와 같다.

- 기존 `local dedicated renderer` 계열의 추가 미세조정
- `queue/warm-state/policy tuning`만으로 해결될 것이라는 가정
- `full microservices` 또는 `broker-first close owner`
- `first-visible` 또는 `small preview`를 목표 달성으로 간주하는 해석

이들은 공식 패턴 관점에서도 맞지 않고, 내부 실패 이력과도 구조적으로 같은 문제를 반복할 가능성이 높다.

### Future Technical Outlook and Innovation Opportunities

단기적으로는 `Windows native/GPU runtime`과 `display projection architecture`가 가장 중요한 혁신 기회다. 중기적으로는 로컬 resident lane이 실패할 경우를 대비해 `single off-box renderer cell/stamp`를 검토할 수 있다. 장기적으로는 renderer cell과 booth를 하나의 repeatable deployment unit으로 다루는 운영 모델까지 갈 수 있지만, 현재 단계에서 거기까지 확장하는 것은 시기상조다.

가장 중요한 점은 기술 트렌드를 좇는 것이 아니라, **현재 제품 계약을 지키면서 병목을 실제로 이동시키는 구조를 선택하는 것**이다. 이 기준으로 보면 지금 투자할 가치가 있는 영역은 `native/GPU coprocessor`, `artifact/read-model separation`, `hardware evidence pipeline`이다.

## Technical Research Methodology and Source Verification

### Primary Sources

이번 리서치는 아래 성격의 1차 자료를 우선 사용했다.

- Microsoft Learn / Azure Architecture Center / Azure Well-Architected
- GitHub Actions 공식 문서
- Playwright 공식 문서
- OpenTelemetry 공식 문서
- gRPC 공식 문서
- Tauri 공식 문서
- WIC, LibRaw, OpenImageIO, Adobe 관련 공식 문서
- 내부 Boothy planning / validation / reassessment 문서

### Source Verification Approach

- 현재성이 중요한 항목은 모두 최신 공식 웹 문서로 재검증했다.
- 제품 적합성 평가는 내부 실패 이력과 공식 패턴을 함께 사용했다.
- 공식 문서가 직접 답하지 않는 부분은 `inference`로 취급해 문서 안에서 명시적으로 구분했다.
- `same-capture preset-applied 24인치 full-screen <= 2500ms`를 직접 증명하는 공개 benchmark는 찾지 못했으므로, 최종 판정은 실장비 검증이 필요하다.

### Confidence and Limitations

- **High confidence:** 아키텍처 패턴 선택의 방향성, 점진 전환/flag/health gate 필요성, CQRS/materialized projection의 정당성
- **Medium confidence:** local native/GPU coprocessor의 실제 달성 확률
- **Lower confidence:** off-box cell/stamp가 현재 장비/현장 조건에서 가지는 ROI

이 리서치의 가장 큰 한계는 공개 자료가 Boothy의 정확한 하드웨어/프리셋/카메라 조합에 대한 절대 수치를 제공하지 않는다는 점이다. 그래서 본 문서는 `어떤 구조가 더 설득력 있는가`를 답할 수는 있지만, `반드시 성공한다`를 보장하지는 않는다.

## Technical Research Conclusion

### Summary of Key Findings

이번 리서치는 `방법이 남아 있는가`보다 `어떤 종류의 방법만 아직 남아 있는가`를 정리한 문서다. 결론은 명확하다. **저위험 해법은 사실상 소진되었고, 남아 있는 것은 구조적으로 다른 close path를 만드는 고위험 베팅뿐이다.** 그중에서도 가장 설득력 있는 다음 시도는 `local native/GPU coprocessor + dedicated full-screen lane`이다.

### Strategic Impact Assessment

이 결론은 제품 의사결정에도 직접 연결된다. 이제부터는 단순 구현 작업이 아니라 `고위험 R&D`로 다뤄야 하며, 성공 여부는 코드 품질이 아니라 `same-capture`, `preset fidelity`, `full-screen <= 2500ms`, `fallback stability`를 함께 만족하는 실장비 증적으로만 판정해야 한다. 즉, 앞으로의 핵심 자산은 기능 그 자체보다 **증명 가능한 evidence pipeline**이다.

### Next-Step Technical Recommendation

다음 실행은 하나면 충분하다.

1. `local native/GPU coprocessor` POC를 짧은 기간 안에 검증한다.
2. 합격 기준은 처음부터 `same-capture preset-applied full-screen <= 2500ms`로 고정한다.
3. shadow validation으로 correctness와 fidelity를 동시에 본다.
4. 반복 실패가 확인될 때만 `single off-box renderer cell/stamp`로 구조를 한 단계 올린다.

---

**Technical Research Completion Date:** 2026-04-14  
**Research Period:** current comprehensive technical analysis  
**Source Verification:** current official documentation and internal validation history  
**Technical Confidence Level:** Medium-High for direction, lower for absolute outcome until hardware validation
