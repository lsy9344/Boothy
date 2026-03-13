import { invoke } from '@tauri-apps/api/core'

import {
  extendSessionTimingPayloadSchema,
  getSessionTimingPayloadSchema,
  initializeSessionTimingPayloadSchema,
  sessionTimingResultEnvelopeSchema,
  type ExtendSessionTimingPayload,
  type GetSessionTimingPayload,
  type InitializeSessionTimingPayload,
  type SessionTimingResultEnvelope,
} from '../../shared-contracts/schemas/sessionTimingSchemas.js'

type SessionTimingInvokeClient = <T>(command: string, args?: Record<string, unknown>) => Promise<T>

export type SessionTimingService = {
  initializeSessionTiming(payload: InitializeSessionTimingPayload): Promise<SessionTimingResultEnvelope>
  getSessionTiming(payload: GetSessionTimingPayload): Promise<SessionTimingResultEnvelope>
  extendSessionTiming(payload: ExtendSessionTimingPayload): Promise<SessionTimingResultEnvelope>
}

export function createSessionTimingService(
  invokeClient: SessionTimingInvokeClient = invoke,
): SessionTimingService {
  return {
    async initializeSessionTiming(payload) {
      const request = initializeSessionTimingPayloadSchema.parse(payload)
      const response = await invokeClient<unknown>('initialize_session_timing', {
        payload: request,
      })

      return sessionTimingResultEnvelopeSchema.parse(response)
    },

    async getSessionTiming(payload) {
      const request = getSessionTimingPayloadSchema.parse(payload)
      const response = await invokeClient<unknown>('get_session_timing', {
        payload: request,
      })

      return sessionTimingResultEnvelopeSchema.parse(response)
    },

    async extendSessionTiming(payload) {
      const request = extendSessionTimingPayloadSchema.parse(payload)
      const response = await invokeClient<unknown>('extend_session_timing', {
        payload: request,
      })

      return sessionTimingResultEnvelopeSchema.parse(response)
    },
  }
}

export const sessionTimingService = createSessionTimingService()
