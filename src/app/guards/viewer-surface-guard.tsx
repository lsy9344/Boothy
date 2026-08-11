import { Outlet } from 'react-router-dom'

import { useCapabilityService } from '../providers/use-capability-service'

/**
 * 관람 화면 전용 가드.
 *
 * `SurfaceAccessGuard`를 쓰지 않는 이유: 접근이 거부되면 `/booth`로 리다이렉트하는데,
 * 그러면 고객 모니터에 부스 조작 화면이 뜬다. 관람 창에서는 리다이렉트 대신
 * customer-safe blank를 유지한다.
 */
export function ViewerSurfaceGuard() {
  const capabilityService = useCapabilityService()

  if (!capabilityService.canAccess('viewer')) {
    return <main className="viewer-surface viewer-surface--blank" />
  }

  return <Outlet />
}
