import type { ReactNode } from 'react'

import { SessionProvider } from '../../session-domain/state/session-provider'
import type { StartSessionService } from '../../session-domain/services/start-session'
import { CapabilityProvider } from './capability-provider'
import { createCapabilityService, type CapabilityService } from '../services/capability-service'

type AppProvidersProps = {
  children: ReactNode
  capabilityService?: CapabilityService
  sessionService?: StartSessionService
}

export function AppProviders({
  children,
  capabilityService = createCapabilityService(),
  sessionService,
}: AppProvidersProps) {
  return (
    <CapabilityProvider capabilityService={capabilityService}>
      <SessionProvider sessionService={sessionService}>{children}</SessionProvider>
    </CapabilityProvider>
  )
}
