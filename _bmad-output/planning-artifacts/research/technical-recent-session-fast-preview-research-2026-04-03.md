---
stepsCompleted:
  - local-doc-review
  - local-codepath-review
  - web-pattern-verification
  - implementation-check
inputDocuments:
  - history/recent-session-thumbnail-speed-brief.md
  - history/current-session-photo-troubleshooting-history.md
  - src/booth-shell/components/SessionPreviewImage.tsx
  - src/booth-shell/screens/CaptureScreen.tsx
  - src/session-domain/state/session-provider.tsx
  - src-tauri/src/commands/capture_commands.rs
  - src-tauri/src/capture/normalized_state.rs
  - sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs
workflowType: 'research'
lastStep: 4
research_type: 'technical'
research_topic: 'recent-session fast preview latency under slow preset application'
research_goals: 'Understand current state from project docs and code, decide whether the recent-session thumbnail can appear immediately without product mismatch, and identify the least disruptive implementation path.'
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

이번 조사는 아래 두 축으로 진행했다.

1. 프로젝트 문서와 현재 워킹트리를 기준으로 `최근 세션` 썸네일 경로의 실제 상태를 다시 확인했다.
2. Lightroom Classic / darktable 공식 문서를 기준으로 "즉시 보이는 썸네일"과 "나중에 정확해지는 preview"를 분리하는 제품 패턴이 맞는지 검증했다.

핵심 질문은 하나였다.

- **프리셋 적용이 느린 상황에서도, 이 프로젝트의 제품 톤을 해치지 않고 같은 촬영의 썸네일을 바로 보여줄 수 있는가?**

---

## Executive Summary

결론은 명확하다.

- **가능하다.**
- 그리고 **이 프로젝트는 이미 그 방향의 구조를 상당 부분 가지고 있다.**
- 남아 있던 문제는 UI 구조가 아니라, **helper가 확보한 fast preview가 host의 capture-saved 응답 안에 바로 실리지 못하던 연결 방식**에 더 가깝다.

현재 코드 기준으로 가장 자연스러운 해법은 아래다.

1. 같은 촬영의 `camera thumbnail`이 있으면 helper가 **capture 저장 결과 안에 즉시 포함**한다.
2. host는 그 경로를 **canonical preview path**로 승격해 `previewWaiting` 상태에서도 바로 recent-session 슬롯에 노출한다.
3. 나중에 darktable 기반 preset-applied preview가 준비되면 **같은 경로를 교체**한다.

이 방식은

- 다른 사진을 대신 보여주지 않고
- `Preview Waiting` truth를 깨지 않으며
- 현재 제품 문맥과도 가장 잘 맞는다.

이번 워킹트리에는 이 방향의 첫 연결을 실제로 반영했다.

---

## Current State

### 1. 제품 의도는 이미 "빠른 첫 썸네일 -> 나중 교체"다

문서와 현재 프런트 구현 모두, recent-session 슬롯이 `previewReady`만 기다리도록 설계돼 있지 않다.

- `CurrentSessionPreview` selector는 `previewWaiting` 상태에서도 같은 capture의 displayable preview path가 있으면 노출할 수 있다.
- `SessionPreviewImage`는 pending preview를 우선 로드하고, later render가 오면 cache-buster로 자연스럽게 교체한다.
- `CaptureScreen`은 `pendingFastPreview`를 기존 rail 위에 합쳐 같은 촬영의 최신 슬롯으로 보여주도록 되어 있다.

즉 **프런트는 이미 준비돼 있다.**

### 2. 체감 지연의 중심은 preset-applied render 자체보다 fast preview handoff 연결이었다

문서상 recent-session latency 문제는 correctness 이슈를 지나 responsiveness 이슈로 재정의돼 있다.

현재 render worker는 여전히 `darktable-cli` 기반이라 preset-applied preview는 수초 단위 비용을 가질 수 있다.
이 자체는 구조적 한계가 맞다.

다만 현재 사용자 불편의 핵심은

- "정식 preview가 늦다" 자체보다
- **같은 촬영의 빠른 썸네일이 capture-saved 응답에 바로 붙지 않아, recent-session 첫 노출이 불필요하게 늦어질 수 있었다**는 점이다.

### 3. 실제 helper 흐름에는 작은 갭이 있었다

현재 helper는 RAW 다운로드 완료 뒤 fast preview 다운로드를 시도할 수 있었지만,
그 정보가 capture 결과 안에 바로 실리지 않는 경우가 있었다.

그 결과 host는 다음 둘 중 하나를 기다리게 됐다.

- 별도 fast-preview 이벤트
- 또는 더 느린 preset-applied preview render

이 연결 방식은 제품 관점에서 아깝다.
왜냐하면 **같은 촬영의 camera thumbnail이 이미 있는 샷에서도 first-visible이 한 박자 늦어질 수 있기 때문**이다.

---

## Product Judgment

### 1. "바로 보여주기"는 이질적이지 않다

외부 제품 패턴과 현재 제품 문서를 함께 보면,
같은 촬영의 camera-embedded preview를 먼저 보여주고
나중에 정확한 processed preview로 바꾸는 것은 충분히 자연스럽다.

