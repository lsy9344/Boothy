import { invoke } from '@tauri-apps/api/core'

import {
  hostErrorEnvelopeSchema,
  sessionStartInputSchema,
  sessionStartResultSchema,
  type HostErrorEnvelope,
  type SessionStartInput,
  type SessionStartResult,
} from '../../shared-contracts'
import { isTauriRuntime } from '../../shared/runtime/is-tauri'

export interface StartSessionGateway {
  startSession(input: SessionStartInput): Promise<unknown>
}

export interface StartSessionService {
  startSession(input: SessionStartInput): Promise<SessionStartResult>
}

class DefaultStartSessionService implements StartSessionService {
  private readonly gateway: StartSessionGateway

  constructor(gateway: StartSessionGateway) {
    this.gateway = gateway
  }

  async startSession(input: SessionStartInput) {
    const parsedInput = sessionStartInputSchema.parse(input)

    try {
      const response = await this.gateway.startSession(parsedInput)

      return sessionStartResultSchema.parse(response)
    } catch (error) {
      throw normalizeHostError(error)
    }
  }
}

function normalizeHostError(error: unknown): HostErrorEnvelope {
  const parsed = hostErrorEnvelopeSchema.safeParse(error)

  if (parsed.success) {
    return parsed.data
  }

  if (error instanceof Error) {
    return {
      code: 'host-unavailable',
      message: error.message,
    }
  }

  return {
    code: 'host-unavailable',
    message: '지금은 시작할 수 없어요. 잠시 후 다시 시도해 주세요.',
  }
}

export function createBrowserStartSessionGateway(): StartSessionGateway {
  return {
    async startSession() {
      throw {
        code: 'host-unavailable',
        message: '이 장치에서는 아직 시작 준비가 끝나지 않았어요.',
      } satisfies HostErrorEnvelope
    },
  }
}

export function createTauriStartSessionGateway(): StartSessionGateway {
  return {
    async startSession(input) {
      return invoke<unknown>('start_session', { input })
    },
  }
}

export function createDefaultStartSessionGateway() {
  return isTauriRuntime()
    ? createTauriStartSessionGateway()
    : createBrowserStartSessionGateway()
}

type CreateStartSessionServiceOptions = {
  gateway?: StartSessionGateway
}

export function createStartSessionService({
  gateway = createDefaultStartSessionGateway(),
}: CreateStartSessionServiceOptions = {}) {
  return new DefaultStartSessionService(gateway)
}
