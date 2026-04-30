import { useEffect, useEffectEvent, useRef, useState } from 'react'

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
import { ReadinessScreen } from './ReadinessScreen'

const CUSTOMER_SAFE_CAPTURE_FALLBACK_ERROR =
  '현재 세션 상태를 다시 확인하고 있어요. 잠시 후 다시 시도해 주세요.'
const CUSTOMER_SAFE_DELETE_FALLBACK_ERROR =
  '사진을 정리하는 중에 다시 확인이 필요해요. 잠시 후 다시 시도해 주세요.'
const CUSTOMER_SAFE_EXPORT_FALLBACK_ERROR =
  '내보내기 준비 중에 다시 확인이 필요해요. 잠시 후 다시 시도해 주세요.'

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
          kind?: string | null
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
    previewKind: input.pendingFastPreview.kind ?? null,
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
        label: '다시 확인 중',
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
    exportCaptures,
    isDeletingCapture,
    isExportingCaptures,
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
  const playedTimingCueKeyRef = useRef<string | null>(null)

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
              kind: sessionDraft.pendingFastPreview.kind ?? null,
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
  const hasExportableCaptures =
    sessionDraft.manifest?.captures.some(
      (capture) =>
        capture.sessionId === sessionDraft.manifest?.sessionId &&
        capture.renderStatus === 'previewReady' &&
        capture.preview.readyAtMs !== null,
    ) ?? false

  const playCue = useEffectEvent((phase: 'warning' | 'ended') => {
    void playTimingCue(phase)
  })

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
    playCue(timing.phase)
  }, [timing])

  async function handlePrimaryAction() {
    if (!copy.canCapture || sessionDraft.sessionId === null) {
      return
    }

    setFallbackError(null)

    try {
      await requestCapture({
        sessionId: sessionDraft.sessionId,
      })
    } catch (error) {
      const hostError = error as HostErrorEnvelope

      if (hostError.readiness?.sessionId === sessionDraft.sessionId) {
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

  async function handleExportCaptures() {
    if (
      sessionDraft.sessionId === null ||
      inFlightCaptureCopy.isExportWaiting ||
      inFlightCaptureCopy.isPostEndFinalized ||
      !hasExportableCaptures
    ) {
      return
    }

    setFallbackError(null)

    try {
      await exportCaptures({
        sessionId: sessionDraft.sessionId,
      })
    } catch (error) {
      const hostError = error as HostErrorEnvelope

      if (hostError.readiness?.sessionId === sessionDraft.sessionId) {
        return
      }

      setFallbackError(hostError.message ?? CUSTOMER_SAFE_EXPORT_FALLBACK_ERROR)
    }
  }

  return (
    <SurfaceLayout
      eyebrow={inFlightCaptureCopy.stateLabel}
      title={inFlightCaptureCopy.headline}
      description={inFlightCaptureCopy.detail}
    >
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
        exportLabel={isExportingCaptures ? '내보내는 중' : '내보내기'}
        canExport={hasExportableCaptures}
        isExportDisabled={
          !hasExportableCaptures ||
          isExportingCaptures ||
          isRequestingCapture ||
          isDeletingCapture
        }
        isExplicitPostEnd={
          inFlightCaptureCopy.isExportWaiting || inFlightCaptureCopy.isPostEndFinalized
        }
        isChangePresetDisabled={
          inFlightCaptureCopy.isExportWaiting ||
          inFlightCaptureCopy.isPostEndFinalized ||
          sessionDraft.sessionId === null ||
          sessionDraft.selectedPreset === null ||
          isLoadingPresetCatalog ||
          isSelectingPreset ||
          isDeletingCapture ||
          isExportingCaptures ||
          isRequestingCapture
        }
        onPrimaryAction={handlePrimaryAction}
        onExport={handleExportCaptures}
        onChangePreset={() => {
          setFallbackError(null)
          beginPresetSwitch()
        }}
      />

      <LatestPhotoRail
        previews={currentSessionPreviews}
        isPreviewWaiting={inFlightCaptureCopy.isPreviewWaiting}
        isExplicitPostEnd={
          inFlightCaptureCopy.isExportWaiting || inFlightCaptureCopy.isPostEndFinalized
        }
        isPhotoActionDisabled={isExportingCaptures}
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
