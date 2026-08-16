/**
 * Story 7.5. 상주 renderer의 유일한 host 경계.
 *
 * Tauri 밖(브라우저 dev, vitest)에서는 **`off` 계획**을 돌려준다. 데스크톱 런타임 없이
 * 상주 경로가 있다고 주장하면, 있지도 않은 GPU context를 세우려다 조용히 실패한다.
 */

import { invoke } from '@tauri-apps/api/core'

import { isTauriRuntime } from '../../shared/runtime/is-tauri'
import type {
  ResidentRendererHostService,
  ResidentRendererPlan,
} from '../state/use-resident-renderer'
import {
  RESIDENT_ENGINE_VERSION,
  RESIDENT_ENGINE_WEBGL2,
} from '../engine/resident-recipe'

/** 승인된 세 preset. host는 이 목록으로만 계획을 만든다. */
export type ResidentPresetBinding = {
  readonly presetId: string
  readonly presetVersion: string
}

export function buildDisabledResidentPlan(): ResidentRendererPlan {
  return {
    schemaVersion: 'resident-renderer-plan/v1',
    mode: 'off',
    engine: RESIDENT_ENGINE_WEBGL2,
    engineVersion: RESIDENT_ENGINE_VERSION,
    recipes: [],
    refusals: [],
  }
}

export function createResidentRendererHostService(
  presets: readonly ResidentPresetBinding[] = [],
): ResidentRendererHostService {
  return {
    async getResidentRendererPlan() {
      if (!isTauriRuntime()) {
        return buildDisabledResidentPlan()
      }

      const plan = await invoke<ResidentRendererPlan>(
        'get_resident_renderer_plan',
        { presets: presets.map((preset) => [preset.presetId, preset.presetVersion]) },
      )

      // host가 알 수 없는 mode를 보내면 켜지 않는다. 오타가 실험을 켜는 일은 없어야 한다.
      if (
        plan.mode !== 'off' &&
        plan.mode !== 'shadow' &&
        plan.mode !== 'evidence'
      ) {
        return { ...plan, mode: 'off' as const, recipes: [] }
      }

      return plan
    },
  }
}
