import type {
  OperatorAuditEntry,
  OperatorAuditQueryResult,
  OperatorBlockedStateCategory,
  OperatorBoundarySummary,
  OperatorCameraConnectionState,
  OperatorCameraConnectionSummary,
  OperatorRecoveryAction,
  OperatorRecoverySummary,
  HostErrorEnvelope,
} from '../../shared-contracts'
import { Link } from 'react-router-dom'
import { useCapabilityService } from '../../app/providers/use-capability-service'
import { SurfaceLayout } from '../../shared-ui/layout/SurfaceLayout'
import { useOperatorDiagnostics } from '../providers/use-operator-diagnostics'

function describeBlockedStateCategory(
  category: OperatorBlockedStateCategory | null,
  hasUnavailableSummary: boolean,
) {
  if (hasUnavailableSummary) {
    return '지금은 최신 진단을 불러오지 못해 막힌 운영 경계를 확정할 수 없어요.'
  }

  if (category === null) {
    return 'host가 현재 세션 문맥과 막힌 경계를 정리하는 중이에요.'
  }

  switch (category) {
    case 'capture-blocked':
      return '촬영 경계가 아직 열리지 않아 capture 전 단계에서 확인이 필요해요.'
    case 'preview-render-blocked':
      return '가장 최근 촬영본의 preview 또는 render 결과 준비가 막혀 있어요.'
    case 'timing-post-end-blocked':
      return '세션 종료 후 완료 판정 또는 후속 안내 정리가 아직 끝나지 않았어요.'
    case 'not-blocked':
    default:
      return '현재 요약 기준으로는 막힌 운영 경계가 보고되지 않았어요.'
  }
}

function describeSummaryHeading(
  summary: OperatorRecoverySummary | null,
  hasUnavailableSummary: boolean,
) {
  if (summary === null) {
    return hasUnavailableSummary
      ? '현재 세션 진단을 불러오지 못했어요'
      : '현재 세션 진단을 확인하는 중이에요'
  }

  if (summary.state === 'no-session') {
    return '진행 중인 세션이 없어요'
  }

  if (summary.blockedStateCategory === 'not-blocked') {
    return '현재 세션은 막힌 경계 없이 진행 중이에요'
  }

  switch (summary.blockedStateCategory) {
    case 'capture-blocked':
      return '현재 세션의 capture 경계 확인이 필요해요'
    case 'preview-render-blocked':
      return '현재 세션의 preview/render 경계 확인이 필요해요'
    case 'timing-post-end-blocked':
      return '현재 세션의 종료 후 경계 확인이 필요해요'
    default:
      return '현재 세션 진단을 다시 확인해 주세요'
  }
}

function formatFieldValue(value: string | null | undefined) {
  return value === null || value === undefined || value.trim() === ''
    ? '아직 없음'
    : value
}

const PRESET_APPLIED_GOAL_MS = 2_000

function formatMetricMs(value: number | null | undefined) {
  if (value === null || value === undefined) {
    return '아직 없음'
  }

  return `${(value / 1_000).toFixed(1)}초 (${value}ms)`
}

function formatGoalSpeed(replacementMs: number | null | undefined) {
  const goalSeconds = (PRESET_APPLIED_GOAL_MS / 1_000).toFixed(1)

  if (replacementMs === null || replacementMs === undefined) {
    return `목표 ${goalSeconds}초 이하 · 아직 계측 없음`
  }

  return `목표 ${goalSeconds}초 이하 · 현재 ${(replacementMs / 1_000).toFixed(1)}초`
}

function formatLifecycleStage(value: string | null | undefined) {
  switch (value) {
    case 'session-started':
      return '세션 시작됨'
    case 'capture-ready':
      return '촬영 준비 완료'
    case 'preview-waiting':
      return 'preview/render 대기 중'
    case 'warning':
      return '종료 경고 구간'
    case 'ended':
      return '세션 종료됨'
    case 'export-waiting':
      return '종료 후 결과 정리 중'
    case 'completed':
      return '완료'
    case 'phone-required':
      return '직원 확인 필요'
    case 'helper-preparing':
      return '보조 준비 중'
    case 'camera-preparing':
    case 'preparing':
      return '준비 중'
    default:
      return formatFieldValue(value)
  }
}

