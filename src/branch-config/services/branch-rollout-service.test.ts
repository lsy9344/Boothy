import { invoke } from '@tauri-apps/api/core'
import { describe, expect, it, vi } from 'vitest'

import { createTauriBranchRolloutGateway } from './branch-rollout-service'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

describe('branch rollout gateway', () => {
  it('maps the typed rollout payload to the Tauri host DTO shape', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(null)
    const gateway = createTauriBranchRolloutGateway()

    await gateway.applyRollout({
      branchIds: ['gangnam-01'],
      targetBaseline: {
        buildVersion: 'boothy-2026.03.27.1',
        presetStackVersion: 'catalog-2026.03.27',
        approvedAt: '2026-03-27T00:10:00.000Z',
        actorId: 'release-kim',
        actorLabel: 'Kim Release',
      },
      actorId: 'release-kim',
      actorLabel: 'Kim Release',
    })

    expect(invoke).toHaveBeenCalledWith('apply_branch_rollout', {
      input: {
        branchIds: ['gangnam-01'],
        targetBuildVersion: 'boothy-2026.03.27.1',
        targetPresetStackVersion: 'catalog-2026.03.27',
        actorId: 'release-kim',
        actorLabel: 'Kim Release',
      },
    })
  })

  it('maps the preview route promotion payload to the Tauri host DTO shape', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(null)
    const gateway = createTauriBranchRolloutGateway()

    await gateway.promotePreviewRendererRoute({
      presetId: 'preset_new-draft-2',
      publishedVersion: '2026.04.10',
      targetRouteStage: 'canary',
      actorId: 'release-kim',
      actorLabel: 'Kim Release',
    })

    expect(invoke).toHaveBeenCalledWith('promote_preview_renderer_route', {
      input: {
        presetId: 'preset_new-draft-2',
        publishedVersion: '2026.04.10',
        targetRouteStage: 'canary',
        actorId: 'release-kim',
        actorLabel: 'Kim Release',
      },
    })
  })

  it('maps the preview route rollback payload to the Tauri host DTO shape', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(null)
    const gateway = createTauriBranchRolloutGateway()

    await gateway.rollbackPreviewRendererRoute({
      presetId: 'preset_new-draft-2',
      publishedVersion: '2026.04.10',
      actorId: 'release-kim',
      actorLabel: 'Kim Release',
    })

    expect(invoke).toHaveBeenCalledWith('rollback_preview_renderer_route', {
      input: {
        presetId: 'preset_new-draft-2',
        publishedVersion: '2026.04.10',
        actorId: 'release-kim',
        actorLabel: 'Kim Release',
      },
    })
  })

  it('maps the preview route status request to the Tauri host DTO shape', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(null)
    const gateway = createTauriBranchRolloutGateway()

    await gateway.loadPreviewRendererRouteStatus({
      presetId: 'preset_new-draft-2',
      publishedVersion: '2026.04.10',
    })

    expect(invoke).toHaveBeenCalledWith('load_preview_renderer_route_status', {
      input: {
        presetId: 'preset_new-draft-2',
        publishedVersion: '2026.04.10',
      },
    })
  })
})
