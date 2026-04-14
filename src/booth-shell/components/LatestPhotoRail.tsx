import type { KeyboardEvent } from 'react'
import { isFinalizedCapturePostEndState } from '../../completion-handoff/post-end'
import type { CurrentSessionPreview } from '../../session-domain/selectors'
import { SessionPreviewImage } from './SessionPreviewImage'

type LatestPhotoRailProps = {
  previews: CurrentSessionPreview[]
  isPreviewWaiting: boolean
  isExplicitPostEnd: boolean
  deletingCaptureId: string | null
  pendingDeleteCaptureId: string | null
  onDeleteCancel(): void
  onDeleteConfirm(captureId: string): void
  onDeleteIntent(captureId: string): void
}

const HORIZONTAL_SCROLL_STEP_PX = 240

function buildPreviewAltText(
  preview: CurrentSessionPreview,
  position: number,
) {
  const presetLabel =
    preview.presetDisplayName ??
    (preview.isCurrentActivePreset ? '현재 룩' : '이전 룩')

  return preview.isLatest
    ? `현재 세션 최신 사진, ${position}번째, ${presetLabel} 룩`
    : `현재 세션 사진, ${position}번째, ${presetLabel} 룩`
}

export function LatestPhotoRail({
  previews,
  isPreviewWaiting,
  isExplicitPostEnd,
  deletingCaptureId,
  pendingDeleteCaptureId,
  onDeleteCancel,
  onDeleteConfirm,
  onDeleteIntent,
}: LatestPhotoRailProps) {
  function isRenderPending(preview: CurrentSessionPreview) {
    return preview.readyAtMs === null
  }

  function isPostEndLocked(preview: CurrentSessionPreview) {
    return isExplicitPostEnd || preview.postEndState !== 'activeSession'
  }

  function buildPostEndHint(preview: CurrentSessionPreview) {
    return preview.postEndState === 'postEndPending' || isExplicitPostEnd
      ? '지금은 결과 안내가 우선이라 여기서 사진을 정리할 수 없어요.'
      : isFinalizedCapturePostEndState(preview.postEndState)
        ? '마무리된 사진은 여기서 정리할 수 없어요.'
        : '지금은 사진 정리를 잠시 멈춰 주세요.'
  }

  function buildRenderPendingHint() {
    return '지금 보이는 첫 사진은 임시 미리보기예요. 현재 룩을 적용한 최종 사진으로 곧 바뀌어요.'
  }

  function handleRailKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key === 'ArrowRight') {
      event.preventDefault()
      event.currentTarget.scrollBy({
        left: HORIZONTAL_SCROLL_STEP_PX,
        behavior: 'smooth',
      })
      return
    }

    if (event.key === 'ArrowLeft') {
      event.preventDefault()
      event.currentTarget.scrollBy({
        left: -HORIZONTAL_SCROLL_STEP_PX,
        behavior: 'smooth',
      })
    }
  }

  return (
    <article className="surface-card latest-photo-rail">
      <div className="latest-photo-rail__header">
        <h2>현재 세션 사진</h2>
        <p>
          {isPreviewWaiting && previews.length === 0
            ? '지금은 아직 비어 있어도 괜찮아요. 방금 저장한 사진이 확인용 보기로 준비되면 여기에 나타나요.'
            : isPreviewWaiting
              ? '방금 찍은 사진을 먼저 보여 주고, 현재 룩 반영이 끝나면 자연스럽게 바뀌어요.'
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
        <div
          className="latest-photo-rail__scroller"
          role="list"
          aria-label="현재 세션 사진 레일"
          tabIndex={0}
          onKeyDown={handleRailKeyDown}
        >
          {previews.map((preview, index) => (
            <figure
              key={preview.captureId}
              className={`latest-photo-rail__item${
                preview.isLatest ? ' latest-photo-rail__item--latest' : ''
              }`}
              role="listitem"
            >
              {preview.isLatest ? (
                <span className="latest-photo-rail__badge">최신 사진</span>
              ) : null}
              {isRenderPending(preview) ? (
                <span className="latest-photo-rail__badge">룩 적용 중</span>
              ) : null}
              <SessionPreviewImage
                key={`${preview.captureId}:${preview.assetPath}:${preview.readyAtMs ?? 'pending'}`}
                assetPath={preview.assetPath}
                alt={buildPreviewAltText(preview, index + 1)}
                captureId={preview.captureId}
                requestId={preview.requestId}
                readyAtMs={preview.readyAtMs}
                isLatest={preview.isLatest}
                prioritizeLoading={preview.isLatest || preview.readyAtMs === null}
                visibilityLabelBase="recent-session"
              />
              <figcaption>
                촬영 당시{' '}
                {preview.presetDisplayName ??
                  (preview.isCurrentActivePreset ? '현재 룩' : '이전 룩')}{' '}
                룩
              </figcaption>
              <p className="latest-photo-rail__hint">
                {preview.isCurrentActivePreset
                  ? '현재 룩과 같은 바인딩으로 유지돼요.'
                  : '이 사진은 이전 룩으로 찍혔고 그대로 유지돼요.'}
              </p>
              {isRenderPending(preview) ? (
                <p className="latest-photo-rail__hint">{buildRenderPendingHint()}</p>
              ) : isPostEndLocked(preview) ? (
                <p className="latest-photo-rail__hint">
                  {buildPostEndHint(preview)}
                </p>
              ) : pendingDeleteCaptureId === preview.captureId ? (
                <div className="latest-photo-rail__confirm">
                  <p>이 사진을 정리할까요?</p>
                  <div className="latest-photo-rail__actions">
                    <button
                      type="button"
                      className="latest-photo-rail__action latest-photo-rail__action--secondary"
                      onClick={onDeleteCancel}
                      disabled={deletingCaptureId === preview.captureId}
                    >
                      취소
                    </button>
                    <button
                      type="button"
                      className="latest-photo-rail__action"
                      onClick={() => {
                        onDeleteConfirm(preview.captureId)
                      }}
                      disabled={deletingCaptureId === preview.captureId}
                    >
                      {deletingCaptureId === preview.captureId
                        ? '정리 중'
                        : '사진 정리'}
                    </button>
                  </div>
                </div>
              ) : (
                <button
                  type="button"
                  className="latest-photo-rail__action latest-photo-rail__action--secondary"
                  onClick={() => {
                    onDeleteIntent(preview.captureId)
                  }}
                  disabled={deletingCaptureId !== null}
                >
                  사진 정리
                </button>
              )}
            </figure>
          ))}
        </div>
      )}
    </article>
  )
}
