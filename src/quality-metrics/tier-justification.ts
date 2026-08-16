/**
 * Story 7.6 AC 6: `rawRefinedDisplay` tier의 정당성 판정.
 *
 * **측정은 `parity-metrics.ts`가 한다. 이 파일은 판정만 한다.** 지표 도구를 다시 만들지 않는다 —
 * CIEDE2000은 Sharma 검증표 17쌍으로 이미 단위 테스트되어 있다.
 *
 * ## 두 축은 서로 다른 질문에 답한다
 *
 * - **detail 축(MTF50)이 tier의 존재 조건이다.** `--hq true`가 실제로 사는 곳이 여기다.
 *   두 결과가 측정 한계 안에서 같으면 교체는 연출일 뿐이고, 고객에게 아무 가치가 없으면서
 *   darktable 부하만 두 배가 된다.
 * - **look 축(ΔE00·clipping)은 품질 기준이 아니라 AC 4의 전환 결함 판정이다.**
 *   두 tier는 **같은 룩**이어야 한다. 교체 순간 색이 바뀌면 그것은 "덜 좋은 tier"가 아니라
 *   **고객이 보는 전환 결함**이다.
 *
 * 한쪽 결과가 다른 쪽을 대신할 수 없다. 그래서 두 판정을 하나의 점수로 합치지 않는다.
 *
 * ## 미실행은 통과가 아니다
 *
 * 측정하지 못한 표본은 분모에서 조용히 빠지지 않는다. `unmeasuredPairs`로 그대로 남고,
 * 최소 corpus를 채우지 못하면 판정 자체가 `not-measured`다.
 */

/**
 * detail 축 통과선. `median MTF50(refined) ≥ 1.10 × median MTF50(proxy)`.
 *
 * 근거: 10%는 slanted-edge MTF50의 통상 측정 노이즈보다 확실히 크고,
 * **촬영마다 darktable 실행이 하나 더 늘어나는 비용을 정당화할 수 있는 최소선**이다.
 * 5% 개선을 위해 renderer 부하를 두 배로 만드는 것은 제품 결정으로 성립하지 않는다.
 */
export const TIER_DETAIL_MIN_MTF50_RATIO = 1.1

/** look 축 통과선. 두 tier는 같은 룩이어야 한다. */
export const TIER_LOOK_MAX_MEDIAN_DELTA_E = 3
export const TIER_LOOK_MAX_P95_DELTA_E = 8
export const TIER_LOOK_MAX_CLIPPING_INCREASE_PP = 2

/**
 * 최소 corpus. **해상도 차트 또는 명확한 slanted edge가 있는 실제 EOS 700D 촬영 3장 ×
 * 승인 preset 3개 = 9쌍.**
 *
 * Story 7.5에서 MTF50이 미실행으로 남은 이유는 HV-14 corpus 35장이 전부 세로 인물/책상
 * 장면이라 slanted-edge 대상이 없었기 때문이다. 같은 실수를 반복하지 않는다.
 */
export const TIER_DETAIL_MIN_MEASURED_PAIRS = 9

export type TierPairSize = {
  readonly widthPx: number
  readonly heightPx: number
}

/** 한 쌍의 look 축 측정 결과. `parity-metrics.ts`의 산출물을 그대로 옮긴 값이다. */
export type TierLookMeasurement = {
  readonly medianDeltaE: number
  readonly p95DeltaE: number
  readonly clippingIncreasePercentagePoints: number
}

/**
 * proxy(`--hq false`) ↔ refined(`--hq true`) 한 쌍.
 *
 * **`null`은 "측정하지 않았다"는 뜻이지 "통과"가 아니다.**
 */
export type TierSample = {
  readonly sampleId: string
  readonly presetId: string
  readonly proxySize: TierPairSize
  readonly refinedSize: TierPairSize
  /** slanted-edge ROI에서 잰 MTF50 (cycles/pixel). ROI가 없으면 `null`이다. */
  readonly proxyMtf50: number | null
  readonly refinedMtf50: number | null
  readonly look: TierLookMeasurement | null
  /**
   * 저노출 표본인가. **Story 7.4의 저노출 제품 예외를 여기서 쓰지 않는다.**
   * 저노출에서는 ΔE00이 디코더 차이가 아니라 노출 차이를 재게 된다 (Story 7.5가 겪었다).
   */
  readonly underexposed?: boolean
}

