import type {
  CaptureReadinessSnapshot,
  SessionPostEndRecord,
} from '../../shared-contracts'
import { resolvePostEndGuidance } from '../../completion-handoff/post-end'

export type CustomerStatusCopy = {
  stateLabel: string
  headline: string
  detail: string
  actionLabel: string
  canCapture: boolean
  isPreviewWaiting: boolean
  isExportWaiting: boolean
  isPostEndFinalized: boolean
  postEnd: CaptureReadinessSnapshot['postEnd']
  helperText: string | null
  nextActionText: string | null
}

const primaryActionLabels: Record<CaptureReadinessSnapshot['primaryAction'], string> = {
  wait: '잠시 기다리기',
  finish: '안내 확인',
  capture: '사진 찍기',
  'choose-preset': '룩 고르기',
  'start-session': '처음으로',
  'call-support': '도움 요청',
}

export function selectCustomerStatusCopy(
  readiness: CaptureReadinessSnapshot,
  manifestPostEnd: SessionPostEndRecord | null = null,
): CustomerStatusCopy {
  const isPreviewWaiting = readiness.reasonCode === 'preview-waiting'
  const isExportWaiting = readiness.reasonCode === 'export-waiting'
  const isPostEndFinalized =
    readiness.reasonCode === 'completed' || readiness.reasonCode === 'phone-required'
  const postEnd = resolvePostEndGuidance(readiness, manifestPostEnd)

  return {
    stateLabel: readiness.customerState,
    headline: readiness.customerMessage,
    detail: readiness.supportMessage,
    actionLabel:
      postEnd?.state === 'completed'
        ? primaryActionLabels.finish
        : postEnd?.state === 'phone-required'
          ? primaryActionLabels[readiness.primaryAction]
        : primaryActionLabels[readiness.primaryAction],
    canCapture: readiness.canCapture,
    isPreviewWaiting,
    isExportWaiting,
    isPostEndFinalized,
    postEnd,
    helperText: isPreviewWaiting
      ? '사진 레일이 아직 비어 있어도 현재 세션 기준으로는 정상이에요.'
      : null,
    nextActionText: isPreviewWaiting ? '지금은 잠시 기다리면 돼요.' : null,
  }
}
