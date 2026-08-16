# HV-13B offscreen 직접 실장비 검증

## 판정

**offscreen 표시 경로는 부분 통과, HV-13B 전체는 `No-Go` 유지다.**

Canon EOS 700D로 실제 촬영 5회를 수행했다. 승인 `DISPLAY3`에서 숨김 사전 로딩된 viewer가 첫 generation에 노출됐고, 최종 sample B가 전체 화면으로 표시됐다. immutable A/B 파일 10개는 모두 무결성 검증을 통과했다.

다만 10개 generation 중 1개(`000008`, sample B)는 host commit 뒤 terminal viewer-present 행이 남지 않았다. 또한 120fps+ 외부 촬영 장비가 없어 물리 프레임 전환 결함을 판정할 수 없다.

## 결과

- 실제 셔터 입력 및 RAW 저장: 5/5
- Daylight 미리보기 완료: 5/5
- immutable generation 무결성: 10/10
- terminal actual-present 계측: 9/10
- 기록된 계측: 9/9 `presented` / `measured` / trusted input
- 입력 후 viewer 창 이벤트: 합계 0
- 승인 모니터 최종 sample B 전체 화면 표시: 확인
- 120fps+ 물리 프레임: 0

## 관찰

- 계측 latency p50/max는 4131.157/20118.957ms다. max는 연속 촬영 중 카메라 요청이 겹쳐 RAW 도착이 19.524초 지연된 요청이다.
- 미리보기 5건 모두 5초 예산을 초과했다(6804~24028ms). 이 성능 문제는 후속 fast-display 스토리 소유이며 임계값을 높이지 않았다.
- 이전에 수정한 사진 카드 잔류 문제는 이번 회차에서 재현되지 않았다. 5개 카드 모두 완료 미리보기를 표시했다.

## 증거

- `approved-display3-final-b.png`: 승인 모니터의 최종 sample B
- `booth-after-five-captures.png`: 실제 촬영 5회 완료 상태
- `session-evidence/session.json`: 촬영·렌더 결과
- `session-evidence/display/`: generation 10개와 pointer/journal
- `session-evidence/diagnostics/viewer-present.jsonl`: terminal actual-present 9행
- `integrity-check.json`: 파일 크기와 FNV-1a hash 검증
- `summary.json`: 집계와 누락 generation 식별

## 후속 조치 (2026-08-12, 관측값은 그대로 둔다)

이 회차의 관측 결과는 위 그대로이며 수정하지 않았다. `000008`의 terminal 행 누락은 원인까지
추적해 코드로 수정했고 회귀 테스트로 고정했다. **이 회차 이후의 수정이므로 아직 실장비에서
재확인되지 않았다.**

- 원인: swap이 커밋된 뒤 effect가 정리되면 after-paint 스탬프가 취소되어 보고 자체가 사라졌다.
  `cancelStamp`는 swap 커밋 뒤에만 만들어지므로 이 취소는 언제나 정당한 표본만 지웠다.
  5번째 셔터 입력이 `000008` commit보다 22ms 앞섰고, 같은 구간에서 hidden-prewarm 노출이
  generation마다 `set_position`·`show`·`set_fullscreen`을 다시 걸어 layout 재보고를 유발했다.
- 수정: 커밋된 교체의 스탬프는 언마운트에서만 취소한다. host는 terminal 보고가 없는 generation을
  `present-unreported` 행으로 닫아 **generation당 terminal 행이 정확히 하나**가 되게 한다.
  노출은 viewer 세대당 한 번만 수행한다.
- 다음 회차 게이트: `tests/hardware/viewer-present/hv-13b/check-telemetry-completeness.ps1`.
  이 증거 패키지에 돌리면 `completenessGate: fail` / `missing: ...-000008`로 같은 결론이 재현된다.

