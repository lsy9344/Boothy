import type { PostEndOutcome } from '../../shared-contracts/dto/postEndOutcome.js'

import { postEndCopy } from '../copy/postEndCopy.js'

export type PostEndView = {
  actionLabel: string
  badge: string
  guidanceMode: PostEndOutcome['guidanceMode']
  handoffTargetLabel: string | null
  phoneNumber: string | null
  sessionName: string | null
  supporting: string
  title: string
}

export function selectPostEndView(outcome: PostEndOutcome, branchPhoneNumber: string): PostEndView {
  const copy = postEndCopy[outcome.outcomeKind]

  return {
    actionLabel: copy.actionLabel,
    badge: copy.badge,
    guidanceMode: outcome.guidanceMode,
    handoffTargetLabel: outcome.handoffTargetLabel,
    phoneNumber: outcome.guidanceMode === 'wait-or-call' ? branchPhoneNumber || null : null,
    sessionName: outcome.showSessionName ? outcome.sessionName : null,
    supporting:
      outcome.guidanceMode === 'wait-or-call'
        ? '매장 안내가 필요하면 아래 연락처로 전화해 주세요.'
        : copy.supporting,
    title: copy.title,
  }
}
