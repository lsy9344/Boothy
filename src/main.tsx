import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { RouterProvider } from 'react-router-dom'

import { resolveBootCapabilityService } from './app/boot/resolve-capability-service'
import { createBrowserAppRouter } from './app/routes'
import { createDefaultRuntimeCapabilityGateway } from './session-domain/services/runtime-capability-gateway'
import { isTauriRuntime } from './shared/runtime/is-tauri'
import './index.css'

async function readCurrentWindowLabel() {
  if (!isTauriRuntime()) {
    return null
  }

  const { getCurrentWindow } = await import('@tauri-apps/api/window')

  return getCurrentWindow().label
}

function alignInitialRouteToWindow(windowLabel: string | null) {
  if (typeof window === 'undefined' || windowLabel === null) {
    return
  }

  const currentPath = window.location.pathname

  if (windowLabel === 'viewer-window' && currentPath !== '/viewer') {
    window.history.replaceState({}, '', '/viewer')
    return
  }

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
    (currentPath === '/authoring' ||
      currentPath === '/operator' ||
      currentPath === '/viewer')
  ) {
    window.history.replaceState({}, '', '/booth')
  }
}

async function bootstrap() {
  const currentWindowLabel = await readCurrentWindowLabel()
  alignInitialRouteToWindow(currentWindowLabel)

  const capabilityService = await resolveBootCapabilityService({
    currentWindowLabel,
    readSnapshot: () => createDefaultRuntimeCapabilityGateway().readSnapshot(),
  })

  const router = createBrowserAppRouter({ capabilityService })

  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <RouterProvider router={router} />
    </StrictMode>,
  )
}

void bootstrap()
