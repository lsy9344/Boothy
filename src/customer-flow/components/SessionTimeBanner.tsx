type SessionTimeBannerProps = {
  alertBadge?: string
  label: string
  value: string
  supporting: string
}

export function SessionTimeBanner({ alertBadge, label, value, supporting }: SessionTimeBannerProps) {
  return (
    <section aria-label={label} className="capture-time-banner">
      <div className="capture-time-banner__header">
        <p className="capture-signal-card__label">{label}</p>
        {alertBadge ? (
          <p aria-live="polite" className="capture-time-banner__badge">
            {alertBadge}
          </p>
        ) : null}
      </div>
      <p className="capture-time-banner__value">{value}</p>
      <p className="capture-signal-card__supporting">{supporting}</p>
    </section>
  )
}
