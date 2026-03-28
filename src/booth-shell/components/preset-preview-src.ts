import { convertFileSrc } from '@tauri-apps/api/core'
import { isTauriRuntime as detectTauriRuntime } from '../../shared/runtime/is-tauri'

type ResolvePresetPreviewSrcOptions = {
  convertFileSrcFn?: typeof convertFileSrc
  isTauriRuntime?: boolean
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
