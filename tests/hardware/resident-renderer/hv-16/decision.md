# HV-16 채택 결정 — 상주 display renderer

Story: 7.5 (Architecture Spike) · 결정일: 2026-08-16 · 기록자: Codex (operator: Noah Lee)

Verdict: Technology No-Go

## 판정

> ## `Technology No-Go`
>
> **production 채택하지 않는다.** `raw-original + pinned darktable 5.4.1` 정확 경로를 그대로 유지한다.
> 상주 후보는 기본 비활성(`off`)이며, 그 기본값은 이 결정으로 바뀌지 않는다.

Story 7.5의 Product Decision Rules 기준으로 `Technology No-Go` 조건 —
**입력 부재 + 성능 미달 + parity 미달**, 그리고 증거와 exact fallback의 완결성 — 을 모두 충족한다.

## 왜 No-Go인가 (근거 순서대로)

### 1. 1순위 후보 WebGL2는 실제 촬영 입력을 처리할 수 없다

WebView2의 WebGL2는 CR2를 디코드하지 못한다. 따라서 누군가 먼저 raster를 만들어 줘야 한다.
그런데 **그 raster를 만드는 비용이 preset까지 적용한 완성본을 만드는 비용과 같다.**

420회 실측(`baseline/darktable-oneshot-latency.csv`):

| | median |
| --- | ---: |
| preset 3종 적용 렌더 | 3716 ms |
| preset 연산 없는 중립 렌더 | 3669 ms |
| **차이 = preset 연산 전체 비용** | **47 ms (렌더의 1.26%)** |

상주 WebGL2가 preset 연산을 완벽하게, 무한히 빠르게 대신해도 **최대 47 ms**를 줄인다.
Story 7.4 실장비 종단 median 8284 ms 기준으로 8237 ms가 남는다.

이것은 엔진의 실패가 아니라 **입력 축의 실패**다. Story 7.3(HV-14)이 세 fast source를 닫은 뒤
승인된 빠른 raster 입력이 하나도 없다는 사실이 그대로 이어진 결과다.

### 2. 대안 후보(WIC/Direct2D)는 입력은 되지만 채택 조건을 못 채운다

측정 중 **Windows가 CR2를 in-process로 직접 디코드할 수 있다**는 사실을 확인했다
(`Microsoft Raw Image Decoder`, median **1284 ms**, `baseline/wic-cr2-decode.csv`).
이것은 이 spike에서 나온 가장 유용한 새 정보이며, HV-14의 세 후보와는 다른 성질의 경로다.

그럼에도 production `Go`가 될 수 없는 이유는 세 가지다.

| 축 | 사실 | 결과 |
| --- | --- | --- |
| **승인** | `Microsoft.RawImageExtension` 2.5.24.0은 Microsoft Store appx다. 오프라인 부스 installer에 동봉하는 경로가 확인되지 않았다 | 미승인 dependency. `RESIDENT_APPROVED_DIRECT_DECODERS`는 비어 있다 |
| **성능** | 디코드만으로 1284 ms. 8284 − (3716 − 1284) = **5855 ms** 잔여 | NFR-003 warm p50 3000 ms **미달** |
| **parity** | SSIM 0.013–0.159, ΔE00 median 22.96–26.56 (기준 SSIM ≥ 0.95, median ≤ 3) | **미달** |

### 3. 렌더러를 0으로 만들어도 NFR-003에 닿지 않는다

| 시나리오 | 잔여 지연 | 3000 ms 목표 |
| --- | ---: | --- |
| display 렌더가 완전히 0 | 4568 ms | 미달 |
| 상주 direct decoder (디코드만 남김) | 5855 ms | 미달 |
| preset 연산만 GPU로 이관 | 8237 ms | 미달 |

