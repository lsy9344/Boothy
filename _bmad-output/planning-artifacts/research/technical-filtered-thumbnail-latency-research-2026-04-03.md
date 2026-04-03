---
stepsCompleted: [1, 2, 3, 4, 5, 6]
inputDocuments:
  - 'C:\Code\Project\Boothy\history\recent-session-thumbnail-speed-brief.md'
workflowType: 'research'
lastStep: 6
research_type: 'technical'
research_topic: '최근 세션 썸네일에서 필터 적용 사진을 최대한 빨리 보여주는 기술'
research_goals: '같은 촬영의 필터 적용 결과를 최근 세션 썸네일에 더 빠르게 노출하기 위해 cold start, preview render warm-up, preset apply elapsed, fast preview/fallback 구조를 기술적으로 분석하고 최적화 방향을 도출한다'
user_name: 'Noah Lee'
date: '2026-04-03'
web_research_enabled: true
source_verification: true
---

# Research Report: technical

**Date:** 2026-04-03
**Author:** Noah Lee
**Research Type:** technical

---

## Research Overview

이 리서치는 Boothy에서 테더링 직후 **선택된 프리셋이 적용된 같은 촬영 결과를 사용자에게 얼마나 빨리 첫 신뢰 화면으로 보여줄 수 있는지**를 파악하기 위해 수행했다. 조사 범위는 카메라 제어 SDK, preview/cache 전략, 로컬 렌더 엔진과 sidecar 구조, edge 전환 가능성, observability, 테스트 및 롤아웃 전략까지 포함했다. 모든 핵심 판단은 현재 공개된 공식 문서와 1차 자료를 기준으로 교차 검증했다.

핵심 결론은 전면 재구축보다 **현재 앱 셸을 유지한 채 `preset-applied first-visible path`만 전용 저지연 렌더 경로로 점진 치환하는 전략**이 가장 현실적이라는 점이다. 현재 기술로 충분하지 않을 경우에는 `로컬 dedicated renderer`, `watch-folder 기반 외부 렌더 엔진 브리지`, `edge render appliance` 순으로 전환 폭을 키우는 것이 적절하다. 반대로 큐 중심 구조, 느린 파일 대기, 복잡한 분산 서비스 체인은 현재 제품 목표인 "바로바로 보이는 체감"과 잘 맞지 않는다.

자세한 종합 결론, 기술 선택 기준, 위험과 로드맵은 아래 **Research Synthesis**의 Executive Summary, Strategic Technical Recommendations, Implementation Roadmap 섹션을 참조한다.

---

## Technical Research Scope Confirmation

**Research Topic:** 테더링된 촬영 결과물에 선택된 프리셋을 최대한 빨리 적용해 이질감 없이 보여주는 기술
**Research Goals:** 같은 촬영의 필터 적용 결과를 사용자가 거의 즉시 보게 만들기 위해, preset-applied preview 경로를 가장 빠르게 만드는 기술적 선택지와 구조를 도출한다. 필요하면 렌더 엔진, 테더링 파이프라인, RAW 처리 방식, 플랫폼 구조까지 바꾸는 대안도 포함한다.

**Technical Research Scope:**

- Architecture Analysis - preset-applied first visible result를 빠르게 만드는 구조
- Implementation Approaches - cold start, warm-up, preset apply elapsed를 줄이는 구현 방식
- Technology Stack - 카메라 SDK, helper/host bridge, RAW 처리 엔진, 프런트 상태 연결
- Integration Patterns - camera -> helper -> host -> UI 전달 경로와 병목
- Performance Considerations - first-visible latency, preset-applied latency, fallback 비용

**Research Methodology:**

- Current web data with rigorous source verification
- Multi-source validation for critical technical claims
- Confidence level framework for uncertain information
- Comprehensive technical coverage with architecture-specific insights

**Scope Confirmed:** 2026-04-03

## Technology Stack Analysis

### Programming Languages

이 주제에서 언어 선택의 핵심은 "무슨 언어가 유행인가"보다 "어떤 계층을 어떤 언어로 맡길 것인가"에 가깝다. 확인한 1차 자료 기준으로, **RAW 디코드와 프리뷰/렌더의 hot path는 여전히 C/C++ 계열 라이브러리와 네이티브 최적화가 중심**이다. darktable은 C 기반의 오픈소스 raw developer이며, RawSpeed는 빠른 RAW 디코딩 라이브러리로 C++ 구현을 유지하고 있고, LibRaw 역시 RAW 데이터와 임베디드 프리뷰/썸네일 추출을 위한 네이티브 라이브러리다. Canon의 EDSDK/CCAPI 역시 여러 플랫폼에서 카메라 제어와 데이터 전송을 여는 네이티브 API 계층을 제공한다.

_Popular Languages:_ C/C++는 카메라 SDK, RAW decode, preview extraction, GPU-friendly imaging core에서 사실상 표준에 가깝다.  
_Emerging Languages:_ 조사한 1차 자료 범위에서는 RAW hot path의 주류가 C/C++에서 다른 언어로 빠르게 이동하고 있다는 근거는 약하다. 변화는 언어 교체보다 GPU offload, 캐시, preview format 대응에서 더 많이 보인다.  
_Language Evolution:_ 제품 전체는 다계층 구조가 유력하다. 즉, UI/상태 관리는 Rust/TypeScript/C# 같은 생산성 계층이 맡고, 같은 촬영의 빠른 필터 적용 결과를 만드는 핵심 경로는 네이티브 라이브러리 또는 네이티브 엔진이 담당하는 형태가 현재 시장/오픈소스 모두에서 더 현실적이다. 이 판단은 공식 자료들로부터의 추론이다.  
_Performance Characteristics:_ sub-second에 가까운 first-visible preset-applied result를 노리면, 메모리 접근·SIMD·OpenMP/OpenCL·GPU 연동이 쉬운 네이티브 언어 계층이 유리하다. 반대로 전체 렌더 파이프라인을 상위 런타임에서 직접 구현하는 방식은 불리할 가능성이 높다. 이것도 공식 자료들의 공통 패턴으로부터의 추론이다.  
_Source:_ https://github.com/darktable-org/darktable, https://github.com/darktable-org/rawspeed, https://www.libraw.org/about, https://www.usa.canon.com/support/sdk

### Development Frameworks and Libraries

이번 주제에서 가장 중요한 프레임워크/라이브러리 축은 네 가지다. 첫째, **카메라 제어 계층**으로 Canon EDSDK/CCAPI 또는 libgphoto2/gphoto2 계열이 있다. Canon은 EDSDK와 CCAPI로 원격 제어와 즉시 데이터 전송을 열고, CCAPI는 무선과 다중 플랫폼 범위를 넓힌다. darktable은 tethering에서 gphoto2를 사용한다. 둘째, **빠른 원본 접근 계층**으로 LibRaw와 RawSpeed가 갈린다. RawSpeed는 "매우 빠른 1단계 RAW decode"에 강하지만 viewable image/thumbnail을 직접 주는 라이브러리는 아니다. LibRaw는 임베디드 프리뷰/썸네일 추출과 기본 RAW 변환을 제공하지만, 스스로도 production-quality rendering은 주기능이 아니라고 밝힌다.

셋째, **프리셋 적용 렌더 엔진 계층**이 있다. Lightroom Classic은 테더 촬영 시 import 시점에 Develop preset 적용을 지원하고, GPU와 Camera Raw cache로 Develop/Library 쪽 표시와 조정을 가속한다. Capture One은 더 직접적으로, tethered capture 중 `Immediately` 모드에서 "adjustments are applied while quickly rendered preview를 먼저 보여주는" 제품 동작을 공식 문서에 노출한다. 또한 `Next Capture Adjustments`로 캡처 시점에 조정과 스타일을 자동 적용한다. 넷째, **GPU 렌더 파이프라인 계층**으로 darktable의 OpenCL pixelpipe처럼 preview/full pipe를 CPU/GPU에 다르게 스케줄링하는 구조가 있다.

_Major Frameworks:_ Canon EDSDK/CCAPI, libgphoto2/gphoto2, LibRaw, RawSpeed, darktable/OpenCL pixelpipe, Adobe Camera Raw/Lightroom Classic, Capture One tethered capture stack.  
_Micro-frameworks:_ 단독으로 "같은 촬영의 preset-applied first visible result"를 완성하는 단일 라이브러리는 드물다. 현실적인 조합은 `camera SDK + embedded preview extraction + fast render engine + cache/profile system`이다. 이 조합 판단은 공식 자료 종합에 따른 추론이다.  
_Evolution Trends:_ 최신 추세는 "full-quality render가 준비될 때까지 기다리는 단일 경로"보다, `빠른 프리뷰`와 `품질 우선 프리뷰`를 분리하는 방향이다. Capture One의 `Immediately` / `When ready` 구분은 이 전략을 제품 차원에서 노출한 사례다.  
_Ecosystem Maturity:_ Canon SDK, Adobe, Capture One은 제품화 수준이 높고, darktable/LibRaw/RawSpeed/gphoto2는 조합 자유도가 높다. 다만 오픈소스 조합은 first-visible latency를 직접 설계해야 하고, 상용 제품은 이미 그런 UX를 제품 정책으로 내장하고 있다.  
_Source:_ https://www.usa.canon.com/support/sdk, https://downloads.canon.com/sdk/CameraControlAPI_OperationGuide_EN.pdf, https://docs.darktable.org/usermanual/4.0/en/tethering/overview/, https://www.libraw.org/about, https://github.com/darktable-org/rawspeed, https://helpx.adobe.com/lightroom-classic/help/import-photos-tethered-camera.html, https://helpx.adobe.com/lightroom-classic/kb/lightroom-gpu-faq.html, https://helpx.adobe.com/lightroom-classic/kb/optimize-performance-lightroom.html, https://support.captureone.com/hc/en-us/articles/360002556797-Selecting-the-appropriate-capture-preview, https://support.captureone.com/hc/en-us/articles/360002556798-Adding-adjustments-automatically-while-capturing

### Database and Storage Technologies

이 문제에서 저장소 기술의 핵심은 전통적인 서버 DB보다 **catalog + cache + sidecar + preview store**다. Lightroom Classic은 catalog에 변경사항을 저장하고, Camera Raw cache에 원본 이미지 데이터를 보관해 preview generation의 초기 단계를 건너뛸 수 있다고 설명한다. 또한 XMP autowrite는 다른 앱과 동기화에는 편하지만 성능을 크게 떨어뜨릴 수 있다고 명시한다. Capture One은 monitor 해상도에 맞는 preview size 설정이 preview generation과 adjustment display 속도에 직접 영향을 준다고 밝히며, 16.3 이후 improved previews로 생성 속도와 저장 공간을 함께 개선했다고 설명한다. darktable은 thumbnail cache, full preview cache, precompiled OpenCL binary cache를 유지하고, RawTherapee는 thumbnail/metadata/sidecar/embedded profile을 묶은 cache set을 저장한다.

_Relational Databases:_ Lightroom식 catalog DB는 세션/메타데이터/히스토리 관리에는 유효하지만, `촬영 직후 필터 적용 결과를 언제 보이게 하느냐`의 직접 병목은 대개 catalog보다 preview cache와 render pipeline에 있다. 이 판단은 Adobe와 darktable 문서를 종합한 추론이다.  
_NoSQL Databases:_ 이번 조사 범위의 대표 제품/오픈소스는 live tethered preset-applied preview의 핵심 저장계층으로 NoSQL을 전면에 두지 않는다. 이 문제의 1차 저장 병목은 DB 종류보다 preview asset과 sidecar/cache 정책에 가깝다.  
_In-Memory Databases:_ Redis류보다는 메모리 캐시, GPU memory, image cache가 훨씬 직접적이다. darktable은 메모리/디스크 preview cache를 분리하고, Capture One과 Lightroom은 각각 preview/image cache를 성능 레버로 둔다.  
_Data Warehousing:_ 분석/리포팅에는 쓸 수 있지만, booth 현장의 sub-second 미리보기에는 부적합하다. 이 문제의 핵심은 저장소 조회보다 파일 도착 후 프리셋 적용 프리뷰를 얼마나 빨리 만들고 재사용하느냐다.  
_Source:_ https://helpx.adobe.com/lightroom-classic/kb/optimize-performance-lightroom.html, https://support.captureone.com/hc/en-us/articles/360002484457-Capture-One-Preferences-Settings-Image-tab, https://docs.darktable.org/usermanual/4.8/en/preferences-settings/processing/, https://docs.darktable.org/usermanual/development/es/special-topics/program-invocation/darktable/, https://rawpedia.rawtherapee.com/RawPedia_Book

### Development Tools and Platforms

플랫폼과 도구 관점에서 확인되는 핵심은 **로컬 GPU 활용**, **프로파일링 도구 존재 여부**, **카메라 연결 플랫폼의 제약**, **캐시 관리 도구**다. darktable은 OpenCL 환경 준비 시 커널 로드/컴파일과 모듈 준비가 시작 시점에 일어나며, `-d opencl -d perf`로 pixelpipe와 커널별 시간을 프로파일링할 수 있다. Capture One은 하드웨어 가속에서 Preview Update, Fit Image to Screen, Process time이 서로 다른 자원을 사용한다고 설명하고, ImageCore cache와 OpenCL 설정을 명시한다. Lightroom Classic은 GPU가 Develop/Library 표시와 조정을 가속하며, Camera Raw cache를 크게 두는 것이 preview generation을 빠르게 만들 수 있다고 안내한다.