export type DetailVerdict = 'justified' | 'tier-not-justified' | 'not-measured'
export type LookVerdict = 'same-look' | 'refined-look-drift' | 'not-measured'

export type DetailAxisReport = {
  readonly verdict: DetailVerdict
  readonly measuredPairs: number
  readonly unmeasuredPairs: readonly string[]
  readonly medianProxyMtf50: number | null
  readonly medianRefinedMtf50: number | null
  readonly ratio: number | null
  /** `MTF50(refined) < MTF50(proxy)`인 표본. **역행은 0건이어야 한다.** */
  readonly regressions: readonly string[]
  /** 표본별 값. **median만 적지 않는다.** */
  readonly perSample: readonly {
    readonly sampleId: string
    readonly presetId: string
    readonly proxyMtf50: number | null
    readonly refinedMtf50: number | null
    readonly ratio: number | null
  }[]
}

export type LookAxisReport = {
  readonly verdict: LookVerdict
  readonly measuredPairs: number
  readonly unmeasuredPairs: readonly string[]
  readonly medianDeltaE: number | null
  readonly p95DeltaE: number | null
  readonly maxClippingIncreasePercentagePoints: number | null
  /** 통과선을 벗어난 표본과 그 이유. */
  readonly offenders: readonly {
    readonly sampleId: string
    readonly presetId: string
    readonly reason: string
  }[]
}

export type TierCorpusReport = {
  readonly uniqueSamples: number
  readonly uniquePresets: number
  readonly uniquePairs: number
  readonly duplicatePairs: readonly string[]
  readonly complete: boolean
}

export type TierJustificationReport = {
  readonly corpus: TierCorpusReport
  readonly detail: DetailAxisReport
  readonly look: LookAxisReport
  /**
   * **두 출력의 픽셀 크기가 다른 쌍.** 지표가 아니라 리샘플러를 재게 되므로 측정에서 뺀다.
   * 뺀 사실은 여기 남는다 — 조용히 사라지지 않는다.
   */
  readonly dimensionMismatchedPairs: readonly string[]
  /** 저노출이라 look 축 측정에서 제외한 표본. 제외 사실을 숨기지 않는다. */
  readonly underexposedExcludedPairs: readonly string[]
  /**
   * **lane 기본값을 `on`으로 바꿀 수 있는가.**
   *
   * 두 축의 **논리곱**이지 합쳐진 점수가 아니다. detail 축이 tier의 존재를 결정하고,
   * look 축은 AC 4의 전환 결함을 결정한다. `Go` 판정은 둘 다 통과할 때만 나온다.
   */
  readonly laneDefaultEligible: boolean
  /** look 축 이탈은 tier 문제가 아니라 AC 4 실패다. */
  readonly transitionDefect: 'refined-look-drift' | null
}

function median(values: readonly number[]): number | null {
  if (values.length === 0) {
    return null
  }

  const sorted = [...values].sort((left, right) => left - right)
  const middle = Math.floor(sorted.length / 2)

  return sorted.length % 2 === 0
    ? (sorted[middle - 1] + sorted[middle]) / 2
    : sorted[middle]
}

function sameSize(left: TierPairSize, right: TierPairSize): boolean {
  return (
    Number.isInteger(left.widthPx) &&
    Number.isInteger(left.heightPx) &&
    Number.isInteger(right.widthPx) &&
    Number.isInteger(right.heightPx) &&
    left.widthPx > 0 &&
    left.heightPx > 0 &&
    right.widthPx > 0 &&
    right.heightPx > 0 &&
    left.widthPx === right.widthPx &&
    left.heightPx === right.heightPx
  )
}

function pairId(sample: Pick<TierSample, 'sampleId' | 'presetId'>): string {
  return `${sample.sampleId}/${sample.presetId}`
}

function isPositiveFinite(value: number | null): value is number {
  return value !== null && Number.isFinite(value) && value > 0
}