function formatTimingPhase(value: string | null | undefined) {
  switch (value) {
    case 'active':
      return '진행 중'
    case 'warning':
      return '경고'
    case 'ended':
      return '종료됨'
    default:
      return formatFieldValue(value)
  }
}

function formatPostEndState(value: string | null | undefined) {
  switch (value) {
    case 'export-waiting':
      return '결과 정리 중'
    case 'completed':
      return '완료'
    case 'phone-required':
      return '직원 확인 필요'
    default:
      return formatFieldValue(value)
  }
}

function formatBoundaryStatus(boundary: OperatorBoundarySummary) {
  return boundary.status === 'blocked' ? 'blocked' : 'clear'
}

function formatCameraConnectionStateLabel(state: OperatorCameraConnectionState) {
  switch (state) {
    case 'disconnected':
      return '미연결'
    case 'connecting':
      return '연결 중'
    case 'connected':
      return '연결됨'
    case 'recovery-required':
    default:
      return '복구 필요'
  }
}

function PreviewArchitectureCard({ summary }: { summary: OperatorRecoverySummary }) {
  const architecture = summary.previewArchitecture

  return (
    <article className="surface-card operator-console__section">
      <div className="operator-console__section-header">
        <div>
          <p className="operator-console__section-label">Preview Architecture</p>
          <h2>프리뷰 아키텍처</h2>
        </div>
      </div>
      <dl className="operator-console__facts">
        <div className="operator-console__fact">
          <dt>Route</dt>
          <dd>{formatFieldValue(architecture.route)}</dd>
        </div>
        <div className="operator-console__fact">
          <dt>Route Stage</dt>
          <dd>{formatFieldValue(architecture.routeStage)}</dd>
        </div>
        <div className="operator-console__fact">
          <dt>Lane Owner</dt>
          <dd>{formatFieldValue(architecture.laneOwner)}</dd>
        </div>
        <div className="operator-console__fact">
          <dt>Fallback Reason</dt>
          <dd>{formatFieldValue(architecture.fallbackReasonCode)}</dd>
        </div>
        <div className="operator-console__fact">
          <dt>First Visible</dt>
          <dd>{formatMetricMs(architecture.firstVisibleMs)}</dd>
        </div>
        <div className="operator-console__fact">
          <dt>Preset Applied</dt>
          <dd>{formatMetricMs(architecture.replacementMs)}</dd>
        </div>
        <div className="operator-console__fact">
          <dt>Slot Replacement</dt>
          <dd>
            {formatMetricMs(
              architecture.originalVisibleToPresetAppliedVisibleMs,
            )}
          </dd>
        </div>
        <div className="operator-console__fact">
          <dt>Goal Speed</dt>
          <dd>{formatGoalSpeed(architecture.replacementMs)}</dd>
        </div>
        <div className="operator-console__fact">
          <dt>Warm State</dt>
          <dd>{formatFieldValue(architecture.warmState)}</dd>
        </div>
        <div className="operator-console__fact">
          <dt>Warm State Updated</dt>
          <dd>{formatFieldValue(architecture.warmStateObservedAt)}</dd>
        </div>
        <div className="operator-console__fact">
          <dt>Hardware Capability</dt>
          <dd>{formatFieldValue(architecture.hardwareCapability)}</dd>
        </div>
      </dl>
    </article>
  )
}

function formatBlockedCategory(category: OperatorRecoverySummary['blockedCategory']) {
  switch (category) {
    case 'capture':
      return 'Capture'
    case 'preview-or-render':
      return 'Preview / Render'
    case 'timing-or-post-end':
      return 'Timing / Post-End'
    default:
      return 'No Recovery'
  }
}

