import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

import {
  captureDeleteInputSchema,
  captureDeleteResultSchema,
  captureReadinessUpdateEvent,
  captureReadinessInputSchema,
  captureReadinessSnapshotSchema,
  captureReadinessUpdateSchema,
  captureRequestInputSchema,
  captureRequestResultSchema,
  hostErrorEnvelopeSchema,
  type CaptureDeleteInput,
  type CaptureDeleteResult,
  type CaptureReadinessInput,
  type CaptureReadinessSnapshot,
  type CaptureRequestInput,
  type CaptureRequestResult,
  type HostErrorEnvelope,
} from '../../shared-contracts'
import { isTauriRuntime } from '../../shared/runtime/is-tauri'
import { logCaptureClientState } from '../../shared/runtime/log-capture-client-state'

export const CAPTURE_READINESS_POLL_MS = 300
const BROWSER_SESSION_FIXTURE_ID = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'

function logCaptureRuntimeDebug(
  label: string,
  details: Record<string, unknown>,
) {
  if (typeof console === 'undefined') {
    return
  }

  console.info(`[boothy][capture-runtime] ${label}`, details)
  void logCaptureClientState({
    label,
    sessionId:
      typeof details.sessionId === 'string' ? details.sessionId : undefined,
    runtimeMode:
      typeof details.mode === 'string'
        ? details.mode
        : typeof details.runtimeMode === 'string'
          ? details.runtimeMode
          : undefined,
    message:
      typeof details.message === 'string' ? details.message : undefined,
    customerState:
      typeof details.customerState === 'string' ? details.customerState : undefined,
    reasonCode:
      typeof details.reasonCode === 'string' ? details.reasonCode : undefined,
    canCapture:
      typeof details.canCapture === 'boolean' ? details.canCapture : undefined,
  })
}

export type CaptureRuntimeMode = 'browser' | 'tauri'

export interface CaptureRuntimeGateway {
  getCaptureReadiness(input: CaptureReadinessInput): Promise<unknown>
  deleteCapture?(input: CaptureDeleteInput): Promise<unknown>
  requestCapture(input: CaptureRequestInput): Promise<unknown>
  subscribeToCaptureReadiness(
    onEvent: (payload: unknown) => void,
  ): Promise<() => void>
}

export interface CaptureRuntimeService {
  getCaptureReadiness(
    input: CaptureReadinessInput,
  ): Promise<CaptureReadinessSnapshot>
  deleteCapture?(input: CaptureDeleteInput): Promise<CaptureDeleteResult>
  requestCapture(input: CaptureRequestInput): Promise<CaptureRequestResult>
  subscribeToCaptureReadiness(input: {
    sessionId: string
    onReadiness(readiness: CaptureReadinessSnapshot): void
  }): Promise<() => void>
}

function buildPreparingCaptureReadiness(): CaptureReadinessSnapshot {
  return {
    schemaVersion: 'capture-readiness/v1',
    sessionId: BROWSER_SESSION_FIXTURE_ID,
    surfaceState: 'blocked',
    customerState: 'Preparing',
    canCapture: false,
    primaryAction: 'wait',
    customerMessage: '촬영 준비 중이에요.',
    supportMessage: '잠시만 기다려 주세요.',
    reasonCode: 'camera-preparing',
    latestCapture: null,
    postEnd: null,
    timing: null,
  }
}

function buildBrowserPreviewCaptureReadiness(): CaptureReadinessSnapshot {
  return {
    schemaVersion: 'capture-readiness/v1',
    sessionId: BROWSER_SESSION_FIXTURE_ID,
    surfaceState: 'blocked',
    customerState: 'Preparing',
    canCapture: false,
    primaryAction: 'wait',
    customerMessage: '카메라 연결 상태를 확인하는 중이에요.',
    supportMessage: '브라우저 미리보기에서는 실제 카메라 연결을 확인할 수 없어요.',
    reasonCode: 'camera-preparing',
    latestCapture: null,
    postEnd: null,
    timing: null,
  }
}

function sanitizeBrowserPreviewFixtureReadiness(
  fixture: CaptureReadinessSnapshot,
): CaptureReadinessSnapshot {
  if (
    fixture.canCapture ||
    fixture.primaryAction === 'capture' ||
    fixture.customerState === 'Ready' ||
    fixture.reasonCode === 'ready'
  ) {
    return {
      ...buildBrowserPreviewCaptureReadiness(),
      latestCapture: fixture.latestCapture,
      postEnd: fixture.postEnd,
      timing: fixture.timing,
    }
  }

  return fixture
}

