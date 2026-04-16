import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { afterEach, describe, expect, it } from 'vitest'

const createdRoots: string[] = []
const validPngBuffer = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQImWNgYGD4DwABBAEAgr2l3gAAAABJRU5ErkJggg==',
  'base64',
)

function createFixtureRepo() {
  const repoRoot = mkdtempSync(path.join(tmpdir(), 'boothy-hardware-scripts-'))
  createdRoots.push(repoRoot)

  const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
  const captureId = 'capture_20260410_001'
  const requestId = 'request_20260410_001'
  const presetId = 'preset_soft-glow'
  const publishedVersion = '2026.04.10'
  const sessionRoot = path.join(repoRoot, 'sessions', sessionId)

  mkdirSync(path.join(sessionRoot, 'diagnostics', 'dedicated-renderer'), {
    recursive: true,
  })
  mkdirSync(path.join(sessionRoot, 'diagnostics'), { recursive: true })
  mkdirSync(path.join(sessionRoot, 'renders', 'previews'), { recursive: true })
  mkdirSync(path.join(repoRoot, 'diagnostics'), { recursive: true })
  mkdirSync(path.join(repoRoot, 'branch-config'), { recursive: true })
  mkdirSync(
    path.join(
      repoRoot,
      'preset-catalog',
      'published',
      presetId,
      publishedVersion,
    ),
    { recursive: true },
  )

  const capturedRoutePolicySnapshotPath = path.join(
    sessionRoot,
    'diagnostics',
    'dedicated-renderer',
    `captured-preview-renderer-policy-${captureId}.json`,
  )
  const capturedCatalogSnapshotPath = path.join(
    sessionRoot,
    'diagnostics',
    'dedicated-renderer',
    `captured-catalog-state-${captureId}.json`,
  )

  writeFileSync(
    path.join(sessionRoot, 'session.json'),
    JSON.stringify(
      {
        schemaVersion: 'session-manifest/v1',
        sessionId,
        boothAlias: 'Kim 4821',
        customer: {
          name: 'Kim',
          phoneLastFour: '4821',
        },
        createdAt: '2026-04-12T00:00:00Z',
        updatedAt: '2026-04-12T00:00:15Z',
        lifecycle: {
          status: 'active',
          stage: 'preview-ready',
        },
        activePreset: {
          presetId,
          publishedVersion,
        },
        activePresetId: presetId,
        captures: [
          {
            schemaVersion: 'session-capture/v1',
            sessionId,
            boothAlias: 'Kim 4821',
            activePresetId: presetId,
            activePresetVersion: publishedVersion,
            activePresetDisplayName: 'Soft Glow',
            captureId,
            requestId,
            raw: {
              assetPath: path.join(sessionRoot, 'captures', 'originals', 'capture.cr3'),
              persistedAtMs: 100,
            },
            preview: {
              assetPath: path.join(
                sessionRoot,
                'renders',
                'previews',
                `${captureId}.jpg`,
              ),
              enqueuedAtMs: 150,
              readyAtMs: 900,
            },
            final: {
              assetPath: null,
              readyAtMs: null,
            },
            renderStatus: 'previewReady',
            postEndState: 'activeSession',
            timing: {
              captureAcknowledgedAtMs: 100,
              previewVisibleAtMs: 900,
              fastPreviewVisibleAtMs: 2810,
              xmpPreviewReadyAtMs: 6425,
              captureBudgetMs: 1000,
              previewBudgetMs: 2500,
              previewBudgetState: 'overBudget',
            },
          },
        ],
        postEnd: null,
      },
      null,
      2,
    ),
  )
  writeFileSync(
    path.join(sessionRoot, 'diagnostics', 'timing-events.log'),
    [
      '2026-04-12T08:00:00+09:00\tsession=' +
        sessionId +
        '\tcapture=none' +
        '\trequest=' +
        requestId +
        '\tevent=request-capture\tdetail=routeStage=canary',
      '2026-04-12T08:00:00.200+09:00\tsession=' +
        sessionId +
        '\tcapture=none' +
        '\trequest=' +
        requestId +
        '\tevent=capture-accepted\tdetail=detailCode=capture-in-flight',
      '2026-04-12T08:00:01+09:00\tsession=' +
        sessionId +
        '\tcapture=' +
        captureId +
        '\trequest=' +
        requestId +
        '\tevent=file-arrived\tdetail=rawPersistedAtMs=100',
      '2026-04-12T08:00:01.100+09:00\tsession=' +
        sessionId +
        '\tcapture=' +
        captureId +
        '\trequest=' +
        requestId +
        '\tevent=fast-preview-ready\tdetail=kind=camera-thumbnail',
      '2026-04-12T08:00:12+09:00\tsession=' +
        sessionId +
        '\tcapture=' +
        captureId +
        '\trequest=' +
        requestId +
        '\tevent=capture_preview_ready\tdetail=truthfulArtifactReadyAtMs=900',
      '2026-04-12T08:00:15+09:00\tsession=' +
        sessionId +
        '\tcapture=' +
        captureId +
        '\trequest=' +
        requestId +
        '\tevent=recent-session-visible\tdetail=visibleOwner=dedicated-renderer;visibleOwnerTransitionAtMs=2410',
      '2026-04-12T08:00:15+09:00\tsession=' +
        sessionId +
        '\tcapture=' +
        captureId +
        '\trequest=' +
        requestId +
        '\tevent=capture_preview_transition_summary\tdetail=laneOwner=dedicated-renderer;fallbackReason=none;routeStage=canary;warmState=warm-ready;firstVisibleMs=1605;replacementMs=2410;originalVisibleToPresetAppliedVisibleMs=805',
    ].join('\n'),
  )
  writeFileSync(
    path.join(repoRoot, 'diagnostics', 'operator-audit-log.json'),
    JSON.stringify(
      {
        schemaVersion: 'operator-audit-store/v1',
        entries: [],
      },
      null,
      2,
    ),
  )
  writeFileSync(
    path.join(
      sessionRoot,
      'diagnostics',
      'dedicated-renderer',
      'preview-promotion-evidence.jsonl',
    ),
    JSON.stringify({
      schemaVersion: 'preview-promotion-evidence-record/v1',
      observedAt: '2026-04-12T08:00:15+09:00',
      sessionId,
      requestId,
      captureId,
      presetId,
      publishedVersion,
      laneOwner: 'dedicated-renderer',
      fallbackReasonCode: null,
      routeStage: 'canary',
      warmState: 'warm-ready',
      captureRequestedAtMs: 100,
      rawPersistedAtMs: 100,
      truthfulArtifactReadyAtMs: 900,
      visibleOwner: 'dedicated-renderer',
      visibleOwnerTransitionAtMs: 2410,
      firstVisibleMs: 1605,
      sameCaptureFullScreenVisibleMs: 2410,
      replacementMs: 2410,
      originalVisibleToPresetAppliedVisibleMs: 805,
      sessionManifestPath: path.join(sessionRoot, 'session.json').replace(/\\/g, '/'),
      timingEventsPath: path
        .join(sessionRoot, 'diagnostics', 'timing-events.log')
        .replace(/\\/g, '/'),
      routePolicySnapshotPath: capturedRoutePolicySnapshotPath.replace(/\\/g, '/'),
      publishedBundlePath: path
        .join(
          repoRoot,
          'preset-catalog',
          'published',
          presetId,
          publishedVersion,
          'bundle.json',
        )
        .replace(/\\/g, '/'),
      catalogStatePath: capturedCatalogSnapshotPath.replace(/\\/g, '/'),
      previewAssetPath: path
        .join(sessionRoot, 'renders', 'previews', `${captureId}.jpg`)
        .replace(/\\/g, '/'),
      warmStateDetailPath: path
        .join(
          sessionRoot,
          'diagnostics',
          'dedicated-renderer',
          'warm-state-preset_soft-glow-2026.04.10.json',
        )
        .replace(/\\/g, '/'),
    }),
  )
  writeFileSync(
    path.join(repoRoot, 'branch-config', 'preview-renderer-policy.json'),
    JSON.stringify({ schemaVersion: 'preview-renderer-route-policy/v1' }, null, 2),
  )
  writeFileSync(
    capturedRoutePolicySnapshotPath,
    JSON.stringify(
      { schemaVersion: 'preview-renderer-route-policy/v1', capturedAt: 'capture-time' },
      null,
      2,
    ),
  )
  writeFileSync(
    path.join(
      repoRoot,
      'preset-catalog',
      'published',
      presetId,
      publishedVersion,
      'bundle.json',
    ),
    JSON.stringify({ schemaVersion: 'published-preset-bundle/v2' }, null, 2),
  )
  writeFileSync(
    path.join(repoRoot, 'preset-catalog', 'catalog-state.json'),
    JSON.stringify({ schemaVersion: 'preset-catalog-state/v1' }, null, 2),
  )
  writeFileSync(
    capturedCatalogSnapshotPath,
    JSON.stringify(
      { schemaVersion: 'preset-catalog-state/v1', capturedAt: true },
      null,
      2,
    ),
  )
  writeFileSync(
    path.join(sessionRoot, 'renders', 'previews', `${captureId}.jpg`),
    Buffer.from([0xff, 0xd8, 0xff, 0xd9]),
  )

  const oracleMetadataPath = path.join(
    sessionRoot,
    'diagnostics',
    'dedicated-renderer',
    `${captureId}-oracle.json`,
  )
  writeFileSync(
    oracleMetadataPath,
    JSON.stringify(
      {
        sessionId,
        captureId,
        presetId,
        publishedVersion,
      },
      null,
      2,
    ),
  )

  const boothVisualPath = path.join(sessionRoot, 'diagnostics', 'booth-visual.png')
  const operatorVisualPath = path.join(sessionRoot, 'diagnostics', 'operator-visual.png')
  const rollbackEvidencePath = path.join(sessionRoot, 'diagnostics', 'rollback-proof.txt')
  writeFileSync(boothVisualPath, 'booth visual placeholder')
  writeFileSync(operatorVisualPath, 'operator visual placeholder')
  writeFileSync(rollbackEvidencePath, 'rollback proof placeholder')

  return {
    repoRoot,
    sessionId,
    captureId,
    requestId,
    presetId,
    publishedVersion,
    oracleMetadataPath,
    boothVisualPath,
    operatorVisualPath,
    rollbackEvidencePath,
  }
}

