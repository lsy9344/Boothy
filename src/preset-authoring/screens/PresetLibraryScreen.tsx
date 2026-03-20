import { SurfaceLayout } from '../../shared-ui/layout/SurfaceLayout'

export function PresetLibraryScreen() {
  return (
    <SurfaceLayout
      eyebrow="Authoring"
      title="Preset Authoring"
      description="This placeholder reserves the internal preset workspace without leaking editing controls into the customer booth experience."
    >
      <article className="surface-card">
        <h2>Authoring placeholder</h2>
        <p>Approved preset creation, preview, and publication workflows will land here in later stories.</p>
      </article>
    </SurfaceLayout>
  )
}