function buildPresetMissingCaptureReadiness(): CaptureReadinessSnapshot {
  return {
    schemaVersion: 'capture-readiness/v1',
    sessionId: BROWSER_SESSION_FIXTURE_ID,
    surfaceState: 'blocked',
    customerState: 'Preparing',
    canCapture: false,
    primaryAction: 'choose-preset',
    customerMessage: '촬영 전에 룩을 먼저 골라 주세요.',
    supportMessage: '선택이 끝나면 바로 찍을 수 있어요.',
    reasonCode: 'preset-missing',
    latestCapture: null,
    postEnd: null,
    timing: null,
  }
}

function buildSessionMissingCaptureReadiness(): CaptureReadinessSnapshot {
  return {
    schemaVersion: 'capture-readiness/v1',
    sessionId: BROWSER_SESSION_FIXTURE_ID,
    surfaceState: 'blocked',
    customerState: 'Preparing',
    canCapture: false,
    primaryAction: 'start-session',
    customerMessage: '세션을 다시 시작해 주세요.',
    supportMessage: '이름과 휴대전화 뒤 4자리를 다시 확인할게요.',
    reasonCode: 'session-missing',
    latestCapture: null,
    postEnd: null,
    timing: null,
  }
}

function buildPhoneRequiredCaptureReadiness(): CaptureReadinessSnapshot {
  return {
    schemaVersion: 'capture-readiness/v1',
    sessionId: BROWSER_SESSION_FIXTURE_ID,
    surfaceState: 'blocked',
    customerState: 'Phone Required',
    canCapture: false,
    primaryAction: 'call-support',
    customerMessage: '지금은 도움이 필요해요.',
    supportMessage: '가까운 직원에게 알려 주세요.',
    reasonCode: 'phone-required',
    latestCapture: null,
    postEnd: null,
    timing: null,
  }
}

function buildTransientUnavailableCaptureReadiness(): CaptureReadinessSnapshot {
  return {
    schemaVersion: 'capture-readiness/v1',
    sessionId: BROWSER_SESSION_FIXTURE_ID,
    surfaceState: 'blocked',
    customerState: 'Preparing',
    canCapture: false,
    primaryAction: 'wait',
    customerMessage: '촬영 준비 상태를 다시 확인하고 있어요.',
    supportMessage: '잠시만 기다려 주세요.',
    reasonCode: 'camera-preparing',
    latestCapture: null,
    postEnd: null,
    timing: null,
  }
}

function buildTransientCaptureBlockReadiness(): CaptureReadinessSnapshot {
  return buildTransientUnavailableCaptureReadiness()
}

function buildSessionMismatchHostError(): HostErrorEnvelope {
  return {
    code: 'host-unavailable',
    message: '현재 세션 상태를 다시 확인할게요.',
  }
}

function withSessionId(
  readiness: CaptureReadinessSnapshot,
  sessionId?: string,
): CaptureReadinessSnapshot {
  if (sessionId === undefined) {
    return readiness
  }

  return {
    ...readiness,
    sessionId,
  }
}

class DefaultCaptureRuntimeService implements CaptureRuntimeService {
  private readonly gateway: CaptureRuntimeGateway

  constructor(gateway: CaptureRuntimeGateway) {
    this.gateway = gateway
  }

  async getCaptureReadiness(input: CaptureReadinessInput) {
    const parsedInput = captureReadinessInputSchema.parse(input)
    const parsedResponse = await (async () => {
      try {
        const response = await this.gateway.getCaptureReadiness(parsedInput)

        return captureReadinessSnapshotSchema.parse(response)
      } catch (error) {
        logCaptureRuntimeDebug('gateway-readiness-error', {
          sessionId: parsedInput.sessionId,
          mode: getCaptureRuntimeMode(),
          message: error instanceof Error ? error.message : String(error),
        })
        throw normalizeHostError(error, parsedInput.sessionId, 'readiness')
      }
    })()

    if (parsedResponse.sessionId !== parsedInput.sessionId) {
      throw buildSessionMismatchHostError()
    }

    logCaptureRuntimeDebug('gateway-readiness-success', {
      sessionId: parsedResponse.sessionId,
      mode: getCaptureRuntimeMode(),
      customerState: parsedResponse.customerState,
      reasonCode: parsedResponse.reasonCode,
      canCapture: parsedResponse.canCapture,
    })

    return parsedResponse
  }

