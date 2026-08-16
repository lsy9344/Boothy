/**
 * Story 7.5 / HV-16. parity 회차 실행기.
 *
 * **평소에는 건너뛴다.** `BOOTHY_HV16_PARITY_MANIFEST`가 가리키는 매니페스트가 있을 때만
 * 돈다. 지표 구현은 `src/quality-metrics/parity-metrics.ts` 하나뿐이고, 그 구현은
 * 공개 CIEDE2000 검증 표로 별도 단위 테스트를 통과한다 — 실행기가 자기만의 지표를
 * 따로 갖지 않는 것이 중요하다. 두 벌이 되면 evidence의 숫자와 회귀 테스트의 숫자가 갈린다.
 *
 * 실행:
 *   BOOTHY_HV16_PARITY_MANIFEST=<manifest.json> \
 *   BOOTHY_HV16_PARITY_OUTPUT=<result.json> \
 *   npx vitest run tests/hardware/resident-renderer/hv-16/tools/hv16-parity.test.ts
 *
 * 매니페스트 모양:
 *   {
 *     "runId": "...",
 *     "candidateLabel": "...",           // 이 픽셀을 만든 것
 *     "referenceLabel": "...",           // 무엇과 비교하는가
 *     "inputProvenance": "predecoded-fixture" | "real-capture-direct" | "real-capture-via-one-shot",
 *     "pairs": [{ "captureId": "...", "preset": "...", "candidatePpm": "...", "referencePpm": "...",
 *                 "skinRoi": { "x": 0, "y": 0, "widthPx": 0, "heightPx": 0 } | null }]
 *   }
 */

import { readFileSync, writeFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

import {
  decodePpm,
  measureParity,
  type Roi,
} from '../../../../../src/quality-metrics/parity-metrics'

type ParityPair = {
  captureId: string
  preset: string
  candidatePpm: string
  referencePpm: string
  skinRoi?: Roi | null
}

type ParityManifest = {
  runId: string
  candidateLabel: string
  referenceLabel: string
  inputProvenance: string
  pairs: ParityPair[]
}

const manifestPath = process.env.BOOTHY_HV16_PARITY_MANIFEST
const outputPath = process.env.BOOTHY_HV16_PARITY_OUTPUT

/**
 * 한 쌍은 1620×1080까지의 픽셀마다 CIEDE2000을 계산한다. 회차 하나가 수십 초 걸리는 것은
 * 정상이며, 기본 timeout에 맞추려고 표본을 줄이면 evidence의 분모가 줄어든다.
 */
const PARITY_RUN_TIMEOUT_MS = 30 * 60 * 1000

describe.skipIf(manifestPath === undefined)('HV-16 parity 회차', () => {
  it('measures every declared pair and writes the raw numbers', () => {
    const manifest = JSON.parse(
      readFileSync(manifestPath!, 'utf8'),
    ) as ParityManifest

    expect(manifest.pairs.length).toBeGreaterThan(0)

    const results = manifest.pairs.map((pair) => {
      const candidate = decodePpm(readFileSync(pair.candidatePpm))
      const reference = decodePpm(readFileSync(pair.referencePpm))

      if (candidate === null || reference === null) {
        // 읽지 못한 표본을 조용히 빼면 분모가 줄어들고 결과가 좋아 보인다.
        return {
          captureId: pair.captureId,
          preset: pair.preset,
          error: 'ppm-unreadable',
        }
      }

      return {
        captureId: pair.captureId,
        preset: pair.preset,
        measurement: measureParity(
          candidate,
          reference,
          pair.skinRoi ?? undefined,
        ),
      }
    })

    const report = {
      schemaVersion: 'hv-16-parity/v2',
      runId: manifest.runId,
      candidateLabel: manifest.candidateLabel,
      referenceLabel: manifest.referenceLabel,
      inputProvenance: manifest.inputProvenance,
      ssimDefinition:
        'pixel-weighted 8x8 block SSIM on Rec.709 luma, including partial edge blocks, K1=0.01 K2=0.03 L=1',
      deltaEDefinition: 'CIEDE2000 (kL=kC=kH=1) on per-pixel sRGB D65',
      clippingDefinition:
        'fraction of pixels where any sRGB channel is <=1/255 or >=254/255',
      declaredPairCount: manifest.pairs.length,
      measuredPairCount: results.filter(
        (row) => 'measurement' in row && row.measurement !== null,
      ).length,
      results,
    }

    if (outputPath !== undefined) {
      writeFileSync(outputPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8')
    }

    // 선언한 쌍이 전부 측정되어야 한다. 실패한 표본은 결과에 남지만 회차는 불완전하다.
    expect(report.measuredPairCount).toBe(report.declaredPairCount)
  }, PARITY_RUN_TIMEOUT_MS)
})
