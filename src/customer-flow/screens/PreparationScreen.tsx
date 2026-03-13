import { PrimaryActionButton } from '../../shared-ui/components/PrimaryActionButton.js'
import type { CustomerPreparationState } from '../../session-domain/state/customerPreparationState.js'
import { HardFramePanel } from '../../shared-ui/components/HardFramePanel.js'
import type { SessionTimeDisplay } from '../../timing-policy/selectors/sessionTimeDisplay.js'
import { SessionTimeBanner } from '../components/SessionTimeBanner.js'
import { preparationCopy } from '../copy/preparationCopy.js'
import { selectCustomerCameraStatusCopy } from '../selectors/customerCameraStatusCopy.js'

type PreparationScreenProps = {
  sessionName: string
  readiness: CustomerPreparationState
  sessionTimeDisplay?: SessionTimeDisplay | null
  onStartCapture?(): void
  showPrimaryAction?: boolean
  statusOverride?: {
    badge: string
    title: string
    supporting: string
    actionHint: string
  }
}

export function PreparationScreen({
  onStartCapture,
  readiness,
  sessionName,
  sessionTimeDisplay,
  showPrimaryAction = true,
  statusOverride,
}: PreparationScreenProps) {
  const copy = statusOverride ?? selectCustomerCameraStatusCopy(readiness)
  const actionHintId = 'preparation-action-hint'
  const phoneLabel = 'phoneLabel' in copy && typeof copy.phoneLabel === 'string' ? copy.phoneLabel : null
  const phoneNumber = 'phoneNumber' in copy && typeof copy.phoneNumber === 'string' ? copy.phoneNumber : null

  return (
    <main
      aria-busy={statusOverride !== undefined || ('kind' in copy && copy.kind === 'preparing')}
      className="customer-shell"
    >
      <HardFramePanel className="customer-shell__panel customer-preparation">
        <p className="customer-shell__eyebrow">{preparationCopy.eyebrow}</p>

        <section className="customer-shell__content customer-preparation__content">
          <p className="customer-preparation__badge">{copy.badge}</p>

          <div aria-live="polite" className="customer-preparation__status" role="status">
            <h1 className="customer-shell__title">{copy.title}</h1>
            <p className="customer-shell__supporting">{copy.supporting}</p>
          </div>

          {sessionTimeDisplay ? <SessionTimeBanner {...sessionTimeDisplay} /> : null}

          {phoneLabel && phoneNumber ? (
            <dl className="customer-preparation__phone-card">
              <dt>{phoneLabel}</dt>
              <dd>{phoneNumber}</dd>
            </dl>
          ) : null}
        </section>

        <div className="customer-shell__actions customer-preparation__actions">
          {showPrimaryAction && (!('kind' in copy) || copy.kind !== 'phone-required') ? (
            <PrimaryActionButton
              describedBy={actionHintId}
              disabled={!readiness.captureEnabled}
              label="촬영 시작"
              onClick={onStartCapture}
            />
          ) : null}
          <p className="customer-preparation__hint" id={actionHintId}>
            {copy.actionHint}
          </p>
          <p aria-label="세션 이름" className="customer-preparation__session-name">
            {sessionName}
          </p>
        </div>
      </HardFramePanel>
    </main>
  )
}