  async deleteCapture(input: CaptureDeleteInput) {
    const parsedInput = captureDeleteInputSchema.parse(input)
    const parsedResponse = await (async () => {
      try {
        const deleteCapture = this.gateway.deleteCapture

        if (deleteCapture === undefined) {
          throw buildSessionMismatchHostError()
        }

        const response = await deleteCapture(parsedInput)

        return captureDeleteResultSchema.parse(response)
      } catch (error) {
        throw normalizeHostError(error, parsedInput.sessionId, 'delete-capture')
      }
    })()

    if (
      parsedResponse.sessionId !== parsedInput.sessionId ||
      parsedResponse.manifest.sessionId !== parsedInput.sessionId ||
      parsedResponse.readiness.sessionId !== parsedInput.sessionId
    ) {
      throw buildSessionMismatchHostError()
    }

    return parsedResponse
  }

  async requestCapture(input: CaptureRequestInput) {
    const parsedInput = captureRequestInputSchema.parse(input)
    const parsedResponse = await (async () => {
      try {
        const response = await this.gateway.requestCapture(parsedInput)

        return captureRequestResultSchema.parse(response)
      } catch (error) {
        throw normalizeHostError(error, parsedInput.sessionId, 'request-capture')
      }
    })()

    if (
      parsedResponse.sessionId !== parsedInput.sessionId ||
      parsedResponse.capture.sessionId !== parsedInput.sessionId ||
      parsedResponse.readiness.sessionId !== parsedInput.sessionId
    ) {
      throw buildSessionMismatchHostError()
    }

    return parsedResponse
  }

  async subscribeToCaptureReadiness(input: {
    sessionId: string
    onReadiness(readiness: CaptureReadinessSnapshot): void
  }) {
    const parsedInput = captureReadinessInputSchema.parse({
      sessionId: input.sessionId,
    })
    let latestReadinessKey: string | null = null
    let isDisposed = false
    let isPolling = false
    let pollId: ReturnType<typeof globalThis.setTimeout> | null = null

    const emitReadiness = (readiness: CaptureReadinessSnapshot) => {
      const nextKey = JSON.stringify(readiness)

      if (nextKey === latestReadinessKey) {
        return
      }

      latestReadinessKey = nextKey
      input.onReadiness(readiness)
    }

    const schedulePoll = (delayMs = CAPTURE_READINESS_POLL_MS) => {
      if (isDisposed) {
        return
      }

      if (pollId !== null) {
        globalThis.clearTimeout(pollId)
      }

      pollId = globalThis.setTimeout(() => {
        pollId = null
        void pollReadiness()
      }, delayMs)
    }

    const pollReadiness = async () => {
      if (isDisposed || isPolling) {
        return
      }

      isPolling = true

      try {
        const readiness = await this.getCaptureReadiness(parsedInput)
        emitReadiness(readiness)
      } catch (error) {
        const hostError = error as HostErrorEnvelope

        if (hostError.readiness) {
          emitReadiness(hostError.readiness)
        }
      } finally {
        isPolling = false
        schedulePoll()
      }
    }

    const unlisten = await this.gateway.subscribeToCaptureReadiness((payload) => {
      const parsedPayload = captureReadinessUpdateSchema.safeParse(payload)

      if (
        parsedPayload.success &&
        parsedPayload.data.sessionId === parsedInput.sessionId
      ) {
        emitReadiness(parsedPayload.data.readiness)
      }
    })

    schedulePoll()

    return () => {
      isDisposed = true

      if (pollId !== null) {
        globalThis.clearTimeout(pollId)
      }

      unlisten()
    }
  }
}

