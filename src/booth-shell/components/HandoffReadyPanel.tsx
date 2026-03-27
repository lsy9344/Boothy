import type { CompletedPostEndRecord } from '../../shared-contracts'

type HandoffReadyPanelProps = {
  boothAlias: string | null
  guidance: CompletedPostEndRecord
}

export function HandoffReadyPanel({
  boothAlias,
  guidance,
}: HandoffReadyPanelProps) {
  const destinationLabel =
    guidance.approvedRecipientLabel ?? guidance.nextLocationLabel ?? null
  const destinationTitle =
    guidance.approvedRecipientLabel != null
      ? '승인된 수령 대상'
      : '다음 이동 위치'

  return (
    <article className="surface-card handoff-ready-panel">
      <p className="handoff-ready-panel__badge">인계 안내</p>
      {destinationLabel !== null ? (
        <div className="handoff-ready-panel__section">
          <h2>{destinationTitle}</h2>
          <p>{destinationLabel}</p>
        </div>
      ) : null}
      <div className="handoff-ready-panel__section">
        <h2>다음 행동</h2>
        <p>{guidance.primaryActionLabel}</p>
      </div>
      {guidance.showBoothAlias && boothAlias !== null ? (
        <div className="handoff-ready-panel__section">
          <h2>확인할 이름</h2>
          <p>{boothAlias}</p>
        </div>
      ) : null}
    </article>
  )
}
