import { render, screen } from '@testing-library/react'
import { RouterProvider, createMemoryRouter } from 'react-router-dom'
import { describe, expect, it } from 'vitest'

import { createCapabilityService } from './services/capability-service'
import { createAppRoutes } from './routes'

function renderRoute(
  initialEntries: string[],
  capabilityService = createCapabilityService(),
) {
  const router = createMemoryRouter(createAppRoutes({ capabilityService }), {
    initialEntries,
  })

  render(<RouterProvider router={router} />)

  return router
}

describe('app routing baseline', () => {
  it('connects the default entry point to the booth surface', async () => {
    const router = renderRoute(['/'])

    expect(
      await screen.findByRole('heading', { name: /이름을 확인할게요/i }),
    ).toBeInTheDocument()
    expect(router.state.location.pathname).toBe('/booth')
  })

  it('does not expose operator, authoring, or settings entry points in the booth UI', async () => {
    renderRoute(['/booth'])

    await screen.findByRole('heading', { name: /이름을 확인할게요/i })

    expect(screen.queryByRole('link', { name: /operator/i })).not.toBeInTheDocument()
    expect(screen.queryByRole('link', { name: /authoring/i })).not.toBeInTheDocument()
    expect(screen.queryByRole('link', { name: /settings/i })).not.toBeInTheDocument()
  })

  it('blocks direct navigation to restricted surfaces by default', async () => {
    const router = renderRoute(['/operator'])

    expect(
      await screen.findByRole('heading', { name: /이름을 확인할게요/i }),
    ).toBeInTheDocument()
    expect(screen.queryByRole('heading', { name: /Operator Console/i })).not.toBeInTheDocument()
    expect(router.state.location.pathname).toBe('/booth')
  })

  it('keeps a typed capability seam for future admin-authenticated flows', async () => {
    renderRoute(
      ['/operator'],
      createCapabilityService({
        isAdminAuthenticated: true,
        allowedSurfaces: ['booth', 'operator'],
      }),
    )

    expect(
      await screen.findByRole('heading', { name: /Operator Console/i }),
    ).toBeInTheDocument()
  })
})