Lightroom Classic 공식 문서는:

- tethered capture에서 가장 최근 촬영본을 preview area에 자동 표시할 수 있고,
- import 시 embedded preview를 먼저 빠르게 표시할 수 있으며,
- 더 정확한 preview는 이후 렌더링된다고 설명한다.

darktable 공식 문서도:

- raw import 시 embedded thumbnail 추출은 보통 매우 빠르며,
- 이 thumbnail은 나중에 darktable의 내부 처리 결과로 교체된다고 설명한다.

즉 **"먼저 진짜 같은 촬영 컷을 보여주고, 나중에 정확한 반영본으로 바꾼다"는 패턴은 업계적으로도 자연스럽다.**

### 2. 반대로, representative tile이나 다른 컷 재사용은 제품 이질감을 만든다

이 프로젝트의 핵심은 capture-bound truth다.
따라서 아래는 여전히 피해야 한다.

- 이전 촬영 컷 재사용
- preset representative tile을 같은 shot처럼 노출
- 아직 준비되지 않은 preview를 `ready`처럼 보이게 만드는 표현

이번 권장안은 이 금지선을 넘지 않는다.

---

## Implemented Change

이번 워킹트리에서 아래 변경을 적용했다.

- `CanonSdkCamera`가 같은 capture의 `camera thumbnail`을 즉시 얻는 경우,
  그 경로를 `CaptureDownloadResult.fastPreviewPath / fastPreviewKind`로 바로 반환하도록 조정했다.
- 즉시 thumbnail을 못 얻는 경우에는 기존 pending/backfill 경로를 그대로 유지한다.

의미:

- fast preview가 가능한 샷은 host가 `file-arrived` 시점에 곧바로 canonical preview로 승격할 수 있다.
- 따라서 recent-session 슬롯은 별도 늦은 이벤트를 기다리지 않고 **capture-saved 응답만으로 첫 썸네일을 띄울 가능성**이 커진다.
- 실패한 샷은 기존 fallback으로 자연스럽게 내려가므로 안정성 리스크가 작다.

이번 변경은 **정확도보다 즉시성을 위한 구조 보강**이지만, truth model은 그대로 유지한다.

---

## Recommendation

우선순위는 아래가 맞다.

### 1. 지금 적용한 fast-preview inline handoff를 기준선으로 삼는다

이게 가장 작고 자연스러운 개선이다.

기대 효과:

- recent-session first-visible 개선
- preset apply 대기 체감 완화
- 현재 UI / manifest semantics 유지

### 2. 다음 측정은 "capture-saved -> recent-session-pending-visible" 구간으로 좁힌다

이제 남은 성능 질문은 더 명확해졌다.

- camera thumbnail이 있는 샷에서 first-visible이 실제로 얼마나 줄었는지
- camera thumbnail miss 샷에서만 darktable 비용이 체감 병목으로 남는지

즉 다음 진단은 막연한 전체 latency가 아니라,
**fast-preview hit / miss 비율과 first-visible 구간**을 분리해서 보면 된다.

### 3. 그래도 느리면 다음 단계는 render warm-up이 아니라 same-capture source 강화다

만약 실장비에서 camera thumbnail hit율이 낮다면,
그 다음 우선순위는 darktable를 조금 더 빠르게 돌리는 것보다
same-capture proxy source를 더 안정화하는 쪽이 맞다.

---

## Confidence

- **높음:** 현재 제품 의도와 프런트 구조가 "빠른 썸네일 먼저 노출"을 이미 지원한다는 판단
- **높음:** preset-applied preview를 기다리지 않고도 제품 톤을 해치지 않는 방법이 존재한다는 판단
- **중간~높음:** 이번 helper 변경이 recent-session first-visible을 실제로 줄일 것이라는 판단
- **중간:** 실장비에서 camera-thumbnail hit율이 얼마나 안정적인지는 추가 계측 필요

---

## Sources

### Internal sources

- `history/recent-session-thumbnail-speed-brief.md`
- `history/current-session-photo-troubleshooting-history.md`
- `src/booth-shell/components/SessionPreviewImage.tsx`
- `src/booth-shell/screens/CaptureScreen.tsx`
- `src/session-domain/state/session-provider.tsx`
- `src-tauri/src/commands/capture_commands.rs`
- `src-tauri/src/capture/normalized_state.rs`
- `sidecar/canon-helper/src/CanonHelper/Runtime/CanonSdkCamera.cs`

### External sources

- Adobe Lightroom Classic, *Import photos from a tethered camera*  
  https://helpx.adobe.com/lightroom-classic/help/import-photos-tethered-camera.html

- Adobe Lightroom Classic, *How to specify import options*  
  https://helpx.adobe.com/lightroom-classic/help/photo-video-import-options.html

- darktable manual, *thumbnails*  
  https://docs.darktable.org/usermanual/3.6/en/lighttable/digital-asset-management/thumbnails/

- darktable manual, *darktable-cli*  
  https://docs.darktable.org/usermanual/4.8/en/special-topics/program-invocation/darktable-cli/
