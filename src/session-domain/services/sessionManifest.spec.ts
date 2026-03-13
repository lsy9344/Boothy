import { describe, expect, it } from 'vitest'

import { sessionManifestSchema as diskSessionManifestSchema } from '../../shared-contracts/schemas/manifestSchemas.js'
import { createSessionManifestDraft } from './sessionManifest.js'
import { resolveSessionPaths } from './sessionPaths.js'

describe('createSessionManifestDraft', () => {
  it('builds a manifest draft that matches the on-disk host contract', () => {
    const manifest = createSessionManifestDraft({
      sessionId: '2026-03-08:김보라 오후 세션',
      sessionName: '김보라 오후 세션',
      createdAt: '2026-03-08T00:00:00.000Z',
      reservationStartAt: '2026-03-08T00:00:00.000Z',
      sessionType: 'standard',
      paths: resolveSessionPaths({
        sessionRootBase: 'C:/Boothy/sessions/2026-03-08',
        sessionId: '김보라 오후 세션',
      }),
    })

    expect(diskSessionManifestSchema.parse(manifest)).toMatchObject({
      schemaVersion: 1,
      sessionId: '2026-03-08:김보라 오후 세션',
      sessionName: '김보라 오후 세션',
      operationalDate: '2026-03-08',
      manifestPath: 'C:/Boothy/sessions/2026-03-08/김보라 오후 세션/session.json',
      sessionDir: 'C:/Boothy/sessions/2026-03-08/김보라 오후 세션',
      processedDir: 'C:/Boothy/sessions/2026-03-08/김보라 오후 세션/processed',
      captureRevision: 0,
      activePresetName: null,
      activePreset: null,
      exportState: {
        status: 'notStarted',
      },
      timing: {
        sessionType: 'standard',
      },
    })
  })
})
