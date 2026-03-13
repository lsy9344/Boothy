import { invoke } from '@tauri-apps/api/core'

import {
  getPostEndOutcomePayloadSchema,
  postEndOutcomeEnvelopeSchema,
  type GetPostEndOutcomePayload,
  type PostEndOutcomeEnvelope,
} from '../../shared-contracts/schemas/postEndOutcomeSchemas.js'

type PostEndOutcomeInvokeClient = <T>(command: string, args?: Record<string, unknown>) => Promise<T>

export type PostEndOutcomeService = {
  getPostEndOutcome(payload: GetPostEndOutcomePayload): Promise<PostEndOutcomeEnvelope>
}

export function createPostEndOutcomeService(
  invokeClient: PostEndOutcomeInvokeClient = invoke,
): PostEndOutcomeService {
  return {
    async getPostEndOutcome(payload) {
      const request = getPostEndOutcomePayloadSchema.parse(payload)
      const response = await invokeClient<unknown>('get_post_end_outcome', {
        payload: request,
      })

      return postEndOutcomeEnvelopeSchema.parse(response)
    },
  }
}

export const postEndOutcomeService = createPostEndOutcomeService()
