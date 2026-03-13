import { useEffect, useEffectEvent } from 'react'

import type {
  PresetCatalogLoadState,
  PresetCatalogService,
} from '../../preset-catalog/services/presetCatalogService.js'
import type { PresetId } from '../../shared-contracts/presets/presetCatalog.js'
import { selectActivePreset, type CaptureFlowState } from '../../session-domain/state/captureFlowState.js'
import { HardFramePanel } from '../../shared-ui/components/HardFramePanel.js'
import { PrimaryActionButton } from '../../shared-ui/components/PrimaryActionButton.js'
import { ActivePresetCard } from '../components/ActivePresetCard.js'
import { DeletePhotoDialog } from '../components/DeletePhotoDialog.js'
import { LatestPhotoPanel } from '../components/LatestPhotoPanel.js'
import { LatestPhotoReviewRail } from '../components/LatestPhotoReviewRail.js'
import { PresetCatalogSheet } from '../components/PresetCatalogSheet.js'
import { PresetChangeToast } from '../components/PresetChangeToast.js'
import { SessionTimeBanner } from '../components/SessionTimeBanner.js'
import { captureScreenCopy } from '../copy/captureScreenCopy.js'
import { reviewRailCopy } from '../copy/reviewRailCopy.js'
import type { CaptureConfidenceView } from '../selectors/captureConfidenceView.js'
import { ReviewScreen } from './ReviewScreen.js'

type CaptureConfidenceScreenProps = {
  sessionName: string
  view: CaptureConfidenceView
}

type InteractiveCaptureScreenProps = {
  captureState: CaptureFlowState
  captureActionDisabled?: boolean
  deletePending?: boolean
  isDeleteDialogOpen?: boolean
  isReviewOpen?: boolean
  onCancelDeleteDialog?(): void
  onCapture(): void
  onClosePresetSelector(): void
  onCloseReviewDialog?(): void
  onConfirmDeletePhoto?(): void | Promise<void>
  onDismissPresetChangeFeedback(): void
  onDismissReviewFeedback?(): void
  onOpenDeleteDialog?(captureId: string): void
  onOpenPresetSelector(): void
  onOpenReview?(): void
  onSelectPreset(presetId: PresetId): void
  onSelectReviewCapture?(captureId: string): void
  presetCatalogState?: PresetCatalogLoadState
  presetSelectionDisabled?: boolean
  presetCatalogService?: PresetCatalogService
  pendingDeletePhotoLabel?: string | null
  reviewFeedback?: string | null
  reviewItems?: Array<{
    captureId: string
    isLatest: boolean
    label: string
    thumbnailPath: string
  }>
  selectedReviewCaptureId?: string | null
  selectedReviewPhoto?: {
    captureId: string
    label: string
    previewPath: string
  } | null
  sessionName: string
  view: CaptureConfidenceView
}

type CaptureScreenProps = CaptureConfidenceScreenProps | InteractiveCaptureScreenProps

function renderConfidenceScreen({ sessionName, view }: CaptureConfidenceScreenProps) {
  return (
    <main className="customer-shell">
      <HardFramePanel className="customer-shell__panel customer-capture">
        <header className="customer-capture__header">
          <p className="customer-shell__eyebrow">Active Capture</p>
          <p aria-label="세션 이름" className="customer-capture__session-name">
            {sessionName}
          </p>
        </header>

        <section className="customer-capture__grid">
          <div className="customer-capture__rail">
            <div className="customer-capture__intro">
              <p className="customer-capture__kicker">Trust anchors stay visible</p>
              <h1 className="customer-capture__title">촬영 흐름을 벗어나지 않고도 진행 상태를 바로 확인할 수 있어요.</h1>
              <p className="customer-capture__supporting">
                종료 시간, 현재 프리셋, 방금 저장된 사진만 크게 보여드려서 고객이 촬영 성공 여부를 빠르게 확인할 수 있게
                합니다.
              </p>
            </div>

            <SessionTimeBanner {...view.endTime} />
            <ActivePresetCard {...view.preset} />

            <section aria-label="촬영 안내" className="capture-guidance-card">
              <p className="capture-signal-card__label">촬영 안내</p>
              <p aria-live={view.timingAlert.kind === 'none' ? undefined : 'polite'} className="capture-guidance-card__value">
                {view.guidance}
              </p>
            </section>
          </div>

          <LatestPhotoPanel latestPhoto={view.latestPhoto} />
        </section>
      </HardFramePanel>
    </main>
  )
}

