import { writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

import {
  judgeTierJustification,
  type TierSample,
} from '../../../../../src/quality-metrics/tier-justification'

const captures = [
  'capture_20260816153458264_5d39fe705d',
  'capture_20260816154051179_3ad0e0ddca',
  'capture_20260816154435892_c488c861f0',
]
const presets = ['preset_daylight', 'preset_mono-pop', 'preset_soft-glow']

const samples: TierSample[] = captures.flatMap((sampleId) =>
  presets.map((presetId) => ({
    sampleId,
    presetId,
    proxySize: { widthPx: 634, heightPx: 953 },
    refinedSize: { widthPx: 0, heightPx: 0 },
    proxyMtf50: null,
    refinedMtf50: null,
    look: null,
    underexposed: true,
  })),
)

describe('HV-17B measured run verdict', () => {
  it('records the incomplete dark corpus as not-measured, never as a pass', () => {
    const verdict = judgeTierJustification(samples)
    const outputPath = resolve(
      process.cwd(),
      'tests/hardware/raw-refined/run-20260817-003242-hv17/tier-justification/verdict.json',
    )
    writeFileSync(outputPath, `${JSON.stringify(verdict, null, 2)}\n`, 'utf8')

    expect(verdict.corpus.complete).toBe(true)
    expect(verdict.detail.verdict).toBe('not-measured')
    expect(verdict.look.verdict).toBe('not-measured')
    expect(verdict.laneDefaultEligible).toBe(false)
  })
})
