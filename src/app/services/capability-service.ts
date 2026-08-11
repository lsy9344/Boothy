import {
  DEFAULT_CAPABILITY_SNAPSHOT,
  capabilitySnapshotSchema,
  type CapabilitySnapshot,
  type SurfaceCapability,
} from '../../shared-contracts'

export type CapabilitySnapshotInput = Partial<CapabilitySnapshot>
export type CapabilityServiceOptions = CapabilitySnapshotInput & {
  currentWindowLabel?: string | null
}

export interface CapabilityService {
  canAccess(surface: SurfaceCapability): boolean
  getSnapshot(): CapabilitySnapshot
}

const SURFACE_WINDOW_LABELS: Partial<Record<SurfaceCapability, string>> = {
  viewer: 'viewer-window',
  operator: 'operator-window',
  authoring: 'authoring-window',
}

/**
 * 고객 surface는 관리자 인증을 요구하지 않는다. 창 label로만 경계를 잡는다.
 * privileged surface(operator/authoring/settings)는 계속 인증을 요구한다.
 */
const CUSTOMER_SURFACES: readonly SurfaceCapability[] = ['booth', 'viewer']

class StaticCapabilityService implements CapabilityService {
  private readonly snapshot: CapabilitySnapshot
  private readonly currentWindowLabel: string | null

  constructor(snapshot: CapabilitySnapshot, currentWindowLabel: string | null) {
    this.snapshot = snapshot
    this.currentWindowLabel = currentWindowLabel
  }

  canAccess(surface: SurfaceCapability) {
    if (surface === 'booth') {
      return true
    }

    if (surface === 'viewer') {
      return this.currentWindowLabel === SURFACE_WINDOW_LABELS.viewer
    }

    const requiredWindowLabel = SURFACE_WINDOW_LABELS[surface]

    if (
      requiredWindowLabel !== undefined &&
      this.currentWindowLabel !== null &&
      this.currentWindowLabel !== requiredWindowLabel
    ) {
      return false
    }

    if (CUSTOMER_SURFACES.includes(surface)) {
      return true
    }

    return (
      this.snapshot.isAdminAuthenticated &&
      this.snapshot.allowedSurfaces.includes(surface)
    )
  }

  getSnapshot() {
    return this.snapshot
  }
}

export function createCapabilityService(
  options: CapabilityServiceOptions = DEFAULT_CAPABILITY_SNAPSHOT,
) {
  const {
    currentWindowLabel = null,
    ...snapshotInput
  } = options
  const snapshot = capabilitySnapshotSchema.parse({
    ...DEFAULT_CAPABILITY_SNAPSHOT,
    ...snapshotInput,
  })

  return new StaticCapabilityService(snapshot, currentWindowLabel)
}
