import { z } from 'zod'

export const customerStateSchema = z.enum(['cameraReconnectNeeded', 'cameraUnavailable'])
export const customerCameraConnectionStateSchema = z.enum(['connected', 'needsAttention', 'offline'])
export const operatorCameraConnectionStateSchema = z.enum(['connected', 'reconnecting', 'disconnected', 'offline'])
export const operatorActionSchema = z.enum(['checkCableAndRetry', 'restartHelper', 'contactSupport'])

export type CustomerCameraConnectionState = z.infer<typeof customerCameraConnectionStateSchema>
export type CustomerState = z.infer<typeof customerStateSchema>
export type OperatorAction = z.infer<typeof operatorActionSchema>
export type OperatorCameraConnectionState = z.infer<typeof operatorCameraConnectionStateSchema>
