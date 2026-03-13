import { invoke } from '@tauri-apps/api/core'

import { resolveOperationalBranchId } from '../../diagnostics-log/services/operationalLogContext.js'
import type {
  SessionResultEnvelope,
  SessionStartPayload,
} from '../../shared-contracts/dto/session.js'
import {
  sessionResultEnvelopeSchema,
  sessionStartPayloadSchema,
  sessionStartCommandPayloadSchema,
} from '../../shared-contracts/schemas/sessionSchemas.js'

type SessionInvokeClient = <T>(command: string, args?: Record<string, unknown>) => Promise<T>

export type SessionLifecycleService = {
  startSession(payload: SessionStartPayload & { branchId?: string }): Promise<SessionResultEnvelope>
}

export function createSessionLifecycleService(invokeClient: SessionInvokeClient = invoke): SessionLifecycleService {
  return {
    async startSession(payload) {
      const basePayload = sessionStartPayloadSchema.parse(payload)
      const request = sessionStartCommandPayloadSchema.parse({
        ...basePayload,
        branchId: resolveOperationalBranchId(payload.branchId),
      })
      const response = await invokeClient<unknown>('start_session', { payload: request })

      return sessionResultEnvelopeSchema.parse(response)
    },
  }
}

export const sessionLifecycleService = createSessionLifecycleService()
