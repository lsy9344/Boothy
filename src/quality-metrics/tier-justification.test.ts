import { describe, expect, it } from 'vitest'

import {
  judgeTierJustification,
  TIER_DETAIL_MIN_MEASURED_PAIRS,
  TIER_DETAIL_MIN_MTF50_RATIO,
  TIER_LOOK_MAX_CLIPPING_INCREASE_PP,
  TIER_LOOK_MAX_MEDIAN_DELTA_E,
  TIER_LOOK_MAX_P95_DELTA_E,
  type TierSample,
} from './tier-justification'

const SIZE = { widthPx: 1620, heightPx: 1080 }

function sample(overrides: Partial<TierSample> = {}): TierSample {
  return {
    sampleId: 'chart-1',
    presetId: 'preset_soft-glow',
    proxySize: SIZE,
    refinedSize: SIZE,
    proxyMtf50: 0.2,
    refinedMtf50: 0.25,
    look: {
      medianDeltaE: 1.1,
      p95DeltaE: 3.4,
      clippingIncreasePercentagePoints: 0.3,
    },
    ...overrides,
  }
}

/** 최소 corpus를 채운 회차. 해상도 차트 3장 × 승인 preset 3개. */
function fullCorpus(overrides: (index: number) => Partial<TierSample> = () => ({})) {
  const presets = ['preset_daylight', 'preset_mono-pop', 'preset_soft-glow']
  const charts = ['chart-1', 'chart-2', 'chart-3']

  return charts.flatMap((chart, chartIndex) =>
    presets.map((presetId, presetIndex) => {
      const index = chartIndex * presets.length + presetIndex

      return sample({
        sampleId: chart,
        presetId,
        ...overrides(index),
      })
    }),
  )
}

