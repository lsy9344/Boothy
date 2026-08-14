# HV-14 Evidence Package — Story 7.3

이 디렉터리는 Story 7.3 `LibRaw embedded JPEG와 RAW+JPEG fast source 비교`의 실장비 증거를
보존한다.

**현재 상태: `Not run`.** 자동 검증은 통과했지만 실장비 회차는 아직 수행되지 않았다.
HV-14가 **완전한 source 비교와 승인된 route 결정**을 기록하기 전까지 Story 7.3은 `review`에
머문다.

> HV-13A(Story 7.1)와 HV-13B(Story 7.2) 증거는 `tests/hardware/viewer-present/`에 있다.
> **섞지 않는다.**

## 수집 대상 (ledger required evidence)

route별 raw 표본 / capability descriptor / correlation 증거 / 품질 corpus / AB-BA 집계 /
route 결정 (primary · fallback · No-Go)

## 이 회차가 답해야 할 질문

1. 내장 JPEG는 **존재율·품질·비용** 면에서 쓸 만한가? (Route A)
   700D의 CR2가 실제로 full-size 내장 JPEG을 담고 있는지, orientation이 항상 1인지 포함한다.
2. 승인 카메라가 RAW+JPEG를 **실제로** 지원하는가? 지원한다면 JPEG가 RAW보다 **안정적으로**
   먼저 오는가? (Route B)
3. 위 둘이 **현재 제품의 incumbent보다 실제로 빠른가?** (Route C 기준선)

세 질문 중 어느 것이든 답이 "아니오"여도 조사 목적은 닫을 수 있다. **No-Go 후보를 production
경로로 승격하지 않는다** (Epic 7 execution rule).

## 사전 조건

| 항목 | 상태 |
| --- | --- |
| Story 7.1 / HV-13A `Go` | 2026-08-11 충족 |
| Story 7.2 / HV-13B 재정의 `Go` | 2026-08-12 충족 |
| 승인 EOS 700D + 승인 부스 PC | 회차 전 확인 |
| Canon EDSDK payload | `BOOTHY_CANON_SDK_ROOT` 또는 `sidecar/canon-helper/vendor/canon-edsdk/`. 기준 릴리스 EDSDK 13.19.0 (2025-02-28) |
| LibRaw 의존성 | **불필요.** 2026-08-12 결정으로 CR2 TIFF IFD를 직접 읽는다 |

> **세 route 모두 구현되어 있다.** 다만 어느 route든 표본이 0건이면 그 사실을 명시적으로
> 기록한다. 표본을 비워 두면 왜 비었는지 알 수 없다.

### 2026-08-14 No-Go 회차 이후 바뀐 것 (재실행 전 필독)

첫 실장비 회차(`run-20260813-115426-hv14`)는 No-Go였다. 원인 4종에 대한 조치가 코드에
반영되었으므로, 재실행은 아래 동작을 전제로 한다.

1. **회전된 촬영이 더 이상 `orientation-unsupported`로 전멸하지 않는다.** 부스 rig의
   EOS 700D가 만드는 orientation(6/8 등)은 표본에 **기록**되고 승격도 가능하다.
   display 승인(orientation 1만)은 바뀌지 않았다 — 정규화는 Story 7.4 소유다.
2. **Route C는 helper의 종단 이벤트를 기다렸다가 측정한다.** 기본 대기 15초
   (`BOOTHY_SOURCE_SHELL_WAIT_MS`로 조정). shell thumbnail이 이 예산 안에 도착하지
   않거나 chain이 다른 생산자로 끝나면 그때만 `absent`다. 제품의 120ms 예산은 그대로다.
3. **`camera-busy`·RAW handoff timeout 완화.** 셔터 DEVICE_BUSY는 최대 8×250ms 재시도,
   paired 활성 시 RAW handoff 예산 +15초(host/helper 동일), ImageQuality는 촬영마다
   되돌리지 않고 lane 종료 시점에 복원한다. **회차 종료 후 카메라의 Image Quality가
   원래 값으로 복원되었는지 `environment.md`의 before/after 항목으로 반드시 확인한다.**
4. **helper 단계에서 실패한 요청도 route마다 `cancelled` 행을 남긴다** (captureId 없음).
   실패 요청이 성공률 분모에서 사라지지 않으며, completeness 게이트가 이 행을 정상으로
   인정한다. qualifying 기준은 여전히 **성공·실패를 합쳐 35회 완결**이다.

### 셔터 작동 예산

`ab` 모드는 촬영 1회마다 세 route를 모두 기록한다.

- 세 route AB 비교: **셔터 35회** (warm-up 5 + 측정 30), telemetry 행은 총 105개
- 개별 route 모드: 해당 route에 대해 셔터 35회
- 실패한 촬영도 35회 분모에 포함한다. 운영 중단으로 회차를 다시 해야 하면 실패 회차를 보존하고
  **새 세션에서 35회 전체를 다시 시작한다**

