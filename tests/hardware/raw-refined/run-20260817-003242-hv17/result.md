# HV-17 실장비 실행 결과

## 판정

- 기계식 gate: **No-Go**
- 제품 판정: **Blocked / 재실행 필요**
- Story 7.6: `in-progress` 유지
- `BOOTHY_RAW_REFINED_MODE`: 제품 기본값 `off` 유지

## 실제로 확인된 것

- Canon EOS 700D를 EDSDK로 직접 제어했다.
- 시작 설정 RAW+JPEG에서는 JPEG만 전달되고 RAW가 30초 안에 도착하지 않았다.
  descriptor가 허용하는 RAW-only로 설정하고 readback까지 검증한 뒤 회차를 재개했다.
- 회복 후 실제 CR2 5회 촬영이 모두 저장됐고 proxy 5개가 모두 고객 모니터에 표시됐다.
- P0 탈락은 0건이었다. 큐 대기 p50/p95/max는 9/22/22 μs였고,
  종단 median 7,141,922 μs의 0.0001260165%였다.
- generation 5개와 terminal present 5개는 일대일로 완결됐다.

## HV-17A — No-Go

5-shot burst와 P0 무탈락, queue span, final 1건 완료는 확인했다. 하지만 필수인
delete/session-replaced/viewer-epoch-changed/newer-capture 네 취소 회차와 process-tree orphan 0,
raw taskkill 결과를 실행하지 못했다. P1 refined 작업도 진입하지 않아 용량·우선순위 전체 경로를
증명하지 못했다.

## HV-17B — Blocked / 기계식 No-Go

5개 generation은 모두 proxy였고 refined generation은 0개였다. tier 정당성은 구현상
`NotMeasured`인 동안 고객 화면 게시가 차단되며, 이번 실제 촬영도 거의 검은 장면이라
slanted-edge·정상 노출·3-preset proxy/refined 9쌍을 만들 수 없었다. 따라서 detail/look 축은
정직하게 `not-measured`이며 lane 기본 활성 자격은 없다.

실제 proxy→refined swap이 없으므로 일반 속도 전환 녹화, zero-defect 전환 보고,
3명 × 20회 판별 시험과 Noah Lee 서명은 생성하지 않았다.

## 재실행 조건

1. 정상 조명과 명확한 slanted-edge/해상도 차트로 EOS 700D 실제 촬영 3장 이상을 만든다.
2. 세 승인 preset 각각 proxy/refined pair를 생성해 detail/look 원자료 9쌍 이상을 측정한다.
3. 네 취소 사유를 각각 독립 실행하고 taskkill 원문과 orphan 0을 남긴다.
4. 실제 refined swap이 생긴 상태에서 일반 속도 녹화와 관찰자 3명 판별 시험을 수행한다.
5. 최종 승인자 Noah Lee가 evidence를 서명한다.

## 증거 경로

- 세션: `C:\Users\dltnd\Pictures\dabi_shoot\sessions\session_000000000018cc530d197ff02c`
- 회차: `tests/hardware/raw-refined/run-20260817-003242-hv17/`
- 사전 회복 회차: `tests/hardware/raw-refined/run-20260816-235448-hv17/`

