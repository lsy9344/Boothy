import { z } from 'zod'

import {
  type CustomerCameraConnectionState,
  type CustomerState,
  type OperatorAction,
  type OperatorCameraConnectionState,
} from './cameraErrorContract.js'
import { normalizedErrorEnvelopeSchema } from './errorEnvelope.js'

export const cameraConnectionStateSchema = z.enum(['connected', 'reconnecting', 'disconnected', 'offline'])
export const cameraReadinessSchema = z.enum(['pending', 'ready', 'degraded'])

export const cameraStatusSnapshotSchema = z
  .object({
    connectionState: cameraConnectionStateSchema,
    readiness: cameraReadinessSchema,
    lastUpdatedAt: z.iso.datetime(),
  })
  .strict()

export const customerReadinessConnectionStateSchema = z.enum([
  'preparing',
  'waiting',
  'ready',
  'phone-required',
])
export const lastSafeCustomerStateSchema = z.enum(['preparing', 'ready'])

export const cameraReadinessStatusSchema = z
  .object({
    sessionId: z.string().min(1),
    connectionState: customerReadinessConnectionStateSchema,
    captureEnabled: z.boolean(),
    lastStableCustomerState: lastSafeCustomerStateSchema.nullable(),
    error: normalizedErrorEnvelopeSchema.nullable(),
    emittedAt: z.iso.datetime(),
  })
  .superRefine((value, ctx) => {
    const shouldAllowCapture = value.connectionState === 'ready'

    if (value.captureEnabled !== shouldAllowCapture) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: 'captureEnabled must match the normalized readiness connectionState',
        path: ['captureEnabled'],
      })
    }
  })
  .strict()

export type CameraStatusSnapshot = z.infer<typeof cameraStatusSnapshotSchema>
export type CameraReadinessStatus = z.infer<typeof cameraReadinessStatusSchema>
export type {
  CustomerCameraConnectionState,
  CustomerState,
  OperatorAction,
  OperatorCameraConnectionState,
}
