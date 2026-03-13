import {
  deleteSessionPhotoResponseSchema,
  sessionGallerySnapshotSchema,
  type DeleteSessionPhotoResponse,
  type SessionGallerySnapshot,
} from '../../shared-contracts/dto/sessionGallery.js'

export function ensureSessionScopedGallery(
  sessionId: string,
  snapshot: SessionGallerySnapshot,
  manifestPath?: string,
): SessionGallerySnapshot {
  const parsedSnapshot = sessionGallerySnapshotSchema.parse(snapshot)

  if (parsedSnapshot.sessionId !== sessionId) {
    throw new Error(`Gallery snapshot session mismatch: expected ${sessionId}`)
  }

  if (parsedSnapshot.items.some((item) => item.sessionId !== sessionId)) {
    throw new Error(`Gallery snapshot leaked a foreign-session thumbnail for ${sessionId}`)
  }

  if (manifestPath) {
    const sessionRoot = normalizeNativePath(manifestPath.replace(/\\/g, '/').replace(/\/session\.json$/i, ''))
    const processedRoot = normalizeNativePath(`${sessionRoot}/processed`)

    for (const item of parsedSnapshot.items) {
      validateSessionScopedAssetPath(item.previewPath, sessionRoot, processedRoot)
      validateSessionScopedAssetPath(item.thumbnailPath, sessionRoot, processedRoot)
    }
  }

  return parsedSnapshot
}

export function ensureSessionScopedDeleteResponse(
  sessionId: string,
  response: DeleteSessionPhotoResponse,
  manifestPath?: string,
): DeleteSessionPhotoResponse {
  const parsedResponse = deleteSessionPhotoResponseSchema.parse(response)

  return deleteSessionPhotoResponseSchema.parse({
    ...parsedResponse,
    gallery: ensureSessionScopedGallery(sessionId, parsedResponse.gallery, manifestPath),
  })
}

function validateSessionScopedAssetPath(path: string, sessionRoot: string, processedRoot: string) {
  if (/^(asset|blob|data|https?):/i.test(path)) {
    return
  }

  const normalizedPath = normalizeNativePath(path)

  if (!normalizedPath.startsWith(sessionRoot) || !normalizedPath.startsWith(processedRoot)) {
    throw new Error(`Gallery snapshot asset resolved outside the active session root: ${path}`)
  }
}

function normalizeNativePath(path: string) {
  const parts = path.replace(/\\/g, '/').split('/')
  const normalized: string[] = []

  for (const part of parts) {
    if (!part || part === '.') {
      continue
    }

    if (part === '..') {
      normalized.pop()
      continue
    }

    normalized.push(part)
  }

  return normalized.join('/')
}
