import { renderHook, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

import {
  RESIDENT_ENGINE_VERSION,
  RESIDENT_ENGINE_WEBGL2,
  RESIDENT_RECIPE_SCHEMA_VERSION,
  type ResidentRecipe,
} from '../engine/resident-recipe'
import {
  useResidentRenderer,
  type ResidentRendererPlan,
} from './use-resident-renderer'

function recipe(): ResidentRecipe {
  return {
    schemaVersion: RESIDENT_RECIPE_SCHEMA_VERSION,
    presetId: 'preset_daylight',
    presetVersion: '2026.03.27',
    engine: RESIDENT_ENGINE_WEBGL2,
    engineVersion: RESIDENT_ENGINE_VERSION,
    recipeVersion: '2026.03.27',
    outputColorSpace: 'sRGB',
    iccIntent: 'perceptual',
    jpegQuality: 95,
    recipeHash: 'fnv1a64:1111111111111111',
    operations: [
      {
        order: 0,
        name: 'exposure',
        enabled: true,
        modVersion: 7,
        paramsHex: '00000000000080b9cdcc8c3f00004842000080c00000000001000000',
        blendopParams: '',
      },
    ],
  }
}

function plan(overrides: Partial<ResidentRendererPlan> = {}): ResidentRendererPlan {
  return {
    schemaVersion: 'resident-renderer-plan/v1',
    mode: 'shadow',
    engine: RESIDENT_ENGINE_WEBGL2,
    engineVersion: RESIDENT_ENGINE_VERSION,
    recipes: [recipe()],
    refusals: [],
    ...overrides,
  }
}

const SURFACE = {
  requiredSourceWidthPx: 1620,
  requiredSourceHeightPx: 1080,
  devicePixelRatio: 1,
  displayProfileId: 'approved-1080p',
}

describe('관람 창의 상주 renderer 수명주기', () => {
  it('does nothing at all while the lane is off', async () => {
    const getResidentRendererPlan = vi
      .fn()
      .mockResolvedValue(plan({ mode: 'off', recipes: [] }))

    const { result } = renderHook(() =>
      useResidentRenderer({
        surface: SURFACE,
        viewerEpoch: 1,
        hostService: { getResidentRendererPlan },
      }),
    )

    await waitFor(() => {
      expect(getResidentRendererPlan).toHaveBeenCalledTimes(1)
    })

    expect(result.current.mode).toBe('off')
    // 꺼진 실험은 GPU context조차 잡지 않는다.
    expect(result.current.engine).toBeNull()
    expect(result.current.status).toBe('uninitialized')
  })

  it('never asks the host for a plan before the viewer reports its photo rectangle', () => {
    const getResidentRendererPlan = vi.fn().mockResolvedValue(plan())

    renderHook(() =>
      useResidentRenderer({
        surface: {
          ...SURFACE,
          requiredSourceWidthPx: 0,
          requiredSourceHeightPx: 0,
        },
        viewerEpoch: 1,
        hostService: { getResidentRendererPlan },
      }),
    )

    expect(getResidentRendererPlan).not.toHaveBeenCalled()
  })

  it('returns to idle when a previously measured photo rectangle disappears', async () => {
    const getResidentRendererPlan = vi.fn().mockResolvedValue(plan())
    const { result, rerender } = renderHook(
      ({ surface }: { surface: typeof SURFACE | null }) =>
        useResidentRenderer({
          surface,
          viewerEpoch: 1,
          hostService: { getResidentRendererPlan },
        }),
      { initialProps: { surface: SURFACE as typeof SURFACE | null } },
    )

    await waitFor(() => {
      expect(result.current.status).toBe('unavailable')
    })

    rerender({ surface: null })

    await waitFor(() => {
      expect(result.current.status).toBe('uninitialized')
    })
    expect(result.current.engine).toBeNull()
  })

  it('reports unavailable instead of throwing when WebGL2 is missing', async () => {
    // jsdom에는 WebGL2가 없다. 상주 경로가 없는 실제 환경과 같은 조건이다.
    const getResidentRendererPlan = vi.fn().mockResolvedValue(plan())

    const { result } = renderHook(() =>
      useResidentRenderer({
        surface: SURFACE,
        viewerEpoch: 1,
        hostService: { getResidentRendererPlan },
      }),
    )

    await waitFor(() => {
      expect(result.current.status).toBe('unavailable')
    })

    expect(result.current.mode).toBe('shadow')
    expect(result.current.readyPresetKeys).toEqual([])
  })

  it('keeps every refusal visible instead of dropping the preset silently', async () => {
    const getResidentRendererPlan = vi.fn().mockResolvedValue(
      plan({
        mode: 'off',
        recipes: [],
        refusals: [
          {
            presetId: 'preset_mono-pop',
            presetVersion: '2026.03.27',
            reason: 'resident-operation-unsupported',
          },
        ],
      }),
    )

    const { result } = renderHook(() =>
      useResidentRenderer({
        surface: SURFACE,
        viewerEpoch: 1,
        hostService: { getResidentRendererPlan },
      }),
    )

    await waitFor(() => {
      expect(result.current.refusals).toHaveLength(1)
    })

    expect(result.current.refusals[0].reason).toBe(
      'resident-operation-unsupported',
    )
  })

  it('falls back to idle when the host plan cannot be read', async () => {
    const getResidentRendererPlan = vi
      .fn()
      .mockRejectedValue(new Error('host unavailable'))

    const { result } = renderHook(() =>
      useResidentRenderer({
        surface: SURFACE,
        viewerEpoch: 1,
        hostService: { getResidentRendererPlan },
      }),
    )

    await waitFor(() => {
      expect(getResidentRendererPlan).toHaveBeenCalled()
    })

    expect(result.current.mode).toBe('off')
    expect(result.current.engine).toBeNull()
  })

  it('re-prepares when the viewer window generation changes', async () => {
    const getResidentRendererPlan = vi.fn().mockResolvedValue(plan())

    const { rerender } = renderHook(
      ({ viewerEpoch }: { viewerEpoch: number }) =>
        useResidentRenderer({
          surface: SURFACE,
          viewerEpoch,
          hostService: { getResidentRendererPlan },
        }),
      { initialProps: { viewerEpoch: 1 } },
    )

    await waitFor(() => {
      expect(getResidentRendererPlan).toHaveBeenCalledTimes(1)
    })

    rerender({ viewerEpoch: 2 })

    // 관람 창이 재생성되면 이전 context는 죽은 세대에 속한다. 다시 세워야 한다.
    await waitFor(() => {
      expect(getResidentRendererPlan).toHaveBeenCalledTimes(2)
    })
  })

  it('re-prepares when the photo rectangle or DPR changes', async () => {
    const getResidentRendererPlan = vi.fn().mockResolvedValue(plan())

    const { rerender } = renderHook(
      ({ devicePixelRatio }: { devicePixelRatio: number }) =>
        useResidentRenderer({
          surface: { ...SURFACE, devicePixelRatio },
          viewerEpoch: 1,
          hostService: { getResidentRendererPlan },
        }),
      { initialProps: { devicePixelRatio: 1 } },
    )

    await waitFor(() => {
      expect(getResidentRendererPlan).toHaveBeenCalledTimes(1)
    })

    rerender({ devicePixelRatio: 2 })

    await waitFor(() => {
      expect(getResidentRendererPlan).toHaveBeenCalledTimes(2)
    })
  })
})
