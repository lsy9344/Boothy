type PresetChangeToastProps = {
  message: string
}

export function PresetChangeToast({ message }: PresetChangeToastProps) {
  return (
    <div aria-live="polite" className="preset-change-toast surface-frame" role="status">
      {message}
    </div>
  )
}