function runPowershell(scriptPath: string, args: string[]) {
  const result = spawnSync(
    'powershell',
    ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', scriptPath, ...args],
    {
      encoding: 'utf8',
    },
  )

  if (result.status !== 0) {
    throw new Error(result.stderr || result.stdout || 'powershell script failed')
  }

  return JSON.parse(result.stdout)
}

function readJsonFile(filePath: string) {
  return JSON.parse(readFileSync(filePath, 'utf8').replace(/^\uFEFF/, '')) as Record<
    string,
    unknown
  >
}

function writeValidTestRaster(filePath: string) {
  writeFileSync(filePath, validPngBuffer)
}

function buildCanaryBundle(
  fixture: ReturnType<typeof createFixtureRepo>,
  extraArgs: string[] = [],
) {
  const outputRoot = path.join(fixture.repoRoot, 'artifacts', 'canary-bundle')
  return runPowershell(path.resolve('scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1'), [
    '-RepoRoot',
    fixture.repoRoot,
    '-SessionId',
    fixture.sessionId,
    '-CaptureId',
    fixture.captureId,
    '-PresetId',
    fixture.presetId,
    '-PublishedVersion',
    fixture.publishedVersion,
    '-BoothVisualEvidencePaths',
    fixture.boothVisualPath,
    '-OperatorVisualEvidencePaths',
    fixture.operatorVisualPath,
    '-RollbackEvidencePaths',
    fixture.rollbackEvidencePath,
    '-OutputRoot',
    outputRoot,
    ...extraArgs,
    '-EmitJson',
  ])
}

afterEach(() => {
  for (const root of createdRoots.splice(0)) {
    rmSync(root, { recursive: true, force: true })
  }
})

