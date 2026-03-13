export const preparationScreenCopy = {
  preparing: {
    badge: '준비 중',
    title: '촬영 준비 중입니다. 잠시만 기다려 주세요.',
    supporting: '세션을 준비하고 있습니다.',
  },
  waiting: {
    badge: '잠시 대기',
    title: '아직 촬영할 수 없습니다. 잠시만 기다려 주세요.',
    supporting: '카메라 준비를 다시 확인하고 있습니다.',
  },
  ready: {
    badge: '촬영 가능',
    title: '카메라가 연결되어 촬영을 시작할 수 있습니다.',
    supporting: '준비가 끝났습니다. 화면의 안내에 따라 촬영을 시작해 주세요.',
  },
  phoneRequired: {
    badge: '전화 필요',
    title: '카메라 연결이 확인되지 않습니다. 전화해 주세요.',
    supporting: '안내된 매장 연락처로 전화해 주시면 바로 도와드리겠습니다.',
  },
  actionHint: '준비가 끝나면 촬영을 시작할 수 있습니다.',
  phoneLabel: '매장 연락처',
} as const;
