import { spawnSync, type SpawnSyncReturns } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { basename, isAbsolute, join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

import {
  decodePpm,
  measureParity,
  mtf50FromSlantedEdge,
  type Roi,
} from '../../../../src/quality-metrics/parity-metrics.ts'
import {
  judgeTierJustification,
  type TierSample,
} from '../../../../src/quality-metrics/tier-justification.ts'

export type Hv17CorpusSample = {
  readonly sampleId: string
  readonly rawPath: string
  readonly edgeRoi: Roi
  /** The operator must explicitly confirm normal exposure; the runner never infers it. */
  readonly exposureConfirmed: boolean
}

export type Hv17Preset = {
  readonly presetId: string
  readonly xmpPath: string
}

export type Hv17TierManifest = {
  readonly schemaVersion: 'hv17-tier-run/v1'
  readonly evidenceRoot: string
  readonly darktableCli: string
  readonly targetWidthPx: number
  readonly targetHeightPx: number
  readonly jpegQuality: number
  readonly outputColorSpace: 'sRGB'
  readonly iccIntent: 'PERCEPTUAL'
  readonly samples: readonly Hv17CorpusSample[]
  readonly presets: readonly Hv17Preset[]
}

type CommandRecord = {
  readonly phase: 'render' | 'decode'
  readonly sampleId: string
  readonly presetId: string
  readonly tier: 'proxy' | 'refined'
  readonly binary: string
  readonly arguments: readonly string[]
  readonly exitCode: number | null
  readonly stdout: string
  readonly stderr: string
}

export type Hv17PairMeasurement = {
  readonly sample: TierSample
  readonly parity: ReturnType<typeof measureParity>
  readonly sourceRawPath: string
  readonly xmpPath: string
  readonly edgeRoi: Roi
}

function requirePositiveInteger(value: number, name: string): void {
  if (!Number.isInteger(value) || value <= 0) {
    throw new Error(`${name} must be a positive integer`)
  }
}

function requireEvidenceId(value: string, name: string): void {
  if (!/^[A-Za-z0-9][A-Za-z0-9_-]*$/.test(value)) {
    throw new Error(`${name} must contain only letters, digits, underscore, or hyphen`)
  }
}

export function validateTierManifest(manifest: Hv17TierManifest): void {
  if (manifest.schemaVersion !== 'hv17-tier-run/v1') {
    throw new Error('unsupported schemaVersion')
  }
  if (!isAbsolute(manifest.evidenceRoot) || !isAbsolute(manifest.darktableCli)) {
    throw new Error('evidenceRoot and darktableCli must be absolute paths')
  }
  if (!basename(resolve(manifest.evidenceRoot)).toLowerCase().includes('hv17')) {
    throw new Error('evidenceRoot final directory must identify an HV-17 run')
  }
  requirePositiveInteger(manifest.targetWidthPx, 'targetWidthPx')
  requirePositiveInteger(manifest.targetHeightPx, 'targetHeightPx')
  if (!Number.isInteger(manifest.jpegQuality) || manifest.jpegQuality < 1 || manifest.jpegQuality > 100) {
    throw new Error('jpegQuality must be an integer from 1 through 100')
  }
  if (manifest.outputColorSpace !== 'sRGB' || manifest.iccIntent !== 'PERCEPTUAL') {
    throw new Error('HV-17 production parity requires sRGB/PERCEPTUAL')
  }
  if (manifest.samples.length < 3 || new Set(manifest.samples.map((item) => item.sampleId)).size !== manifest.samples.length) {
    throw new Error('at least three uniquely named RAW samples are required')
  }
  if (manifest.presets.length < 3 || new Set(manifest.presets.map((item) => item.presetId)).size !== manifest.presets.length) {
    throw new Error('at least three uniquely named presets are required')
  }
  for (const sample of manifest.samples) {
    requireEvidenceId(sample.sampleId, 'sampleId')
    if (!isAbsolute(sample.rawPath) || !existsSync(sample.rawPath)) {
      throw new Error(`RAW does not exist: ${sample.sampleId}`)
    }
    if (typeof sample.exposureConfirmed !== 'boolean') {
      throw new Error(`${sample.sampleId}.exposureConfirmed must be a boolean`)
    }
    if (!Number.isInteger(sample.edgeRoi.x) || sample.edgeRoi.x < 0 || !Number.isInteger(sample.edgeRoi.y) || sample.edgeRoi.y < 0) {
      throw new Error(`${sample.sampleId}.edgeRoi origin must use non-negative integers`)
    }
    requirePositiveInteger(sample.edgeRoi.widthPx, `${sample.sampleId}.edgeRoi.widthPx`)
    requirePositiveInteger(sample.edgeRoi.heightPx, `${sample.sampleId}.edgeRoi.heightPx`)
  }
  for (const preset of manifest.presets) {
    requireEvidenceId(preset.presetId, 'presetId')
    if (!isAbsolute(preset.xmpPath) || !existsSync(preset.xmpPath)) {
      throw new Error(`XMP does not exist: ${preset.presetId}`)
    }
  }
  if (!existsSync(manifest.darktableCli)) {
    throw new Error(`darktable-cli does not exist: ${manifest.darktableCli}`)
  }
}

/** Mirrors build_display_proxy_invocation. The offline runner never advances a viewer pointer. */
export function buildProductionRenderArguments(
  manifest: Pick<Hv17TierManifest, 'targetWidthPx' | 'targetHeightPx' | 'jpegQuality'>,
  rawPath: string,
  xmpPath: string,
  outputPath: string,
  quality: 'proxy' | 'refined',
  workerRoot: string,
): string[] {
  return [
    rawPath.replaceAll('\\', '/'),
    xmpPath.replaceAll('\\', '/'),
    outputPath.replaceAll('\\', '/'),
    '--width', manifest.targetWidthPx.toString(),
    '--height', manifest.targetHeightPx.toString(),
    '--upscale', 'false',
    '--hq', quality === 'proxy' ? 'false' : 'true',
    '--apply-custom-presets', 'false',
    '--icc-type', 'SRGB',
    '--icc-intent', 'PERCEPTUAL',
    '--out-ext', 'jpg',
    '--core',
    '--configdir', join(workerRoot, 'config').replaceAll('\\', '/'),
    '--library', join(workerRoot, 'library.db').replaceAll('\\', '/'),
    '--conf', `plugins/imageio/format/jpeg/quality=${manifest.jpegQuality}`,
  ]
}

export function measurePairFromPpm(
  proxyBytes: Uint8Array,
  refinedBytes: Uint8Array,
  identity: Pick<Hv17CorpusSample, 'sampleId' | 'edgeRoi' | 'exposureConfirmed'> & Pick<Hv17Preset, 'presetId'>,
  sourceRawPath = '',
  xmpPath = '',
): Hv17PairMeasurement {
  const proxy = decodePpm(proxyBytes)
  const refined = decodePpm(refinedBytes)
  if (proxy === null || refined === null) {
    throw new Error(`${identity.sampleId}/${identity.presetId}: darktable PPM decode failed`)
  }
  const parity = measureParity(refined, proxy)
  const exposureUsable = identity.exposureConfirmed
  const sample: TierSample = {
    sampleId: identity.sampleId,
    presetId: identity.presetId,
    proxySize: { widthPx: proxy.widthPx, heightPx: proxy.heightPx },
    refinedSize: { widthPx: refined.widthPx, heightPx: refined.heightPx },
    proxyMtf50: exposureUsable ? mtf50FromSlantedEdge(proxy, identity.edgeRoi) : null,
    refinedMtf50: exposureUsable ? mtf50FromSlantedEdge(refined, identity.edgeRoi) : null,
    look: !exposureUsable || parity?.deltaE === null || parity === null
      ? null
      : {
          medianDeltaE: parity.deltaE.median,
          p95DeltaE: parity.deltaE.p95,
          clippingIncreasePercentagePoints: parity.clippingIncreasePercentagePoints,
        },
    underexposed: !identity.exposureConfirmed,
  }
  return { sample, parity, sourceRawPath, xmpPath, edgeRoi: identity.edgeRoi }
}

function runCommand(binary: string, args: string[]): SpawnSyncReturns<string> {
  return spawnSync(binary, args, { encoding: 'utf8', windowsHide: true })
}

function commandRecord(
  result: SpawnSyncReturns<string>,
  fields: Omit<CommandRecord, 'exitCode' | 'stdout' | 'stderr'>,
): CommandRecord {
  return {
    ...fields,
    exitCode: result.status,
    stdout: result.stdout ?? '',
    stderr: result.stderr ?? result.error?.message ?? '',
  }
}

function requireCommandSuccess(record: CommandRecord, tierRoot: string): void {
  if (record.exitCode !== 0) {
    writeFileSync(join(tierRoot, 'failure.json'), JSON.stringify({
      schemaVersion: 'hv17-tier-failure/v1',
      failedAt: new Date().toISOString(),
      failedCommand: record,
      customerPointerAdvanced: false,
    }, null, 2))
    throw new Error(`${record.sampleId}/${record.presetId}/${record.tier} ${record.phase} failed; see commands.jsonl`)
  }
}

export function runTierEvidence(manifest: Hv17TierManifest): ReturnType<typeof judgeTierJustification> {
  validateTierManifest(manifest)
  const evidenceRoot = resolve(manifest.evidenceRoot)
  const tierRoot = join(evidenceRoot, 'tier-justification')
  const pairsRoot = join(tierRoot, 'pairs')
  const sourcesRoot = join(tierRoot, 'sources')
  const presetsRoot = join(tierRoot, 'presets')
  const commandLogPath = join(tierRoot, 'commands.jsonl')
  if (existsSync(join(tierRoot, 'verdict.json'))) {
    throw new Error('tier evidence root already contains a verdict; use a fresh HV-17 run root')
  }
  mkdirSync(pairsRoot, { recursive: true })
  mkdirSync(sourcesRoot, { recursive: true })
  mkdirSync(presetsRoot, { recursive: true })
  writeFileSync(commandLogPath, '')

  const measurements: Hv17PairMeasurement[] = []
  const appendCommand = (record: CommandRecord): void => {
    writeFileSync(commandLogPath, `${JSON.stringify(record)}\n`, { flag: 'a' })
    requireCommandSuccess(record, tierRoot)
  }

  for (const sample of manifest.samples) {
    const evidenceRawPath = join(sourcesRoot, `${sample.sampleId}-${basename(sample.rawPath)}`)
    copyFileSync(sample.rawPath, evidenceRawPath)
    for (const preset of manifest.presets) {
      const pairRoot = join(pairsRoot, sample.sampleId, preset.presetId)
      mkdirSync(pairRoot, { recursive: true })
      const evidenceXmpPath = join(presetsRoot, `${preset.presetId}-${basename(preset.xmpPath)}`)
      if (!existsSync(evidenceXmpPath)) {
        copyFileSync(preset.xmpPath, evidenceXmpPath)
      }

      for (const tier of ['proxy', 'refined'] as const) {
        const jpgPath = join(pairRoot, `${tier}.jpg`)
        const ppmPath = join(pairRoot, `${tier}.ppm`)
        const workerRoot = join(evidenceRoot, '.darktable', tier)
        mkdirSync(workerRoot, { recursive: true })
        const renderArgs = buildProductionRenderArguments(
          manifest,
          evidenceRawPath,
          evidenceXmpPath,
          jpgPath,
          tier,
          workerRoot,
        )
        appendCommand(commandRecord(runCommand(manifest.darktableCli, renderArgs), {
          phase: 'render', sampleId: sample.sampleId, presetId: preset.presetId,
          tier, binary: manifest.darktableCli, arguments: renderArgs,
        }))

        const decodeRoot = join(evidenceRoot, '.darktable', 'ppm-decode', tier)
        mkdirSync(decodeRoot, { recursive: true })
        const decodeArgs = [
          jpgPath.replaceAll('\\', '/'), ppmPath.replaceAll('\\', '/'), '--out-ext', 'ppm',
          '--core', '--configdir', join(decodeRoot, 'config').replaceAll('\\', '/'),
          '--library', join(decodeRoot, 'library.db').replaceAll('\\', '/'),
        ]
        appendCommand(commandRecord(runCommand(manifest.darktableCli, decodeArgs), {
          phase: 'decode', sampleId: sample.sampleId, presetId: preset.presetId,
          tier, binary: manifest.darktableCli, arguments: decodeArgs,
        }))
      }

      const measurement = measurePairFromPpm(
        readFileSync(join(pairRoot, 'proxy.ppm')),
        readFileSync(join(pairRoot, 'refined.ppm')),
        { ...sample, presetId: preset.presetId },
        evidenceRawPath,
        evidenceXmpPath,
      )
      writeFileSync(join(pairRoot, 'measurement.json'), JSON.stringify(measurement, null, 2))
      measurements.push(measurement)
    }
  }

  const verdict = judgeTierJustification(measurements.map((entry) => entry.sample))
  writeFileSync(join(tierRoot, 'verdict.json'), JSON.stringify(verdict, null, 2))
  writeFileSync(join(tierRoot, 'run.json'), JSON.stringify({
    schemaVersion: manifest.schemaVersion,
    measuredAt: new Date().toISOString(),
    inputManifest: manifest,
    pairCount: measurements.length,
    exposureUnconfirmedSamples: manifest.samples
      .filter((sample) => !sample.exposureConfirmed)
      .map((sample) => sample.sampleId),
    verdictPath: 'tier-justification/verdict.json',
    customerPointerAdvanced: false,
  }, null, 2))
  return verdict
}

function main(): void {
  const manifestFlag = process.argv.indexOf('--manifest')
  if (manifestFlag < 0 || process.argv[manifestFlag + 1] === undefined) {
    throw new Error('usage: node --experimental-strip-types run-tier-evidence.ts --manifest <absolute-json-path>')
  }
  const manifestPath = resolve(process.argv[manifestFlag + 1])
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8')) as Hv17TierManifest
  const verdict = runTierEvidence(manifest)
  process.stdout.write(`${JSON.stringify(verdict, null, 2)}\n`)
}

if (process.argv[1] !== undefined && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try {
    main()
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`)
    process.exitCode = 1
  }
}
