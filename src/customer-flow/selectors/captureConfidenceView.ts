import type { CaptureConfidenceSnapshot } from '../../shared-contracts/dto/captureConfidence.js'
import {
  initialSessionTimingAlertState,
  type SessionTimingAlertState,
} from '../../session-domain/state/sessionTimingAlertState.js'
import {
  selectSessionTimeDisplay,
  sessionTimeDisplayLabel,
} from '../../timing-policy/selectors/sessionTimeDisplay.js'
import { captureFlowCopy } from '../copy/captureFlowCopy.js'

type TimingAlertView =
  | {
      kind: 'none'
    }
  | {
      kind: 'warning' | 'ended'
      badge: string
      message: string
    }

export type CaptureConfidenceView = {
  endTime: ReturnType<typeof selectSessionTimeDisplay> & {
    alertBadge?: string
  }
  preset: {
    label: string
    value: string
  }
  latestPhoto:
    | {
        kind: 'empty'
        title: string
        supporting: string
        assetUrl: null
        alt: null
      }
    | {
        kind: 'updating' | 'ready'
        title: string
        supporting: string
        assetUrl: string
        alt: string
      }
  guidance: string
  timingAlert: TimingAlertView
}

function resolvePresetLabel(snapshot: CaptureConfidenceSnapshot, activePresetLabelOverride?: string): string {
  return activePresetLabelOverride ?? snapshot.activePreset.label
}

function selectTimingAlertView(timingAlert: SessionTimingAlertState): TimingAlertView {
  void timingAlert

  return {
    kind: 'none',
  }
}

export function createCaptureReadyView(
  activePresetLabel: string,
  timingAlertState: SessionTimingAlertState = initialSessionTimingAlertState,
): CaptureConfidenceView {
  const timingAlert = selectTimingAlertView(timingAlertState)

  return {
    endTime: {
      label: sessionTimeDisplayLabel,
      value: '계산 중',
      supporting: '저장된 세션 종료 시간을 불러오는 중입니다.',
    },
    preset: {
      label: captureFlowCopy.presetLabel,
      value: activePresetLabel,
    },
    latestPhoto: {
      kind: 'empty',
      ...captureFlowCopy.latestPhoto.empty,
      assetUrl: null,
      alt: null,
    },
    guidance: captureFlowCopy.guidance,
    timingAlert,
  }
}

export function selectCaptureConfidenceView(
  snapshot: CaptureConfidenceSnapshot,
  activePresetLabelOverride?: string,
  timingAlertState: SessionTimingAlertState = initialSessionTimingAlertState,
): CaptureConfidenceView {
  const timingAlert = selectTimingAlertView(timingAlertState)
  const endTime = selectSessionTimeDisplay(snapshot.shootEndsAt)
  const presetLabel = resolvePresetLabel(snapshot, activePresetLabelOverride)
  const guidance = captureFlowCopy.guidance

  if (snapshot.latestPhoto.kind === 'empty') {
    return {
      endTime,
      preset: {
        label: captureFlowCopy.presetLabel,
        value: presetLabel,
      },
      latestPhoto: {
        kind: 'empty',
        ...captureFlowCopy.latestPhoto.empty,
        assetUrl: null,
        alt: null,
      },
      guidance,
      timingAlert,
    }
  }

  if (snapshot.latestPhoto.kind === 'updating') {
    return {
      endTime,
      preset: {
        label: captureFlowCopy.presetLabel,
        value: presetLabel,
      },
      latestPhoto: {
        kind: 'updating',
        ...captureFlowCopy.latestPhoto.updating,
        assetUrl: snapshot.latestPhoto.preview?.assetUrl ?? '',
        alt: captureFlowCopy.latestPhoto.ready.alt,
      },
      guidance,
      timingAlert,
    }
  }

  return {
    endTime,
    preset: {
      label: captureFlowCopy.presetLabel,
      value: presetLabel,
    },
    latestPhoto: {
      kind: 'ready',
      ...captureFlowCopy.latestPhoto.ready,
      assetUrl: snapshot.latestPhoto.photo.assetUrl,
      alt: captureFlowCopy.latestPhoto.ready.alt,
    },
    guidance,
    timingAlert,
  }
}
