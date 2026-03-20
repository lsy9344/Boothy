import type { SessionManifest } from '../../shared-contracts'

export type CurrentSessionPreview = {
  captureId: string
  assetPath: string
  activePresetVersion: string
  readyAtMs: number
}

export function selectCurrentSessionPreviews(
  manifest: SessionManifest | null,
): CurrentSessionPreview[] {
  if (manifest === null) {
    return []
  }

  return manifest.captures
    .filter(
      (capture) =>
        capture !== undefined &&
        capture.sessionId === manifest.sessionId &&
        capture.renderStatus === 'previewReady' &&
        capture.preview.assetPath !== null &&
        capture.preview.readyAtMs !== null,
    )
    .toSorted((left, right) => right.preview.readyAtMs! - left.preview.readyAtMs!)
    .map((capture) => ({
      captureId: capture.captureId,
      assetPath: capture.preview.assetPath!,
      activePresetVersion: capture.activePresetVersion,
      readyAtMs: capture.preview.readyAtMs!,
    }))
}
