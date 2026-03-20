import type { ReactNode } from 'react'
import { useEffect, useEffectEvent, useRef, useState } from 'react'

import type {
  HostErrorEnvelope,
  SessionManifest,
  PresetCatalogResult,
  SessionStartInput,
} from '../../shared-contracts'
import {
  buildLocalCaptureReadiness,
  createCaptureRuntimeService,
  type CaptureRuntimeService,
} from '../../capture-adapter/services/capture-runtime'
import {
  createPresetCatalogService,
  type PresetCatalogService,
} from '../../preset-catalog/services/preset-catalog-service'
import {
  createActivePresetService,
  type ActivePresetService,
} from '../services/active-preset'
import {
  createStartSessionService,
  type StartSessionService,
} from '../services/start-session'
import { SessionStateContext } from './session-context'
import { DEFAULT_SESSION_DRAFT, type SessionDraft } from './session-draft'

type SessionProviderProps = {
  children: ReactNode
  sessionService?: StartSessionService
  presetCatalogService?: PresetCatalogService
  activePresetService?: ActivePresetService
  captureRuntimeService?: CaptureRuntimeService
}

const CAPTURE_PRESET_CATALOG_RETRY_MS = 1500
const CAPTURE_PRESET_CATALOG_MAX_RETRIES = 1

function mergeCaptureIntoManifest(
  manifest: SessionManifest | null,
  capture: NonNullable<
    NonNullable<SessionDraft['captureReadiness']>['latestCapture']
  >,
): SessionManifest | null {
  if (manifest === null || capture === null || capture === undefined) {
    return null
  }

  if (capture.sessionId !== manifest.sessionId) {
    return null
  }

  const nextCaptures = [...manifest.captures]
  const existingIndex = nextCaptures.findIndex(
    (currentCapture) =>
      currentCapture !== undefined && currentCapture.captureId === capture.captureId,
  )

  if (existingIndex === -1) {
    nextCaptures.push(capture)
  } else {
    nextCaptures[existingIndex] = capture
  }

  return {
    ...manifest,
    captures: nextCaptures,
  }
}

function deriveLifecycleStage(
  readiness: NonNullable<SessionDraft['captureReadiness']>,
  currentStage: string | null,
) {
  switch (readiness.surfaceState) {
    case 'captureSaved':
    case 'previewWaiting':
      return 'preview-waiting'
    case 'previewReady':
    case 'captureReady':
      return 'capture-ready'
    case 'blocked':
      if (readiness.reasonCode === 'phone-required') {
        return 'phone-required'
      }

      return currentStage ?? 'preparing'
    default:
      return currentStage ?? 'preparing'
  }
}

