import { useModalFocusTrap } from '../../shared-ui/hooks/useModalFocusTrap.js'

type ReviewScreenProps = {
  deletePending: boolean
  onClose(): void
  onRequestDelete(captureId: string): void
  photo: {
    captureId: string
    label: string
    previewPath: string
  }
  sessionName: string
}

export function ReviewScreen({
  deletePending,
  onClose,
  onRequestDelete,
  photo,
  sessionName,
}: ReviewScreenProps) {
  const dialogRef = useModalFocusTrap(onClose)

  return (
    <div
      aria-label="사진 크게 보기"
      aria-modal="true"
      className="review-overlay"
      ref={dialogRef}
      role="dialog"
      tabIndex={-1}
    >
      <section className="surface-frame review-dialog review-dialog--wide">
        <header className="review-screen__header">
          <p className="customer-shell__eyebrow">Review</p>
          <p aria-label="세션 이름" className="customer-capture__session-name">
            {sessionName}
          </p>
          <button className="secondary-action-button" onClick={onClose} type="button">
            닫기
          </button>
        </header>

        <div className="review-screen__frame">
          <img alt={`${photo.label} 크게 보기`} className="review-screen__image" src={photo.previewPath} />
        </div>

        <div className="customer-shell__actions review-dialog__actions">
          <button
            className="secondary-action-button"
            disabled={deletePending}
            onClick={() => onRequestDelete(photo.captureId)}
            type="button"
          >
            사진 삭제
          </button>
        </div>
      </section>
    </div>
  )
}