_IDE and Editors:_ 이 도메인에서는 IDE 선택보다 profiler/logging/traceability가 더 중요하다. 특히 darktable처럼 모듈별, 커널별 시간까지 볼 수 있는 도구 체계가 중요하다. 이 점은 Boothy의 requestId 단위 계측 전략과 잘 맞는다.  
_Version Control:_ 오픈소스 쪽 핵심 엔진들(darktable, RawSpeed)은 GitHub 기반으로 공개 개발되고 있어 벤치마킹, 포크, 서브모듈 통합이 가능하다. darktable는 RawSpeed와 LibRaw를 실제로 submodule로 관리한다.  
_Build Systems:_ darktable는 OpenMP, OpenCL, SSE/AVX를 감지해 빌드하며, RawSpeed는 CMake 기반 빌드를 제공한다. 이는 "플랫폼 변경도 허용"이라는 현재 목표와 잘 맞는다. 즉, Windows 전용 사용자 앱을 유지하더라도 imaging core는 별도 네이티브 빌드 체인으로 분리하는 전략이 현실적이다. 이 판단은 공식 자료 기반 추론이다.  
_Testing Frameworks:_ 공식 제품 문서보다 오픈소스 도구 쪽이 성능 프로파일링 수단을 더 직접 노출한다. 따라서 Boothy처럼 체감 속도가 곧 제품성인 경우, 단순 기능 테스트보다 계측·프로파일링 도구를 기술 스택의 일부로 봐야 한다.  
_Source:_ https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/, https://docs.darktable.org/usermanual/development/es/special-topics/program-invocation/darktable/, https://docs.darktable.org/usermanual/4.8/en/preferences-settings/processing/, https://support.captureone.com/hc/en-us/articles/360002412798-What-does-Hardware-Acceleration-do-and-how-do-I-use-it-in-Capture-One, https://support.captureone.com/hc/en-us/articles/360002484457-Capture-One-Preferences-Settings-Image-tab, https://helpx.adobe.com/lightroom-classic/kb/lightroom-gpu-faq.html, https://helpx.adobe.com/lightroom-classic/kb/optimize-performance-lightroom.html, https://github.com/darktable-org/darktable, https://github.com/darktable-org/rawspeed

### Cloud Infrastructure and Deployment

이번 문제에서 "클라우드 인프라"는 주역이 아니라 **배치 위치 전략**으로 보는 편이 정확하다. 검토한 상용/오픈소스 1차 자료는 모두 `카메라 입력 -> 로컬 캐시/로컬 GPU -> 즉시 프리뷰 표시` 쪽에 무게를 둔다. Lightroom은 로컬 GPU와 Camera Raw cache를, Capture One은 로컬 preview/cache/hardware acceleration을, darktable은 로컬 OpenCL pixelpipe와 캐시를 핵심 성능 수단으로 둔다. Canon은 EDSDK/CCAPI로 Windows, Mac, Linux, Raspberry Pi OS, 모바일까지 연결 폭을 넓힌다.

_Major Cloud Providers:_ 현재 조사 범위의 대표 제품들은 AWS/Azure/GCP 기반 원격 렌더를 tethered first-visible path의 핵심으로 설명하지 않는다. 이 문제의 기준 아키텍처는 클라우드보다 로컬/엣지다.  
_Container Technologies:_ 컨테이너는 배포 재현성과 분리에는 유리하지만, 촬영 직후 필터 적용 결과를 곧바로 보여주는 hot path에 두면 프로세스 경계와 I/O가 늘 수 있다. 실시간 표시 경로보다 후처리나 비동기 파이프라인에 더 어울린다. 이것은 공식 자료들로부터의 추론이다.  
_Serverless Platforms:_ cold start와 업로드/다운로드 왕복이 있는 구조는 이번 목표와 상충할 가능성이 높다. 사용자가 요구한 "바로바로" 기준을 맞추려면 booth 옆 로컬 머신 또는 전용 edge box가 우선순위가 높다. 이것도 자료 종합에 따른 추론이다.  
_CDN and Edge Computing:_ 여기서의 실질적 edge는 CDN이 아니라 **카메라와 같은 현장에 있는 로컬 처리 노드**다. Canon CCAPI가 Wi-Fi 기반 제어를 허용하므로, 장기적으로는 `카메라 + 전용 edge render appliance + 경량 UI client` 구조도 검토 가능한 플랫폼 대안이다.  
_Source:_ https://www.usa.canon.com/support/sdk, https://downloads.canon.com/sdk/CameraControlAPI_OperationGuide_EN.pdf, https://helpx.adobe.com/lightroom-classic/kb/lightroom-gpu-faq.html, https://helpx.adobe.com/lightroom-classic/kb/optimize-performance-lightroom.html, https://support.captureone.com/hc/en-us/articles/360002412798-What-does-Hardware-Acceleration-do-and-how-do-I-use-it-in-Capture-One, https://support.captureone.com/hc/en-us/articles/360002484457-Capture-One-Preferences-Settings-Image-tab, https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/, https://docs.darktable.org/usermanual/4.8/en/preferences-settings/processing/

### Technology Adoption Trends

가장 강한 흐름은 **single-path full render**에서 **multi-stage preview strategy**로 이동하는 것이다. Capture One은 `Immediately`와 `When ready`를 제품 옵션으로 분리해 공개하고 있고, Lightroom Classic은 tethered import 시 preset 적용과 GPU/cache 가속을 제공한다. darktable은 preview/full pixelpipe, thumbnail/full preview cache, OpenCL warm-up 구조를 문서화한다. 즉, 사용자가 "필터가 적용된 같은 촬영 결과가 바로 보여진다"고 느끼게 만드는 제품들은, 내부적으로는 하나의 무거운 경로보다 `즉시성용 경로`와 `품질용 경로`를 전략적으로 나누는 쪽에 더 가깝다. 이 판단은 여러 공식 자료를 함께 읽은 결과다.

동시에, 더 빠른 첫 표시를 가능하게 하는 하부 포맷 대응도 늘고 있다. LibRaw는 최신 버전에서 DNG 1.7과 JPEG-XL preview, Canon H265 thumbnail, Adobe DNG SDK 연동을 강화했다. 이는 "필터 적용 full render를 기다리지 않고도, 같은 촬영의 더 풍부한 intermediate preview를 활용할 수 있는 여지"가 커지고 있음을 뜻한다. 반면 RawSpeed는 여전히 첫 단계 decode에 집중하고 viewable thumbnail은 직접 제공하지 않으므로, fastest first-visible 전략에서 단독 해법이 되기보다는 decode core로 쓰이는 편이 적합하다.

_Migration Patterns:_ 시장/오픈소스 모두 camera SDK + preview cache + GPU accelerate + staged preview 쪽으로 모인다.  
_Emerging Technologies:_ richer embedded preview formats, DNG SDK integration, GPU-first preview generation, precompiled kernel/cache 전략이 중요해지고 있다.  
_Legacy Technology:_ "capture saved 후 full render 완료까지 기다린 다음 첫 이미지를 보여주는" 구조는 체감 기준에서 점점 경쟁력이 떨어진다. 이 결론은 공식 자료들의 공통 패턴으로부터의 추론이다.  
_Community Trends:_ 오픈소스는 조합 가능성과 제어권이 크고, 상용 제품은 이미 tethered UX 정책을 명시적으로 노출한다. Boothy가 빠르게 따라가려면 기존 스택의 미세 튜닝만이 아니라, preview 전략 자체를 제품 수준에서 다시 정의해야 한다.  
_Source:_ https://support.captureone.com/hc/en-us/articles/360002556797-Selecting-the-appropriate-capture-preview, https://support.captureone.com/hc/en-us/articles/360002556798-Adding-adjustments-automatically-while-capturing, https://helpx.adobe.com/lightroom-classic/help/import-photos-tethered-camera.html, https://helpx.adobe.com/lightroom-classic/kb/lightroom-gpu-faq.html, https://helpx.adobe.com/lightroom-classic/kb/optimize-performance-lightroom.html, https://docs.darktable.org/usermanual/4.8/en/preferences-settings/processing/, https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/, https://www.libraw.org/download, https://github.com/darktable-org/rawspeed

## Integration Patterns Analysis

### API Design Patterns

이 주제에서 중요한 API는 전통적인 웹 공개 API가 아니라, **카메라 제어 API**, **로컬 렌더 작업 API**, **UI 반영 IPC API**다. 먼저 Canon CCAPI는 네트워크를 통한 카메라 제어를 위해 HTTP 기반 API를 제공하며, HTTP/HTTPS 포트와 사용자 인증을 설정할 수 있다. HTTP 자체는 RFC 9110이 설명하듯 리소스 종류와 구현에 독립적인 uniform interface를 제공하므로, 카메라를 booth 본체와 분리한 edge box나 별도 제어 프로세스로 옮길 때 유리하다. 다만 첫 필터 적용 결과를 가능한 빨리 보여줘야 하는 hot path에서는 HTTP 요청을 여러 단계로 연쇄 호출하는 설계가 불리할 수 있다. 이 평가는 RFC 9110과 Canon 문서를 바탕으로 한 추론이다.

반대로 로컬 애플리케이션 내부에서는 **RPC/명령 + 이벤트** 구조가 더 적합하다. Tauri 공식 문서는 IPC를 비동기 메시지 전달로 설명하며, `Commands`는 JSON-RPC 유사 직렬화를 통한 요청/응답에, `Events`는 lifecycle/state 변화 전파에 적합하다고 밝힌다. 즉 `capture`, `set-selected-preset`, `start-preview-render`는 command 계층에 두고, `capture-accepted`, `fast-preview-ready`, `preset-preview-ready`, `visible` 같은 상태 전이는 event 계층으로 분리하는 것이 현재 Boothy 문제와 잘 맞는다. 이 판단은 Tauri IPC 공식 문서와 현재 제품 문제를 연결한 해석이다.

카메라 쪽에서는 **capture-and-deliver** 성격의 API가 중요하다. gphoto 공식 원격 제어 문서는 일부 카메라에서 `capturetarget=sdram`으로 카메라 RAM에 직접 촬영하고 같은 호출 안에서 즉시 다운로드할 수 있다고 설명한다. 이는 메모리카드 저장 후 다시 찾는 파일 중심 통합보다, "캡처와 첫 프리뷰 전달을 한 트랜잭션처럼 묶는 API"가 latency 면에서 더 유리하다는 단서를 준다. 다만 gphoto의 remote page는 스스로도 불완전할 수 있다고 적고 있어, 이 부분의 신뢰도는 Canon 공식 문서보다 낮다.

마지막으로 상용 제품은 **preset selection과 capture session state를 같은 API 경계에 둔다.** Lightroom Classic은 tether bar에서 카메라 제어와 develop settings를 함께 조작하고 preset을 import 시점에 적용할 수 있게 하며, Capture One은 `Next Capture Adjustments`와 `Auto Select New Capture`로 촬영 직후 어떤 조정 결과를 보여줄지 제품 차원에서 묶어 둔다. 이것이 시사하는 바는 Boothy도 "촬영 후 나중에 preset을 붙이는 구조"보다 "선택된 preset이 이미 next-capture session contract에 포함된 구조"가 더 낫다는 점이다. 이 결론은 Adobe와 Capture One의 제품 동작에서 끌어낸 추론이다.

_RESTful APIs:_ Canon CCAPI 같은 네트워크 제어 경계에서는 적합하다. 다만 first-visible hot path의 세부 단계를 모두 REST round-trip으로 만들면 불리할 수 있다.  
_GraphQL APIs:_ 이 도메인의 핵심 문제는 여러 데이터를 한 번에 질의하는 것보다, 같은 촬영의 필터 적용 결과를 얼마나 빨리 계산하고 밀어주느냐에 있다. 따라서 현재 우선순위는 낮다. 이 평가는 주제 특성에 따른 추론이다.  
_RPC and gRPC:_ 로컬 또는 edge render worker를 별도 프로세스/노드로 떼어낼 경우 높은 우선순위 후보가 된다. gRPC는 고성능 RPC 프레임워크이며 HTTP/2와 protobuf 기반 서비스 정의를 지원한다. 현재 same-machine Tauri loop에는 바로 필요하지 않지만, 스택 전환 시 유력한 대안이다.  
_Webhook Patterns:_ 외부 webhook보다 내부 event emission이 더 적절하다. Tauri의 event 모델이 이 역할에 가깝다.  
_Source:_ https://cam.start.canon/vi/C017/manual/html/UG-06_Network_0130.html, https://www.rfc-editor.org/rfc/rfc9110, https://v2.tauri.app/concept/inter-process-communication/, https://gphoto.github.io/doc/remote/, https://helpx.adobe.com/lightroom-classic/help/import-photos-tethered-camera.html, https://support.captureone.com/hc/en-us/articles/360002556797-Selecting-the-appropriate-capture-preview, https://support.captureone.com/hc/en-us/articles/360002556798-Adding-adjustments-automatically-while-capturing, https://grpc.io/about/, https://grpc.io/docs/what-is-grpc/introduction/

