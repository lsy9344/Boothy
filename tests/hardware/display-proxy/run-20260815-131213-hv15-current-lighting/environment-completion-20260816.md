# HV-15 환경 정보 보완

- 기록 일시: 2026-08-16 01:15 +09:00
- 승인자: Noah Lee

## 확인 정보

| 항목 | 값 | 근거 |
| --- | --- | --- |
| 카메라 | Canon EOS 700D | helper 및 실촬영 |
| 렌즈 | Canon EF-S 18–55mm 번들렌즈, 세부 리비전 미확인 | 사용자 제공 |
| 메모리카드 | 사용하지 않음, EDSDK SaveTo Host | HV-14 환경 및 세션 저장 경로 |
| 전원 | 연속 AC 전원, 어댑터/커플러 모델 미확인 | HV-14 환경 |
| USB | Port 6, 기존 작동 케이블, PC 직결, 허브 없음 | Windows USB 위치 및 HV-14 환경 |
| 모니터 | LG GSM5B55, 1920×1080, 60Hz, DPR 1 | Windows WMI 및 viewer profile |
| ICC | 별도 사용자 디스플레이 ICC 연결을 찾지 못함, 제품 출력 sRGB/perceptual | Windows ICM 조회 및 bundle output profile |
| HDR | 활성 상태 증거 없음, 이번 회차는 sRGB/SDR 운영 기준으로 해석 | Windows 조회 한계 및 제품 output profile |
| 펌웨어 | 정확 버전 미확인 | helper가 값을 보고하지 않음 |

펌웨어 정확 버전, 렌즈 세부 리비전, 전원 어댑터 모델, HDR 활성 상태는 사실값을 만들어내지 않고 사용자 승인 제한 예외로 처리한다.

## 화질 증거 처리

현재 밝기 예외는 `product-decision-20260816.md`에 기록했다. metrics/blind review는 수행한 것으로 만들지 않는다. Story 7.4의 현재 `raw-original + darktable` 운영 경로에 한해 제품 책임자 제한 예외를 적용하고, 정확 경로와 결과가 달라질 수 있는 Story 7.5 상주 렌더러 검증에서 다시 필수로 요구한다.

