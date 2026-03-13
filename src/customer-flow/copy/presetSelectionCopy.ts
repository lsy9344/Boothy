export const presetSelectionCopy = {
  eyebrow: 'Preset',
  title: '원하는 프리셋을 눌러 주세요.',
  supporting: '한 가지를 고른 뒤 확인하면 촬영 준비로 넘어가요.',
  confirmAction: '이 프리셋으로 계속',
  selectionRequired: '프리셋을 먼저 고르면 다음으로 넘어갈 수 있어요.',
  selectionRetryRequired: '프리셋을 적용하지 못했어요. 다시 선택해 주세요.',
  selectionRestartRequired: '세션을 다시 확인해 주세요. 문제가 계속되면 처음부터 다시 시작해 주세요.',
  sessionLabel: '세션 이름',
  states: {
    empty: {
      title: '프리셋을 준비하고 있어요.',
      supporting: '잠시만 기다려 주세요.',
    },
    loading: {
      title: '프리셋을 불러오고 있어요.',
      supporting: '잠시만 기다려 주세요.',
    },
    unavailable: {
      title: '프리셋을 준비하고 있어요.',
      supporting: '잠시만 기다려 주세요. 계속 안 되면 직원에게 알려 주세요.',
    },
  },
} as const
