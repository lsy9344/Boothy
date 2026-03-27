import type { PhoneRequiredPostEndRecord } from '../../shared-contracts'

type PhoneRequiredSupportCardProps = {
  guidance: PhoneRequiredPostEndRecord
}

export function PhoneRequiredSupportCard({
  guidance,
}: PhoneRequiredSupportCardProps) {
  return (
    <article className="surface-card phone-required-support-card">
      <p className="phone-required-support-card__badge">보호 안내</p>
      <div className="phone-required-support-card__section">
        <h2>지금 해야 할 일</h2>
        <p>{guidance.primaryActionLabel}</p>
      </div>
      {guidance.supportActionLabel !== undefined &&
      guidance.supportActionLabel !== null ? (
        <p className="phone-required-support-card__support">
          {guidance.supportActionLabel}
        </p>
      ) : null}
      <p className="phone-required-support-card__warning">
        {guidance.unsafeActionWarning}
      </p>
    </article>
  )
}
