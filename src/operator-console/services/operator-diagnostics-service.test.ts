import { afterEach, describe, expect, it } from 'vitest'

import {
  createBrowserOperatorDiagnosticsGateway,
  createOperatorDiagnosticsService,
} from './operator-diagnostics-service'

type BrowserGlobals = typeof globalThis & {
  __BOOTHY_BROWSER_OPERATOR_RECOVERY_SUMMARY__?: unknown
  __BOOTHY_BROWSER_OPERATOR_AUDIT_HISTORY__?: unknown
}

function createOperatorRecoverySummary(overrides: Record<string, unknown> = {}) {
  return {
    schemaVersion: 'operator-recovery-summary/v1',
    state: 'session-loaded',
    blockedStateCategory: 'preview-render-blocked',
    blockedCategory: 'preview-or-render',
    diagnosticsSummary: {
      title: '프리뷰/렌더 결과 준비 지연',
      detail: '가장 최근 촬영본은 저장되었지만 결과 준비가 아직 끝나지 않았어요.',
      observedAt: '2026-03-26T00:10:01.000Z',
    },
    allowedActions: ['retry', 'approved-boundary-restart', 'route-phone-required'],
    sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
    boothAlias: 'Kim 4821',
    activePresetId: 'preset_soft-glow',
    activePresetDisplayName: 'Soft Glow',
    activePresetVersion: '2026.03.26',
    lifecycleStage: 'preview-waiting',
    timingPhase: 'active',
    updatedAt: '2026-03-26T00:10:00.000Z',
    postEndState: null,
    recentFailure: {
      title: '프리뷰/렌더 결과 준비 지연',
      detail: '가장 최근 촬영본은 저장되었지만 결과 준비가 아직 끝나지 않았어요.',
      observedAt: '2026-03-26T00:10:01.000Z',
    },
    cameraConnection: {
      state: 'connected',
      title: '카메라와 helper 연결이 확인됐어요.',
      detail: '카메라와 helper가 현재 세션 기준으로 연결된 상태예요.',
      observedAt: '2026-03-26T00:10:00.000Z',
    },
    captureBoundary: {
      status: 'clear',
      title: '캡처 경계 정상',
      detail: '활성 preset이 선택돼 있어 capture 경계는 열려 있어요.',
    },
    previewRenderBoundary: {
      status: 'blocked',
      title: '프리뷰/렌더 결과 준비 지연',
      detail:
        '가장 최근 촬영본은 저장되었지만 preview/render 결과가 아직 준비되지 않았어요.',
    },
    completionBoundary: {
      status: 'clear',
      title: '완료 경계 대기 전',
      detail: '아직 종료 후 완료 경계로 들어가지 않았어요.',
    },
    previewArchitecture: {
      route: 'local-renderer-sidecar',
      routeStage: 'canary',
      laneOwner: 'inline-truthful-fallback',
      fallbackReasonCode: 'route-policy-shadow',
      firstVisibleMs: 2810,
      replacementMs: 3615,
      originalVisibleToPresetAppliedVisibleMs: 805,
      hardwareCapability: 'dedicated-renderer-available',
      warmState: 'warm-ready',
      warmStateObservedAt: '2026-04-12T08:00:00.000Z',
    },
    ...overrides,
  }
}

function createOperatorRecoveryActionResult(overrides: Record<string, unknown> = {}) {
  return {
    schemaVersion: 'operator-recovery-action-result/v1',
    sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
    action: 'retry',
    status: 'applied',
    message: '현재 막힌 preview/render 경계를 다시 시도했어요.',
    rejectionReason: null,
    diagnosticsSummary: {
      title: '프리뷰/렌더 결과 준비 지연',
      detail: '가장 최근 촬영본은 저장되었지만 결과 준비가 아직 끝나지 않았어요.',
      observedAt: '2026-03-26T00:10:01.000Z',
    },
    nextState: {
      customerState: 'Ready',
      reasonCode: 'ready',
      lifecycleStage: 'capture-ready',
      timingPhase: 'active',
      postEndState: null,
    },
    summary: createOperatorRecoverySummary({
      blockedStateCategory: 'not-blocked',
      blockedCategory: null,
      diagnosticsSummary: null,
      allowedActions: [],
      lifecycleStage: 'capture-ready',
      previewRenderBoundary: {
        status: 'clear',
        title: '프리뷰/렌더 경계 정상',
        detail: '가장 최근 촬영본의 결과 준비가 끝나 있어요.',
      },
    }),
    ...overrides,
  }
}

