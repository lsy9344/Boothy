import { z } from 'zod';

export const customerCameraConnectionStateSchema = z.enum([
  'preparing',
  'waiting',
  'ready',
  'phone-required',
]);

export const operatorCameraConnectionStateSchema = z.enum([
  'initializing-session',
  'camera-connecting',
  'camera-ready',
  'camera-timeout',
  'camera-unavailable',
]);

export const hostErrorSeveritySchema = z.enum(['info', 'warning', 'error']);
export const hostOperatorActionSchema = z.enum(['none', 'retry-camera', 'call-branch']);

export const hostErrorEnvelopeSchema = z
  .object({
    code: z.string().trim().min(1),
    severity: hostErrorSeveritySchema,
    customerState: customerCameraConnectionStateSchema,
    customerCameraConnectionState: customerCameraConnectionStateSchema,
    operatorCameraConnectionState: operatorCameraConnectionStateSchema,
    operatorAction: hostOperatorActionSchema,
    retryable: z.boolean(),
    technicalSummary: z.string().trim().min(1).optional(),
  })
  .strip();

export type CustomerCameraConnectionState = z.infer<typeof customerCameraConnectionStateSchema>;
export type HostErrorEnvelope = z.infer<typeof hostErrorEnvelopeSchema>;
