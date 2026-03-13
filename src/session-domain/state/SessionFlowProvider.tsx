import {
  createContext,
  useContext,
  useEffect,
  useEffectEvent,
  useReducer,
  useState,
  useRef,
  type ReactNode,
} from 'react'

import {
  presetSelectionSettingsService as defaultPresetSelectionSettingsService,
  type PresetSelectionSettingsService,
} from '../../branch-config/services/presetSelectionStore.js'
import type { TimingAlertAudioService } from '../../customer-flow/services/timingAlertAudio.js'
import {
  postEndOutcomeService as defaultPostEndOutcomeService,
  type PostEndOutcomeService,
} from '../../completion-handoff/services/postEndOutcomeService.js'
import { useBranchConfig } from '../../branch-config/useBranchConfig.js'
import {
  captureAdapter as defaultCaptureAdapter,
  type CaptureAdapter,
} from '../../capture-adapter/host/captureAdapter.js'
import {
  cameraAdapter as defaultCameraAdapter,
  type CameraReadinessAdapter,
} from '../../capture-adapter/host/cameraAdapter.js'
import { activePresetService } from '../../session-domain/services/activePresetService.js'
import { presetSelectionCopy } from '../../customer-flow/copy/presetSelectionCopy.js'
import { resolveOperationalBranchId } from '../../diagnostics-log/services/operationalLogContext.js'
import { createLifecycleLogger, type LifecycleLogger } from '../../diagnostics-log/services/lifecycleLogger.js'
import {
  presetCatalogService as defaultPresetCatalogService,
  type PresetCatalogAuditReason,
  type PresetCatalogLoadState,
  type PresetCatalogService,
} from '../../preset-catalog/services/presetCatalogService.js'
import type { SessionStartPayload } from '../../shared-contracts/dto/session.js'
import { getPresetCatalogEntryById, type PresetId } from '../../shared-contracts/presets/presetCatalog.js'
import {
  sessionTimingService as defaultSessionTimingService,
  type SessionTimingService,
} from '../../timing-policy/services/sessionTimingService.js'
import { deriveTimingThresholds } from '../../timing-policy/state/timingSelectors.js'
import type { SessionTimingState } from '../../shared-contracts/dto/sessionTiming.js'
import {
  presetSelectionService as defaultPresetSelectionService,
  resolveDefaultPresetId,
  type PresetSelectionService,
} from '../services/presetSelection.js'
import { validateSessionStartInput } from '../services/reservationValidation.js'
import { sessionLifecycleService, type SessionLifecycleService } from '../services/sessionLifecycle.js'
import { mergeCaptureConfidenceState } from './captureConfidenceState.js'
import {
  createContractFailurePreparationState,
  createInitialCustomerPreparationState,
  createThresholdEscalatedPreparationState,
  deriveCustomerPreparationState,
} from './customerPreparationState.js'
import {
  initialSessionState,
  sessionReducer,
  selectCaptureActionEnabled,
  type PresetSelectionFailure,
  type SessionPhase,
  type SessionState,
} from './sessionReducer.js'

const readyStateRevealDurationMs = 1200
const readinessWarningLeadMinutes = 5
const preparationPhoneEscalationLeadMinutes = 2
const maxBrowserTimerDurationMs = 2_147_483_647
const presetSelectionSessionIntegrityErrorCodes = new Set([
  'session.preset_selection_invalid_session',
  'session.preset_selection_session_not_found',
])

function resolvePresetCatalogFallbackReason(
  presetCatalogState: PresetCatalogLoadState,
): PresetCatalogAuditReason | null {
  if (presetCatalogState.status === 'unavailable') {
    return presetCatalogState.auditReason
  }

  if (presetCatalogState.status === 'ready' && presetCatalogState.source === 'approved-fallback') {
    return presetCatalogState.auditReason ?? null
  }

  return null
}

