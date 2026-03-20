import { SurfaceLayout } from '../../shared-ui/layout/SurfaceLayout'
import { useSessionState } from '../../session-domain/state/use-session-state'
import { SessionStartForm } from '../components/SessionStartForm'
import { sessionStartCopy } from '../copy/sessionStartCopy'

export function SessionStartScreen() {
  const { sessionDraft } = useSessionState()

  if (sessionDraft.flowStep === 'preset-selection' && sessionDraft.boothAlias) {
    return (
      <SurfaceLayout
        eyebrow={sessionStartCopy.eyebrow}
        title={sessionStartCopy.successTitle}
        description={sessionStartCopy.successDescription}
      >
        <article className="surface-card">
          <h2>{sessionStartCopy.aliasLabel}</h2>
          <p>{sessionDraft.boothAlias}</p>
        </article>
      </SurfaceLayout>
    )
  }

  return (
    <SurfaceLayout
      eyebrow={sessionStartCopy.eyebrow}
      title={sessionStartCopy.title}
      description={sessionStartCopy.description}
    >
      <SessionStartForm />
    </SurfaceLayout>
  )
}
