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

  it('still blocks restricted surfaces when capability flags exist without admin authentication', async () => {
    const router = renderRoute(
      ['/authoring'],
      createCapabilityService({
        isAdminAuthenticated: false,
        allowedSurfaces: ['booth', 'authoring'],
      }),
    )

    expect(
      await screen.findByRole('heading', { name: /이름을 확인할게요/i }),
    ).toBeInTheDocument()
    expect(
      screen.queryByRole('heading', { name: /Preset Authoring/i }),
    ).not.toBeInTheDocument()
    expect(router.state.location.pathname).toBe('/booth')
  })

  it('keeps a typed capability seam for future admin-authenticated flows', async () => {
    renderRoute(
      ['/operator'],
      createCapabilityService({
        isAdminAuthenticated: true,
        allowedSurfaces: ['booth', 'operator'],
        currentWindowLabel: 'operator-window',
      }),
    )

    expect(
      await screen.findByRole('heading', { name: /Operator Console/i }),
    ).toBeInTheDocument()
  })

  it('still blocks operator on the booth window even when the runtime snapshot includes operator capability', async () => {
    const router = renderRoute(
      ['/operator'],
      createCapabilityService({
        isAdminAuthenticated: true,
        allowedSurfaces: ['booth', 'operator'],
        currentWindowLabel: 'booth-window',
      }),
    )

    expect(
      await screen.findByRole('heading', { name: /이름을 확인할게요/i }),
    ).toBeInTheDocument()
    expect(screen.queryByRole('heading', { name: /Operator Console/i })).not.toBeInTheDocument()
    expect(router.state.location.pathname).toBe('/booth')
  })

  it('allows the authoring surface only after admin-authenticated capability access is granted', async () => {
    const router = renderRoute(
      ['/authoring'],
      createCapabilityService({
        isAdminAuthenticated: true,
        allowedSurfaces: ['booth', 'authoring'],
        currentWindowLabel: 'authoring-window',
      }),
    )

    expect(
      await screen.findByRole('heading', { name: /Draft Preset Workspace/i }),
    ).toBeInTheDocument()
    expect(router.state.location.pathname).toBe('/authoring')
  })

  it('allows the settings surface only after admin-authenticated capability access is granted', async () => {
    const router = renderRoute(
      ['/settings'],
      createCapabilityService({
        isAdminAuthenticated: true,
        allowedSurfaces: ['booth', 'settings'],
        currentWindowLabel: 'operator-window',
      }),
    )

    expect(
      await screen.findByRole('heading', { name: /Settings Governance/i }),
    ).toBeInTheDocument()
    expect(
      screen.getByRole('heading', { name: /Branch Rollout Governance/i }),
    ).toBeInTheDocument()
    expect(router.state.location.pathname).toBe('/settings')
  })

  it('still blocks authoring on the booth window even when the runtime snapshot includes authoring capability', async () => {
    const router = renderRoute(
      ['/authoring'],
      createCapabilityService({
        isAdminAuthenticated: true,
        allowedSurfaces: ['booth', 'authoring'],
        currentWindowLabel: 'booth-window',
      }),
    )

    expect(
      await screen.findByRole('heading', { name: /이름을 확인할게요/i }),
    ).toBeInTheDocument()
    expect(router.state.location.pathname).toBe('/booth')
  })
})
