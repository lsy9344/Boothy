type ReadinessScreenProps = {
  boothAlias: string | null
  selectedPresetName: string | null
  actionLabel: string
  canCapture: boolean
  isBusy: boolean
  onPrimaryAction(): void
}

export function ReadinessScreen({
  boothAlias,
  selectedPresetName,
  actionLabel,
  canCapture,
  isBusy,
  onPrimaryAction,
}: ReadinessScreenProps) {
  return (
    <>
      <article className="surface-card readiness-screen__summary">
        <div>
          <h2>현재 세션</h2>
          <p>{boothAlias ?? '세션 확인 중'}</p>
        </div>
        <div>
          <h2>선택한 룩</h2>
          <p>{selectedPresetName ?? '선택 대기 중'}</p>
        </div>
      </article>

      <article className="surface-card readiness-screen__action-card">
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
