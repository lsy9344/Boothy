import { invoke } from '@tauri-apps/api/core'

import {
  hostErrorEnvelopeSchema,
  operatorAuditQueryFilterSchema,
  operatorAuditQueryResultSchema,
  operatorRecoveryActionRequestSchema,
  operatorRecoveryActionResultSchema,
  operatorRecoverySummarySchema,
  type HostErrorEnvelope,
  type OperatorAuditQueryFilter,
  type OperatorAuditQueryResult,
  type OperatorRecoveryActionRequest,
  type OperatorRecoveryActionResult,
  type OperatorRecoverySummary,
} from '../../shared-contracts'

export interface OperatorDiagnosticsGateway {
  loadOperatorRecoverySummary(): Promise<unknown>
  loadOperatorAuditHistory(input: OperatorAuditQueryFilter): Promise<unknown>
  runOperatorRecoveryAction(input: OperatorRecoveryActionRequest): Promise<unknown>
}

export interface OperatorDiagnosticsService {
  loadOperatorRecoverySummary(): Promise<OperatorRecoverySummary>
  loadOperatorAuditHistory(
    input: OperatorAuditQueryFilter,
  ): Promise<OperatorAuditQueryResult>
  runOperatorRecoveryAction(
    input: OperatorRecoveryActionRequest,
  ): Promise<OperatorRecoveryActionResult>
}

class DefaultOperatorDiagnosticsService implements OperatorDiagnosticsService {
  private readonly gateway: OperatorDiagnosticsGateway

  constructor(gateway: OperatorDiagnosticsGateway) {
    this.gateway = gateway
  }

  async loadOperatorRecoverySummary() {
    try {
      const response = await this.gateway.loadOperatorRecoverySummary()

      return operatorRecoverySummarySchema.parse(response)
    } catch (error) {
      throw normalizeHostError(error)
    }
  }

  async loadOperatorAuditHistory(input: OperatorAuditQueryFilter) {
    const parsedInput = operatorAuditQueryFilterSchema.parse(input)

    try {
      const response = await this.gateway.loadOperatorAuditHistory(parsedInput)
      const parsedResponse = operatorAuditQueryResultSchema.parse(response)

      return ensureMatchingOperatorAuditResult(parsedInput, parsedResponse)
    } catch (error) {
      throw normalizeHostError(error)
    }
  }

  async runOperatorRecoveryAction(input: OperatorRecoveryActionRequest) {
    const parsedInput = operatorRecoveryActionRequestSchema.parse(input)

    try {
      const response = await this.gateway.runOperatorRecoveryAction(parsedInput)
      const parsedResponse = operatorRecoveryActionResultSchema.parse(response)

      return ensureMatchingOperatorRecoveryResult(parsedInput, parsedResponse)
    } catch (error) {
      throw normalizeHostError(error)
    }
  }
}

function normalizeHostError(error: unknown): HostErrorEnvelope {
  const parsed = hostErrorEnvelopeSchema.safeParse(error)

  if (parsed.success) {
    switch (parsed.data.code) {
      case 'capability-denied':
        return {
          code: parsed.data.code,
          message: '승인된 operator 창에서만 현재 세션 진단을 볼 수 있어요.',
        }
      case 'session-not-found':
        return {
          code: parsed.data.code,
          message: '현재 세션 문맥을 다시 확인하고 있어요. 잠시 후 다시 시도해 주세요.',
        }
      default:
        return {
          code: parsed.data.code,
          message:
            '지금은 현재 세션 진단을 불러올 수 없어요. 잠시 후 다시 시도해 주세요.',
        }
    }
  }

  if (error instanceof Error) {
    return {
      code: 'host-unavailable',
      message: '지금은 현재 세션 진단을 불러올 수 없어요. 잠시 후 다시 시도해 주세요.',
    }
  }

  return {
    code: 'host-unavailable',
    message: '지금은 현재 세션 진단을 불러올 수 없어요. 잠시 후 다시 시도해 주세요.',
  }
}

