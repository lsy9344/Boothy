import type { PublishedPresetSummary, SessionManifest } from '../../shared-contracts'
import { isSessionScopedAssetPath } from '../utils/session-scoped-asset-path'

export type CurrentSessionPreview = {
  captureId: string
  requestId: string
  assetPath: string
  activePresetId: string | null
  activePresetVersion: string
  presetDisplayName: string | null
  isCurrentActivePreset: boolean
  postEndState: SessionManifest['captures'][number]['postEndState']
  readyAtMs: number | null
  isLatest: boolean
}

function compareCurrentSessionCaptureRecency(
  left: NonNullable<SessionManifest['captures'][number]>,
  right: NonNullable<SessionManifest['captures'][number]>,
) {
  const acknowledgedDelta =
    right.timing.captureAcknowledgedAtMs - left.timing.captureAcknowledgedAtMs

  if (acknowledgedDelta !== 0) {
    return acknowledgedDelta
  }

  const persistedDelta = right.raw.persistedAtMs - left.raw.persistedAtMs

  if (persistedDelta !== 0) {
    return persistedDelta
  }

  const enqueuedDelta =
    (right.preview.enqueuedAtMs ?? -1) - (left.preview.enqueuedAtMs ?? -1)

  if (enqueuedDelta !== 0) {
    return enqueuedDelta
  }

  return right.captureId.localeCompare(left.captureId)
}

function hasSessionScopedPreviewAsset(
  sessionId: string,
  capture: NonNullable<SessionManifest['captures'][number]>,
) {
  return (
    capture.preview.assetPath !== null &&
    isSessionScopedAssetPath(sessionId, capture.preview.assetPath)
  )
}

function isVisibleCurrentSessionPreview(
  sessionId: string,
  capture: NonNullable<SessionManifest['captures'][number]>,
) {
  return (
    capture.renderStatus === 'previewReady' ||
    capture.renderStatus === 'finalReady'
  ) &&
    hasSessionScopedPreviewAsset(sessionId, capture) &&
    capture.preview.readyAtMs !== null
}

function isDisplayablePendingCurrentSessionPreview(
  sessionId: string,
  capture: NonNullable<SessionManifest['captures'][number]>,
) {
  return (
    (capture.renderStatus === 'captureSaved' ||
      capture.renderStatus === 'previewWaiting') &&
    hasSessionScopedPreviewAsset(sessionId, capture) &&
    capture.preview.readyAtMs === null
  )
}

function isPendingCurrentSessionPreview(
  capture: NonNullable<SessionManifest['captures'][number]>,
) {
  return (
    (capture.renderStatus === 'captureSaved' ||
      capture.renderStatus === 'previewWaiting') &&
    capture.preview.readyAtMs === null
  )
}

export function selectCurrentSessionPreviews(
  manifest: SessionManifest | null,
  presetCatalog: PublishedPresetSummary[] = [],
): CurrentSessionPreview[] {
  if (manifest === null) {
    return []
  }

  const currentSessionCaptures = manifest.captures.filter(
    (capture): capture is NonNullable<SessionManifest['captures'][number]> =>
      capture !== undefined && capture.sessionId === manifest.sessionId,
  )
  let latestVisibleCaptureId: string | null = null

  for (const capture of currentSessionCaptures.toSorted(
    compareCurrentSessionCaptureRecency,
  )) {
    if (isVisibleCurrentSessionPreview(manifest.sessionId, capture)) {
      latestVisibleCaptureId = capture.captureId
      break
    }

    if (isDisplayablePendingCurrentSessionPreview(manifest.sessionId, capture)) {
      latestVisibleCaptureId = capture.captureId
      break
    }

    if (isPendingCurrentSessionPreview(capture)) {
      break
    }
  }

  return currentSessionCaptures
    .filter(
      (capture) =>
        isVisibleCurrentSessionPreview(manifest.sessionId, capture) ||
        isDisplayablePendingCurrentSessionPreview(manifest.sessionId, capture),
    )
    .toSorted(compareCurrentSessionCaptureRecency)
    .map((capture) => ({
      captureId: capture.captureId,
      requestId: capture.requestId,
      assetPath: capture.preview.assetPath!,
      activePresetId: capture.activePresetId ?? null,
      activePresetVersion: capture.activePresetVersion,
      presetDisplayName:
        capture.activePresetDisplayName ??
        (capture.activePresetId === undefined || capture.activePresetId === null
          ? null
          : presetCatalog.find(
              (preset) =>
                preset.presetId === capture.activePresetId &&
                preset.publishedVersion === capture.activePresetVersion,
            )?.displayName) ??
        null,
      isCurrentActivePreset:
        capture.activePresetId !== undefined &&
        capture.activePresetId !== null &&
        manifest.activePreset?.presetId === capture.activePresetId &&
        manifest.activePreset.publishedVersion === capture.activePresetVersion,
      postEndState: capture.postEndState,
      readyAtMs: capture.preview.readyAtMs ?? null,
      isLatest: capture.captureId === latestVisibleCaptureId,
    }))
}