function formatBlockedStateCategoryLabel(
  category: OperatorBlockedStateCategory | null,
  hasUnavailableSummary: boolean,
) {
  if (hasUnavailableSummary) {
    return '진단 재시도 필요'
  }

  switch (category) {
    case 'capture-blocked':
      return 'Capture 확인 필요'
    case 'preview-render-blocked':
      return 'Preview / Render 확인 필요'
    case 'timing-post-end-blocked':
      return '종료 후 확인 필요'
    case 'not-blocked':
      return '정상'
    default:
      return '불러오는 중'
  }
}

function describeNextStateReason(reasonCode: string) {
  switch (reasonCode) {
    case 'preset-missing':
      return '촬영 전에 preset 선택이 먼저 필요해요.'
    case 'camera-preparing':
      return '카메라 준비가 아직 끝나지 않았어요.'
    case 'helper-preparing':
      return '보조 준비 단계가 아직 끝나지 않았어요.'
    case 'preview-waiting':
      return '최근 촬영본의 결과를 준비하는 중이에요.'
    case 'export-waiting':
      return '종료 후 결과와 안내를 정리하는 중이에요.'
    case 'completed':
      return '완료 안내가 확정된 상태예요.'
    case 'phone-required':
      return '고객 보호를 위해 직원 확인이 필요한 상태예요.'
    case 'warning':
      return '종료 전 경고 구간에 들어가 있어요.'
    case 'ended':
      return '세션 시간이 끝나 종료 후 판정을 기다리고 있어요.'
    case 'ready':
      return '바로 다음 촬영이나 안내로 이어질 수 있어요.'
    case 'blocked':
      return '추가 안내가 필요해 잠시 차단된 상태예요.'
    case 'session-missing':
    default:
      return '현재 세션 상태를 다시 확인하고 있어요.'
  }
}

function describeRejectionReason(rejectionReason: string) {
  switch (rejectionReason) {
    case 'not-blocked':
      return '지금은 복구가 필요한 막힘이 없어요.'
    case 'action-not-allowed':
      return '현재 범주에서는 허용되지 않은 액션이에요.'
    case 'session-mismatch':
      return '현재 화면과 다른 세션 응답이라 실행하지 않았어요.'
    case 'recovery-unavailable':
      return '지금은 안전하게 이어서 복구할 수 없어요.'
    case 'extension-limit-reached':
      return '허용된 시간 연장 한도에 도달했어요.'
    default:
      return '현재 복구 결과를 다시 확인해 주세요.'
  }
}

function formatActionResultStatus(status: 'applied' | 'rejected') {
  return status === 'applied' ? '적용됨' : '거절됨'
}

function describeBlockedCategory(category: OperatorRecoverySummary['blockedCategory']) {
  switch (category) {
    case 'capture':
      return 'capture 경계만 다시 열 수 있는 제한된 복구 액션만 보여 줘요.'
    case 'preview-or-render':
      return 'preview/render 경계에 허용된 retry 또는 boundary restart만 실행할 수 있어요.'
    case 'timing-or-post-end':
      return '종료 후 판정 재시도, 승인된 시간 연장, Phone Required 보호 전환만 허용돼요.'
    default:
      return '지금은 policy 기반 recovery action이 필요하지 않아요.'
  }
}

function describeAction(action: OperatorRecoveryAction) {
  switch (action) {
    case 'retry':
      return {
        label: 'Retry',
        detail: '현재 막힌 경계를 host 진실 위에서 다시 시도해 다음 상태를 재평가해요.',
      }
    case 'approved-boundary-restart':
      return {
        label: 'Approved Boundary Restart',
        detail: '허용된 경계만 bounded restart로 다시 열고 unsafe restart는 막아요.',
      }
    case 'approved-time-extension':
      return {
        label: 'Approved Time Extension',
        detail: 'Session Timing Policy 안에서 한 번의 승인된 시간 연장만 적용해요.',
      }
    case 'route-phone-required':
      return {
        label: 'Route To Phone Required',
        detail: '안전 복구를 더 이어가기 어렵다면 고객 흐름을 보호 상태로 전환해요.',
      }
    default:
      return {
        label: action,
        detail: '',
      }
  }
}

