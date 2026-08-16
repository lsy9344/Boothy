import { describe, expect, it } from 'vitest'

import {
  RESIDENT_ENGINE_VERSION,
  RESIDENT_ENGINE_WEBGL2,
  RESIDENT_RECIPE_SCHEMA_VERSION,
  compileResidentPlan,
  fnv1a64,
  type ResidentRecipe,
} from './resident-recipe'
import {
  RESIDENT_RENDER_BUDGET_MICROS,
  Webgl2ResidentEngine,
  collectUniformNames,
  type ResidentGl,
  type ResidentGlTextures,
} from './webgl2-resident-engine'
import { RESIDENT_POINTWISE_SHADER, RESIDENT_PROGRAM_SOURCES } from './shaders'

/**
 * 실제 GPU 없이 같은 코드 경로를 통과시키는 대역.
 *
 * 이 Story의 AC 1은 "hot path에서 shader compile이 일어나지 않는다"이고,
 * 그건 GPU가 아니라 **호출 순서**의 문제다. 그래서 대역으로 검증할 수 있고,
 * 검증해야 한다 — 실장비에서만 알 수 있는 것으로 미루면 회귀를 막을 수 없다.
 */
function createFakeGl() {
  const state = {
    compileCount: 0,
    linkCount: 0,
    drawCount: 0,
    createdTextures: 0,
    deletedTextures: 0,
    contextLost: false,
    failCompile: false,
  }

  const gl = {
    VERTEX_SHADER: 1,
    FRAGMENT_SHADER: 2,
    COMPILE_STATUS: 3,
    LINK_STATUS: 4,
    ARRAY_BUFFER: 5,
    STATIC_DRAW: 6,
    FLOAT: 7,
    TRIANGLE_STRIP: 8,
    TEXTURE_2D: 9,
    TEXTURE0: 100,
    RGBA: 11,
    RGBA8: 12,
    UNSIGNED_BYTE: 13,
    FRAMEBUFFER: 14,
    COLOR_ATTACHMENT0: 15,
    TEXTURE_MIN_FILTER: 16,
    TEXTURE_MAG_FILTER: 17,
    TEXTURE_WRAP_S: 18,
    TEXTURE_WRAP_T: 19,
    LINEAR: 20,
    CLAMP_TO_EDGE: 21,

    createShader: () => ({ tag: 'shader' }),
    shaderSource: () => undefined,
    compileShader: () => {
      state.compileCount += 1
    },
    getShaderParameter: () => !state.failCompile,
    getShaderInfoLog: () => null,
    deleteShader: () => undefined,

    createProgram: () => ({ tag: 'program' }),
    attachShader: () => undefined,
    linkProgram: () => {
      state.linkCount += 1
    },
    getProgramParameter: () => true,
    getProgramInfoLog: () => null,
    deleteProgram: () => undefined,
    useProgram: () => undefined,
    getUniformLocation: (_program: unknown, name: string) => ({ name }),

    uniform1i: () => undefined,
    uniform1f: () => undefined,
    uniform2f: () => undefined,
    uniform3f: () => undefined,

    createBuffer: () => ({ tag: 'buffer' }),
    bindBuffer: () => undefined,
    bufferData: () => undefined,
    enableVertexAttribArray: () => undefined,
    vertexAttribPointer: () => undefined,

    createTexture: () => {
      state.createdTextures += 1
      return { tag: 'texture', id: state.createdTextures }
    },
    bindTexture: () => undefined,
    activeTexture: () => undefined,
    texParameteri: () => undefined,
    deleteTexture: () => {
      state.deletedTextures += 1
    },

    createFramebuffer: () => ({ tag: 'framebuffer' }),
    bindFramebuffer: () => undefined,
    framebufferTexture2D: () => undefined,
    deleteFramebuffer: () => undefined,

    viewport: () => undefined,
    drawArrays: () => {
      state.drawCount += 1
    },
    finish: () => undefined,
    isContextLost: () => state.contextLost,
  } satisfies ResidentGl

  const textures: ResidentGlTextures = {
    allocate: () => undefined,
    upload: () => undefined,
    readBack: (_gl, width, height) =>
      new Uint8ClampedArray(width * height * 4),
  }

  return { gl, textures, state }
}