### Communication Protocols

통신 프로토콜 관점에서 보면, 이 문제는 **USB/PTP 계열의 카메라 입력**, **HTTP/HTTPS 기반 네트워크 제어**, **로컬 비동기 메시지 전달**, 필요 시 **고성능 RPC나 스트리밍**으로 나뉜다. darktable은 tethering에서 gphoto2를 사용하고 USB로 카메라를 직접 제어한다. gphoto 문서는 일부 카메라에서 SDRAM capture와 즉시 다운로드를 지원한다고 설명한다. 즉, 가장 빠른 로컬 첫 결과를 노릴 때는 USB/PTP 또는 벤더 SDK 기반 direct control이 여전히 강하다.

네트워크 분리 구조를 택할 경우 Canon CCAPI는 HTTP/HTTPS와 사용자 인증을 갖춘 공식 경계를 제공한다. Canon 제품 문서는 CCAPI가 HTTP 기반이며 HTTP/HTTPS 포트와 사용자 인증을 설정할 수 있다고 밝힌다. 따라서 카메라가 네트워크 제어 가능한 모델이라면 `camera -> CCAPI -> local/edge renderer -> UI` 구조가 가능하다. 다만 이 경우도 필터 적용 결과의 첫 표시까지 여러 요청/응답 hop이 늘어나지 않도록 주의해야 한다. 이 평가는 Canon 문서와 현 과제의 latency 요구를 연결한 추론이다.

앱 내부 프로토콜로는 Tauri의 비동기 메시지 전달이 더 적합하다. 공식 문서상 `Events`는 fire-and-forget one-way 메시지이고, `Commands`는 JSON-RPC 유사 직렬화 기반 요청/응답이다. 현재 Boothy의 핵심 병목은 "상태를 언제 알려주느냐"이기도 하므로, 파일 polling보다 `event-first` 전파가 제품 목표와 맞는다. 이 결론은 Tauri IPC 공식 문서와 현재 제품 문제에서 도출한 추론이다.

추가로 프로세스나 노드 분리가 커질 경우, gRPC와 WebSocket은 각각 다른 용도로 의미가 생긴다. gRPC는 HTTP/2 기반의 고성능 RPC와 protobuf 서비스 정의를 제공해 render worker 분리에 유리하고, WebSocket은 장시간 유지 연결과 양방향 메시지 전달에 강하므로 progressive preview stream이나 원격 모니터링 UI에 적합하다. 하지만 현재 single-booth same-machine hot path에선 이 둘보다 direct tether + local IPC 최적화가 우선이다.

_HTTP/HTTPS Protocols:_ Canon CCAPI와 네트워크 edge 제어에 적합하다. HTTP는 uniform interface를 제공하고, Canon은 HTTP/HTTPS와 인증 설정을 노출한다.  
_WebSocket Protocols:_ 원격 live preview, progressive preview stream, telemetry dashboard가 필요한 경우에만 의미가 커진다. 현재 핵심 first-visible path에서는 필수는 아니다.  
_Message Queue Protocols:_ queue는 overload 완화에는 유효하지만 minimal latency 응답이 필요한 hot path에는 부적합할 수 있다. 따라서 critical path보다는 재시도, 후처리, export에 두는 편이 낫다.  
_gRPC and Protocol Buffers:_ render engine을 sidecar에서 더 나아가 독립 worker/edge service로 분리하면 강력한 후보가 된다.  
_Source:_ https://docs.darktable.org/usermanual/4.0/en/tethering/overview/, https://gphoto.github.io/doc/remote/, https://cam.start.canon/vi/C017/manual/html/UG-06_Network_0130.html, https://www.rfc-editor.org/rfc/rfc9110, https://v2.tauri.app/concept/inter-process-communication/, https://grpc.io/about/, https://grpc.io/docs/what-is-grpc/introduction/, https://www.rfc-editor.org/rfc/rfc6455.html, https://learn.microsoft.com/sl-si/azure/architecture/patterns/queue-based-load-leveling

### Data Formats and Standards

이 문제에서 중요한 데이터 포맷은 단순 JSON이 아니라 **RAW 본문**, **embedded preview/thumbnail**, **XMP/metadata**, **intermediate preview formats**다. LibRaw는 RAW 데이터, 처리에 필요한 메타데이터, 그리고 embedded preview/thumbnail 추출을 위한 통합 인터페이스를 제공한다고 밝힌다. 동시에 LibRaw 스스로 production-quality rendering은 기능 범위가 아니라고 설명한다. 이것은 제품적으로 매우 중요하다. 즉 LibRaw는 "최종 필터 적용 이미지 엔진"이라기보다, **같은 촬영의 가장 빠른 early visual source를 꺼내는 계층**으로 보는 편이 더 정확하다.

또한 최신 LibRaw 0.22는 DNG 1.7 JPEG-XL 지원을 Adobe DNG SDK 1.7.x 통합으로 제공하고, Canon H265 thumbnail과 JPEG-XL thumbnail도 지원한다. 더 중요한 점은 JPEG/JPEG-XL compressed DNG 이미지에 대해 DNG OpcodeList2/3를 Adobe DNG SDK로 처리해 corrected image를 만들 수 있다는 것이다. 이건 "완전한 최종 RAW render를 기다리지 않아도, 더 풍부하게 보정된 same-capture intermediate preview를 만들 수 있는 여지"를 의미한다. 이 해석은 LibRaw 릴리즈 노트로부터의 추론이다.

로컬 IPC 포맷으로는 JSON이 기본이다. Tauri command는 JSON 직렬화 가능한 인자와 반환값을 요구한다. 다만 프로세스 분리가 커져 preview-ready 이벤트, requestId timeline, preset metadata, histogram/thumbnail metadata 같은 구조화 메시지 양이 많아지면, protobuf가 더 적합해질 수 있다. protobuf 공식 문서는 이를 language-neutral, platform-neutral한 구조화 직렬화 메커니즘으로 설명하며 XML보다 더 작고 빠르고 단순하다고 안내한다. 따라서 `same-machine UI loop`에는 JSON으로 충분하지만, `edge renderer split`까지 가면 protobuf 채택 가치가 생긴다. 이 결론은 protobuf 공식 문서와 현재 아키텍처 목표를 연결한 추론이다.

상호운용 포맷으로는 XMP/metadata와 watched-folder 입력도 중요하다. Lightroom Auto Import는 watched folder를 감시하면서 Develop settings, metadata, keywords를 적용할 수 있고, Standard preview를 렌더해 embedded preview만 쓰는 것을 넘길 수 있다. 이 말은 곧, 현재 direct tether path가 한계에 부딪히면 `camera helper -> watched folder -> dedicated render engine -> preview return` 같은 파일 기반 상호운용도 현실적인 fallback이 된다는 뜻이다. 다만 이 구조는 direct SDK/API path보다 파일 시스템 경계를 더 가지므로, 기본 해법이 아니라 fallback/전환용 경로로 보는 편이 맞다. 이 평가는 Adobe 문서와 latency 요구를 합친 추론이다.

_JSON and XML:_ 현재 로컬 IPC는 JSON 계열이 자연스럽다. XML은 이번 hot path의 우선 포맷이 아니다.  
_Protobuf and MessagePack:_ worker/edge 분리 시 protobuf가 더 유력하다. 언어/플랫폼 중립성과 효율성이 강점이다.  
_CSV and Flat Files:_ 본 문제의 critical path에는 적합하지 않다.  
_Custom Data Formats:_ RAW embedded preview, DNG JPEG-XL preview, Canon H265 thumbnail, XMP/metadata가 핵심이다.  
_Source:_ https://www.libraw.org/about, https://www.libraw.org/download, https://v2.tauri.app/concept/inter-process-communication/, https://protobuf.dev/, https://helpx.adobe.com/lightroom-classic/help/import-photos-automatically.html

### System Interoperability Approaches

시스템 상호운용 관점에서 가장 현실적인 1차 패턴은 네 가지다. 첫째, **point-to-point direct integration**이다. darktable 문서는 USB로 연결한 카메라를 darktable이 직접 제어하고, 다른 앱이 쓰지 못하도록 다시 lock한다고 설명한다. 이는 booth 본체가 카메라를 직접 소유하는 구조가 여전히 latency와 일관성 면에서 강하다는 뜻이다.

둘째, **sidecar worker interoperability**다. Tauri 공식 문서는 sidecar binary를 외부 바이너리로 포함하고, Rust에서 `shell().sidecar()`로 spawn한 뒤 stdout 이벤트를 읽어 앱 이벤트로 다시 emit하는 방식을 보여 준다. 이 패턴은 현재 Boothy가 이미 가진 `앱 셸`과 `네이티브 처리기`의 분리를 더 명확하게 가져갈 수 있게 한다. 제품적으로는 preset-applied preview 엔진을 darktable-cli, custom GPU worker, 혹은 다른 네이티브 렌더러로 바꾸더라도 프런트엔드 셸을 유지하기 쉬워진다.

셋째, **watch-folder interoperability**다. Lightroom Auto Import는 watched folder 기반 자동 가져오기와 develop settings 적용을 지원하고, tethered import가 안 되는 카메라에서도 카메라 소프트웨어가 watched folder로 다운로드하면 계속 쓸 수 있다고 설명한다. 이 패턴은 "카메라 SDK와 렌더 엔진을 한 앱에 다 넣지 않고, 파일로 경계를 나누는 호환 구조"를 의미한다. 현재 기술로 direct path가 충분히 빠르지 않다면, 상용 엔진과의 결합을 위해 가장 현실적인 브리지 패턴이 될 수 있다.

넷째, **gateway/intermediary pattern**은 지금 당장보다는 장기 옵션이다. Microsoft의 gateway aggregation 문서는 다수 백엔드 서비스 호출의 chattiness를 줄이고 cross-cutting concern을 한 곳에 모을 수 있다고 설명한다. 만약 Boothy가 `camera-control service`, `preview-render service`, `telemetry service`, `print service`로 쪼개진다면 의미가 있다. 하지만 현재 단일 booth 장비에서 first-visible latency를 줄이는 관점에서는 gateway가 한 hop 더 늘릴 위험이 있어 기본 선택은 아니다. 이 평가는 Microsoft 문서와 현재 제품 목표를 연결한 추론이다.

_Point-to-Point Integration:_ direct USB/SDK 제어가 여전히 가장 빠른 기준선이다.  
_API Gateway Patterns:_ 다중 서비스 분리 후에는 유효하지만, 현재 single-machine hot path에는 과할 가능성이 높다.  
_Service Mesh:_ 현 시점 booth 단일 장비 문제에는 과도하다.  
_Enterprise Service Bus:_ latency와 운영 복잡도 측면에서 우선순위가 낮다.  
_Source:_ https://docs.darktable.org/usermanual/4.0/en/tethering/overview/, https://v2.tauri.app/es/develop/sidecar/, https://helpx.adobe.com/lightroom-classic/help/import-photos-automatically.html, https://learn.microsoft.com/en-us/azure/architecture/patterns/gateway-aggregation, https://learn.microsoft.com/en-us/azure/architecture/microservices/design/gateway

### Microservices Integration Patterns

이번 문제에서 마이크로서비스 패턴은 "지금 당장 도입해야 할 정답"이라기보다, **기술/플랫폼을 바꿔도 된다**는 조건에서 어떤 분리까지 가치가 있는지를 가늠하는 기준이다. 가장 먼저 의미가 생기는 것은 **local sidecar -> dedicated render service** 전환이다. 여기서 gateway aggregation은 여러 백엔드 호출을 하나로 묶어 chattiness를 줄이는 데 도움이 될 수 있다. 다만 이것은 `camera service`, `render service`, `session state service`처럼 분리가 실제로 일어났을 때의 이야기다. 현재 same-machine path에서는 sidecar 단이 더 현실적이다. 이 평가는 Microsoft gateway 문서를 기반으로 한 추론이다.

**Circuit Breaker**는 더 직접적으로 유용하다. Microsoft의 circuit breaker 패턴은 실패 가능성이 높은 연산을 반복 호출하지 않게 해 faulting dependency 과부하를 막고, graceful degradation을 돕는다고 설명한다. booth 관점에서는 원격 카메라 제어, 원격 render appliance, 혹은 불안정한 외부 엔진 호출이 생길 경우 유효하다. 예를 들어 preset-applied preview 엔진이 일시적으로 느려지거나 실패할 때, UI가 무한 재시도로 막히지 않고 fast same-capture fallback이나 직전 안정 상태로 degrade하는 데 쓸 수 있다.

