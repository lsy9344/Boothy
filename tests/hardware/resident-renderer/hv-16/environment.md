# HV-16 환경

실행: 2026-08-16 · Story 7.5 상주 renderer 검증 spike

## 이 회차가 무엇이고 무엇이 아닌가

이 회차는 **구성요소 수준 측정**이다. 실제 촬영 세션을 돌린 회차가 **아니다.**

- 측정한 것: pinned darktable one-shot 렌더 지연, Windows WIC direct CR2 디코드 지연,
  두 파이프라인의 시각 parity, 상주 엔진 계약의 자동 검증
- 측정하지 않은 것: 살아 있는 카메라 촬영 → 표시까지의 종단 지연,
  실제 상주 WebGL2 렌더의 GPU 실행 시간, 사람 눈 blind review

종단 지연 기준선은 Story 7.4 / HV-15의 실장비 회차
(`tests/hardware/display-proxy/run-20260815-131213-hv15-current-lighting/`) 값을 인용한다.
**같은 PC에서 측정된 값이므로 기기 간 비교가 아니다.**

## PC

| 항목 | 값 |
| --- | --- |
| OS | Microsoft Windows 11 Pro 10.0.26200 (build 26200, x64) |
| CPU | Intel Core i9-9900KF @ 3.60GHz — 8 core / 16 thread |
| RAM | 63.9 GB |
| GPU | NVIDIA GeForce GTX 1080, driver **32.0.15.6094** (2024-08-14) |
| 전원 구성표 | 균형 조정 (`381b4222-f694-41f0-9685-ff5bb260df2e`) |
| 전원 | 연속 AC |

HV-15 회차와 **동일한 PC·GPU·driver**다. 그래서 8284 ms 종단 기준선과 이 회차의
렌더 구간 측정을 같은 기기 기준으로 비교할 수 있다.

## 디스플레이

| 항목 | 값 |
| --- | --- |
| 연결된 모니터 | `\\.\DISPLAY1` 1440×2560 / `\\.\DISPLAY2` 2560×1080 (primary) / `\\.\DISPLAY3` 1920×1080 |
| HV-15 고객 모니터 | LG FULL HD (`GSM5B55`) 1920×1080, DPR 1, 60Hz |
| display profile | `1080p`, required source 1429×953 (HV-15 기준) |
| ICC | 별도 사용자 디스플레이 ICC 없음. 제품 출력은 sRGB / perceptual |
| HDR | 활성 증거 없음. sRGB/SDR 기준 |

이 회차는 화면 표시를 하지 않았으므로 모니터 항목은 **참고용**이다.
parity 측정의 목표 크기는 darktable이 실제로 만든 719×1080(세로 촬영 contain 결과)이다.

## 런타임

| 항목 | 값 |
| --- | --- |
| darktable | **5.4.1** (`C:\Program Files\darktable\bin\darktable-cli.exe`) |
| WebView2 Runtime | **151.0.4129.86** (HV-15 회차는 151.0.4129.78 — 이후 갱신됨) |
| Windows RAW decoder | `Microsoft Raw Image Decoder` |
| RAW decoder 패키지 | `Microsoft.RawImageExtension` **2.5.24.0** (x64, Microsoft Store appx) |
| 상주 엔진 | `webgl2-resident` **0.1.0-spike**, program hash `fnv1a64:bcbf55e37b6cc622` |
| resident recipe 계약 | `resident-recipe/v1` |
| 상주 lane mode | `off` (제품 기본값. 이 회차에서 켜지 않았다) |

## 카메라 / corpus

| 항목 | 값 |
| --- | --- |
| 카메라 | Canon EOS 700D — **이 회차에서는 연결하지 않았다** |
| corpus | HV-14 실촬영 CR2 35장 (`tests/hardware/capture-source/run-20260813-115426-hv14/session-evidence/captures/originals/`) |
| 센서 크기 | 5208×3476 (WIC 디코드 기준) |
| 렌즈 | Canon EF-S 18-55mm 번들렌즈 (HV-14/HV-15 회차 기록) |
| 노출 | **심한 저노출.** HV-15에서 평균 luma 12/255 수준으로 기록된 그 corpus다 |
| camera firmware | 미확인 (HV-15와 동일한 제한) |

## 알 수 없는 값 (추정하지 않는다)

- 부스 현장의 전원/열 조건에서의 WIC 디코드 p95 — 이 PC의 실내 조건 값만 있다
- 상주 WebGL2 엔진의 실제 GPU 실행 시간 — headless 환경에 WebGL2가 없어 측정 불가.
  실행하려면 부스 PC에서 앱을 띄워야 한다
- blind review 패널 5명 — 미확정. **AC 3 통과로 기록하지 않으며**, 입력 축 `Technology No-Go` 종료에는 비차단이고 후보 재활성화 시 필수다.
- `Microsoft.RawImageExtension`의 오프라인/기업 배포 가능 여부 — Story 7.7 범위

## 재현 방법

```bash
# darktable one-shot 기준선 (420회)
#   corpus × {preset-daylight, preset-mono-pop, preset-soft-glow, default-render-template} × 3
#   결과: baseline/darktable-oneshot-latency.csv

# WIC direct CR2 디코드 (72회)
#   PresentationCore BitmapDecoder, OnDemand, full / fit1620
#   결과: baseline/wic-cr2-decode.csv

# parity
BOOTHY_HV16_PARITY_MANIFEST=<manifest.json> \
BOOTHY_HV16_PARITY_OUTPUT=<result.json> \
npx vitest run tests/hardware/resident-renderer/hv-16/tools/hv16-parity.test.ts
```