회차 전에 카드 용량, 배터리/전원, 발열, 셔터 수명을 확인한다.

## 실행 방법

```powershell
# 측정 lane을 켠다. 기본은 off이며, off일 때 제품 경로는 지금과 완전히 동일하다.
$env:BOOTHY_SOURCE_COMPARE_MODE = 'ab'    # off | embedded | paired | shell | ab
$env:BOOTHY_CANON_SDK_ROOT = 'C:\Code\cannon_sdk\canon-edsdk'

# Story 7.2의 display lane은 반드시 꺼 둔다. 두 lane을 동시에 켜면 표본이 서로 오염된다.
Remove-Item Env:\BOOTHY_DISPLAY_SAMPLE_MODE -ErrorAction SilentlyContinue

pnpm dev:desktop
```

**측정이 끝나면 반드시 `BOOTHY_SOURCE_COMPARE_MODE`를 지운다.**

```powershell
Remove-Item Env:\BOOTHY_SOURCE_COMPARE_MODE
```

## 1. 환경 fingerprint — `environment.md`

- 승인 booth PC 모델, CPU/RAM/GPU/driver
- **EOS 700D 펌웨어 버전**, 렌즈, 카드 모델/용량/클래스
- USB 포트(3.0/2.0)와 케이블, 허브 유무
- EDSDK 버전, helper 버전 (Route A는 외부 라이브러리를 쓰지 않으므로 해당 버전 없음)
- 앱 버전, 커밋 해시
- `BOOTHY_SOURCE_COMPARE_MODE`
- **카메라 image quality 설정: 측정 전 값과 측정 후 복원된 값**

## 2. Route별 raw 표본 — `samples/`

route당 **warm-up 5회 + 측정 30회 이상.** 표본마다 다음을 기록한다.

| 항목 | 비고 |
| --- | --- |
| source 존재 여부 | 없으면 `absent`로 남긴다. 행을 비우지 않는다 |
| **실측 픽셀 크기** | 요청값이 아니라 실제 산출 크기. Route C의 `BiggerSizeOk` 때문에 필수다 |
| byte size | |
| EXIF orientation | `1`이 아니면 `orientation-unsupported` |
| decode 유효성 | SOI/SOF/EOI trailer |
| 손상 여부 | |
| request correlation | `requestId`, `captureId`, `groupId` |
| ready 시각 | host monotonic micros |
| **추출 비용** | cold / warm 구분 |

원본은 `<session_root>/diagnostics/source-comparison.jsonl`에 있다. 세션별로 복사해 둔다.

> **연구 실측(cold 약 50ms / warm 약 12ms)은 표본이 2개뿐이었다.** 기본 경로 확정 근거가
> 아니므로 30회 이상에서 다시 센다.

## 3. Capability descriptor 증거 — `capability/`

`capability/summary.json` (`hv-14-capability/v1`)에 측정 전/후 현재값, descriptor 원문,
`rawPlusJpegSupported`, 시도·미시도 조합과 이유를 구조화해 기록한다. run package의
`summary.template.json`을 복사해 사용한다.

- `EdsGetPropertyDesc(PropID_ImageQuality)` 지원 목록 **원문** (16진수 그대로)
- 현재값 (측정 전 / 측정 후)
- **시도한 조합과 시도하지 않은 조합, 그리고 각각의 이유**
- `rawPlusJpegSupported` 판정 결과

> descriptor에 RAW+JPEG가 없으면 그것이 곧 Route B의 결과다. **시도해서 실패한 것처럼
> 기록하지 않는다.** `unsupported-combination`과 `extraction-failed`는 다른 결과다.

## 4. Correlation 증거 — `correlation/`

`correlation/summary.json` (`hv-14-correlation/v1`)에 paired request 수, 거부·fallback 수,
originals JPEG 수, RAW truth 보존 여부와 근거 파일 목록을 기록한다.

- 모든 transfer object가 **하나의 request에 pair**됐음을 보이는 로그 구간
- **버려진 object가 0건**임을 보인다. 거부된 object가 있다면 사유(`group-mismatch`,
  `correlation-failed`, `duplicate-*`)와 함께 남긴다
- `groupId`가 0이어서 보조 correlation을 쓴 표본 수 (`usedFallbackCorrelation: true`)
- **JPEG object가 `captures/originals/`에 저장되지 않았음**을 보이는 디렉터리 목록
- **object 하나가 실패해도 RAW truth가 유지됐음**을 보이는 사례 (있다면)

## 5. 품질 corpus — `quality/`

대표 촬영의 route별 산출물을 **나란히 놓고 비교한 이미지 세트.**

