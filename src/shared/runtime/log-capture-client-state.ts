import { isTauriRuntime } from './is-tauri'

type CaptureClientDebugLogInput = {
  label: string
  sessionId?: string
  runtimeMode?: string
  customerState?: string
  reasonCode?: string
  canCapture?: boolean
  message?: string
}

export async function logCaptureClientState(
  input: CaptureClientDebugLogInput,
): Promise<void> {
  if (!isTauriRuntime()) {
    return
  }

  try {
    const { invoke } = await import('@tauri-apps/api/core')

    await invoke('log_capture_client_state', {
      input,
    })
  } catch {
    // Debug logging must never block capture flow updates.
  }
}
