import { convertFileSrc } from '@tauri-apps/api/core'

type ResolvePresetPreviewSrcOptions = {
  convertFileSrcFn?: typeof convertFileSrc
  isTauriRuntime?: boolean
}

function detectTauriRuntime() {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

function isAbsoluteFilesystemPath(assetPath: string) {
  return /^[a-zA-Z]:[\\/]/.test(assetPath) || assetPath.startsWith('/')
}

export function resolvePresetPreviewSrc(
  assetPath: string,
  {
    convertFileSrcFn = convertFileSrc,
    isTauriRuntime = detectTauriRuntime(),
  }: ResolvePresetPreviewSrcOptions = {},
) {
  if (!isTauriRuntime || !isAbsoluteFilesystemPath(assetPath)) {
    return assetPath
  }

  return convertFileSrcFn(assetPath)
}