function recipe(overrides: Partial<ResidentRecipe> = {}): ResidentRecipe {
  const base: ResidentRecipe = {
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
        name: 'temperature',
        enabled: true,
        modVersion: 4,
        paramsHex: '000010400000803fcdccec3f0000000002000000',
        blendopParams: '',
      },
      {
        order: 1,
        name: 'exposure',
        enabled: true,
        modVersion: 7,
        paramsHex: '00000000000080b9cdcc8c3f00004842000080c00000000001000000',
        blendopParams: '',
      },
      {
        order: 3,
        name: 'bloom',
        enabled: true,
        modVersion: 1,
        paramsHex: '000040410000be4200008040',
        blendopParams: '',
      },
      {
        order: 4,
        name: 'sharpen',
        enabled: true,
        modVersion: 1,
        paramsHex: '000000409a99193fcdcccc3e',
        blendopParams: '',
      },
    ],
  }

  return { ...base, ...overrides }
}

const SURFACE = {
  widthPx: 1620,
  heightPx: 1080,
  devicePixelRatio: 1,
  displayProfileId: 'approved-1080p',
} as const

function engineWith(fake: ReturnType<typeof createFakeGl>) {
  let clock = 1_000

  return new Webgl2ResidentEngine({
    createContext: () => ({ gl: fake.gl, textures: fake.textures }),
    nowMicros: () => {
      clock += 100
      return clock
    },
    gpuVendor: 'fake-vendor',
    gpuRenderer: 'fake-renderer',
  })
}