describe('judgeTierJustification', () => {
  it('pins the approved thresholds', () => {
    // 이 값들은 2026-08-16 Noah Lee 승인 항목이다. 조용히 완화되면 안 된다.
    expect(TIER_DETAIL_MIN_MTF50_RATIO).toBe(1.1)
    expect(TIER_LOOK_MAX_MEDIAN_DELTA_E).toBe(3)
    expect(TIER_LOOK_MAX_P95_DELTA_E).toBe(8)
    expect(TIER_LOOK_MAX_CLIPPING_INCREASE_PP).toBe(2)
    expect(TIER_DETAIL_MIN_MEASURED_PAIRS).toBe(9)
  })

  it('justifies the tier when detail clears 1.10x with no regression', () => {
    const report = judgeTierJustification(fullCorpus())

    expect(report.detail.verdict).toBe('justified')
    expect(report.detail.measuredPairs).toBe(9)
    expect(report.detail.ratio).toBeCloseTo(1.25, 5)
    expect(report.detail.regressions).toEqual([])
    expect(report.look.verdict).toBe('same-look')
    expect(report.laneDefaultEligible).toBe(true)
    expect(report.transitionDefect).toBeNull()
  })

  /**
   * **5% 개선을 위해 renderer 부하를 두 배로 만드는 것은 제품 결정으로 성립하지 않는다.**
   */
  it('refuses a tier whose detail gain sits below the approved minimum', () => {
    const report = judgeTierJustification(
      fullCorpus(() => ({ proxyMtf50: 0.2, refinedMtf50: 0.21 })),
    )

    expect(report.detail.ratio).toBeCloseTo(1.05, 5)
    expect(report.detail.verdict).toBe('tier-not-justified')
    expect(report.laneDefaultEligible).toBe(false)
    // tier가 없다는 것이지 전환 결함이 있다는 뜻은 아니다.
    expect(report.transitionDefect).toBeNull()
  })

  it('refuses a tier when any single sample walks backwards', () => {
    // median은 통과선을 넘지만 한 표본에서 정밀본이 더 흐리다.
    const report = judgeTierJustification(
      fullCorpus((index) =>
        index === 4
          ? { proxyMtf50: 0.3, refinedMtf50: 0.29 }
          : { proxyMtf50: 0.2, refinedMtf50: 0.26 },
      ),
    )

    expect(report.detail.ratio).not.toBeNull()
    expect(report.detail.ratio as number).toBeGreaterThanOrEqual(
      TIER_DETAIL_MIN_MTF50_RATIO,
    )
    expect(report.detail.regressions).toHaveLength(1)
    expect(report.detail.verdict).toBe('tier-not-justified')
  })

  /**
   * **Story 7.5에서 MTF50이 미실행으로 남은 이유가 corpus였다.**
   * 부족한 corpus로 얻은 좋은 숫자를 통과로 적으면 같은 실수를 반복한다.
   */
  it('reports not-measured instead of passing on a short corpus', () => {
    const report = judgeTierJustification(fullCorpus().slice(0, 8))

    expect(report.detail.measuredPairs).toBe(8)
    expect(report.detail.verdict).toBe('not-measured')
    expect(report.look.measuredPairs).toBe(8)
    expect(report.look.verdict).toBe('not-measured')
    expect(report.laneDefaultEligible).toBe(false)
  })

  it('requires a complete three-photo by three-preset matrix', () => {
    const duplicate = fullCorpus()
    duplicate[8] = duplicate[0]

    const duplicateReport = judgeTierJustification(duplicate)
    expect(duplicateReport.corpus.complete).toBe(false)
    expect(duplicateReport.corpus.duplicatePairs).toEqual([
      'chart-1/preset_daylight',
    ])
    expect(duplicateReport.detail.verdict).toBe('not-measured')
    expect(duplicateReport.look.verdict).toBe('not-measured')

    const onePresetReport = judgeTierJustification(
      Array.from({ length: 9 }, (_, index) =>
        sample({ sampleId: `chart-${index + 1}`, presetId: 'preset_daylight' }),
      ),
    )
    expect(onePresetReport.corpus.uniquePairs).toBe(9)
    expect(onePresetReport.corpus.uniquePresets).toBe(1)
    expect(onePresetReport.corpus.complete).toBe(false)
    expect(onePresetReport.laneDefaultEligible).toBe(false)
  })

  it('keeps unmeasurable samples in the report instead of dropping them', () => {
    const report = judgeTierJustification(
      fullCorpus((index) =>
        index === 0 ? { proxyMtf50: null, refinedMtf50: null } : {},
      ),
    )

    // 측정 불가 표본은 통과가 아니라 **미실행**으로 남는다.
    expect(report.detail.unmeasuredPairs).toContain('chart-1/preset_daylight')
    expect(report.detail.measuredPairs).toBe(8)
    expect(report.detail.verdict).toBe('not-measured')
  })

  it('does not pass by excluding an unmeasured pair from a larger corpus', () => {
    const fourthChart = [
      'preset_daylight',
      'preset_mono-pop',
      'preset_soft-glow',
    ].map((presetId, index) =>
      sample({
        sampleId: 'chart-4',
        presetId,
        ...(index === 0
          ? { proxyMtf50: null, refinedMtf50: null, look: null }
          : {}),
      }),
    )
    const report = judgeTierJustification([...fullCorpus(), ...fourthChart])

    expect(report.corpus.complete).toBe(true)
    expect(report.detail.measuredPairs).toBe(11)
    expect(report.look.measuredPairs).toBe(11)
    expect(report.detail.verdict).toBe('not-measured')
    expect(report.look.verdict).toBe('not-measured')
    expect(report.laneDefaultEligible).toBe(false)
  })

  it('treats non-finite and physically invalid metrics as unmeasured', () => {
    const report = judgeTierJustification(
      fullCorpus((index) =>
        index === 0
          ? {
              proxyMtf50: Number.NaN,
              refinedMtf50: 0,
              look: {
                medianDeltaE: -1,
                p95DeltaE: Number.POSITIVE_INFINITY,
                clippingIncreasePercentagePoints: 0,
              },
            }
          : {},
      ),
    )

    expect(report.detail.unmeasuredPairs).toContain('chart-1/preset_daylight')
    expect(report.look.unmeasuredPairs).toContain('chart-1/preset_daylight')
    expect(report.detail.verdict).toBe('not-measured')
    expect(report.look.verdict).toBe('not-measured')
  })

  /**
   * **전체 해상도 final을 기준으로 두 tier를 비교하지 않는다.**
   * 크기가 다르면 측정하려는 것이 아니라 리샘플러를 재게 된다.
   */
  it('excludes mismatched pixel sizes and says so out loud', () => {
    const report = judgeTierJustification([
      ...fullCorpus(),
      sample({
        sampleId: 'full-resolution-final',
        refinedSize: { widthPx: 5184, heightPx: 3456 },
      }),
    ])

    expect(report.dimensionMismatchedPairs).toEqual([
      'full-resolution-final/preset_soft-glow',
    ])
    expect(report.detail.unmeasuredPairs).toContain(
      'full-resolution-final/preset_soft-glow',
    )
    expect(report.detail.measuredPairs).toBe(9)
    expect(report.detail.verdict).toBe('not-measured')
    expect(report.look.verdict).toBe('not-measured')
  })

  /**
   * **look 축 이탈은 tier 문제가 아니라 AC 4의 전환 결함이다.**
   * 같은 XMP·같은 renderer로 색이 달라졌다면 `--hq` 외의 무언가가 바뀐 것이다.
   */
  it('records a look drift as a transition defect, not as an unjustified tier', () => {
    const report = judgeTierJustification(
      fullCorpus((index) =>
        index === 2
          ? {
              look: {
                medianDeltaE: 4.2,
                p95DeltaE: 9.5,
                clippingIncreasePercentagePoints: 0.1,
              },
            }
          : {},
      ),
    )

    expect(report.detail.verdict).toBe('justified')
    expect(report.look.verdict).toBe('refined-look-drift')
    expect(report.transitionDefect).toBe('refined-look-drift')
    // detail이 통과해도 전환 결함이 있으면 기본값을 켤 수 없다.
    expect(report.laneDefaultEligible).toBe(false)
    expect(report.look.offenders).toContainEqual(
      expect.objectContaining({
        sampleId: 'chart-1',
        presetId: 'preset_soft-glow',
      }),
    )
  })

  it('flags a clipping increase beyond the approved budget', () => {
    const report = judgeTierJustification(
      fullCorpus((index) =>
        index === 5
          ? {
              look: {
                medianDeltaE: 1.0,
                p95DeltaE: 2.0,
                clippingIncreasePercentagePoints: 2.6,
              },
            }
          : {},
      ),
    )

    expect(report.look.verdict).toBe('refined-look-drift')
    expect(report.look.maxClippingIncreasePercentagePoints).toBeCloseTo(2.6, 5)
  })

  /**
   * **Story 7.4의 저노출 제품 예외를 사용하지 않는다.**
   * 저노출 표본에서는 ΔE00이 디코더 차이가 아니라 노출 차이를 재게 된다.
   */
  it('does not exempt underexposed samples from the Story 7.6 look gate', () => {
    const report = judgeTierJustification(
      fullCorpus((index) =>
        index === 1
          ? {
              underexposed: true,
              look: {
                medianDeltaE: 12,
                p95DeltaE: 30,
                clippingIncreasePercentagePoints: 5,
              },
            }
          : {},
      ),
    )

    expect(report.underexposedExcludedPairs).toEqual([])
    expect(report.look.verdict).toBe('refined-look-drift')
    expect(report.laneDefaultEligible).toBe(false)
    // detail 축은 저노출과 무관하다. 그 표본은 여전히 센다.
    expect(report.detail.measuredPairs).toBe(9)
  })

  it('reports not-measured when the look axis has no usable pair', () => {
    const report = judgeTierJustification(fullCorpus(() => ({ look: null })))

    expect(report.look.verdict).toBe('not-measured')
    expect(report.look.unmeasuredPairs).toHaveLength(9)
    expect(report.laneDefaultEligible).toBe(false)
    // 미실행은 전환 결함이 아니다. 두 상태를 섞지 않는다.
    expect(report.transitionDefect).toBeNull()
  })

  it('keeps every per-sample value, not just the median', () => {
    const report = judgeTierJustification(fullCorpus())

    expect(report.detail.perSample).toHaveLength(9)
    expect(report.detail.perSample[0].ratio).toBeCloseTo(1.25, 5)
    expect(report.detail.perSample.every((entry) => entry.presetId.length > 0)).toBe(
      true,
    )
    // 세 승인 preset이 전부 측정됐는지 evidence에서 확인할 수 있어야 한다.
    expect(new Set(report.detail.perSample.map((entry) => entry.presetId)).size).toBe(
      3,
    )
  })
})
