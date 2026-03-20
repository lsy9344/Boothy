import type { ReactNode } from 'react'

type SurfaceLayoutProps = {
  eyebrow: string
  title: string
  description: string
  children: ReactNode
}

export function SurfaceLayout({
  eyebrow,
  title,
  description,
  children,
}: SurfaceLayoutProps) {
  return (
    <main className="surface-layout">
      <section className="surface-layout__panel">
        <p className="surface-layout__eyebrow">{eyebrow}</p>
        <h1 className="surface-layout__title">{title}</h1>
        <p className="surface-layout__description">{description}</p>
        <div className="surface-layout__content">{children}</div>
      </section>
    </main>
  )
}