describe('상주 WebGL2 엔진', () => {
  it('compiles every program during prewarm, before any capture', () => {
    const fake = createFakeGl()
    const engine = engineWith(fake)

    expect(engine.status).toBe('uninitialized')

    const status = engine.prewarm([recipe()], SURFACE)

    expect(status).toBe('ready')
    // 프로그램마다 vertex + fragment 두 개를 컴파일한다.
    expect(fake.state.compileCount).toBe(
      Object.keys(RESIDENT_PROGRAM_SOURCES).length * 2,
    )
    expect(fake.state.linkCount).toBe(Object.keys(RESIDENT_PROGRAM_SOURCES).length)
    expect(engine.contextInitializedAtMicros).toBeGreaterThan(0)
  })

  it('renders the hot path without compiling anything', () => {
    const fake = createFakeGl()
    const engine = engineWith(fake)
    engine.prewarm([recipe()], SURFACE)

    const compilesAfterPrewarm = fake.state.compileCount
    const result = engine.render(
      'preset_daylight',
      '2026.03.27',
      'fnv1a64:1111111111111111',
      {} as TexImageSource,
    )

    expect(result.ok).toBe(true)
    expect(fake.state.compileCount).toBe(compilesAfterPrewarm)
    expect(result.ok && result.hotPathProgramCompileCount).toBe(0)
    expect(engine.hotPathProgramCompileCount).toBe(0)
    // 상주 엔진은 프로세스를 띄우지 않는다. 띄운다면 상주가 아니다.
    expect(engine.hotPathProcessStartCount).toBe(0)
  })

  it('does not allocate new GPU textures during the hot path', () => {
    const fake = createFakeGl()
    const engine = engineWith(fake)
    engine.prewarm([recipe()], SURFACE)

    const texturesAfterPrewarm = fake.state.createdTextures

    engine.render(
      'preset_daylight',
      '2026.03.27',
      'fnv1a64:1111111111111111',
      {} as TexImageSource,
    )

    expect(fake.state.createdTextures).toBe(texturesAfterPrewarm)
  })

  it('refuses a preset it never prewarmed instead of compiling it late', () => {
    const fake = createFakeGl()
    const engine = engineWith(fake)
    engine.prewarm([recipe()], SURFACE)

    const result = engine.render(
      'preset_mono-pop',
      '2026.03.27',
      'fnv1a64:1111111111111111',
      {} as TexImageSource,
    )

    expect(result).toEqual({ ok: false, reason: 'resident-operation-unsupported' })
    expect(engine.canRender('preset_mono-pop', '2026.03.27')).toBe(false)
  })

  it('refuses a capture whose recipe hash drifted from the prewarmed plan', () => {
    const fake = createFakeGl()
    const engine = engineWith(fake)
    engine.prewarm([recipe()], SURFACE)

    const result = engine.render(
      'preset_daylight',
      '2026.03.27',
      'fnv1a64:2222222222222222',
      {} as TexImageSource,
    )

    expect(result).toEqual({ ok: false, reason: 'resident-recipe-hash-mismatch' })
  })

  it('refuses a preset version that does not match the capture binding', () => {
    // 세션 중 preset이 바뀌면(Story 2.3) 이전 버전의 계획을 다시 쓰면 안 된다.
    const fake = createFakeGl()
    const engine = engineWith(fake)
    engine.prewarm([recipe()], SURFACE)

    expect(engine.canRender('preset_daylight', '2026.03.27')).toBe(true)
    expect(engine.canRender('preset_daylight', '2026.09.01')).toBe(false)
  })

  it('falls back instead of rendering when the context is lost mid-session', () => {
    const fake = createFakeGl()
    const engine = engineWith(fake)
    engine.prewarm([recipe()], SURFACE)

    fake.state.contextLost = true

    const result = engine.render(
      'preset_daylight',
      '2026.03.27',
      'fnv1a64:1111111111111111',
      {} as TexImageSource,
    )

    expect(result).toEqual({ ok: false, reason: 'resident-context-not-ready' })
    expect(engine.status).toBe('context-lost')
    // 손실 상태에서는 자격도 사라진다. 준비되지 않은 결과가 화면에 올라가지 않는다.
    expect(engine.canRender('preset_daylight', '2026.03.27')).toBe(false)
  })

  it('re-initializes safely after a context loss', () => {
    const fake = createFakeGl()
    const engine = engineWith(fake)
    engine.prewarm([recipe()], SURFACE)

    const firstEpoch = engine.contextEpoch
    engine.handleContextLost()
    fake.state.contextLost = false

    expect(engine.prewarm([recipe()], SURFACE)).toBe('ready')
    expect(engine.contextEpoch).toBe(firstEpoch + 1)
    expect(engine.canRender('preset_daylight', '2026.03.27')).toBe(true)
  })

  it('reallocates the output surface when the monitor or DPR changes', () => {
    const fake = createFakeGl()
    const engine = engineWith(fake)
    engine.prewarm([recipe()], SURFACE)

    const compilesAfterPrewarm = fake.state.compileCount
    const deletedBefore = fake.state.deletedTextures

    expect(
      engine.resizeSurface({ ...SURFACE, widthPx: 2560, heightPx: 1440, devicePixelRatio: 2 }),
    ).toBe(true)

    // 화면이 바뀌었다고 shader가 바뀌지는 않는다. 다시 컴파일하면 hot path에 위험이 생긴다.
    expect(fake.state.compileCount).toBe(compilesAfterPrewarm)
    expect(fake.state.deletedTextures).toBeGreaterThan(deletedBefore)
  })

  it('reports unavailable instead of throwing when WebGL2 cannot be created', () => {
    let clock = 0
    const engine = new Webgl2ResidentEngine({
      createContext: () => null,
      nowMicros: () => {
        clock += 1
        return clock
      },
    })

    expect(engine.prewarm([recipe()], SURFACE)).toBe('unavailable')
    expect(
      engine.render('preset_daylight', '2026.03.27', 'x', {} as TexImageSource),
    ).toEqual({ ok: false, reason: 'resident-capability-unavailable' })
  })

  it('reports unavailable when a shader fails to compile at prewarm', () => {
    const fake = createFakeGl()
    fake.state.failCompile = true
    const engine = engineWith(fake)

    expect(engine.prewarm([recipe()], SURFACE)).toBe('unavailable')
  })

  it('keeps frontend compile refusals visible by preset', () => {
    const fake = createFakeGl()
    const engine = engineWith(fake)
    const unsupported = recipe({ engineVersion: '0.0.1-old' })

    expect(engine.prewarm([unsupported], SURFACE)).toBe('ready')
    expect(engine.refusals).toEqual([
      {
        presetId: 'preset_daylight',
        presetVersion: '2026.03.27',
        reason: 'resident-engine-version-mismatch',
      },
    ])
    expect(engine.canRender('preset_daylight', '2026.03.27')).toBe(false)
  })

  it('discards a frame that blew the render budget instead of showing it late', () => {
    // 드라이버가 멈추면 상주 lane이 조용히 오래 걸린다. 그 결과를 그대로 쓰면
    // 고객은 정확 경로보다 더 오래 기다리고, evidence에는 성공으로 남는다.
    const fake = createFakeGl()
    let clock = 0
    const engine = new Webgl2ResidentEngine({
      createContext: () => ({ gl: fake.gl, textures: fake.textures }),
      // 매 호출마다 2초씩 흐른다. 예산 1.5초를 반드시 넘긴다.
      nowMicros: () => {
        clock += 2_000_000
        return clock
      },
    })
    engine.prewarm([recipe()], SURFACE)

    expect(
      engine.render(
        'preset_daylight',
        '2026.03.27',
        'fnv1a64:1111111111111111',
        {} as TexImageSource,
      ),
    ).toEqual({ ok: false, reason: 'resident-render-timeout' })
  })

  it('keeps a frame that stayed inside the render budget', () => {
    const fake = createFakeGl()
    let clock = 0
    const engine = new Webgl2ResidentEngine({
      createContext: () => ({ gl: fake.gl, textures: fake.textures }),
      nowMicros: () => {
        clock += 1_000
        return clock
      },
      renderBudgetMicros: RESIDENT_RENDER_BUDGET_MICROS,
    })
    engine.prewarm([recipe()], SURFACE)

    expect(
      engine.render(
        'preset_daylight',
        '2026.03.27',
        'fnv1a64:1111111111111111',
        {} as TexImageSource,
      ).ok,
    ).toBe(true)
  })

  it('refuses to render before the viewer reports a photo rectangle', () => {
    const fake = createFakeGl()
    const engine = engineWith(fake)
    engine.prewarm([recipe()], { ...SURFACE, widthPx: 0, heightPx: 0 })

    expect(
      engine.render(
        'preset_daylight',
        '2026.03.27',
        'fnv1a64:1111111111111111',
        {} as TexImageSource,
      ),
    ).toEqual({ ok: false, reason: 'resident-viewer-not-ready' })
  })

  it('derives its program hash from the shader source itself', () => {
    const fake = createFakeGl()
    const engine = engineWith(fake)

    expect(engine.programHash).toMatch(/^fnv1a64:[0-9a-f]{16}$/)
    // 같은 원문이면 같은 해시. 이것이 성립해야 drift 판정이 의미를 갖는다.
    expect(engineWith(createFakeGl()).programHash).toBe(engine.programHash)
    // HV-16 evidence에 기록된 값. shader를 고치면 이 테스트가 먼저 실패하고,
    // 그때 evidence의 program hash도 함께 갱신해야 한다는 것을 알려 준다.
    expect(engine.programHash).toBe('fnv1a64:bcbf55e37b6cc622')
  })

  it('collects uniform names from the shader source instead of a hand-written list', () => {
    const names = collectUniformNames(RESIDENT_POINTWISE_SHADER)

    expect(names).toContain('uSource')
    expect(names).toContain('uTemperatureCoeffs')
    expect(names).toContain('uSigmoidContrast')
    expect(names).toContain('uMonochromeHighlights')
  })
})

