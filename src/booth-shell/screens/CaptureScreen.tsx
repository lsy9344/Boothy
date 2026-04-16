import type { Dispatch, MutableRefObject, SetStateAction } from 'react'
import { useEffect, useRef, useState } from 'react'

import type { HostErrorEnvelope } from '../../shared-contracts'
import {
  buildLocalCaptureReadiness,
  getCaptureRuntimeMode,
  type CaptureRuntimeMode,
} from '../../capture-adapter/services/capture-runtime'
import { SurfaceLayout } from '../../shared-ui/layout/SurfaceLayout'
import {
  selectCurrentSessionPreviews,
  type CurrentSessionPreview,
} from '../../session-domain/selectors'
import { useSessionState } from '../../session-domain/state/use-session-state'
import { playTimingCue } from '../../timing-policy/audio'
import { LatestPhotoRail } from '../components/LatestPhotoRail'
import { PreviewWaitingPanel } from '../components/PreviewWaitingPanel'
import { selectCustomerStatusCopy } from '../selectors/customerStatusCopy'
import { shouldShowFocusRetryOverlay } from './focusRetryOverlay'
import { ReadinessScreen } from './ReadinessScreen'

const CUSTOMER_SAFE_CAPTURE_FALLBACK_ERROR =
  '현재 세션 상태를 다시 확인하고 있어요. 잠시 후 다시 시도해 주세요.'
const CUSTOMER_SAFE_DELETE_FALLBACK_ERROR =
  '사진을 정리하는 중에 다시 확인이 필요해요. 잠시 후 다시 시도해 주세요.'
const FOCUS_RETRY_OVERLAY_VISIBLE_MS = 2000
const FOCUS_RETRY_OVERLAY_FADE_MS = 600

function clearFocusRetryOverlayTimers(
  focusRetryHideTimerRef: MutableRefObject<number | null>,
) {
  if (focusRetryHideTimerRef.current !== null) {
    window.clearTimeout(focusRetryHideTimerRef.current)
    focusRetryHideTimerRef.current = null
  }
}

function showFocusRetryOverlay(
  focusRetryHideTimerRef: MutableRefObject<number | null>,
  focusRetryOverlayIdRef: MutableRefObject<number>,
  setFocusRetryOverlay: Dispatch<
    SetStateAction<{
      id: number
      headline: string
      detail: string
    } | null>
  >,
  input: {
    headline: string
    detail: string
  },
) {
  clearFocusRetryOverlayTimers(focusRetryHideTimerRef)
  focusRetryOverlayIdRef.current += 1
  setFocusRetryOverlay({
    id: focusRetryOverlayIdRef.current,
    headline: input.headline,
    detail: input.detail,
  })
}

function mergePendingFastPreview(
  previews: CurrentSessionPreview[],
  input: {
    activePresetId: string | null
    activePresetVersion: string | null
    pendingCaptureActivePresetId: string | null
    pendingCaptureActivePresetVersion: string | null
    pendingCapturePresetDisplayName: string | null
    pendingFastPreview:
      | {
          captureId: string
          requestId: string
          assetPath: string
        }
      | null
    presetDisplayName: string | null
  },
) {
  const pendingPreviewActivePresetId =
    input.pendingCaptureActivePresetId ?? input.activePresetId
  const pendingPreviewActivePresetVersion =
    input.pendingCaptureActivePresetVersion ?? input.activePresetVersion

  if (
    input.pendingFastPreview === null ||
    pendingPreviewActivePresetVersion === null ||
    previews.some(
      (preview) => preview.captureId === input.pendingFastPreview?.captureId,
    )
  ) {
    return previews
  }

  const pendingPreview: CurrentSessionPreview = {
    captureId: input.pendingFastPreview.captureId,
    requestId: input.pendingFastPreview.requestId,
    assetPath: input.pendingFastPreview.assetPath,
    activePresetId: pendingPreviewActivePresetId,
    activePresetVersion: pendingPreviewActivePresetVersion,
    presetDisplayName:
      input.pendingCapturePresetDisplayName ?? input.presetDisplayName,
    isCurrentActivePreset:
      pendingPreviewActivePresetId !== null &&
      pendingPreviewActivePresetId === input.activePresetId &&
      pendingPreviewActivePresetVersion === input.activePresetVersion,
    postEndState: 'activeSession',
    readyAtMs: null,
    isLatest: true,
  }

  return [
    pendingPreview,
    ...previews.map((preview) => ({
      ...preview,
      isLatest: false,
    })),
  ]
}

