# 2026-08-12 daylight capture / secondary-monitor spot check

## 판정

**보조 모니터 표시 경로는 Pass, 촬영 경험 전체는 Partial Pass다.**

Canon EOS 700D로 실제 촬영 2회를 수행했고 두 요청의 A→B가 승인 `DISPLAY3`에 정상 표시됐다.
다만 연속 촬영 후 첫 사진 카드가 완료된 미리보기를 갖고도 `마무리 중` 상태에 남는 UI 경합과
두 미리보기의 5초 예산 초과를 확인했다. 120fps+ 물리 프레임 장비를 사용하지 않았으므로
HV-13B 전체 판정은 기존 `No-Go`를 유지한다.

## 통과한 항목

- Canon EOS 700D 연결 및 실제 셔터 입력 2/2 성공
- CR2 원본 2/2 저장, 각각 약 19.75MB
- Daylight 미리보기 2/2 `previewReady`
- immutable display generation 4/4 존재·크기·FNV-1a hash 일치
- actual-present 4/4 `presented`, `measured`, trusted input
- `viewerWindowEventsAfterInput` 합계 0
- `Boothy Viewer`가 `DISPLAY3`의 `(640,1080) 1920×1080` 전체 영역을 사용
- 최종 sample B가 조작 요소 없이 보조 모니터에 표시됨

## 계측

| 촬영 | RAW 저장 | fast preview | XMP preview | A present | B present |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 2721ms | 3185ms | 7632ms | 3256.305ms | 4125.105ms |
| 2 | 2600ms | 2755ms | 7109ms | 3125.105ms | 3858.605ms |

두 XMP preview 모두 5000ms 예산을 초과했다.

## 새로 발견한 문제

1. 첫 촬영의 preview가 준비되기 전에 두 번째 촬영이 접수됐다. manifest와 파일은 두 촬영 모두
   `previewReady`지만, 첫 사진 카드는 수 분 뒤에도 `방금 찍은 사진을 현재 룩으로 마무리하고 있어요.`로 남았다.
   `booth-photo-rail.png`에 상태 불일치가 보존돼 있다.
2. 실제 preview 두 장은 저장·표시됐으나 육안상 매우 어둡다. 표시 경로 결함과는 별개로,
   고객 품질 검증 전 카메라 노출/구도 설정을 다시 확인해야 한다.

## 증거

- `display3-after-capture.png`: 실제 `DISPLAY3` 화면 캡처
- `booth-photo-rail.png`: 완료 파일과 남은 처리중 UI 상태
- `session-evidence/session.json`: 두 capture의 `previewReady` 상태와 timing
- `session-evidence/display/`: 4개 generation과 pointer/journal
- `session-evidence/diagnostics/viewer-present.jsonl`: actual-present 4행
- `session-evidence/previews/`: 실제 촬영 preview 2장


## 후속 조치 (2026-08-12)

### 1. 첫 사진 카드 `마무리 중` 잔류 — 수정 완료

**원인.** host readiness는 `latestCapture` 한 건만 실어 보낸다. 두 번째 촬영이 접수되는 순간
첫 촬영은 더 이상 latest가 아니므로, 그 뒤에 끝난 첫 촬영의 렌더 완료가 클라이언트에 도달할
경로가 사라진다. manifest와 파일은 `previewReady`였고 부스 카드 상태만 옛 값에 멈춰 있었다.
클라이언트에는 manifest를 다시 읽을 다른 경로가 없어 스스로 회복하지 못한다.

**수정.**

- host가 readiness에 `recentCaptures`를 함께 싣는다. 최근 8건과 **렌더가 끝나지 않은 기록 전부**이며,
  창 크기가 정확성을 좌우하지 않도록 진행 중인 기록은 창 밖이라도 항상 포함한다.
- 클라이언트는 이 목록을 manifest에 병합한다. 병합 전용이라 기록을 지우지 않고, 이미 준비된
  미리보기를 준비 전으로 되돌리지도 않는다. 삭제된 capture는 병합 대상에서 제외한다.
- 병합 순서는 `latestCapture` 먼저, 최근 목록이 마지막이다. 새 tick에 latest가 없으면 이전 tick의
  latest가 이어붙는데, 그 기록은 이번 tick의 목록보다 오래됐기 때문이다.

**회귀 테스트.**

- Rust: `readiness_reports_earlier_captures_so_a_displaced_card_can_finish`,
  `recent_captures_always_include_a_render_that_is_still_pending`
- TypeScript: `reconciles an earlier capture that finished rendering after a newer capture became latest`,
  `never rolls a finished capture back to rendering when a stale reconciliation tick arrives`
- 네 건 모두 수정을 되돌리면 실패하는 것을 확인했다.

**남은 확인.** 다음 실장비 회차에서 연속 2회 촬영 뒤 첫 사진 카드가 실제로 `마무리 중`을
벗어나는지 육안으로 확인해야 한다. 이 수정은 아직 실장비에서 검증되지 않았다.

### 2. 미리보기 5초 예산 초과 — 미해결, Epic 7 후속 스토리 소유

이번 회차 구간 분해:

| 촬영 | request → RAW 도착 | darktable-cli | 합계 |
| --- | ---: | ---: | ---: |
| 1 | 2721 ms | 4735 ms | 7632 ms |
| 2 | 2600 ms | 4326 ms | 7109 ms |

`timing-events.log`의 `preview-render-ready` 행이 darktable 구간(`elapsedMs`)을 직접 기록한다.
출력이 `--width 384 --height 384`인데도 4.3초를 넘으므로 지배 요인은 해상도가 아니라 매 촬영마다
새로 뜨는 프로세스의 기동과 RAW 디코드다. 이는 Story 7.3(LibRaw embedded JPEG),
7.4(display-fit proxy), 7.5(상주 display renderer), 7.6(deadline scheduler)이 소유한 문제이며
이번 수정 범위 밖이다. **예산 임계값을 늘려 통과시키지 않는다.** `previewBudgetState`는
`exceededBudget`을 그대로 보고한다.

### 3. 어두운 사진 — 카메라 설정

표시 경로 결함이 아니다. 표시 경로는 원본 픽셀을 그대로 올린다. 고객 품질 검증 전에 촬영
노출과 조명을 다시 잡아야 한다.
