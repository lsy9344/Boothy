import { useEffect, useEffectEvent, useRef, useState } from 'react'

import type { HostErrorEnvelope } from '../../shared-contracts'
import {
  buildLocalCaptureReadiness,
  getCaptureRuntimeMode,
  type CaptureRuntimeMode,
} from '../../capture-adapter/services/capture-runtime'
import { SurfaceLayout } from '../../shared-ui/layout/SurfaceLayout'
import { selectCurrentSessionPreviews } from '../../session-domain/selectors'
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
    isLoadingCaptureReadiness,
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
  const timing = readiness.timing ?? sessionDraft.manifest?.timing ?? null
  const currentSessionPreviews = selectCurrentSessionPreviews(
    sessionDraft.manifest,
    sessionDraft.presetCatalog,
  )
  const activePendingDeleteCaptureId =
    pendingDeleteCaptureId !== null &&
    currentSessionPreviews.some(
      (preview) => preview.captureId === pendingDeleteCaptureId,
    )
      ? pendingDeleteCaptureId
      : null

  const selectedPresetName =
    sessionDraft.manifest?.activePresetDisplayName ??
    sessionDraft.presetCatalog.find(
      (preset) =>
        preset.presetId === sessionDraft.selectedPreset?.presetId &&
        preset.publishedVersion === sessionDraft.selectedPreset?.publishedVersion,
    )?.displayName ??
    (sessionDraft.selectedPreset === null ? null : '현재 룩 확인 중')

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

  return (
    <SurfaceLayout
      eyebrow={copy.stateLabel}
      title={copy.headline}
      description={copy.detail}
    >
      {copy.isPreviewWaiting && copy.helperText !== null && copy.nextActionText !== null ? (
        <PreviewWaitingPanel
          helperText={copy.helperText}
          nextActionText={copy.nextActionText}
        />
      ) : null}

      <ReadinessScreen
        boothAlias={sessionDraft.boothAlias}
        selectedPresetName={selectedPresetName}
        postEndGuidance={copy.postEnd ?? null}
        timing={timing}
        stateLabel={copy.stateLabel}
        cameraStatusLabel={cameraStatus.label}
        cameraStatusDetail={cameraStatus.detail}
        cameraStatusTone={cameraStatus.tone}
        actionLabel={copy.actionLabel}
        canCapture={copy.canCapture}
        isBusy={
          isLoadingCaptureReadiness || isRequestingCapture || isDeletingCapture
        }
        isExplicitPostEnd={copy.isExportWaiting || copy.isPostEndFinalized}
        isChangePresetDisabled={
          copy.isExportWaiting ||
          copy.isPostEndFinalized ||
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
        isPreviewWaiting={copy.isPreviewWaiting}
        isExplicitPostEnd={copy.isExportWaiting || copy.isPostEndFinalized}
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