function formatAuditCategoryLabel(entry: OperatorAuditEntry) {
  switch (entry.eventCategory) {
    case 'critical-failure':
      return 'Critical Failure'
    case 'operator-intervention':
      return 'Operator Intervention'
    case 'post-end-outcome':
      return 'Post-End Outcome'
    case 'timing-transition':
      return 'Timing Transition'
    case 'publication-recovery':
      return 'Publication Recovery'
    case 'session-lifecycle':
    default:
      return 'Session Lifecycle'
  }
}

function AuditSummaryCard({ history }: { history: OperatorAuditQueryResult }) {
  const metrics = [
    ['Critical Failure', history.summary.criticalFailureEvents],
    ['Operator Intervention', history.summary.operatorInterventionEvents],
    ['Post-End Outcome', history.summary.postEndOutcomeEvents],
  ] as const

  return (
    <article className="surface-card operator-console__section">
      <div className="operator-console__section-header">
        <div>
          <p className="operator-console__section-label">Audit Summary</p>
          <h2>세션 감사 기록</h2>
        </div>
      </div>
      <div className="operator-console__facts">
        {metrics.map(([label, count]) => (
          <div key={label} className="operator-console__fact">
            <dt>{label}</dt>
            <dd>{`${label} ${count}건`}</dd>
          </div>
        ))}
      </div>
      {history.summary.latestOutcome === null ? null : (
        <div className="operator-console__callout">
          <p className="operator-console__callout-title">Latest Outcome</p>
          <p>
            {formatAuditCategoryLabel({
              ...history.summary.latestOutcome,
              schemaVersion: 'operator-audit-entry/v1',
              sessionId: null,
              detail: '',
              actorId: null,
              source: 'operator-console',
            } as OperatorAuditEntry)}
          </p>
          <p className="operator-console__meta">
            Latest Outcome: {history.summary.latestOutcome.occurredAt}
          </p>
        </div>
      )}
    </article>
  )
}

function AuditHistoryPanel({
  history,
  error,
}: {
  history: OperatorAuditQueryResult | null
  error: HostErrorEnvelope | null
}) {
  if (error !== null) {
    return (
      <article className="surface-card operator-console__section">
        <div className="operator-console__section-header">
          <div>
            <p className="operator-console__section-label">Audit History</p>
            <h2>감사 기록을 불러오지 못했어요</h2>
          </div>
        </div>
        <p className="operator-console__empty">
          세션 진단과 허용 액션은 계속 확인할 수 있어요. 잠시 후 다시 새로고침해 주세요.
        </p>
      </article>
    )
  }

  if (history === null) {
    return null
  }

  return (
    <>
      <AuditSummaryCard history={history} />
      <article className="surface-card operator-console__section">
        <div className="operator-console__section-header">
          <div>
            <p className="operator-console__section-label">Recent Audit</p>
            <h2>최근 감사 이력</h2>
          </div>
        </div>
        {history.events.length === 0 ? (
          <p className="operator-console__empty">
            아직 표시할 감사 이력이 없어요.
          </p>
        ) : (
          <div className="operator-console__action-grid">
            {history.events.map((entry) => (
              <article key={entry.eventId} className="operator-console__action-card">
                <div className="operator-console__action-copy">
                  <p className="operator-console__callout-title">{entry.summary}</p>
                  <p>{entry.detail}</p>
                  <p className="operator-console__meta">
                    {formatAuditCategoryLabel(entry)} · {entry.occurredAt}
                  </p>
                </div>
              </article>
            ))}
          </div>
        )}
      </article>
    </>
  )
}