describe('hardware evidence scripts', () => {
  it(
    'builds a deterministic dry-run trace start plan',
    () => {
    const fixture = createFixtureRepo()
    const result = runPowershell(
      path.resolve('scripts/hardware/Start-PreviewPromotionTrace.ps1'),
      [
        '-RepoRoot',
        fixture.repoRoot,
        '-SessionId',
        fixture.sessionId,
        '-PresetId',
        fixture.presetId,
        '-PublishedVersion',
        fixture.publishedVersion,
        '-CaptureId',
        fixture.captureId,
        '-DryRun',
        '-EmitJson',
      ],
    )

    expect(result.schemaVersion).toBe('preview-promotion-trace-plan/v1')
    expect(result.mode).toBe('dry-run')
    expect(result.traces.wprTracePath).toContain('preview-promotion.etl')
    expect(result.traces.pixTimingCapturePath).toContain(
      'preview-promotion-timing.wpix',
    )
    },
    40000,
  )

  it(
    'builds a dry-run evidence bundle from the structured promotion record',
    () => {
    const fixture = createFixtureRepo()
    const result = runPowershell(
      path.resolve('scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1'),
      [
        '-RepoRoot',
        fixture.repoRoot,
        '-SessionId',
        fixture.sessionId,
        '-CaptureId',
        fixture.captureId,
        '-PresetId',
        fixture.presetId,
        '-PublishedVersion',
        fixture.publishedVersion,
        '-BoothVisualEvidencePaths',
        fixture.boothVisualPath,
        '-OperatorVisualEvidencePaths',
        fixture.operatorVisualPath,
        '-RollbackEvidencePaths',
        fixture.rollbackEvidencePath,
        '-DryRun',
        '-EmitJson',
      ],
    )

    expect(result.schemaVersion).toBe('preview-promotion-evidence-bundle/v1')
    expect(result.laneOwner).toBe('dedicated-renderer')
    expect(result.routeStage).toBe('canary')
    expect(result.visibleOwner).toBe('dedicated-renderer')
    expect(result.visibleOwnerTransitionAtMs).toBe(2410)
    expect(result.captureRequestedAtMs).toBe(100)
    expect(result.rawPersistedAtMs).toBe(100)
    expect(result.truthfulArtifactReadyAtMs).toBe(900)
    expect(result.sameCaptureFullScreenVisibleMs).toBe(2410)
    expect(result.replacementMs).toBe(2410)
    expect(result.fallbackRatio).toBe(0)
    expect(result.artifacts.sessionManifest.source).toContain('session.json')
    expect(result.artifacts.routePolicySnapshot.source).toContain(
      `captured-preview-renderer-policy-${fixture.captureId}.json`,
    )
    expect(result.parity.result).toBe('not-run')
    expect(result.missingArtifacts).toEqual([])
    },
    40000,
  )

  it(
    'builds a deterministic dry-run trace stop plan',
    () => {
    const fixture = createFixtureRepo()
    const result = runPowershell(
      path.resolve('scripts/hardware/Stop-PreviewPromotionTrace.ps1'),
      [
        '-RepoRoot',
        fixture.repoRoot,
        '-SessionId',
        fixture.sessionId,
        '-PresetId',
        fixture.presetId,
        '-PublishedVersion',
        fixture.publishedVersion,
        '-CaptureId',
        fixture.captureId,
        '-DryRun',
        '-EmitJson',
      ],
    )

    expect(result.schemaVersion).toBe('preview-promotion-trace-summary/v1')
    expect(result.mode).toBe('dry-run')
    expect(result.traces.wprTracePath).toContain('preview-promotion.etl')
    expect(result.commands.stopWpr === null || result.commands.stopWpr).toBeTruthy()
    },
    40000,
  )

  it(
    'uses the same default trace root for dry-run start and stop plans',
    () => {
      const fixture = createFixtureRepo()
      const start = runPowershell(path.resolve('scripts/hardware/Start-PreviewPromotionTrace.ps1'), [
        '-RepoRoot',
        fixture.repoRoot,
        '-SessionId',
        fixture.sessionId,
        '-PresetId',
        fixture.presetId,
        '-PublishedVersion',
        fixture.publishedVersion,
        '-CaptureId',
        fixture.captureId,
        '-DryRun',
        '-EmitJson',
      ])
      const stop = runPowershell(path.resolve('scripts/hardware/Stop-PreviewPromotionTrace.ps1'), [
        '-RepoRoot',
        fixture.repoRoot,
        '-SessionId',
        fixture.sessionId,
        '-PresetId',
        fixture.presetId,
        '-PublishedVersion',
        fixture.publishedVersion,
        '-CaptureId',
        fixture.captureId,
        '-DryRun',
        '-EmitJson',
      ])

      expect(start.traceRoot).toBe(stop.traceRoot)
      expect(start.traces.wprTracePath).toBe(stop.traces.wprTracePath)
    },
    40000,
  )

  it(
    'does not synthesize same-capture full-screen timing from legacy replacement timing',
    () => {
      const fixture = createFixtureRepo()
      const evidenceLogPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'diagnostics',
        'dedicated-renderer',
        'preview-promotion-evidence.jsonl',
      )
      const record = JSON.parse(readFileSync(evidenceLogPath, 'utf8')) as Record<string, unknown>
      delete record.sameCaptureFullScreenVisibleMs
      record.replacementMs = 2410
      writeFileSync(evidenceLogPath, `${JSON.stringify(record)}\n`)

      const result = runPowershell(
        path.resolve('scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1'),
        [
          '-RepoRoot',
          fixture.repoRoot,
          '-SessionId',
          fixture.sessionId,
          '-CaptureId',
          fixture.captureId,
          '-PresetId',
          fixture.presetId,
          '-PublishedVersion',
          fixture.publishedVersion,
          '-BoothVisualEvidencePaths',
          fixture.boothVisualPath,
          '-OperatorVisualEvidencePaths',
          fixture.operatorVisualPath,
          '-RollbackEvidencePaths',
          fixture.rollbackEvidencePath,
          '-DryRun',
          '-EmitJson',
        ],
      )

      expect(result.sameCaptureFullScreenVisibleMs).toBeNull()
      expect(result.replacementMs).toBe(2410)
    },
    40000,
  )

  it(
    'fails closed when the canonical preview-promotion evidence record is missing',
    () => {
      const fixture = createFixtureRepo()
      rmSync(
        path.join(
          fixture.repoRoot,
          'sessions',
          fixture.sessionId,
          'diagnostics',
          'dedicated-renderer',
          'preview-promotion-evidence.jsonl',
        ),
      )

      expect(() =>
        runPowershell(path.resolve('scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1'), [
          '-RepoRoot',
          fixture.repoRoot,
          '-SessionId',
          fixture.sessionId,
          '-CaptureId',
          fixture.captureId,
          '-PresetId',
          fixture.presetId,
          '-PublishedVersion',
          fixture.publishedVersion,
          '-BoothVisualEvidencePaths',
          fixture.boothVisualPath,
          '-OperatorVisualEvidencePaths',
          fixture.operatorVisualPath,
          '-RollbackEvidencePaths',
          fixture.rollbackEvidencePath,
          '-DryRun',
          '-EmitJson',
        ]),
      ).toThrow(/preview promotion evidence/i)
    },
    40000,
  )

  it(
    'rejects parity oracle metadata that does not match the requested capture correlation',
    () => {
      const fixture = createFixtureRepo()
      const mismatchedMetadataPath = path.join(fixture.repoRoot, 'mismatched-oracle.json')
      writeFileSync(
        mismatchedMetadataPath,
        JSON.stringify(
          {
            sessionId: fixture.sessionId,
            captureId: 'capture_other',
            presetId: fixture.presetId,
            publishedVersion: fixture.publishedVersion,
          },
          null,
          2,
        ),
      )

      expect(() =>
        runPowershell(path.resolve('scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1'), [
          '-RepoRoot',
          fixture.repoRoot,
          '-SessionId',
          fixture.sessionId,
          '-CaptureId',
          fixture.captureId,
          '-PresetId',
          fixture.presetId,
          '-PublishedVersion',
          fixture.publishedVersion,
          '-BoothVisualEvidencePaths',
          fixture.boothVisualPath,
          '-OperatorVisualEvidencePaths',
          fixture.operatorVisualPath,
          '-RollbackEvidencePaths',
          fixture.rollbackEvidencePath,
          '-BaselineImagePath',
          path.join(
            fixture.repoRoot,
            'sessions',
            fixture.sessionId,
            'renders',
            'previews',
            `${fixture.captureId}.jpg`,
          ),
          '-BaselineMetadataPath',
          mismatchedMetadataPath,
          '-DryRun',
          '-EmitJson',
        ]),
      ).toThrow(/same-capture|capture correlation|correlation/i)
    },
    40000,
  )

  it(
    'fails closed when selected capture correlation drifts to a foreign request id',
    () => {
      const fixture = createFixtureRepo()
      const evidenceLogPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'diagnostics',
        'dedicated-renderer',
        'preview-promotion-evidence.jsonl',
      )
      const record = JSON.parse(readFileSync(evidenceLogPath, 'utf8')) as Record<string, unknown>
      record.requestId = 'request_other'
      writeFileSync(evidenceLogPath, `${JSON.stringify(record)}\n`)

      expect(() =>
        runPowershell(path.resolve('scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1'), [
          '-RepoRoot',
          fixture.repoRoot,
          '-SessionId',
          fixture.sessionId,
          '-CaptureId',
          fixture.captureId,
          '-PresetId',
          fixture.presetId,
          '-PublishedVersion',
          fixture.publishedVersion,
          '-BoothVisualEvidencePaths',
          fixture.boothVisualPath,
          '-OperatorVisualEvidencePaths',
          fixture.operatorVisualPath,
          '-RollbackEvidencePaths',
          fixture.rollbackEvidencePath,
          '-DryRun',
          '-EmitJson',
        ]),
      ).toThrow(/wrong-capture|request id|capture correlation/i)
    },
    40000,
  )

  it(
    'fails closed when visible owner transition evidence is missing',
    () => {
      const fixture = createFixtureRepo()
      const evidenceLogPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'diagnostics',
        'dedicated-renderer',
        'preview-promotion-evidence.jsonl',
      )
      const record = JSON.parse(readFileSync(evidenceLogPath, 'utf8')) as Record<string, unknown>
      delete record.visibleOwner
      delete record.visibleOwnerTransitionAtMs
      writeFileSync(evidenceLogPath, `${JSON.stringify(record)}\n`)

      expect(() =>
        runPowershell(path.resolve('scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1'), [
          '-RepoRoot',
          fixture.repoRoot,
          '-SessionId',
          fixture.sessionId,
          '-CaptureId',
          fixture.captureId,
          '-PresetId',
          fixture.presetId,
          '-PublishedVersion',
          fixture.publishedVersion,
          '-BoothVisualEvidencePaths',
          fixture.boothVisualPath,
          '-OperatorVisualEvidencePaths',
          fixture.operatorVisualPath,
          '-RollbackEvidencePaths',
          fixture.rollbackEvidencePath,
          '-DryRun',
          '-EmitJson',
        ]),
      ).toThrow(/visible owner|transition/i)
    },
    40000,
  )

  it(
    'fails closed when bundle assembly tries to reuse live route policy or catalog state',
    () => {
      const fixture = createFixtureRepo()
      const evidenceLogPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'diagnostics',
        'dedicated-renderer',
        'preview-promotion-evidence.jsonl',
      )
      const record = JSON.parse(readFileSync(evidenceLogPath, 'utf8')) as Record<string, unknown>
      record.routePolicySnapshotPath = path
        .join(fixture.repoRoot, 'branch-config', 'preview-renderer-policy.json')
        .replace(/\\/g, '/')
      record.catalogStatePath = path
        .join(fixture.repoRoot, 'preset-catalog', 'catalog-state.json')
        .replace(/\\/g, '/')
      writeFileSync(evidenceLogPath, `${JSON.stringify(record)}\n`)

      expect(() =>
        runPowershell(path.resolve('scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1'), [
          '-RepoRoot',
          fixture.repoRoot,
          '-SessionId',
          fixture.sessionId,
          '-CaptureId',
          fixture.captureId,
          '-PresetId',
          fixture.presetId,
          '-PublishedVersion',
          fixture.publishedVersion,
          '-BoothVisualEvidencePaths',
          fixture.boothVisualPath,
          '-OperatorVisualEvidencePaths',
          fixture.operatorVisualPath,
          '-RollbackEvidencePaths',
          fixture.rollbackEvidencePath,
          '-DryRun',
          '-EmitJson',
        ]),
      ).toThrow(/stale-preview|capture-time snapshot|live policy|live catalog/i)
    },
    40000,
  )

  it(
    'fails closed when bundle assembly points at a stale same-session snapshot for another capture',
    () => {
      const fixture = createFixtureRepo()
      const staleRoutePolicySnapshotPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'diagnostics',
        'dedicated-renderer',
        'captured-preview-renderer-policy-capture_other.json',
      )
      const staleCatalogSnapshotPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'diagnostics',
        'dedicated-renderer',
        'captured-catalog-state-capture_other.json',
      )
      writeFileSync(
        staleRoutePolicySnapshotPath,
        JSON.stringify({ schemaVersion: 'preview-renderer-route-policy/v1', capturedAt: 'capture-time' }, null, 2),
      )
      writeFileSync(
        staleCatalogSnapshotPath,
        JSON.stringify({ schemaVersion: 'preset-catalog-state/v1', capturedAt: true }, null, 2),
      )

      const evidenceLogPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'diagnostics',
        'dedicated-renderer',
        'preview-promotion-evidence.jsonl',
      )
      const record = JSON.parse(readFileSync(evidenceLogPath, 'utf8')) as Record<string, unknown>
      record.routePolicySnapshotPath = staleRoutePolicySnapshotPath.replace(/\\/g, '/')
      record.catalogStatePath = staleCatalogSnapshotPath.replace(/\\/g, '/')
      writeFileSync(evidenceLogPath, `${JSON.stringify(record)}\n`)

      expect(() =>
        runPowershell(path.resolve('scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1'), [
          '-RepoRoot',
          fixture.repoRoot,
          '-SessionId',
          fixture.sessionId,
          '-CaptureId',
          fixture.captureId,
          '-PresetId',
          fixture.presetId,
          '-PublishedVersion',
          fixture.publishedVersion,
          '-BoothVisualEvidencePaths',
          fixture.boothVisualPath,
          '-OperatorVisualEvidencePaths',
          fixture.operatorVisualPath,
          '-RollbackEvidencePaths',
          fixture.rollbackEvidencePath,
          '-DryRun',
          '-EmitJson',
        ]),
      ).toThrow(/stale-preview|capture-time snapshot|capture_other|selected capture/i)
    },
    40000,
  )

  it(
    'keeps only the selected promotion evidence record in the assembled bundle output',
    () => {
      const fixture = createFixtureRepo()
      const evidenceLogPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'diagnostics',
        'dedicated-renderer',
        'preview-promotion-evidence.jsonl',
      )
      writeFileSync(
        evidenceLogPath,
        [
          JSON.stringify({
            schemaVersion: 'preview-promotion-evidence-record/v1',
            observedAt: '2026-04-12T08:00:10+09:00',
            sessionId: fixture.sessionId,
            requestId: 'request_earlier',
            captureId: 'capture_other',
            presetId: fixture.presetId,
            publishedVersion: fixture.publishedVersion,
            laneOwner: 'dedicated-renderer',
            fallbackReasonCode: null,
            routeStage: 'canary',
            warmState: 'warm-ready',
            firstVisibleMs: 1000,
            replacementMs: 1200,
            originalVisibleToPresetAppliedVisibleMs: 200,
            sessionManifestPath: path.join(
              fixture.repoRoot,
              'sessions',
              fixture.sessionId,
              'session.json',
            ).replace(/\\/g, '/'),
            timingEventsPath: path.join(
              fixture.repoRoot,
              'sessions',
              fixture.sessionId,
              'diagnostics',
              'timing-events.log',
            ).replace(/\\/g, '/'),
            routePolicySnapshotPath: path.join(
              fixture.repoRoot,
              'branch-config',
              'preview-renderer-policy.json',
            ).replace(/\\/g, '/'),
            publishedBundlePath: path.join(
              fixture.repoRoot,
              'preset-catalog',
              'published',
              fixture.presetId,
              fixture.publishedVersion,
              'bundle.json',
            ).replace(/\\/g, '/'),
            catalogStatePath: path.join(
              fixture.repoRoot,
              'preset-catalog',
              'catalog-state.json',
            ).replace(/\\/g, '/'),
            previewAssetPath: path.join(
              fixture.repoRoot,
              'sessions',
              fixture.sessionId,
              'renders',
              'previews',
              `${fixture.captureId}.jpg`,
            ).replace(/\\/g, '/'),
            warmStateDetailPath: null,
          }),
          readFileSync(evidenceLogPath, 'utf8'),
        ].join('\n'),
      )

      const outputRoot = path.join(fixture.repoRoot, 'artifacts', 'bundle-output')
      runPowershell(path.resolve('scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1'), [
        '-RepoRoot',
        fixture.repoRoot,
        '-SessionId',
        fixture.sessionId,
        '-CaptureId',
        fixture.captureId,
        '-PresetId',
        fixture.presetId,
        '-PublishedVersion',
        fixture.publishedVersion,
        '-BoothVisualEvidencePaths',
        fixture.boothVisualPath,
        '-OperatorVisualEvidencePaths',
        fixture.operatorVisualPath,
        '-RollbackEvidencePaths',
        fixture.rollbackEvidencePath,
        '-OutputRoot',
        outputRoot,
        '-EmitJson',
      ])

      const copiedEvidenceLines = readFileSync(
        path.join(outputRoot, 'preview-promotion-evidence.jsonl'),
        'utf8',
      )
        .trim()
        .split(/\r?\n/)
      expect(copiedEvidenceLines).toHaveLength(1)
      expect(JSON.parse(copiedEvidenceLines[0]).captureId).toBe(fixture.captureId)
    },
    40000,
  )

  it(
    'keeps only the selected capture timing chain in the assembled bundle output',
    () => {
      const fixture = createFixtureRepo()
      const timingEventsPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'diagnostics',
        'timing-events.log',
      )
      writeFileSync(
        timingEventsPath,
        [
          readFileSync(timingEventsPath, 'utf8'),
          '2026-04-12T08:00:22+09:00\tsession=' +
            fixture.sessionId +
            '\tcapture=capture_other\trequest=request_other\tevent=recent-session-visible\tdetail=visibleOwner=foreign-lane;visibleOwnerTransitionAtMs=1800',
          '2026-04-12T08:00:23+09:00\tsession=session_other\tcapture=' +
            fixture.captureId +
            '\trequest=' +
            'request_cross_session' +
            '\tevent=capture_preview_transition_summary\tdetail=laneOwner=foreign;fallbackReason=none;routeStage=shadow;warmState=warm-ready;firstVisibleMs=1200;replacementMs=1800;originalVisibleToPresetAppliedVisibleMs=600',
        ].join('\n'),
      )

      const outputRoot = path.join(fixture.repoRoot, 'artifacts', 'bundle-output-filtered-timing')
      runPowershell(path.resolve('scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1'), [
        '-RepoRoot',
        fixture.repoRoot,
        '-SessionId',
        fixture.sessionId,
        '-CaptureId',
        fixture.captureId,
        '-PresetId',
        fixture.presetId,
        '-PublishedVersion',
        fixture.publishedVersion,
        '-BoothVisualEvidencePaths',
        fixture.boothVisualPath,
        '-OperatorVisualEvidencePaths',
        fixture.operatorVisualPath,
        '-RollbackEvidencePaths',
        fixture.rollbackEvidencePath,
        '-OutputRoot',
        outputRoot,
        '-EmitJson',
      ])

      const copiedTimingLines = readFileSync(path.join(outputRoot, 'timing-events.log'), 'utf8')
        .trim()
        .split(/\r?\n/)
      expect(copiedTimingLines.length).toBeGreaterThan(0)
      expect(
        copiedTimingLines.every(
          (line) =>
            line.includes(`session=${fixture.sessionId}`) &&
            line.includes('request=request_20260410_001') &&
            (
              line.includes('event=request-capture') ||
              line.includes('event=capture-accepted') ||
              line.includes(`capture=${fixture.captureId}`)
            ),
        ),
      ).toBe(true)
    },
    40000,
  )

  it(
    'reports fallback ratio from the matching promotion evidence family while bundling only the selected capture',
    () => {
      const fixture = createFixtureRepo()
      const evidenceLogPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'diagnostics',
        'dedicated-renderer',
        'preview-promotion-evidence.jsonl',
      )
      writeFileSync(
        evidenceLogPath,
        [
          JSON.stringify({
            schemaVersion: 'preview-promotion-evidence-record/v1',
            observedAt: '2026-04-12T08:00:10+09:00',
            sessionId: fixture.sessionId,
            requestId: 'request_fallback',
            captureId: 'capture_fallback',
            presetId: fixture.presetId,
            publishedVersion: fixture.publishedVersion,
            laneOwner: 'inline-truthful-fallback',
            fallbackReasonCode: 'shadow-submission-only',
            routeStage: 'canary',
            warmState: 'warm-ready',
            firstVisibleMs: 1100,
            replacementMs: null,
            originalVisibleToPresetAppliedVisibleMs: null,
            sessionManifestPath: path.join(
              fixture.repoRoot,
              'sessions',
              fixture.sessionId,
              'session.json',
            ).replace(/\\/g, '/'),
            timingEventsPath: path.join(
              fixture.repoRoot,
              'sessions',
              fixture.sessionId,
              'diagnostics',
              'timing-events.log',
            ).replace(/\\/g, '/'),
            routePolicySnapshotPath: path.join(
              fixture.repoRoot,
              'branch-config',
              'preview-renderer-policy.json',
            ).replace(/\\/g, '/'),
            publishedBundlePath: path.join(
              fixture.repoRoot,
              'preset-catalog',
              'published',
              fixture.presetId,
              fixture.publishedVersion,
              'bundle.json',
            ).replace(/\\/g, '/'),
            catalogStatePath: path.join(
              fixture.repoRoot,
              'preset-catalog',
              'catalog-state.json',
            ).replace(/\\/g, '/'),
            previewAssetPath: path.join(
              fixture.repoRoot,
              'sessions',
              fixture.sessionId,
              'renders',
              'previews',
              `${fixture.captureId}.jpg`,
            ).replace(/\\/g, '/'),
            warmStateDetailPath: null,
          }),
          readFileSync(evidenceLogPath, 'utf8'),
        ].join('\n'),
      )

      const result = runPowershell(
        path.resolve('scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1'),
        [
          '-RepoRoot',
          fixture.repoRoot,
          '-SessionId',
          fixture.sessionId,
          '-CaptureId',
          fixture.captureId,
          '-PresetId',
          fixture.presetId,
          '-PublishedVersion',
          fixture.publishedVersion,
          '-BoothVisualEvidencePaths',
          fixture.boothVisualPath,
          '-OperatorVisualEvidencePaths',
          fixture.operatorVisualPath,
          '-RollbackEvidencePaths',
          fixture.rollbackEvidencePath,
          '-DryRun',
          '-EmitJson',
        ],
      )

      expect(result.fallbackRatio).toBe(0.5)
      expect(result.captureId).toBe(fixture.captureId)
    },
    40000,
  )

  it(
    'classifies invalid parity image payloads as structured invalid-input failures',
    () => {
      const fixture = createFixtureRepo()
      const result = runPowershell(
        path.resolve('scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1'),
        [
          '-RepoRoot',
          fixture.repoRoot,
          '-SessionId',
          fixture.sessionId,
          '-CaptureId',
          fixture.captureId,
          '-PresetId',
          fixture.presetId,
          '-PublishedVersion',
          fixture.publishedVersion,
          '-BoothVisualEvidencePaths',
          fixture.boothVisualPath,
          '-OperatorVisualEvidencePaths',
          fixture.operatorVisualPath,
          '-RollbackEvidencePaths',
          fixture.rollbackEvidencePath,
          '-BaselineImagePath',
          path.join(
            fixture.repoRoot,
            'sessions',
            fixture.sessionId,
            'renders',
            'previews',
            `${fixture.captureId}.jpg`,
          ),
          '-BaselineMetadataPath',
          fixture.oracleMetadataPath,
          '-DryRun',
          '-EmitJson',
        ],
      )

      expect(result.parity.baseline.status).toBe('invalid-input')
      expect(result.parity.baseline.reason).toBe('image-decode-failed')
    },
    40000,
  )

  it(
    'fails closed when booth/operator visuals or rollback proof are omitted',
    () => {
      const fixture = createFixtureRepo()

      expect(() =>
        runPowershell(path.resolve('scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1'), [
          '-RepoRoot',
          fixture.repoRoot,
          '-SessionId',
          fixture.sessionId,
          '-CaptureId',
          fixture.captureId,
          '-PresetId',
          fixture.presetId,
          '-PublishedVersion',
          fixture.publishedVersion,
          '-DryRun',
          '-EmitJson',
        ]),
      ).toThrow(/booth visual evidence|operator visual evidence|rollback evidence/i)
    },
    40000,
  )

  it(
    'copies the capture-time policy and catalog snapshots recorded in the evidence record',
    () => {
      const fixture = createFixtureRepo()
      const routePolicySnapshotPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'diagnostics',
        `captured-preview-renderer-policy-${fixture.captureId}.json`,
      )
      const catalogSnapshotPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'diagnostics',
        `captured-catalog-state-${fixture.captureId}.json`,
      )
      writeFileSync(
        routePolicySnapshotPath,
        JSON.stringify({ schemaVersion: 'preview-renderer-route-policy/v1', defaultRoute: 'canary' }, null, 2),
      )
      writeFileSync(
        catalogSnapshotPath,
        JSON.stringify({ schemaVersion: 'preset-catalog-state/v1', captured: true }, null, 2),
      )
      writeFileSync(
        path.join(fixture.repoRoot, 'branch-config', 'preview-renderer-policy.json'),
        JSON.stringify({ schemaVersion: 'preview-renderer-route-policy/v1', defaultRoute: 'shadow' }, null, 2),
      )
      writeFileSync(
        path.join(fixture.repoRoot, 'preset-catalog', 'catalog-state.json'),
        JSON.stringify({ schemaVersion: 'preset-catalog-state/v1', captured: false }, null, 2),
      )

      const evidenceLogPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'diagnostics',
        'dedicated-renderer',
        'preview-promotion-evidence.jsonl',
      )
      const record = JSON.parse(readFileSync(evidenceLogPath, 'utf8')) as Record<string, unknown>
      record.routePolicySnapshotPath = routePolicySnapshotPath.replace(/\\/g, '/')
      record.catalogStatePath = catalogSnapshotPath.replace(/\\/g, '/')
      writeFileSync(evidenceLogPath, `${JSON.stringify(record)}\n`)

      const outputRoot = path.join(fixture.repoRoot, 'artifacts', 'captured-snapshot-bundle')
      runPowershell(path.resolve('scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1'), [
        '-RepoRoot',
        fixture.repoRoot,
        '-SessionId',
        fixture.sessionId,
        '-CaptureId',
        fixture.captureId,
        '-PresetId',
        fixture.presetId,
        '-PublishedVersion',
        fixture.publishedVersion,
        '-BoothVisualEvidencePaths',
        fixture.boothVisualPath,
        '-OperatorVisualEvidencePaths',
        fixture.operatorVisualPath,
        '-RollbackEvidencePaths',
        fixture.rollbackEvidencePath,
        '-OutputRoot',
        outputRoot,
        '-EmitJson',
      ])

      expect(
        JSON.parse(readFileSync(path.join(outputRoot, 'preview-renderer-policy.json'), 'utf8')),
      ).toEqual(JSON.parse(readFileSync(routePolicySnapshotPath, 'utf8')))
      expect(
        JSON.parse(readFileSync(path.join(outputRoot, 'catalog-state.json'), 'utf8')),
      ).toEqual(JSON.parse(readFileSync(catalogSnapshotPath, 'utf8')))
    },
    40000,
  )

  it(
    'assesses a valid canary bundle as Go when KPI, parity, rollback proof, and safety checks all pass',
    () => {
      const fixture = createFixtureRepo()
      const previewPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'renders',
        'previews',
        `${fixture.captureId}.jpg`,
      )
      const baselineImagePath = path.join(fixture.repoRoot, 'baseline-preview.png')
      writeValidTestRaster(previewPath)
      writeValidTestRaster(baselineImagePath)

      const bundle = buildCanaryBundle(fixture, [
        '-BaselineImagePath',
        baselineImagePath,
        '-BaselineMetadataPath',
        fixture.oracleMetadataPath,
      ])

      const assessment = runPowershell(
        path.resolve('scripts/hardware/Test-PreviewPromotionCanary.ps1'),
        ['-BundlePath', bundle.bundleManifestPath, '-EmitJson'],
      )

      expect(assessment.schemaVersion).toBe('preview-promotion-canary-assessment/v1')
      expect(assessment.gate).toBe('Go')
      expect(assessment.nextStageAllowed).toBe(true)
      expect(assessment.checks.kpi.status).toBe('pass')
      expect(assessment.checks.fallbackStability.status).toBe('pass')
      expect(assessment.checks.wrongCapture.status).toBe('pass')
      expect(assessment.checks.fidelityDrift.status).toBe('pass')
      expect(assessment.checks.rollbackReadiness.status).toBe('pass')
      expect(assessment.checks.activeSessionSafety.status).toBe('pass')
      expect(assessment.blockers).toEqual([])
    },
    40000,
  )

  it(
    'keeps the canary verdict at Go when bundle-local rollback evidence remains after the source artifact is removed',
    () => {
      const fixture = createFixtureRepo()
      const previewPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'renders',
        'previews',
        `${fixture.captureId}.jpg`,
      )
      const baselineImagePath = path.join(fixture.repoRoot, 'baseline-preview.png')
      writeValidTestRaster(previewPath)
      writeValidTestRaster(baselineImagePath)

      const bundle = buildCanaryBundle(fixture, [
        '-BaselineImagePath',
        baselineImagePath,
        '-BaselineMetadataPath',
        fixture.oracleMetadataPath,
      ])

      rmSync(fixture.rollbackEvidencePath, { force: true })

      const assessment = runPowershell(
        path.resolve('scripts/hardware/Test-PreviewPromotionCanary.ps1'),
        ['-BundlePath', bundle.bundleManifestPath, '-EmitJson'],
      )

      expect(assessment.gate).toBe('Go')
      expect(assessment.checks.rollbackReadiness.status).toBe('pass')
      expect(assessment.blockers).toEqual([])
    },
    40000,
  )

  it(
    'treats fallbackReasonCode=none as non-fallback evidence for canary stability',
    () => {
      const fixture = createFixtureRepo()
      const previewPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'renders',
        'previews',
        `${fixture.captureId}.jpg`,
      )
      const baselineImagePath = path.join(fixture.repoRoot, 'baseline-preview.png')
      writeValidTestRaster(previewPath)
      writeValidTestRaster(baselineImagePath)

      const bundle = buildCanaryBundle(fixture, [
        '-BaselineImagePath',
        baselineImagePath,
        '-BaselineMetadataPath',
        fixture.oracleMetadataPath,
      ])
      const bundleRecord = readJsonFile(bundle.bundleManifestPath)
      bundleRecord.fallbackReasonCode = 'none'
      writeFileSync(bundle.bundleManifestPath, JSON.stringify(bundleRecord, null, 2))

      const assessment = runPowershell(
        path.resolve('scripts/hardware/Test-PreviewPromotionCanary.ps1'),
        ['-BundlePath', bundle.bundleManifestPath, '-EmitJson'],
      )

      expect(assessment.gate).toBe('Go')
      expect(assessment.checks.fallbackStability.status).toBe('pass')
      expect(assessment.blockers).toEqual([])
    },
    40000,
  )

  it(
    'keeps the canary verdict at No-Go when the primary full-screen KPI misses 2500ms',
    () => {
      const fixture = createFixtureRepo()
      const evidenceLogPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'diagnostics',
        'dedicated-renderer',
        'preview-promotion-evidence.jsonl',
      )
      const record = JSON.parse(readFileSync(evidenceLogPath, 'utf8')) as Record<string, unknown>
      record.sameCaptureFullScreenVisibleMs = 2801
      writeFileSync(evidenceLogPath, `${JSON.stringify(record)}\n`)

      const previewPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'renders',
        'previews',
        `${fixture.captureId}.jpg`,
      )
      const baselineImagePath = path.join(fixture.repoRoot, 'baseline-preview.png')
      writeValidTestRaster(previewPath)
      writeValidTestRaster(baselineImagePath)

      const bundle = buildCanaryBundle(fixture, [
        '-BaselineImagePath',
        baselineImagePath,
        '-BaselineMetadataPath',
        fixture.oracleMetadataPath,
      ])
      const assessment = runPowershell(
        path.resolve('scripts/hardware/Test-PreviewPromotionCanary.ps1'),
        ['-BundlePath', bundle.bundleManifestPath, '-EmitJson'],
      )

      expect(assessment.gate).toBe('No-Go')
      expect(assessment.nextStageAllowed).toBe(false)
      expect(assessment.checks.kpi.status).toBe('fail')
      expect(assessment.blockers).toContain('kpi-miss')
    },
    40000,
  )

  it(
    'keeps the canary verdict at No-Go when fallback-heavy evidence remains in the same canary family',
    () => {
      const fixture = createFixtureRepo()
      const evidenceLogPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'diagnostics',
        'dedicated-renderer',
        'preview-promotion-evidence.jsonl',
      )
      writeFileSync(
        evidenceLogPath,
        [
          JSON.stringify({
            schemaVersion: 'preview-promotion-evidence-record/v1',
            observedAt: '2026-04-12T08:00:10+09:00',
            sessionId: fixture.sessionId,
            requestId: 'request_fallback',
            captureId: 'capture_fallback',
            presetId: fixture.presetId,
            publishedVersion: fixture.publishedVersion,
            laneOwner: 'inline-truthful-fallback',
            fallbackReasonCode: 'shadow-submission-only',
            routeStage: 'canary',
            warmState: 'warm-ready',
            firstVisibleMs: 1100,
            replacementMs: null,
            originalVisibleToPresetAppliedVisibleMs: null,
            sessionManifestPath: path.join(
              fixture.repoRoot,
              'sessions',
              fixture.sessionId,
              'session.json',
            ).replace(/\\/g, '/'),
            timingEventsPath: path.join(
              fixture.repoRoot,
              'sessions',
              fixture.sessionId,
              'diagnostics',
              'timing-events.log',
            ).replace(/\\/g, '/'),
            routePolicySnapshotPath: path.join(
              fixture.repoRoot,
              'sessions',
              fixture.sessionId,
              'diagnostics',
              'dedicated-renderer',
              `captured-preview-renderer-policy-${fixture.captureId}.json`,
            ).replace(/\\/g, '/'),
            publishedBundlePath: path.join(
              fixture.repoRoot,
              'preset-catalog',
              'published',
              fixture.presetId,
              fixture.publishedVersion,
              'bundle.json',
            ).replace(/\\/g, '/'),
            catalogStatePath: path.join(
              fixture.repoRoot,
              'sessions',
              fixture.sessionId,
              'diagnostics',
              'dedicated-renderer',
              `captured-catalog-state-${fixture.captureId}.json`,
            ).replace(/\\/g, '/'),
            previewAssetPath: path.join(
              fixture.repoRoot,
              'sessions',
              fixture.sessionId,
              'renders',
              'previews',
              `${fixture.captureId}.jpg`,
            ).replace(/\\/g, '/'),
            warmStateDetailPath: null,
          }),
          readFileSync(evidenceLogPath, 'utf8'),
        ].join('\n'),
      )

      const previewPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'renders',
        'previews',
        `${fixture.captureId}.jpg`,
      )
      const baselineImagePath = path.join(fixture.repoRoot, 'baseline-preview.png')
      writeValidTestRaster(previewPath)
      writeValidTestRaster(baselineImagePath)

      const bundle = buildCanaryBundle(fixture, [
        '-BaselineImagePath',
        baselineImagePath,
        '-BaselineMetadataPath',
        fixture.oracleMetadataPath,
      ])
      const assessment = runPowershell(
        path.resolve('scripts/hardware/Test-PreviewPromotionCanary.ps1'),
        ['-BundlePath', bundle.bundleManifestPath, '-EmitJson'],
      )

      expect(assessment.gate).toBe('No-Go')
      expect(assessment.checks.fallbackStability.status).toBe('fail')
      expect(assessment.blockers).toContain('fallback-instability')
    },
    40000,
  )

  it(
    'keeps the canary verdict at No-Go when the selected-capture timing chain no longer matches the bundle request id',
    () => {
      const fixture = createFixtureRepo()
      const previewPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'renders',
        'previews',
        `${fixture.captureId}.jpg`,
      )
      const baselineImagePath = path.join(fixture.repoRoot, 'baseline-preview.png')
      writeValidTestRaster(previewPath)
      writeValidTestRaster(baselineImagePath)

      const bundle = buildCanaryBundle(fixture, [
        '-BaselineImagePath',
        baselineImagePath,
        '-BaselineMetadataPath',
        fixture.oracleMetadataPath,
      ])
      writeFileSync(
        path.join(path.dirname(bundle.bundleManifestPath), 'timing-events.log'),
        '2026-04-12T08:00:00+09:00\tsession=' +
          fixture.sessionId +
          '\tcapture=none\trequest=request_other\tevent=request-capture\tdetail=routeStage=canary\n',
      )

      const assessment = runPowershell(
        path.resolve('scripts/hardware/Test-PreviewPromotionCanary.ps1'),
        ['-BundlePath', bundle.bundleManifestPath, '-EmitJson'],
      )

      expect(assessment.gate).toBe('No-Go')
      expect(assessment.checks.wrongCapture.status).toBe('fail')
      expect(assessment.blockers).toContain('wrong-capture')
    },
    40000,
  )

  it(
    'keeps the canary verdict at No-Go when parity indicates fidelity drift',
    () => {
      const fixture = createFixtureRepo()
      const previewPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'renders',
        'previews',
        `${fixture.captureId}.jpg`,
      )
      writeValidTestRaster(previewPath)
      const baselineImagePath = path.join(fixture.repoRoot, 'baseline-preview.png')
      writeValidTestRaster(baselineImagePath)

      const bundle = buildCanaryBundle(fixture, [
        '-BaselineImagePath',
        baselineImagePath,
        '-BaselineMetadataPath',
        fixture.oracleMetadataPath,
      ])
      const bundleRecord = readJsonFile(bundle.bundleManifestPath)
      ;(bundleRecord.parity as Record<string, unknown>).result = 'fail'
      ;(bundleRecord.parity as Record<string, unknown>).reason = 'threshold-exceeded'
      writeFileSync(bundle.bundleManifestPath, JSON.stringify(bundleRecord, null, 2))

      const assessment = runPowershell(
        path.resolve('scripts/hardware/Test-PreviewPromotionCanary.ps1'),
        ['-BundlePath', bundle.bundleManifestPath, '-EmitJson'],
      )

      expect(assessment.gate).toBe('No-Go')
      expect(assessment.checks.fidelityDrift.status).toBe('fail')
      expect(assessment.blockers).toContain('fidelity-drift')
    },
    40000,
  )

  it(
    'keeps the canary verdict at No-Go when rollback proof is missing from the bundle manifest',
    () => {
      const fixture = createFixtureRepo()
      const previewPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'renders',
        'previews',
        `${fixture.captureId}.jpg`,
      )
      const baselineImagePath = path.join(fixture.repoRoot, 'baseline-preview.png')
      writeValidTestRaster(previewPath)
      writeValidTestRaster(baselineImagePath)

      const bundle = buildCanaryBundle(fixture, [
        '-BaselineImagePath',
        baselineImagePath,
        '-BaselineMetadataPath',
        fixture.oracleMetadataPath,
      ])
      const bundleRecord = readJsonFile(bundle.bundleManifestPath)
      bundleRecord.rollbackEvidence = []
      writeFileSync(bundle.bundleManifestPath, JSON.stringify(bundleRecord, null, 2))

      const assessment = runPowershell(
        path.resolve('scripts/hardware/Test-PreviewPromotionCanary.ps1'),
        ['-BundlePath', bundle.bundleManifestPath, '-EmitJson'],
      )

      expect(assessment.gate).toBe('No-Go')
      expect(assessment.nextStageAllowed).toBe(false)
      expect(assessment.checks.rollbackReadiness.status).toBe('fail')
      expect(assessment.blockers).toContain('rollback-proof-missing')
    },
    40000,
  )

  it(
    'keeps the canary verdict at No-Go when the bundle no longer represents a canary-scoped session',
    () => {
      const fixture = createFixtureRepo()
      const previewPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'renders',
        'previews',
        `${fixture.captureId}.jpg`,
      )
      const baselineImagePath = path.join(fixture.repoRoot, 'baseline-preview.png')
      writeValidTestRaster(previewPath)
      writeValidTestRaster(baselineImagePath)

      const bundle = buildCanaryBundle(fixture, [
        '-BaselineImagePath',
        baselineImagePath,
        '-BaselineMetadataPath',
        fixture.oracleMetadataPath,
      ])
      const bundleRecord = readJsonFile(bundle.bundleManifestPath)
      bundleRecord.routeStage = 'default'
      writeFileSync(bundle.bundleManifestPath, JSON.stringify(bundleRecord, null, 2))

      const assessment = runPowershell(
        path.resolve('scripts/hardware/Test-PreviewPromotionCanary.ps1'),
        ['-BundlePath', bundle.bundleManifestPath, '-EmitJson'],
      )

      expect(assessment.gate).toBe('No-Go')
      expect(assessment.checks.activeSessionSafety.status).toBe('fail')
      expect(assessment.blockers).toContain('active-session-safety')
    },
    40000,
  )

  it(
    'keeps the canary verdict at No-Go when the bundled promotion evidence is stale',
    () => {
      const fixture = createFixtureRepo()
      const previewPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'renders',
        'previews',
        `${fixture.captureId}.jpg`,
      )
      const baselineImagePath = path.join(fixture.repoRoot, 'baseline-preview.png')
      writeValidTestRaster(previewPath)
      writeValidTestRaster(baselineImagePath)

      const bundle = buildCanaryBundle(fixture, [
        '-BaselineImagePath',
        baselineImagePath,
        '-BaselineMetadataPath',
        fixture.oracleMetadataPath,
      ])
      const bundleRecord = readJsonFile(bundle.bundleManifestPath)
      writeFileSync(
        path.join(path.dirname(bundle.bundleManifestPath), 'preview-promotion-evidence.jsonl'),
        JSON.stringify({
          ...readJsonFile(path.join(path.dirname(bundle.bundleManifestPath), 'preview-promotion-evidence.jsonl')),
          observedAt: '2026-04-10T08:00:15+09:00',
        }) + '\n',
      )

      const assessment = runPowershell(
        path.resolve('scripts/hardware/Test-PreviewPromotionCanary.ps1'),
        [
          '-BundlePath',
          bundle.bundleManifestPath,
          '-PrimaryThresholdMs',
          '2500',
          '-EmitJson',
        ],
      )

      expect(assessment.gate).toBe('No-Go')
      expect(assessment.checks.activeSessionSafety.status).toBe('fail')
      expect(assessment.blockers).toContain('stale-evidence')
      expect(bundleRecord.routeStage).toBe('canary')
    },
    40000,
  )

  it(
    'keeps the canary verdict at No-Go when a follow-up capture timeout is recorded after the selected capture',
    () => {
      const fixture = createFixtureRepo()
      const previewPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'renders',
        'previews',
        `${fixture.captureId}.jpg`,
      )
      const baselineImagePath = path.join(fixture.repoRoot, 'baseline-preview.png')
      writeValidTestRaster(previewPath)
      writeValidTestRaster(baselineImagePath)
      writeFileSync(
        path.join(fixture.repoRoot, 'diagnostics', 'operator-audit-log.json'),
        JSON.stringify(
          {
            schemaVersion: 'operator-audit-store/v1',
            entries: [
              {
                schemaVersion: 'operator-audit-entry/v1',
                eventId: 'audit-follow-up-timeout',
                occurredAt: '2026-04-12T08:01:15+09:00',
                sessionId: fixture.sessionId,
                eventCategory: 'critical-failure',
                eventType: 'capture-round-trip-failed',
                summary: '촬영 결과를 세션에 저장하지 못했어요.',
                detail:
                  '셔터 요청 뒤 file-arrived 경계를 확인하지 못해 세션을 phone-required로 고정했어요.',
                actorId: null,
                source: 'capture-boundary',
                captureId: null,
                presetId: fixture.presetId,
                publishedVersion: fixture.publishedVersion,
                reasonCode: 'capture-timeout',
              },
            ],
          },
          null,
          2,
        ),
      )

      const bundle = buildCanaryBundle(fixture, [
        '-BaselineImagePath',
        baselineImagePath,
        '-BaselineMetadataPath',
        fixture.oracleMetadataPath,
      ])

      const assessment = runPowershell(
        path.resolve('scripts/hardware/Test-PreviewPromotionCanary.ps1'),
        ['-BundlePath', bundle.bundleManifestPath, '-EmitJson'],
      )

      expect(assessment.gate).toBe('No-Go')
      expect(assessment.checks.activeSessionSafety.status).toBe('fail')
      expect(assessment.blockers).toContain('follow-up-capture-health')
    },
    40000,
  )

  it(
    'keeps the canary verdict at No-Go when timing evidence points outside the assembled bundle',
    () => {
      const fixture = createFixtureRepo()
      const previewPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'renders',
        'previews',
        `${fixture.captureId}.jpg`,
      )
      const baselineImagePath = path.join(fixture.repoRoot, 'baseline-preview.png')
      writeValidTestRaster(previewPath)
      writeValidTestRaster(baselineImagePath)

      const bundle = buildCanaryBundle(fixture, [
        '-BaselineImagePath',
        baselineImagePath,
        '-BaselineMetadataPath',
        fixture.oracleMetadataPath,
      ])
      const bundleRecord = readJsonFile(bundle.bundleManifestPath)
      ;(bundleRecord.artifacts as Record<string, { destination: string }>).timingEvents.destination =
        path
          .join(
            fixture.repoRoot,
            'sessions',
            fixture.sessionId,
            'diagnostics',
            'timing-events.log',
          )
          .replace(/\\/g, '/')
      writeFileSync(bundle.bundleManifestPath, JSON.stringify(bundleRecord, null, 2))

      const assessment = runPowershell(
        path.resolve('scripts/hardware/Test-PreviewPromotionCanary.ps1'),
        ['-BundlePath', bundle.bundleManifestPath, '-EmitJson'],
      )

      expect(assessment.gate).toBe('No-Go')
      expect(assessment.checks.wrongCapture.status).toBe('fail')
      expect(assessment.blockers).toContain('wrong-capture')
    },
    40000,
  )

  it(
    'keeps the canary verdict at No-Go when rollback proof paths are moved outside the assembled bundle',
    () => {
      const fixture = createFixtureRepo()
      const previewPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'renders',
        'previews',
        `${fixture.captureId}.jpg`,
      )
      const baselineImagePath = path.join(fixture.repoRoot, 'baseline-preview.png')
      writeValidTestRaster(previewPath)
      writeValidTestRaster(baselineImagePath)

      const bundle = buildCanaryBundle(fixture, [
        '-BaselineImagePath',
        baselineImagePath,
        '-BaselineMetadataPath',
        fixture.oracleMetadataPath,
      ])
      const bundleRecord = readJsonFile(bundle.bundleManifestPath)
      bundleRecord.rollbackEvidence = [fixture.rollbackEvidencePath]
      writeFileSync(bundle.bundleManifestPath, JSON.stringify(bundleRecord, null, 2))

      const assessment = runPowershell(
        path.resolve('scripts/hardware/Test-PreviewPromotionCanary.ps1'),
        ['-BundlePath', bundle.bundleManifestPath, '-EmitJson'],
      )

      expect(assessment.gate).toBe('No-Go')
      expect(assessment.checks.rollbackReadiness.status).toBe('fail')
      expect(assessment.blockers).toContain('rollback-proof-missing')
    },
    40000,
  )

  it(
    'keeps the canary verdict at No-Go when snapshot artifacts only match the bundle root by string prefix',
    () => {
      const fixture = createFixtureRepo()
      const previewPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'renders',
        'previews',
        `${fixture.captureId}.jpg`,
      )
      const baselineImagePath = path.join(fixture.repoRoot, 'baseline-preview.png')
      writeValidTestRaster(previewPath)
      writeValidTestRaster(baselineImagePath)

      const bundle = buildCanaryBundle(fixture, [
        '-BaselineImagePath',
        baselineImagePath,
        '-BaselineMetadataPath',
        fixture.oracleMetadataPath,
      ])
      const bundleRoot = path.dirname(bundle.bundleManifestPath)
      const siblingRoot = `${bundleRoot}-old`
      mkdirSync(siblingRoot, { recursive: true })
      const prefixedRoutePolicyPath = path.join(siblingRoot, 'preview-renderer-policy.json')
      const prefixedCatalogPath = path.join(siblingRoot, 'catalog-state.json')
      writeFileSync(prefixedRoutePolicyPath, JSON.stringify({ schemaVersion: 'preview-renderer-route-policy/v1' }))
      writeFileSync(prefixedCatalogPath, JSON.stringify({ schemaVersion: 'preset-catalog-state/v1' }))

      const bundleRecord = readJsonFile(bundle.bundleManifestPath)
      ;(bundleRecord.artifacts as Record<string, { destination: string }>).routePolicySnapshot.destination =
        prefixedRoutePolicyPath.replace(/\\/g, '/')
      ;(bundleRecord.artifacts as Record<string, { destination: string }>).catalogState.destination =
        prefixedCatalogPath.replace(/\\/g, '/')
      writeFileSync(bundle.bundleManifestPath, JSON.stringify(bundleRecord, null, 2))

      const assessment = runPowershell(
        path.resolve('scripts/hardware/Test-PreviewPromotionCanary.ps1'),
        ['-BundlePath', bundle.bundleManifestPath, '-EmitJson'],
      )

      expect(assessment.gate).toBe('No-Go')
      expect(assessment.checks.activeSessionSafety.status).toBe('fail')
      expect(assessment.blockers).toContain('active-session-safety')
    },
    40000,
  )

  it(
    'keeps the canary verdict at No-Go when selected-capture event details drift from the bundle owner fields',
    () => {
      const fixture = createFixtureRepo()
      const previewPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'renders',
        'previews',
        `${fixture.captureId}.jpg`,
      )
      const baselineImagePath = path.join(fixture.repoRoot, 'baseline-preview.png')
      writeValidTestRaster(previewPath)
      writeValidTestRaster(baselineImagePath)

      const bundle = buildCanaryBundle(fixture, [
        '-BaselineImagePath',
        baselineImagePath,
        '-BaselineMetadataPath',
        fixture.oracleMetadataPath,
      ])
      writeFileSync(
        path.join(path.dirname(bundle.bundleManifestPath), 'timing-events.log'),
        [
          '2026-04-12T08:00:00+09:00\tsession=' +
            fixture.sessionId +
            '\tcapture=none\trequest=' +
            fixture.requestId +
            '\tevent=request-capture\tdetail=routeStage=canary',
          '2026-04-12T08:00:00.200+09:00\tsession=' +
            fixture.sessionId +
            '\tcapture=none\trequest=' +
            fixture.requestId +
            '\tevent=capture-accepted\tdetail=detailCode=capture-in-flight',
          '2026-04-12T08:00:01+09:00\tsession=' +
            fixture.sessionId +
            '\tcapture=' +
            fixture.captureId +
            '\trequest=' +
            fixture.requestId +
            '\tevent=file-arrived\tdetail=rawPersistedAtMs=100',
          '2026-04-12T08:00:01.100+09:00\tsession=' +
            fixture.sessionId +
            '\tcapture=' +
            fixture.captureId +
            '\trequest=' +
            fixture.requestId +
            '\tevent=fast-preview-ready\tdetail=kind=camera-thumbnail',
          '2026-04-12T08:00:12+09:00\tsession=' +
            fixture.sessionId +
            '\tcapture=' +
            fixture.captureId +
            '\trequest=' +
            fixture.requestId +
            '\tevent=capture_preview_ready\tdetail=truthfulArtifactReadyAtMs=900',
          '2026-04-12T08:00:15+09:00\tsession=' +
            fixture.sessionId +
            '\tcapture=' +
            fixture.captureId +
            '\trequest=' +
            fixture.requestId +
            '\tevent=recent-session-visible\tdetail=visibleOwner=foreign-owner;visibleOwnerTransitionAtMs=2410',
          '2026-04-12T08:00:15+09:00\tsession=' +
            fixture.sessionId +
            '\tcapture=' +
            fixture.captureId +
            '\trequest=' +
            fixture.requestId +
            '\tevent=capture_preview_transition_summary\tdetail=laneOwner=foreign-owner;fallbackReason=none;routeStage=canary;warmState=warm-ready;firstVisibleMs=1605;replacementMs=2410;originalVisibleToPresetAppliedVisibleMs=805',
        ].join('\n'),
      )

      const assessment = runPowershell(
        path.resolve('scripts/hardware/Test-PreviewPromotionCanary.ps1'),
        ['-BundlePath', bundle.bundleManifestPath, '-EmitJson'],
      )

      expect(assessment.gate).toBe('No-Go')
      expect(assessment.checks.wrongCapture.status).toBe('fail')
      expect(assessment.blockers).toContain('wrong-capture')
    },
    40000,
  )

  it(
    'returns a typed No-Go assessment when bundle fields are malformed instead of crashing',
    () => {
      const fixture = createFixtureRepo()
      const previewPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'renders',
        'previews',
        `${fixture.captureId}.jpg`,
      )
      const baselineImagePath = path.join(fixture.repoRoot, 'baseline-preview.png')
      writeValidTestRaster(previewPath)
      writeValidTestRaster(baselineImagePath)

      const bundle = buildCanaryBundle(fixture, [
        '-BaselineImagePath',
        baselineImagePath,
        '-BaselineMetadataPath',
        fixture.oracleMetadataPath,
      ])
      const bundleRecord = readJsonFile(bundle.bundleManifestPath)
      bundleRecord.fallbackRatio = { invalid: true }
      bundleRecord.sameCaptureFullScreenVisibleMs = { invalid: true }
      writeFileSync(bundle.bundleManifestPath, JSON.stringify(bundleRecord, null, 2))

      const assessment = runPowershell(
        path.resolve('scripts/hardware/Test-PreviewPromotionCanary.ps1'),
        ['-BundlePath', bundle.bundleManifestPath, '-EmitJson'],
      )

      expect(assessment.schemaVersion).toBe('preview-promotion-canary-assessment/v1')
      expect(assessment.gate).toBe('No-Go')
      expect(assessment.nextStageAllowed).toBe(false)
      expect(assessment.blockers).toContain('malformed-bundle')
      expect(assessment.checks.activeSessionSafety.status).toBe('fail')
    },
    40000,
  )
})
