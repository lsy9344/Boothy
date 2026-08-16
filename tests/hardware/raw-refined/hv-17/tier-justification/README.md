# HV-17B tier 정당성 원자료 (AC 6)

**측정은 `src/quality-metrics/parity-metrics.ts`가 하고, 판정은
`src/quality-metrics/tier-justification.ts`가 한다.** 지표 도구를 다시 만들지 않는다 —
CIEDE2000은 Sharma 검증표 17쌍으로 이미 단위 테스트되어 있다.

## 두 축은 서로 다른 질문에 답한다

| 축 | 질문 | 통과선 | 벗어나면 |
| --- | --- | --- | --- |
| **detail (MTF50)** | 정밀본이 실제로 더 선명한가? | `median MTF50(refined) ≥ 1.10 × median MTF50(proxy)` **그리고 역행 0건** | `tier-not-justified` — tier를 만들지 않고 lane 기본 `off` 유지 |
| **look (ΔE00 · clipping)** | 두 tier가 같은 룩인가? | `median ΔE00 ≤ 3`, `p95 ≤ 8`, clipping 증가 `≤ 2%p` | **`refined-look-drift` — tier 문제가 아니라 AC 4 전환 결함** |

**detail 축이 tier의 존재 조건이다.** `--hq true`가 실제로 사는 곳이 여기다.
두 결과가 측정 한계 안에서 같으면 교체는 연출일 뿐이고, 고객에게 아무 가치가 없으면서
darktable 부하만 두 배가 된다.

10%가 통과선인 근거: slanted-edge MTF50의 통상 측정 노이즈보다 확실히 크고,
**촬영마다 darktable 실행이 하나 더 늘어나는 비용을 정당화할 수 있는 최소선**이다.
5% 개선을 위해 renderer 부하를 두 배로 만드는 것은 제품 결정으로 성립하지 않는다.

look 축을 벗어났다면 그것은 "덜 좋은 tier"가 아니라 **고객이 보는 전환 결함**이다.
같은 XMP·같은 renderer로 색이 달라졌다면 `--hq` 외의 무언가가 바뀐 것이므로 원인을 찾는다.

## Story 7.4의 시각 승인 임계값을 여기 쓰지 않는다

Story 7.4의 `SSIM ≥ 0.95`, `median ΔE00 ≤ 3`은 **"정확 경로와 얼마나 같은가"**를 재는 값이다.
detail 축은 다른 질문이므로 그 임계값을 그대로 쓰지 않는다.

**전체 해상도 final을 기준으로 두 tier를 비교하지 않는다.** 5184×3456을 1429×953으로 내리는
리샘플러 선택이 결과를 좌우해, 측정하려는 것이 아니라 리샘플러를 재게 된다.
`judgeTierJustification()`이 크기가 다른 쌍을 `dimensionMismatchedPairs`로 빼고 그 사실을 남긴다.

## corpus 요구 (선행 조건)

**해상도 차트 또는 명확한 slanted edge가 있는 실제 EOS 700D 촬영 최소 3장**을 포함하고,
세 승인 preset 전부에 대해 측정한다 → **최소 9쌍**.

Story 7.5에서 MTF50이 미실행으로 남은 이유가 이것이다. HV-14 corpus 35장이 전부
세로 인물/책상 장면이라 slanted-edge 대상이 없었다.

**자연 사진에서 slanted edge를 자동 탐지하지 않는다.** 엉뚱한 영역을 재고 그 숫자가
evidence에 남는다. ROI는 사람이 지정하고 `pairs/<sampleId>/roi.json`에 적는다.

## 디렉터리 구조

```
tier-justification/
├── verdict.json                      # judgeTierJustification() 결과 그대로
└── pairs/
    └── <chart>/<presetId>/
        ├── proxy.ppm                 # --hq false 출력
        ├── refined.ppm               # --hq true 출력
        ├── roi.json                  # { "x":.., "y":.., "widthPx":.., "heightPx":.. }
        └── measurement.json          # parity-metrics 산출값 원본
```

**두 출력의 픽셀 크기가 같아야 한다.** 다르면 지표가 아니라 리샘플러를 재는 것이다.

## 저노출 표본

**Story 7.4의 저노출 제품 예외를 사용하지 않는다.** 노출이 정상인 표본으로 측정한다.
저노출 표본에서는 ΔE00이 디코더 차이가 아니라 노출 차이를 재게 된다 (Story 7.5가 같은 함정을 겪었다).

`TierSample.underexposed = true`로 표시한 표본은 look 축에서 제외되고, 제외 사실이
`underexposedExcludedPairs`에 남는다. **조용히 사라지지 않는다.**

## 실패 표본을 제외하지 않는다

측정 불가 항목은 통과가 아니라 **미실행**으로 적는다. `judgeTierJustification()`이
`unmeasuredPairs`에 남기고, 최소 corpus를 못 채우면 판정 자체가 `not-measured`다.
`check-raw-refined-evidence.ps1`은 `not-measured`를 통과로 받지 않는다.
