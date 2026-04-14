import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { RouterProvider } from 'react-router-dom'

import { AppErrorBoundary } from './app/components/AppErrorBoundary'
import { createBrowserAppRouter } from './app/routes'
import {
  createCapabilityService,
  type CapabilityService,
  type CapabilityServiceOptions,
} from './app/services/capability-service'
import { createDefaultRuntimeCapabilityGateway } from './session-domain/services/runtime-capability-gateway'
import { DEFAULT_CAPABILITY_SNAPSHOT } from './shared-contracts'
import { isTauriRuntime } from './shared/runtime/is-tauri'

type AppRouter = ReturnType<typeof createBrowserAppRouter>

const DEFAULT_BOOTSTRAP_TIMEOUT_MS = 1_500

async function reportBootstrapError(error: Error) {
  if (!isTauriRuntime()) {
    console.error(error)
    return
  }

  try {
    const { invoke } = await import('@tauri-apps/api/core')

    await invoke('log_capture_client_state', {
      input: {
        label: 'bootstrap-error',
        message: error.stack ?? error.message,
      },
    })
  } catch {
    console.error(error)
  }
}

async function readCurrentWindowLabel() {
  if (!isTauriRuntime()) {
    return null
  }

  const { getCurrentWindow } = await import('@tauri-apps/api/window')

  return getCurrentWindow().label
}

type TauriWindowMetadata = {
  currentWindow?: {
    label?: unknown
  }
}

function readCurrentWindowLabelFromTauriMetadata() {
  if (typeof window === 'undefined') {
    return null
  }

  const candidate = window as typeof window & {
    __TAURI_INTERNALS__?: {
      metadata?: TauriWindowMetadata
    }
  }
  const label = candidate.__TAURI_INTERNALS__?.metadata?.currentWindow?.label

  return typeof label === 'string' && label.trim().length > 0 ? label : null
}

function createFallbackCapabilityOptions(
  currentWindowLabel: string | null,
): CapabilityServiceOptions {
  if (currentWindowLabel === 'operator-window') {
    return {
      currentWindowLabel,
      isAdminAuthenticated: true,
      allowedSurfaces: ['booth', 'operator', 'settings'],
    }
  }

  if (currentWindowLabel === 'authoring-window') {
    return {
      currentWindowLabel,
      isAdminAuthenticated: true,
      allowedSurfaces: ['booth', 'operator', 'authoring', 'settings'],
    }
  }

  return {
    ...DEFAULT_CAPABILITY_SNAPSHOT,
    currentWindowLabel,
  }
}

async function withTimeout<T>(
  promise: Promise<T>,
  timeoutMs: number,
  label: string,
) {
  let timeoutId: ReturnType<typeof setTimeout> | undefined

  try {
    return await Promise.race([
      promise,
      new Promise<T>((_, reject) => {
        timeoutId = setTimeout(() => {
          reject(new Error(`${label}-timed-out`))
        }, timeoutMs)
      }),
    ])
  } finally {
    if (timeoutId !== undefined) {
      clearTimeout(timeoutId)
    }
  }
}

export function alignInitialRouteToWindow(windowLabel: string | null) {
  if (typeof window === 'undefined' || windowLabel === null) {
    return
  }

  const currentPath = window.location.pathname

  if (windowLabel === 'operator-window' && currentPath !== '/operator') {
    window.history.replaceState({}, '', '/operator')
    return
  }

  if (windowLabel === 'authoring-window' && currentPath !== '/authoring') {
    window.history.replaceState({}, '', '/authoring')
    return
  }

  if (
    windowLabel === 'booth-window' &&
    (currentPath === '/authoring' || currentPath === '/operator')
  ) {
    window.history.replaceState({}, '', '/booth')
  }
}

function renderApp(router: AppRouter) {
  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <AppErrorBoundary onError={(error) => void reportBootstrapError(error)}>
        <RouterProvider router={router} />
      </AppErrorBoundary>
    </StrictMode>,
  )
}

type CreateCapabilityService = (
  options: CapabilityServiceOptions,
) => CapabilityService

type CreateAppRouter = (options: {
  capabilityService: CapabilityService
}) => AppRouter

type BootstrapAppOptions = {
  readCurrentWindowLabel?: () => Promise<string | null>
  readCapabilitySnapshot?: () => Promise<CapabilityServiceOptions>
  createCapabilityService?: CreateCapabilityService
  createAppRouter?: CreateAppRouter
  renderApp?: (router: AppRouter) => void
  bootstrapTimeoutMs?: number
}

export async function bootstrapApp({
  readCurrentWindowLabel: readCurrentWindowLabelOverride = readCurrentWindowLabel,
  readCapabilitySnapshot = () =>
    createDefaultRuntimeCapabilityGateway().readSnapshot(),
  createCapabilityService: createCapabilityServiceOverride = createCapabilityService,
  createAppRouter = ({ capabilityService }) =>
    createBrowserAppRouter({ capabilityService }),
  renderApp: renderAppOverride = renderApp,
  bootstrapTimeoutMs = DEFAULT_BOOTSTRAP_TIMEOUT_MS,
}: BootstrapAppOptions = {}) {
  let currentWindowLabel = readCurrentWindowLabelFromTauriMetadata()

  if (currentWindowLabel === null) {
    try {
      currentWindowLabel = await withTimeout(
        readCurrentWindowLabelOverride(),
        bootstrapTimeoutMs,
        'window-label',
      )
    } catch {
      currentWindowLabel = null
    }
  }

  alignInitialRouteToWindow(currentWindowLabel)

  let capabilityService = createCapabilityServiceOverride(
    createFallbackCapabilityOptions(currentWindowLabel),
  )

  try {
    const capabilitySnapshot = await withTimeout(
      readCapabilitySnapshot(),
      bootstrapTimeoutMs,
      'capability-snapshot',
    )
    capabilityService = createCapabilityServiceOverride({
      ...capabilitySnapshot,
      currentWindowLabel,
    })
  } catch {
    capabilityService = createCapabilityServiceOverride(
      createFallbackCapabilityOptions(currentWindowLabel),
    )
  }

  const router = createAppRouter({ capabilityService })
  renderAppOverride(router)

  return {
    currentWindowLabel,
    capabilityService,
    router,
  }
}
