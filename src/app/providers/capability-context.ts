import { createContext } from 'react'

import type { CapabilityService } from '../services/capability-service'

export const CapabilityContext = createContext<CapabilityService | null>(null)
