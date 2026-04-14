import { render, screen } from '@testing-library/react'

import { AppErrorBoundary } from './AppErrorBoundary'

function ThrowingScreen() {
  throw new Error('render-crashed')
}

describe('AppErrorBoundary', () => {
  it('shows a fallback surface when the app crashes during render', async () => {
    render(
      <AppErrorBoundary>
        <ThrowingScreen />
      </AppErrorBoundary>,
    )

    expect(
      await screen.findByRole('heading', { name: /앱을 다시 불러오는 중이에요/i }),
    ).toBeInTheDocument()
    expect(screen.getByText(/render-crashed/i)).toBeInTheDocument()
  })
})
