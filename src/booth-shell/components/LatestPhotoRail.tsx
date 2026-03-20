import { resolvePresetPreviewSrc } from './preset-preview-src'
import type { CurrentSessionPreview } from '../../session-domain/selectors'

type LatestPhotoRailProps = {
  previews: CurrentSessionPreview[]
  isPreviewWaiting: boolean
}

export function LatestPhotoRail({
  previews,
  isPreviewWaiting,
}: LatestPhotoRailProps) {
  return (
    <article className="surface-card latest-photo-rail">
      <div className="latest-photo-rail__header">
        <h2>현재 세션 사진</h2>
        <p>
          {isPreviewWaiting && previews.length === 0
            ? '지금은 아직 비어 있어도 괜찮아요. 방금 저장한 사진이 확인용 보기로 준비되면 여기에 나타나요.'
            : '현재 세션에서 준비된 확인용 사진만 보여줘요.'}
        </p>
      </div>

      {previews.length === 0 ? (
        <p className="latest-photo-rail__empty">
          {isPreviewWaiting
            ? '잠시만 기다리면 최신 사진이 여기에 추가돼요.'
            : '아직 준비된 확인용 사진이 없어요.'}
        </p>
      ) : (
        <div className="latest-photo-rail__grid">
          {previews.map((preview) => (
            <figure
              key={preview.captureId}
              className="latest-photo-rail__item"
            >
              <img
                src={resolvePresetPreviewSrc(preview.assetPath)}
                alt={`현재 세션 사진 ${preview.captureId}`}
              />
              <figcaption>{preview.activePresetVersion}</figcaption>
            </figure>
          ))}
        </div>
      )}
    </article>
  )
}
