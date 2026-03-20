import { SurfaceLayout } from '../../shared-ui/layout/SurfaceLayout'

export function OperatorSummaryScreen() {
  return (
    <SurfaceLayout
      eyebrow="Operator"
      title="Operator Console"
      description="This placeholder is reachable only through the typed capability seam and will host recovery and diagnostics workflows in later stories."
    >
      <article className="surface-card">
        <h2>Console placeholder</h2>
        <p>Future stories will attach operator diagnostics, recovery actions, and audit-aware tooling here.</p>
      </article>
    </SurfaceLayout>
  )
}
