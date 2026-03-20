import { z } from 'zod'

const surfaceCapabilities = ['booth', 'operator', 'authoring', 'settings'] as const

export const surfaceCapabilitySchema = z.enum(surfaceCapabilities)
export type SurfaceCapability = z.infer<typeof surfaceCapabilitySchema>

function normalizeAllowedSurfaces(
  allowedSurfaces: SurfaceCapability[],
): SurfaceCapability[] {
  return Array.from(new Set(['booth', ...allowedSurfaces])) as SurfaceCapability[]
}

export const capabilitySnapshotSchema = z
  .object({
    isAdminAuthenticated: z.boolean().default(false),
    allowedSurfaces: z.array(surfaceCapabilitySchema).default(['booth']),
  })
  .transform((snapshot) => ({
    ...snapshot,
    allowedSurfaces: normalizeAllowedSurfaces(snapshot.allowedSurfaces),
  }))

export type CapabilitySnapshot = z.output<typeof capabilitySnapshotSchema>

export const DEFAULT_CAPABILITY_SNAPSHOT = capabilitySnapshotSchema.parse({})
