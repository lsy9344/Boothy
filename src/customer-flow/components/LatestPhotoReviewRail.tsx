type LatestPhotoReviewRailProps = {
  items: Array<{
    captureId: string
    label: string
    thumbnailPath: string
    isLatest: boolean
  }>
  selectedCaptureId: string | null
  onSelectCapture(captureId: string): void
}

export function LatestPhotoReviewRail({
  items,
  onSelectCapture,
  selectedCaptureId,
}: LatestPhotoReviewRailProps) {
  return (
    <section aria-label="세션 사진 썸네일" className="review-rail">
      <div className="review-rail__header">
        <p className="review-rail__eyebrow">Session Review</p>
        <p className="review-rail__supporting">현재 세션에서 촬영된 사진만 보여드립니다.</p>
      </div>

      <div className="review-rail__list" role="list">
        {items.map((item) => {
          const isSelected = item.captureId === selectedCaptureId

          return (
            <button
              aria-label={`${item.label} 선택`}
              aria-pressed={isSelected}
              className={`review-rail__item${isSelected ? ' review-rail__item--selected' : ''}`}
              key={item.captureId}
              onClick={() => onSelectCapture(item.captureId)}
              type="button"
            >
              <img alt="" className="review-rail__thumbnail" src={item.thumbnailPath} />
              <span className="review-rail__label">{item.label}</span>
              {item.isLatest ? <span className="review-rail__badge">최신 사진</span> : null}
            </button>
          )
        })}
      </div>
    </section>
  )
}
