# HV-17 실장비 환경

실행: `2026-08-17 00:32 +09:00` · Story 7.6 RAW 정밀본 무중단 교체

## 회차 범위

Canon EOS 700D와 고객 모니터를 사용한 실제 촬영 세션이다. 120fps 물리 프레임,
compositor→photon 오프셋, frame 단위 판정은 HV-18B 범위라 측정하지 않았다.

## PC

| 항목 | 값 |
| --- | --- |
| OS | Windows 11 Pro 10.0.26200 build 26200, 64-bit |
| CPU | Intel Core i9-9900KF 3.60 GHz, 8 core / 16 thread |
| RAM | 63.92 GiB |
| GPU | NVIDIA GeForce GTX 1080, driver 32.0.15.6094, 2024-08-14 |
| 전원 구성표 | 균형 조정, `381b4222-f694-41f0-9685-ff5bb260df2e` |
| 전원 | unknown |

HV-15·HV-16을 실행한 동일 물리 workstation이다.

## 디스플레이 (고객 모니터)

| 항목 | 값 |
| --- | --- |
| 고객 모니터 | LG FULL HD, `DISPLAY\\GSM5B55`, serial 16843009 |
| 해상도 | 1920×1080 |
| DPR | 1.0 |
| 주사율 | 60 Hz |
| 승인 display profile | `1080p`, required source 1429×953 |
| ICC | 사용자 ICC 상태 unknown; 제품 출력 sRGB / perceptual |
| HDR | unknown |

## 런타임

| 항목 | 값 |
| --- | --- |
| darktable | 5.4.1, `C:\Program Files\darktable\bin\darktable-cli.exe` |
| WebView2 Runtime | 151.0.4129.86 |
| Windows RAW decoder | Microsoft Raw Image Extension 2.5.24.0 |
| 앱 버전 | Tauri 0.1.0 |
| 커밋 해시 | `c390f53e2079e9379cb36501517fa05e26beb41d` |

## 카메라

| 항목 | 값 |
| --- | --- |
| 모델 | Canon EOS 700D |
| 펌웨어 | unknown |
| 렌즈 | Canon EF-S 18-55mm 계열; 정확한 revision/초점거리/조리개 unknown |
| 카드 | 저장 대상 Host; 카드 사용 안 함 |
| 케이블 | PC USB port 6 직결; 규격/길이 unknown |
| 전원 | unknown |
| EDSDK | 13.19.0 |
| helper | canon-helper 0.1.0 |
| ImageQuality | 시작 시 RAW+JPEG `0x00640013`; 회복 후 RAW-only `0x0064FF0F`, readback 검증 완료 |

## Lane 설정

| 환경 변수 | 이 회차 값 | 제품 기본값 |
| --- | --- | --- |
| `BOOTHY_DISPLAY_PROXY_MODE` | `on` | `on` |
| `BOOTHY_RAW_REFINED_MODE` | `on` | `off` |
| `BOOTHY_DISPLAY_SAMPLE_MODE` | `off` | `off` |
| `BOOTHY_SOURCE_COMPARE_MODE` | `off` | `off` |
| `BOOTHY_RESIDENT_RENDERER_MODE` | `off` | `off` |

## tier 정당성 corpus

| 항목 | 값 |
| --- | --- |
| 실제 EOS 700D 촬영 수 | 5 |
| slanted-edge 촬영 수 | 0 |
| 대상 | 일반 부스 장면; 결과가 거의 검고 명확한 slanted edge 없음 |
| 측정 preset | Daylight로 실제 표시. 3-preset offline pair는 생성되지 않음 |
| 총 측정 쌍 수 | 0 |
| ROI 정의 | 없음 |
| 노출 | 저노출; look/detail 측정 불가 |

`tier-justification/verdict.json`은 첫 3개 capture × 3 preset의 요구 matrix를
`not-measured`로 기록한다. refined 산출물이나 측정치를 만들어내지 않았다.

## 탐지 검사 관찰자

관찰자 3명 × 20 시행과 Noah Lee 서명은 실행하지 않았다. 실제 refined swap이 없었고
관찰자도 확보되지 않았으므로 사람 판별 결과 파일은 생성하지 않았다.

## 남은 지연

| 항목 | 값 |
| --- | --- |
| 이 회차 종단 median | 7141.922 ms (qualifying 4 samples) |
| 큐 대기 p50 / p95 / max | 0.009 / 0.022 / 0.022 ms |
| 큐 대기가 종단에서 차지하는 비율 | 0.0001260165% |
| Story 7.8로 넘기는 잔여 구간 | queue 외 7141.913 ms. 이 회차만으로 RAW 전송·render·publish·present를 더 분해하지 않음 |

