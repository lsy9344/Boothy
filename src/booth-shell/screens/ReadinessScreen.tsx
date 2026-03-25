type ReadinessScreenProps = {
  boothAlias: string | null
  selectedPresetName: string | null
  actionLabel: string
  canCapture: boolean
  isBusy: boolean
  isChangePresetDisabled: boolean
  onPrimaryAction(): void
  onChangePreset(): void
}

export function ReadinessScreen({
  boothAlias,
  selectedPresetName,
  actionLabel,
  canCapture,
  isBusy,
  isChangePresetDisabled,
  onPrimaryAction,
  onChangePreset,
}: ReadinessScreenProps) {
  return (
    <>
      <article className="surface-card readiness-screen__summary">
        <div>
          <h2>현재 세션</h2>
          <p>{boothAlias ?? '세션 확인 중'}</p>
        </div>
        <div>
          <h2>현재 룩</h2>
          <p>{selectedPresetName ?? '선택 대기 중'}</p>
          <p>지금 바꾸면 다음 촬영부터만 새 룩이 적용돼요.</p>
        </div>
      </article>

      <article className="surface-card readiness-screen__action-card">
        <button
          type="button"
          className="latest-photo-rail__action latest-photo-rail__action--secondary"
          disabled={isChangePresetDisabled}
          onClick={onChangePreset}
        >
          다음 촬영 룩 바꾸기
        </button>
        <button
          type="button"
          className="session-start-form__submit readiness-screen__action"
          disabled={!canCapture || isBusy}
          onClick={onPrimaryAction}
        >
          {actionLabel}
        </button>
      </article>
    </>
  )
}
