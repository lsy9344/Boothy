import {
  type CaptureRuntimeService,
} from '../capture-adapter/services/capture-runtime'
import {
  Navigate,
  createBrowserRouter,
  type RouteObject,
} from 'react-router-dom'

import { PresetLibraryScreen } from '../preset-authoring/screens/PresetLibraryScreen'
import type { PresetCatalogService } from '../preset-catalog/services/preset-catalog-service'
import { SessionStartScreen } from '../booth-shell/screens/SessionStartScreen'
import { OperatorSummaryScreen } from '../operator-console/screens/OperatorSummaryScreen'
import { SettingsScreen } from '../settings/screens/SettingsScreen'
import type { ActivePresetService } from '../session-domain/services/active-preset'
import type { StartSessionService } from '../session-domain/services/start-session'
import { App } from './App'
import { SurfaceAccessGuard } from './guards/surface-access-guard'
import {
  AppProviders,
} from './providers/app-providers'
import {
  createCapabilityService,
  type CapabilityService,
} from './services/capability-service'

type CreateAppRoutesOptions = {
  capabilityService?: CapabilityService
  sessionService?: StartSessionService
  presetCatalogService?: PresetCatalogService
  activePresetService?: ActivePresetService
  captureRuntimeService?: CaptureRuntimeService
}

export function createAppRoutes({
  capabilityService = createCapabilityService(),
  sessionService,
  presetCatalogService,
  activePresetService,
  captureRuntimeService,
}: CreateAppRoutesOptions = {}): RouteObject[] {
  return [
    {
      path: '/',
      element: (
        <AppProviders
          capabilityService={capabilityService}
          sessionService={sessionService}
          presetCatalogService={presetCatalogService}
          activePresetService={activePresetService}
          captureRuntimeService={captureRuntimeService}
        >
          <App />
        </AppProviders>
      ),
      children: [
        {
          index: true,
          element: <Navigate replace to="/booth" />,
        },
        {
          path: 'booth',
          element: <SessionStartScreen />,
        },
        {
          element: <SurfaceAccessGuard surface="operator" />,
          children: [
            {
              path: 'operator',
              element: <OperatorSummaryScreen />,
            },
          ],
        },
        {
          element: <SurfaceAccessGuard surface="authoring" />,
          children: [
            {
              path: 'authoring',
              element: <PresetLibraryScreen />,
            },
          ],
        },
        {
          element: <SurfaceAccessGuard surface="settings" />,
          children: [
            {
              path: 'settings',
              element: <SettingsScreen />,
            },
          ],
        },
      ],
    },
  ]
}

export function createBrowserAppRouter(options?: CreateAppRoutesOptions) {
  return createBrowserRouter(createAppRoutes(options))
}