**남은 지연의 구간과 소유 Story** (승인 결정 #2가 요구한 지목):

| 구간 | 소유 Story |
| --- | --- |
| 카메라 → PC RAW 전송 (19 MB/장, HV-15에서 timeout 2회) | **Story 7.8** |
| 렌더 큐 대기 (in-flight 2 제한, 우선순위 없음) | **Story 7.6** |
| 게시 (stage → probe → pointer commit → journal) | **Story 7.6** (계약은 7.2 유지) |
| present (viewer decode → swap → actual-present) | **Story 7.8** / 물리 프레임은 HV-18B |

이 지목 없이 "렌더러가 빨라졌다"만 보고하면 Story가 닫히지 않는다는 조건을 충족한다.

## 무엇을 만들었고 무엇이 남는가

`No-Go`는 "아무것도 만들지 않았다"가 아니다. 아래는 이 결정과 함께 남는다.

**남는 것 (제품 코드, 기본 비활성):**

- `webgl2-resident` 0.1.0-spike 엔진 — context/program/출력 surface 사전 준비,
  hot-path 컴파일·process 시작 0건 검증, context loss·DPR 변경 재초기화, 전부 fallback으로 닫힘
- `resident-recipe/v1` 계약 — operation allowlist, mask/blend/modversion 거부, recipe hash drift 검출
- **AC 6의 기계적 gate** — fixture로 얻은 결과에 `productionEligible: true`를 붙이면
  host와 TS 계약 양쪽에서 게시 경계 앞에서 거부된다
- `viewer-display/v3` — 프레임을 만든 renderer와 비교 기준 renderer의 분리 기록 (v1/v2 읽기 유지)
- 검증된 parity 지표 도구 — CIEDE2000은 Sharma 공개 검증 표 17쌍으로 단위 테스트

**남는 위험 / 열린 항목:**

| 항목 | 상태 | 다음 |
| --- | --- | --- |
| blind review 패널 5명 | **미확정.** AC 3을 통과로 기록하지 않는다 | 후보 재활성화 시 실명 확정 후 실행 |
| MTF50 | 도구는 구현·검증했으나 HV-14 corpus에 적합한 slanted-edge 대상이 없다 | 후보 재활성화 시 대상 차트가 있는 회차에서 측정 |

이 미실행 항목들은 통과로 간주하지 않는다. 다만 2026-08-16 승인된 종료 예외에 따라, 실제 CR2를 처리할 승인된 상주 입력 경로가 없다는 독립적인 입력 축 `Technology No-Go`와 exact fallback 증거가 Story 7.5 종료를 결정한다.
| 상주 엔진의 실제 GPU 실행 시간 | 미측정 (headless에 WebGL2 없음) | 채택을 재검토할 때만 필요 |
| corpus 저노출 | HV-15와 같은 저노출 corpus. parity 절대값을 과장한다 | 정상 노출 corpus 확보 시 재측정 |
| `Microsoft.RawImageExtension` 오프라인 배포 | **판정 완료 (2026-08-17): `offline-distribution: not-possible`** | Story 7.7 이 닫았다 → `tests/hardware/installer/hv-18a/raw-image-extension-verdict.md` |

## exact fallback 확인

- production 경로는 변경되지 않았다. `BOOTHY_DISPLAY_PROXY_MODE` 기본값은 계속 `on`이고,
  `raw-original + pinned darktable 5.4.1`이 고객 화면을 만든다.
- 상주 lane 기본값은 `off`이며 알 수 없는 설정값도 `off`로 닫힌다.
- 상주 후보의 모든 거절 사유(15종)는 고유 코드를 가지며 결과가 같다 — **정확 darktable 경로로 내려간다.**
- Story 7.4의 384px preview/final 경로와 proxy lane은 회귀하지 않았다 (T7 검증 참조).

## 다음 단계

1. **Story 7.6**은 렌더러 교체가 아니라 **scheduler와 큐 대기**에서 시작한다.
   이 회차의 숫자는 그쪽에 남은 시간이 더 크다는 것을 보여 준다.
2. ~~**Story 7.7**이 `Microsoft.RawImageExtension`의 오프라인 배포 가능 여부를 판정하면,
   그때 direct decoder 경로를 **별도 승인** 안건으로 다시 올릴 수 있다.~~
   → **2026-08-17 판정 완료.** 세 문항(오프라인 설치 수단 / 재배포 권리 / 버전 고정)이 모두
   「아니오」여서 `offline-distribution: not-possible`로 닫혔다. 판정문은
   `tests/hardware/installer/hv-18a/raw-image-extension-verdict.md`.
   **`RESIDENT_APPROVED_DIRECT_DECODERS`는 비어 있는 채로 유지된다.**
   이 판정이 긍정이었더라도 채택은 아니었다 — 이 회차의 parity 미달이 독립적으로 막는다.
3. **Story 7.8**이 RAW 전송과 present 구간을 잡는다.
4. HV-18D(Story 7.10)는 이 `Technology No-Go`를 결손이 아니라 **완결된 판정**으로 인용할 수 있다.
   승인된 production route(`raw-original + pinned darktable 5.4.1`)가 살아 있기 때문이다.
