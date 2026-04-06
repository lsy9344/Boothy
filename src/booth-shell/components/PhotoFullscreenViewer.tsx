import { useEffect } from 'react'

import type { CurrentSessionPreview } from '../../session-domain/selectors'
import { SessionPreviewImage } from './SessionPreviewImage'

type PhotoFullscreenViewerProps = {
  preview: CurrentSessionPreview
  onClose(): void
}

export function PhotoFullscreenViewer({ preview, onClose }: PhotoFullscreenViewerProps) {
  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === 'Escape') {
        onClose()
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => {
      window.removeEventListener('keydown', handleKeyDown)
    }
  }, [onClose])

  const presetLabel =
    preview.presetDisplayName ??
    (preview.isCurrentActivePreset ? '현재 룩' : '이전 룩')

  return (
    <div
      className="photo-fullscreen-viewer"
      role="dialog"
      aria-modal="true"
      aria-label={`${presetLabel} 룩 전체 화면 보기`}
      onClick={onClose}
    >
      <div
        className="photo-fullscreen-viewer__panel"
        onClick={(e) => e.stopPropagation()}
      >
        {preview.isLatest ? (
          <span className="photo-fullscreen-viewer__badge">최신 사진</span>
        ) : null}
        <SessionPreviewImage
          assetPath={preview.assetPath}
          alt={`${presetLabel} 룩으로 찍은 사진 전체 화면`}
          captureId={preview.captureId}
          requestId={preview.requestId}
          readyAtMs={preview.readyAtMs}
          isLatest={preview.isLatest}
          prioritizeLoading
          visibilityLabelBase="fullscreen-viewer"
        />
        <p className="photo-fullscreen-viewer__label">{presetLabel} 룩</p>
        <button
          type="button"
          className="photo-fullscreen-viewer__close"
          onClick={onClose}
          aria-label="전체 화면 닫기"
        >
          닫기
        </button>
      </div>
    </div>
  )
}
