import type { ReactNode } from 'react'

import { CapabilityContext } from './capability-context'
import type { CapabilityService } from '../services/capability-service'

type CapabilityProviderProps = {
  children: ReactNode
  capabilityService: CapabilityService
}

export function CapabilityProvider({
  children,
  capabilityService,
}: CapabilityProviderProps) {
  return (
    <CapabilityContext.Provider value={capabilityService}>
      {children}
    </CapabilityContext.Provider>
  )
}
