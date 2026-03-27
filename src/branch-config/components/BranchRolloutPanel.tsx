import { useEffect, useState } from 'react'

import type {
  BranchRollbackInput,
  BranchRolloutActionResult,
  BranchRolloutOverviewResult,
  BranchRolloutBranchState,
  BranchRolloutInput,
} from '../../shared-contracts'
import type { HostErrorEnvelope } from '../../shared-contracts'
import type { BranchRolloutService } from '../services/branch-rollout-service'

type BranchRolloutPanelProps = {
  branchRolloutService: BranchRolloutService
}

type SelectionState = Record<string, boolean>

function formatBaselineOption(
  baseline: BranchRolloutOverviewResult['approvedBaselines'][number],
) {
  return `${baseline.buildVersion} :: ${baseline.presetStackVersion}`
}

function describeBranchSession(branch: BranchRolloutBranchState) {
  if (branch.activeSession === null) {
    return '현재 진행 중인 세션 없이 바로 적용할 수 있어요.'
  }

  return '진행 중인 세션이 있어 안전한 전환 시점 이후에만 새 baseline이 적용돼요.'
}

function summarizeSelection(
  branches: BranchRolloutOverviewResult['branches'],
  selectedBranchIds: string[],
) {
  return branches.filter((branch) => selectedBranchIds.includes(branch.branchId))
}

