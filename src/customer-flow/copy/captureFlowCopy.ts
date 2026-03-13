export const captureFlowCopy = {
  presetLabel: '현재 프리셋',
  guidance: '리모컨으로 촬영을 계속해 주세요.',
  timingAlert: {
    warning: {
      badge: '5분 남음',
      message: '종료까지 5분 남았어요. 마무리 촬영을 진행해 주세요.',
    },
    ended: {
      badge: '촬영 종료',
      message: '촬영 시간이 끝났어요. 더 이상 촬영할 수 없어요.',
    },
  },
  latestPhoto: {
    empty: {
      title: '첫 사진을 기다리고 있어요.',
      supporting: '촬영이 저장되면 바로 여기에 보여드릴게요.',
    },
    updating: {
      title: '방금 찍은 사진을 불러오고 있어요.',
      supporting: '촬영 흐름은 그대로 유지됩니다.',
    },
    ready: {
      title: '방금 저장된 사진이에요.',
      supporting: '현재 세션에서 가장 최근에 저장된 사진입니다.',
      alt: '현재 세션의 최신 촬영 사진 미리보기',
    },
  },
} as const
