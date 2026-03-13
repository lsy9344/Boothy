import {
  captureConfidenceSnapshotSchema,
  type CaptureConfidenceSnapshot,
  type LatestPhotoState,
  type LatestSessionPhoto,
} from '../../shared-contracts/dto/captureConfidence.js'

function isSessionPhoto(value: LatestPhotoState): value is Extract<LatestPhotoState, { kind: 'ready' }> {
  return value.kind === 'ready'
}

function sanitizePhoto(sessionId: string, photo: LatestSessionPhoto): LatestSessionPhoto | null {
  return photo.sessionId === sessionId ? photo : null
}

function sanitizeLatestPhoto(
  sessionId: string,
  latestPhoto: LatestPhotoState,
): LatestPhotoState {
  if (latestPhoto.kind === 'ready') {
    const sanitizedPhoto = sanitizePhoto(sessionId, latestPhoto.photo)
    if (!sanitizedPhoto) {
      return {
        kind: 'empty',
      }
    }

    return {
      kind: 'ready',
      photo: sanitizedPhoto,
    }
  }

  if (latestPhoto.kind === 'updating') {
    const preview = latestPhoto.preview ? sanitizePhoto(sessionId, latestPhoto.preview) : null

    return {
      kind: 'updating',
      nextCaptureId: latestPhoto.nextCaptureId,
      ...(preview ? { preview } : {}),
    }
  }

  return latestPhoto
}

function shouldPreserveCurrentLatestPhoto(
  current: CaptureConfidenceSnapshot | null,
  incoming: CaptureConfidenceSnapshot,
  nextLatestPhoto: LatestPhotoState,
) {
  if (!current || !isSessionPhoto(current.latestPhoto)) {
    return false
  }

  if (nextLatestPhoto.kind === 'updating' && !nextLatestPhoto.preview) {
    return true
  }

  if (nextLatestPhoto.kind !== 'empty') {
    return false
  }

  if (incoming.latestPhoto.kind === 'ready') {
    return incoming.latestPhoto.photo.sessionId !== incoming.sessionId
  }

  return incoming.latestPhoto.kind === 'updating'
    ? Boolean(incoming.latestPhoto.preview?.sessionId !== incoming.sessionId)
    : false
}

export function mergeCaptureConfidenceState(
  current: CaptureConfidenceSnapshot | null,
  incoming: CaptureConfidenceSnapshot,
): CaptureConfidenceSnapshot {
  const parsedIncoming = captureConfidenceSnapshotSchema.parse(incoming)

  if (current) {
    if (parsedIncoming.sessionId !== current.sessionId) {
      return current
    }

    if (parsedIncoming.revision <= current.revision) {
      return current
    }
  }

  const nextLatestPhoto = sanitizeLatestPhoto(parsedIncoming.sessionId, parsedIncoming.latestPhoto)
  if (current && shouldPreserveCurrentLatestPhoto(current, parsedIncoming, nextLatestPhoto)) {
    return {
      ...parsedIncoming,
      latestPhoto: current.latestPhoto,
    }
  }

  return {
    ...parsedIncoming,
    latestPhoto: nextLatestPhoto,
  }
}