function BoundaryCard({
  label,
  boundary,
}: {
  label: string
  boundary: OperatorBoundarySummary
}) {
  return (
    <article
      className={`surface-card operator-boundary-card operator-boundary-card--${boundary.status}`}
    >
      <div className="operator-boundary-card__header">
        <div>
          <p className="operator-boundary-card__label">{label}</p>
          <h2>{boundary.title}</h2>
        </div>
        <p className="operator-boundary-card__badge">{formatBoundaryStatus(boundary)}</p>
      </div>
      <p>{boundary.detail}</p>
    </article>
  )
}

function CameraConnectionCard({
  cameraConnection,
}: {
  cameraConnection: OperatorCameraConnectionSummary
}) {
  return (
    <article className="surface-card operator-console__camera-card">
      <div className="operator-console__section-header">
        <div>
          <p className="operator-console__section-label">Camera Connection</p>
          <h2>카메라 연결 상태</h2>
        </div>
        <p
          className={`operator-console__camera-badge operator-console__camera-badge--${cameraConnection.state}`}
        >
          {formatCameraConnectionStateLabel(cameraConnection.state)}
        </p>
      </div>
      <div className="operator-console__callout">
        <p className="operator-console__callout-title">{cameraConnection.title}</p>
        <p>{cameraConnection.detail}</p>
        {cameraConnection.observedAt === null ||
        cameraConnection.observedAt === undefined ? null : (
          <p className="operator-console__meta">
            Observed At: {formatFieldValue(cameraConnection.observedAt)}
          </p>
        )}
      </div>
    </article>
  )
}

function SessionFacts({ summary }: { summary: OperatorRecoverySummary }) {
  return (
    <article className="surface-card operator-console__section">
      <div className="operator-console__section-header">
        <div>
          <p className="operator-console__section-label">Session Context</p>
          <h2>현재 세션 문맥</h2>
        </div>
      </div>
      <dl className="operator-console__facts">
        <div className="operator-console__fact">
          <dt>Session ID</dt>
          <dd>{formatFieldValue(summary.sessionId)}</dd>
        </div>
        <div className="operator-console__fact">
          <dt>Booth Alias</dt>
          <dd>{formatFieldValue(summary.boothAlias)}</dd>
        </div>
        <div className="operator-console__fact">
          <dt>Preset</dt>
          <dd>{formatFieldValue(summary.activePresetDisplayName)}</dd>
        </div>
        <div className="operator-console__fact">
          <dt>Preset Version</dt>
          <dd>{formatFieldValue(summary.activePresetVersion)}</dd>
        </div>
        <div className="operator-console__fact">
          <dt>Lifecycle</dt>
          <dd>{formatLifecycleStage(summary.lifecycleStage)}</dd>
        </div>
        <div className="operator-console__fact">
          <dt>Timing</dt>
          <dd>{formatTimingPhase(summary.timingPhase)}</dd>
        </div>
        <div className="operator-console__fact">
          <dt>Post-End</dt>
          <dd>{formatPostEndState(summary.postEndState)}</dd>
        </div>
        <div className="operator-console__fact">
          <dt>Updated At</dt>
          <dd>{formatFieldValue(summary.updatedAt)}</dd>
        </div>
      </dl>
    </article>
  )
}

function RecentFailureCard({ summary }: { summary: OperatorRecoverySummary }) {
  return (
    <article className="surface-card operator-console__section">
      <div className="operator-console__section-header">
        <div>
          <p className="operator-console__section-label">Recent Failure</p>
          <h2>최근 장애 문맥</h2>
        </div>
      </div>
      {summary.diagnosticsSummary === null ? (
        <p className="operator-console__empty">
          지금은 추가로 정규화된 장애 문맥이 없어요. 아래 경계별 상태를 기준으로 확인해
          주세요.
        </p>
      ) : (
        <div className="operator-console__callout">
          <p className="operator-console__callout-title">
            {summary.diagnosticsSummary.title}
          </p>
          <p>{summary.diagnosticsSummary.detail}</p>
          {summary.diagnosticsSummary.observedAt === null ||
          summary.diagnosticsSummary.observedAt === undefined ? null : (
            <p className="operator-console__meta">
              Observed At: {formatFieldValue(summary.diagnosticsSummary.observedAt)}
            </p>
          )}
        </div>
      )}
    </article>
  )
}