반대로 **Saga**는 이 문제에 기본적으로 잘 맞지 않는다. AWS는 saga orchestration을 여러 서비스와 데이터 저장소에 걸친 분산 트랜잭션 일관성을 위한 패턴으로 설명하며, 복잡성, eventual consistency, 보상 트랜잭션, 추가 latency를 함께 언급한다. 이것은 주문/결제/재고처럼 여러 시스템의 무결성이 중요한 경우에는 맞지만, booth의 핵심 과제인 `촬영 직후 필터 적용 결과를 즉시 보여주는 것`과는 결이 다르다. 따라서 preview hot path를 saga로 풀려는 방향은 피하는 것이 좋다.

Queue-based load leveling도 비슷하다. Microsoft 문서는 queue를 버퍼로 두어 overload를 완화할 수 있지만, minimal latency 응답이 필요한 애플리케이션에는 적합하지 않다고 명시한다. 그러므로 export, 업로드, 기록 보존, background pre-render에는 유용할 수 있어도, 사용자 첫 체감 화면을 만드는 critical path에 직접 넣는 것은 부적절하다.

_API Gateway Pattern:_ 서비스 분리 이후에는 의미가 있다. 현재는 sidecar 수준이 더 적합하다.  
_Service Discovery:_ 고정 booth 장비에는 정적 연결이 더 단순하다. discovery는 다중 카메라/다중 edge node 환경에서나 중요해진다. 이 평가는 주제 특성에 따른 추론이다.  
_Circuit Breaker Pattern:_ 원격 의존성 또는 불안정한 렌더 worker 보호에 유효하다.  
_Saga Pattern:_ 미리보기 hot path에는 부적합하다. 분산 트랜잭션 일관성 문제를 풀 때만 고려할 가치가 있다.  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/gateway-aggregation, https://learn.microsoft.com/en-us/azure/architecture/patterns/circuit-breaker, https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/saga-orchestration.html, https://learn.microsoft.com/sl-si/azure/architecture/patterns/queue-based-load-leveling, https://v2.tauri.app/es/develop/sidecar/

### Event-Driven Integration

이 문제에서 가장 중요한 event-driven 패턴은 복잡한 메시지 브로커가 아니라, **capture lifecycle을 명시적인 이벤트로 쪼개는 것**이다. Tauri는 이벤트를 fire-and-forget one-way IPC로 정의하고 lifecycle/state changes에 적합하다고 설명한다. 따라서 `button-pressed`, `capture-accepted`, `embedded-preview-extracted`, `preset-preview-render-started`, `preset-preview-ready`, `visible` 같은 이벤트 체인을 먼저 제품의 공식 상태 모델로 만드는 것이 중요하다. 이는 현재 Boothy 브리프가 요구한 requestId 계측 방향과도 정확히 맞물린다.

이벤트를 저장하고 재생하는 full event-sourcing까지 갈 필요는 없다. 이 문제의 핵심은 비즈니스 상태 재구성이 아니라 first-visible latency 단축이기 때문이다. 하지만 event log 자체는 강한 가치가 있다. 즉, event sourcing은 아니더라도, requestId 기반 상태 이벤트를 정확히 남기면 병목 판별과 회귀 감시에 직접 도움이 된다. 이는 Tauri event 모델과 현재 제품 목표를 연결한 추론이다.

메시지 브로커/큐는 2차 경로에 두는 편이 낫다. queue-based load leveling 문서는 queue가 과부하 완화에는 좋지만 minimal latency 응답에는 부적합하다고 명시한다. 따라서 `첫 필터 적용 결과` 경로에는 queue를 두지 말고, `추가 품질 렌더`, `업로드`, `재처리`, `장애 재시도`에 두는 것이 더 적절하다. 장기적으로 원격 edge 장비와 preview stream을 연결해야 한다면 MQTT, WebSocket, gRPC streaming 같은 후보가 생길 수 있지만, 지금 가장 큰 제품 가치는 내부 이벤트 체계 정립에 있다.

_Publish-Subscribe Patterns:_ 내부 상태 전파에 적합하다.  
_Event Sourcing:_ hot path 기본 구조로는 과하다. 계측 로그 수준이면 충분하다.  
_Message Broker Patterns:_ overload 격리에는 유효하지만 first-visible critical path에는 부적합하다.  
_CQRS Patterns:_ command(캡처/렌더 시작)와 read model(현재 보여줄 프리뷰 상태)을 분리하는 가벼운 CQRS는 유효하다. 이 평가는 Tauri command/event 구조와 현재 문제를 연결한 추론이다.  
_Source:_ https://v2.tauri.app/concept/inter-process-communication/, https://learn.microsoft.com/sl-si/azure/architecture/patterns/queue-based-load-leveling, https://www.rfc-editor.org/rfc/rfc6455.html, https://grpc.io/about/

### Integration Security Patterns

보안 관점에서도 이 문제는 일반 SaaS API와 조금 다르다. 우선 네트워크 카메라 제어를 택할 경우 Canon CCAPI는 HTTP/HTTPS 포트 설정과 사용자 인증을 제공한다. 따라서 booth 내부 LAN만 쓴다고 해도, 카메라 제어를 별도 edge 노드로 분리하는 순간부터는 HTTPS와 인증을 기본 전제로 두는 편이 맞다. 이 평가는 Canon 공식 문서에 근거한다.

로컬 앱 내부에서는 **최소 권한 IPC**가 핵심이다. Tauri 권한 문서는 permissions가 명시적 privilege 설명이며, command 허용/거부와 scope 매핑을 정의한다고 설명한다. 또한 event 기본 권한과 개별 emit/listen 권한이 별도로 존재한다. 즉, preview 관련 event, file read, sidecar 실행, shell access를 전부 묶어 넓게 열기보다, 실제 필요한 명령과 이벤트만 capability에 연결하는 편이 안전하다.

sidecar를 쓸 경우에도 마찬가지다. Tauri sidecar 문서는 `shell:allow-execute` 또는 `shell:allow-spawn` 권한을 명시적으로 부여해야 sidecar child process를 실행할 수 있다고 설명한다. 이는 렌더 엔진을 외부 바이너리로 분리하더라도, 그 실행 권한과 인자 허용 범위를 제품이 통제할 수 있음을 뜻한다. 제품적으로는 "어떤 preset engine을 허용할 것인가", "어떤 경로와 인자를 렌더 worker에 넘길 것인가"를 security policy로 다룰 수 있다는 뜻이기도 하다.

마지막으로, API gateway를 도입하는 단계로 가면 Microsoft 문서가 설명하듯 SSL termination, mutual TLS, authentication, rate limiting 같은 cross-cutting concern을 중앙화할 수 있다. 그러나 현재 single-machine booth hot path에서는 gateway 보안 계층이 기본값이 아니라 후속 확장용 옵션이다.

_OAuth 2.0 and JWT:_ 현재 booth local loop의 1차 보안 수단은 아니다. 네트워크 서비스 분리 시에만 고려 가치가 커진다. 이 평가는 주제 특성에 따른 추론이다.  
_API Key Management:_ Canon CCAPI는 API key보다 사용자 인증 모델에 가깝다.  
_Mutual TLS:_ 다중 서비스/노드 분리 시 gateway 계층에서 의미가 생긴다.  
_Data Encryption:_ 네트워크 제어 경계에서는 HTTPS가 기본 선택지다. 로컬 IPC는 capability와 권한 범위 통제가 더 중요하다.  
_Source:_ https://cam.start.canon/vi/C017/manual/html/UG-06_Network_0130.html, https://v2.tauri.app/security/permissions/, https://v2.tauri.app/reference/acl/core-permissions/, https://v2.tauri.app/es/develop/sidecar/, https://learn.microsoft.com/en-us/azure/architecture/microservices/design/gateway

## Architectural Patterns and Design

### System Architecture Patterns

이번 문제에 맞는 시스템 아키텍처 패턴은 범용적인 "좋은 구조"가 아니라, **같은 촬영의 preset-applied first visible result를 가장 짧은 경로로 전달하는 구조**여야 한다. 공식 자료를 종합하면 가장 직접적인 기준선은 `direct camera control + local processing + immediate preview policy`다. darktable는 USB로 연결된 카메라를 직접 제어하고 gphoto2를 통해 darktable가 카메라를 점유한다. Capture One은 tethered capture에서 `Immediately`와 `When ready`를 분리해, 빠르게 작업할 때는 "조정이 적용되는 동안 quickly rendered preview"를 먼저 보여주는 정책을 공식적으로 제공한다. 이 둘은 공통적으로, "복잡한 분산 아키텍처"보다 "로컬 입력과 빠른 첫 표시를 최대한 가깝게 붙이는 구조"를 우선한다는 점을 보여준다.

이 관점에서 Boothy의 1차 후보는 **로컬 event-driven monolith + dedicated render sidecar**다. UI 셸, 세션 상태, 카메라 제어는 현재처럼 하나의 제품 안에 두되, preset-applied preview 생성을 별도 네이티브 워커로 분리하는 구조다. Tauri는 외부 바이너리를 `app.shell().sidecar()`로 실행하고 stdout 이벤트를 프런트 이벤트로 다시 emit하는 패턴을 공식적으로 지원한다. Cockburn의 Hexagonal Architecture는 애플리케이션 코어가 UI와 데이터베이스 없이도 동작하고, 외부 장치와는 port와 adapter를 통해 대화해야 한다고 설명한다. 이 조합은 Boothy에 잘 맞는다. 즉, `camera adapter`, `render adapter`, `ui adapter`를 분리하되, 사용자가 체감하는 핵심 플로우는 한 로컬 장비 안에서 유지할 수 있다.

반대로 **microservices-first**는 이 문제의 기본 해법으로 적합하지 않다. Microsoft는 마이크로서비스가 독립 배포와 빠른 진화에 유리하지만, 작은 서비스가 많아질수록 interservice communication이 늘고 긴 호출 사슬이 추가 latency 문제를 만들 수 있다고 설명한다. 현재 과제는 조직 독립 배포보다 "촬영 후 바로 보이는 시간"이 절대 기준이므로, preview hot path를 서비스 체인으로 쪼개는 것은 손해가 될 가능성이 높다. 따라서 아키텍처 수준의 1차 결론은 `microservices보다 local bounded system`이다.

장기 2차 후보는 **edge render appliance architecture**다. AWS는 latency-sensitive 기능은 클라우드보다 on-premises가 더 적합할 수 있다고 설명하고, low latency를 위해 edge 또는 on-prem component를 둘 수 있다고 안내한다. Canon CCAPI가 네트워크 제어를 허용하므로, 카메라 제어와 렌더를 booth 옆 edge box로 옮기고 UI는 얇게 유지하는 구조도 기술적으로 가능하다. 다만 이 구조는 현재보다 운영 복잡도가 커지므로, 현 시점에서는 "현재 스택의 한계가 확인될 경우의 상위 옵션"으로 두는 편이 적절하다. 이 판단은 AWS edge 문서와 Canon CCAPI를 종합한 추론이다.

_Source:_ https://docs.darktable.org/usermanual/4.0/en/tethering/overview/, https://support.captureone.com/hc/en-us/articles/360002556797-Selecting-the-appropriate-capture-preview, https://v2.tauri.app/es/develop/sidecar/, https://alistair.cockburn.us/hexagonal-architecture, https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/microservices, https://docs.aws.amazon.com/prescriptive-guidance/latest/mes-on-aws/edge.html, https://cam.start.canon/vi/C017/manual/html/UG-06_Network_0130.html

### Design Principles and Best Practices

설계 원칙 측면에서 가장 중요한 것은 **inside/outside 분리**와 **hot path 우선 설계**다. Hexagonal Architecture 원문은 애플리케이션을 UI나 데이터베이스 없이도 동작하게 만들고, 바깥 세계와는 port와 adapter를 통해 연결해야 한다고 설명한다. 이 원칙을 Boothy에 적용하면, `preset-applied preview decision`과 `requestId timeline` 같은 제품 핵심 규칙은 코어에 두고, Canon SDK/gphoto2/Lightroom watch folder/darktable worker 같은 것은 모두 교체 가능한 adapter로 보는 편이 맞다.

두 번째 원칙은 **명시적 preview policy**다. Capture One의 공식 제품 정책은 tethered capture에서 `Immediately`와 `When ready`를 분리하고, 촬영 성격에 따라 사용자가 선택하도록 한다. 즉, "어떤 품질/지연 정책을 첫 표시 기준으로 채택할 것인가"는 단순 구현 디테일이 아니라 아키텍처 결정이다. Boothy도 `same-capture + selected-preset + first-visible SLA`를 제품 정책으로 승격해야 한다. 이 결론은 Capture One 문서에서 도출한 추론이다.

세 번째 원칙은 **preview generation을 데이터 접근과 함께 설계**하는 것이다. Adobe는 Camera Raw cache에 원본 이미지 데이터가 있으면 초기 처리 단계를 건너뛰어 preview generation을 더 빠르게 할 수 있다고 설명한다. 또한 Standard preview를 미리 만들어 두지 않으면 작업 중 자동 생성이 성능을 방해할 수 있다고 밝힌다. 이는 Boothy에서도 `렌더 엔진`, `캐시`, `입력 포맷`, `프리셋 선택 상태`를 따로 최적화하지 말고 하나의 설계 대상으로 봐야 함을 뜻한다.