export function SessionProvider({
  children,
  sessionService: sessionServiceProp,
  presetCatalogService: presetCatalogServiceProp,
  activePresetService: activePresetServiceProp,
  captureRuntimeService: captureRuntimeServiceProp,
}: SessionProviderProps) {
  const sessionServiceRef = useRef(
    sessionServiceProp ?? createStartSessionService(),
  )
  const presetCatalogServiceRef = useRef(
    presetCatalogServiceProp ?? createPresetCatalogService(),
  )
  const activePresetServiceRef = useRef(
    activePresetServiceProp ?? createActivePresetService(),
  )
  const captureRuntimeServiceRef = useRef(
    captureRuntimeServiceProp ?? createCaptureRuntimeService(),
  )
  const sessionDraftRef = useRef(DEFAULT_SESSION_DRAFT)
  const presetCatalogRequestRef = useRef<Promise<PresetCatalogResult> | null>(null)
  const presetCatalogRequestSessionIdRef = useRef<string | null>(null)
  const presetCatalogRequestVersionRef = useRef(0)
  const captureReadinessRequestVersionRef = useRef(0)
  const requestCaptureRequestVersionRef = useRef(0)
  const capturePresetCatalogRetryKeyRef = useRef<string | null>(null)
  const capturePresetCatalogRetryCountRef = useRef(0)
  const [sessionDraft, setSessionDraft] = useState<SessionDraft>(DEFAULT_SESSION_DRAFT)
  const [isStarting, setIsStarting] = useState(false)
  const [isLoadingPresetCatalog, setIsLoadingPresetCatalog] = useState(false)
  const [isSelectingPreset, setIsSelectingPreset] = useState(false)
  const [isLoadingCaptureReadiness, setIsLoadingCaptureReadiness] = useState(false)
  const [isRequestingCapture, setIsRequestingCapture] = useState(false)
  const isStartingRef = useRef(false)
  const isLoadingPresetCatalogRef = useRef(false)
  const isSelectingPresetRef = useRef(false)
  const isRequestingCaptureRef = useRef(false)

  useEffect(() => {
    sessionDraftRef.current = sessionDraft
  }, [sessionDraft])

  function invalidatePresetCatalogRequests() {
    presetCatalogRequestVersionRef.current += 1
    presetCatalogRequestRef.current = null
    presetCatalogRequestSessionIdRef.current = null
    isLoadingPresetCatalogRef.current = false
    setIsLoadingPresetCatalog(false)
  }

  function invalidateCaptureRequests() {
    captureReadinessRequestVersionRef.current += 1
    requestCaptureRequestVersionRef.current += 1
    isRequestingCaptureRef.current = false
    setIsLoadingCaptureReadiness(false)
    setIsRequestingCapture(false)
  }

  function clearTransientSessionActivity() {
    invalidatePresetCatalogRequests()
    invalidateCaptureRequests()
    isSelectingPresetRef.current = false
    setIsSelectingPreset(false)
  }

  function resetToSessionStart() {
    clearTransientSessionActivity()
    setSessionDraft(DEFAULT_SESSION_DRAFT)
  }

  function resetToPresetSelection() {
    invalidatePresetCatalogRequests()
    invalidateCaptureRequests()
    setSessionDraft((current) => ({
      ...current,
      flowStep: 'preset-selection',
      selectedPreset: null,
      presetCatalog: [],
      presetCatalogState: 'idle',
      captureReadiness: null,
      manifest:
        current.manifest === null
          ? null
          : {
              ...current.manifest,
              activePreset: null,
            },
    }))
  }

  function applyReadinessState(readiness: SessionDraft['captureReadiness']) {
    if (readiness === null) {
      return
    }

    const latestCapture = readiness.latestCapture ?? null

    if (readiness.primaryAction === 'start-session') {
      resetToSessionStart()
      return
    }

    if (readiness.primaryAction === 'choose-preset') {
      resetToPresetSelection()
      return
    }

    setSessionDraft((current) => ({
      ...current,
      captureReadiness: readiness,
      manifest:
        current.manifest === null
          ? null
          : {
              ...(latestCapture === null
                ? current.manifest
                : mergeCaptureIntoManifest(current.manifest, latestCapture) ??
                  current.manifest),
              lifecycle: {
                ...current.manifest.lifecycle,
                stage: deriveLifecycleStage(readiness, current.manifest.lifecycle.stage),
              },
            },
    }))
  }

  function hasActiveSession(sessionId: string) {
    return sessionDraftRef.current.sessionId === sessionId
  }

  function buildCurrentCaptureFallbackReadiness() {
    const current = sessionDraftRef.current

    return (
      current.captureReadiness ??
      buildLocalCaptureReadiness({
        sessionId: current.sessionId,
        hasSession: current.sessionId !== null,
        hasPreset: current.selectedPreset !== null,
      })
    )
  }

  function shouldInvalidateCaptureRequest(
    readiness: SessionDraft['captureReadiness'],
  ) {
    return readiness !== null && (!readiness.canCapture || readiness.primaryAction !== 'capture')
  }

  function createStaleSessionError(
    message: string,
    readiness?: SessionDraft['captureReadiness'] | null,
  ): HostErrorEnvelope {
    return readiness === undefined || readiness === null
      ? {
          code: 'host-unavailable',
          message,
        }
      : {
          code: 'host-unavailable',
          message,
          readiness,
        }
  }

  async function startSession(input: SessionStartInput) {
    if (isStartingRef.current) {
      throw {
        code: 'host-unavailable',
        message: '이미 시작하는 중이에요. 잠시만 기다려 주세요.',
      } satisfies HostErrorEnvelope
    }

    isStartingRef.current = true
    setIsStarting(true)

    try {
      const result = await sessionServiceRef.current.startSession(input)
      const hasPreset = result.manifest.activePreset !== null

      clearTransientSessionActivity()
      setSessionDraft({
        flowStep: hasPreset ? 'capture' : 'preset-selection',
        sessionId: result.sessionId,
        boothAlias: result.boothAlias,
        selectedPreset: result.manifest.activePreset,
        presetCatalog: [],
        presetCatalogState: 'idle',
        captureReadiness: buildLocalCaptureReadiness({
          sessionId: result.sessionId,
          hasSession: true,
          hasPreset,
        }),
        manifest: result.manifest,
      })

      return result
    } finally {
      isStartingRef.current = false
      setIsStarting(false)
    }
  }

  async function loadPresetCatalog(input: { sessionId: string }) {
    if (
      isLoadingPresetCatalogRef.current &&
      presetCatalogRequestRef.current !== null &&
      presetCatalogRequestSessionIdRef.current === input.sessionId
    ) {
      return presetCatalogRequestRef.current
    }

    const requestVersion = presetCatalogRequestVersionRef.current + 1

    presetCatalogRequestVersionRef.current = requestVersion
    isLoadingPresetCatalogRef.current = true
    setIsLoadingPresetCatalog(true)
    setSessionDraft((current) => ({
      ...current,
      presetCatalogState: 'loading',
    }))

    const request = presetCatalogServiceRef.current
      .loadPresetCatalog(input)
      .then((result) => {
        if (
          !hasActiveSession(input.sessionId) ||
          presetCatalogRequestVersionRef.current !== requestVersion
        ) {
          throw createStaleSessionError(
            '이전 세션의 프리셋 응답이 늦게 도착했어요. 현재 세션으로 다시 불러올게요.',
          )
        }

        setSessionDraft((current) => ({
          ...current,
          presetCatalog: result.presets,
          presetCatalogState: result.state === 'ready' ? 'ready' : 'empty',
        }))

        return result
      })
      .catch((error) => {
        const hostError = error as HostErrorEnvelope

        if (
          !hasActiveSession(input.sessionId) ||
          presetCatalogRequestVersionRef.current !== requestVersion
        ) {
          throw error
        }

        if (hostError.code === 'session-not-found') {
          resetToSessionStart()
        } else {
          setSessionDraft((current) => ({
            ...current,
            presetCatalog: [],
            presetCatalogState: 'error',
          }))
        }

        throw error
      })
      .finally(() => {
        if (presetCatalogRequestVersionRef.current !== requestVersion) {
          return
        }

        isLoadingPresetCatalogRef.current = false
        presetCatalogRequestRef.current = null
        presetCatalogRequestSessionIdRef.current = null
        setIsLoadingPresetCatalog(false)
      })

    presetCatalogRequestRef.current = request
    presetCatalogRequestSessionIdRef.current = input.sessionId

    return request
  }

  async function selectActivePreset(input: {
    sessionId: string
    preset: { presetId: string; publishedVersion: string }
  }) {
    if (isSelectingPresetRef.current) {
      throw {
        code: 'host-unavailable',
        message: '선택을 저장하는 중이에요. 잠시만 기다려 주세요.',
      } satisfies HostErrorEnvelope
    }

    isSelectingPresetRef.current = true
    setIsSelectingPreset(true)

    try {
      const result = await activePresetServiceRef.current.selectActivePreset(input)

      if (!hasActiveSession(input.sessionId)) {
        throw createStaleSessionError(
          '이전 세션의 프리셋 선택 응답이 늦게 도착했어요. 현재 세션에서 다시 골라 주세요.',
        )
      }

      setSessionDraft((current) => ({
        ...current,
        flowStep: 'capture',
        selectedPreset: result.activePreset,
        captureReadiness: buildLocalCaptureReadiness({
          sessionId: input.sessionId,
          hasSession: true,
          hasPreset: true,
        }),
        manifest: result.manifest,
      }))

      return result
    } catch (error) {
      const hostError = error as HostErrorEnvelope

      if (!hasActiveSession(input.sessionId)) {
        throw error
      }

      if (hostError.code === 'session-not-found') {
        resetToSessionStart()
      } else if (hostError.code === 'preset-not-available') {
        resetToPresetSelection()
      }

      throw error
    } finally {
      isSelectingPresetRef.current = false
      setIsSelectingPreset(false)
    }
  }

  async function getCaptureReadiness(input: { sessionId: string }) {
    const requestVersion = captureReadinessRequestVersionRef.current + 1

    captureReadinessRequestVersionRef.current = requestVersion
    setIsLoadingCaptureReadiness(true)

    try {
      const readiness = await captureRuntimeServiceRef.current.getCaptureReadiness(input)

      if (
        !hasActiveSession(input.sessionId) ||
        readiness.sessionId !== input.sessionId ||
        captureReadinessRequestVersionRef.current !== requestVersion
      ) {
        throw createStaleSessionError(
          '이전 세션의 준비 상태 응답이 늦게 도착했어요. 현재 세션 상태를 다시 확인할게요.',
        )
      }

      applyReadinessState(readiness)

      return readiness
    } catch (error) {
      const hostError = error as HostErrorEnvelope

      if (
        !hasActiveSession(input.sessionId) ||
        captureReadinessRequestVersionRef.current !== requestVersion
      ) {
        throw error
      }

      if (hostError.readiness?.sessionId === input.sessionId) {
        applyReadinessState(hostError.readiness)
      } else if (hostError.code === 'session-not-found') {
        resetToSessionStart()
      } else if (hostError.code === 'preset-not-available') {
        resetToPresetSelection()
      }

      throw error
    } finally {
      if (captureReadinessRequestVersionRef.current === requestVersion) {
        setIsLoadingCaptureReadiness(false)
      }
    }
  }

  async function requestCapture(input: { sessionId: string }) {
    if (isRequestingCaptureRef.current) {
      throw {
        code: 'host-unavailable',
        message: '이미 촬영을 준비하는 중이에요. 잠시만 기다려 주세요.',
      } satisfies HostErrorEnvelope
    }

    const requestVersion = requestCaptureRequestVersionRef.current + 1

    requestCaptureRequestVersionRef.current = requestVersion
    isRequestingCaptureRef.current = true
    setIsRequestingCapture(true)

    try {
      const result = await captureRuntimeServiceRef.current.requestCapture(input)
      const hasMatchingActiveSession = hasActiveSession(input.sessionId)
      const staleReadiness = hasMatchingActiveSession
        ? buildCurrentCaptureFallbackReadiness()
        : undefined

      if (result.sessionId !== input.sessionId) {
        throw createStaleSessionError(
          '현재 세션 상태를 다시 확인할게요.',
          staleReadiness,
        )
      }

      if (
        !hasMatchingActiveSession ||
        requestCaptureRequestVersionRef.current !== requestVersion
      ) {
        throw createStaleSessionError(
          '이전 세션의 촬영 응답이 늦게 도착했어요. 현재 세션에서 다시 시도해 주세요.',
          staleReadiness,
        )
      }

      setSessionDraft((current) => ({
        ...current,
        captureReadiness: result.readiness,
        manifest:
          current.manifest === null
            ? null
            : {
                ...(mergeCaptureIntoManifest(current.manifest, result.capture) ??
                  current.manifest),
                lifecycle: {
                  ...current.manifest.lifecycle,
                  stage: deriveLifecycleStage(
                    result.readiness,
                    current.manifest.lifecycle.stage,
                  ),
                },
              },
      }))

      return result
    } catch (error) {
      const hostError = error as HostErrorEnvelope

      if (
        !hasActiveSession(input.sessionId) ||
        requestCaptureRequestVersionRef.current !== requestVersion
      ) {
        throw error
      }

      if (hostError.readiness?.sessionId === input.sessionId) {
        applyReadinessState(hostError.readiness)
      } else if (hostError.code === 'session-not-found') {
        resetToSessionStart()
      } else if (hostError.code === 'preset-not-available') {
        resetToPresetSelection()
      }

      throw error
    } finally {
      if (requestCaptureRequestVersionRef.current === requestVersion) {
        isRequestingCaptureRef.current = false
        setIsRequestingCapture(false)
      }
    }
  }

  const applyCaptureReadiness = useEffectEvent((sessionId: string) => {
    void getCaptureReadiness({ sessionId }).catch(() => undefined)
  })

  const syncSubscribedReadiness = useEffectEvent((readiness: SessionDraft['captureReadiness']) => {
    if (
      readiness === null ||
      readiness.sessionId !== sessionDraftRef.current.sessionId
    ) {
      return
    }

    captureReadinessRequestVersionRef.current += 1
    setIsLoadingCaptureReadiness(false)

    if (shouldInvalidateCaptureRequest(readiness)) {
      requestCaptureRequestVersionRef.current += 1
      isRequestingCaptureRef.current = false
      setIsRequestingCapture(false)
    }

    applyReadinessState(readiness)
  })

  const hydrateCapturePresetCatalog = useEffectEvent((sessionId: string) => {
    if (
      sessionDraftRef.current.flowStep !== 'capture' ||
      sessionDraftRef.current.sessionId !== sessionId ||
      sessionDraftRef.current.selectedPreset === null
    ) {
      return
    }

    void loadPresetCatalog({
      sessionId,
    }).catch(() => undefined)
  })

  useEffect(() => {
    const retryKey =
      sessionDraft.flowStep === 'capture' && sessionDraft.selectedPreset !== null
        ? `${sessionDraft.sessionId}:${sessionDraft.selectedPreset.presetId}:${sessionDraft.selectedPreset.publishedVersion}`
        : null

    if (capturePresetCatalogRetryKeyRef.current === retryKey) {
      return
    }

    capturePresetCatalogRetryKeyRef.current = retryKey
    capturePresetCatalogRetryCountRef.current = 0
  }, [
    sessionDraft.flowStep,
    sessionDraft.sessionId,
    sessionDraft.selectedPreset,
  ])

  useEffect(() => {
    if (sessionDraft.flowStep !== 'capture' || sessionDraft.sessionId === null) {
      return
    }

    let isDisposed = false
    let stopListening: (() => void) | undefined

    applyCaptureReadiness(sessionDraft.sessionId)

    void captureRuntimeServiceRef.current
      .subscribeToCaptureReadiness({
        sessionId: sessionDraft.sessionId,
        onReadiness(readiness) {
          if (!isDisposed) {
            syncSubscribedReadiness(readiness)
          }
        },
      })
      .then((unlisten) => {
        if (isDisposed) {
          unlisten()
          return
        }

        stopListening = unlisten
      })
      .catch(() => undefined)

    return () => {
      isDisposed = true

      if (stopListening) {
        stopListening()
      }
    }
  }, [
    sessionDraft.flowStep,
    sessionDraft.sessionId,
  ])

  useEffect(() => {
    if (
      sessionDraft.flowStep !== 'capture' ||
      sessionDraft.sessionId === null ||
      sessionDraft.selectedPreset === null ||
      sessionDraft.presetCatalog.length > 0
    ) {
      return
    }

    if (sessionDraft.presetCatalogState === 'idle') {
      hydrateCapturePresetCatalog(sessionDraft.sessionId)
      return
    }

    if (sessionDraft.presetCatalogState !== 'error') {
      return
    }

    if (
      capturePresetCatalogRetryKeyRef.current === null ||
      capturePresetCatalogRetryCountRef.current >= CAPTURE_PRESET_CATALOG_MAX_RETRIES
    ) {
      return
    }

    capturePresetCatalogRetryCountRef.current += 1

    const retryId = globalThis.setTimeout(() => {
      hydrateCapturePresetCatalog(sessionDraft.sessionId!)
    }, CAPTURE_PRESET_CATALOG_RETRY_MS)

    return () => {
      globalThis.clearTimeout(retryId)
    }
  }, [
    sessionDraft.flowStep,
    sessionDraft.sessionId,
    sessionDraft.selectedPreset,
    sessionDraft.presetCatalogState,
    sessionDraft.presetCatalog.length,
  ])

  return (
    <SessionStateContext.Provider
      value={{
        isStarting,
        isLoadingPresetCatalog,
        isSelectingPreset,
        isLoadingCaptureReadiness,
        isRequestingCapture,
        sessionDraft,
        startSession,
        loadPresetCatalog,
        selectActivePreset,
        getCaptureReadiness,
        requestCapture,
      }}
    >
      {children}
    </SessionStateContext.Provider>
  )
}
