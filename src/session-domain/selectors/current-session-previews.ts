import type { PublishedPresetSummary, SessionManifest } from '../../shared-contracts'
import { isSessionScopedAssetPath } from '../utils/session-scoped-asset-path'

export type CurrentSessionPreview = {
  captureId: string
  assetPath: string
  activePresetId: string | null
  activePresetVersion: string
  presetDisplayName: string | null
  isCurrentActivePreset: boolean
  postEndState: SessionManifest['captures'][number]['postEndState']
  readyAtMs: number
  isLatest: boolean
}

function compareCurrentSessionPreviewOrder(
  left: NonNullable<SessionManifest['captures'][number]>,
  right: NonNullable<SessionManifest['captures'][number]>,
) {
  const readyDelta = right.preview.readyAtMs! - left.preview.readyAtMs!

  if (readyDelta !== 0) {
    return readyDelta
  }

  const enqueuedDelta =
    (right.preview.enqueuedAtMs ?? -1) - (left.preview.enqueuedAtMs ?? -1)

  if (enqueuedDelta !== 0) {
    return enqueuedDelta
  }

  const persistedDelta = right.raw.persistedAtMs - left.raw.persistedAtMs

  if (persistedDelta !== 0) {
    return persistedDelta
  }

  return right.captureId.localeCompare(left.captureId)
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

function isVisibleCurrentSessionPreview(
  sessionId: string,
  capture: NonNullable<SessionManifest['captures'][number]>,
) {
  return (
    capture.renderStatus === 'previewReady' ||
    capture.renderStatus === 'finalReady'
  ) &&
    capture.preview.assetPath !== null &&
    isSessionScopedAssetPath(sessionId, capture.preview.assetPath) &&
    capture.preview.readyAtMs !== null
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

    if (isPendingCurrentSessionPreview(capture)) {
      break
    }
  }

  return currentSessionCaptures
    .filter((capture) => isVisibleCurrentSessionPreview(manifest.sessionId, capture))
    .toSorted(compareCurrentSessionPreviewOrder)
    .map((capture, index) => ({
      captureId: capture.captureId,
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
      readyAtMs: capture.preview.readyAtMs!,
      isLatest: capture.captureId === latestVisibleCaptureId,
    }))
}
