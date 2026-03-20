import { useContext } from 'react'

import { createCapabilityService } from '../services/capability-service'
import { CapabilityContext } from './capability-context'

export function useCapabilityService() {
  return useContext(CapabilityContext) ?? createCapabilityService()
}
