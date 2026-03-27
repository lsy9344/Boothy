import type {
  CaptureReadinessSnapshot,
  SessionCaptureRecord,
  SessionPostEndRecord,
} from '../shared-contracts'

const POST_END_WAITING_REASON = 'export-waiting'
const POST_END_COMPLETED_REASON = 'completed'
const POST_END_PHONE_REQUIRED_REASON = 'phone-required'

export function isHostOwnedPostEndReason(
  reasonCode: CaptureReadinessSnapshot['reasonCode'],
) {
  return (
    reasonCode === POST_END_WAITING_REASON ||
    reasonCode === POST_END_COMPLETED_REASON ||
    reasonCode === POST_END_PHONE_REQUIRED_REASON
  )
}

export function isFinalizedPostEndReason(
  reasonCode: CaptureReadinessSnapshot['reasonCode'],
) {
  return (
    reasonCode === POST_END_COMPLETED_REASON ||
    reasonCode === POST_END_PHONE_REQUIRED_REASON
  )
}

export function isFinalizedCapturePostEndState(
  postEndState: SessionCaptureRecord['postEndState'],
) {
  return (
    postEndState === 'completed' ||
    postEndState === 'localDeliverableReady' ||
    postEndState === 'handoffReady'
  )
}

export function resolvePostEndGuidance(
  readiness: Pick<CaptureReadinessSnapshot, 'postEnd'>,
  manifestPostEnd: SessionPostEndRecord | null | undefined,
) {
  return readiness.postEnd ?? manifestPostEnd ?? null
}

export function getPostEndStrength(
  reasonCode: CaptureReadinessSnapshot['reasonCode'],
) {
  switch (reasonCode) {
    case POST_END_COMPLETED_REASON:
    case POST_END_PHONE_REQUIRED_REASON:
      return 2
    case POST_END_WAITING_REASON:
      return 1
    default:
      return 0
  }
}
