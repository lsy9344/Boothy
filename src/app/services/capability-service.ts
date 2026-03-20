import {
  DEFAULT_CAPABILITY_SNAPSHOT,
  capabilitySnapshotSchema,
  type CapabilitySnapshot,
  type SurfaceCapability,
} from '../../shared-contracts'

export type CapabilitySnapshotInput = Partial<CapabilitySnapshot>

export interface CapabilityService {
  canAccess(surface: SurfaceCapability): boolean
  getSnapshot(): CapabilitySnapshot
}

class StaticCapabilityService implements CapabilityService {
  private readonly snapshot: CapabilitySnapshot

  constructor(snapshot: CapabilitySnapshot) {
    this.snapshot = snapshot
  }

  canAccess(surface: SurfaceCapability) {
    return this.snapshot.allowedSurfaces.includes(surface)
  }

  getSnapshot() {
    return this.snapshot
  }
}

export function createCapabilityService(
  snapshotInput: CapabilitySnapshotInput = DEFAULT_CAPABILITY_SNAPSHOT,
) {
  const snapshot = capabilitySnapshotSchema.parse({
    ...DEFAULT_CAPABILITY_SNAPSHOT,
    ...snapshotInput,
  })

  return new StaticCapabilityService(snapshot)
}
