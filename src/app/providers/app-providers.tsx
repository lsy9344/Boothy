import type { ReactNode } from 'react'

import type { CaptureRuntimeService } from '../../capture-adapter/services/capture-runtime'
import type { PresetCatalogService } from '../../preset-catalog/services/preset-catalog-service'
import type { ActivePresetService } from '../../session-domain/services/active-preset'
import { SessionProvider } from '../../session-domain/state/session-provider'
import type { StartSessionService } from '../../session-domain/services/start-session'
import { CapabilityProvider } from './capability-provider'
import { createCapabilityService, type CapabilityService } from '../services/capability-service'

type AppProvidersProps = {
  children: ReactNode
  capabilityService?: CapabilityService
  sessionService?: StartSessionService
  presetCatalogService?: PresetCatalogService
  activePresetService?: ActivePresetService
  captureRuntimeService?: CaptureRuntimeService
}

export function AppProviders({
  children,
  capabilityService = createCapabilityService(),
  sessionService,
  presetCatalogService,
  activePresetService,
  captureRuntimeService,
}: AppProvidersProps) {
  return (
    <CapabilityProvider capabilityService={capabilityService}>
      <SessionProvider
        sessionService={sessionService}
        presetCatalogService={presetCatalogService}
        activePresetService={activePresetService}
        captureRuntimeService={captureRuntimeService}
      >
        {children}
      </SessionProvider>
    </CapabilityProvider>
  )
}
