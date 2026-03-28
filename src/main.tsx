import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { RouterProvider } from 'react-router-dom'

import { createCapabilityService } from './app/services/capability-service'
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

async function bootstrap() {
  const currentWindowLabel = await readCurrentWindowLabel()
  alignInitialRouteToWindow(currentWindowLabel)

  let capabilityService = createCapabilityService({ currentWindowLabel })

  try {
    const capabilitySnapshot =
      await createDefaultRuntimeCapabilityGateway().readSnapshot()
    capabilityService = createCapabilityService({
      ...capabilitySnapshot,
      currentWindowLabel,
    })
  } catch {
    capabilityService = createCapabilityService({ currentWindowLabel })
  }

  const router = createBrowserAppRouter({ capabilityService })

  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <RouterProvider router={router} />
    </StrictMode>,
  )
}

void bootstrap()
