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

type PreviewCardProps = {
  preview: CurrentSessionPreview
  position: number
  isExplicitPostEnd: boolean
  deletingCaptureId: string | null
  pendingDeleteCaptureId: string | null
  onDeleteCancel(): void
  onDeleteConfirm(captureId: string): void
  onDeleteIntent(captureId: string): void
  visibilityLabelBase: string
  className: string
  role?: 'listitem'
}

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

function PreviewCard({
  preview,
  position,
  isExplicitPostEnd,
  deletingCaptureId,
  pendingDeleteCaptureId,
  onDeleteCancel,
  onDeleteConfirm,
  onDeleteIntent,
  visibilityLabelBase,
  className,
  role,
}: PreviewCardProps) {
  function isRenderPending() {
    return preview.readyAtMs === null
  }

  function isPostEndLocked() {
    return isExplicitPostEnd || preview.postEndState !== 'activeSession'
  }

  function buildPostEndHint() {
    return preview.postEndState === 'postEndPending' || isExplicitPostEnd
      ? '지금은 결과 안내가 우선이라 여기서 사진을 정리할 수 없어요.'
      : isFinalizedCapturePostEndState(preview.postEndState)
        ? '마무리된 사진은 여기서 정리할 수 없어요.'
        : '지금은 사진 정리를 잠시 멈춰 주세요.'
  }

  function buildRenderPendingHint() {
    return '지금 보이는 첫 사진은 임시 미리보기예요. 현재 룩을 적용한 최종 사진으로 곧 바뀌어요.'
  }

  return (
    <figure className={className} role={role}>
      {preview.isLatest ? (
        <span className="latest-photo-rail__badge">최신 사진</span>
      ) : null}
      {isRenderPending() ? (
        <span className="latest-photo-rail__badge">룩 적용 중</span>
      ) : null}
      <SessionPreviewImage
        key={`${preview.captureId}:${preview.assetPath}:${preview.readyAtMs ?? 'pending'}`}
        assetPath={preview.assetPath}
        alt={buildPreviewAltText(preview, position)}
        captureId={preview.captureId}
        requestId={preview.requestId}
        readyAtMs={preview.readyAtMs}
        isLatest={preview.isLatest}
        prioritizeLoading={preview.isLatest || preview.readyAtMs === null}
        visibilityLabelBase={visibilityLabelBase}
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
      {isRenderPending() ? (
        <p className="latest-photo-rail__hint">{buildRenderPendingHint()}</p>
      ) : isPostEndLocked() ? (
        <p className="latest-photo-rail__hint">{buildPostEndHint()}</p>
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
              {deletingCaptureId === preview.captureId ? '정리 중' : '사진 정리'}
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
  )
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

  const latestPreview = previews.find((preview) => preview.isLatest) ?? previews[0] ?? null
  const historyPreviews =
    latestPreview === null
      ? []
      : previews.filter((preview) => preview.captureId !== latestPreview.captureId)
  const previewPositions = new Map(
    previews.map((preview, index) => [preview.captureId, index + 1]),
  )

  return (
    <article className="surface-card latest-photo-rail">
      <div className="latest-photo-rail__header">
        <h2>방금 찍은 사진</h2>
        <p>
          {isPreviewWaiting && latestPreview === null
            ? '같은 촬영 결과를 먼저 크게 보여주기 위해 준비 중이에요. 방금 저장한 컷이 확인 가능해지면 여기부터 채워져요.'
            : isPreviewWaiting
              ? '가장 최근 컷을 먼저 크게 보여 주고, 현재 룩 반영이 끝나면 자연스럽게 교체해요.'
              : '가장 최근 컷을 먼저 확인하고, 이전 컷은 아래 히스토리로 유지해요.'}
        </p>
      </div>

      {latestPreview === null ? (
        <p className="latest-photo-rail__empty">
          {isPreviewWaiting
            ? '잠시만 기다리면 방금 찍은 사진이 여기에 나타나요.'
            : '아직 보여 줄 현재 컷이 없어요.'}
        </p>
      ) : (
        <>
          <PreviewCard
            preview={latestPreview}
            position={previewPositions.get(latestPreview.captureId) ?? 1}
            isExplicitPostEnd={isExplicitPostEnd}
            deletingCaptureId={deletingCaptureId}
            pendingDeleteCaptureId={pendingDeleteCaptureId}
            onDeleteCancel={onDeleteCancel}
            onDeleteConfirm={onDeleteConfirm}
            onDeleteIntent={onDeleteIntent}
            visibilityLabelBase="current-capture"
            className="latest-photo-rail__spotlight latest-photo-rail__item latest-photo-rail__item--latest"
          />

          {historyPreviews.length > 0 ? (
            <section className="latest-photo-rail__history">
              <div className="latest-photo-rail__history-header">
                <h3>현재 세션 히스토리</h3>
                <p>이전 컷은 여기에서 유지돼요.</p>
              </div>
              <div
                className="latest-photo-rail__scroller"
                role="list"
                aria-label="현재 세션 사진 히스토리"
                tabIndex={0}
                onKeyDown={handleRailKeyDown}
              >
                {historyPreviews.map((preview) => (
                  <PreviewCard
                    key={preview.captureId}
                    preview={preview}
                    position={previewPositions.get(preview.captureId) ?? 1}
                    isExplicitPostEnd={isExplicitPostEnd}
                    deletingCaptureId={deletingCaptureId}
                    pendingDeleteCaptureId={pendingDeleteCaptureId}
                    onDeleteCancel={onDeleteCancel}
                    onDeleteConfirm={onDeleteConfirm}
                    onDeleteIntent={onDeleteIntent}
                    visibilityLabelBase="recent-session"
                    className="latest-photo-rail__item"
                    role="listitem"
                  />
                ))}
              </div>
            </section>
          ) : null}
        </>
      )}
    </article>
  )
}