function isValidLookMeasurement(
  value: TierLookMeasurement | null,
): value is TierLookMeasurement {
  return (
    value !== null &&
    Number.isFinite(value.medianDeltaE) &&
    value.medianDeltaE >= 0 &&
    Number.isFinite(value.p95DeltaE) &&
    value.p95DeltaE >= 0 &&
    Number.isFinite(value.clippingIncreasePercentagePoints)
  )
}

/**
 * 두 축을 판정한다. **표본을 제외하지 않는다** — 측정 불가는 `unmeasuredPairs`에 남는다.
 */
export function judgeTierJustification(
  samples: readonly TierSample[],
): TierJustificationReport {
  const sampleIds = new Set(samples.map((sample) => sample.sampleId))
  const presetIds = new Set(samples.map((sample) => sample.presetId))
  const pairCounts = new Map<string, number>()

  for (const sample of samples) {
    const id = pairId(sample)
    pairCounts.set(id, (pairCounts.get(id) ?? 0) + 1)
  }

  const duplicatePairs = [...pairCounts.entries()]
    .filter(([, count]) => count > 1)
    .map(([id]) => id)
  const corpusComplete =
    sampleIds.size >= 3 &&
    presetIds.size >= 3 &&
    pairCounts.size >= TIER_DETAIL_MIN_MEASURED_PAIRS &&
    pairCounts.size === samples.length &&
    pairCounts.size === sampleIds.size * presetIds.size
  const dimensionMismatchedPairs: string[] = []
  const comparablePairs: TierSample[] = []

  for (const sample of samples) {
    // 크기가 다르면 이 쌍은 `--hq`를 재는 것이 아니라 리샘플러를 재게 된다.
    if (sameSize(sample.proxySize, sample.refinedSize)) {
      comparablePairs.push(sample)
    } else {
      dimensionMismatchedPairs.push(pairId(sample))
    }
  }

  // ---- detail 축 ----------------------------------------------------------
  const detailPerSample = comparablePairs.map((sample) => ({
    sampleId: sample.sampleId,
    presetId: sample.presetId,
    proxyMtf50: sample.proxyMtf50,
    refinedMtf50: sample.refinedMtf50,
    ratio:
      isPositiveFinite(sample.proxyMtf50) &&
      isPositiveFinite(sample.refinedMtf50)
        ? sample.refinedMtf50 / sample.proxyMtf50
        : null,
  }))
  const detailMeasured = detailPerSample.filter(
    (entry) => entry.ratio !== null,
  )
  const detailUnmeasured = [
    ...detailPerSample
      .filter((entry) => entry.ratio === null)
      .map((entry) => pairId(entry)),
    ...dimensionMismatchedPairs,
  ]
  const medianProxyMtf50 = median(
    detailMeasured.map((entry) => entry.proxyMtf50 as number),
  )
  const medianRefinedMtf50 = median(
    detailMeasured.map((entry) => entry.refinedMtf50 as number),
  )
  const ratio =
    medianProxyMtf50 !== null &&
    medianRefinedMtf50 !== null &&
    medianProxyMtf50 > 0
      ? medianRefinedMtf50 / medianProxyMtf50
      : null
  // 역행 0건이 통과 조건이다. median이 좋아도 어떤 표본에서 더 흐려지면 안 된다.
  const regressions = detailMeasured
    .filter((entry) => (entry.refinedMtf50 as number) < (entry.proxyMtf50 as number))
    .map((entry) => pairId(entry))

  let detailVerdict: DetailVerdict

  if (
    !corpusComplete ||
    detailUnmeasured.length > 0 ||
    detailMeasured.length < TIER_DETAIL_MIN_MEASURED_PAIRS ||
    ratio === null
  ) {
    // 최소 corpus를 못 채웠다. **미실행은 통과도 실패도 아니다.**
    detailVerdict = 'not-measured'
  } else if (ratio >= TIER_DETAIL_MIN_MTF50_RATIO && regressions.length === 0) {
    detailVerdict = 'justified'
  } else {
    detailVerdict = 'tier-not-justified'
  }

  // ---- look 축 ------------------------------------------------------------
  // Story 7.6은 저노출 예외를 승인 규칙으로 사용하지 않는다. 모든 비교 가능 pair를 본다.
  const underexposedExcludedPairs: string[] = []
  const lookCandidates = comparablePairs
  const lookMeasured = lookCandidates.filter((sample) =>
    isValidLookMeasurement(sample.look),
  )
  const lookUnmeasured = [
    ...lookCandidates
      .filter((sample) => !isValidLookMeasurement(sample.look))
      .map((sample) => pairId(sample)),
    ...dimensionMismatchedPairs,
  ]
  const medianDeltaE = median(
    lookMeasured.map((sample) => (sample.look as TierLookMeasurement).medianDeltaE),
  )
  const p95DeltaE = median(
    lookMeasured.map((sample) => (sample.look as TierLookMeasurement).p95DeltaE),
  )
  const clippingIncreases = lookMeasured.map(
    (sample) =>
      (sample.look as TierLookMeasurement).clippingIncreasePercentagePoints,
  )
  const maxClippingIncreasePercentagePoints =
    clippingIncreases.length === 0 ? null : Math.max(...clippingIncreases)
  const offenders: { sampleId: string; presetId: string; reason: string }[] = []

  for (const sample of lookMeasured) {
    const look = sample.look as TierLookMeasurement

    if (look.medianDeltaE > TIER_LOOK_MAX_MEDIAN_DELTA_E) {
      offenders.push({
        sampleId: sample.sampleId,
        presetId: sample.presetId,
        reason: `median ΔE00 ${look.medianDeltaE.toFixed(2)} > ${TIER_LOOK_MAX_MEDIAN_DELTA_E}`,
      })
    }

    if (look.p95DeltaE > TIER_LOOK_MAX_P95_DELTA_E) {
      offenders.push({
        sampleId: sample.sampleId,
        presetId: sample.presetId,
        reason: `p95 ΔE00 ${look.p95DeltaE.toFixed(2)} > ${TIER_LOOK_MAX_P95_DELTA_E}`,
      })
    }

    if (
      look.clippingIncreasePercentagePoints >
      TIER_LOOK_MAX_CLIPPING_INCREASE_PP
    ) {
      offenders.push({
        sampleId: sample.sampleId,
        presetId: sample.presetId,
        reason: `clipping +${look.clippingIncreasePercentagePoints.toFixed(2)}%p > ${TIER_LOOK_MAX_CLIPPING_INCREASE_PP}%p`,
      })
    }
  }

  let lookVerdict: LookVerdict

  if (
    !corpusComplete ||
    lookUnmeasured.length > 0 ||
    lookMeasured.length < TIER_DETAIL_MIN_MEASURED_PAIRS ||
    medianDeltaE === null ||
    p95DeltaE === null
  ) {
    lookVerdict = 'not-measured'
  } else if (
    offenders.length === 0 &&
    medianDeltaE <= TIER_LOOK_MAX_MEDIAN_DELTA_E &&
    p95DeltaE <= TIER_LOOK_MAX_P95_DELTA_E
  ) {
    lookVerdict = 'same-look'
  } else {
    lookVerdict = 'refined-look-drift'
  }

  return {
    corpus: {
      uniqueSamples: sampleIds.size,
      uniquePresets: presetIds.size,
      uniquePairs: pairCounts.size,
      duplicatePairs,
      complete: corpusComplete,
    },
    detail: {
      verdict: detailVerdict,
      measuredPairs: detailMeasured.length,
      unmeasuredPairs: detailUnmeasured,
      medianProxyMtf50,
      medianRefinedMtf50,
      ratio,
      regressions,
      perSample: detailPerSample,
    },
    look: {
      verdict: lookVerdict,
      measuredPairs: lookMeasured.length,
      unmeasuredPairs: lookUnmeasured,
      medianDeltaE,
      p95DeltaE,
      maxClippingIncreasePercentagePoints,
      offenders,
    },
    dimensionMismatchedPairs,
    underexposedExcludedPairs,
    laneDefaultEligible:
      detailVerdict === 'justified' && lookVerdict === 'same-look',
    transitionDefect:
      lookVerdict === 'refined-look-drift' ? 'refined-look-drift' : null,
  }
}