function InteractiveCaptureScreen({
  captureState,
  captureActionDisabled = false,
  deletePending = false,
  isDeleteDialogOpen = false,
  isReviewOpen = false,
  onCancelDeleteDialog,
  onCapture,
  onClosePresetSelector,
  onCloseReviewDialog,
  onConfirmDeletePhoto,
  onDismissPresetChangeFeedback,
  onDismissReviewFeedback,
  onOpenDeleteDialog,
  onOpenPresetSelector,
  onOpenReview,
  onSelectPreset,
  onSelectReviewCapture,
  presetCatalogState,
  presetSelectionDisabled = false,
  presetCatalogService,
  pendingDeletePhotoLabel = null,
  reviewFeedback = null,
  reviewItems,
  selectedReviewCaptureId = null,
  selectedReviewPhoto = null,
  sessionName,
  view,
}: InteractiveCaptureScreenProps) {
  const activePreset = selectActivePreset(captureState)
  const dismissFeedback = useEffectEvent(() => {
    onDismissPresetChangeFeedback()
  })
  const dismissReviewFeedback = useEffectEvent(() => {
    onDismissReviewFeedback?.()
  })
  const showReviewSurface = reviewItems !== undefined

  useEffect(() => {
    if (!captureState.pendingPresetChangeMessage) {
      return undefined
    }

    const timeoutId = window.setTimeout(() => {
      dismissFeedback()
    }, 2200)

    return () => {
      window.clearTimeout(timeoutId)
    }
  }, [captureState.pendingPresetChangeMessage])

  useEffect(() => {
    if (!reviewFeedback || !onDismissReviewFeedback) {
      return undefined
    }

    const timeoutId = window.setTimeout(() => {
      dismissReviewFeedback()
    }, 2200)

    return () => {
      window.clearTimeout(timeoutId)
    }
  }, [onDismissReviewFeedback, reviewFeedback])

  return (
    <main className="customer-shell">
      <HardFramePanel className="customer-shell__panel customer-capture">
        <header className="customer-capture__header">
          <p className="customer-shell__eyebrow">{captureScreenCopy.eyebrow}</p>
          <p aria-label={captureScreenCopy.sessionLabel} className="customer-capture__session-name">
            {sessionName}
          </p>
        </header>

        <section className="customer-capture__grid">
          <div className="customer-capture__rail">
            <div className="customer-capture__intro">
              <p className="customer-capture__kicker">{captureScreenCopy.activePresetBadge}</p>
              <h1 className="customer-capture__title">{captureScreenCopy.title}</h1>
              <p className="customer-capture__supporting">{captureScreenCopy.supporting}</p>
            </div>

            <SessionTimeBanner {...view.endTime} />

            <section aria-label={captureScreenCopy.activePresetLabel} className="capture-signal-card">
              <p className="capture-signal-card__label">{captureScreenCopy.activePresetLabel}</p>
              <p className="capture-signal-card__value">{activePreset.name}</p>
              <p className="capture-signal-card__supporting">{activePreset.description}</p>
            </section>

            <div className="customer-shell__actions customer-capture__actions">
              <button
                className="secondary-action-button"
                disabled={presetSelectionDisabled}
                onClick={onOpenPresetSelector}
                type="button"
              >
                {captureScreenCopy.openPresetSelector}
              </button>
              <PrimaryActionButton
                disabled={captureActionDisabled}
                label={captureScreenCopy.captureAction}
                onClick={onCapture}
              />
            </div>

            <section aria-label="촬영 안내" className="capture-guidance-card">
              <p className="capture-signal-card__label">촬영 안내</p>
              <p className="capture-guidance-card__value">{view.guidance}</p>
            </section>

            {showReviewSurface
              ? reviewItems.length > 0
                ? (
                    <>
                      <LatestPhotoReviewRail
                        items={reviewItems}
                        onSelectCapture={(captureId) => {
                          onSelectReviewCapture?.(captureId)
                        }}
                        selectedCaptureId={selectedReviewCaptureId}
                      />

                      {selectedReviewPhoto ? (
                        <div className="customer-shell__actions review-dialog__actions">
                          <button className="secondary-action-button" onClick={onOpenReview} type="button">
                            {reviewRailCopy.openReview}
                          </button>
                          <button
                            className="secondary-action-button"
                            disabled={deletePending}
                            onClick={() => {
                              onOpenDeleteDialog?.(selectedReviewPhoto.captureId)
                            }}
                            type="button"
                          >
                            {reviewRailCopy.deletePhoto}
                          </button>
                        </div>
                      ) : null}
                    </>
                  )
                : (
                    <section aria-label="현재 세션 사진 검토" className="review-rail">
                      <div className="review-rail__header">
                        <p className="review-rail__eyebrow">Session Review</p>
                        <p className="review-rail__supporting">{reviewRailCopy.emptySupporting}</p>
                      </div>
                    </section>
                  )
              : null}

            {showReviewSurface && reviewFeedback ? (
              <p aria-live="polite" className="review-dialog__supporting">
                {reviewFeedback}
              </p>
            ) : null}

            {captureState.pendingPresetChangeMessage ? (
              <PresetChangeToast message={captureState.pendingPresetChangeMessage} />
            ) : null}
          </div>

          <section aria-label={captureScreenCopy.latestPhotoLabel} className="customer-capture__latest" role="region">
            <LatestPhotoPanel latestPhoto={view.latestPhoto} />
          </section>
        </section>

        {captureState.isPresetSelectorOpen ? (
          <PresetCatalogSheet
            activePresetId={captureState.activePresetId}
            catalogState={presetCatalogState}
            onClose={onClosePresetSelector}
            onSelectPreset={onSelectPreset}
            selectionDisabled={presetSelectionDisabled}
            presetCatalogService={presetCatalogService}
          />
        ) : null}

        {showReviewSurface && isReviewOpen && selectedReviewPhoto ? (
          <ReviewScreen
            deletePending={deletePending}
            onClose={() => {
              onCloseReviewDialog?.()
            }}
            onRequestDelete={(captureId) => {
              onOpenDeleteDialog?.(captureId)
            }}
            photo={selectedReviewPhoto}
            sessionName={sessionName}
          />
        ) : null}

        {showReviewSurface && isDeleteDialogOpen && pendingDeletePhotoLabel ? (
          <DeletePhotoDialog
            deletePending={deletePending}
            onCancel={() => {
              onCancelDeleteDialog?.()
            }}
            onConfirm={() => {
              void onConfirmDeletePhoto?.()
            }}
            photoLabel={pendingDeletePhotoLabel}
          />
        ) : null}
      </HardFramePanel>
    </main>
  )
}

export function CaptureScreen(props: CaptureScreenProps) {
  if ('captureState' in props) {
    return <InteractiveCaptureScreen {...props} />
  }

  return renderConfidenceScreen(props)
}