function ActionPanel({
  summary,
  isActing,
  actionsDisabled,
  onAction,
}: {
  summary: OperatorRecoverySummary
  isActing: boolean
  actionsDisabled: boolean
  onAction(action: OperatorRecoveryAction): void
}) {
  return (
    <article className="surface-card operator-console__section">
      <div className="operator-console__section-header">
        <div>
          <p className="operator-console__section-label">Recovery Policy</p>
          <h2>허용 액션</h2>
        </div>
        <p className="operator-console__policy-badge">
          {formatBlockedCategory(summary.blockedCategory)}
        </p>
      </div>
      <p className="operator-console__empty">
        {describeBlockedCategory(summary.blockedCategory)}
      </p>
      {actionsDisabled ? (
        <p className="operator-console__empty">
          최신 세션 진단을 다시 확인할 때까지 recovery action은 잠시 멈춰 둘게요.
        </p>
      ) : null}
      {summary.allowedActions.length === 0 ? (
        <p className="operator-console__empty">
          현재 세션 범주에서는 추가 recovery action을 열지 않았어요.
        </p>
      ) : (
        <div className="operator-console__action-grid">
          {summary.allowedActions.map((action) => {
            const copy = describeAction(action)

            return (
              <article key={action} className="operator-console__action-card">
                <div className="operator-console__action-copy">
                  <p className="operator-console__callout-title">{copy.label}</p>
                  <p>{copy.detail}</p>
                </div>
                <button
                  type="button"
                  className="surface-card__action"
                  disabled={isActing || actionsDisabled}
                  onClick={() => onAction(action)}
                >
                  {isActing ? '실행 중...' : copy.label}
                </button>
              </article>
            )
          })}
        </div>
      )}
    </article>
  )
}

function ActionResultCard() {
  const { lastActionResult } = useOperatorDiagnostics()

  if (lastActionResult === null) {
    return null
  }

  return (
    <article className="surface-card operator-console__section">
      <div className="operator-console__section-header">
        <div>
          <p className="operator-console__section-label">Latest Recovery Result</p>
          <h2>최근 recovery 결과</h2>
        </div>
        <p
          className={`operator-console__result-badge operator-console__result-badge--${lastActionResult.status}`}
        >
          {formatActionResultStatus(lastActionResult.status)}
        </p>
      </div>
      <div className="operator-console__callout">
        <p className="operator-console__callout-title">{lastActionResult.message}</p>
        <p>다음 booth 상태: {lastActionResult.nextState.customerState}</p>
        <p className="operator-console__meta">
          상태 이유: {describeNextStateReason(lastActionResult.nextState.reasonCode)}
        </p>
        <p className="operator-console__meta">
          Lifecycle: {formatLifecycleStage(lastActionResult.nextState.lifecycleStage)}
        </p>
        {lastActionResult.rejectionReason === null ||
        lastActionResult.rejectionReason === undefined ? null : (
          <p className="operator-console__meta">
            거절 사유: {describeRejectionReason(lastActionResult.rejectionReason)}
          </p>
        )}
      </div>
    </article>
  )
}