function buildCameraStatus(
  runtimeMode: CaptureRuntimeMode,
  input: {
    canCapture: boolean
    reasonCode: string
    supportMessage: string
  },
) {
  if (runtimeMode === 'browser') {
    return {
      label: '브라우저 미리보기',
      detail: '실제 카메라 연결 상태는 앱에서만 확인할 수 있어요.',
      tone: 'neutral' as const,
    }
  }

  if (input.canCapture) {
    return {
      label: '연결됨',
      detail: '카메라 연결이 확인되어 지금 촬영할 수 있어요.',
      tone: 'ready' as const,
    }
  }

  switch (input.reasonCode) {
    case 'camera-preparing':
    case 'helper-preparing':
      return {
        label: '연결 확인 중',
        detail: '카메라 연결 또는 초기화가 끝나면 자동으로 촬영 가능 상태로 바뀌어요.',
        tone: 'neutral' as const,
      }
    case 'capture-retry-required':
      return {
        label: '초점 다시 맞추기',
        detail: input.supportMessage,
        tone: 'neutral' as const,
      }
    case 'phone-required':
      return {
        label: '직원 확인 필요',
        detail: input.supportMessage,
        tone: 'blocked' as const,
      }
    default:
      return {
        label: '촬영 대기 중',
        detail: input.supportMessage,
        tone: 'neutral' as const,
      }
  }
}

