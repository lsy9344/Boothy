import { useEffect, useState } from 'react'

import { logCaptureClientState } from '../../shared/runtime/log-capture-client-state'
import { resolvePresetPreviewSrc } from './preset-preview-src'

type SessionPreviewImageProps = {
  alt: string
  assetPath: string
  captureId: string
  readyAtMs: number | null
  isLatest: boolean
}

function isSvgAssetPath(assetPath: string) {
  return assetPath.toLowerCase().endsWith('.svg')
}

function isAbsoluteFilesystemPath(assetPath: string) {
  return /^[a-zA-Z]:[\\/]/.test(assetPath) || assetPath.startsWith('/')
}

function withCacheBuster(
  src: string,
  assetPath: string,
  readyAtMs: number | null,
) {
  if (
    readyAtMs === null ||
    src.startsWith('data:') ||
    !isAbsoluteFilesystemPath(assetPath)
  ) {
    return src
  }

  const separator = src.includes('?') ? '&' : '?'

  return `${src}${separator}v=${readyAtMs}`
}

export function SessionPreviewImage({
  alt,
  assetPath,
  captureId,
  readyAtMs,
  isLatest,
}: SessionPreviewImageProps) {
  const directSrc = resolvePresetPreviewSrc(assetPath)
  const [svgSrc, setSvgSrc] = useState<string | null>(null)
  const [hasLoadError, setHasLoadError] = useState(false)
  const [hasReportedVisible, setHasReportedVisible] = useState(false)

  useEffect(() => {
    let isDisposed = false

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

        setSvgSrc(`data:image/svg+xml;charset=utf-8,${encodeURIComponent(svgMarkup)}`)
      })
      .catch(() => {
        if (!isDisposed) {
          setSvgSrc(null)
        }
      })

    return () => {
      isDisposed = true
    }
  }, [
    assetPath,
    directSrc,
  ])

  const src = withCacheBuster(svgSrc ?? directSrc, assetPath, readyAtMs)

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
        const isPendingPreview = readyAtMs === null
        const uiLagMs = isPendingPreview ? null : Math.max(0, Date.now() - readyAtMs)
        const sessionId =
          assetPath.match(/sessions[\\/](session_[^\\/]+)/i)?.[1] ?? undefined
        const visibilityLabel = isPendingPreview
          ? 'current-session-preview-pending-visible'
          : 'current-session-preview-visible'

        if (typeof console !== 'undefined') {
          console.info(`[boothy][capture] ${visibilityLabel}`, {
            sessionId,
            captureId,
            readyAtMs,
            uiLagMs,
            isLatest,
          })
        }

        void logCaptureClientState({
          label: visibilityLabel,
          sessionId,
          message: `captureId=${captureId};uiLagMs=${uiLagMs ?? 'pending'};latest=${isLatest}`,
        })
      }}
      onError={() => {
        setHasLoadError(true)
      }}
    />
  )
}
