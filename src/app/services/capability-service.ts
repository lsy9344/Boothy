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

const SURFACE_WINDOW_LABELS: Partial<Record<SurfaceCapability, readonly string[]>> = {
  operator: ['operator-window'],
  authoring: ['authoring-window'],
  settings: ['operator-window', 'authoring-window'],
}

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

    const requiredWindowLabels = SURFACE_WINDOW_LABELS[surface]

    if (
      requiredWindowLabels !== undefined &&
      this.currentWindowLabel !== null &&
      !requiredWindowLabels.includes(this.currentWindowLabel)
    ) {
      return false
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