네 번째 원칙은 **점진적 대체 가능성**이다. Tauri sidecar 구조는 현재 UI 셸을 유지한 채 외부 바이너리를 교체할 수 있게 해 주고, Hexagonal 구조는 adapter 단위 교체를 정당화한다. 따라서 현재 darktable 경로를 유지하더라도, 나중에 custom GPU renderer나 상용 엔진 브리지로 갈아타기 쉬운 구조를 지금부터 택하는 것이 바람직하다. 이것은 Tauri와 Cockburn 문서에 근거한 추론이다.

_Source:_ https://alistair.cockburn.us/hexagonal-architecture, https://support.captureone.com/hc/en-us/articles/360002556797-Selecting-the-appropriate-capture-preview, https://helpx.adobe.com/lightroom-classic/kb/optimize-performance-lightroom.html, https://helpx.adobe.com/lightroom-classic/help/import-photos-automatically.html, https://v2.tauri.app/es/develop/sidecar/

### Scalability and Performance Patterns

이번 과제의 성능 패턴은 일반적인 웹 scale-out보다 **latency-first compute placement**에 가깝다. AWS는 리소스 배치 위치가 네트워크 latency와 throughput에 직접 영향을 주며, data-heavy application은 코드가 데이터 가까이에서 실행되어야 한다고 설명한다. 촬영 직후 RAW/preview 데이터가 들어오는 상황에서는 이 원칙이 매우 직접적이다. 즉, preset-applied preview를 만드는 코드가 카메라/저장소/GPU와 멀어질수록 불리하다.

GPU 관점에서는 **CPU 중심 스케줄링을 줄이고 GPU resident workflow를 늘리는 패턴**이 중요하다. NVIDIA CUDA Graph 문서는 그래프 정의와 실행을 분리하면 CPU launch cost를 줄이고, 반복되는 workflow를 매우 낮은 오버헤드로 다시 실행할 수 있다고 설명한다. Holoscan의 GPU-resident graphs는 이 아이디어를 더 밀어붙여, compute pipeline을 애플리케이션 수명 동안 GPU에 유지하면서 CPU 스케줄링과 동기화 오버헤드를 줄여 deterministic low-latency execution을 제공한다고 밝힌다. 물론 Holoscan은 의료/센서 도메인이지만, `카메라 입력 -> 반복되는 이미지 처리 -> 즉시 표시`라는 구조적 유사성은 크다. 따라서 장기적으로는 preset preview 파이프라인을 `GPU warm and resident` 상태로 유지하는 아키텍처가 강력한 후보다. 이 적용 가능성 판단은 NVIDIA 문서로부터의 추론이다.

캐시 패턴도 중요하다. Adobe는 Camera Raw cache가 원본 이미지 데이터 캐시를 제공해 초기 처리 단계를 건너뛰게 해 준다고 설명하고, Capture One은 Preview Update, Fit Image to Screen, Process time이 서로 다른 자원에 의존한다고 밝힌다. 이는 `한 번 렌더하고 끝`이 아니라, `선택된 preset 조합에 대한 warm cache와 precompiled pipeline을 유지`하는 패턴이 중요하다는 뜻이다.

마지막으로, 이 문제는 horizontal scaling보다 **workload isolation**이 더 중요하다. preview hot path는 큐나 장기 작업과 분리하고, export/upload/full-quality render는 백그라운드로 보내야 한다. Azure는 queue-based load leveling이 overload 완화에는 좋지만 minimal latency 응답에는 적합하지 않다고 설명한다. 따라서 first-visible preset path는 별도 low-latency lane으로 고정하는 것이 맞다.

_Source:_ https://docs.aws.amazon.com/wellarchitected/latest/framework/perf_networking_choose_workload_location_network_requirements.html, https://docs.nvidia.com/cuda/archive/13.1.1/cuda-programming-guide/04-special-topics/cuda-graphs.html, https://docs.nvidia.com/holoscan/sdk-user-guide/gpu_resident.html, https://helpx.adobe.com/lightroom-classic/kb/optimize-performance-lightroom.html, https://support.captureone.com/hc/en-us/articles/360002412798-What-does-Hardware-Acceleration-do-and-how-do-I-use-it-in-Capture-One, https://learn.microsoft.com/sl-si/azure/architecture/patterns/queue-based-load-leveling

### Integration and Communication Patterns

아키텍처 수준의 통합 원칙은 **synchronous control + asynchronous state propagation**이다. Azure의 event-driven architecture는 producer, consumer, channel로 구성되며 near real time 전달과 producer/consumer decoupling에 강하다고 설명한다. 동시에 단순 request-response만으로 충분한 워크로드라면 event broker의 운영 복잡도가 과할 수 있다고도 말한다. 이를 Boothy에 적용하면, `capture`나 `set selected preset` 같은 제어는 동기 명령으로, `capture-accepted`, `preset-preview-ready`, `visible` 같은 상태는 내부 이벤트로 전달하는 혼합형이 가장 적합하다.

또한 CQRS는 읽기와 쓰기 모델을 분리해 각 모델을 독립 최적화할 수 있다고 설명된다. Boothy에서는 `command side`가 촬영, 프리셋 선택, 렌더 시작이고, `read side`가 "지금 사용자에게 무엇을 보여줄 것인가"다. 이 둘의 성격이 다르므로, read model을 `현재 보여줄 preview 상태`에 최적화하고 write model과 느슨하게 결합하는 것이 자연스럽다. 이는 Azure CQRS 문서에 기반한 해석이다.

그러나 이 문제는 full event broker나 대규모 pub/sub가 아니라, **프로세스 내부 또는 로컬 노드 간 짧은 이벤트 체인**이 핵심이다. Tauri IPC는 Commands와 Events를 명확히 분리하며, sidecar 패턴은 외부 worker의 stdout을 다시 앱 이벤트로 전달할 수 있게 한다. 따라서 `UI shell <-> core <-> render worker` 사이를 짧은 이벤트 체인으로 묶는 구조가 가장 실용적이다.

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/event-driven, https://learn.microsoft.com/en-us/azure/architecture/patterns/cqrs, https://v2.tauri.app/concept/inter-process-communication/, https://v2.tauri.app/es/develop/sidecar/

### Security Architecture Patterns

보안 구조는 속도 목표를 방해하지 않으면서도 **실행 권한과 제어 경계를 최소화**하는 방향이 적절하다. Tauri는 permissions를 명시적 privilege 설명으로 두고, capability에서 어떤 명령과 scope를 허용할지 연결하도록 설계한다. core event 권한도 `allow-listen`, `allow-emit` 등으로 세분화되어 있다. 이는 preview hot path의 이벤트와 sidecar 실행 권한을 넓게 열지 않고 최소 범위로 제한할 수 있음을 뜻한다.

네트워크 카메라 제어 또는 edge appliance 구조로 갈 경우에는 Canon CCAPI의 HTTP/HTTPS 포트와 사용자 인증을 활용하는 방식이 기본선이다. 외부 서비스 분리 수준이 커질수록 circuit breaker 같은 회복력 패턴과 보안 경계를 함께 가져가야 한다. Azure의 circuit breaker 문서는 실패 가능성이 높은 remote operation을 반복 호출하지 않게 하고, 기본값 반환이나 대체 경로로 degrade할 수 있다고 설명한다. Boothy에서는 원격 렌더 워커나 외부 상용 엔진 연결이 실패할 때 `capture flow 전체`가 막히지 않게 하는 방어선으로 해석할 수 있다.

_Source:_ https://v2.tauri.app/security/permissions/, https://v2.tauri.app/reference/acl/core-permissions/, https://v2.tauri.app/es/develop/sidecar/, https://cam.start.canon/vi/C017/manual/html/UG-06_Network_0130.html, https://learn.microsoft.com/en-us/azure/architecture/patterns/circuit-breaker

### Data Architecture Patterns

데이터 아키텍처의 핵심 단위는 전통적 업무 DB가 아니라 **capture artifact, preset state, preview cache, render metadata**다. Adobe는 raw 편집 시 up-to-date preview를 생성하며, 원본 이미지 데이터가 Camera Raw cache에 있으면 early-stage processing을 건너뛸 수 있다고 설명한다. 또한 Standard preview는 Filmstrip/Grid thumbnail과 각종 preview 영역에서 사용되며, 미리 준비되지 않으면 작업 중 자동 생성이 성능을 방해할 수 있다고 밝힌다. 이것은 Boothy에서도 `촬영 결과물`, `선택된 preset`, `현재 표시 중인 preview`, `다음 고품질 결과`를 한 데이터 모델로 다루기보다, 서로 다른 latency 요구를 가진 artifact로 분리해야 함을 뜻한다.

이 관점에서 가장 유력한 데이터 패턴은 **dual artifact model**이다. 하나는 `first-visible preset-applied artifact`, 다른 하나는 `final-quality artifact`다. Capture One의 `Immediately` / `When ready` 분리는 사실상 이 패턴의 제품 정책 표현이다. LibRaw의 embedded preview/thumbnail, Adobe의 Standard preview, Camera Raw cache는 모두 "최종 결과 이전의 빠른 시각 산출물"을 정당화한다. 따라서 Boothy도 같은 촬영에 대해 단일 preview path만 유지하기보다, 두 artifact를 requestId로 묶고 각각의 SLA를 별도로 가져가는 쪽이 맞다. 이 결론은 여러 공식 자료를 종합한 추론이다.

_Source:_ https://helpx.adobe.com/lightroom-classic/kb/optimize-performance-lightroom.html, https://helpx.adobe.com/lightroom-classic/help/import-photos-automatically.html, https://support.captureone.com/hc/en-us/articles/360002556797-Selecting-the-appropriate-capture-preview, https://www.libraw.org/about, https://www.libraw.org/download

### Deployment and Operations Architecture

배포/운영 관점에서 가장 적합한 기본 구조는 **single booth local deployment**다. AWS는 low latency가 중요한 기능은 on-premises component가 더 적합할 수 있고, edge나 local placement를 통해 사용자 경험을 개선할 수 있다고 설명한다. 즉, 현재 목표에는 클라우드 우선 구조보다 booth 옆 로컬 GPU/스토리지/카메라 제어가 더 적절하다.

운영 구조는 세 단계 옵션으로 나뉜다. 1단계는 `현재 Boothy + local render sidecar`다. 이는 가장 낮은 리스크로 첫 표시 latency를 줄일 수 있다. 2단계는 `local dedicated render service`다. 이 경우 앱 셸과 렌더 워커를 분리하고, 필요하면 별도 프로세스/VM/GPU 머신으로 확장할 수 있다. 3단계는 `edge appliance + thin client UI`다. Canon CCAPI나 watched folder 브리지를 이용해 카메라 입력을 edge box로 보내고, 거기서 preset-applied preview를 만든 뒤 UI는 결과만 받는다. 이 단계는 기존 앱 기술 한계가 분명할 때만 여는 것이 바람직하다.

운영 계측 역시 아키텍처 일부로 봐야 한다. Holoscan sensor bridge 문서는 프레임 시작, 수신, 오퍼레이터 실행, 파이프라인 완료까지 시간축을 분해해 under-20ms latency를 설명한다. 도메인은 다르지만 메시지는 분명하다. 즉, latency product는 기능 로그가 아니라 **stage-by-stage timing architecture**를 가져야 한다. Boothy의 requestId 계측 계획은 이 방향과 일치한다.

_Source:_ https://docs.aws.amazon.com/prescriptive-guidance/latest/mes-on-aws/edge.html, https://docs.aws.amazon.com/wellarchitected/latest/framework/perf_networking_choose_workload_location_network_requirements.html, https://cam.start.canon/vi/C017/manual/html/UG-06_Network_0130.html, https://helpx.adobe.com/lightroom-classic/help/import-photos-automatically.html, https://docs.nvidia.com/holoscan/sensor-bridge/latest/latency.html

## Implementation Approaches and Technology Adoption

### Technology Adoption Strategies

구현 전략의 첫 원칙은 **빅뱅 교체보다 점진적 치환**이다. Azure Architecture Center의 Strangler Fig 패턴은 큰 시스템이나 복잡한 기능을 한 번에 교체하기보다, 점진적으로 새 기능을 도입하면서 기존 시스템을 유지하는 방식이 위험을 줄인다고 설명한다. 이 패턴은 현재 Boothy 맥락에 매우 잘 맞는다. 즉, 기존 `capture -> preview` 경로를 당장 폐기하지 말고, `selected-preset first-visible path`만 새 경로로 먼저 도입한 뒤 requestId 기준으로 품질과 속도를 비교해 전환하는 방식이 가장 현실적이다.

