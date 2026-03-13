import { startTransition, useState } from 'react'

import { useBranchConfig } from '../../branch-config/useBranchConfig.js'
import {
  defaultPresetId,
  getPresetCatalogEntryById,
  type PresetId,
} from '../../shared-contracts/presets/presetCatalog.js'
import {
  createCaptureFlowState,
  presetChangeConfirmationMessage,
  type CaptureFlowState,
} from '../../session-domain/state/captureFlowState.js'
import { SessionFlowProvider, useSessionFlow } from '../../session-domain/state/SessionFlowProvider.js'
import { selectCaptureActionEnabled } from '../../session-domain/state/sessionReducer.js'
import {
  selectSessionTimeDisplay,
  type SessionTimeDisplay,
} from '../../timing-policy/selectors/sessionTimeDisplay.js'
import { preparationCopy } from '../copy/preparationCopy.js'
import { sessionStartErrorCopy } from '../copy/sessionStartErrorCopy.js'
import {
  createCaptureReadyView,
  selectCaptureConfidenceView,
} from '../selectors/captureConfidenceView.js'
import { presetSelectionCopy } from '../copy/presetSelectionCopy.js'
import { selectPostEndView } from '../selectors/postEndView.js'
import { CaptureScreen } from './CaptureScreen.js'
import { CustomerStartScreen } from './CustomerStartScreen.js'
import { PostEndScreen } from './PostEndScreen.js'
import { PreparationScreen } from './PreparationScreen.js'
import { PresetSelectionSurface } from './PresetSelectionSurface.js'
import { SessionStartHandoffScreen } from './SessionStartHandoffScreen.js'
import { resolveCaptureView } from './customerFlowView.js'

function resolveActivePresetId(
  pendingPresetId: string | null | undefined,
  activePresetId: string | null | undefined,
  snapshotPresetId: string | null | undefined,
): PresetId {
  const resolvedPendingPreset = pendingPresetId ? getPresetCatalogEntryById(pendingPresetId)?.id : undefined

  if (resolvedPendingPreset) {
    return resolvedPendingPreset
  }

  const resolvedActivePreset = activePresetId ? getPresetCatalogEntryById(activePresetId)?.id : undefined

  if (resolvedActivePreset) {
    return resolvedActivePreset
  }

  const resolvedSnapshotPreset = snapshotPresetId ? getPresetCatalogEntryById(snapshotPresetId)?.id : undefined

  if (resolvedSnapshotPreset) {
    return resolvedSnapshotPreset
  }

  return defaultPresetId
}

function resolveCandidatePresetId(selectedPresetId: string | null | undefined): PresetId | null {
  return getPresetCatalogEntryById(selectedPresetId ?? '')?.id ?? null
}

function resolveSessionTimeDisplay(state: ReturnType<typeof useSessionFlow>['state']): SessionTimeDisplay | null {
  if (state.sessionTiming) {
    return selectSessionTimeDisplay(state.sessionTiming.actualShootEndAt)
  }

  return null
}

type ActiveSessionSurfaceProps = Pick<
  ReturnType<typeof useSessionFlow>,
  | 'applyActivePresetChange'
  | 'isActivePresetChangePending'
  | 'presetCatalogState'
  | 'requestCapture'
  | 'state'
>

function ActiveSessionSurface({
  applyActivePresetChange,
  isActivePresetChangePending,
  presetCatalogState,
  requestCapture,
  state,
}: ActiveSessionSurfaceProps) {
  const [isPresetSelectorOpen, setIsPresetSelectorOpen] = useState(false)
  const [pendingPresetChangeMessage, setPendingPresetChangeMessage] = useState<string | null>(null)
  const sessionTimeDisplay = resolveSessionTimeDisplay(state)

  const captureState: CaptureFlowState = createCaptureFlowState({
    activePresetId: resolveActivePresetId(
      state.pendingActivePresetId,
      state.activePreset?.presetId,
      state.captureConfidence?.activePreset.presetId,
    ),
    isPresetSelectorOpen,
    pendingPresetChangeMessage,
    sessionEndTimeLabel: sessionTimeDisplay?.value ?? '계산 중',
  })

  const activePresetLabel =
    getPresetCatalogEntryById(captureState.activePresetId)?.name ?? state.activePreset?.displayName ?? '프리셋 준비 중'

  const handleSelectPreset = async (presetId: PresetId) => {
    if (captureState.activePresetId === presetId) {
      setIsPresetSelectorOpen(false)
      return
    }

    const didApplyPresetChange = await applyActivePresetChange(presetId)

    if (!didApplyPresetChange) {
      startTransition(() => {
        setPendingPresetChangeMessage(presetSelectionCopy.selectionRetryRequired)
      })
      return
    }

    startTransition(() => {
      setIsPresetSelectorOpen(false)
      setPendingPresetChangeMessage(presetChangeConfirmationMessage)
    })
  }

  const view = state.captureConfidence
    ? selectCaptureConfidenceView(state.captureConfidence, activePresetLabel, state.timingAlert)
    : createCaptureReadyView(activePresetLabel, state.timingAlert)
  const resolvedView = resolveCaptureView(view, sessionTimeDisplay)

  return (
    <CaptureScreen
      captureActionDisabled={
        !selectCaptureActionEnabled(state) ||
        state.captureRequestStatus === 'requesting' ||
        isActivePresetChangePending
      }
      captureState={captureState}
      onCapture={() => {
        void requestCapture()
      }}
      onClosePresetSelector={() => {
        setIsPresetSelectorOpen(false)
      }}
      onDismissPresetChangeFeedback={() => {
        setPendingPresetChangeMessage(null)
      }}
      onOpenPresetSelector={() => {
        setIsPresetSelectorOpen(true)
      }}
      onSelectPreset={handleSelectPreset}
      presetCatalogState={presetCatalogState}
      presetSelectionDisabled={isActivePresetChangePending}
      sessionName={state.activeSession!.sessionName}
      view={resolvedView}
    />
  )
}