function isTauriRuntime() {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

function readBrowserSummaryFixture() {
  const fixture = (
    globalThis as typeof globalThis & {
      __BOOTHY_BROWSER_OPERATOR_RECOVERY_SUMMARY__?: unknown
    }
  ).__BOOTHY_BROWSER_OPERATOR_RECOVERY_SUMMARY__

  if (fixture === undefined) {
    return null
  }

  const parsed = operatorRecoverySummarySchema.safeParse(fixture)

  if (parsed.success) {
    return parsed.data
  }

  throw {
    code: 'host-unavailable',
    message: '브라우저 operator diagnostics fixture 형식이 올바르지 않아요.',
  } satisfies HostErrorEnvelope
}

function readBrowserAuditHistoryFixture() {
  const fixture = (
    globalThis as typeof globalThis & {
      __BOOTHY_BROWSER_OPERATOR_AUDIT_HISTORY__?: unknown
    }
  ).__BOOTHY_BROWSER_OPERATOR_AUDIT_HISTORY__

  if (fixture === undefined) {
    return null
  }

  const parsed = operatorAuditQueryResultSchema.safeParse(fixture)

  if (parsed.success) {
    return parsed.data
  }

  throw {
    code: 'host-unavailable',
    message: '브라우저 operator audit fixture 형식이 올바르지 않아요.',
  } satisfies HostErrorEnvelope
}

export function createBrowserOperatorDiagnosticsGateway(): OperatorDiagnosticsGateway {
  return {
    async loadOperatorRecoverySummary() {
      const fixture = readBrowserSummaryFixture()

      if (fixture !== null) {
        return fixture
      }

      throw {
        code: 'host-unavailable',
        message: '브라우저 미리보기에서는 operator diagnostics fixture를 먼저 연결해 주세요.',
      } satisfies HostErrorEnvelope
    },
    async loadOperatorAuditHistory() {
      const fixture = readBrowserAuditHistoryFixture()

      if (fixture !== null) {
        return fixture
      }

      throw {
        code: 'host-unavailable',
        message: '브라우저 미리보기에서는 operator audit fixture를 먼저 연결해 주세요.',
      } satisfies HostErrorEnvelope
    },
    async runOperatorRecoveryAction() {
      throw {
        code: 'host-unavailable',
        message:
          '브라우저 미리보기에서는 operator recovery action을 실행하지 않아요.',
      } satisfies HostErrorEnvelope
    },
  }
}

export function createTauriOperatorDiagnosticsGateway(): OperatorDiagnosticsGateway {
  return {
    async loadOperatorRecoverySummary() {
      return invoke<unknown>('load_operator_recovery_summary')
    },
    async loadOperatorAuditHistory(input) {
      return invoke<unknown>('load_operator_audit_history', { input })
    },
    async runOperatorRecoveryAction(input) {
      return invoke<unknown>('run_operator_recovery_action', { input })
    },
  }
}

export function createDefaultOperatorDiagnosticsGateway() {
  return isTauriRuntime()
    ? createTauriOperatorDiagnosticsGateway()
    : createBrowserOperatorDiagnosticsGateway()
}

type CreateOperatorDiagnosticsServiceOptions = {
  gateway?: OperatorDiagnosticsGateway
}

export function createOperatorDiagnosticsService({
  gateway = createDefaultOperatorDiagnosticsGateway(),
}: CreateOperatorDiagnosticsServiceOptions = {}) {
  return new DefaultOperatorDiagnosticsService(gateway)
}

function ensureMatchingOperatorRecoveryResult(
  input: OperatorRecoveryActionRequest,
  response: OperatorRecoveryActionResult,
) {
  if (
    response.sessionId !== input.sessionId ||
    response.action !== input.action ||
    response.summary.sessionId !== input.sessionId
  ) {
    throw {
      code: 'host-unavailable',
      message: '요청한 세션과 다른 operator recovery 응답을 받았어요. 다시 시도해 주세요.',
    } satisfies HostErrorEnvelope
  }

  return response
}

function ensureMatchingOperatorAuditResult(
  input: OperatorAuditQueryFilter,
  response: OperatorAuditQueryResult,
) {
  if ((input.sessionId ?? null) !== (response.filter.sessionId ?? null)) {
    throw {
      code: 'host-unavailable',
      message: '요청한 세션과 다른 operator audit 응답을 받았어요. 다시 시도해 주세요.',
    } satisfies HostErrorEnvelope
  }

  return response
}
