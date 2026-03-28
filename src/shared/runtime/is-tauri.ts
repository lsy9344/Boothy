export function isTauriRuntime() {
  if (typeof window === 'undefined') {
    return false
  }

  const candidate = window as typeof window & {
    __TAURI_INTERNALS__?: unknown
    __TAURI__?: unknown
    __TAURI_IPC__?: unknown
  }

  return (
    '__TAURI_INTERNALS__' in candidate ||
    '__TAURI__' in candidate ||
    '__TAURI_IPC__' in candidate ||
    (typeof navigator !== 'undefined' &&
      /\btauri\b/i.test(navigator.userAgent ?? ''))
  )
}
