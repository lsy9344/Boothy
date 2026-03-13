import type { SessionGallerySnapshot } from '../../shared-contracts/dto/sessionGallery.js'

export function selectReviewRailPhoto(
  gallery: SessionGallerySnapshot | null,
  selectedCaptureId: string | null,
) {
  if (!gallery) {
    return null
  }

  const selectedItem =
    gallery.items.find((item) => item.captureId === selectedCaptureId) ??
    gallery.items.find((item) => item.captureId === gallery.selectedCaptureId) ??
    gallery.items.at(0) ??
    null

  if (!selectedItem) {
    return null
  }

  return {
    captureId: selectedItem.captureId,
    label: selectedItem.label,
    previewPath: selectedItem.previewPath,
  }
}

export function selectReviewRailThumbnails(gallery: SessionGallerySnapshot | null) {
  if (!gallery) {
    return []
  }

  return gallery.items.map((item) => ({
    captureId: item.captureId,
    label: item.label,
    thumbnailPath: item.thumbnailPath,
    isLatest: item.isLatest,
  }))
}
