import { useModalFocusTrap } from '../../shared-ui/hooks/useModalFocusTrap.js'

type DeletePhotoDialogProps = {
  deletePending: boolean
  onCancel(): void
  onConfirm(): void
  photoLabel: string
}

export function DeletePhotoDialog({
  deletePending,
  onCancel,
  onConfirm,
  photoLabel,
}: DeletePhotoDialogProps) {
  const dialogRef = useModalFocusTrap(onCancel)

  return (
    <div
      aria-label="사진 삭제 확인"
      aria-modal="true"
      className="review-overlay"
      ref={dialogRef}
      role="dialog"
      tabIndex={-1}
    >
      <section className="surface-frame review-dialog">
        <p className="review-dialog__eyebrow">Delete Photo</p>
        <h2 className="review-dialog__title">선택한 사진을 삭제할까요?</h2>
        <p className="review-dialog__supporting">{photoLabel}은(는) 삭제 후 되돌릴 수 없어요.</p>

        <div className="customer-shell__actions review-dialog__actions">
          <button className="secondary-action-button" onClick={onCancel} type="button">
            취소
          </button>
          <button
            className="primary-action-button"
            disabled={deletePending}
            onClick={onConfirm}
            type="button"
          >
            삭제하기
          </button>
        </div>
      </section>
    </div>
  )
}
