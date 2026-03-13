import type { SessionTimeDisplay } from '../../timing-policy/selectors/sessionTimeDisplay.js'
import type { CaptureConfidenceView } from '../selectors/captureConfidenceView.js'

export function resolveCaptureView(
  view: CaptureConfidenceView,
  sessionTimeDisplay: SessionTimeDisplay | null,
): CaptureConfidenceView {
  if (!sessionTimeDisplay) {
    return view
  }

  return {
    ...view,
    endTime: {
      ...sessionTimeDisplay,
      ...(view.endTime.alertBadge ? { alertBadge: view.endTime.alertBadge } : {}),
      supporting: view.timingAlert.kind === 'none' ? sessionTimeDisplay.supporting : view.timingAlert.message,
    },
  }
}
