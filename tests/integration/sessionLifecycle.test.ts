import {
  appendFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'vitest'

import { createSessionLifecycleService } from '../../src/session-domain/services/sessionLifecycle.js'
import { resolveSameDaySessionName } from '../../src/session-domain/services/sessionNaming.js'
import { sessionManifestSchema } from '../../src/shared-contracts/schemas/manifestSchemas.js'
import { sessionStartCommandPayloadSchema } from '../../src/shared-contracts/schemas/sessionSchemas.js'

function createFakeHost(sessionRoot: string) {
  return async (command: string, args?: Record<string, unknown>) => {
    expect(command).toBe('start_session')

    const payload = sessionStartCommandPayloadSchema.parse(args?.payload)
    const operationalDate = '2026-03-08'
    const dayRoot = join(sessionRoot, operationalDate)

    mkdirSync(dayRoot, { recursive: true })

    const existingNames = existsSync(dayRoot) ? readDirectoryNames(dayRoot) : []
    const sessionName = resolveSameDaySessionName(payload.sessionName, existingNames)
    const sessionDir = join(dayRoot, sessionName)
    const manifestPath = join(sessionDir, 'session.json')
    const eventsPath = join(sessionDir, 'events.ndjson')
    const exportStatusPath = join(sessionDir, 'export-status.json')
    const processedDir = join(sessionDir, 'processed')
    const createdAt = '2026-03-08T00:00:00.000Z'
    const sessionId = `${operationalDate}:${sessionName}`
    const reservationStartAt = payload.reservationStartAt ?? '2026-03-08T09:00:00.000Z'
    const sessionType = payload.sessionType ?? 'standard'
    const actualShootEndAt =
      sessionType === 'couponExtended' ? '2026-03-08T10:40:00.000Z' : '2026-03-08T09:50:00.000Z'

    mkdirSync(sessionDir, { recursive: true })
    mkdirSync(processedDir, { recursive: true })

    const manifest = sessionManifestSchema.parse({
      schemaVersion: 1,
      sessionId,
      sessionName,
      operationalDate,
      createdAt,
      sessionDir: toWirePath(sessionDir),
      manifestPath: toWirePath(manifestPath),
      eventsPath: toWirePath(eventsPath),
      exportStatusPath: toWirePath(exportStatusPath),
      processedDir: toWirePath(processedDir),
      latestCaptureId: null,
      activePresetName: null,
      activePreset: null,
      captures: [],
      timing: {
        reservationStartAt,
        actualShootEndAt,
        sessionType,
        operatorExtensionCount: 0,
        lastTimingUpdateAt: createdAt,
      },
      cameraState: {
        connectionState: 'offline',
      },
      exportState: {
        status: 'notStarted',
      },
    })

    writeFileSync(manifestPath, JSON.stringify(manifest, null, 2))
    writeFileSync(exportStatusPath, JSON.stringify({ schemaVersion: 1, status: 'notStarted' }, null, 2))
    appendFileSync(eventsPath, '')

    return {
      ok: true,
      value: {
        sessionId,
        sessionName,
        sessionFolder: toWirePath(sessionDir),
        manifestPath: toWirePath(manifestPath),
        createdAt,
        preparationState: 'preparing',
      },
    }
  }
}

function readDirectoryNames(path: string): string[] {
  return readdirSync(path, { withFileTypes: true })
    .filter((entry: { isDirectory: () => boolean }) => entry.isDirectory())
    .map((entry: { name: string }) => entry.name)
}

function toWirePath(path: string): string {
  return path.replaceAll('\\', '/')
}

describe('session lifecycle integration', () => {
  it('preserves typed validation failures from the host start-session boundary', async () => {
    const service = createSessionLifecycleService(async () => ({
      ok: false,
      errorCode: 'session.validation_failed',
      message: 'Session details are invalid.',
    }))

    await expect(
      service.startSession({
        branchId: 'branch-unconfigured',
        sessionName: '김보라 오후 세션',
      }),
    ).resolves.toEqual({
      ok: false,
      errorCode: 'session.validation_failed',
      message: 'Session details are invalid.',
    })
  })

  it('provisions a valid session and writes session.json through the current host contract', async () => {
    const sessionRoot = mkdtempSync(join(tmpdir(), 'boothy-session-lifecycle-'))
    const service = createSessionLifecycleService(createFakeHost(sessionRoot))

    const result = await service.startSession({
      branchId: 'branch-unconfigured',
      sessionName: '김보라 오후 세션',
    })

    expect(result).toMatchObject({
      ok: true,
    })

    if (!result.ok) {
      throw new Error('expected successful provisioning')
    }

    const manifest = sessionManifestSchema.parse(
      JSON.parse(readFileSync(result.value.manifestPath, 'utf8')),
    )
    expect(manifest.sessionName).toBe('김보라 오후 세션')
    expect(result.value.sessionId).toBe('2026-03-08:김보라 오후 세션')
    expect(result.value.preparationState).toBe('preparing')
  })

  it('adds same-day suffixes when the base session name already exists', async () => {
    const sessionRoot = mkdtempSync(join(tmpdir(), 'boothy-session-collision-'))
    const service = createSessionLifecycleService(createFakeHost(sessionRoot))

    const first = await service.startSession({
      branchId: 'branch-unconfigured',
      sessionName: '김보라 오후 세션',
    })
    const second = await service.startSession({
      branchId: 'branch-unconfigured',
      sessionName: '김보라 오후 세션',
    })

    expect(first).toMatchObject({
      ok: true,
    })
    expect(second).toMatchObject({
      ok: true,
    })

    if (!first.ok || !second.ok) {
      throw new Error('expected successful provisioning')
    }

    expect(first.value.sessionName).toBe('김보라 오후 세션')
    expect(second.value.sessionName).toBe('김보라 오후 세션_2')
    expect(existsSync(join(second.value.sessionFolder, 'session.json'))).toBe(true)
  })

  it('forwards coupon-aware timing inputs through the session-start boundary', async () => {
    const sessionRoot = mkdtempSync(join(tmpdir(), 'boothy-session-coupon-'))
    const service = createSessionLifecycleService(createFakeHost(sessionRoot))

    const result = await service.startSession({
      branchId: 'branch-unconfigured',
      sessionName: '김보라 오후 세션',
      reservationStartAt: '2026-03-08T09:00:00.000Z',
      sessionType: 'couponExtended',
    })

    expect(result).toMatchObject({
      ok: true,
    })

    if (!result.ok) {
      throw new Error('expected successful provisioning')
    }

    const manifest = sessionManifestSchema.parse(
      JSON.parse(readFileSync(result.value.manifestPath, 'utf8')),
    )

    expect(manifest.timing).toMatchObject({
      reservationStartAt: '2026-03-08T09:00:00.000Z',
      actualShootEndAt: '2026-03-08T10:40:00.000Z',
      sessionType: 'couponExtended',
    })
  })
})