function createPresetSelectionFailure(error: unknown): PresetSelectionFailure {
  if (
    error &&
    typeof error === 'object' &&
    'errorCode' in error &&
    typeof error.errorCode === 'string' &&
    'message' in error &&
    typeof error.message === 'string'
  ) {
    return {
      ok: false,
      errorCode: error.errorCode as PresetSelectionFailure['errorCode'],
      message: error.message.trim() || 'Unexpected preset selection failure',
    }
  }

  const message =
    error instanceof Error && error.message.trim().length > 0
      ? error.message.trim()
      : 'Unexpected preset selection failure'

  return {
    ok: false,
    errorCode: 'session.preset_selection_failed',
    message,
  }
}

function derivePresetSelectionFeedback(failure: PresetSelectionFailure) {
  if (presetSelectionSessionIntegrityErrorCodes.has(failure.errorCode)) {
    return presetSelectionCopy.selectionRestartRequired
  }

  return presetSelectionCopy.selectionRetryRequired
}

function scheduleBoundedTimeout(
  callback: () => void,
  targetTimeMs: number,
  maxDelayMs = maxBrowserTimerDurationMs,
) {
  let timer: ReturnType<typeof globalThis.setTimeout> | undefined
  let cancelled = false

  const scheduleNextTick = () => {
    const remainingMs = targetTimeMs - Date.now()

    if (remainingMs <= 0) {
      callback()
      return
    }

    timer = globalThis.setTimeout(() => {
      if (cancelled) {
        return
      }

      scheduleNextTick()
    }, Math.min(remainingMs, maxDelayMs))
  }

  scheduleNextTick()

  return () => {
    cancelled = true

    if (timer !== undefined) {
      globalThis.clearTimeout(timer)
    }
  }
}

type SessionFlowContextValue = {
  presetCatalogState: PresetCatalogLoadState
  isActivePresetChangePending: boolean
  state: SessionState
  cancelDeletePhoto(): void
  applyActivePresetChange(presetId: PresetId): Promise<boolean>
  clearActiveSession(): void
  closeReview(): void
  confirmDeletePhoto(): Promise<void>
  confirmPresetSelection(): Promise<void>
  dismissReviewFeedback(): void
  openReview(): void
  requestCapture(): Promise<void>
  requestDeletePhoto(captureId: string): void
  selectReviewCapture(captureId: string): void
  selectPreset(presetId: PresetId): Promise<void>
  continueFromPreparation(): void
  startJourney(sessionName?: string): void
  updateField(field: keyof SessionStartPayload, value: string): void
  submitCheckIn(): Promise<void>
}

const SessionFlowContext = createContext<SessionFlowContextValue | null>(null)

type SessionFlowProviderProps = {
  children: ReactNode
  captureAdapter?: CaptureAdapter
  lifecycleService?: SessionLifecycleService
  cameraAdapter?: CameraReadinessAdapter
  lifecycleLogger?: LifecycleLogger
  presetCatalogService?: PresetCatalogService
  postEndOutcomeService?: PostEndOutcomeService
  sessionTimingService?: SessionTimingService
  presetSelectionService?: PresetSelectionService
  presetSelectionSettingsService?: PresetSelectionSettingsService
  timingAlertAudio?: TimingAlertAudioService
}

