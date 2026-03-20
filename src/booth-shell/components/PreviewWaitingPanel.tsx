type PreviewWaitingPanelProps = {
  helperText: string
  nextActionText: string
}

export function PreviewWaitingPanel({
  helperText,
  nextActionText,
}: PreviewWaitingPanelProps) {
  return (
    <article className="surface-card preview-waiting-panel">
      <p className="preview-waiting-panel__badge">저장 완료</p>
      <p className="preview-waiting-panel__helper">{helperText}</p>
      <p className="preview-waiting-panel__next-action">{nextActionText}</p>
    </article>
  )
}