function normalizeHostError(
  error: unknown,
  requestedSessionId?: string,
  operation: 'readiness' | 'request-capture' | 'delete-capture' = 'readiness',
): HostErrorEnvelope {
  const parsed = hostErrorEnvelopeSchema.safeParse(error)

  if (parsed.success) {
    if (parsed.data.readiness) {
      return parsed.data
    }

    switch (parsed.data.code) {
      case 'session-not-found':
        if (operation === 'request-capture') {
          return {
            ...parsed.data,
            message: '촬영 준비 상태를 다시 확인하고 있어요.',
            readiness: withSessionId(
              buildTransientCaptureBlockReadiness(),
              requestedSessionId,
            ),
          }
        }

        return {
          ...parsed.data,
          message: '세션을 다시 시작해 주세요.',
          readiness: withSessionId(
            buildSessionMissingCaptureReadiness(),
            requestedSessionId,
          ),
        }
      case 'preset-not-available':
        return {
          ...parsed.data,
          message: '촬영 전에 룩을 다시 골라 주세요.',
          readiness: withSessionId(
            buildPresetMissingCaptureReadiness(),
            requestedSessionId,
          ),
        }
      case 'capture-delete-blocked':
        return {
          ...parsed.data,
          message: '이 사진은 지금 정리할 수 없어요. 잠시 후 다시 확인해 주세요.',
        }
      case 'capture-not-ready':
        return {
          ...parsed.data,
          message: '촬영 준비 상태를 다시 확인하고 있어요.',
          readiness: withSessionId(
            buildTransientCaptureBlockReadiness(),
            requestedSessionId,
          ),
        }
      case 'host-unavailable':
      case 'preset-catalog-unavailable':
      case 'session-persistence-failed':
      case 'validation-error':
        if (operation === 'readiness' || operation === 'request-capture') {
          return {
            ...parsed.data,
            message: '촬영 준비 상태를 다시 확인하고 있어요.',
            readiness: withSessionId(
              buildTransientUnavailableCaptureReadiness(),
              requestedSessionId,
            ),
          }
        }

        return {
          ...parsed.data,
          message: '지금은 도움이 필요해요.',
          readiness: withSessionId(
            buildPhoneRequiredCaptureReadiness(),
            requestedSessionId,
          ),
        }
      default:
        return parsed.data
    }
  }

  if (error instanceof Error) {
    if (operation === 'readiness' || operation === 'request-capture') {
      return {
        code: 'host-unavailable',
        message: '촬영 준비 상태를 다시 확인하고 있어요.',
        readiness: withSessionId(
          buildTransientUnavailableCaptureReadiness(),
          requestedSessionId,
        ),
      }
    }

    return {
      code: 'host-unavailable',
      message: '지금은 도움이 필요해요.',
      readiness: withSessionId(
        buildPhoneRequiredCaptureReadiness(),
        requestedSessionId,
      ),
    }
  }

  if (operation === 'readiness' || operation === 'request-capture') {
    return {
      code: 'host-unavailable',
      message: '촬영 준비 상태를 다시 확인하고 있어요.',
      readiness: withSessionId(
        buildTransientUnavailableCaptureReadiness(),
        requestedSessionId,
      ),
    }
  }

  return {
    code: 'host-unavailable',
    message: '지금은 도움이 필요해요.',
    readiness: withSessionId(
      buildPhoneRequiredCaptureReadiness(),
      requestedSessionId,
    ),
  }
}

export function getCaptureRuntimeMode(): CaptureRuntimeMode {
  return isTauriRuntime() ? 'tauri' : 'browser'
}

export function buildLocalCaptureReadiness(input: {
  sessionId?: string | null
  hasSession: boolean
  hasPreset: boolean
}): CaptureReadinessSnapshot {
  const sessionId = input.sessionId ?? BROWSER_SESSION_FIXTURE_ID

  if (!input.hasSession) {
    return {
      ...buildSessionMissingCaptureReadiness(),
      sessionId,
    }
  }

  if (!input.hasPreset) {
    return {
      ...buildPresetMissingCaptureReadiness(),
      sessionId,
    }
  }

  return {
    ...buildPreparingCaptureReadiness(),
    sessionId,
  }
}

function readBrowserCaptureReadinessFixture() {
  const fixture = (
    globalThis as typeof globalThis & {
      __BOOTHY_BROWSER_CAPTURE_READINESS__?: unknown
    }
  ).__BOOTHY_BROWSER_CAPTURE_READINESS__

  if (fixture === undefined) {
    return null
  }

  const parsed = captureReadinessSnapshotSchema.safeParse(fixture)

  if (parsed.success) {
    return sanitizeBrowserPreviewFixtureReadiness(parsed.data)
  }

  throw {
    code: 'host-unavailable',
    message: '브라우저 readiness fixture 형식이 올바르지 않아요.',
  } satisfies HostErrorEnvelope
}

