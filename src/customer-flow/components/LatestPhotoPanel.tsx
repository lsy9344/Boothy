import type { CaptureConfidenceView } from '../selectors/captureConfidenceView.js'

type LatestPhotoPanelProps = {
  latestPhoto: CaptureConfidenceView['latestPhoto']
}

export function LatestPhotoPanel({ latestPhoto }: LatestPhotoPanelProps) {
  return (
    <article aria-live="polite" className={`latest-photo-panel latest-photo-panel--${latestPhoto.kind}`}>
      <div className="latest-photo-panel__copy">
        <p className="latest-photo-panel__eyebrow">Latest Photo</p>
        <h2 className="latest-photo-panel__title">{latestPhoto.title}</h2>
      </div>

      <div className="latest-photo-panel__frame">
        {latestPhoto.assetUrl ? (
          <img alt={latestPhoto.alt ?? ''} className="latest-photo-panel__image" src={latestPhoto.assetUrl} />
        ) : (
          <div aria-label="최신 사진 대기 상태" className="latest-photo-panel__placeholder" role="img">
            <span>PHOTO</span>
          </div>
        )}
      </div>

      <p className="latest-photo-panel__supporting">{latestPhoto.supporting}</p>
    </article>
  )
}
