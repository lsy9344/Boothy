import type { CaptureReadinessSnapshot } from '../../shared-contracts'

export type CustomerStatusCopy = {
  stateLabel: string
  headline: string
  detail: string
  actionLabel: string
  canCapture: boolean
  isPreviewWaiting: boolean
  helperText: string | null
  nextActionText: string | null
}

const primaryActionLabels: Record<CaptureReadinessSnapshot['primaryAction'], string> = {
  wait: '잠시 기다리기',
  capture: '사진 찍기',
  'choose-preset': '룩 고르기',
  'start-session': '처음으로',
  'call-support': '도움 요청',
}

export function selectCustomerStatusCopy(
  readiness: CaptureReadinessSnapshot,
): CustomerStatusCopy {
  const isPreviewWaiting = readiness.reasonCode === 'preview-waiting'

  return {
    stateLabel: readiness.customerState,
    headline: readiness.customerMessage,
    detail: readiness.supportMessage,
    actionLabel: primaryActionLabels[readiness.primaryAction],
    canCapture: readiness.canCapture,
    isPreviewWaiting,
    helperText: isPreviewWaiting
      ? '사진 레일이 아직 비어 있어도 현재 세션 기준으로는 정상이에요.'
      : null,
    nextActionText: isPreviewWaiting ? '지금은 잠시 기다리면 돼요.' : null,
  }
}
