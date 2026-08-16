# HV-14 Preflight — 2026-08-13

Status: **Ready for operator confirmation**

## 자동 확인 결과

| 항목 | 결과 |
| --- | --- |
| 점검 시각 | 2026-08-13T11:54:26+09:00 |
| 앱 commit | `c390f53` |
| 부스 PC | `NOAH_WIN` |
| OS | Windows 11 Pro 10.0.26200 (build 26200) |
| CPU / RAM | Intel Core i9-9900KF / 63.9 GB |
| GPU / driver | NVIDIA GeForce GTX 1080 / 32.0.15.6094 |
| 여유 저장 공간 | C: 572.6 GB |
| 카메라 | Canon EOS 700D, Windows 장치 상태 OK |
| USB 위치 | Port 6, USB Root Hub 3.0 (`PCIROOT(0)#PCI(1400)#USBROOT(0)#USB(6)`) |
| Canon EDSDK | 13.19.0, `C:\Code\cannon_sdk\canon-edsdk` |
| helper | 0.1.0 / protocol v2 |
| helper 자체 점검 | SDK 초기화 성공, 카메라 1대 발견, `camera-ready` |
| 제품 runtime root | `C:\Users\dltnd\Pictures\dabi_shoot` |
| 시작 전 source/display 측정 mode | 모두 unset |
| 시작 전 최신 세션 | `session_000000000018cb01461b43d49c` (2026-08-12 17:27) |

## 확정된 운용 조건

- 렌즈: Canon EF-S 18–55mm 번들렌즈, 세부 리비전 미상
- 메모리카드: 없음, USB를 통한 PC 직접 저장
- 전원: 기존 상시 전원, 어댑터/커플러 모델 미상
- Image Quality: 기존 RAW 제품 설정 유지
- 기존 카메라 → PC 전송 및 RAW → 필터 → JPEG 흐름은 정상 동작이 확인되어 이번 사전 검증에서 제외
- 펌웨어 버전은 미상으로 기록한다. Canon이 공개한 최신 1.1.5의 수정 대상은 현재 18–55mm 렌즈와 무관하므로 측정 차단 조건으로 삼지 않는다.

## 실행 직전 운영자 확인

- [ ] `scene-plan.md`의 네 장면을 준비했다.
- [ ] EOS Utility 등 카메라를 점유하는 다른 앱이 꺼져 있다.
- [ ] 고정 구도와 초점·노출·화이트밸런스 운용 방식을 확인했다.
- [ ] 새 세션에서 정확히 35회만 촬영할 준비가 됐다.

운영자가 위 항목을 확인한 경우에만 `start-measurement.ps1 -OperatorReady`를 실행한다.
