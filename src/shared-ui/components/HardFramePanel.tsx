import type { HTMLAttributes, ReactNode } from 'react'

type HardFramePanelProps = {
  children: ReactNode
  className?: string
} & HTMLAttributes<HTMLElement>

export function HardFramePanel({ children, className = '', ...rest }: HardFramePanelProps) {
  const classes = ['surface-frame', className].filter(Boolean).join(' ')

  return (
    <section className={classes} {...rest}>
      {children}
    </section>
  )
}
