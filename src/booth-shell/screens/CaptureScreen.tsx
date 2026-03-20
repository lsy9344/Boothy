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

export function CaptureScreen() {
  const {
    isLoadingCaptureReadiness,
    isRequestingCapture,
    requestCapture,
    sessionDraft,
  } = useSessionState()
  const [fallbackError, setFallbackError] = useState<string | null>(null)

  const readiness =
    sessionDraft.captureReadiness ??
    buildLocalCaptureReadiness({
      sessionId: sessionDraft.sessionId,
      hasSession: sessionDraft.sessionId !== null,
      hasPreset: sessionDraft.selectedPreset !== null,
    })

  const copy = selectCustomerStatusCopy(readiness)
  const currentSessionPreviews = selectCurrentSessionPreviews(sessionDraft.manifest)

  const selectedPresetName =
    sessionDraft.presetCatalog.find(
      (preset) =>
        preset.presetId === sessionDraft.selectedPreset?.presetId &&
        preset.publishedVersion === sessionDraft.selectedPreset?.publishedVersion,
    )?.displayName ??
    (sessionDraft.selectedPreset === null ? null : '선택한 룩 확인 중')

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

      if (hostError.readiness) {
        return
      }

      setFallbackError(hostError.message)
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
        isBusy={isLoadingCaptureReadiness || isRequestingCapture}
        onPrimaryAction={handlePrimaryAction}
      />

      <LatestPhotoRail
        previews={currentSessionPreviews}
        isPreviewWaiting={copy.isPreviewWaiting}
      />

      {fallbackError !== null ? (
        <p className="preset-select-screen__error">{fallbackError}</p>
      ) : null}
    </SurfaceLayout>
  )
}
