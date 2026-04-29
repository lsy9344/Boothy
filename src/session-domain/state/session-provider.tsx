import type { ReactNode } from 'react'
import { useEffect, useEffectEvent, useRef, useState } from 'react'

import type {
  CaptureDeleteResult,
  CaptureRequestInput,
  CaptureRequestResult,
  HostErrorEnvelope,
  SessionManifest,
  PresetCatalogResult,
  SessionStartInput,
} from '../../shared-contracts'
import {
  buildLocalCaptureReadiness,
  createCaptureRuntimeService,
  generateCaptureRequestId,
  type CaptureRuntimeService,
} from '../../capture-adapter/services/capture-runtime'
import {
  getPostEndStrength,
  isHostOwnedPostEndReason,
} from '../../completion-handoff/post-end'
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
import { isTauriRuntime } from '../../shared/runtime/is-tauri'
import { logCaptureClientState } from '../../shared/runtime/log-capture-client-state'

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
type PendingFastPreview = NonNullable<SessionDraft['pendingFastPreview']>

function sanitizePendingFastPreviewForSession(
  sessionId: string | null,
  pendingFastPreview: SessionDraft['pendingFastPreview'],
): PendingFastPreview | null {
  if (
    sessionId === null ||
    pendingFastPreview === null ||
    pendingFastPreview.sessionId !== sessionId ||
    !isSessionScopedAssetPath(sessionId, pendingFastPreview.assetPath)
  ) {
    return null
  }

  return pendingFastPreview
}

function matchesPendingFastPreviewCapture(
  pendingFastPreview: PendingFastPreview | null,
  capture: SessionScopedCapture | null,
) {
  return (
    pendingFastPreview !== null &&
    capture !== null &&
    pendingFastPreview.captureId === capture.captureId &&
    pendingFastPreview.requestId === capture.requestId
  )
}

function hasDisplayablePreviewAsset(
  sessionId: string | null,
  capture: SessionScopedCapture | null,
) {
  return (
    sessionId !== null &&
    capture !== null &&
    capture.preview.assetPath !== null &&
    isSessionScopedAssetPath(sessionId, capture.preview.assetPath)
  )
}

function mergePreservedPostEndReadiness(
  current: SessionScopedReadiness | null,
  next: SessionScopedReadiness,
): SessionScopedReadiness {
  if (current === null) {
    return next
  }

  const shouldPreserveCurrentPostEnd =
    current.postEnd !== null && current.postEnd !== undefined
  const currentStrength = getPostEndStrength(current.reasonCode)
  const nextStrength = getPostEndStrength(next.reasonCode)

  if (shouldPreserveCurrentPostEnd && currentStrength >= 1 && nextStrength === 0) {
    return current
  }

  if (
    isHostOwnedPostEndReason(current.reasonCode) &&
    current.reasonCode === next.reasonCode &&
    current.postEnd !== null &&
    next.postEnd === null
  ) {
    return {
      ...next,
      postEnd: current.postEnd,
    }
  }

  return next
}

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
  if (isHostOwnedPostEndReason(readiness.reasonCode) || readiness.reasonCode === 'ended') {
    return 'blocked'
  }

  if (latestCapture?.renderStatus === 'previewReady') {
    return readiness.canCapture ? 'previewReady' : 'blocked'
  }

  if (latestCapture?.renderStatus === 'previewWaiting') {
    return readiness.reasonCode === 'preview-waiting'
      ? 'previewWaiting'
      : readiness.canCapture
        ? 'captureReady'
        : 'blocked'
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
    postEnd: null,
    timing: null,
  }
}