export function CustomerFlowContent() {
  const { config } = useBranchConfig()
  const {
    applyActivePresetChange,
    confirmPresetSelection,
    continueFromPreparation,
    isActivePresetChangePending,
    presetCatalogState,
    requestCapture,
    selectPreset,
    startJourney,
    state,
    submitCheckIn,
    updateField,
  } = useSessionFlow()
  const sessionTimeDisplay = resolveSessionTimeDisplay(state)
  const sessionNameError = state.fieldErrors.sessionName ? sessionStartErrorCopy[state.fieldErrors.sessionName] : null
  const formError = state.formErrorCode ? sessionStartErrorCopy[state.formErrorCode] : undefined

  if (!state.activeSession && state.pendingSessionName) {
    return (
      <SessionStartHandoffScreen
        isContinuing={state.phase === 'provisioning'}
        onContinue={async () => {
          await submitCheckIn()
        }}
        sessionName={state.pendingSessionName}
      />
    )
  }

  if (!state.activeSession) {
    return (
      <CustomerStartScreen
        formErrorMessage={formError}
        onSessionNameChange={(sessionName) => {
          updateField('sessionName', sessionName)
        }}
        onStart={(sessionName) => {
          startJourney(sessionName)
        }}
        sessionName={state.fields.sessionName}
        validationMessage={sessionNameError}
      />
    )
  }

  if (state.phase === 'preset-selection' && state.activeSession) {
    return (
      <PresetSelectionSurface
        catalogState={presetCatalogState}
        isApplyingPreset={state.presetSelectionStatus === 'applying'}
        onConfirmPreset={confirmPresetSelection}
        onSelectPreset={selectPreset}
        selectionFeedback={state.presetSelectionFeedback}
        selectedPresetId={resolveCandidatePresetId(state.selectedPresetId)}
        sessionName={state.activeSession.sessionName}
        sessionTimeDisplay={sessionTimeDisplay}
      />
    )
  }

  if (state.activeSession && state.readiness && state.phase === 'preparing') {
    return (
      <PreparationScreen
        onStartCapture={continueFromPreparation}
        readiness={state.readiness}
        sessionName={state.activeSession.sessionName}
        sessionTimeDisplay={sessionTimeDisplay}
      />
    )
  }

  if (state.phase === 'capture-loading' && state.activeSession && state.readiness) {
    return (
      <PreparationScreen
        readiness={state.readiness}
        sessionName={state.activeSession.sessionName}
        sessionTimeDisplay={sessionTimeDisplay}
        showPrimaryAction={false}
        statusOverride={preparationCopy.captureLoading}
      />
    )
  }

  if (state.phase === 'capture-ready' && state.activeSession) {
    return (
      <ActiveSessionSurface
        applyActivePresetChange={applyActivePresetChange}
        isActivePresetChangePending={isActivePresetChangePending}
        key={state.activeSession.sessionId}
        presetCatalogState={presetCatalogState}
        requestCapture={requestCapture}
        state={state}
      />
    )
  }

  if (state.phase === 'post-end' && state.activeSession && state.postEndOutcome) {
    return <PostEndScreen view={selectPostEndView(state.postEndOutcome, config.branchPhoneNumber)} />
  }

  return <SessionStartHandoffScreen sessionName={state.activeSession.sessionName} />
}

export function CustomerFlowScreen() {
  return (
    <SessionFlowProvider>
      <CustomerFlowContent />
    </SessionFlowProvider>
  )
}
