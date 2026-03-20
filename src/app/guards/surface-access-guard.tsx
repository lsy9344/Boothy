import { Navigate, Outlet } from 'react-router-dom'

import type { SurfaceCapability } from '../../shared-contracts'
import { useCapabilityService } from '../providers/use-capability-service'

type SurfaceAccessGuardProps = {
  surface: SurfaceCapability
}

export function SurfaceAccessGuard({ surface }: SurfaceAccessGuardProps) {
  const capabilityService = useCapabilityService()

  if (!capabilityService.canAccess(surface)) {
    return <Navigate replace to="/booth" />
  }

  return <Outlet />
}