function createOperatorAuditHistory(overrides: Record<string, unknown> = {}) {
  return {
    schemaVersion: 'operator-audit-query-result/v1',
    filter: {
      sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
      eventCategories: ['operator-intervention', 'critical-failure'],
      limit: 10,
    },
    events: [
      {
        schemaVersion: 'operator-audit-entry/v1',
        eventId: 'audit_20260327_0001',
        occurredAt: '2026-03-27T00:10:01.000Z',
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        eventCategory: 'operator-intervention',
        eventType: 'retry',
        summary: '막힌 preview/render 경계를 다시 시도했어요.',
        detail: '최근 촬영 결과를 다시 준비할 수 있도록 재시도를 적용했어요.',
        actorId: 'operator-kim',
        source: 'operator-console',
        captureId: 'capture_01hs6n1r8b8zc5v4ey2x7b9g1m',
        presetId: 'preset_soft-glow',
        publishedVersion: '2026.03.26',
        reasonCode: null,
      },
    ],
    summary: {
      totalEvents: 1,
      sessionLifecycleEvents: 0,
      timingTransitionEvents: 0,
      postEndOutcomeEvents: 0,
      operatorInterventionEvents: 1,
      publicationRecoveryEvents: 0,
      criticalFailureEvents: 0,
      latestOutcome: {
        occurredAt: '2026-03-27T00:10:01.000Z',
        eventCategory: 'operator-intervention',
        eventType: 'retry',
        summary: '막힌 preview/render 경계를 다시 시도했어요.',
      },
    },
    ...overrides,
  }
}

