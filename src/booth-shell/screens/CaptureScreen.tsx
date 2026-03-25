import { useState } from 'react'

import type { HostErrorEnvelope } from '../../shared-contracts'
import { buildLocalCaptureReadiness } from '../../capture-adapter/services/capture-runtime'
import { SurfaceLayout } from '../../shared-ui/layout/SurfaceLayout'
import { selectCurrentSessionPreviews } from '../../session-domain/selectors'
import { useSessionState } from '../../session-domain/state/use-session-state'
import { LatestPhotoRail } from '../components/LatestPhotoRail'
import { PreviewWaitingPanel } from '../components/PreviewWaitingPanel'
import { selectCustomerStatusCopy } from '../selectors/customerStatusCopy'
import { ReadinessScreen } from './ReadinessScreen'

const CUSTOMER_SAFE_CAPTURE_FALLBACK_ERROR =
  '현재 세션 상태를 다시 확인하고 있어요. 잠시 후 다시 시도해 주세요.'
const CUSTOMER_SAFE_DELETE_FALLBACK_ERROR =
  '사진을 정리하는 중에 다시 확인이 필요해요. 잠시 후 다시 시도해 주세요.'

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

  const readiness =
    sessionDraft.captureReadiness ??
    buildLocalCaptureReadiness({
      sessionId: sessionDraft.sessionId,
      hasSession: sessionDraft.sessionId !== null,
      hasPreset: sessionDraft.selectedPreset !== null,
    })

  const copy = selectCustomerStatusCopy(readiness)
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
        actionLabel={copy.actionLabel}
        canCapture={copy.canCapture}
        isBusy={
          isLoadingCaptureReadiness || isRequestingCapture || isDeletingCapture
        }
        isChangePresetDisabled={
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