function buildCaptureFallbackReadiness(
  sessionId: string,
  capture: SessionScopedCapture,
): SessionScopedReadiness {
  if (
    capture.postEndState === 'handoffReady' ||
    capture.postEndState === 'completed'
  ) {
    return {
      schemaVersion: 'capture-readiness/v1',
      sessionId,
      surfaceState: 'blocked',
      customerState: 'Completed',
      canCapture: false,
      primaryAction: 'wait',
      customerMessage: '부스 준비가 끝났어요.',
      supportMessage: '마지막 안내를 확인해 주세요.',
      reasonCode: 'completed',
      latestCapture: capture,
      postEnd: null,
      timing: null,
    }
  }

  switch (capture.renderStatus) {
    case 'finalReady':
    case 'previewReady':
      return {
        schemaVersion: 'capture-readiness/v1',
        sessionId,
        surfaceState: 'blocked',
        customerState: 'Preparing',
        canCapture: false,
        primaryAction: 'wait',
        customerMessage: '카메라 연결 상태를 다시 확인하는 중이에요.',
        supportMessage: '잠시만 기다려 주세요.',
        reasonCode: 'camera-preparing',
        latestCapture: capture,
        postEnd: null,
        timing: null,
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
        postEnd: null,
        timing: null,
      }
    case 'captureSaved':
    default:
      return buildCaptureSavedFallbackReadiness(sessionId, capture)
  }
}

