import type {
  SessionPostEndRecord,
  SessionTimingSnapshot,
} from '../../shared-contracts'
import { SessionTimingPanel } from '../../timing-policy/components/SessionTimingPanel'

type ReadinessScreenProps = {
  boothAlias: string | null
  selectedPresetName: string | null
  postEndGuidance: SessionPostEndRecord | null
  timing: SessionTimingSnapshot | null
  stateLabel: string
  cameraStatusLabel: string
  cameraStatusDetail: string
  cameraStatusTone: 'ready' | 'neutral' | 'blocked'
  actionLabel: string
  canCapture: boolean
  isBusy: boolean
  isExplicitPostEnd: boolean
  isChangePresetDisabled: boolean
  onPrimaryAction(): void
  onChangePreset(): void
}

export function ReadinessScreen({
  boothAlias,
  selectedPresetName,
  postEndGuidance,
  timing,
  stateLabel,
  cameraStatusLabel,
  cameraStatusDetail,
  cameraStatusTone,
  actionLabel,
  canCapture,
  isBusy,
  isExplicitPostEnd,
  isChangePresetDisabled,
  onPrimaryAction,
  onChangePreset,
}: ReadinessScreenProps) {
  const shouldHidePrimaryAction =
    postEndGuidance?.state === 'completed' ||
    postEndGuidance?.state === 'phone-required'
  const shouldHideTimingPanel =
    postEndGuidance?.state === 'completed' ||
    postEndGuidance?.state === 'phone-required' ||
    (isExplicitPostEnd && timing !== null && timing.phase === 'ended')
  const currentLookDetail = shouldHidePrimaryAction
    ? '이번 세션에서 사용된 룩이 그대로 반영돼요.'
    : '지금 바꾸면 다음 촬영부터만 새 룩이 적용돼요.'

  return (
    <>
      {timing !== null && !shouldHideTimingPanel ? (
        <SessionTimingPanel timing={timing} canCapture={canCapture} />
      ) : null}

      <article className="surface-card readiness-screen__summary">
        <div>
          <h2>현재 세션</h2>
          <p>{boothAlias ?? '세션 확인 중'}</p>
        </div>
        <div>
          <h2>현재 룩</h2>
          <p>{selectedPresetName ?? '선택 대기 중'}</p>
          <p>{currentLookDetail}</p>
        </div>
        <div className="readiness-screen__camera-status">
          <h2>카메라 상태</h2>
          <p
            className={`readiness-screen__camera-badge readiness-screen__camera-badge--${cameraStatusTone}`}
          >
            {cameraStatusLabel}
          </p>
          <p>{cameraStatusDetail}</p>
        </div>
      </article>

      <article className="surface-card readiness-screen__action-card">
        {isExplicitPostEnd ? (
          <div
            className={`readiness-screen__post-end${
              postEndGuidance?.state === 'phone-required'
                ? ' readiness-screen__post-end--phone-required'
                : ''
            }`}
          >
            <p className="readiness-screen__post-end-label">{stateLabel}</p>
          </div>
        ) : (
          <button
            type="button"
            className="latest-photo-rail__action latest-photo-rail__action--secondary"
            disabled={isChangePresetDisabled}
            onClick={onChangePreset}
          >
            다음 촬영 룩 바꾸기
          </button>
        )}
        {!shouldHidePrimaryAction ? (
          <button
            type="button"
            className="session-start-form__submit readiness-screen__action"
            disabled={!canCapture || isBusy}
            onClick={onPrimaryAction}
          >
            {actionLabel}
          </button>
        ) : null}
      </article>
    </>
  )
}
