import { HardFramePanel } from '../../shared-ui/components/HardFramePanel.js'
import type { PostEndView } from '../selectors/postEndView.js'

type PostEndScreenProps = {
  view: PostEndView
}

export function PostEndScreen({ view }: PostEndScreenProps) {
  return (
    <main className="customer-shell">
      <HardFramePanel className="customer-shell__panel customer-capture">
        <header className="customer-capture__header">
          <p className="customer-shell__eyebrow">{view.badge}</p>
        </header>

        <section className="customer-shell__content">
          <h1 className="customer-shell__title">{view.title}</h1>
          <p className="customer-shell__supporting">{view.supporting}</p>
        </section>

        {view.sessionName ? (
          <section aria-label="세션 이름" className="capture-signal-card">
            <p className="capture-signal-card__label">세션 이름</p>
            <p className="capture-signal-card__value">{view.sessionName}</p>
          </section>
        ) : null}

        {view.handoffTargetLabel ? (
          <section aria-label="다음 안내 위치" className="capture-signal-card">
            <p className="capture-signal-card__label">다음 안내 위치</p>
            <p className="capture-signal-card__value">{view.handoffTargetLabel}</p>
          </section>
        ) : null}

        {view.phoneNumber ? (
          <section aria-label="매장 연락처" className="capture-signal-card">
            <p className="capture-signal-card__label">매장 연락처</p>
            <p className="capture-signal-card__value">{view.phoneNumber}</p>
          </section>
        ) : null}

        <section aria-label="다음 단계" className="capture-guidance-card">
          <p className="capture-signal-card__label">다음 단계</p>
          <p className="capture-guidance-card__value">{view.actionLabel}</p>
        </section>
      </HardFramePanel>
    </main>
  )
}