- 노출 / 색 / 선명도 / artifact
- 사람이 승인한다. 자동 지표로 대체하지 않는다
- 밝은 장면, 어두운 장면, 인물, 고대비를 포함한다

`quality/manifest.json` (`hv-14-quality/v1`)에서 네 장면 각각의 세 route 이미지와 사람 승인자를
가리킨다. 참조 파일이 없거나 디렉터리 밖을 가리키면 최종 게이트가 실패한다.

## 6. AB/BA 집계 — `aggregate/`

`aggregate/summary.json` (`hv-14-aggregate/v1`)에 아래 route별 수치와 seed/block 수를 기록한다.

route별로:

- **p50 / p95 / max** (capture trigger → source ready)
- **성공률** (분모는 시도 횟수 전체)
- **source 도착 순서 분포** (Route B에서 JPEG가 RAW보다 먼저 온 비율)
- **실패 모드 분포** (거부 사유별 건수)

> **실패와 timeout을 제외하지 않는다.** 표본을 골라내면 성공률이 거짓이 된다.

`randomizationSeed`와 `blockIndex`를 함께 남겨 배치를 재현할 수 있게 한다.

## 7. 계측 완결성 — `completeness/`

`check-source-completeness.ps1`을 실행해 **35개 request 각각에 세 route 행이 정확히 하나씩**
남았는지, warm-up/측정 구분과 AB/BA 배치가 일관적인지 확인한다.

```powershell
.\check-source-completeness.ps1 `
  -SessionEvidenceDir '..\run-<timestamp>-hv14\session-evidence' `
  -ExpectedPerRoute 35 `
  -OutFile '..\run-<timestamp>-hv14\completeness\source-completeness.md'
```

이 단계의 PASS는 **telemetry completeness만** 뜻하며 HV-14 Go가 아니다. 검사 자체의 정상·결함
탐지 동작은 다음 합성 fixture로 언제든 재현한다.

```powershell
.\test-check-source-completeness.ps1
```

> Story 7.2는 commit된 generation 10개 중 1개에 terminal 행이 없어 한 회차를 잃었다.
> "표시되지 않음"과 "보고 유실"을 구분할 수 없었고 분모가 조용히 줄었다.
> 이 회차도 같은 눈으로 본다.

Story 7.2의 게이트도 함께 태운다.

```powershell
..\..\viewer-present\hv-13b\check-telemetry-completeness.ps1 `
  -SessionEvidenceDir '..\run-<timestamp>-hv14\session-evidence'
```

2026-08-12에 수정한 terminal 행 누락이 실장비에서 재발하지 않음을 함께 확인한다.

## 8. Route 결정 — `decision.md`

**primary route / fallback route / No-Go 중 하나를 명시적 증거와 함께 선택한다.**

결정문에 반드시 포함할 것:

- 선택한 route와 그 근거가 된 수치 (기준선 대비 개선폭)
- 탈락한 route와 탈락 이유
- 이 결정이 Story 7.4 display proxy에 주는 제약
- **두 후보가 모두 No-Go인 경우:** 대체 경로 결정(incumbent 유지 또는 7.5 resident renderer
  우선)을 명시적 증거와 함께 기록한다. 조사 목적은 닫을 수 있다

run package의 `decision.template.md` 필드명을 유지한다. 최종 게이트는 선택 유형뿐 아니라 선택
route, 기준선 개선폭, 탈락 route, Story 7.4 제약이 모두 확정됐는지 검사한다.

결정문과 필수 증거 디렉터리를 모두 채운 뒤 최종 패키지 게이트를 실행한다.

```powershell
.\check-source-completeness.ps1 `
  -SessionEvidenceDir '..\run-<timestamp>-hv14\session-evidence' `
  -ExpectedPerRoute 35 `
  -FinalPackageGate `
  -OutFile '..\run-<timestamp>-hv14\completeness\final-package-gate.md'
```

최종 패키지 PASS도 사람의 품질 승인과 ledger 갱신을 대신하지 않는다.

## 9. 마무리

- [ ] `BOOTHY_SOURCE_COMPARE_MODE` 환경 변수를 지웠다
- [ ] **카메라 image quality 설정을 원래 값으로 되돌렸고 그 사실을 `environment.md`에 기록했다**
- [ ] `renders/sources/` 산출물이 `renders/previews/`로 새어 나가지 않았음을 확인했다
- [ ] `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`의 Story 7.3 행을 갱신했다
- [ ] route 결정을 기록한 뒤에만 story status를 `done`으로 전환했다

> **자동 테스트 통과만으로 `done` 처리하지 않는다.** Story 1.9, 7.1(HV-13A), 7.2(HV-13B)가
> 모두 자동 검증 전부 통과 뒤 실장비에서 실패했다.