export function createBrowserCaptureRuntimeGateway(): CaptureRuntimeGateway {
  return {
    async getCaptureReadiness(input) {
      const fixture = readBrowserCaptureReadinessFixture()

      if (fixture !== null) {
        return withSessionId(fixture, input.sessionId)
      }

      return withSessionId(buildBrowserPreviewCaptureReadiness(), input.sessionId)
    },
    async deleteCapture(input) {
      throw {
        code: 'host-unavailable',
        message: '브라우저 미리보기에서는 실제 촬영 상태를 바꾸지 않아요.',
        readiness: withSessionId(buildBrowserPreviewCaptureReadiness(), input.sessionId),
      } satisfies HostErrorEnvelope
    },
    async requestCapture(input) {
      throw {
        code: 'host-unavailable',
        message: '브라우저 미리보기에서는 실제 촬영을 실행할 수 없어요.',
        readiness: withSessionId(buildBrowserPreviewCaptureReadiness(), input.sessionId),
      } satisfies HostErrorEnvelope
    },
    async subscribeToCaptureReadiness() {
      return () => undefined
    },
  }
}

export function createTauriCaptureRuntimeGateway(): CaptureRuntimeGateway {
  return {
    async getCaptureReadiness(input) {
      return invoke<unknown>('get_capture_readiness', { input })
    },
    async deleteCapture(input) {
      return invoke<unknown>('delete_capture', { input })
    },
    async requestCapture(input) {
      return invoke<unknown>('request_capture', { input })
    },
    async subscribeToCaptureReadiness(onEvent) {
      try {
        return await listen(captureReadinessUpdateEvent, (event) => {
          onEvent(event.payload)
        })
      } catch {
        return () => undefined
      }
    },
  }
}

function resolveActiveCaptureRuntimeGateway(): {
  gateway: CaptureRuntimeGateway
  mode: CaptureRuntimeMode
} {
  if (isTauriRuntime()) {
    return {
      gateway: createTauriCaptureRuntimeGateway(),
      mode: 'tauri',
    }
  }

  return {
    gateway: createBrowserCaptureRuntimeGateway(),
    mode: 'browser',
  }
}

export function createDefaultCaptureRuntimeGateway(): CaptureRuntimeGateway {
  return {
    async getCaptureReadiness(input: CaptureReadinessInput) {
      const { gateway, mode } = resolveActiveCaptureRuntimeGateway()
      logCaptureRuntimeDebug('gateway-get-readiness', {
        sessionId: input.sessionId,
        mode,
      })
      return gateway.getCaptureReadiness(input)
    },
    async deleteCapture(input: CaptureDeleteInput) {
      const { gateway, mode } = resolveActiveCaptureRuntimeGateway()
      logCaptureRuntimeDebug('gateway-delete-capture', {
        sessionId: input.sessionId,
        captureId: input.captureId,
        mode,
      })

      if (gateway.deleteCapture === undefined) {
        throw {
          code: 'host-unavailable',
          message: '브라우저 미리보기에서는 실제 촬영 상태를 바꾸지 않아요.',
          readiness: withSessionId(buildBrowserPreviewCaptureReadiness(), input.sessionId),
        } satisfies HostErrorEnvelope
      }

      return gateway.deleteCapture(input)
    },
    async requestCapture(input: CaptureRequestInput) {
      const { gateway, mode } = resolveActiveCaptureRuntimeGateway()
      logCaptureRuntimeDebug('gateway-request-capture', {
        sessionId: input.sessionId,
        mode,
      })
      return gateway.requestCapture(input)
    },
    async subscribeToCaptureReadiness(onEvent: (payload: unknown) => void) {
      const { gateway, mode } = resolveActiveCaptureRuntimeGateway()
      logCaptureRuntimeDebug('gateway-subscribe-readiness', {
        mode,
      })
      return gateway.subscribeToCaptureReadiness(onEvent)
    },
  }
}

type CreateCaptureRuntimeServiceOptions = {
  gateway?: CaptureRuntimeGateway
}

export function createCaptureRuntimeService({
  gateway = createDefaultCaptureRuntimeGateway(),
}: CreateCaptureRuntimeServiceOptions = {}) {
  return new DefaultCaptureRuntimeService(gateway)
}
