import { useEffect, useState } from 'react'

import { logCaptureClientState } from '../../shared/runtime/log-capture-client-state'
import { resolvePresetPreviewSrc } from './preset-preview-src'

type SessionPreviewImageProps = {
  alt: string
  assetPath: string
  captureId: string
  readyAtMs: number
  isLatest: boolean
}

function isSvgAssetPath(assetPath: string) {
  return assetPath.toLowerCase().endsWith('.svg')
}

export function SessionPreviewImage({
  alt,
  assetPath,
  captureId,
  readyAtMs,
  isLatest,
}: SessionPreviewImageProps) {
  const directSrc = resolvePresetPreviewSrc(assetPath)
  const [src, setSrc] = useState(directSrc)
  const [hasLoadError, setHasLoadError] = useState(false)
  const [hasReportedVisible, setHasReportedVisible] = useState(false)

  useEffect(() => {
    let isDisposed = false

    setSrc(directSrc)
    setHasLoadError(false)
    setHasReportedVisible(false)

    if (!isSvgAssetPath(assetPath)) {
      return () => {
        isDisposed = true
      }
    }

    void fetch(directSrc)
      .then(async (response) => {
        if (!response.ok) {
          throw new Error('svg-preview-unreadable')
        }

        return response.text()
      })
      .then((svgMarkup) => {
        if (isDisposed) {
          return
        }

        setSrc(`data:image/svg+xml;charset=utf-8,${encodeURIComponent(svgMarkup)}`)
      })
      .catch(() => {
        if (!isDisposed) {
          setSrc(directSrc)
        }
      })

    return () => {
      isDisposed = true
    }
  }, [
    assetPath,
    directSrc,
  ])

  if (hasLoadError) {
    return (
      <div role="img" aria-label={alt}>
        확인용 사진 준비 중
      </div>
    )
  }

  return (
    <img
      src={src}
      alt={alt}
      onLoad={() => {
        if (hasReportedVisible) {
          return
        }

        setHasReportedVisible(true)
        const uiLagMs = Math.max(0, Date.now() - readyAtMs)
        const sessionId =
          assetPath.match(/sessions[\\/](session_[^\\/]+)/i)?.[1] ?? undefined

        if (typeof console !== 'undefined') {
          console.info('[boothy][capture] current-session-preview-visible', {
            sessionId,
            captureId,
            readyAtMs,
            uiLagMs,
            isLatest,
          })
        }

        void logCaptureClientState({
          label: 'current-session-preview-visible',
          sessionId,
          message: `captureId=${captureId};uiLagMs=${uiLagMs};latest=${isLatest}`,
        })
      }}
      onError={() => {
        setHasLoadError(true)
      }}
    />
  )
}
