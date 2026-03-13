import { HardFramePanel } from '../../shared-ui/components/HardFramePanel.js'
import { PrimaryActionButton } from '../../shared-ui/components/PrimaryActionButton.js'
import { customerStartCopy } from '../copy/customerStartCopy.js'

type SessionStartHandoffScreenProps = {
  isContinuing?: boolean
  onContinue?: () => void | Promise<void>
  sessionName: string
}

export function SessionStartHandoffScreen({
  isContinuing = false,
  onContinue,
  sessionName,
}: SessionStartHandoffScreenProps) {
  return (
    <main aria-busy={isContinuing} className="customer-shell">
      <HardFramePanel className="customer-shell__panel customer-preparation">
        <p className="customer-shell__eyebrow">{customerStartCopy.eyebrow}</p>

        <section className="customer-shell__content customer-preparation__content">
          <p className="customer-preparation__badge">{customerStartCopy.handoffBadge}</p>

          <div className="customer-preparation__status" role="status">
            <h1 className="customer-shell__title">{customerStartCopy.handoffTitle}</h1>
            <p className="customer-shell__supporting">{customerStartCopy.handoffSupporting}</p>
          </div>
        </section>

        <div className="customer-shell__actions customer-preparation__actions">
          <p className="customer-preparation__hint">{customerStartCopy.handoffHint}</p>
          <p aria-label={customerStartCopy.handoffSessionNameLabel} className="customer-preparation__session-name">
            {sessionName.trim()}
          </p>
          {onContinue ? (
            <PrimaryActionButton
              disabled={isContinuing}
              label={customerStartCopy.handoffPrimaryAction}
              onClick={() => {
                void onContinue()
              }}
              type="button"
            />
          ) : null}
        </div>
      </HardFramePanel>
    </main>
  )
}
