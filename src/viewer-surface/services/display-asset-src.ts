import { convertFileSrc } from '@tauri-apps/api/core'

import { isTauriRuntime as detectTauriRuntime } from '../../shared/runtime/is-tauri'

type ResolveDisplayAssetSrcOptions = {
  convertFileSrcFn?: typeof convertFileSrc
  isTauriRuntime?: boolean
}

function isAbsoluteFilesystemPath(assetPath: string) {
  return /^[a-zA-Z]:[\\/]/.test(assetPath) || assetPath.startsWith('/')
}

/**
 * immutable generation 경로를 WebView가 읽을 수 있는 asset URL로 바꾼다.
 *
 * **cache buster(`?v=`)를 붙이지 않는다.** generation 경로는 이미 유일하므로 query string은
 * 불필요한 재요청만 만든다. (`SessionPreviewImage`의 `withCacheBuster`는 하나의 canonical
 * 파일을 덮어쓰던 Story 1.9 계보의 보정책이며 이 경로에는 해당하지 않는다.)
 *
 * Windows에서 이 URL의 origin은 `http://asset.localhost`이고 앱 문서는 `http://tauri.localhost`다.
 * 즉 cross-origin이므로 Element Timing `renderTime`이 0이 될 수 있다 (`after-paint.ts` 참조).
 */
export function resolveDisplayAssetSrc(
  assetPath: string,
  {
    convertFileSrcFn = convertFileSrc,
    isTauriRuntime = detectTauriRuntime(),
  }: ResolveDisplayAssetSrcOptions = {},
) {
  if (assetPath === '' || !isTauriRuntime || !isAbsoluteFilesystemPath(assetPath)) {
    return assetPath
  }

  return convertFileSrcFn(assetPath)
}