describe('상주 계획 컴파일', () => {
  it('refuses a recipe compiled for a different engine version', () => {
    expect(compileResidentPlan(recipe({ engineVersion: '0.0.1-old' }))).toEqual({
      ok: false,
      reason: 'resident-engine-version-mismatch',
    })
  })

  it('refuses an output profile the engine did not implement', () => {
    expect(compileResidentPlan(recipe({ outputColorSpace: 'AdobeRGB' }))).toEqual({
      ok: false,
      reason: 'resident-output-profile-mismatch',
    })
    expect(
      compileResidentPlan(recipe({ iccIntent: 'relative_colorimetric' })),
    ).toEqual({ ok: false, reason: 'resident-output-profile-mismatch' })
  })

  it('refuses the whole recipe when one operation is outside the allowlist', () => {
    const withUnknown = recipe({
      operations: [
        ...recipe().operations,
        {
          order: 5,
          name: 'retouch',
          enabled: true,
          modVersion: 1,
          paramsHex: '00000000',
          blendopParams: '',
        },
      ],
    })

    expect(compileResidentPlan(withUnknown)).toEqual({
      ok: false,
      reason: 'resident-operation-unsupported',
    })
  })

  it('refuses exposure deflicker instead of silently disabling exposure', () => {
    const deflickerExposure = recipe().operations.map((operation) =>
      operation.name === 'exposure'
        ? {
            ...operation,
            paramsHex:
              '01000000000080b9cdcc8c3f00004842000080c00000000001000000',
          }
        : operation,
    )

    expect(compileResidentPlan(recipe({ operations: deflickerExposure }))).toEqual({
      ok: false,
      reason: 'resident-operation-unsupported',
    })
  })

  it('keeps the darktable operation order instead of the array order', () => {
    const shuffled = recipe({
      operations: [...recipe().operations].reverse(),
    })
    const result = compileResidentPlan(shuffled)

    expect(result.ok).toBe(true)
    expect(result.ok && result.plan.steps.map((step) => step.kind)).toEqual([
      'temperature',
      'exposure',
      'bloom',
      'sharpen',
    ])
  })

  it('produces the same fnv1a64 shape the Rust host writes', () => {
    expect(fnv1a64('')).toBe('fnv1a64:cbf29ce484222325')
    expect(fnv1a64('a')).toBe('fnv1a64:af63dc4c8601ec8c')
  })
})