구체적으로는 세 단계가 적절하다. 1단계는 **현재 셸 유지 + 저지연 preset preview sidecar 추가**다. 2단계는 direct path가 부족할 때 **로컬 dedicated renderer** 또는 **watch-folder bridge**를 추가하는 것이다. 3단계는 현재 스택 한계가 명확히 드러날 때만 **edge render appliance**나 외부 엔진 중심 구조를 검토한다. 이 순서는 기술 리스크, 제품 리스크, 운영 리스크의 균형을 가장 잘 맞춘다. 이 평가는 Strangler Fig 문서와 앞선 아키텍처 조사 결과를 결합한 추론이다.

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig, https://support.captureone.com/hc/en-us/articles/360002556797-Selecting-the-appropriate-capture-preview, https://v2.tauri.app/es/develop/sidecar/, https://helpx.adobe.com/lightroom-classic/help/import-photos-automatically.html

### Development Workflows and Tooling

개발 워크플로우는 **Windows 실장비와 동일한 조건에서 빠르게 반복 검증할 수 있는 체계**가 중요하다. GitHub는 self-hosted runner를 사용하면 사용자가 직접 관리하는 시스템에서 GitHub Actions 작업을 실행할 수 있다고 설명한다. Boothy처럼 카메라 드라이버, GPU, sidecar, Windows 권한, 장비 연결 상태가 중요한 제품은 클라우드 CI만으로 충분하지 않고, booth와 유사한 Windows 장비 runner가 매우 유효하다.

E2E 도구로는 Playwright가 적합하다. 공식 문서는 설정 파일에서 CI일 때만 retries를 높이고, `trace: 'on-first-retry'`를 기본으로 둘 수 있다고 설명한다. 또한 Trace Viewer는 실패 시점의 DOM snapshot, 네트워크, 콘솔, 액션 타임라인을 함께 볼 수 있게 해 준다. Boothy의 핵심은 일반 웹 오류보다 "어느 시점에 어떤 preview가 보였는가"이므로, Playwright trace는 실제 사용자 체감 회귀를 잡는 데 유용하다. 다만 브라우저 UI 바깥의 네이티브 렌더 구간은 OpenTelemetry나 앱 내부 requestId 로그와 결합해야 완전해진다. 이 판단은 Playwright와 OTel 문서를 결합한 추론이다.

_Source:_ https://docs.github.com/actions/concepts/runners/self-hosted-runners, https://playwright.dev/docs/test-configuration, https://playwright.dev/docs/trace-viewer-intro, https://playwright.dev/docs/test-retries, https://opentelemetry.io/docs/concepts/signals/

### Testing and Quality Assurance

테스트 전략은 **shift-left + shift-right**의 이중 구조가 적절하다. Microsoft는 shift-left에서 빠르고 신뢰할 수 있는 L0/L1 테스트를 최대한 앞단에 배치해 품질을 upstream으로 옮겨야 한다고 설명한다. 반대로 shift-right는 일부 테스트를 실제 운영 환경 또는 실제 사용자 워크로드에 더 가까운 시점으로 옮겨, 실제 조건에서 품질을 검증하는 접근이다. Boothy는 이 둘이 모두 필요하다.

따라서 unit/integration 단계에서는 `preset selection`, `requestId correlation`, `preview state machine`, `fallback policy`를 빠른 테스트로 보호하고, 실장비 단계에서는 `button-pressed -> preset-preview-visible` 시간과 실패율을 실제 장비에서 검증해야 한다. 여기서 핵심은 "단지 이미지가 뜨는가"가 아니라, "선택된 프리셋이 적용된 같은 촬영 결과가 SLA 안에 보이는가"를 acceptance 기준으로 삼는 것이다. 이것은 Microsoft shift-left/right 문서와 현재 제품 목표를 결합한 해석이다.

_Source:_ https://learn.microsoft.com/en-us/devops/develop/shift-left-make-testing-fast-reliable, https://learn.microsoft.com/en-us/devops/deliver/shift-right-test-production, https://playwright.dev/docs/trace-viewer-intro, https://playwright.dev/docs/test-configuration

### Deployment and Operations Practices

배포 전략은 **blue-green 또는 canary처럼 되돌리기 쉬운 점진 배포**가 맞다. Microsoft는 blue-green deployment가 다운타임을 줄이고 새 버전 배포 위험을 낮추며, 문제가 있으면 이전 버전으로 쉽게 rollback할 수 있다고 설명한다. 또한 canary 배포는 새 버전을 작은 트래픽에 먼저 노출해 관찰한 뒤 확장하는 전략이다. Boothy가 로컬 앱/sidecar 구조라면 전통적인 서버 트래픽 분배와 동일하지는 않지만, 개념적으로는 `새 preview renderer를 일부 booth 또는 일부 preset/session에만 제한 적용`하는 방식으로 그대로 응용할 수 있다.

운영 관측은 OpenTelemetry 기반이 표준적이다. OpenTelemetry는 traces, metrics, logs를 핵심 observability signals로 설명하고, Azure Monitor Application Insights는 OpenTelemetry 기반 계측을 기본 권장 경로로 안내한다. 현재 시점에서 profiles는 OpenTelemetry 문서상 public alpha이므로, 1차 운영 표준은 traces/metrics/logs에 두고 profiles는 후속 성능 심화 분석용으로 두는 편이 더 적절하다. 이 판단은 OTel 상태 문서에 근거한다.

_Source:_ https://learn.microsoft.com/en-us/azure/container-apps/blue-green-deployment, https://learn.microsoft.com/en-us/azure/devops/pipelines/ecosystems/kubernetes/canary-demo, https://opentelemetry.io/docs/concepts/signals/, https://opentelemetry.io/docs/concepts/signals/profiles/, https://learn.microsoft.com/en-us/azure/azure-monitor/app/opentelemetry-overview

### Team Organization and Skills

팀 구성은 전통적인 프론트/백 분리보다 **camera IO + preview engine + product instrumentation** 역량이 더 중요하다. 즉, 최소한 아래 네 축이 필요하다.

- 카메라 연결과 SDK/프로토콜 계층을 다룰 수 있는 역량
- 로컬 네이티브 렌더/GPU/캐시를 다룰 수 있는 역량
- 앱 셸과 상태 모델, UI 노출 정책을 설계할 수 있는 역량
- requestId 기반 관측성과 실장비 성능 검증을 운영할 수 있는 역량

DORA는 문서 품질, 사용자 중심성, 측정 체계가 조직 성과와 연결된다고 강조한다. 이 말은 곧, 단순히 엔지니어 수를 늘리는 것보다 `어떤 체감 속도를 제품 성공으로 볼지`, `그 수치를 누구나 추적할 수 있게 만들지`가 더 중요하다는 뜻이다. 현재 과제는 이미지 처리 문제이면서 동시에 운영/측정 문제이기도 하다. 이 평가는 DORA 문서와 현재 과제를 결합한 추론이다.

_Source:_ https://dora.dev/devops-capabilities/process/documentation-quality/, https://dora.dev/capabilities/user-centric-focus/, https://dora.dev/guides/value-stream-management/, https://docs.github.com/actions/concepts/runners/self-hosted-runners

### Cost Optimization and Resource Management

비용 최적화의 핵심은 **렌더 경로 전체를 바꾸기 전에, 가장 비싼 지연 구간만 먼저 대체하는 것**이다. Strangler Fig 방식은 이 점에서 유리하다. 기존 UI와 세션 관리, 기존 안정 경로를 유지한 채, `preset-applied first-visible`만 새 경로로 바꾸면 투자 대비 효과를 가장 빨리 볼 수 있다.

반대로 상용 엔진 브리지는 구현 속도 면에서 유리할 수 있지만, 라이선스 비용, 배포 방식, 자동화 제어 범위, 장애 시 대응권이 제약될 수 있다. 로컬 custom renderer는 초기 구현비가 더 들 수 있지만 장기 제어권과 제품 최적화 여지가 크다. 따라서 비용 관점의 현실적 순서는 `현재 셸 유지 -> 저지연 렌더 경로 추가 -> 상용 브리지 또는 edge 전환 판단`이다. 이 평가는 기술 조사 결과를 바탕으로 한 추론이다.

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig, https://support.captureone.com/hc/en-us/articles/360002556797-Selecting-the-appropriate-capture-preview, https://helpx.adobe.com/lightroom-classic/help/import-photos-automatically.html, https://docs.aws.amazon.com/prescriptive-guidance/latest/mes-on-aws/edge.html

### Risk Assessment and Mitigation

가장 큰 리스크는 세 가지다. 첫째, **새 렌더 경로가 빨라져도 실제 프리셋 적용 결과가 first-visible이 되지 않을 수 있는 리스크**다. 둘째, **현재 경로와 새 경로의 requestId 상관관계가 깨져 잘못된 촬영/세션 결과를 보여줄 리스크**다. 셋째, **운영 배포 후 일부 장비에서만 GPU/드라이버/카메라 조합 문제가 발생할 리스크**다.

이에 대한 완화책은 공식 자료가 권장하는 운영 패턴과 잘 맞는다. Strangler Fig로 단계적으로 치환하고, blue-green/canary로 제한 롤아웃하며, traces/metrics/logs로 requestId 전 구간을 관찰해야 한다. 또한 shift-right 테스트를 통해 실제 장비 조건에서 회귀를 확인해야 한다. 이 조합이 현재 과제에서 가장 현실적인 리스크 대응 구조다.

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig, https://learn.microsoft.com/en-us/azure/container-apps/blue-green-deployment, https://learn.microsoft.com/en-us/azure/devops/pipelines/ecosystems/kubernetes/canary-demo, https://opentelemetry.io/docs/concepts/signals/, https://learn.microsoft.com/en-us/devops/deliver/shift-right-test-production

## Technical Research Recommendations

### Implementation Roadmap

권장 실행 순서는 다음과 같다.

1. `requestId` 기준 first-visible preset latency를 공식 KPI로 정의하고 traces/metrics/logs를 연결한다.
2. 현재 앱 셸을 유지한 채 `preset-applied first-visible sidecar`를 별도 저지연 경로로 붙인다.
3. cold start 제거를 위해 preset preload, renderer warm-up, cache priming을 상시 유지한다.
4. 일부 booth 또는 일부 조건에서만 새 경로를 켜는 canary/blue-green 성격의 제한 배포를 한다.
5. 성과가 확인되면 기존 느린 경로를 fallback으로 남기고, 새 경로를 기본값으로 승격한다.
6. 그래도 목표를 못 맞추면 watched-folder bridge, 로컬 dedicated renderer, edge appliance 순으로 더 근본적인 전환을 검토한다.

_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig, https://learn.microsoft.com/en-us/azure/container-apps/blue-green-deployment, https://learn.microsoft.com/en-us/azure/devops/pipelines/ecosystems/kubernetes/canary-demo, https://opentelemetry.io/docs/concepts/signals/

### Technology Stack Recommendations

기술 스택 우선 추천은 다음과 같다.

- 제품 셸: 현재 Tauri 기반 구조 유지
- 카메라 제어: 현재 Canon SDK 경로 유지, 필요 시 CCAPI/다른 adapter 준비
- first-visible renderer: 로컬 네이티브 sidecar 또는 dedicated local worker
- observability: OpenTelemetry traces/metrics/logs + 중앙 수집
- 실장비 E2E: Playwright trace + requestId 로그 결합
- 대체 경로: watched folder 기반 외부 렌더 엔진 브리지

이 조합이 현재 리스크와 속도 개선 가능성의 균형이 가장 좋다. 이 판단은 본 리서치 전체를 종합한 추론이다.

_Source:_ https://v2.tauri.app/es/develop/sidecar/, https://cam.start.canon/vi/C017/manual/html/UG-06_Network_0130.html, https://opentelemetry.io/docs/concepts/signals/, https://playwright.dev/docs/trace-viewer-intro, https://helpx.adobe.com/lightroom-classic/help/import-photos-automatically.html

### Skill Development Requirements

필수 역량은 아래와 같다.

- 카메라/프로토콜/SDK 이해
- RAW/preview/caching/GPU 처리 이해
- 로컬 멀티프로세스 앱 설계
- requestId 기반 observability 설계
- 실장비 성능 검증과 롤아웃 운영

특히 이번 과제는 "이미지 처리 엔진 개발"과 "프로덕트 체감 속도 운영"이 동시에 필요하므로, 한쪽만 강한 팀으로는 부족할 가능성이 높다. 이 판단은 본 리서치 전체를 종합한 추론이다.

_Source:_ https://docs.github.com/actions/concepts/runners/self-hosted-runners, https://opentelemetry.io/docs/concepts/signals/, https://learn.microsoft.com/en-us/devops/develop/shift-left-make-testing-fast-reliable, https://dora.dev/capabilities/user-centric-focus/

### Success Metrics and KPIs

성공 측정은 개발 생산성 지표와 제품 체감 지표를 분리해서 봐야 한다.

**제품 핵심 KPI**

- `button-pressed -> preset-preview-visible` p50 / p95
- `preset-preview-visible success rate`
- `wrong-capture / wrong-session mismatch rate`
- `fallback to slow path rate`
- `cold start penalty` 유무

**개발/운영 보조 KPI**

- change lead time
- deployment frequency
- failed deployment recovery time
- change fail rate
- deployment rework rate

DORA는 2026년 1월 5일 기준, software delivery throughput과 instability를 설명하는 다섯 지표 체계로 metrics를 정리하고 있다. Boothy는 여기에 product latency KPI를 결합해 보는 편이 맞다. 이 결론은 DORA 문서와 현재 과제를 결합한 추론이다.

