import type { PresetSelectionMode } from '../../session-domain/state/session-draft'

const initialSelectionCopy = {
  eyebrow: 'Booth',
  title: '원하는 룩을 골라 주세요',
  description: '대표 이미지를 보고 마음에 드는 하나를 고르면 다음 촬영부터 바로 적용돼요.',
  sessionLabel: '지금 세션',
  sessionDescription: '이 세션에 적용할 프리셋을 하나만 고를 수 있어요.',
  loadingTitle: '프리셋을 준비하고 있어요',
  loadingDescription: '화면에 보여 줄 옵션을 확인하고 있어요.',
  errorTitle: '프리셋을 다시 확인할게요',
  emptyTitle: '지금은 고를 수 있는 룩을 준비 중이에요',
  emptyDescription: '잠시 후 다시 확인해 주세요.',
  guidanceTitle: '다음 단계',
  guidanceDescription: '대표 이미지를 보고 원하는 분위기를 골라 주세요.',
  selectedDescription: '선택한 룩이 다음 촬영부터 적용돼요.',
  saveLabel: '이 룩으로 진행할게요',
  selectedLabel: '선택됨',
  retryLabel: '다시 불러올게요',
  loadErrorDescription: '지금은 프리셋을 불러올 수 없어요. 잠시 후 다시 시도해 주세요.',
  saveErrorDescription: '선택을 저장하지 못했어요. 다시 한 번 눌러 주세요.',
  cancelLabel: null,
  cancelDescription: null,
  unavailableDescription: '지금 고른 룩은 사용할 수 없어요. 다른 룩을 골라 주세요.',
} as const

const inSessionSwitchCopy = {
  eyebrow: 'Booth',
  title: '다음 촬영 룩을 다시 골라 주세요',
  description: '이미 찍은 사진은 그대로 두고, 다음 촬영부터만 새 룩으로 이어져요.',
  sessionLabel: '현재 세션',
  sessionDescription: '지금 세션은 유지한 채 다음 촬영에 쓸 룩만 바꿀 수 있어요.',
  loadingTitle: '바꿀 수 있는 룩을 확인하고 있어요',
  loadingDescription: '현재 세션에서 안전하게 쓸 수 있는 옵션만 다시 보여드릴게요.',
  errorTitle: '룩을 다시 확인할게요',
  emptyTitle: '지금은 바꿀 수 있는 룩을 준비 중이에요',
  emptyDescription: '현재 룩은 그대로 유지돼요. 잠시 후 다시 확인해 주세요.',
  guidanceTitle: '안내',
  guidanceDescription: '새로 고른 룩은 다음 촬영부터 적용되고, 이전 사진은 그대로 유지돼요.',
  selectedDescription: '현재 룩이에요. 다른 룩을 고르면 다음 촬영부터 바뀌어요.',
  saveLabel: '다음 촬영에 적용할게요',
  selectedLabel: '현재 룩',
  retryLabel: '다시 불러올게요',
  loadErrorDescription: '지금은 룩을 다시 불러올 수 없어요. 잠시 후 다시 시도해 주세요.',
  saveErrorDescription: '지금은 룩을 바꾸지 못했어요. 현재 룩은 그대로 유지돼요.',
  cancelLabel: '현재 룩으로 계속 촬영할게요',
  cancelDescription: '바꾸지 않아도 이미 찍은 사진과 현재 세션은 그대로예요.',
  unavailableDescription:
    '지금 고른 룩은 사용할 수 없어요. 현재 룩은 그대로 두고 다른 승인된 룩을 골라 주세요.',
} as const

export function getPresetSelectCopy(mode: PresetSelectionMode) {
  return mode === 'in-session-switch' ? inSessionSwitchCopy : initialSelectionCopy
}

export const presetSelectCopy = initialSelectionCopy
