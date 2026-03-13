import {
  sessionIdentitySchema,
  type SessionIdentity,
} from '../../shared-contracts/dto/sessionManifest.js'
import { type SessionType } from '../../shared-contracts/dto/sessionTiming.js'
import {
  sessionManifestSchema as diskSessionManifestSchema,
  type SessionManifest,
} from '../../shared-contracts/schemas/manifestSchemas.js'
import { createSessionTimingState } from '../../timing-policy/services/shootEndCalculator.js'
import type { SessionPaths } from './sessionPaths.js'

type SessionManifestDraftInput = {
  createdAt: string
  paths: SessionPaths
  reservationStartAt: string
  sessionType: SessionType
} & SessionIdentity

export function createSessionManifestDraft(input: SessionManifestDraftInput): SessionManifest {
  const identity = sessionIdentitySchema.parse({
    sessionId: input.sessionId,
    sessionName: input.sessionName,
  })
  const timing = createSessionTimingState({
    reservationStartAt: input.reservationStartAt,
    sessionType: input.sessionType,
    updatedAt: input.createdAt,
  })
  const [operationalDate] = input.sessionId.split(':')

  return diskSessionManifestSchema.parse({
    schemaVersion: 1,
    ...identity,
    operationalDate,
    createdAt: input.createdAt,
    sessionDir: input.paths.sessionDir,
    manifestPath: input.paths.manifestPath,
    eventsPath: input.paths.eventsPath,
    exportStatusPath: input.paths.exportStatusPath,
    processedDir: input.paths.processedDir,
    captureRevision: 0,
    latestCaptureId: null,
    activePresetName: null,
    activePreset: null,
    captures: [],
    cameraState: {
      connectionState: 'offline',
    },
    timing,
    exportState: {
      status: 'notStarted',
    },
  })
}