export function OperatorSummaryScreen() {
  const capabilityService = useCapabilityService()
  const {
    summary,
    auditHistory,
    error,
    auditError,
    isLoading,
    isActing,
    refresh,
    runAction,
  } = useOperatorDiagnostics()
  const hasUnavailableSummary = summary === null && error !== null
  const actionsDisabled = isLoading
  const activeCategory = summary?.blockedStateCategory ?? null
  const categoryBadgeTone = activeCategory ?? 'loading'
  const categoryBadgeLabel = formatBlockedStateCategoryLabel(
    activeCategory,
    hasUnavailableSummary,
  )
  const canOpenSettings = capabilityService.canAccess('settings')

  return (
    <SurfaceLayout
      eyebrow="Operator"
      title="Operator Console"
      description="현재 세션 문맥과 허용된 recovery action을 정책 범위 안에서만 보여줍니다."
    >
      <section className="operator-console">
        <article className="surface-card operator-console__hero">
          <div className="operator-console__hero-header">
            <div>
              <p className="operator-console__section-label">Current Session</p>
              <h2>{describeSummaryHeading(summary, hasUnavailableSummary)}</h2>
            </div>
            <div className="operator-console__hero-actions">
              {canOpenSettings ? (
                <Link
                  className="surface-card__action surface-card__action--secondary"
                  to="/settings"
                >
                  운영 설정
                </Link>
              ) : null}
              <button
                type="button"
                className="surface-card__action surface-card__action--secondary"
                onClick={refresh}
              >
                {isLoading ? '새로고침 중...' : '진단 새로고침'}
              </button>
            </div>
          </div>

          <div className="operator-console__hero-status">
            <p
              className={`operator-console__category-badge operator-console__category-badge--${categoryBadgeTone}`}
            >
              {categoryBadgeLabel}
            </p>
            <p className="operator-console__support">
              {describeBlockedStateCategory(activeCategory, hasUnavailableSummary)}
            </p>
          </div>

          {error === null ? null : (
            <p className="operator-console__error">{error.message}</p>
          )}
        </article>

        {summary === null ? (
          <article className="surface-card operator-console__section">
            {error === null ? (
              <>
                <h2>세션 요약을 준비하고 있어요</h2>
                <p className="operator-console__empty">
                  host가 현재 세션 문맥과 boundary 진단을 정리하는 동안 잠시 기다려
                  주세요.
                </p>
              </>
            ) : (
              <>
                <h2>현재 세션 진단을 다시 불러와 주세요</h2>
                <p className="operator-console__empty">
                  최신 세션 문맥을 읽지 못해 이전 요약은 화면에서 내렸어요. 연결 상태를
                  확인한 뒤 다시 새로고침해 주세요.
                </p>
              </>
            )}
          </article>
        ) : (
          <>
            {summary.state === 'session-loaded' ? (
              <>
                <CameraConnectionCard cameraConnection={summary.cameraConnection} />
                <SessionFacts summary={summary} />
                <PreviewArchitectureCard summary={summary} />
                <RecentFailureCard summary={summary} />
                <ActionPanel
                  summary={summary}
                  isActing={isActing}
                  actionsDisabled={actionsDisabled}
                  onAction={runAction}
                />
                <AuditHistoryPanel history={auditHistory} error={auditError} />
                <ActionResultCard />
              </>
            ) : (
              <article className="surface-card operator-console__section">
                <div className="operator-console__section-header">
                  <div>
                    <p className="operator-console__section-label">Empty State</p>
                    <h2>진행 중인 세션이 아직 없어요</h2>
                  </div>
                </div>
                <p className="operator-console__empty">
                  새 세션이 시작되면 booth alias, lifecycle, blocked-state category, 최근
                  진단 문맥, 허용 recovery action을 여기에서 바로 확인할 수 있어요.
                </p>
              </article>
            )}

            {summary.state === 'no-session' ? (
              <CameraConnectionCard cameraConnection={summary.cameraConnection} />
            ) : null}

            <section className="operator-console__boundary-grid">
              <BoundaryCard
                label="Capture Boundary"
                boundary={summary.captureBoundary}
              />
              <BoundaryCard
                label="Preview / Render Boundary"
                boundary={summary.previewRenderBoundary}
              />
              <BoundaryCard
                label="Completion Boundary"
                boundary={summary.completionBoundary}
              />
            </section>
          </>
        )}
      </section>
    </SurfaceLayout>
  )
}