function logCaptureDebug(label: string, details: Record<string, unknown>) {
  if (typeof console === 'undefined') {
    return
  }

  console.info(`[boothy][capture] ${label}`, details)
  void logCaptureClientState({
    label,
    sessionId:
      typeof details.sessionId === 'string' ? details.sessionId : undefined,
    runtimeMode:
      typeof details.runtimeMode === 'string' ? details.runtimeMode : undefined,
    customerState:
      typeof details.customerState === 'string' ? details.customerState : undefined,
    reasonCode:
      typeof details.reasonCode === 'string' ? details.reasonCode : undefined,
    canCapture:
      typeof details.canCapture === 'boolean' ? details.canCapture : undefined,
    message:
      typeof details.message === 'string' ? details.message : undefined,
  })
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

function isExplicitPostEndReadiness(
  readiness: NonNullable<SessionDraft['captureReadiness']>,
) {
  return (
    isHostOwnedPostEndReason(readiness.reasonCode) ||
    (readiness.postEnd !== undefined &&
      readiness.postEnd !== null &&
      (readiness.postEnd.state === 'export-waiting' ||
        readiness.postEnd.state === 'completed' ||
        readiness.postEnd.state === 'phone-required'))
  )
}

function resolveExplicitPostEndState(
  readiness: NonNullable<SessionDraft['captureReadiness']>,
) {
  if (readiness.postEnd !== undefined && readiness.postEnd !== null) {
    return readiness.postEnd.state
  }

  switch (readiness.reasonCode) {
    case 'completed':
      return 'completed'
    case 'phone-required':
      return 'phone-required'
    case 'export-waiting':
      return 'export-waiting'
    default:
      return null
  }
}

function lockStablePostEndReadiness(
  currentReadiness: SessionDraft['captureReadiness'],
  nextReadiness: NonNullable<SessionDraft['captureReadiness']>,
) {
  if (
    currentReadiness === null ||
    currentReadiness.sessionId !== nextReadiness.sessionId
  ) {
    return nextReadiness
  }

  return mergePreservedPostEndReadiness(currentReadiness, {
    ...nextReadiness,
    latestCapture: nextReadiness.latestCapture ?? currentReadiness.latestCapture ?? null,
    timing: nextReadiness.timing ?? currentReadiness.timing ?? null,
  })
}

function deriveLifecycleStage(
  readiness: NonNullable<SessionDraft['captureReadiness']>,
  currentStage: string | null,
) {
  const explicitPostEndState = isExplicitPostEndReadiness(readiness)
    ? resolveExplicitPostEndState(readiness)
    : null

  if (explicitPostEndState !== null) {
    return explicitPostEndState
  }

  if (readiness.timing?.phase === 'ended') {
    switch (currentStage) {
      case 'phone-required':
      case 'completed':
        return currentStage
      default:
        return 'export-waiting'
    }
  }

  if (readiness.reasonCode === 'warning' || readiness.timing?.phase === 'warning') {
    switch (currentStage) {
      case 'preview-waiting':
      case 'export-waiting':
      case 'completed':
      case 'phone-required':
      case 'ended':
        return currentStage
      default:
        return 'warning'
    }
  }

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

function mergeTimingIntoManifest(
  manifest: SessionManifest | null,
  readiness: SessionDraft['captureReadiness'],
) {
  if (manifest === null || readiness?.timing === null || readiness?.timing === undefined) {
    return manifest
  }

  if (readiness.timing.sessionId !== manifest.sessionId) {
    return manifest
  }

  return {
    ...manifest,
    timing: readiness.timing,
  }
}

function mergePostEndIntoManifest(
  manifest: SessionManifest | null,
  readiness: SessionDraft['captureReadiness'],
) {
  if (manifest === null) {
    return manifest
  }

  if (readiness?.postEnd !== undefined) {
    if (
      readiness.postEnd === null &&
      readiness !== null &&
      isHostOwnedPostEndReason(readiness.reasonCode)
    ) {
      return {
        ...manifest,
        postEnd: manifest.postEnd,
      }
    }

    return {
      ...manifest,
      postEnd: readiness.postEnd,
    }
  }

  if (readiness?.reasonCode === 'ended' || readiness?.timing?.phase === 'ended') {
    return {
      ...manifest,
      postEnd: null,
    }
  }

  return {
    ...manifest,
    postEnd: manifest.postEnd,
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
  const deletedCaptureIdsRef = useRef<Set<string>>(new Set())
  const activeCaptureRequestIdRef = useRef<string | null>(null)
  const activeCaptureIdRef = useRef<string | null>(null)
  const capturePresetCatalogRetryKeyRef = useRef<string | null>(null)
  const capturePresetCatalogRetryCountRef = useRef(0)
  const capturePreviewRuntimePrimeKeyRef = useRef<string | null>(null)
  const capturePreviewRuntimePrimePromiseRef = useRef<Promise<void> | null>(null)
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
    activeCaptureRequestIdRef.current = null
    activeCaptureIdRef.current = null
    isDeletingCaptureRef.current = false
    isRequestingCaptureRef.current = false
    setIsLoadingCaptureReadiness(false)
    setIsDeletingCapture(false)
    setIsRequestingCapture(false)
  }

  function clearCapturePreviewRuntimePrime() {
    capturePreviewRuntimePrimeKeyRef.current = null
    capturePreviewRuntimePrimePromiseRef.current = null
  }

  function ensureCapturePreviewRuntimePrime(input: {
    sessionId: string | null
    preset: { presetId: string; publishedVersion: string } | null
  }) {
    if (input.sessionId === null || input.preset === null) {
      return capturePreviewRuntimePrimePromiseRef.current
    }

    const primeKey = `${input.sessionId}:${input.preset.presetId}:${input.preset.publishedVersion}`

    if (capturePreviewRuntimePrimeKeyRef.current === primeKey) {
      return capturePreviewRuntimePrimePromiseRef.current ?? Promise.resolve()
    }

    capturePreviewRuntimePrimeKeyRef.current = primeKey

    const primePromise =
      captureRuntimeServiceRef.current
        .primePreviewRuntime?.({
          sessionId: input.sessionId,
          presetId: input.preset.presetId,
          publishedVersion: input.preset.publishedVersion,
        })
        .catch(() => {
          if (capturePreviewRuntimePrimeKeyRef.current === primeKey) {
            capturePreviewRuntimePrimeKeyRef.current = null
          }
        })

    if (primePromise === undefined) {
      capturePreviewRuntimePrimePromiseRef.current = null
      return null
    }

    capturePreviewRuntimePrimePromiseRef.current = primePromise
    void primePromise.finally(() => {
      if (capturePreviewRuntimePrimePromiseRef.current === primePromise) {
        capturePreviewRuntimePrimePromiseRef.current = null
      }
    })

    return primePromise
  }

  function prunePendingFastPreview(
    sessionId: string | null,
    pendingFastPreview: SessionDraft['pendingFastPreview'],
    latestCapture: SessionScopedCapture | null,
  ) {
    const safePendingFastPreview = sanitizePendingFastPreviewForSession(
      sessionId,
      pendingFastPreview,
    )

    if (safePendingFastPreview === null) {
      return null
    }

    if (deletedCaptureIdsRef.current.has(safePendingFastPreview.captureId)) {
      return null
    }

    if (latestCapture === null) {
      return safePendingFastPreview
    }

    if (!matchesPendingFastPreviewCapture(safePendingFastPreview, latestCapture)) {
      return null
    }

    return hasDisplayablePreviewAsset(sessionId, latestCapture)
      ? null
      : safePendingFastPreview
  }

  function clearTransientSessionActivity() {
    invalidatePresetCatalogRequests()
    invalidateCaptureRequests()
    clearCapturePreviewRuntimePrime()
    deletedCaptureIdsRef.current.clear()
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
      pendingFastPreview: null,
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
        current.selectedPreset === null ||
        current.captureReadiness === null ||
        !current.captureReadiness.canCapture ||
        isExplicitPostEndReadiness(current.captureReadiness)
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

  function suppressDeletedLatestCapture(
    readiness: SessionScopedReadiness,
  ): SessionScopedReadiness {
    const latestCapture = readiness.latestCapture ?? null

    if (
      latestCapture === null ||
      !deletedCaptureIdsRef.current.has(latestCapture.captureId)
    ) {
      return readiness
    }

    return {
      ...readiness,
      surfaceState: inferSurfaceStateFromSanitizedCapture(readiness, null),
      latestCapture: null,
    }
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

    logCaptureDebug('apply-readiness-state', {
      sessionId: readiness.sessionId,
      customerState: readiness.customerState,
      reasonCode: readiness.reasonCode,
      canCapture: readiness.canCapture,
    })

    setSessionDraft((current) => ({
      ...current,
      ...(() => {
        const safePendingFastPreview = sanitizePendingFastPreviewForSession(
          current.manifest?.sessionId ?? current.sessionId,
          current.pendingFastPreview,
        )
        const safeReadiness = suppressDeletedLatestCapture(
          lockStablePostEndReadiness(
            current.captureReadiness,
            sanitizeCaptureReadinessForSession(
              current.manifest?.sessionId ?? current.sessionId,
              readiness,
            ),
          ),
        )
        const latestCapture = safeReadiness.latestCapture ?? null

        return {
          captureReadiness: safeReadiness,
          pendingFastPreview: prunePendingFastPreview(
            current.manifest?.sessionId ?? current.sessionId,
            safePendingFastPreview,
            latestCapture,
          ),
          manifest:
            current.manifest === null
              ? null
              : {
                  ...(mergePostEndIntoManifest(
                    mergeTimingIntoManifest(
                      latestCapture === null
                        ? current.manifest
                        : mergeCaptureIntoManifest(current.manifest, latestCapture) ??
                          current.manifest,
                      safeReadiness,
                    ),
                    safeReadiness,
                  ) ?? current.manifest),
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
      if (hasPreset) {
        ensureCapturePreviewRuntimePrime({
          sessionId: result.sessionId,
          preset: result.manifest.activePreset,
        })
      }
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
        pendingFastPreview: null,
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

      ensureCapturePreviewRuntimePrime({
        sessionId: input.sessionId,
        preset: result.activePreset,
      })

      setSessionDraft((current) => ({
        ...current,
        flowStep: 'capture',
        selectedPreset: result.activePreset,
        presetSelectionMode: 'initial-selection',
        pendingFastPreview: null,
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
      } else if (
        hostError.readiness !== undefined &&
        hostError.readiness !== null &&
        hostError.readiness.sessionId === input.sessionId
      ) {
        applyReadinessState(hostError.readiness)
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
      logCaptureDebug('request-readiness', {
        sessionId: input.sessionId,
        runtimeMode: isTauriRuntime() ? 'tauri' : 'browser',
      })
      const readiness = await captureRuntimeServiceRef.current.getCaptureReadiness(input)
      const safeReadiness = sanitizeCaptureReadinessForSession(
        input.sessionId,
        readiness,
      )
      logCaptureDebug('apply-readiness-response', {
        sessionId: safeReadiness.sessionId,
        customerState: safeReadiness.customerState,
        reasonCode: safeReadiness.reasonCode,
        canCapture: safeReadiness.canCapture,
      })

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
      logCaptureDebug('readiness-error', {
        sessionId: input.sessionId,
        code: hostError.code,
        message: hostError.message,
        hasReadiness: hostError.readiness !== undefined && hostError.readiness !== null,
      })
      const hadForeignReadiness = hasForeignReadinessForSession(
        input.sessionId,
        rawHostError,
      )
      const shouldPreserveCurrentSession =
        hostError.code === 'session-not-found' &&
        !hadForeignReadiness &&
        isRequestingCaptureRef.current

      if (
        !hasActiveSession(input.sessionId) ||
        captureReadinessRequestVersionRef.current !== requestVersion
      ) {
        throw hostError
      }

      if (shouldPreserveCurrentSession) {
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

  async function requestCapture(input: CaptureRequestInput) {
    if (isRequestingCaptureRef.current) {
      throw {
        code: 'host-unavailable',
        message: '이미 촬영을 준비하는 중이에요. 잠시만 기다려 주세요.',
      } satisfies HostErrorEnvelope
    }

    const requestVersion = requestCaptureRequestVersionRef.current + 1
    const requestId = input.requestId ?? generateCaptureRequestId()

    requestCaptureRequestVersionRef.current = requestVersion
    activeCaptureRequestIdRef.current = requestId
    activeCaptureIdRef.current = null
    isRequestingCaptureRef.current = true
    setIsRequestingCapture(true)
    setSessionDraft((current) => ({
      ...current,
      pendingFastPreview: null,
    }))

    try {
      const activePreset =
        sessionDraftRef.current.selectedPreset ??
        sessionDraftRef.current.manifest?.activePreset ??
        null
      await ensureCapturePreviewRuntimePrime({
        sessionId: input.sessionId,
        preset:
          activePreset !== null &&
          sessionDraftRef.current.sessionId === input.sessionId
            ? activePreset
            : null,
      })

      const result = await captureRuntimeServiceRef.current.requestCapture({
        ...input,
        requestId,
      })
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

      activeCaptureRequestIdRef.current = safeResult.capture.requestId
      activeCaptureIdRef.current = safeResult.capture.captureId

      setSessionDraft((current) => ({
        ...current,
        ...(() => {
          const safePendingFastPreview = sanitizePendingFastPreviewForSession(
            current.manifest?.sessionId ?? current.sessionId,
            current.pendingFastPreview,
          )
          const safeReadiness = lockStablePostEndReadiness(
            current.captureReadiness,
            sanitizeCaptureReadinessForSession(
              current.manifest?.sessionId ?? current.sessionId,
              safeResult.readiness,
            ),
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
          const manifestWithTiming = mergeTimingIntoManifest(
            manifestWithLatestCapture,
            safeReadiness,
          )
          const manifestWithPostEnd = mergePostEndIntoManifest(
            manifestWithTiming,
            safeReadiness,
          )

          return {
            captureReadiness: safeReadiness,
            pendingFastPreview: prunePendingFastPreview(
              current.manifest?.sessionId ?? current.sessionId,
              safePendingFastPreview,
              safeReadiness.latestCapture ?? safeResult.capture,
            ),
            manifest:
              manifestWithPostEnd === null
                ? null
                : {
                    ...manifestWithPostEnd,
                    lifecycle: {
                      ...manifestWithPostEnd.lifecycle,
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

      if (activeCaptureRequestIdRef.current === requestId) {
        activeCaptureRequestIdRef.current = null
        activeCaptureIdRef.current = null
      }
      setSessionDraft((current) => ({
        ...current,
        pendingFastPreview: null,
      }))

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
      const deleteCapture = captureRuntimeServiceRef.current.deleteCapture

      if (deleteCapture === undefined) {
        throw {
          code: 'host-unavailable',
          message: '지금은 도움이 필요해요.',
        } satisfies HostErrorEnvelope
      }

      const result = await deleteCapture(input)
      const hasMatchingActiveSession = hasActiveSession(input.sessionId)
      const staleReadiness = hasMatchingActiveSession
        ? buildCurrentCaptureFallbackReadiness()
        : undefined

      if (
        result.sessionId !== input.sessionId ||
        result.captureId !== input.captureId ||
        result.manifest.sessionId !== input.sessionId ||
        result.readiness.sessionId !== input.sessionId
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

      deletedCaptureIdsRef.current.add(input.captureId)
      if (activeCaptureIdRef.current === input.captureId) {
        activeCaptureRequestIdRef.current = null
        activeCaptureIdRef.current = null
      }

      setSessionDraft((current) => {
        const safeReadiness = lockStablePostEndReadiness(
          current.captureReadiness,
          sanitizeCaptureReadinessForSession(
            current.manifest?.sessionId ?? current.sessionId,
            safeResult.readiness,
          ),
        )
        const safeManifest =
          sanitizeManifestForSession(
            current.manifest?.sessionId ?? current.sessionId ?? input.sessionId,
            safeResult.manifest,
          ) ?? safeResult.manifest
        const manifestWithTiming = mergeTimingIntoManifest(
          safeManifest,
          safeReadiness,
        )
        const manifestWithPostEnd = mergePostEndIntoManifest(
          manifestWithTiming,
          safeReadiness,
        )

        return {
          ...current,
          captureReadiness: safeReadiness,
          pendingFastPreview: prunePendingFastPreview(
            current.manifest?.sessionId ?? current.sessionId ?? input.sessionId,
            current.pendingFastPreview,
            safeReadiness.latestCapture ?? null,
          ),
          manifest: {
            ...(manifestWithPostEnd ?? safeManifest),
            lifecycle: {
              ...(manifestWithPostEnd ?? safeManifest).lifecycle,
              stage: deriveLifecycleStage(
                safeReadiness,
                (manifestWithPostEnd ?? safeManifest).lifecycle.stage,
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

  const syncFastPreview = useEffectEvent((pendingFastPreview: SessionDraft['pendingFastPreview']) => {
    if (
      pendingFastPreview === null ||
      sessionDraftRef.current.flowStep !== 'capture'
    ) {
      return
    }

    const sanitizedFastPreview = sanitizePendingFastPreviewForSession(
      sessionDraftRef.current.manifest?.sessionId ?? sessionDraftRef.current.sessionId,
      pendingFastPreview,
    )

    if (sanitizedFastPreview === null) {
      return
    }

    logCaptureDebug('apply-fast-preview-update', {
      sessionId: sanitizedFastPreview.sessionId,
      captureId: sanitizedFastPreview.captureId,
      requestId: sanitizedFastPreview.requestId,
      kind: sanitizedFastPreview.kind ?? 'none',
    })
    void logCaptureClientState({
      label: 'fast-preview-ready',
      sessionId: sanitizedFastPreview.sessionId,
      message: `captureId=${sanitizedFastPreview.captureId};requestId=${sanitizedFastPreview.requestId};kind=${sanitizedFastPreview.kind ?? 'none'}`,
    })

    setSessionDraft((current) => {
      const safeFastPreview = sanitizePendingFastPreviewForSession(
        current.manifest?.sessionId ?? current.sessionId,
        sanitizedFastPreview,
      )

      if (safeFastPreview === null) {
        return current
      }

      if (deletedCaptureIdsRef.current.has(safeFastPreview.captureId)) {
        return current
      }

      const currentLatestCapture = current.captureReadiness?.latestCapture ?? null
      const matchesCurrentLatestCapture = matchesPendingFastPreviewCapture(
        safeFastPreview,
        currentLatestCapture,
      )
      const matchesActiveCaptureRequest =
        activeCaptureRequestIdRef.current !== null &&
        safeFastPreview.requestId === activeCaptureRequestIdRef.current &&
        (activeCaptureIdRef.current === null ||
          safeFastPreview.captureId === activeCaptureIdRef.current)

      if (!matchesCurrentLatestCapture && !matchesActiveCaptureRequest) {
        return current
      }

      const manifestOwnedCapture =
        current.manifest?.captures.find(
          (capture) =>
            capture.captureId === safeFastPreview.captureId &&
            capture.requestId === safeFastPreview.requestId,
        ) ?? null
      const manifestOwnsDisplayablePreview = hasDisplayablePreviewAsset(
        current.manifest?.sessionId ?? current.sessionId,
        manifestOwnedCapture,
      )

      if (manifestOwnsDisplayablePreview) {
        return current.pendingFastPreview === null
          ? current
          : {
              ...current,
              pendingFastPreview: null,
            }
      }

      if (
        current.pendingFastPreview?.captureId === safeFastPreview.captureId &&
        current.pendingFastPreview.assetPath === safeFastPreview.assetPath &&
        current.pendingFastPreview.visibleAtMs === safeFastPreview.visibleAtMs
      ) {
        return current
      }

      return {
        ...current,
        pendingFastPreview: safeFastPreview,
      }
    })
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
    logCaptureDebug('apply-subscribed-readiness', {
      sessionId: sanitizedReadiness.sessionId,
      customerState: sanitizedReadiness.customerState,
      reasonCode: sanitizedReadiness.reasonCode,
      canCapture: sanitizedReadiness.canCapture,
    })

    captureReadinessRequestVersionRef.current += 1
    setIsLoadingCaptureReadiness(false)

    if (shouldInvalidateCaptureRequest(sanitizedReadiness)) {
      requestCaptureRequestVersionRef.current += 1
      isRequestingCaptureRef.current = false
      setIsRequestingCapture(false)
      setSessionDraft((current) => ({
        ...current,
        pendingFastPreview: null,
      }))
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
    const activePreset =
      sessionDraft.selectedPreset ?? sessionDraft.manifest?.activePreset ?? null

    if (
      sessionDraft.flowStep !== 'capture' ||
      sessionDraft.sessionId === null ||
      activePreset === null
    ) {
      clearCapturePreviewRuntimePrime()
      return
    }

    ensureCapturePreviewRuntimePrime({
      sessionId: sessionDraft.sessionId,
      preset: activePreset,
    })
  }, [
    sessionDraft.flowStep,
    sessionDraft.sessionId,
    sessionDraft.selectedPreset,
    sessionDraft.manifest?.activePreset,
  ])

  useEffect(() => {
    if (sessionDraft.flowStep !== 'capture' || sessionDraft.sessionId === null) {
      return
    }

    let isDisposed = false
    let stopListening: (() => void) | undefined
    let stopFastPreviewListening: (() => void) | undefined

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

    const subscribeToCaptureFastPreview =
      captureRuntimeServiceRef.current.subscribeToCaptureFastPreview

    if (subscribeToCaptureFastPreview !== undefined) {
      void subscribeToCaptureFastPreview({
        sessionId: sessionDraft.sessionId,
        onFastPreview(fastPreview) {
          if (!isDisposed) {
            syncFastPreview(fastPreview)
          }
        },
      })
        .then((unlisten) => {
          if (isDisposed) {
            unlisten()
            return
          }

          stopFastPreviewListening = unlisten
        })
        .catch(() => undefined)
    }

    return () => {
      isDisposed = true

      if (stopListening) {
        stopListening()
      }

      if (stopFastPreviewListening) {
        stopFastPreviewListening()
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
