import { DEFAULT_CAPABILITY_SNAPSHOT } from './shared-contracts'
import { bootstrapApp } from './bootstrap'

describe('bootstrapApp', () => {
  it('still renders the app when reading the current window label fails', async () => {
    const createCapabilityService = vi.fn((options) => ({
      kind: 'capability-service',
      options,
    }))
    const createAppRouter = vi.fn(({ capabilityService }) => ({
      kind: 'router',
      capabilityService,
    }))
    const renderApp = vi.fn()

    await expect(
      bootstrapApp({
        readCurrentWindowLabel: async () => {
          throw new Error('window-label-unavailable')
        },
        readCapabilitySnapshot: async () => DEFAULT_CAPABILITY_SNAPSHOT,
        createCapabilityService,
        createAppRouter,
        renderApp,
      }),
    ).resolves.toMatchObject({
      currentWindowLabel: null,
    })

    expect(createCapabilityService).toHaveBeenCalledWith({
      ...DEFAULT_CAPABILITY_SNAPSHOT,
      currentWindowLabel: null,
    })
    expect(createAppRouter).toHaveBeenCalledTimes(1)
    expect(renderApp).toHaveBeenCalledWith({
      kind: 'router',
      capabilityService: {
        kind: 'capability-service',
        options: {
          ...DEFAULT_CAPABILITY_SNAPSHOT,
          currentWindowLabel: null,
        },
      },
    })
  })

  it('falls back to authoring capabilities when the Tauri snapshot stalls', async () => {
    vi.useFakeTimers()

    const createCapabilityService = vi.fn((options) => ({
      kind: 'capability-service',
      options,
    }))
    const createAppRouter = vi.fn(({ capabilityService }) => ({
      kind: 'router',
      capabilityService,
    }))
    const renderApp = vi.fn()

    const bootstrapPromise = bootstrapApp({
      readCurrentWindowLabel: async () => 'authoring-window',
      readCapabilitySnapshot: () => new Promise(() => {}),
      createCapabilityService,
      createAppRouter,
      renderApp,
      bootstrapTimeoutMs: 25,
    })

    await vi.advanceTimersByTimeAsync(25)

    await expect(bootstrapPromise).resolves.toMatchObject({
      currentWindowLabel: 'authoring-window',
    })

    expect(createCapabilityService).toHaveBeenLastCalledWith({
      currentWindowLabel: 'authoring-window',
      isAdminAuthenticated: true,
      allowedSurfaces: ['booth', 'operator', 'authoring', 'settings'],
    })
    expect(renderApp).toHaveBeenCalledTimes(1)

    vi.useRealTimers()
  })
})
