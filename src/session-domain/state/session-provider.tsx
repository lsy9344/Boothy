import type { ReactNode } from 'react'
import { useEffect, useEffectEvent, useRef, useState } from 'react'

import type {
  CaptureDeleteResult,
  CaptureRequestResult,
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
import { isSessionScopedAssetPath } from '../utils/session-scoped-asset-path'
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
type SessionScopedReadiness = NonNullable<SessionDraft['captureReadiness']>
type SessionScopedCapture = NonNullable<SessionScopedReadiness['latestCapture']>

function normalizeCaptureRenderStatus(
  capture: SessionScopedCapture,
): SessionScopedCapture['renderStatus'] {
  const hasReadyPreview =
    capture.preview.assetPath !== null && capture.preview.readyAtMs !== null
  const hasReadyFinal =
    capture.final.assetPath !== null && capture.final.readyAtMs !== null

  switch (capture.renderStatus) {
    case 'finalReady':
      if (hasReadyFinal) {
        return 'finalReady'
      }

      return hasReadyPreview ? 'previewReady' : 'captureSaved'
    case 'previewReady':
      return hasReadyPreview ? 'previewReady' : 'captureSaved'
    default:
      return capture.renderStatus
  }
}

function sanitizeCaptureForManifest(
  sessionId: string,
  capture: SessionScopedCapture,
): SessionScopedCapture | null {
  if (capture.sessionId !== sessionId) {
    return null
  }

  const isPreviewSafe =
    capture.preview.assetPath === null ||
    isSessionScopedAssetPath(sessionId, capture.preview.assetPath)
  const isFinalSafe =
    capture.final.assetPath === null ||
    isSessionScopedAssetPath(sessionId, capture.final.assetPath)

  if (isPreviewSafe && isFinalSafe) {
    return capture
  }

  return {
    ...capture,
    preview:
      isPreviewSafe
        ? capture.preview
        : {
            ...capture.preview,
            assetPath: null,
            readyAtMs: null,
          },
    final:
      isFinalSafe
        ? capture.final
        : {
            ...capture.final,
            assetPath: null,
            readyAtMs: null,
          },
    renderStatus: normalizeCaptureRenderStatus({
      ...capture,
      preview:
        isPreviewSafe
          ? capture.preview
          : {
              ...capture.preview,
              assetPath: null,
              readyAtMs: null,
            },
      final:
        isFinalSafe
          ? capture.final
          : {
              ...capture.final,
              assetPath: null,
              readyAtMs: null,
            },
    }),
  }
}

function mergeCaptureIntoManifest(
  manifest: SessionManifest | null,
  capture: SessionScopedCapture,
): SessionManifest | null {
  if (manifest === null || capture === null || capture === undefined) {
    return null
  }

  const safeCapture = sanitizeCaptureForManifest(manifest.sessionId, capture)

  if (safeCapture === null) {
    return null
  }

  const nextCaptures = [...manifest.captures]
  const existingIndex = nextCaptures.findIndex(
    (currentCapture) =>
      currentCapture !== undefined && currentCapture.captureId === safeCapture.captureId,
  )

  if (existingIndex === -1) {
    nextCaptures.push(safeCapture)
  } else {
    const existingCapture = nextCaptures[existingIndex]
    const shouldPreserveExistingPreview =
      existingCapture !== undefined &&
      safeCapture.renderStatus === 'finalReady' &&
      safeCapture.preview.assetPath === null &&
      safeCapture.preview.readyAtMs === null &&
      existingCapture.preview.assetPath !== null &&
      existingCapture.preview.readyAtMs !== null &&
      isSessionScopedAssetPath(manifest.sessionId, existingCapture.preview.assetPath)

    nextCaptures[existingIndex] = shouldPreserveExistingPreview
      ? {
          ...safeCapture,
          preview: existingCapture.preview,
        }
      : safeCapture
  }

  return {
    ...manifest,
    captures: nextCaptures,
  }
}

function sanitizeManifestForSession(
  sessionId: string,
  manifest: SessionManifest | null,
): SessionManifest | null {
  if (manifest === null || manifest.sessionId !== sessionId) {
    return null
  }

  return {
    ...manifest,
    captures: manifest.captures
      .map((capture) => sanitizeCaptureForManifest(sessionId, capture))
      .filter((capture): capture is SessionScopedCapture => capture !== null),
  }
}

function inferSurfaceStateFromSanitizedCapture(
  readiness: Pick<SessionScopedReadiness, 'canCapture' | 'reasonCode'>,
  latestCapture: SessionScopedCapture | null,
): SessionScopedReadiness['surfaceState'] {
  if (latestCapture?.renderStatus === 'previewReady') {
    return 'previewReady'
  }

  if (latestCapture?.renderStatus === 'previewWaiting') {
    return 'previewWaiting'
  }

  if (latestCapture?.renderStatus === 'captureSaved') {
    return readiness.reasonCode === 'preview-waiting'
      ? 'captureSaved'
      : readiness.canCapture
        ? 'captureReady'
        : 'blocked'
  }

  if (latestCapture?.renderStatus === 'finalReady') {
    return readiness.canCapture ? 'captureReady' : 'blocked'
  }

  if (readiness.reasonCode === 'preview-waiting') {
    return latestCapture === null ? 'captureSaved' : 'previewWaiting'
  }

  return readiness.canCapture ? 'captureReady' : 'blocked'
}

function sanitizeCaptureReadinessForSession(
  sessionId: string | null,
  readiness: SessionScopedReadiness,
) {
  const latestCapture = readiness.latestCapture ?? null
  const sanitizedLatestCapture =
    sessionId !== null &&
    latestCapture !== null &&
    latestCapture.sessionId === sessionId
      ? sanitizeCaptureForManifest(sessionId, latestCapture)
      : null
  const previewWasScrubbedWithoutSafeFinal =
    latestCapture !== null &&
    sanitizedLatestCapture !== null &&
    latestCapture.preview.assetPath !== null &&
    sanitizedLatestCapture.preview.assetPath === null &&
    sanitizedLatestCapture.final.assetPath === null
  const shouldUseCaptureSavedFallback =
    sessionId !== null &&
    sanitizedLatestCapture !== null &&
    previewWasScrubbedWithoutSafeFinal &&
    readiness.reasonCode === 'ready' &&
    readiness.canCapture &&
    readiness.primaryAction === 'capture'

  if (shouldUseCaptureSavedFallback) {
    return buildCaptureSavedFallbackReadiness(sessionId, sanitizedLatestCapture)
  }

  const safeLatestCapture = sanitizedLatestCapture
  const normalizedSurfaceState = inferSurfaceStateFromSanitizedCapture(
    readiness,
    safeLatestCapture,
  )

  if (
    safeLatestCapture === readiness.latestCapture &&
    normalizedSurfaceState === readiness.surfaceState
  ) {
    return readiness
  }

  return {
    ...readiness,
    surfaceState: normalizedSurfaceState,
    latestCapture: safeLatestCapture,
  }
}

function buildCaptureSavedFallbackReadiness(
  sessionId: string,
  capture: SessionScopedCapture,
): SessionScopedReadiness {
  return {
    schemaVersion: 'capture-readiness/v1',
    sessionId,
    surfaceState: 'captureSaved',
    customerState: 'Preview Waiting',
    canCapture: false,
    primaryAction: 'wait',
    customerMessage: '사진이 안전하게 저장되었어요.',
    supportMessage: '확인용 사진을 준비하고 있어요. 잠시만 기다려 주세요.',
    reasonCode: 'preview-waiting',
    latestCapture: capture,
  }
}

function buildCaptureFallbackReadiness(
  sessionId: string,
  capture: SessionScopedCapture,
): SessionScopedReadiness {
  switch (capture.renderStatus) {
    case 'finalReady':
    case 'previewReady':
      return {
        schemaVersion: 'capture-readiness/v1',
        sessionId,
        surfaceState: 'captureReady',
        customerState: 'Ready',
        canCapture: true,
        primaryAction: 'capture',
        customerMessage: '지금 촬영할 수 있어요.',
        supportMessage: '버튼을 누르면 바로 시작돼요.',
        reasonCode: 'ready',
        latestCapture: capture,
      }
    case 'previewWaiting':
      return {
        schemaVersion: 'capture-readiness/v1',
        sessionId,
        surfaceState: 'previewWaiting',
        customerState: 'Preview Waiting',
        canCapture: false,
        primaryAction: 'wait',
        customerMessage: '사진이 안전하게 저장되었어요.',
        supportMessage: '확인용 사진을 준비하고 있어요. 잠시만 기다려 주세요.',
        reasonCode: 'preview-waiting',
        latestCapture: capture,
      }
    case 'captureSaved':
    default:
      return buildCaptureSavedFallbackReadiness(sessionId, capture)
  }
}

function sanitizeCaptureRequestResultForSession(
  sessionId: string,
  result: CaptureRequestResult,
) {
  const safeCapture = sanitizeCaptureForManifest(sessionId, result.capture)

  return {
    ...result,
    capture: safeCapture ?? result.capture,
    readiness:
      result.readiness.sessionId === sessionId
        ? sanitizeCaptureReadinessForSession(sessionId, result.readiness)
        : sanitizeCaptureReadinessForSession(
            sessionId,
            buildCaptureFallbackReadiness(
              sessionId,
              safeCapture ?? result.capture,
            ),
          ),
  }
}

function sanitizeCaptureDeleteResultForSession(
  sessionId: string,
  result: CaptureDeleteResult,
) {
  const safeManifest = sanitizeManifestForSession(sessionId, result.manifest)

  return {
    ...result,
    manifest: safeManifest ?? result.manifest,
    readiness: sanitizeCaptureReadinessForSession(sessionId, result.readiness),
  }
}

function sanitizeHostErrorForSession(
  sessionId: string,
  hostError: HostErrorEnvelope,
): HostErrorEnvelope {
  if (hostError.readiness === undefined || hostError.readiness === null) {
    return hostError
  }

  if (hostError.readiness.sessionId !== sessionId) {
    return {
      ...hostError,
      readiness: undefined,
    }
  }

  return {
    ...hostError,
    readiness: sanitizeCaptureReadinessForSession(sessionId, hostError.readiness),
  }
}

function hasForeignReadinessForSession(
  sessionId: string,
  hostError: HostErrorEnvelope,
) {
  return (
    hostError.readiness !== undefined &&
    hostError.readiness !== null &&
    hostError.readiness.sessionId !== sessionId
  )
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
  const activePresetSelectionRequestVersionRef = useRef<number | null>(null)
  const activePresetSelectionVersionRef = useRef(0)
  const captureReadinessRequestVersionRef = useRef(0)
  const deleteCaptureRequestVersionRef = useRef(0)
  const requestCaptureRequestVersionRef = useRef(0)
  const capturePresetCatalogRetryKeyRef = useRef<string | null>(null)
  const capturePresetCatalogRetryCountRef = useRef(0)
  const [sessionDraft, setSessionDraft] = useState<SessionDraft>(DEFAULT_SESSION_DRAFT)
  const [isStarting, setIsStarting] = useState(false)
  const [isLoadingPresetCatalog, setIsLoadingPresetCatalog] = useState(false)
  const [isSelectingPreset, setIsSelectingPreset] = useState(false)
  const [isLoadingCaptureReadiness, setIsLoadingCaptureReadiness] = useState(false)
  const [isDeletingCapture, setIsDeletingCapture] = useState(false)
  const [isRequestingCapture, setIsRequestingCapture] = useState(false)
  const isStartingRef = useRef(false)
  const isLoadingPresetCatalogRef = useRef(false)
  const isSelectingPresetRef = useRef(false)
  const isDeletingCaptureRef = useRef(false)
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
    deleteCaptureRequestVersionRef.current += 1
    requestCaptureRequestVersionRef.current += 1
    isDeletingCaptureRef.current = false
    isRequestingCaptureRef.current = false
    setIsLoadingCaptureReadiness(false)
    setIsDeletingCapture(false)
    setIsRequestingCapture(false)
  }

  function clearTransientSessionActivity() {
    invalidatePresetCatalogRequests()
    invalidateCaptureRequests()
    activePresetSelectionVersionRef.current += 1
    activePresetSelectionRequestVersionRef.current = null
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
      presetSelectionMode: 'initial-selection',
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
                  activePresetId: null,
                  activePresetDisplayName: null,
                },
    }))
  }

  function beginPresetSwitch() {
    invalidatePresetCatalogRequests()

    setSessionDraft((current) => {
      if (
        current.sessionId === null ||
        current.manifest === null ||
        current.selectedPreset === null
      ) {
        return current
      }

      return {
        ...current,
        flowStep: 'preset-selection',
        presetSelectionMode: 'in-session-switch',
        presetCatalog: [],
        presetCatalogState: 'idle',
      }
    })
  }

  function cancelPresetSwitch() {
    if (sessionDraftRef.current.presetSelectionMode === 'in-session-switch') {
      activePresetSelectionVersionRef.current += 1
    }

    setSessionDraft((current) => {
      if (current.presetSelectionMode !== 'in-session-switch') {
        return current
      }

      return {
        ...current,
        flowStep: 'capture',
      }
    })
  }

  function applyReadinessState(readiness: SessionDraft['captureReadiness']) {
    if (readiness === null) {
      return
    }

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
      ...(() => {
        const safeReadiness = sanitizeCaptureReadinessForSession(
          current.manifest?.sessionId ?? current.sessionId,
          readiness,
        )
        const latestCapture = safeReadiness.latestCapture ?? null

        return {
          captureReadiness: safeReadiness,
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
                    stage: deriveLifecycleStage(
                      safeReadiness,
                      current.manifest.lifecycle.stage,
                    ),
                  },
                },
        }
      })(),
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
        presetSelectionMode: 'initial-selection',
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

    const requestVersion = activePresetSelectionVersionRef.current + 1

    activePresetSelectionVersionRef.current = requestVersion
    activePresetSelectionRequestVersionRef.current = requestVersion
    isSelectingPresetRef.current = true
    setIsSelectingPreset(true)

    try {
      const wasInSessionSwitch =
        sessionDraftRef.current.presetSelectionMode === 'in-session-switch'
      const result = await activePresetServiceRef.current.selectActivePreset(input)

      if (
        !hasActiveSession(input.sessionId) ||
        activePresetSelectionVersionRef.current !== requestVersion
      ) {
        throw createStaleSessionError(
          '이미 취소했거나 이전 상태의 룩 변경 응답이 늦게 도착했어요. 현재 룩을 유지할게요.',
        )
      }

      setSessionDraft((current) => ({
        ...current,
        flowStep: 'capture',
        selectedPreset: result.activePreset,
        presetSelectionMode: 'initial-selection',
        captureReadiness:
          wasInSessionSwitch && current.captureReadiness !== null
            ? current.captureReadiness
            : buildLocalCaptureReadiness({
                sessionId: input.sessionId,
                hasSession: true,
                hasPreset: true,
              }),
        manifest:
          wasInSessionSwitch &&
          current.manifest !== null &&
          current.manifest.sessionId === result.manifest.sessionId
            ? {
                ...result.manifest,
                activePresetId:
                  result.manifest.activePresetId ?? result.activePreset.presetId,
                captures: current.manifest.captures,
              }
            : {
                ...result.manifest,
                activePresetId:
                  result.manifest.activePresetId ?? result.activePreset.presetId,
              },
      }))

      return result
    } catch (error) {
      const hostError = error as HostErrorEnvelope

      if (
        !hasActiveSession(input.sessionId) ||
        activePresetSelectionVersionRef.current !== requestVersion
      ) {
        throw error
      }

      if (hostError.code === 'session-not-found') {
        resetToSessionStart()
      } else if (
        hostError.code === 'preset-not-available' &&
        sessionDraftRef.current.presetSelectionMode !== 'in-session-switch'
      ) {
        resetToPresetSelection()
      }

      throw error
    } finally {
      if (activePresetSelectionRequestVersionRef.current === requestVersion) {
        activePresetSelectionRequestVersionRef.current = null
        isSelectingPresetRef.current = false
        setIsSelectingPreset(false)
      }
    }
  }

  async function getCaptureReadiness(input: { sessionId: string }) {
    const requestVersion = captureReadinessRequestVersionRef.current + 1

    captureReadinessRequestVersionRef.current = requestVersion
    setIsLoadingCaptureReadiness(true)

    try {
      const readiness = await captureRuntimeServiceRef.current.getCaptureReadiness(input)
      const safeReadiness = sanitizeCaptureReadinessForSession(
        input.sessionId,
        readiness,
      )

      if (
        !hasActiveSession(input.sessionId) ||
        safeReadiness.sessionId !== input.sessionId ||
        captureReadinessRequestVersionRef.current !== requestVersion
      ) {
        throw createStaleSessionError(
          '이전 세션의 준비 상태 응답이 늦게 도착했어요. 현재 세션 상태를 다시 확인할게요.',
        )
      }

      applyReadinessState(safeReadiness)

      return safeReadiness
    } catch (error) {
      const rawHostError = error as HostErrorEnvelope
      const hostError = sanitizeHostErrorForSession(
        input.sessionId,
        rawHostError,
      )
      const hadForeignReadiness = hasForeignReadinessForSession(
        input.sessionId,
        rawHostError,
      )

      if (
        !hasActiveSession(input.sessionId) ||
        captureReadinessRequestVersionRef.current !== requestVersion
      ) {
        throw hostError
      }

      if (hostError.readiness?.sessionId === input.sessionId) {
        applyReadinessState(hostError.readiness)
      } else if (
        hostError.code === 'session-not-found' &&
        !hadForeignReadiness
      ) {
        resetToSessionStart()
      } else if (
        hostError.code === 'preset-not-available' &&
        !hadForeignReadiness
      ) {
        resetToPresetSelection()
      }

      throw hostError
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

      if (
        result.sessionId !== input.sessionId ||
        result.capture.sessionId !== input.sessionId
      ) {
        throw createStaleSessionError(
          '현재 세션 상태를 다시 확인할게요.',
          staleReadiness,
        )
      }

      const safeResult = sanitizeCaptureRequestResultForSession(
        input.sessionId,
        result,
      )

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
        ...(() => {
          const safeReadiness = sanitizeCaptureReadinessForSession(
            current.manifest?.sessionId ?? current.sessionId,
            safeResult.readiness,
          )
          const manifestWithCapture =
            current.manifest === null
              ? null
              : mergeCaptureIntoManifest(current.manifest, safeResult.capture) ??
                current.manifest
          const manifestWithLatestCapture =
            manifestWithCapture === null || safeReadiness.latestCapture === null
              ? manifestWithCapture
              : mergeCaptureIntoManifest(
                  manifestWithCapture,
                  safeReadiness.latestCapture,
                ) ?? manifestWithCapture

          return {
            captureReadiness: safeReadiness,
            manifest:
              manifestWithLatestCapture === null
                ? null
                : {
                    ...manifestWithLatestCapture,
                    lifecycle: {
                      ...manifestWithLatestCapture.lifecycle,
                      stage: deriveLifecycleStage(
                        safeReadiness,
                        current.manifest?.lifecycle.stage ?? null,
                      ),
                    },
                  },
          }
        })(),
      }))

      return safeResult
    } catch (error) {
      const rawHostError = error as HostErrorEnvelope
      const hostError = sanitizeHostErrorForSession(
        input.sessionId,
        rawHostError,
      )
      const hadForeignReadiness = hasForeignReadinessForSession(
        input.sessionId,
        rawHostError,
      )

      if (
        !hasActiveSession(input.sessionId) ||
        requestCaptureRequestVersionRef.current !== requestVersion
      ) {
        throw hostError
      }

      if (hostError.readiness?.sessionId === input.sessionId) {
        applyReadinessState(hostError.readiness)
      } else if (
        hostError.code === 'session-not-found' &&
        !hadForeignReadiness
      ) {
        resetToSessionStart()
      } else if (
        hostError.code === 'preset-not-available' &&
        !hadForeignReadiness
      ) {
        resetToPresetSelection()
      }

      throw hostError
    } finally {
      if (requestCaptureRequestVersionRef.current === requestVersion) {
        isRequestingCaptureRef.current = false
        setIsRequestingCapture(false)
      }
    }
  }

  async function deleteCapture(input: { sessionId: string; captureId: string }) {
    if (isDeletingCaptureRef.current) {
      throw {
        code: 'host-unavailable',
        message: '이미 사진을 정리하는 중이에요. 잠시만 기다려 주세요.',
      } satisfies HostErrorEnvelope
    }

    const requestVersion = deleteCaptureRequestVersionRef.current + 1

    deleteCaptureRequestVersionRef.current = requestVersion
    isDeletingCaptureRef.current = true
    setIsDeletingCapture(true)

    try {
      const result = await captureRuntimeServiceRef.current.deleteCapture(input)
      const hasMatchingActiveSession = hasActiveSession(input.sessionId)
      const staleReadiness = hasMatchingActiveSession
        ? buildCurrentCaptureFallbackReadiness()
        : undefined

      if (
        result.sessionId !== input.sessionId ||
        result.captureId !== input.captureId ||
        result.manifest.sessionId !== input.sessionId
      ) {
        throw createStaleSessionError(
          '현재 세션 상태를 다시 확인할게요.',
          staleReadiness,
        )
      }

      const safeResult = sanitizeCaptureDeleteResultForSession(
        input.sessionId,
        result,
      )

      if (
        !hasMatchingActiveSession ||
        deleteCaptureRequestVersionRef.current !== requestVersion
      ) {
        throw createStaleSessionError(
          '이전 세션의 사진 정리 응답이 늦게 도착했어요. 현재 세션에서 다시 확인해 주세요.',
          staleReadiness,
        )
      }

      setSessionDraft((current) => {
        const safeReadiness = sanitizeCaptureReadinessForSession(
          current.manifest?.sessionId ?? current.sessionId,
          safeResult.readiness,
        )
        const safeManifest =
          sanitizeManifestForSession(
            current.manifest?.sessionId ?? current.sessionId ?? input.sessionId,
            safeResult.manifest,
          ) ?? safeResult.manifest

        return {
          ...current,
          captureReadiness: safeReadiness,
          manifest: {
            ...safeManifest,
            lifecycle: {
              ...safeManifest.lifecycle,
              stage: deriveLifecycleStage(
                safeReadiness,
                safeManifest.lifecycle.stage,
              ),
            },
          },
        }
      })

      return safeResult
    } catch (error) {
      const rawHostError = error as HostErrorEnvelope
      const hostError = sanitizeHostErrorForSession(
        input.sessionId,
        rawHostError,
      )
      const hadForeignReadiness = hasForeignReadinessForSession(
        input.sessionId,
        rawHostError,
      )

      if (
        !hasActiveSession(input.sessionId) ||
        deleteCaptureRequestVersionRef.current !== requestVersion
      ) {
        throw hostError
      }

      if (hostError.readiness?.sessionId === input.sessionId) {
        applyReadinessState(hostError.readiness)
      } else if (
        hostError.code === 'session-not-found' &&
        !hadForeignReadiness
      ) {
        resetToSessionStart()
      } else if (
        hostError.code === 'preset-not-available' &&
        !hadForeignReadiness
      ) {
        resetToPresetSelection()
      }

      throw hostError
    } finally {
      if (deleteCaptureRequestVersionRef.current === requestVersion) {
        isDeletingCaptureRef.current = false
        setIsDeletingCapture(false)
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

    const sanitizedReadiness = sanitizeCaptureReadinessForSession(
      sessionDraftRef.current.manifest?.sessionId ?? sessionDraftRef.current.sessionId,
      readiness,
    )

    captureReadinessRequestVersionRef.current += 1
    setIsLoadingCaptureReadiness(false)

    if (shouldInvalidateCaptureRequest(sanitizedReadiness)) {
      requestCaptureRequestVersionRef.current += 1
      isRequestingCaptureRef.current = false
      setIsRequestingCapture(false)
    }

    applyReadinessState(sanitizedReadiness)
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
        isDeletingCapture,
        isRequestingCapture,
        sessionDraft,
        startSession,
        beginPresetSwitch,
        cancelPresetSwitch,
        loadPresetCatalog,
        selectActivePreset,
        getCaptureReadiness,
        deleteCapture,
        requestCapture,
      }}
    >
      {children}
    </SessionStateContext.Provider>
  )
}
