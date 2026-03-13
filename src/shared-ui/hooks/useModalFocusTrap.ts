import { useLayoutEffect, useRef } from 'react'

function getFocusableElements(container: HTMLElement): HTMLElement[] {
  return Array.from(
    container.querySelectorAll<HTMLElement>(
      'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
    ),
  )
}

export function useModalFocusTrap(onEscape: () => void) {
  const containerRef = useRef<HTMLDivElement | null>(null)

  useLayoutEffect(() => {
    const container = containerRef.current

    if (!container) {
      return undefined
    }

    let returnFocusTarget =
      document.activeElement instanceof HTMLElement ? document.activeElement : null

    const focusFirstElement = () => {
      getFocusableElements(container)[0]?.focus()
    }

    queueMicrotask(focusFirstElement)

    let allowOutsideFocus = false

    const handleFocusIn = (event: FocusEvent) => {
      if (allowOutsideFocus) {
        return
      }

      if (!(event.target instanceof Node) || container.contains(event.target)) {
        return
      }

      if (event.target instanceof HTMLElement) {
        returnFocusTarget = event.target
      }

      focusFirstElement()
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault()
        allowOutsideFocus = true
        onEscape()
        returnFocusTarget?.focus()
        return
      }

      if (event.key !== 'Tab') {
        return
      }

      const focusableElements = getFocusableElements(container)

      if (focusableElements.length === 0) {
        return
      }

      const firstElement = focusableElements[0]
      const lastElement = focusableElements[focusableElements.length - 1]
      const activeElement = document.activeElement

      if (event.shiftKey) {
        if (activeElement === firstElement || !container.contains(activeElement)) {
          event.preventDefault()
          lastElement.focus()
        }

        return
      }

      if (activeElement === lastElement || !container.contains(activeElement)) {
        event.preventDefault()
        firstElement.focus()
      }
    }

    document.addEventListener('focusin', handleFocusIn)
    document.addEventListener('keydown', handleKeyDown)

    return () => {
      document.removeEventListener('focusin', handleFocusIn)
      document.removeEventListener('keydown', handleKeyDown)
      returnFocusTarget?.focus()
    }
  }, [onEscape])

  return containerRef
}
