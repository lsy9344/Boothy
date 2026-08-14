# Viewer Readiness 계약

## 목적

이 문서는 Story 7.1에서 도입한 전용 관람 창(`viewer-window`)의 readiness truth와
physical display-size 계약을 고정한다.

이 계약은 **사진이 놓일 자리와 그 자리에 필요한 픽셀 수**까지만 소유한다.
immutable display generation, display pointer, double-buffer swap, actual-present 계측은
Story 7.2 이후가 소유하며 이 문서에 추가하지 않는다.

> 그 자리에 **무엇을 언제 올릴 수 있는가**는 `docs/contracts/viewer-display.md`(Story 7.2)가 소유한다.
> 특히 `.viewer-surface__photo`의 기하와 standby 행의 레이아웃 점유는 이 문서의 layout-ready 계약이
> 걸려 있는 불변식이므로 표시 경로에서 바꿀 수 없다.

## 소유권 원칙

- `viewer-window`는 `booth-window`와 **별개의 WebView이고 별개의 React root**다.
  두 창은 서로의 React 상태를 볼 수 없다.
- 따라서 현재 세션 binding, viewer epoch, revision은 **host가 소유**한다.
  `localStorage`, `BroadcastChannel`, URL query로 `sessionId`를 옮기는 우회는 금지한다.
- snapshot이 durable recovery 경계다. live event만으로 readiness를 소유하지 않는다.
  viewer는 마운트/재연결마다 `get_viewer_readiness`를 먼저 읽는다.

## Viewer Readiness Snapshot v1

- schemaVersion: `viewer-readiness/v1`
- sessionId: host가 고정한 현재 세션. 세션 전 부팅 구간에서는 `null`
- viewerEpoch: viewer window 생성/재생성마다 증가
- revision: 모든 상태 변경마다 단조 증가. 절대 되돌아가지 않는다
- windowState: `absent` | `creating` | `open` | `closed`
- listenerReady: viewer의 이벤트 구독이 실제로 성립했는지
- layoutReady: 실측 photo rect가 기대한 3:2 contain-fit과 일치하는지
- monitorTargeting: `unresolved` | `approved-customer-monitor` | `single-monitor-fallback` |
  `unapproved-profile` | `monitor-unavailable`
- displayProfile: 승인된 profile일 때만 존재
  - profileId: `1080p` | `1440p` | `4k`
  - monitorName, monitorWidthPx, monitorHeightPx, monitorScaleFactor
- photoRect: 실측 CSS rect와 거기서 파생한 필요 소스 픽셀
  - cssWidth, cssHeight, devicePixelRatio
  - requiredSourceWidthPx, requiredSourceHeightPx
- viewerReady: 아래 모든 조건이 성립할 때만 `true`
- reasonCode: `viewer-ready` 또는 미준비 원인
- observedAtMs, lastReportAtMs

### viewerReady 판정 조건 (전부 AND)

1. `windowState == open`
2. `monitorTargeting == approved-customer-monitor`
3. `sessionId != null`
4. `listenerReady`
5. `layoutReady` 이고 `photoRect`가 존재
6. viewer가 보고한 세션 == host가 고정한 세션
7. 마지막 report가 host clock 기준 `VIEWER_REPORT_STALE_AFTER_MS`(5s) 이내

### reasonCode 우선순위

`viewer-absent` / `viewer-window-closed` → `monitor-not-approved` → `session-unbound`
→ `listener-not-ready` (또는 `stale-epoch`) → `layout-not-ready` (또는 `stale-epoch`)
→ `session-mismatch` → `stale-report` → `viewer-ready`

## Physical display-size 계약

필요 소스 픽셀은 고정 thumbnail 크기가 아니라 **실측 CSS rect × DPR**에서 파생한다.

```
requiredSourceWidthPx  = ceil(cssWidth  * devicePixelRatio)
requiredSourceHeightPx = ceil(cssHeight * devicePixelRatio)
```

올림을 쓰는 이유는 내림이 1px upscale을 허용하기 때문이다.

적합 판정: `naturalWidth >= requiredSourceWidthPx && naturalHeight >= requiredSourceHeightPx`

| Display profile | 기대 photo rect (CSS) | 최소 필요 photo pixels | 기본 proxy class |
| --- | --- | ---: | ---: |
| 1920×1080, DPR 1 | 1620×1080 | 1620×1080 | 1920×1280 |
| 2560×1440, DPR 1.25 | 1728×1152 | 2160×1440 | 2592×1728 |
| 3840×2160, DPR 2 | 1620×1080 | 3240×2160 | 3456×2304 |

