import { useEffect, useState } from 'react'

import type { HostErrorEnvelope } from '../../shared-contracts'
import type {
  PreviewRendererRouteMutationResult,
  PreviewRendererRoutePromotionStage,
  PreviewRendererRouteStatusResult,
} from '../../shared-contracts'
import type { BranchRolloutService } from '../services/branch-rollout-service'

type PreviewRouteGovernancePanelProps = {
  branchRolloutService: BranchRolloutService
}

function formatStageLabel(stage: PreviewRendererRoutePromotionStage) {
  return stage === 'canary' ? 'canary' : 'default'
}

export function PreviewRouteGovernancePanel({
  branchRolloutService,
}: PreviewRouteGovernancePanelProps) {
  const [presetId, setPresetId] = useState('')
  const [publishedVersion, setPublishedVersion] = useState('')
  const [targetRouteStage, setTargetRouteStage] =
    useState<PreviewRendererRoutePromotionStage>('canary')
  const [actorId, setActorId] = useState('')
  const [actorLabel, setActorLabel] = useState('')
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState<HostErrorEnvelope | null>(null)
  const [result, setResult] = useState<PreviewRendererRouteMutationResult | null>(null)
  const [status, setStatus] = useState<PreviewRendererRouteStatusResult | null>(null)
  const [isStatusLoading, setIsStatusLoading] = useState(false)

  const hasRequiredFields =
    presetId.trim() !== '' &&
    publishedVersion.trim() !== '' &&
    actorId.trim() !== '' &&
    actorLabel.trim() !== ''
  const hasStatusLookupFields =
    presetId.trim() !== '' && publishedVersion.trim() !== ''

  useEffect(() => {
    let cancelled = false

    async function loadStatus() {
      if (!hasStatusLookupFields) {
        setStatus(null)
        return
      }

      setIsStatusLoading(true)

      try {
        const nextStatus = await branchRolloutService.loadPreviewRendererRouteStatus({
          presetId: presetId.trim(),
          publishedVersion: publishedVersion.trim(),
        })

        if (!cancelled) {
          setStatus(nextStatus)
        }
      } catch (nextError) {
        if (!cancelled) {
          setError(nextError as HostErrorEnvelope)
          setStatus(null)
        }
      } finally {
        if (!cancelled) {
          setIsStatusLoading(false)
        }
      }
    }

    void loadStatus()

    return () => {
      cancelled = true
    }
  }, [branchRolloutService, hasStatusLookupFields, presetId, publishedVersion])

  async function submitPromotion() {
    if (!hasRequiredFields || isSubmitting) {
      return
    }

    setIsSubmitting(true)

    try {
      const nextResult = await branchRolloutService.promotePreviewRendererRoute({
        presetId: presetId.trim(),
        publishedVersion: publishedVersion.trim(),
        targetRouteStage,
        actorId: actorId.trim(),
        actorLabel: actorLabel.trim(),
      })

      setResult(nextResult)
      setError(null)
      setStatus({
        schemaVersion: 'preview-renderer-route-status-result/v1',
        presetId: nextResult.presetId,
        publishedVersion: nextResult.publishedVersion,
        routeStage: nextResult.routeStage,
        resolvedRoute:
          nextResult.routeStage === 'shadow' ? 'darktable' : 'local-renderer-sidecar',
        reason:
          nextResult.routeStage === 'default'
            ? 'host-approved-default'
            : 'host-approved-canary',
        message: `이 프리셋 버전은 ${nextResult.routeStage} 상태예요.`,
      })
    } catch (nextError) {
      setError(nextError as HostErrorEnvelope)
    } finally {
      setIsSubmitting(false)
    }
  }

  async function submitRollback() {
    if (!hasRequiredFields || isSubmitting) {
      return
    }

    setIsSubmitting(true)

    try {
      const nextResult = await branchRolloutService.rollbackPreviewRendererRoute({
        presetId: presetId.trim(),
        publishedVersion: publishedVersion.trim(),
        actorId: actorId.trim(),
        actorLabel: actorLabel.trim(),
      })

      setResult(nextResult)
      setError(null)
      setStatus({
        schemaVersion: 'preview-renderer-route-status-result/v1',
        presetId: nextResult.presetId,
        publishedVersion: nextResult.publishedVersion,
        routeStage: nextResult.routeStage,
        resolvedRoute: 'darktable',
        reason: 'rollback',
        message: '이 프리셋 버전은 shadow 상태예요.',
      })
    } catch (nextError) {
      setError(nextError as HostErrorEnvelope)
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <section className="branch-rollout">
      <article className="surface-card branch-rollout__hero">
        <div className="branch-rollout__hero-header">
          <div>
            <p className="branch-rollout__eyebrow">Preview Governance</p>
            <h2>Preview Route Governance</h2>
          </div>
        </div>
        <p className="branch-rollout__support">
          특정 프리셋과 게시 버전에만 preview route를 canary 또는 default로 승격하고,
          필요하면 한 번에 shadow 경로로 rollback합니다.
        </p>
        <p className="branch-rollout__support">
          여기서의 default는 전체 booth 기본값 변경이 아니라, 선택한 프리셋 버전의
          운영 기본 경로 승격입니다.
        </p>
        {error === null ? null : (
          <p className="branch-rollout__error">{error.message}</p>
        )}
      </article>

      <article className="surface-card branch-rollout__result-card">
        <h2>현재 Preview Route 상태</h2>
        {!hasStatusLookupFields ? (
          <p>프리셋 ID와 게시 버전을 넣으면 현재 상태를 바로 보여드릴게요.</p>
        ) : isStatusLoading ? (
          <p>현재 상태를 확인하는 중이에요.</p>
        ) : status === null ? (
          <p>현재 상태를 아직 확인하지 못했어요.</p>
        ) : (
          <>
            <p>현재 상태: {status.routeStage}</p>
            <p>적용 경로: {status.resolvedRoute}</p>
            <p>{status.message}</p>
          </>
        )}
      </article>

      <article className="surface-card branch-rollout__control-card">
        <div className="branch-rollout__control-grid">
          <label className="session-start-form__field">
            <span className="session-start-form__label">프리셋 ID</span>
            <input
              aria-label="프리셋 ID"
              className="session-start-form__input"
              value={presetId}
              onChange={(event) => setPresetId(event.target.value)}
            />
          </label>

          <label className="session-start-form__field">
            <span className="session-start-form__label">게시 버전</span>
            <input
              aria-label="게시 버전"
              className="session-start-form__input"
              value={publishedVersion}
              onChange={(event) => setPublishedVersion(event.target.value)}
            />
          </label>

          <label className="session-start-form__field">
            <span className="session-start-form__label">승격 단계</span>
            <select
              aria-label="승격 단계"
              className="session-start-form__input"
              value={targetRouteStage}
              onChange={(event) =>
                setTargetRouteStage(event.target.value as PreviewRendererRoutePromotionStage)
              }
            >
              <option value="canary">canary</option>
              <option value="default">default</option>
            </select>
          </label>
 
          <label className="session-start-form__field">
            <span className="session-start-form__label">Preview route 승인자 ID</span>
            <input
              aria-label="Preview route 승인자 ID"
              className="session-start-form__input"
              value={actorId}
              onChange={(event) => setActorId(event.target.value)}
            />
          </label>

          <label className="session-start-form__field">
            <span className="session-start-form__label">Preview route 승인자 이름</span>
            <input
              aria-label="Preview route 승인자 이름"
              className="session-start-form__input"
              value={actorLabel}
              onChange={(event) => setActorLabel(event.target.value)}
            />
          </label>
        </div>

        <div className="branch-rollout__actions">
          <button
            type="button"
            className="surface-card__action"
            disabled={!hasRequiredFields || isSubmitting}
            onClick={() => void submitPromotion()}
          >
            {isSubmitting
              ? '적용 중...'
              : `${formatStageLabel(targetRouteStage)} 승격 적용`}
          </button>
          <button
            type="button"
            className="surface-card__action surface-card__action--secondary"
            disabled={!hasRequiredFields || isSubmitting}
            onClick={() => void submitRollback()}
          >
            {isSubmitting ? '확인 중...' : 'shadow로 rollback'}
          </button>
        </div>
      </article>

      {result === null ? null : (
        <article className="surface-card branch-rollout__result-card">
          <h2>최근 Preview Route 결과</h2>
          <p>{result.message}</p>
          <p>
            대상: {result.presetId} / {result.publishedVersion}
          </p>
          <p>
            적용 단계: {result.routeStage} / 증거 누적: {result.auditEntry.canarySuccessCount}
          </p>
        </article>
      )}
    </section>
  )
}