export function CaptureScreen() {
  const {
    beginPresetSwitch,
    deleteCapture,
    isDeletingCapture,
    isLoadingPresetCatalog,
    isRequestingCapture,
    isSelectingPreset,
    requestCapture,
    sessionDraft,
  } = useSessionState()
  const [fallbackError, setFallbackError] = useState<string | null>(null)
  const [pendingDeleteCaptureId, setPendingDeleteCaptureId] = useState<string | null>(
    null,
  )
  const [focusRetryOverlay, setFocusRetryOverlay] = useState<{
    id: number
    headline: string
    detail: string
  } | null>(null)
  const playedTimingCueKeyRef = useRef<string | null>(null)
  const focusRetryHideTimerRef = useRef<number | null>(null)
  const focusRetryOverlayIdRef = useRef(0)
  const lastReadinessReasonCodeRef = useRef<string | null>(null)
  const focusRetryShownForRequestRef = useRef(false)
  const wasRequestingCaptureRef = useRef(false)

  const readiness =
    sessionDraft.captureReadiness ??
    buildLocalCaptureReadiness({
      sessionId: sessionDraft.sessionId,
      hasSession: sessionDraft.sessionId !== null,
      hasPreset: sessionDraft.selectedPreset !== null,
    })

  const copy = selectCustomerStatusCopy(readiness, sessionDraft.manifest?.postEnd ?? null)
  const cameraStatus = buildCameraStatus(getCaptureRuntimeMode(), {
    canCapture: copy.canCapture,
    reasonCode: readiness.reasonCode,
    supportMessage: copy.detail,
  })
  const inFlightCaptureCopy = isRequestingCapture
    ? {
        ...copy,
        stateLabel: '촬영 처리 중',
        headline: '방금 찍은 사진을 불러오는 중이에요.',
        detail:
          '카메라에서 같은 촬영의 원본 파일을 전송 중이에요. 잠시만 기다려 주세요.',
        actionLabel: '잠시 기다리기',
        canCapture: false,
        isPreviewWaiting: false,
        helperText: null,
        nextActionText: null,
      }
    : copy
  const inFlightCameraStatus = isRequestingCapture
    ? {
        label: '촬영 처리 중',
        detail: '원본 파일이 도착하면 바로 다음 표시 단계로 이어져요.',
        tone: 'neutral' as const,
      }
    : cameraStatus
  const timing = readiness.timing ?? sessionDraft.manifest?.timing ?? null

  const selectedPresetName =
    sessionDraft.manifest?.activePresetDisplayName ??
    sessionDraft.presetCatalog.find(
      (preset) =>
        preset.presetId === sessionDraft.selectedPreset?.presetId &&
        preset.publishedVersion === sessionDraft.selectedPreset?.publishedVersion,
    )?.displayName ??
    (sessionDraft.selectedPreset === null ? null : '현재 룩 확인 중')
  const pendingFastPreviewCapture =
    sessionDraft.pendingFastPreview === null
      ? null
      : sessionDraft.manifest?.captures.find(
          (capture) =>
            capture.captureId === sessionDraft.pendingFastPreview?.captureId &&
            capture.requestId === sessionDraft.pendingFastPreview?.requestId,
        ) ?? null
  const pendingFastPreviewPresetDisplayName =
    pendingFastPreviewCapture?.activePresetDisplayName ??
    (pendingFastPreviewCapture?.activePresetId === undefined ||
    pendingFastPreviewCapture?.activePresetId === null
      ? null
      : sessionDraft.presetCatalog.find(
          (preset) =>
            preset.presetId === pendingFastPreviewCapture.activePresetId &&
            preset.publishedVersion ===
              pendingFastPreviewCapture.activePresetVersion,
        )?.displayName) ??
    null
  const currentSessionPreviews = mergePendingFastPreview(
    selectCurrentSessionPreviews(sessionDraft.manifest, sessionDraft.presetCatalog),
    {
      activePresetId:
        sessionDraft.selectedPreset?.presetId ??
        sessionDraft.manifest?.activePresetId ??
        null,
      activePresetVersion:
        sessionDraft.selectedPreset?.publishedVersion ??
        sessionDraft.manifest?.activePreset?.publishedVersion ??
        null,
      pendingCaptureActivePresetId:
        pendingFastPreviewCapture?.activePresetId ?? null,
      pendingCaptureActivePresetVersion:
        pendingFastPreviewCapture?.activePresetVersion ?? null,
      pendingCapturePresetDisplayName: pendingFastPreviewPresetDisplayName,
      pendingFastPreview:
        sessionDraft.pendingFastPreview === null
          ? null
          : {
              captureId: sessionDraft.pendingFastPreview.captureId,
              requestId: sessionDraft.pendingFastPreview.requestId,
              assetPath: sessionDraft.pendingFastPreview.assetPath,
            },
      presetDisplayName: selectedPresetName,
    },
  )
  const activePendingDeleteCaptureId =
    pendingDeleteCaptureId !== null &&
    currentSessionPreviews.some(
      (preview) => preview.captureId === pendingDeleteCaptureId,
    )
      ? pendingDeleteCaptureId
      : null

  useEffect(() => {
    if (timing === null) {
      playedTimingCueKeyRef.current = null
      return
    }

    if (timing.phase !== 'warning' && timing.phase !== 'ended') {
      playedTimingCueKeyRef.current = `${timing.sessionId}:${timing.phase}`
      return
    }

    const cueKey = `${timing.sessionId}:${timing.phase}`

    if (playedTimingCueKeyRef.current === cueKey) {
      return
    }

    playedTimingCueKeyRef.current = cueKey
    void playTimingCue(timing.phase)
  }, [timing])

  useEffect(() => {
    const previousReasonCode = lastReadinessReasonCodeRef.current
    const wasRequestingCapture = wasRequestingCaptureRef.current
    const overlayDecision = shouldShowFocusRetryOverlay({
      previousReasonCode,
      nextReasonCode: readiness.reasonCode,
      wasRequestingCapture,
      isRequestingCapture,
      alreadyShownForCurrentRequest: focusRetryShownForRequestRef.current,
    })

    if (overlayDecision.resetShownForCurrentRequest) {
      focusRetryShownForRequestRef.current = false
    }

    if (overlayDecision.shouldShow) {
      showFocusRetryOverlay(
        focusRetryHideTimerRef,
        focusRetryOverlayIdRef,
        setFocusRetryOverlay,
        {
        headline: copy.headline,
        detail: copy.detail,
        },
      )
    }
    if (overlayDecision.markShownForCurrentRequest) {
      focusRetryShownForRequestRef.current = true
    }

    lastReadinessReasonCodeRef.current = readiness.reasonCode
    wasRequestingCaptureRef.current = isRequestingCapture
  }, [
    copy.detail,
    copy.headline,
    isRequestingCapture,
    readiness.reasonCode,
  ])

  useEffect(() => {
    if (focusRetryOverlay === null) {
      return
    }

    clearFocusRetryOverlayTimers(focusRetryHideTimerRef)
    focusRetryHideTimerRef.current = window.setTimeout(() => {
      setFocusRetryOverlay(null)
      focusRetryHideTimerRef.current = null
    }, FOCUS_RETRY_OVERLAY_VISIBLE_MS + FOCUS_RETRY_OVERLAY_FADE_MS)

    return () => {
      clearFocusRetryOverlayTimers(focusRetryHideTimerRef)
    }
  }, [focusRetryOverlay])

  useEffect(() => {
    return () => {
      clearFocusRetryOverlayTimers(focusRetryHideTimerRef)
    }
  }, [])

  async function handlePrimaryAction() {
    if (!copy.canCapture || sessionDraft.sessionId === null) {
      return
    }

    setFallbackError(null)
    focusRetryShownForRequestRef.current = false

    try {
      await requestCapture({
        sessionId: sessionDraft.sessionId,
      })
    } catch (error) {
      const hostError = error as HostErrorEnvelope

      if (hostError.readiness?.sessionId === sessionDraft.sessionId) {
        if (
          hostError.readiness.reasonCode === 'capture-retry-required' &&
          !focusRetryShownForRequestRef.current
        ) {
          showFocusRetryOverlay(
            focusRetryHideTimerRef,
            focusRetryOverlayIdRef,
            setFocusRetryOverlay,
            {
              headline: hostError.readiness.customerMessage,
              detail: hostError.readiness.supportMessage,
            },
          )
          focusRetryShownForRequestRef.current = true
        }
        return
      }

      setFallbackError(CUSTOMER_SAFE_CAPTURE_FALLBACK_ERROR)
    }
  }

  async function handleDeleteConfirm(captureId: string) {
    if (sessionDraft.sessionId === null) {
      return
    }

    setFallbackError(null)

    try {
      await deleteCapture({
        sessionId: sessionDraft.sessionId,
        captureId,
      })
      setPendingDeleteCaptureId(null)
    } catch (error) {
      const hostError = error as HostErrorEnvelope

      if (
        hostError.readiness !== undefined &&
        hostError.readiness?.sessionId !== sessionDraft.sessionId
      ) {
        setFallbackError(CUSTOMER_SAFE_DELETE_FALLBACK_ERROR)
        return
      }

      setFallbackError(hostError.message ?? CUSTOMER_SAFE_DELETE_FALLBACK_ERROR)
    }
  }

  return (
    <SurfaceLayout
      eyebrow={inFlightCaptureCopy.stateLabel}
      title={inFlightCaptureCopy.headline}
      description={inFlightCaptureCopy.detail}
    >
      {focusRetryOverlay !== null ? (
        <div
          key={focusRetryOverlay.id}
          role="alert"
          aria-live="assertive"
          className="focus-retry-overlay focus-retry-overlay--dismissing"
        >
          <div className="focus-retry-overlay__card">
            <p className="focus-retry-overlay__badge">초점 안내</p>
            <h2>{focusRetryOverlay.headline}</h2>
            <p>{focusRetryOverlay.detail}</p>
          </div>
        </div>
      ) : null}

      {inFlightCaptureCopy.isPreviewWaiting &&
      inFlightCaptureCopy.helperText !== null &&
      inFlightCaptureCopy.nextActionText !== null ? (
        <PreviewWaitingPanel
          helperText={inFlightCaptureCopy.helperText}
          nextActionText={inFlightCaptureCopy.nextActionText}
        />
      ) : null}

      <ReadinessScreen
        boothAlias={sessionDraft.boothAlias}
        selectedPresetName={selectedPresetName}
        postEndGuidance={copy.postEnd ?? null}
        timing={timing}
        stateLabel={inFlightCaptureCopy.stateLabel}
        cameraStatusLabel={inFlightCameraStatus.label}
        cameraStatusDetail={inFlightCameraStatus.detail}
        cameraStatusTone={inFlightCameraStatus.tone}
        actionLabel={inFlightCaptureCopy.actionLabel}
        canCapture={inFlightCaptureCopy.canCapture}
        isBusy={isRequestingCapture || isDeletingCapture}
        isExplicitPostEnd={
          inFlightCaptureCopy.isEndedBridge ||
          inFlightCaptureCopy.isExportWaiting ||
          inFlightCaptureCopy.isPostEndFinalized
        }
        isChangePresetDisabled={
          inFlightCaptureCopy.isEndedBridge ||
          inFlightCaptureCopy.isExportWaiting ||
          inFlightCaptureCopy.isPostEndFinalized ||
          sessionDraft.sessionId === null ||
          sessionDraft.selectedPreset === null ||
          isLoadingPresetCatalog ||
          isSelectingPreset ||
          isDeletingCapture ||
          isRequestingCapture
        }
        onPrimaryAction={handlePrimaryAction}
        onChangePreset={() => {
          setFallbackError(null)
          beginPresetSwitch()
        }}
      />

      <LatestPhotoRail
        previews={currentSessionPreviews}
        isPreviewWaiting={inFlightCaptureCopy.isPreviewWaiting}
        isExplicitPostEnd={
          inFlightCaptureCopy.isEndedBridge ||
          inFlightCaptureCopy.isExportWaiting ||
          inFlightCaptureCopy.isPostEndFinalized
        }
        deletingCaptureId={isDeletingCapture ? activePendingDeleteCaptureId : null}
        pendingDeleteCaptureId={activePendingDeleteCaptureId}
        onDeleteCancel={() => {
          setPendingDeleteCaptureId(null)
        }}
        onDeleteConfirm={handleDeleteConfirm}
        onDeleteIntent={(captureId) => {
          setFallbackError(null)
          setPendingDeleteCaptureId(captureId)
        }}
      />

      {fallbackError !== null ? (
        <p className="preset-select-screen__error">{fallbackError}</p>
      ) : null}
    </SurfaceLayout>
  )
}
