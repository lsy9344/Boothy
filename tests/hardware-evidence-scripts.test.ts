import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { afterEach, describe, expect, it } from 'vitest'

const createdRoots: string[] = []

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
  mkdirSync(path.join(sessionRoot, 'renders', 'previews'), { recursive: true })
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
              previewBudgetMs: 5000,
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
      '2026-04-12T08:00:15+09:00\tsession=' +
        sessionId +
        '\tcapture=' +
        captureId +
        '\trequest=' +
        requestId +
        '\tevent=capture_preview_transition_summary\tdetail=laneOwner=dedicated-renderer;fallbackReason=none;routeStage=canary;warmState=warm-ready;firstVisibleMs=2810;replacementMs=3615;originalVisibleToPresetAppliedVisibleMs=805',
    ].join('\n'),
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
      firstVisibleMs: 2810,
      replacementMs: 3615,
      originalVisibleToPresetAppliedVisibleMs: 805,
      sessionManifestPath: path.join(sessionRoot, 'session.json').replace(/\\/g, '/'),
      timingEventsPath: path
        .join(sessionRoot, 'diagnostics', 'timing-events.log')
        .replace(/\\/g, '/'),
      routePolicySnapshotPath: path
        .join(repoRoot, 'branch-config', 'preview-renderer-policy.json')
        .replace(/\\/g, '/'),
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
      catalogStatePath: path
        .join(repoRoot, 'preset-catalog', 'catalog-state.json')
        .replace(/\\/g, '/'),
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
    expect(result.fallbackRatio).toBe(0)
    expect(result.artifacts.sessionManifest.source).toContain('session.json')
    expect(result.artifacts.routePolicySnapshot.source).toContain(
      'preview-renderer-policy.json',
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
        'captured-preview-renderer-policy.json',
      )
      const catalogSnapshotPath = path.join(
        fixture.repoRoot,
        'sessions',
        fixture.sessionId,
        'diagnostics',
        'captured-catalog-state.json',
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
})
