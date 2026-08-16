/**
 * Story 7.5. host가 촬영 **전에** 만들어 둔 상주 실행 계획을 엔진이 쓰는 형태로 편다.
 *
 * 이 파일의 존재 이유는 AC 1이다. 촬영 hot path에서 XMP를 다시 순회하거나 shader를
 * 다시 컴파일하면 상주 renderer라고 부를 수 없다. 그래서 **계획을 펴는 일은 전부 여기서,
 * 촬영이 시작되기 전에** 끝난다.
 */

import {
  decodeOperationParams,
  type DarktableOperationParams,
} from './darktable-params'

/** host의 `RESIDENT_RECIPE_SCHEMA_VERSION`과 같은 값이어야 한다. */
export const RESIDENT_RECIPE_SCHEMA_VERSION = 'resident-recipe/v1' as const

/** host의 `RESIDENT_ENGINE_WEBGL2`와 같은 값이어야 한다. */
export const RESIDENT_ENGINE_WEBGL2 = 'webgl2-resident' as const

/** host의 `RESIDENT_ENGINE_VERSION`과 같은 값이어야 한다. */
export const RESIDENT_ENGINE_VERSION = '0.1.0-spike' as const

/**
 * 엔진이 **실제로 구현한** operation 목록.
 *
 * host의 `RESIDENT_OPERATION_ALLOWLIST`와 같아야 한다. 두 목록이 갈라지면
 * host는 통과시키고 엔진은 못 그리는 recipe가 생기고, 그 촬영은 화면이 비어 있게 된다.
 */
export const RESIDENT_OPERATION_ALLOWLIST = [
  'temperature',
  'exposure',
  'sigmoid',
  'monochrome',
  'bloom',
  'sharpen',
] as const

export type ResidentOperationName =
  (typeof RESIDENT_OPERATION_ALLOWLIST)[number]

/** host가 보내는 계획 안의 한 연산. params는 원문 그대로다. */
export type ResidentRecipeOperation = {
  readonly order: number
  readonly name: string
  readonly enabled: boolean
  readonly modVersion: number
  readonly paramsHex: string
  readonly blendopParams: string
}

/** host가 촬영 전에 확정한 상주 실행 계획. */
export type ResidentRecipe = {
  readonly schemaVersion: string
  readonly presetId: string
  readonly presetVersion: string
  readonly engine: string
  readonly engineVersion: string
  readonly recipeVersion: string
  readonly operations: readonly ResidentRecipeOperation[]
  readonly outputColorSpace: string
  readonly iccIntent: string
  readonly jpegQuality: number
  readonly recipeHash: string
}

/** 엔진이 바로 실행할 수 있게 펴 둔 계획. */
export type CompiledResidentPlan = {
  readonly presetId: string
  readonly presetVersion: string
  readonly recipeHash: string
  readonly steps: readonly DarktableOperationParams[]
}

/** 계획을 펼 수 없는 이유. host의 `ResidentIneligibleReason`과 같은 코드를 쓴다. */
export type ResidentPlanFailure =
  | 'resident-engine-version-mismatch'
  | 'resident-recipe-unresolvable'
  | 'resident-operation-unsupported'
  | 'resident-output-profile-mismatch'

export type ResidentPlanResult =
  | { readonly ok: true; readonly plan: CompiledResidentPlan }
  | { readonly ok: false; readonly reason: ResidentPlanFailure }

/**
 * 계획을 펴 둔다. **촬영 전에 부른다.**
 *
 * 실패는 예외가 아니라 결과다. 호출자는 그 preset을 상주 경로에서 빼고
 * 정확 darktable 경로로 내려보낸다.
 */
export function compileResidentPlan(recipe: ResidentRecipe): ResidentPlanResult {
  if (
    recipe.schemaVersion !== RESIDENT_RECIPE_SCHEMA_VERSION ||
    recipe.engine !== RESIDENT_ENGINE_WEBGL2 ||
    recipe.engineVersion !== RESIDENT_ENGINE_VERSION
  ) {
    return { ok: false, reason: 'resident-engine-version-mismatch' }
  }

  // 상주 엔진은 sRGB / perceptual 조합만 구현했다. 다른 조합을 조용히 sRGB로 처리하면
  // 색이 달라진 화면이 정상 화면처럼 게시된다.
  if (
    recipe.outputColorSpace.trim().toLowerCase() !== 'srgb' ||
    recipe.iccIntent.trim().toLowerCase() !== 'perceptual'
  ) {
    return { ok: false, reason: 'resident-output-profile-mismatch' }
  }

  if (recipe.operations.length === 0) {
    return { ok: false, reason: 'resident-recipe-unresolvable' }
  }

  const ordered = [...recipe.operations].sort((left, right) => left.order - right.order)
  const steps: DarktableOperationParams[] = []

  for (const operation of ordered) {
    if (!operation.enabled) {
      continue
    }

    if (
      !(RESIDENT_OPERATION_ALLOWLIST as readonly string[]).includes(
        operation.name,
      )
    ) {
      return { ok: false, reason: 'resident-operation-unsupported' }
    }

    const params = decodeOperationParams(
      operation.name,
      operation.modVersion,
      operation.paramsHex,
    )

    if (params === null) {
      // 구조체 버전이나 길이가 다르다. 같은 바이트를 다르게 읽으면 조용히 다른 화면이 나온다.
      return { ok: false, reason: 'resident-operation-unsupported' }
    }

    if (params.kind === 'exposure' && params.mode !== 0) {
      // deflicker는 촬영 통계가 필요한 별도 연산이다. 값을 무시한 채 manual exposure처럼
      // 게시하면 조용히 다른 화면이 되므로 정확 경로로 내린다.
      return { ok: false, reason: 'resident-operation-unsupported' }
    }

    steps.push(params)
  }

  if (steps.length === 0) {
    return { ok: false, reason: 'resident-recipe-unresolvable' }
  }

  return {
    ok: true,
    plan: {
      presetId: recipe.presetId,
      presetVersion: recipe.presetVersion,
      recipeHash: recipe.recipeHash,
      steps,
    },
  }
}

/**
 * host의 `content_hash`와 **같은** FNV-1a 64비트 해시.
 *
 * shader 원문이 바뀌었는데 같은 hash가 나오면 drift를 잡을 수 없다.
 * JS에는 u64가 없으므로 BigInt로 계산하고 host와 같은 형식으로 찍는다.
 */
export function fnv1a64(text: string): string {
  const mask = (1n << 64n) - 1n
  let hash = 0xcbf29ce484222325n

  const bytes = new TextEncoder().encode(text)

  for (const byte of bytes) {
    hash ^= BigInt(byte)
    hash = (hash * 0x00000100000001b3n) & mask
  }

  return `fnv1a64:${hash.toString(16).padStart(16, '0')}`
}
