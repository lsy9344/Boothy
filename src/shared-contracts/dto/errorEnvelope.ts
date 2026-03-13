import { z } from 'zod'

import {
  customerCameraConnectionStateSchema,
  customerStateSchema,
  operatorActionSchema,
  operatorCameraConnectionStateSchema,
} from './cameraErrorContract.js'
import { errorEnvelopeSchemaVersionSchema } from './schemaVersion.js'

export const errorSeveritySchema = z.enum(['info', 'warning', 'error', 'critical'])

export const normalizedErrorEnvelopeSchema = z
  .object({
    schemaVersion: errorEnvelopeSchemaVersionSchema,
    code: z.string().min(1),
    severity: errorSeveritySchema,
    retryable: z.boolean(),
    customerState: customerStateSchema,
    customerCameraConnectionState: customerCameraConnectionStateSchema,
    operatorCameraConnectionState: operatorCameraConnectionStateSchema,
    operatorAction: operatorActionSchema,
    message: z.string().min(1),
    details: z.string().min(1).optional(),
  })
  .strict()

export type ErrorSeverity = z.infer<typeof errorSeveritySchema>
export type NormalizedErrorEnvelope = z.infer<typeof normalizedErrorEnvelopeSchema>