_Source:_ https://dora.dev/guides/dora-metrics/history, https://dora.dev/research/2024/dora-report/, https://opentelemetry.io/docs/concepts/signals/

## Research Synthesis

# 선택된 프리셋이 적용된 첫 결과를 즉시 보여주기 위한 기술 연구

## Executive Summary

이 리서치는 Boothy의 핵심 사용자 경험 문제를 하나의 질문으로 압축해 다뤘다. **테더링 직후 선택된 프리셋이 적용된 같은 촬영 결과를, 사용자가 지연으로 느끼지 않도록 얼마나 빨리 첫 화면으로 만들 수 있는가.** 조사 결과, 현재 시장 기준의 빠른 제품들은 단순히 렌더 엔진이 빠른 것이 아니라, `첫 표시용 경로`와 `최종 품질 경로`를 전략적으로 분리하고, 카메라 입력, 로컬 캐시, GPU 가속, preview 정책을 하나의 제품 설계 문제로 다룬다는 공통점을 보였다.

현재 Boothy에 가장 적합한 전략은 **현재 앱 셸 유지 + 선택된 프리셋이 적용된 first-visible 전용 저지연 렌더 워커 추가**다. 이 방향은 현재 제품 구조를 유지하면서도 가장 큰 체감 병목에 직접 대응할 수 있다. 이때 성공 조건은 단순 "이미지가 빨리 뜬다"가 아니라, `selected-preset applied`, `same-capture guaranteed`, `requestId correlated`, `first-visible SLA achieved`를 동시에 만족하는 것이다.

현재 기술로 목표를 달성하지 못할 가능성도 충분히 있다. 그 경우에도 다음 수순은 명확하다. **로컬 dedicated renderer**, **watch-folder 기반 외부 엔진 브리지**, **edge render appliance + thin client** 순으로 전환 폭을 넓히는 것이 가장 현실적이다. 즉, 이번 리서치의 결론은 “조금 더 튜닝해보자”가 아니라, **점진적 치환을 전제로 한 제품 수준의 preview 전략 재정의**가 필요하다는 것이다.

**Key Technical Findings:**

- 시장 기준의 빠른 제품은 `single-path full render`보다 `multi-stage preview strategy`를 사용한다.
- first-visible 경험은 UI 렌더링보다 `카메라 입력`, `preview source`, `캐시`, `프리셋 정책`, `로컬 GPU`에 더 크게 좌우된다.
- 현재 Boothy의 1차 정답은 전면 재구축이 아니라 `local low-latency preset preview lane` 추가다.
- queue, 과도한 microservices, 느린 파일 중심 경계는 첫 체감 화면 경로에 불리하다.
- observability가 기능 부속물이 아니라 제품 전략 일부여야 한다.

**Technical Recommendations:**

- `button-pressed -> preset-preview-visible`를 핵심 제품 KPI로 승격한다.
- 현재 앱 셸을 유지한 채 preset-applied first-visible sidecar/worker를 추가한다.
- cold start, preset warm-up, cache priming을 상주형 구조로 재설계한다.
- traces / metrics / logs를 requestId 기준으로 연결하고 rollout은 canary/blue-green 성격으로 제한 적용한다.
- 현재 기술이 부족하면 local dedicated renderer, watched-folder bridge, edge appliance 순으로 단계적으로 전환한다.

## Table of Contents

1. Technical Research Introduction and Methodology
2. 현재 기술 지형과 아키텍처 분석
3. 구현 접근 방식과 베스트 프랙티스
4. 기술 스택 진화와 현재 트렌드
5. 통합 및 상호운용 패턴
6. 성능 및 확장성 분석
7. 보안 및 준수 고려사항
8. 전략적 기술 권고안
9. 구현 로드맵과 위험 평가
10. 향후 기술 전망과 혁신 기회
11. 리서치 방법론과 출처 검증
12. 부록과 참고 자료

## 1. Technical Research Introduction and Methodology

### Technical Research Significance

테더링 제품에서 사용자가 느끼는 속도는 단순 처리 시간보다 **무엇이 먼저 보이느냐**에 의해 좌우된다. 특히 프리셋이 중요한 제품에서는 "같은 촬영 결과가 보인다"만으로는 충분하지 않고, "선택된 프리셋이 적용된 결과가 바로 보인다"가 제품 완성도를 결정한다. Capture One이 `Immediately`와 `When ready`를 정책으로 노출하고, Lightroom Classic이 tethered import와 preview/cache 전략을 계속 강조하는 이유도 여기에 있다.

_Technical Importance:_ first-visible preview 전략은 camera IO, render engine, cache, preset policy, observability가 동시에 맞물리는 종단간 시스템 설계 문제다.  
_Business Impact:_ 제품 체감 속도는 사용자 신뢰, 촬영 흐름 유지, 장비 활용도, 도입 경쟁력에 직접 연결된다.  
_Source:_ https://support.captureone.com/hc/en-us/articles/360002556797-Selecting-the-appropriate-capture-preview, https://helpx.adobe.com/lightroom-classic/help/import-photos-tethered-camera.html, https://helpx.adobe.com/lightroom-classic/kb/optimize-performance-lightroom.html

### Technical Research Methodology

- **Technical Scope:** 카메라 제어 계층, 로컬 렌더 엔진, preview/cache, sidecar/edge 배치, observability, rollout, 조직 역량
- **Data Sources:** Canon, Adobe, Capture One, darktable, LibRaw, RawSpeed, Tauri, Microsoft Learn, AWS, NVIDIA, OpenTelemetry, DORA 공식 자료
- **Analysis Framework:** 기술 스택 -> 통합 패턴 -> 아키텍처 -> 구현 전략 -> 운영/리스크 순으로 누적 분석
- **Time Period:** 2026년 4월 3일 기준 공개 자료 중심
- **Technical Depth:** 제품 의사결정과 구현 우선순위를 동시에 지원할 수 있는 실무 수준

### Technical Research Goals and Objectives

**Original Technical Goals:** 같은 촬영의 필터 적용 결과를 최근 세션 썸네일에 더 빠르게 노출하기 위해 cold start, preview render warm-up, preset apply elapsed, fast preview/fallback 구조를 기술적으로 분석하고 최적화 방향을 도출한다.

**Achieved Technical Objectives:**

- 현재 시장/오픈소스에서 빠른 first-visible 경험을 만드는 기술 패턴을 확인했다.
- Boothy에 가장 적합한 기본 아키텍처 후보를 도출했다.
- 점진적 치환과 운영 관측성을 포함한 구현 전략을 정리했다.
- 현재 기술로 불충분할 경우 열어야 할 상위 전환 옵션을 정리했다.

## 2. 현재 기술 지형과 아키텍처 분석

### Current Technical Architecture Patterns

현재 우세한 구조는 `카메라 입력을 로컬에서 최대한 짧게 받고`, `빠른 preview artifact를 먼저 만들고`, `나중에 품질 경로로 교체하는` 패턴이다. 상용 제품은 이 전략을 제품 정책으로 노출하고, 오픈소스 스택은 cache와 GPU path를 통해 이를 뒷받침한다.

_Dominant Patterns:_ direct camera control, local cache, GPU-assisted preview, staged preview policy  
_Architectural Evolution:_ full render 단일 경로에서 multi-stage preview 경로로 이동  
_Architectural Trade-offs:_ 단순성보다 first-visible 속도와 correctness를 우선해야 함  
_Source:_ https://support.captureone.com/hc/en-us/articles/360002556797-Selecting-the-appropriate-capture-preview, https://docs.darktable.org/usermanual/4.0/en/tethering/overview/, https://helpx.adobe.com/lightroom-classic/kb/lightroom-gpu-faq.html

### System Design Principles and Best Practices

가장 중요한 원칙은 `inside/outside 분리`, `preview policy의 명시화`, `dual artifact model`, `점진적 대체 가능성`이다. 즉, 선택된 프리셋 적용 여부와 first-visible SLA는 제품 코어 규칙이어야 하고, 실제 엔진과 카메라 연결 방식은 adapter로 취급하는 편이 옳다.

_Design Principles:_ port-and-adapter, explicit preview policy, requestId correlation, hot-path isolation  
_Best Practice Patterns:_ local render worker, warm cache, sidecar 교체 가능 구조  
_Architectural Quality Attributes:_ latency, correctness, recoverability, maintainability  
_Source:_ https://alistair.cockburn.us/hexagonal-architecture, https://v2.tauri.app/es/develop/sidecar/, https://helpx.adobe.com/lightroom-classic/kb/optimize-performance-lightroom.html

## 3. Implementation Approaches and Best Practices

### Current Implementation Methodologies

구현은 전면 재개발보다 점진적 치환이 맞다. Strangler Fig 방식으로 새 preset preview lane을 도입하고, 기존 경로는 fallback으로 유지한 채 성능과 정확성을 검증해야 한다. 배포는 제한 적용, 계측은 전 구간 연결, 실장비 검증은 별도 단계로 분리하는 것이 적절하다.

_Development Approaches:_ incremental replacement, sidecar-first adoption, measured rollout  
_Code Organization Patterns:_ core policy + adapter separation, command/event 분리  
_Quality Assurance Practices:_ shift-left unit/integration + shift-right hardware validation  
_Deployment Strategies:_ blue-green / canary 성격의 제한 적용  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig, https://learn.microsoft.com/en-us/devops/develop/shift-left-make-testing-fast-reliable, https://learn.microsoft.com/en-us/devops/deliver/shift-right-test-production

### Implementation Framework and Tooling

가장 실용적인 조합은 Tauri 셸 유지, native sidecar renderer, Playwright trace, OpenTelemetry 기반 requestId 계측이다. CI는 booth와 유사한 Windows self-hosted runner가 핵심이다.

_Development Frameworks:_ Tauri sidecar, local native worker, OpenTelemetry instrumentation  
_Tool Ecosystem:_ Playwright trace, GitHub Actions self-hosted runners, App Insights or OTLP collector  
_Build and Deployment Systems:_ Windows 실장비 기반 자동 검증 + 제한 롤아웃  
_Source:_ https://v2.tauri.app/es/develop/sidecar/, https://playwright.dev/docs/trace-viewer-intro, https://docs.github.com/actions/concepts/runners/self-hosted-runners, https://learn.microsoft.com/en-us/azure/azure-monitor/app/opentelemetry-overview

## 4. Technology Stack Evolution and Current Trends

### Current Technology Stack Landscape

핵심 hot path는 여전히 C/C++ 네이티브 계층과 카메라 SDK, preview extraction 라이브러리, GPU-friendly renderer에 의해 주도된다. 상위 셸은 Rust/TypeScript/C# 같은 생산성 계층으로 감싸는 다계층 구조가 현실적이다.

_Programming Languages:_ C/C++ 중심의 imaging core, 상위 생산성 계층의 셸 분리  
_Frameworks and Libraries:_ Canon SDK/CCAPI, LibRaw, RawSpeed, darktable/OpenCL, Adobe/Camera Raw, Capture One  
_Database and Storage Technologies:_ catalog보다 preview cache와 image cache가 더 중요  
_API and Communication Technologies:_ direct SDK, HTTP/HTTPS CCAPI, local command/event IPC  
_Source:_ https://www.usa.canon.com/support/sdk, https://www.libraw.org/about, https://github.com/darktable-org/rawspeed, https://helpx.adobe.com/lightroom-classic/kb/optimize-performance-lightroom.html

### Technology Adoption Patterns

최근 방향은 `full render를 기다리지 않는 것`, `embedded preview와 richer intermediate format 활용`, `GPU warm 상태 유지`, `cache precomputation 강화`로 요약된다.

_Adoption Trends:_ preview-first 전략 강화  
_Migration Patterns:_ 기존 UI 유지 + render path 점진 교체  
_Emerging Technologies:_ DNG JPEG-XL preview, Canon H265 thumbnail, GPU-resident execution  
_Source:_ https://www.libraw.org/download, https://docs.nvidia.com/holoscan/sdk-user-guide/gpu_resident.html, https://support.captureone.com/hc/en-us/articles/360002556797-Selecting-the-appropriate-capture-preview

## 5. 통합 및 상호운용 패턴

### Current Integration Approaches

통합의 핵심은 `동기 제어 + 비동기 상태 전파` 혼합형이다. capture와 preset 선택은 command로, preview-ready와 visible은 event로 분리하는 것이 현재 문제와 가장 잘 맞는다.

_API Design Patterns:_ direct control API + local IPC  
_Service Integration:_ same-machine bounded system, 필요 시 sidecar/edge로 확장  
_Data Integration:_ requestId로 묶인 dual artifact 흐름  
_Source:_ https://v2.tauri.app/concept/inter-process-communication/, https://cam.start.canon/vi/C017/manual/html/UG-06_Network_0130.html, https://learn.microsoft.com/en-us/azure/architecture/patterns/cqrs

### Interoperability Standards and Protocols

