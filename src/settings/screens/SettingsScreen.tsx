import { SurfaceLayout } from '../../shared-ui/layout/SurfaceLayout'

export function SettingsScreen() {
  return (
    <SurfaceLayout
      eyebrow="Settings"
      title="Settings Surface"
      description="Settings are present as a top-level surface but remain hidden behind the same restricted capability boundary as other internal tools."
    >
      <article className="surface-card">
        <h2>Restricted placeholder</h2>
        <p>Branch-local configuration and runtime flags can be connected here once the admin flow exists.</p>
      </article>
    </SurfaceLayout>
  )
}
