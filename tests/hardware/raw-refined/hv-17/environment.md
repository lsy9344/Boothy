# HV-17 환경 — 템플릿

> **이 파일은 아직 실행되지 않은 회차의 템플릿이다.**
> 회차를 돌 때 `run-<timestamp>-hv17/environment.md`로 복사해 채운다.
>
> **읽을 수 없는 값은 만들어내지 않고 `unknown`으로 적는다.** 추정치를 적으면
> 그 숫자가 나중에 실측으로 인용된다.

실행: `<YYYY-MM-DD HH:MM +09:00>` · Story 7.6 RAW 정밀본 무중단 교체

## 이 회차가 무엇이고 무엇이 아닌가

이 회차는 **실제 촬영 세션**이다. HV-16처럼 구성요소 수준 측정이 아니다.

- 측정하는 것: scheduler span과 실제 실행 순서, 용량 상한, 취소·process tree,
  proxy → 정밀본 전환의 결정/상태 수준 무결성, tier 정당성 두 축, 사람 탐지 검사
- **측정하지 않는 것: 120fps+ 물리 monitor frame, compositor→photon 오프셋,
  frame 단위 zero-defect 판정.** → **HV-18B 소유**

## PC

| 항목 | 값 |
| --- | --- |
| OS | `<Windows 버전 / build / 아키텍처>` |
| CPU | `<모델 / core / thread>` |
| RAM | `<GB>` |
| GPU | `<모델 / driver 버전 / driver 날짜>` |
| 전원 구성표 | `<이름 / GUID>` |
| 전원 | `<연속 AC / 배터리>` |

HV-15·HV-16과 **같은 PC인지** 명시한다. 다르면 그 회차들의 종단 기준선과 직접 비교할 수 없다.

## 디스플레이 (고객 모니터)

| 항목 | 값 |
| --- | --- |
| 고객 모니터 | `<모델 / 식별자>` |
| 해상도 | `<W×H>` |
| DPR | `<devicePixelRatio>` |
| 주사율 | `<Hz>` |
| 승인 display profile | `<profileId>`, required source `<W×H>` |
| ICC | `<사용자 디스플레이 ICC 유무. 제품 출력은 sRGB / perceptual>` |
| HDR | `<활성 여부. 근거>` |

## 런타임

| 항목 | 값 |
| --- | --- |
| darktable | **5.4.1** (pin). 실제 경로: `<path>` |
| WebView2 Runtime | `<버전>` |
| Windows RAW decoder | `<이름 / 버전 또는 없음>` |
| 앱 버전 | `<version>` |
| 커밋 해시 | `<git rev-parse HEAD>` |

## 카메라

| 항목 | 값 |
| --- | --- |
| 모델 | Canon EOS 700D |
| 펌웨어 | `<버전>` |
| 렌즈 | `<모델 / 초점거리 / 조리개>` |
| 카드 | `<제조사 / 용량 / 속도 등급>` |
| 케이블 | `<USB 규격 / 길이>` |
| 전원 | `<배터리 / AC 어댑터>` |
| EDSDK | `<버전>` |
| helper | `<식별자 / 버전>` |

## Lane 설정 (전부 명시한다)

| 환경 변수 | 이 회차 값 | 제품 기본값 |
| --- | --- | --- |
| `BOOTHY_DISPLAY_PROXY_MODE` | `on` | `on` |
| `BOOTHY_RAW_REFINED_MODE` | `<off \| on>` | **`off`** (HV-17 `Go` 전까지) |
| `BOOTHY_DISPLAY_SAMPLE_MODE` | `off` | `off` |
| `BOOTHY_SOURCE_COMPARE_MODE` | `off` | `off` |
| `BOOTHY_RESIDENT_RENDERER_MODE` | `off` | `off` (HV-16 `Technology No-Go`) |

**계측 fixture lane(`BOOTHY_DISPLAY_SAMPLE_MODE`)이 켜져 있으면 그 회차의 표본은 오염된다.**

## tier 정당성 corpus

detail 축은 **해상도 차트 또는 명확한 slanted edge가 있는 실제 EOS 700D 촬영 최소 3장**을
요구한다. Story 7.5에서 MTF50이 미실행으로 남은 이유가 corpus였다 — HV-14 corpus 35장이
전부 세로 인물/책상 장면이라 slanted-edge 대상이 없었다.

| 항목 | 값 |
| --- | --- |
| slanted-edge 촬영 수 | `<n>` (최소 3) |
| 대상 | `<해상도 차트 모델 또는 경계 설명>` |
| 측정 preset | `preset_daylight`, `preset_mono-pop`, `preset_soft-glow` |
| 총 쌍 수 | `<n × 3>` (최소 9) |
| ROI 정의 | `tier-justification/pairs/<sample>/roi.json` |
| 노출 | `<정상 노출 확인. 저노출 표본은 look 축에서 제외되고 그 사실이 verdict에 남는다>` |

**자연 사진에서 slanted edge를 자동 탐지하지 않는다.** ROI는 사람이 지정한다 —
자동 탐지는 엉뚱한 영역을 재고 그 숫자가 evidence에 남는다.

## 탐지 검사 관찰자

| 항목 | 값 |
| --- | --- |
| 관찰자 수 | 3 |
| 관찰자 실명 | `detection-trial/signoff.md` |
| 시행 수 | 관찰자당 20 (교체 10 / 대조군 10, 무작위 순서) |
| 최종 승인자 | Noah Lee |

## 남은 지연의 소유

HV-16이 실장비 종단 median **8284 ms** 중 display 렌더가 **3716 ms**임을 지목했다.
렌더를 0으로 만들어도 **4568 ms**가 남고, 그 소유는 Story 7.6(렌더 큐 대기, 게시)과
Story 7.8(카메라 RAW 전송, present)이다.

| 항목 | 값 |
| --- | --- |
| 이 회차 종단 median | `<ms>` |
| 큐 대기 p50 / p95 / max | `<ms / ms / ms>` |
| 큐 대기가 종단에서 차지하는 비율 | `<%>` |
| Story 7.8로 넘기는 잔여 구간 | `<ms>` — `<근거>` |

**이 Story는 NFR-003의 warm p50 3초를 달성하지 못한다.** 그것은 이 Story의 실패가 아니다.
합격 조건은 무중단 교체의 정확성, scheduler·취소·process tree의 정확성, 그리고
**큐 대기 구간이 실제로 얼마였는지 정직하게 측정해 남은 지연의 소유를 7.8로 확정하는 것**이다.