표준 관점에서는 SDK/API뿐 아니라 watch-folder interoperability도 중요하다. direct tether 경로가 한계에 부딪히면 외부 상용 엔진과의 파일 기반 브리지로 전환할 수 있어야 한다.

_Standards Compliance:_ 벤더 SDK/CCAPI, 파일 기반 watched-folder 호환  
_Protocol Selection:_ direct SDK 우선, 네트워크 제어 필요 시 CCAPI/HTTPS  
_Integration Challenges:_ 잘못된 capture correlation, 늦은 preview propagation, 파일 경계 지연  
_Source:_ https://helpx.adobe.com/lightroom-classic/help/import-photos-automatically.html, https://docs.darktable.org/usermanual/4.0/en/tethering/overview/, https://www.rfc-editor.org/rfc/rfc9110

## 6. 성능 및 확장성 분석

### Performance Characteristics and Optimization

이번 주제는 scale-out보다 low-latency placement 문제다. 카메라, 저장소, GPU에 가까울수록 유리하고, cold start와 pipeline warm-up을 사실상 사용자 밖으로 밀어내야 한다.

_Performance Benchmarks:_ 사용자 체감 핵심은 `button-pressed -> preset-preview-visible`  
_Optimization Strategies:_ preset preload, warm cache, GPU resident path, dual artifact model  
_Monitoring and Measurement:_ traces / metrics / logs 기반 requestId 계측  
_Source:_ https://docs.aws.amazon.com/wellarchitected/latest/framework/perf_networking_choose_workload_location_network_requirements.html, https://docs.nvidia.com/cuda/archive/13.1.1/cuda-programming-guide/04-special-topics/cuda-graphs.html, https://opentelemetry.io/docs/concepts/signals/

### Scalability Patterns and Approaches

확장성은 사용자 수보다 운영 조합 수에 가깝다. 즉, 많은 booth/장비 조합과 다양한 GPU/카메라 조건을 감당해야 하므로, hot path와 background path 분리가 중요하다.

_Scalability Patterns:_ low-latency lane vs background lane  
_Capacity Planning:_ booth 단위 GPU/스토리지/카메라 조합 기준  
_Elasticity and Auto-scaling:_ 클라우드 자동확장보다 로컬 워커/edge 증설 개념이 더 적합  
_Source:_ https://learn.microsoft.com/sl-si/azure/architecture/patterns/queue-based-load-leveling, https://docs.aws.amazon.com/prescriptive-guidance/latest/mes-on-aws/edge.html

## 7. 보안 및 준수 고려사항

### Security Best Practices and Frameworks

속도를 해치지 않으면서도 권한을 최소화하는 보안 구조가 적절하다. 로컬 IPC와 sidecar 실행은 capability와 allow-list로 제한하고, 네트워크 카메라/edge 제어는 HTTPS와 인증을 기본으로 둬야 한다.

_Security Frameworks:_ Tauri permissions/capabilities, HTTPS, circuit breaker 기반 실패 격리  
_Threat Landscape:_ 과도한 실행 권한, 잘못된 파일 접근, 원격 엔진 장애 전파  
_Secure Development Practices:_ sidecar 권한 최소화, 이벤트 권한 최소화, requestId 감사 가능성 확보  
_Source:_ https://v2.tauri.app/security/permissions/, https://v2.tauri.app/reference/acl/core-permissions/, https://learn.microsoft.com/en-us/azure/architecture/patterns/circuit-breaker

### Compliance and Regulatory Considerations

이번 소스 범위에서는 특정 외부 규제 프레임워크보다, 제품 내부의 안전 운영과 이미지 처리 흐름의 무결성이 더 중요하게 나타났다. 따라서 현재 우선순위는 formal compliance보다 secure operation, auditability, privacy-aware deployment다. 이 판단은 현재 source set 기반의 추론이다.

_Industry Standards:_ 보안 통신과 최소 권한 원칙  
_Regulatory Compliance:_ 현 단계에서 제품별/지역별 별도 조사 필요  
_Audit and Governance:_ requestId trace와 rollout 기록이 핵심  
_Source:_ https://cam.start.canon/vi/C017/manual/html/UG-06_Network_0130.html, https://opentelemetry.io/docs/concepts/signals/

## 8. 전략적 기술 권고안

### Technical Strategy and Decision Framework

가장 추천하는 결정 프레임은 아래와 같다.

- 먼저 `preset-applied first-visible`를 제품 핵심 성능 문제로 재정의한다.
- 현재 셸 유지 전략을 기본으로 두고, 새 render lane만 점진 치환한다.
- requestId 기준 observability 없이는 구조 변경을 승인하지 않는다.
- rollout은 작은 범위에서 제한 적용하고, 성공 기준은 latency + correctness를 함께 본다.

_Architecture Recommendations:_ local shell + low-latency preset renderer  
_Technology Selection:_ direct SDK / local renderer / OTel / Playwright trace / warm cache  
_Implementation Strategy:_ Strangler Fig + canary/blue-green 성격 제한 적용  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig, https://opentelemetry.io/docs/concepts/signals/, https://playwright.dev/docs/trace-viewer-intro

### Competitive Technical Advantage

기술 차별화 포인트는 화질 자체보다 **프리셋 적용 결과를 first-visible로 만드는 일관성**에 있다. 사용자가 "바로 보인다"고 느끼는 지점은 여기서 나온다. 이 부분을 잡으면 Capture UX 경쟁력이 크게 올라간다.

_Technology Differentiation:_ preset-applied first-visible correctness  
_Innovation Opportunities:_ richer intermediate preview, GPU resident pipeline, adaptive preview policy  
_Strategic Technology Investments:_ low-latency renderer, observability, hardware validation  
_Source:_ https://support.captureone.com/hc/en-us/articles/360002556797-Selecting-the-appropriate-capture-preview, https://docs.nvidia.com/holoscan/sdk-user-guide/gpu_resident.html

## 9. 구현 로드맵과 위험 평가

### Technical Implementation Framework

**Phase 1: 계측과 정책 정리**

- requestId 전 구간 traces/metrics/logs 연결
- 제품 KPI를 `button-pressed -> preset-preview-visible`로 확정
- wrong-capture / wrong-session 방지 규칙 확정

**Phase 2: 저지연 preset renderer 도입**

- 현재 셸 유지
- 별도 local sidecar/worker 추가
- preset preload, warm cache, startup warm-up 도입

**Phase 3: 제한 적용과 기본값 승격**

- canary/blue-green 성격으로 일부 booth/조건에 적용
- 성공 시 기본값으로 승격, 기존 경로는 fallback 유지

**Phase 4: 상위 전환 옵션 검토**

- local dedicated renderer
- watched-folder bridge
- edge appliance

_Implementation Phases:_ 점진적 치환 4단계  
_Technology Migration Strategy:_ 기존 경로 유지 + 새 경로 병행 검증  
_Resource Planning:_ camera SDK, native render, observability, hardware validation 역량 확보  
_Source:_ https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig, https://learn.microsoft.com/en-us/azure/container-apps/blue-green-deployment, https://learn.microsoft.com/en-us/azure/devops/pipelines/ecosystems/kubernetes/canary-demo

### Technical Risk Management

_Technical Risks:_ 새 경로가 빠르지만 correctness를 깨뜨릴 수 있음  
_Implementation Risks:_ GPU/드라이버/카메라 조합별 회귀 가능성  
_Business Impact Risks:_ 일부 장비에서만 느려지거나 잘못된 사진이 보일 경우 제품 신뢰 손상  

완화책은 `incremental rollout + observability + hardware validation + explicit fallback` 조합이다.

_Source:_ https://learn.microsoft.com/en-us/devops/deliver/shift-right-test-production, https://opentelemetry.io/docs/concepts/signals/, https://learn.microsoft.com/en-us/azure/architecture/patterns/circuit-breaker

## 10. 향후 기술 전망과 혁신 기회

### Emerging Technology Trends

_Near-term Technical Evolution:_ richer embedded preview와 더 나은 intermediate artifact 활용이 강화될 가능성  
_Medium-term Technology Trends:_ GPU resident pipeline, lower CPU scheduling overhead, local edge specialization  
_Long-term Technical Vision:_ camera/edge/UI가 느슨하게 결합된 but first-visible는 여전히 local에 가까운 구조

_Source:_ https://www.libraw.org/download, https://docs.nvidia.com/cuda/archive/13.1.1/cuda-programming-guide/04-special-topics/cuda-graphs.html, https://docs.nvidia.com/holoscan/sdk-user-guide/gpu_resident.html

### Innovation and Research Opportunities

- same-capture intermediate preview를 preset-aware하게 만드는 방법
- GPU warm 상태 유지와 preset variation precomputation
- adaptive preview policy: 장비 상태와 세션 조건에 따라 first-visible 경로를 동적으로 선택
- edge appliance + thin client UI 분리의 운영 모델

_Research Opportunities:_ same-capture preset-aware intermediate preview  
_Emerging Technology Adoption:_ GPU resident path와 richer preview format 우선  
_Innovation Framework:_ 측정 가능성 -> 작은 실험 -> 제한 롤아웃 -> 기본값 승격  
_Source:_ https://docs.nvidia.com/holoscan/sdk-user-guide/gpu_resident.html, https://www.libraw.org/download, https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig

## 11. 리서치 방법론과 출처 검증

### Comprehensive Technical Source Documentation

_Primary Technical Sources:_ Canon, Adobe Lightroom Classic, Capture One, darktable, LibRaw, RawSpeed, Tauri, Microsoft Learn, AWS, NVIDIA, OpenTelemetry, DORA  
_Secondary Technical Sources:_ 본 문서의 내부 브리프 및 프로젝트 기록  
_Technical Web Search Queries:_ 

- selected preset applied preview immediate display technology
- Lightroom Classic tethered capture preset import official
- Capture One immediately preview official
- Canon CCAPI official network API
- darktable tethering overview official
- LibRaw embedded preview thumbnail official
- Tauri IPC / sidecar official
- Strangler Fig pattern official
- shift-left testing official
- shift-right production testing official
- OpenTelemetry signals official
- blue-green deployment official
- canary rollout official
- CUDA Graphs official
- Holoscan GPU resident official
- DORA metrics official

### Technical Research Quality Assurance

_Technical Source Verification:_ 핵심 기술 판단은 가능한 한 공식 문서 또는 1차 소스만 사용했다.  
_Technical Confidence Levels:_ 전체적으로 높음. 단, Lightroom/Capture One 내부 엔진 세부 구현은 공개 자료가 제한되어 있어 일부는 제품 동작과 문서 기반 추론이다.  
_Technical Limitations:_ 상용 제품 내부 렌더 파이프라인의 완전한 구현 세부는 공개되지 않음.  
_Methodology Transparency:_ 검색 질의, 소스 유형, 추론 여부를 명시했다.  

## 12. 부록과 참고 자료

### Detailed Technical Data Tables

_Architectural Pattern Tables:_ local shell + sidecar / local dedicated renderer / watched-folder bridge / edge appliance 비교  
_Technology Stack Analysis:_ camera SDK, preview extraction, renderer, cache, observability 도구 비교  
_Performance Benchmark Data:_ 본 리서치는 외부 제품의 구조적 특성과 원리를 중심으로 했으며, Boothy 고유 수치는 별도 실장비 계측으로 보완해야 한다.

### Technical Resources and References

_Technical Standards:_ RFC 9110 HTTP, RFC 6455 WebSocket, OpenTelemetry signals  
_Open Source Projects:_ darktable, RawSpeed, LibRaw  
_Research Papers and Publications:_ 이번 범위에서는 공식 제품/플랫폼 문서 중심  
_Technical Communities:_ 각 제품 공식 문서, 오픈소스 저장소, observability community

---

## Technical Research Conclusion

### Summary of Key Technical Findings

이번 리서치의 최종 결론은, Boothy의 목표를 달성하려면 **선택된 프리셋이 적용된 같은 촬영 결과를 first-visible artifact로 만드는 제품 전략**이 필요하다는 것이다. 이를 위해서는 현재 셸을 유지하면서 새 저지연 렌더 경로를 점진적으로 추가하는 방향이 가장 현실적이다.

### Strategic Technical Impact Assessment

이 결정은 단순 성능 최적화가 아니라 제품 포지셔닝 결정이다. 사용자가 "문제없이 바로 보인다"고 느끼게 만드는 구조를 확보하면, 촬영 흐름과 신뢰가 동시에 개선된다.

### Next Steps Technical Recommendations

1. requestId 기준 제품 KPI 확정
2. traces / metrics / logs 연결
3. local preset preview worker 도입
4. 제한 롤아웃으로 성능/정확성 검증
5. 현재 기술 한계 시 상위 전환 옵션 검토

---

**Technical Research Completion Date:** 2026-04-03  
**Research Period:** current comprehensive technical analysis  
**Document Length:** comprehensive technical coverage  
**Source Verification:** official docs and primary sources used wherever possible  
**Technical Confidence Level:** High, with limited visibility into proprietary internal renderer details

_This document is intended to serve as a practical decision-making reference for improving Boothy's preset-applied first-visible experience and for choosing the next implementation path._

<!-- Content will be appended sequentially through research workflow steps -->
