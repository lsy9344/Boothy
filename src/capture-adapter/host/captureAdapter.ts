import { convertFileSrc, invoke, isTauri } from '@tauri-apps/api/core'

import {
  deleteSessionPhotoRequestSchema,
  deleteSessionPhotoResponseSchema,
  sessionGalleryRequestSchema,
  sessionGallerySnapshotSchema,
  type DeleteSessionPhotoRequest,
  type DeleteSessionPhotoResponse,
  type SessionGalleryRequest,
  type SessionGallerySnapshot,
} from '../../shared-contracts/dto/sessionGallery.js'
import { schemaVersions } from '../../shared-contracts/dto/schemaVersion.js'
import {
  ensureSessionScopedDeleteResponse,
  ensureSessionScopedGallery,
} from '../../export-pipeline/services/thumbnailIsolation.js'

type InvokeFn = (command: string, args?: Record<string, unknown>) => Promise<unknown>

type CaptureAdapterDependencies = {
  convertFileSrcFn?: (filePath: string) => string
  invokeFn?: InvokeFn
  isTauriFn?: () => boolean
}

function isPreviewablePath(path: string): boolean {
  return /^(asset|blob|data|https?):/i.test(path)
}

function toPreviewablePath(path: string, convertFileSrcFn: (filePath: string) => string): string {
  return isPreviewablePath(path) ? path : convertFileSrcFn(path)
}

function normalizeSessionGalleryPaths(
  snapshot: SessionGallerySnapshot,
  convertFileSrcFn: (filePath: string) => string,
): SessionGallerySnapshot {
  return {
    ...snapshot,
    items: snapshot.items.map((item) => ({
      ...item,
      previewPath: toPreviewablePath(item.previewPath, convertFileSrcFn),
      thumbnailPath: toPreviewablePath(item.thumbnailPath, convertFileSrcFn),
    })),
  }
}

function normalizeSessionScopedGallery(
  sessionId: string,
  snapshot: SessionGallerySnapshot,
  convertFileSrcFn: (filePath: string) => string,
  manifestPath?: string,
): SessionGallerySnapshot {
  const sessionScopedSnapshot = ensureSessionScopedGallery(sessionId, snapshot, manifestPath)

  return ensureSessionScopedGallery(
    sessionId,
    normalizeSessionGalleryPaths(sessionScopedSnapshot, convertFileSrcFn),
  )
}

function createFallbackGallery(sessionId: string): SessionGallerySnapshot {
  return {
    schemaVersion: schemaVersions.contract,
    sessionId,
    sessionName: `${sessionId}-review`,
    shootEndsAt: '2026-03-08T10:50:00.000Z',
    activePresetName: 'Soft Noir',
    latestCaptureId: 'capture-002',
    selectedCaptureId: 'capture-002',
    items: [
      {
        captureId: 'capture-001',
        sessionId,
        capturedAt: '2026-03-08T10:05:00.000Z',
        displayOrder: 0,
        isLatest: false,
        previewPath: `asset://${sessionId}/capture-001`,
        thumbnailPath: `asset://${sessionId}/thumb-capture-001`,
        label: '첫 번째 사진',
      },
      {
        captureId: 'capture-002',
        sessionId,
        capturedAt: '2026-03-08T10:06:00.000Z',
        displayOrder: 1,
        isLatest: true,
        previewPath: `asset://${sessionId}/capture-002`,
        thumbnailPath: `asset://${sessionId}/thumb-capture-002`,
        label: '두 번째 사진',
      },
    ],
  }
}

export type CaptureAdapter = ReturnType<typeof createCaptureAdapter>

export function createCaptureAdapter({
  convertFileSrcFn = convertFileSrc,
  invokeFn = invoke,
  isTauriFn = isTauri,
}: CaptureAdapterDependencies = {}) {
  const fallbackStore = new Map<string, SessionGallerySnapshot>()

  const getFallbackGallery = (sessionId: string) => {
    const existingSnapshot = fallbackStore.get(sessionId)
    if (existingSnapshot) {
      return existingSnapshot
    }

    const createdSnapshot = createFallbackGallery(sessionId)
    fallbackStore.set(sessionId, createdSnapshot)
    return createdSnapshot
  }

  return {
    async loadSessionGallery(request: SessionGalleryRequest): Promise<SessionGallerySnapshot> {
      const parsedRequest = sessionGalleryRequestSchema.parse(request)

      if (!isTauriFn()) {
        return ensureSessionScopedGallery(
          parsedRequest.sessionId,
          getFallbackGallery(parsedRequest.sessionId),
          parsedRequest.manifestPath,
        )
      }

      const response = await invokeFn('load_session_gallery', { request: parsedRequest })
      return normalizeSessionScopedGallery(
        parsedRequest.sessionId,
        sessionGallerySnapshotSchema.parse(response),
        convertFileSrcFn,
        parsedRequest.manifestPath,
      )
    },

    async deleteSessionPhoto(request: DeleteSessionPhotoRequest): Promise<DeleteSessionPhotoResponse> {
      const parsedRequest = deleteSessionPhotoRequestSchema.parse(request)

      if (!parsedRequest.manifestPath) {
        throw new Error('manifestPath is required to delete a session photo')
      }

      if (!isTauriFn()) {
        const currentSnapshot = getFallbackGallery(parsedRequest.sessionId)
        const nextItems = currentSnapshot.items.filter((item) => item.captureId !== parsedRequest.captureId)
        if (nextItems.length === currentSnapshot.items.length) {
          throw new Error(`capture not found for session: ${parsedRequest.captureId}`)
        }
        const latestCaptureId = nextItems.at(-1)?.captureId ?? null
        const nextSnapshot: SessionGallerySnapshot = {
          ...currentSnapshot,
          latestCaptureId,
          selectedCaptureId: latestCaptureId ?? nextItems[0]?.captureId ?? null,
          items: nextItems.map((item, index) => ({
            ...item,
            displayOrder: index,
            isLatest: item.captureId === latestCaptureId,
          })),
        }
        const response = {
          schemaVersion: schemaVersions.contract,
          deletedCaptureId: parsedRequest.captureId,
          confirmationMessage: '사진이 삭제되었습니다.',
          gallery: nextSnapshot,
        } satisfies DeleteSessionPhotoResponse

        fallbackStore.set(parsedRequest.sessionId, nextSnapshot)
        return ensureSessionScopedDeleteResponse(parsedRequest.sessionId, response, parsedRequest.manifestPath)
      }

      const response = await invokeFn('delete_session_photo', {
        request: parsedRequest,
      })
      const parsedResponse = deleteSessionPhotoResponseSchema.parse(response)
      const sessionScopedGallery = normalizeSessionScopedGallery(
        parsedRequest.sessionId,
        parsedResponse.gallery,
        convertFileSrcFn,
        parsedRequest.manifestPath,
      )

      return ensureSessionScopedDeleteResponse(
        parsedRequest.sessionId,
        {
          ...parsedResponse,
          gallery: sessionScopedGallery,
        },
      )
    },
  }
}

export const captureAdapter = createCaptureAdapter()