describe('operator diagnostics service', () => {
  afterEach(() => {
    delete (globalThis as BrowserGlobals).__BOOTHY_BROWSER_OPERATOR_RECOVERY_SUMMARY__
    delete (globalThis as BrowserGlobals).__BOOTHY_BROWSER_OPERATOR_AUDIT_HISTORY__
  })

  it('uses an injected browser fixture when one is available', async () => {
    ;(globalThis as BrowserGlobals).__BOOTHY_BROWSER_OPERATOR_RECOVERY_SUMMARY__ =
      createOperatorRecoverySummary()

    const result =
      await createBrowserOperatorDiagnosticsGateway().loadOperatorRecoverySummary()

    expect(result).toMatchObject({
      schemaVersion: 'operator-recovery-summary/v1',
      blockedCategory: 'preview-or-render',
    })
  })

  it('surfaces a clear host-unavailable error when no browser fixture exists', async () => {
    await expect(
      createBrowserOperatorDiagnosticsGateway().loadOperatorRecoverySummary(),
    ).rejects.toMatchObject({
      code: 'host-unavailable',
      message:
        '브라우저 미리보기에서는 operator diagnostics fixture를 먼저 연결해 주세요.',
    })
  })

  it('normalizes malformed browser fixtures into the host error envelope', async () => {
    ;(globalThis as BrowserGlobals).__BOOTHY_BROWSER_OPERATOR_RECOVERY_SUMMARY__ = {
      state: 'session-loaded',
      blockedCategory: 'preview-or-render',
    }

    await expect(
      createBrowserOperatorDiagnosticsGateway().loadOperatorRecoverySummary(),
    ).rejects.toMatchObject({
      code: 'host-unavailable',
      message: '브라우저 operator diagnostics fixture 형식이 올바르지 않아요.',
    })
  })

  it('parses the typed operator recovery summary through the service seam', async () => {
    const service = createOperatorDiagnosticsService({
      gateway: {
        async loadOperatorRecoverySummary() {
          return createOperatorRecoverySummary()
        },
        async loadOperatorAuditHistory() {
          return createOperatorAuditHistory()
        },
        async runOperatorRecoveryAction() {
          return createOperatorRecoveryActionResult()
        },
      },
    })

    await expect(service.loadOperatorRecoverySummary()).resolves.toMatchObject({
      boothAlias: 'Kim 4821',
      blockedCategory: 'preview-or-render',
      previewArchitecture: {
        replacementMs: 3615,
        warmState: 'warm-ready',
      },
    })
  })

  it('accepts helper timestamps with timezone offsets through the service seam', async () => {
    const service = createOperatorDiagnosticsService({
      gateway: {
        async loadOperatorRecoverySummary() {
          return createOperatorRecoverySummary({
            cameraConnection: {
              state: 'connected',
              title: '카메라와 helper 연결이 확인됐어요.',
              detail: '카메라와 helper가 현재 세션 기준으로 연결된 상태예요.',
              observedAt: '2026-04-10T08:17:58.5548198+00:00',
            },
            liveCaptureTruth: {
              source: 'canon-helper-sidecar',
              freshness: 'fresh',
              sessionMatch: 'matched',
              cameraState: 'ready',
              helperState: 'healthy',
              observedAt: '2026-04-10T08:17:58.5548198+00:00',
              sequence: 162,
              detailCode: 'camera-ready',
            },
          })
        },
        async loadOperatorAuditHistory() {
          return createOperatorAuditHistory()
        },
        async runOperatorRecoveryAction() {
          return createOperatorRecoveryActionResult()
        },
      },
    })

    await expect(service.loadOperatorRecoverySummary()).resolves.toMatchObject({
      cameraConnection: {
        observedAt: '2026-04-10T08:17:58.5548198+00:00',
      },
      liveCaptureTruth: {
        observedAt: '2026-04-10T08:17:58.5548198+00:00',
      },
    })
  })

  it('parses recovery action results and enforces session consistency', async () => {
    const service = createOperatorDiagnosticsService({
      gateway: {
        async loadOperatorRecoverySummary() {
          return createOperatorRecoverySummary()
        },
        async loadOperatorAuditHistory() {
          return createOperatorAuditHistory()
        },
        async runOperatorRecoveryAction() {
          return createOperatorRecoveryActionResult()
        },
      },
    })

    await expect(
      service.runOperatorRecoveryAction({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        action: 'retry',
      }),
    ).resolves.toMatchObject({
      status: 'applied',
      nextState: {
        customerState: 'Ready',
      },
    })
  })

  it('rejects a foreign session recovery response before it reaches the UI', async () => {
    const service = createOperatorDiagnosticsService({
      gateway: {
        async loadOperatorRecoverySummary() {
          return createOperatorRecoverySummary()
        },
        async loadOperatorAuditHistory() {
          return createOperatorAuditHistory()
        },
        async runOperatorRecoveryAction() {
          return createOperatorRecoveryActionResult({
            sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
            summary: createOperatorRecoverySummary({
              sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
            }),
          })
        },
      },
    })

    await expect(
      service.runOperatorRecoveryAction({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        action: 'retry',
      }),
    ).rejects.toMatchObject({
      code: 'host-unavailable',
      message:
        '지금은 현재 세션 진단을 불러올 수 없어요. 잠시 후 다시 시도해 주세요.',
    })
  })

  it('redacts raw host persistence errors into a safe operator message', async () => {
    const service = createOperatorDiagnosticsService({
      gateway: {
        async loadOperatorRecoverySummary() {
          throw {
            code: 'session-persistence-failed',
            message: '앱 데이터 경로를 확인하지 못했어요: C:\\secret\\session.json',
          }
        },
        async loadOperatorAuditHistory() {
          return createOperatorAuditHistory()
        },
        async runOperatorRecoveryAction() {
          return createOperatorRecoveryActionResult()
        },
      },
    })

    await expect(service.loadOperatorRecoverySummary()).rejects.toMatchObject({
      code: 'session-persistence-failed',
      message:
        '지금은 현재 세션 진단을 불러올 수 없어요. 잠시 후 다시 시도해 주세요.',
    })
  })

  it('parses the typed operator audit history through the service seam', async () => {
    const service = createOperatorDiagnosticsService({
      gateway: {
        async loadOperatorRecoverySummary() {
          return createOperatorRecoverySummary()
        },
        async loadOperatorAuditHistory() {
          return createOperatorAuditHistory()
        },
        async runOperatorRecoveryAction() {
          return createOperatorRecoveryActionResult()
        },
      },
    })

    await expect(
      service.loadOperatorAuditHistory({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        eventCategories: [],
        limit: 10,
      }),
    ).resolves.toMatchObject({
      summary: {
        operatorInterventionEvents: 1,
      },
    })
  })

  it('rejects a foreign session audit response before it reaches the UI', async () => {
    const service = createOperatorDiagnosticsService({
      gateway: {
        async loadOperatorRecoverySummary() {
          return createOperatorRecoverySummary()
        },
        async loadOperatorAuditHistory() {
          return createOperatorAuditHistory({
            filter: {
              sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
              limit: 10,
            },
            events: [
              {
                ...createOperatorAuditHistory().events[0],
                sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
              },
            ],
          })
        },
        async runOperatorRecoveryAction() {
          return createOperatorRecoveryActionResult()
        },
      },
    })

    await expect(
      service.loadOperatorAuditHistory({
        sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
        eventCategories: [],
        limit: 10,
      }),
    ).rejects.toMatchObject({
      code: 'host-unavailable',
      message:
        '지금은 현재 세션 진단을 불러올 수 없어요. 잠시 후 다시 시도해 주세요.',
    })
  })
})
