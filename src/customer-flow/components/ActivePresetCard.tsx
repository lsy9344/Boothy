type ActivePresetCardProps = {
  label: string
  value: string
}

export function ActivePresetCard({ label, value }: ActivePresetCardProps) {
  return (
    <section aria-label={label} className="capture-signal-card">
      <p className="capture-signal-card__label">{label}</p>
      <p className="capture-signal-card__value">{value}</p>
    </section>
  )
}
