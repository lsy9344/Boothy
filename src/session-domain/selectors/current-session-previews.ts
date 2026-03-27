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

export function selectCurrentSessionPreviews(
  manifest: SessionManifest | null,
  presetCatalog: PublishedPresetSummary[] = [],
): CurrentSessionPreview[] {
  if (manifest === null) {
    return []
  }

  return manifest.captures
    .filter(
      (capture) =>
        capture !== undefined &&
        capture.sessionId === manifest.sessionId &&
        (capture.renderStatus === 'previewReady' ||
          capture.renderStatus === 'finalReady') &&
        capture.preview.assetPath !== null &&
        isSessionScopedAssetPath(manifest.sessionId, capture.preview.assetPath) &&
        capture.preview.readyAtMs !== null,
    )
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
      isLatest: index === 0,
    }))
}
