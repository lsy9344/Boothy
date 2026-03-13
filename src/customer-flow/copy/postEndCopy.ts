import type { PostEndOutcomeKind } from '../../shared-contracts/dto/postEndOutcome.js'

export type PostEndCopy = {
  badge: string
  title: string
  supporting: string
  actionLabel: string
}

export const postEndCopy: Record<PostEndOutcomeKind, PostEndCopy> = {
  'export-waiting': {
    badge: 'Export Waiting',
    title: '잠시만 기다려 주세요.',
    supporting: '사진을 정리하고 있습니다. 준비가 끝나면 바로 안내해 드릴게요.',
    actionLabel: '안내를 기다려 주세요',
  },
  completed: {
    badge: 'Completed',
    title: '세션이 완료되었어요.',
    supporting: '직원 안내에 따라 다음 전달 단계를 확인해 주세요.',
    actionLabel: '직원 안내 확인',
  },
  handoff: {
    badge: 'Handoff',
    title: '세션 안내를 확인해 주세요.',
    supporting: '세션 이름을 확인한 뒤 직원에게 전달해 주세요.',
    actionLabel: '직원에게 전달',
  },
}