export function SessionFlowProvider({
  children,
  captureAdapter = defaultCaptureAdapter,
  lifecycleService = sessionLifecycleService,
  cameraAdapter = defaultCameraAdapter,
  lifecycleLogger = createLifecycleLogger(),
  presetCatalogService = defaultPresetCatalogService,
  postEndOutcomeService = defaultPostEndOutcomeService,
  sessionTimingService = defaultSessionTimingService,
  presetSelectionService = defaultPresetSelectionService,
  presetSelectionSettingsService = defaultPresetSelectionSettingsService,
}: SessionFlowProviderProps) {
  const { config } = useBranchConfig()
  const [state, dispatch] = useReducer(sessionReducer, initialSessionState)
  const [presetCatalogState, setPresetCatalogState] = useState<PresetCatalogLoadState>({
    status: 'loading',
  })
  const [isActivePresetChangePending, setIsActivePresetChangePending] = useState(false)
  const loggedReadinessSessionsRef = useRef<Set<string>>(new Set())
  const loggedPresetCatalogFallbacksRef = useRef<Set<string>>(new Set())
  const syncedReadinessSessionsRef = useRef<Set<string>>(new Set())
  const normalizedLastUsedPresetIdRef = useRef(false)
  const savedLastUsedPresetKeysRef = useRef<Set<string>>(new Set())
  const activePresetChangePendingRef = useRef(false)
  const activeSessionIdRef = useRef<string | null>(state.activeSession?.sessionId ?? null)

  activeSessionIdRef.current = state.activeSession?.sessionId ?? null
  const latestStateRef = useRef(state)

  useEffect(() => {
    latestStateRef.current = state
  }, [state])

  const syncSessionTimingFromSnapshot = useEffectEvent(
    (snapshot: NonNullable<SessionState['captureConfidence']>) => {
      if (state.activeSession?.sessionId !== snapshot.sessionId) {
        return
      }

      const nextTiming: SessionTimingState = state.sessionTiming
        ? {
            ...state.sessionTiming,
            actualShootEndAt: snapshot.shootEndsAt,
            lastTimingUpdateAt: snapshot.updatedAt,
          }
        : {
            reservationStartAt: state.activeSession.createdAt,
            actualShootEndAt: snapshot.shootEndsAt,
            sessionType: 'standard',
            operatorExtensionCount: 0,
            lastTimingUpdateAt: snapshot.updatedAt,
          }

      dispatch({
        type: 'session_timing_loaded',
        sessionId: snapshot.sessionId,
        timing: nextTiming,
      })
    },
  )

  const updateField = (field: keyof SessionStartPayload, value: string) => {
    dispatch({
      type: 'field_changed',
      field,
      value,
    })
  }

  const submitSessionStart = async (fields: SessionStartPayload) => {
    dispatch({ type: 'validation_started' })

    const validationResult = validateSessionStartInput(fields)

    if (!validationResult.ok) {
      dispatch({
        type: 'validation_failed',
        fieldErrors: validationResult.fieldErrors,
      })
      return
    }

    dispatch({ type: 'provisioning_started' })

    try {
      const result = await lifecycleService.startSession({
        ...validationResult.value,
        branchId: resolveOperationalBranchId(config.branchId),
      })

      if (!result.ok) {
        dispatch({
          type: 'provisioning_failed',
          errorCode: result.errorCode,
        })
        return
      }

      dispatch({
        type: 'provisioning_succeeded',
        session: result.value,
      })
      dispatch({
        type: 'readiness_changed',
        readiness: createInitialCustomerPreparationState(result.value.sessionId, config.branchPhoneNumber),
      })
    } catch {
      dispatch({
        type: 'provisioning_failed',
        errorCode: 'session.provisioning_failed',
      })
    }
  }

  const startJourney = (sessionName?: string) => {
    dispatch({ type: 'journey_started', sessionName })
  }

  const continueFromPreparation = () => {
    if (state.phase !== 'preparing' || state.readiness?.kind !== 'ready') {
      return
    }

    if (state.activePreset) {
      dispatch({ type: 'capture_surface_resumed' })
      return
    }

    dispatch({
      type: 'preset_selection_started',
      selectedPresetId: state.selectedPresetId,
    })
  }

  const clearActiveSession = () => {
    dispatch({ type: 'active_session_cleared' })
  }

  const selectReviewCapture = (captureId: string) => {
    dispatch({
      type: 'review_capture_selected',
      captureId,
    })
  }

  const openReview = () => {
    if (!state.reviewGallery?.items.length) {
      return
    }

    dispatch({ type: 'review_expanded_opened' })
  }

  const closeReview = () => {
    dispatch({ type: 'review_expanded_closed' })
  }

  const requestDeletePhoto = (captureId: string) => {
    dispatch({
      type: 'review_delete_requested',
      captureId,
    })
  }

  const cancelDeletePhoto = () => {
    dispatch({ type: 'review_delete_cancelled' })
  }

  const dismissReviewFeedback = () => {
    dispatch({ type: 'review_feedback_cleared' })
  }

  const confirmPresetSelection = async () => {
    if (
      state.phase !== 'preset-selection' ||
      !state.activeSession ||
      !state.selectedPresetId ||
      state.presetSelectionStatus === 'applying'
    ) {
      return
    }

    const requestedSessionId = state.activeSession.sessionId

    dispatch({
      type: 'preset_selection_applying',
      sessionId: requestedSessionId,
      selectedPresetId: state.selectedPresetId,
    })

    try {
      const result = await presetSelectionService.selectPreset({
        presetId: state.selectedPresetId,
        sessionId: requestedSessionId,
      })

      if (activeSessionIdRef.current !== requestedSessionId) {
        return
      }

      if (!result.ok) {
        dispatch({
          type: 'preset_selection_failed',
          sessionId: requestedSessionId,
          failure: result,
          feedback: derivePresetSelectionFeedback(result),
        })
        return
      }

      dispatch({
        type: 'preset_selection_succeeded',
        sessionId: requestedSessionId,
        activePreset: result.value.activePreset,
        selectedPresetId: result.value.activePreset.presetId,
      })

      try {
        const snapshot = await cameraAdapter.getCaptureConfidenceSnapshot({
          sessionId: requestedSessionId,
        })

        if (activeSessionIdRef.current === requestedSessionId && snapshot.sessionId === requestedSessionId) {
          dispatch({
            type: 'capture_confidence_updated',
            snapshot: mergeCaptureConfidenceState(latestStateRef.current.captureConfidence, snapshot),
          })
        }
      } catch {
        // The active capture-confidence watcher remains the source of truth if the refresh fails.
      }
    } catch (error) {
      if (activeSessionIdRef.current !== requestedSessionId) {
        return
      }

      const failure = createPresetSelectionFailure(error)

      dispatch({
        type: 'preset_selection_failed',
        sessionId: requestedSessionId,
        failure,
        feedback: derivePresetSelectionFeedback(failure),
      })
    }
  }

  const submitCheckIn = async () => {
    await submitSessionStart(state.fields)
  }

  const selectPreset = async (presetId: PresetId) => {
    if (
      state.phase !== 'preset-selection' ||
      state.presetSelectionStatus === 'applying'
    ) {
      return
    }

    dispatch({
      type: 'preset_candidate_selected',
      selectedPresetId: presetId,
    })
  }

  const applyActivePresetChange = async (presetId: PresetId) => {
    if (
      state.phase !== 'capture-ready' ||
      !state.activeSession ||
      state.activePreset?.presetId === presetId ||
      activePresetChangePendingRef.current
    ) {
      return false
    }

    try {
      activePresetChangePendingRef.current = true
      setIsActivePresetChangePending(true)

      const result = await activePresetService.applyPresetChange({
        sessionId: state.activeSession.sessionId,
        presetId,
      })

      if (activeSessionIdRef.current !== result.sessionId) {
        return false
      }

      const nextPreset = getPresetCatalogEntryById(result.activePresetId)

      if (!nextPreset) {
        return false
      }

      dispatch({
        type: 'active_preset_changed',
        activePreset: {
          presetId: nextPreset.id,
          displayName: nextPreset.name,
        },
      })
    } catch {
      return false
    } finally {
      activePresetChangePendingRef.current = false
      setIsActivePresetChangePending(false)
    }

    return true
  }

  const requestCapture = async () => {
    const currentState = latestStateRef.current

    if (
      currentState.phase !== 'capture-ready' ||
      !currentState.activeSession ||
      !currentState.activePreset ||
      activePresetChangePendingRef.current ||
      !selectCaptureActionEnabled(currentState) ||
      currentState.captureRequestStatus === 'requesting'
    ) {
      return
    }

    const requestId =
      typeof globalThis.crypto?.randomUUID === 'function'
        ? globalThis.crypto.randomUUID()
        : `capture-${Date.now()}`

    try {
      await cameraAdapter.requestCapture(
        {
          requestId,
          correlationId: currentState.activeSession.sessionId,
          sessionId: currentState.activeSession.sessionId,
          activePreset: {
            presetId: currentState.activePreset.presetId,
            label: currentState.activePreset.displayName,
          },
        },
        (event) => {
          if (event.payload.stage !== 'captureStarted') {
            return
          }

          dispatch({
            type: 'capture_request_started',
            captureId: event.payload.captureId,
            requestedAt: event.payload.lastUpdatedAt,
          })
        },
      )

      const snapshot = await cameraAdapter.getCaptureConfidenceSnapshot({
        sessionId: currentState.activeSession.sessionId,
      })

      if (latestStateRef.current.activeSession?.sessionId === snapshot.sessionId) {
        dispatch({
          type: 'capture_confidence_updated',
          snapshot: mergeCaptureConfidenceState(latestStateRef.current.captureConfidence, snapshot),
        })
      }
    } catch {
      // The capture-confidence watcher remains the source of truth if the immediate refresh fails.
    } finally {
      dispatch({ type: 'capture_request_finished' })
    }
  }

  const confirmDeletePhoto = async () => {
    if (!state.activeSession || !state.pendingDeleteCaptureId || state.reviewStatus === 'deleting') {
      return
    }

    const requestedSessionId = state.activeSession.sessionId
    const requestedManifestPath = state.activeSession.manifestPath
    const requestedCaptureId = state.pendingDeleteCaptureId

    dispatch({ type: 'review_delete_started' })

    try {
      const response = await captureAdapter.deleteSessionPhoto({
        sessionId: requestedSessionId,
        captureId: requestedCaptureId,
        manifestPath: requestedManifestPath,
      })

      if (activeSessionIdRef.current !== requestedSessionId) {
        return
      }

      dispatch({
        type: 'review_delete_succeeded',
        sessionId: requestedSessionId,
        response,
      })

      try {
        const snapshot = await cameraAdapter.getCaptureConfidenceSnapshot({
          sessionId: requestedSessionId,
        })

        if (activeSessionIdRef.current === requestedSessionId && snapshot.sessionId === requestedSessionId) {
          dispatch({
            type: 'capture_confidence_updated',
            snapshot: mergeCaptureConfidenceState(latestStateRef.current.captureConfidence, snapshot),
          })
        }
      } catch {
        // Keep the review/delete flow successful even if the latest-photo refresh fails.
      }
    } catch {
      if (activeSessionIdRef.current !== requestedSessionId) {
        return
      }

      dispatch({
        type: 'review_delete_failed',
        sessionId: requestedSessionId,
        feedback: '사진을 삭제하지 못했어요. 다시 시도해 주세요.',
      })
    }
  }

  const handleReadinessUpdate = useEffectEvent(
    (sessionId: string, readinessStatus: Parameters<typeof deriveCustomerPreparationState>[0]) => {
      if (state.activeSession?.sessionId !== sessionId) {
        return
      }

      dispatch({
        type: 'readiness_changed',
        readiness: deriveCustomerPreparationState(readinessStatus, config.branchPhoneNumber),
      })
    },
  )

  const handleCaptureConfidenceUpdate = useEffectEvent(
    (snapshot: NonNullable<SessionState['captureConfidence']>) => {
      if (state.activeSession?.sessionId !== snapshot.sessionId) {
        return
      }

      if (
        state.phase === 'preset-selection' ||
        (state.phase === 'preparing' && state.readiness?.kind === 'phone-required')
      ) {
        syncSessionTimingFromSnapshot(snapshot)
        return
      }

      dispatch({
        type: 'capture_confidence_updated',
        snapshot: mergeCaptureConfidenceState(state.captureConfidence, snapshot),
      })
    },
  )

  const handleSessionTimingLoaded = useEffectEvent((sessionId: string, timing: SessionTimingState) => {
    if (state.activeSession?.sessionId !== sessionId) {
      return
    }

    dispatch({
      type: 'session_timing_loaded',
      sessionId,
      timing,
    })
  })

  const handleReviewGalleryLoaded = useEffectEvent(
    (sessionId: string, gallery: NonNullable<SessionState['reviewGallery']>) => {
      if (state.activeSession?.sessionId !== sessionId) {
        return
      }

      dispatch({
        type: 'review_gallery_loaded',
        gallery,
      })
    },
  )

  useEffect(() => {
    let isActive = true

    void presetCatalogService.loadApprovedPresetCatalog().then((result) => {
      if (!isActive) {
        return
      }

      setPresetCatalogState(result)
    })

    return () => {
      isActive = false
    }
  }, [presetCatalogService])

  useEffect(() => {
    if (presetCatalogState.status !== 'ready' || normalizedLastUsedPresetIdRef.current) {
      return
    }

    normalizedLastUsedPresetIdRef.current = true
    let isActive = true

    void presetSelectionSettingsService.loadLastUsedPresetId().then((storedPresetId) => {
      if (!isActive || !storedPresetId) {
        return
      }

      const resolvedPresetId = resolveDefaultPresetId(storedPresetId)

      if (resolvedPresetId !== storedPresetId) {
        void presetSelectionSettingsService.saveLastUsedPresetId(resolvedPresetId).catch(() => undefined)
      }
    }).catch(() => undefined)

    return () => {
      isActive = false
    }
  }, [presetCatalogState.status, presetSelectionSettingsService])

  useEffect(() => {
    const preferencePersistencePhases = new Set<SessionPhase>(['capture-loading', 'capture-ready'])

    if (
      !state.activeSession ||
      !state.activePreset ||
      !preferencePersistencePhases.has(state.phase)
    ) {
      return
    }

    const persistenceKey = `${state.activeSession.sessionId}:${state.activePreset.presetId}`

    if (savedLastUsedPresetKeysRef.current.has(persistenceKey)) {
      return
    }

    savedLastUsedPresetKeysRef.current.add(persistenceKey)
    void presetSelectionSettingsService.saveLastUsedPresetId(state.activePreset.presetId).catch(() => {
      savedLastUsedPresetKeysRef.current.delete(persistenceKey)
    })
  }, [presetSelectionSettingsService, state.activePreset, state.activeSession, state.phase])

  useEffect(() => {
    if (!state.activeSession || state.sessionTiming) {
      return
    }

    let isActive = true
    const { manifestPath, sessionId } = state.activeSession

    const loadSessionTiming = async () => {
      try {
        const result = await sessionTimingService.getSessionTiming({
          manifestPath,
          sessionId,
        })

        if (!isActive || !result.ok) {
          return
        }

        handleSessionTimingLoaded(sessionId, result.value.timing)
      } catch {
        // Keep customer flow moving even if the display-time read fails.
      }
    }

    void loadSessionTiming()

    return () => {
      isActive = false
    }
  }, [sessionTimingService, state.activeSession, state.phase, state.sessionTiming])

  const isReadinessWatchActive =
    state.activeSession !== null &&
    ['preparing', 'preset-selection', 'capture-loading', 'capture-ready'].includes(state.phase)

  useEffect(() => {
    if (!isReadinessWatchActive || !state.activeSession) {
      return
    }

    let isActive = true
    let unsubscribe: (() => void) | undefined
    const sessionId = state.activeSession.sessionId
    const assignUnsubscribe = (nextUnsubscribe: () => void) => {
      if (!isActive) {
        nextUnsubscribe()
        return
      }

      unsubscribe = nextUnsubscribe
    }

    const syncReadiness = async () => {
      try {
        if (!syncedReadinessSessionsRef.current.has(sessionId)) {
          const snapshot = await cameraAdapter.getReadinessSnapshot({ sessionId })

          if (!isActive) {
            return
          }

          syncedReadinessSessionsRef.current.add(sessionId)
          handleReadinessUpdate(sessionId, snapshot)
        }

        assignUnsubscribe(await cameraAdapter.watchReadiness({
          sessionId,
          onStatus: (status) => {
            handleReadinessUpdate(sessionId, status)
          },
        }))
      } catch {
        if (!isActive) {
          return
        }

        dispatch({
          type: 'readiness_changed',
          readiness: createContractFailurePreparationState(
            sessionId,
            config.branchPhoneNumber,
            'Failed to load camera readiness snapshot or channel.',
          ),
        })
      }
    }

    void syncReadiness()

    return () => {
      isActive = false
      unsubscribe?.()
    }
  }, [cameraAdapter, config.branchPhoneNumber, isReadinessWatchActive, state.activeSession])

  useEffect(() => {
    if (state.phase !== 'preparing' || !state.activeSession || state.readiness?.kind !== 'ready') {
      return
    }

    if (state.activePreset) {
      dispatch({ type: 'capture_surface_resumed' })
      return
    }

    let isActive = true
    const readyStateTimer = globalThis.setTimeout(() => {
      if (!isActive) {
        return
      }

      dispatch({
        type: 'preset_selection_started',
        selectedPresetId: null,
      })
    }, readyStateRevealDurationMs)

    return () => {
      isActive = false
      globalThis.clearTimeout(readyStateTimer)
    }
  }, [state.activePreset, state.activeSession, state.phase, state.readiness, state.readiness?.kind])

  useEffect(() => {
    if (
      !['preparing', 'preset-selection', 'capture-loading', 'capture-ready'].includes(state.phase) ||
      !state.activeSession ||
      state.readiness?.kind !== 'ready'
    ) {
      return
    }

    if (loggedReadinessSessionsRef.current.has(state.activeSession.sessionId)) {
      return
    }

    loggedReadinessSessionsRef.current.add(state.activeSession.sessionId)

    void lifecycleLogger.recordReadinessReached({
      branchId: resolveOperationalBranchId(config.branchId),
      sessionId: state.activeSession.sessionId,
      sessionName: state.activeSession.sessionName,
    })
  }, [config.branchId, lifecycleLogger, state.activeSession, state.phase, state.readiness])

  useEffect(() => {
    const fallbackReason = resolvePresetCatalogFallbackReason(presetCatalogState)

    if (!fallbackReason || !lifecycleLogger.recordPresetCatalogFallback) {
      loggedPresetCatalogFallbacksRef.current.clear()
      return
    }

    const branchId = resolveOperationalBranchId(config.branchId)
    const auditKey = `${branchId}:${fallbackReason}`

    if (loggedPresetCatalogFallbacksRef.current.has(auditKey)) {
      return
    }

    loggedPresetCatalogFallbacksRef.current.clear()
    loggedPresetCatalogFallbacksRef.current.add(auditKey)

    void lifecycleLogger.recordPresetCatalogFallback({
      branchId,
      reason: fallbackReason,
      sessionId: state.activeSession?.sessionId,
      sessionName: state.activeSession?.sessionName,
    })
  }, [config.branchId, lifecycleLogger, presetCatalogState, state.activeSession])

  const isCaptureConfidenceActive =
    state.activeSession !== null &&
    (
      state.phase === 'preset-selection' ||
      (state.phase === 'preparing' &&
        (
          state.readiness?.kind === 'phone-required' ||
          state.activePreset !== null
        )) ||
      state.phase === 'capture-loading' ||
      state.phase === 'capture-ready'
    )

  useEffect(() => {
    if (!isCaptureConfidenceActive || !state.activeSession) {
      return
    }

    let isActive = true
    let unsubscribe: (() => void) | undefined
    const sessionId = state.activeSession.sessionId
    const assignUnsubscribe = (nextUnsubscribe: () => void) => {
      if (!isActive) {
        nextUnsubscribe()
        return
      }

      unsubscribe = nextUnsubscribe
    }

    const syncCaptureConfidence = async () => {
      try {
        const snapshot = await cameraAdapter.getCaptureConfidenceSnapshot({ sessionId })

        if (!isActive) {
          return
        }

        handleCaptureConfidenceUpdate(snapshot)

        assignUnsubscribe(await cameraAdapter.watchCaptureConfidence({
          sessionId,
          onSnapshot: (snapshot) => {
            handleCaptureConfidenceUpdate(snapshot)
          },
        }))
      } catch {
        if (!isActive) {
          return
        }

        if (state.phase === 'capture-loading' && state.readiness?.kind === 'ready') {
          dispatch({ type: 'capture_surface_resumed' })
        }
      }
    }

    void syncCaptureConfidence()

    return () => {
      isActive = false
      unsubscribe?.()
    }
  }, [cameraAdapter, isCaptureConfidenceActive, state.activeSession, state.phase, state.readiness?.kind])

  useEffect(() => {
    if (state.phase !== 'capture-ready' || !state.activeSession) {
      return
    }

    let isActive = true
    const { manifestPath, sessionId } = state.activeSession

    dispatch({ type: 'review_gallery_loading' })

    const syncReviewGallery = async () => {
      try {
        const gallery = await captureAdapter.loadSessionGallery({
          sessionId,
          manifestPath,
        })

        if (!isActive) {
          return
        }

        handleReviewGalleryLoaded(sessionId, gallery)
      } catch {
        if (!isActive) {
          return
        }

        dispatch({ type: 'review_gallery_failed' })
      }
    }

    void syncReviewGallery()

    return () => {
      isActive = false
    }
  }, [captureAdapter, state.activeSession, state.captureConfidence?.revision, state.phase])

  useEffect(() => {
    if (state.phase !== 'capture-ready' || !state.activeSession || !state.sessionTiming) {
      return
    }

    const { actualShootEndAt } = state.sessionTiming
    const shootEndAtMs = Date.parse(actualShootEndAt)
    let isActive = true
    let retryTimer: ReturnType<typeof globalThis.setTimeout> | undefined
    let cancelResolutionTimer: (() => void) | undefined
    const { manifestPath, sessionId } = state.activeSession

    const resolvePostEndOutcome = async () => {
      try {
        const result = await postEndOutcomeService.getPostEndOutcome({
          manifestPath,
          sessionId,
        })

        if (!isActive || activeSessionIdRef.current !== sessionId) {
          return
        }

        if (!result.ok) {
          if (result.errorCode === 'post_end.not_ready') {
            retryTimer = globalThis.setTimeout(() => {
              void resolvePostEndOutcome()
            }, 250)
          }

          return
        }

        dispatch({
          type: 'post_end_started',
          outcome: result.value,
        })
      } catch {
        return
      }
    }

    if (shootEndAtMs <= Date.now()) {
      void resolvePostEndOutcome()
    } else {
      cancelResolutionTimer = scheduleBoundedTimeout(() => {
        void resolvePostEndOutcome()
      }, shootEndAtMs)
    }

    return () => {
      isActive = false
      cancelResolutionTimer?.()

      if (retryTimer !== undefined) {
        globalThis.clearTimeout(retryTimer)
      }
    }
  }, [postEndOutcomeService, state.activeSession, state.phase, state.sessionTiming])

  useEffect(() => {
    if (
      !config.operationalToggles.enablePhoneEscalation ||
      state.phase !== 'preparing' ||
      !state.activeSession ||
      !state.sessionTiming ||
      !state.readiness ||
      state.readiness.kind !== 'preparing'
    ) {
      return
    }

    let isActive = true
    const { sessionId } = state.activeSession
    const { phoneEscalationAt } = deriveTimingThresholds(state.sessionTiming, {
      warningLeadMinutes: readinessWarningLeadMinutes,
      phoneEscalationDelayMinutes: preparationPhoneEscalationLeadMinutes * -1,
    })
    const delayMs = Math.max(0, Date.parse(phoneEscalationAt) - Date.now())

    if (delayMs > maxBrowserTimerDurationMs) {
      return
    }

    const escalationTimer = globalThis.setTimeout(() => {
      if (!isActive) {
        return
      }

      dispatch({
        type: 'readiness_changed',
        readiness: createThresholdEscalatedPreparationState(sessionId, config.branchPhoneNumber),
      })
    }, delayMs)

    return () => {
      isActive = false
      globalThis.clearTimeout(escalationTimer)
    }
  }, [
    config.branchPhoneNumber,
    config.operationalToggles.enablePhoneEscalation,
    state.activeSession,
    state.phase,
    state.readiness,
    state.sessionTiming,
  ])

  return (
    <SessionFlowContext.Provider
      value={{
        cancelDeletePhoto,
        applyActivePresetChange,
        clearActiveSession,
        closeReview,
        confirmDeletePhoto,
        confirmPresetSelection,
        continueFromPreparation,
        dismissReviewFeedback,
        isActivePresetChangePending,
        openReview,
        presetCatalogState,
        requestCapture,
        requestDeletePhoto,
        selectReviewCapture,
        selectPreset,
        startJourney,
        state,
        updateField,
        submitCheckIn,
      }}
    >
      {children}
    </SessionFlowContext.Provider>
  )
}

export function useSessionFlow() {
  const value = useContext(SessionFlowContext)

  if (!value) {
    throw new Error('SessionFlowProvider is required')
  }

  return value
}