`proxy class`는 Story 7.4가 생성할 목표값이며 Story 7.1의 완료 조건이 아니다.

고정 384px thumbnail(`src-tauri/src/render/mod.rs`의 preview cap)은 승인된 어떤 profile에서도
이 조건을 만족하지 못한다. 이 사실은 TS/Rust 양쪽 테스트로 고정되어 있다.

`requiredSource*`는 **host가 report에서 직접 재계산**한다. client가 보낸 계산 결과는 신뢰하지 않는다.

## Host commands

| command | 방향 | 설명 |
| --- | --- | --- |
| `get_viewer_readiness` | 양방향 | 현재 snapshot을 읽는다. booth/viewer 모두 같은 truth를 본다 |
| `ensure_viewer_window` | booth → host | viewer window를 보장한다. 없으면 새 epoch로 생성 |
| `report_viewer_listener_ready` | viewer → host | 구독 성립을 알린다 |
| `report_viewer_layout` | viewer → host | 실측 rect/DPR 보고이자 liveness heartbeat |

## Event

- `viewer-readiness-update` (`viewer-readiness-update/v1`)
- `viewer-session-binding-update`는 같은 durable snapshot envelope을 사용한다.
- `app.emit`으로 전체 창에 브로드캐스트한다. revision이 실제로 오를 때만 emit한다.

## Liveness와 background throttling

- viewer는 `report_viewer_layout`을 `VIEWER_HEARTBEAT_INTERVAL_MS`(1s) 주기로 재전송한다.
- 값이 변하지 않으면 host는 `last_report_at_ms`만 갱신하고 revision을 올리지 않는다.
- staleness는 wall-clock 보정의 영향을 받지 않는 **host monotonic clock**으로 판정한다. WebView2가 viewer 타이머를 throttle하면
  readiness가 정직하게 내려가고 촬영이 막힌다.

## Epoch와 stale 방어

- viewer window가 재생성되면 epoch가 증가하고 이전 epoch의 listener/layout readiness는 전부 무효화된다.
- 이전 epoch에서 늦게 도착한 report는 폐기하며 `stale-epoch`로 진단된다.
- client는 낮은 revision snapshot을 폐기한다. 단, revision이 같아도 reasonCode/epoch/session이
  바뀐 경우는 staleness 전이이므로 적용한다.

## Capture eligibility gate

- `viewer-preparing`을 `captureReasonCode`에 추가했다.
- gate는 **downgrade 전용**이다. 이미 막혀 있는 촬영(post-end, camera 준비 등)의 원인을 덮지 않고,
  촬영 가능 상태만 viewer 미준비로 내린다.
- 촬영은 `monitorTargeting == approved-customer-monitor`일 때만 허용한다.
  `single-monitor-fallback`, `unapproved-profile`, `monitor-unavailable`은 모두 정직하게 보고하면서
  `viewer-preparing`으로 촬영을 차단한다.

## Read-only surface 계약

- 관람 화면에는 editing, deletion, preset selection, navigation, diagnostic control이 없어야 한다.
- 포커스 가능한 요소를 두지 않는다.
- 준비 실패는 관람 화면이 아니라 booth control surface의 wait/call guidance로 전달한다.
- `/viewer` 라우트는 `SurfaceAccessGuard`를 쓰지 않는다. 접근 거부 시 `/booth`로 리다이렉트하면
  고객 모니터에 부스 조작 화면이 뜨기 때문이다. 전용 가드가 customer-safe blank를 유지한다.

## 창 권한

`src-tauri/capabilities/viewer-window.json`이 없으면 viewer webview는 IPC 권한이 0이라
`invoke`와 `listen`이 전부 실패한다. 이 파일은 선택 사항이 아니다.

## 설정

- `BOOTHY_APPROVED_CUSTOMER_MONITOR`: 승인된 고객 모니터 이름.
  설정되어 있으면 이름이 일치하는 모니터만 사용하고, 없으면 임의 대체 없이 `monitor-unavailable`로 보고한다.
  미설정 시에는 primary가 아닌 첫 모니터를 쓰고, 모니터가 하나뿐이면 `single-monitor-fallback`으로 보고한다.
