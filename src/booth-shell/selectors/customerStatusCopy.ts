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
  const isWarning =
    readiness.reasonCode === 'warning' ||
    (readiness.timing?.phase === 'warning' &&
      readiness.canCapture &&
      readiness.primaryAction === 'capture')
  const isPreviewWaiting = readiness.reasonCode === 'preview-waiting'
  const isExportWaiting = readiness.reasonCode === 'export-waiting'
  const isPostEndFinalized =
    readiness.reasonCode === 'completed' || readiness.reasonCode === 'phone-required'
  const postEnd = resolvePostEndGuidance(readiness, manifestPostEnd)

  return {
    stateLabel: isWarning ? '곧 종료돼요' : readiness.customerState,
    headline: isWarning ? '종료가 얼마 남지 않았어요.' : readiness.customerMessage,
    detail: isWarning
      ? readiness.canCapture
        ? '남은 시간 안에는 계속 촬영할 수 있어요.'
        : '지금 상태를 마무리한 뒤 다음 안내를 확인해 주세요.'
      : readiness.supportMessage,
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