export function BranchRolloutPanel({
  branchRolloutService,
}: BranchRolloutPanelProps) {
  const [overview, setOverview] = useState<BranchRolloutOverviewResult | null>(null)
  const [error, setError] = useState<HostErrorEnvelope | null>(null)
  const [selection, setSelection] = useState<SelectionState>({})
  const [selectedBaselineValue, setSelectedBaselineValue] = useState('')
  const [actorId, setActorId] = useState('')
  const [actorLabel, setActorLabel] = useState('')
  const [isLoading, setIsLoading] = useState(true)
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [result, setResult] = useState<BranchRolloutActionResult | null>(null)

  useEffect(() => {
    let cancelled = false

    async function load() {
      setIsLoading(true)

      try {
        const nextOverview = await branchRolloutService.loadOverview()

        if (!cancelled) {
          setOverview(nextOverview)
          setError(null)
        }
      } catch (nextError) {
        if (!cancelled) {
          setError(nextError as HostErrorEnvelope)
        }
      } finally {
        if (!cancelled) {
          setIsLoading(false)
        }
      }
    }

    void load()

    return () => {
      cancelled = true
    }
  }, [branchRolloutService])

  const selectedBranchIds = Object.entries(selection)
    .filter(([, isSelected]) => isSelected)
    .map(([branchId]) => branchId)

  const selectedBaseline =
    overview?.approvedBaselines.find(
      (baseline) => formatBaselineOption(baseline) === selectedBaselineValue,
    ) ?? null
  const selectedBranches =
    overview === null
      ? []
      : summarizeSelection(overview.branches, selectedBranchIds)

  const canSubmitRollout =
    overview !== null &&
    selectedBranchIds.length > 0 &&
    selectedBaseline !== null &&
    actorId.trim() !== '' &&
    actorLabel.trim() !== '' &&
    !isSubmitting

  const canSubmitRollback =
    overview !== null &&
    selectedBranchIds.length > 0 &&
    actorId.trim() !== '' &&
    actorLabel.trim() !== '' &&
    !isSubmitting

  async function submitRollout() {
    if (!canSubmitRollout || selectedBaseline === null) {
      return
    }

    setIsSubmitting(true)

    try {
      const payload: BranchRolloutInput = {
        branchIds: selectedBranchIds,
        targetBaseline: {
          ...selectedBaseline,
          actorId: actorId.trim(),
          actorLabel: actorLabel.trim(),
        },
        actorId: actorId.trim(),
        actorLabel: actorLabel.trim(),
      }
      const nextResult = await branchRolloutService.applyRollout(payload)

      setResult(nextResult)
      setError(null)
    } catch (nextError) {
      setError(nextError as HostErrorEnvelope)
    } finally {
      setIsSubmitting(false)
    }
  }

  async function submitRollback() {
    if (!canSubmitRollback) {
      return
    }

    setIsSubmitting(true)

    try {
      const payload: BranchRollbackInput = {
        branchIds: selectedBranchIds,
        actorId: actorId.trim(),
        actorLabel: actorLabel.trim(),
      }
      const nextResult = await branchRolloutService.applyRollback(payload)

      setResult(nextResult)
      setError(null)
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
            <p className="branch-rollout__eyebrow">Release Governance</p>
            <h2>Branch Rollout Governance</h2>
          </div>
        </div>
        <p className="branch-rollout__support">
          선택한 지점 집합에만 rollout 또는 단일 액션 rollback을 적용하고, 진행 중인
          세션은 안전한 전환 시점까지 그대로 유지합니다.
        </p>
        {error === null ? null : (
          <p className="branch-rollout__error">{error.message}</p>
        )}
      </article>

      {isLoading ? (
        <article className="surface-card">
          <p>지점 배포 baseline을 불러오는 중이에요.</p>
        </article>
      ) : overview === null || overview.branches.length === 0 ? (
        <article className="surface-card">
          <h2>관리 중인 지점이 아직 없어요</h2>
          <p>branch-config가 준비되면 승인 baseline과 지점별 rollout 상태를 여기에서 보여 드릴게요.</p>
        </article>
      ) : (
        <>
          <article className="surface-card branch-rollout__control-card">
            <div className="branch-rollout__control-grid">
              <fieldset
                aria-label="대상 지점 선택"
                className="session-start-form__field branch-rollout__fieldset"
              >
                <legend className="session-start-form__label">대상 지점 선택</legend>
                <div className="branch-rollout__branch-list">
                  {overview.branches.map((branch) => (
                    <label key={branch.branchId} className="branch-rollout__branch-option">
                      <input
                        type="checkbox"
                        checked={selection[branch.branchId] ?? false}
                        onChange={(event) =>
                          setSelection((current) => ({
                            ...current,
                            [branch.branchId]: event.target.checked,
                          }))
                        }
                      />
                      <span>{branch.displayName}</span>
                    </label>
                  ))}
                </div>
              </fieldset>

              <label className="session-start-form__field">
                <span className="session-start-form__label">배포 target baseline</span>
                <select
                  aria-label="배포 target baseline"
                  className="session-start-form__input"
                  value={selectedBaselineValue}
                  onChange={(event) => setSelectedBaselineValue(event.target.value)}
                >
                  <option value="">승인된 baseline을 선택해 주세요.</option>
                  {overview.approvedBaselines.map((baseline) => (
                    <option
                      key={`${baseline.buildVersion}-${baseline.presetStackVersion}`}
                      value={formatBaselineOption(baseline)}
                    >
                      {formatBaselineOption(baseline)}
                    </option>
                  ))}
                </select>
              </label>

              <label className="session-start-form__field">
                <span className="session-start-form__label">승인자 ID</span>
                <input
                  aria-label="승인자 ID"
                  className="session-start-form__input"
                  value={actorId}
                  onChange={(event) => setActorId(event.target.value)}
                />
              </label>

              <label className="session-start-form__field">
                <span className="session-start-form__label">승인자 이름</span>
                <input
                  aria-label="승인자 이름"
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
                disabled={!canSubmitRollout}
                onClick={() => void submitRollout()}
              >
                {isSubmitting ? '적용 중...' : '선택한 지점에 rollout'}
              </button>
              <button
                type="button"
                className="surface-card__action surface-card__action--secondary"
                disabled={!canSubmitRollback}
                onClick={() => void submitRollback()}
              >
                {isSubmitting ? '확인 중...' : '선택한 지점에 rollback'}
              </button>
            </div>
          </article>

          <article className="surface-card branch-rollout__selection-card">
            <h2>선택 요약</h2>
            {selectedBranches.length === 0 ? (
              <p>먼저 대상 지점을 선택해 주세요.</p>
            ) : (
              <div className="branch-rollout__branch-cards">
                {selectedBranches.map((branch) => (
                  <article key={branch.branchId} className="branch-rollout__branch-card">
                    <h3>{branch.displayName}</h3>
                    <p>{branch.localSettings.summary}</p>
                    <p>{describeBranchSession(branch)}</p>
                  </article>
                ))}
              </div>
            )}
          </article>

          <article className="surface-card branch-rollout__state-card">
            <h2>지점 상태</h2>
            <div className="branch-rollout__branch-cards">
              {overview.branches.map((branch) => (
                <article key={branch.branchId} className="branch-rollout__branch-card">
                  <h3>{branch.displayName}</h3>
                  <p>{branch.localSettings.summary}</p>
                  <p>
                    현재 baseline: {branch.deploymentBaseline.buildVersion} /{' '}
                    {branch.deploymentBaseline.presetStackVersion}
                  </p>
                  {branch.pendingBaseline === null ? null : (
                    <p>
                      staged baseline: {branch.pendingBaseline.buildVersion} /{' '}
                      {branch.pendingBaseline.presetStackVersion}
                    </p>
                  )}
                  <p>{describeBranchSession(branch)}</p>
                </article>
              ))}
            </div>
          </article>

          {result === null ? null : (
            <article className="surface-card branch-rollout__result-card">
              <h2>최근 결과</h2>
              <p>{result.message}</p>
              <div className="branch-rollout__branch-cards">
                {result.outcomes.map((outcome) => (
                  <article key={outcome.branchId} className="branch-rollout__branch-card">
                    <h3>{outcome.displayName}</h3>
                    <p>{outcome.localSettings.summary}</p>
                    <p>{outcome.compatibility.summary}</p>
                    {outcome.rejection === null ? null : (
                      <p>{outcome.rejection.guidance}</p>
                    )}
                  </article>
                ))}
              </div>
            </article>
          )}
        </>
      )}
    </section>
  )
}
