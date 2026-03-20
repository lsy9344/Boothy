import { SurfaceLayout } from '../../shared-ui/layout/SurfaceLayout'
import { useSessionState } from '../../session-domain/state/use-session-state'
import { SessionStartForm } from '../components/SessionStartForm'
import { CaptureScreen } from './CaptureScreen'
import { PresetSelectScreen } from './PresetSelectScreen'
import { sessionStartCopy } from '../copy/sessionStartCopy'

export function SessionStartScreen() {
  const { sessionDraft } = useSessionState()

  if (
    sessionDraft.flowStep === 'capture' &&
    sessionDraft.sessionId &&
    sessionDraft.manifest
  ) {
    return <CaptureScreen />
  }

  if (
    sessionDraft.flowStep === 'preset-selection' &&
    sessionDraft.sessionId &&
    sessionDraft.boothAlias &&
    sessionDraft.manifest
  ) {
    return <PresetSelectScreen />
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
