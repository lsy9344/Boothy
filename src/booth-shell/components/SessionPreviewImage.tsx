import { useEffect, useState } from 'react'

import { resolvePresetPreviewSrc } from './preset-preview-src'

type SessionPreviewImageProps = {
  alt: string
  assetPath: string
}

function isSvgAssetPath(assetPath: string) {
  return assetPath.toLowerCase().endsWith('.svg')
}

export function SessionPreviewImage({
  alt,
  assetPath,
}: SessionPreviewImageProps) {
  const directSrc = resolvePresetPreviewSrc(assetPath)
  const [src, setSrc] = useState(directSrc)

  useEffect(() => {
    let isDisposed = false

    setSrc(directSrc)

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

  return <img src={src} alt={alt} />
}
