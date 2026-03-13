type PrimaryActionButtonProps = {
  label: string
  disabled?: boolean
  describedBy?: string
  onClick?: () => void
  type?: 'button' | 'submit'
}

export function PrimaryActionButton({
  describedBy,
  disabled = false,
  label,
  onClick,
  type = 'button',
}: PrimaryActionButtonProps) {
  return (
    <button
      aria-describedby={describedBy}
      className="primary-action-button"
      disabled={disabled}
      onClick={onClick}
      type={type}
    >
      {label}
    </button>
  )
}
