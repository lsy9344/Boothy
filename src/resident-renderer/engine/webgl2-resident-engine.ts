/**
 * Story 7.5. 앱이 켜져 있는 동안 계속 준비된 WebGL2 display renderer prototype.
 *
 * 이 엔진의 계약은 하나다: **촬영이 시작된 뒤에는 아무것도 만들지 않는다.**
 * context, program, 출력 surface는 전부 촬영 전에 준비되고, hot path는 이미 있는 것을
 * 쓰기만 한다. 그 사실을 주장이 아니라 **관측값**으로 남기려고 컴파일 횟수와 process 시작
 * 횟수를 세어서 generation provenance에 싣는다 (AC 1).
 *
 * 실패는 전부 같은 곳으로 간다: **정확 darktable 경로로 fallback**, 그리고 현재 화면 유지.
 * 준비되지 않은 결과는 어떤 경우에도 고객 화면에 올라가지 않는다.
 */

import {
  RESIDENT_ENGINE_VERSION,
  RESIDENT_ENGINE_WEBGL2,
  compileResidentPlan,
  fnv1a64,
  type CompiledResidentPlan,
  type ResidentPlanFailure,
  type ResidentRecipe,
} from './resident-recipe'
import {
  RESIDENT_PROGRAM_SOURCES,
  RESIDENT_VERTEX_SHADER,
  type ResidentProgramName,
} from './shaders'

/**
 * 엔진이 실제로 쓰는 WebGL2 표면만 추린 구조적 타입.
 *
 * 진짜 `WebGL2RenderingContext`가 이 모양을 만족하고, 테스트는 작은 대역으로 같은 경로를
 * 통과시킨다. 실제 GPU 없이도 **"hot path에서 컴파일이 일어나지 않는다"**를 검증할 수 있어야
 * 하기 때문이다 — 그 검증이 이 Story의 AC 1 그 자체다.
 */
export type ResidentGl = {
  readonly VERTEX_SHADER: number
  readonly FRAGMENT_SHADER: number
  readonly COMPILE_STATUS: number
  readonly LINK_STATUS: number
  readonly ARRAY_BUFFER: number
  readonly STATIC_DRAW: number
  readonly FLOAT: number
  readonly TRIANGLE_STRIP: number
  readonly TEXTURE_2D: number
  readonly TEXTURE0: number
  readonly RGBA: number
  readonly RGBA8: number
  readonly UNSIGNED_BYTE: number
  readonly FRAMEBUFFER: number
  readonly COLOR_ATTACHMENT0: number
  readonly TEXTURE_MIN_FILTER: number
  readonly TEXTURE_MAG_FILTER: number
  readonly TEXTURE_WRAP_S: number
  readonly TEXTURE_WRAP_T: number
  readonly LINEAR: number
  readonly CLAMP_TO_EDGE: number

  createShader(type: number): unknown
  shaderSource(shader: unknown, source: string): void
  compileShader(shader: unknown): void
  getShaderParameter(shader: unknown, name: number): unknown
  getShaderInfoLog(shader: unknown): string | null
  deleteShader(shader: unknown): void

  createProgram(): unknown
  attachShader(program: unknown, shader: unknown): void
  linkProgram(program: unknown): void
  getProgramParameter(program: unknown, name: number): unknown
  getProgramInfoLog(program: unknown): string | null
  deleteProgram(program: unknown): void
  useProgram(program: unknown): void
  getUniformLocation(program: unknown, name: string): unknown

  uniform1i(location: unknown, value: number): void
  uniform1f(location: unknown, value: number): void
  uniform2f(location: unknown, x: number, y: number): void
  uniform3f(location: unknown, x: number, y: number, z: number): void

  createBuffer(): unknown
  bindBuffer(target: number, buffer: unknown): void
  bufferData(target: number, data: ArrayBufferView, usage: number): void
  enableVertexAttribArray(index: number): void
  vertexAttribPointer(
    index: number,
    size: number,
    type: number,
    normalized: boolean,
    stride: number,
    offset: number,
  ): void

  createTexture(): unknown
  bindTexture(target: number, texture: unknown): void
  activeTexture(unit: number): void
  texParameteri(target: number, name: number, value: number): void
  deleteTexture(texture: unknown): void

  createFramebuffer(): unknown
  bindFramebuffer(target: number, framebuffer: unknown): void
  framebufferTexture2D(
    target: number,
    attachment: number,
    textureTarget: number,
    texture: unknown,
    level: number,
  ): void
  deleteFramebuffer(framebuffer: unknown): void

  viewport(x: number, y: number, width: number, height: number): void
  drawArrays(mode: number, first: number, count: number): void
  finish(): void
  isContextLost(): boolean
}

/** `texImage2D`는 오버로드가 많아 좁은 두 형태만 따로 요구한다. */
export type ResidentGlTextures = {
  allocate(gl: ResidentGl, width: number, height: number): void
  upload(gl: ResidentGl, source: TexImageSource): void
  readBack(
    gl: ResidentGl,
    width: number,
    height: number,
  ): Uint8ClampedArray | null
}

export type ResidentEngineStatus =
  | 'uninitialized'
  | 'ready'
  | 'context-lost'
  | 'unavailable'

/** 상주 렌더가 거절된 이유. host의 `ResidentIneligibleReason` 코드와 같다. */
export type ResidentRenderFailure =
  | ResidentPlanFailure
  | 'resident-capability-unavailable'
  | 'resident-context-not-ready'
  | 'resident-viewer-not-ready'
  | 'resident-recipe-hash-mismatch'
  | 'resident-render-timeout'

/**
 * 한 프레임에 허용하는 상한. 넘으면 결과를 쓰지 않고 정확 경로로 내려간다.
 *
 * 드라이버가 멈추거나 GPU가 붙잡혀 있을 때 상주 lane이 조용히 오래 걸리면,
 * 그 시간이 그대로 고객이 기다리는 시간이 된다. **느린 성공보다 빠른 fallback이 낫다.**
 * 값은 darktable 정확 경로 median(약 3.7초)보다 충분히 작아야 의미가 있다.
 */
export const RESIDENT_RENDER_BUDGET_MICROS = 1_500_000

export type ResidentRenderSuccess = {
  readonly ok: true
  readonly pixels: Uint8ClampedArray
  readonly widthPx: number
  readonly heightPx: number
  readonly renderStartedAtMicros: number
  readonly renderCompletedAtMicros: number
  /** 이번 렌더 중 일어난 컴파일 횟수. **0이 아니면 AC 1 실패다.** */
  readonly hotPathProgramCompileCount: number
}

export type ResidentRenderResult =
  | ResidentRenderSuccess
  | { readonly ok: false; readonly reason: ResidentRenderFailure }

export type ResidentSurfaceSpec = {
  readonly widthPx: number
  readonly heightPx: number
  readonly devicePixelRatio: number
  readonly displayProfileId: string
}

export type ResidentEngineDeps = {
  /** WebGL2 context를 만든다. 만들 수 없으면 `null`. */
  createContext(): { gl: ResidentGl; textures: ResidentGlTextures } | null
  /** host monotonic으로 보정된 micros clock. */
  nowMicros(): number
  gpuVendor?: string
  gpuRenderer?: string
  /** 생략하면 `RESIDENT_RENDER_BUDGET_MICROS`. */
  renderBudgetMicros?: number
}

type ProgramRecord = {
  readonly program: unknown
  readonly uniforms: Map<string, unknown>
}

type SurfaceRecord = {
  readonly spec: ResidentSurfaceSpec
  readonly textures: unknown[]
  readonly framebuffers: unknown[]
}

export type ResidentPlanRefusal = {
  readonly presetId: string
  readonly presetVersion: string
  readonly reason: ResidentPlanFailure
}

/**
 * 앱 수명주기 동안 살아 있는 상주 renderer.
 *
 * 인스턴스 하나가 context 하나를 소유한다. 여러 개를 만들면 GPU 자원이 조용히 늘어나고
 * device loss 복구가 어느 인스턴스의 것인지 알 수 없게 된다.
 */
export class Webgl2ResidentEngine {
  readonly engine = RESIDENT_ENGINE_WEBGL2
  readonly engineVersion = RESIDENT_ENGINE_VERSION

  #deps: ResidentEngineDeps
  #gl: ResidentGl | null = null
  #textures: ResidentGlTextures | null = null
  #status: ResidentEngineStatus = 'uninitialized'
  #programs = new Map<ResidentProgramName, ProgramRecord>()
  #plans = new Map<string, CompiledResidentPlan>()
  #refusals: ResidentPlanRefusal[] = []
  #surface: SurfaceRecord | null = null
  #contextInitializedAtMicros = 0
  #contextEpoch = 0
  #totalProgramCompileCount = 0
  #hotPathProgramCompileCount = 0
  #hotPathProcessStartCount = 0
  #renderInFlight = false
  #programHash: string

  constructor(deps: ResidentEngineDeps) {
    this.#deps = deps
    // program hash는 shader 원문에서만 나온다. 원문이 바뀌면 값이 바뀌어야 drift를 잡는다.
    this.#programHash = fnv1a64(
      [
        RESIDENT_VERTEX_SHADER,
        ...Object.keys(RESIDENT_PROGRAM_SOURCES)
          .sort()
          .map(
            (name) =>
              RESIDENT_PROGRAM_SOURCES[name as ResidentProgramName],
          ),
      ].join(String.fromCharCode(0)),
    )
  }

  get status(): ResidentEngineStatus {
    return this.#status
  }

  get programHash(): string {
    return this.#programHash
  }

  get contextInitializedAtMicros(): number {
    return this.#contextInitializedAtMicros
  }

  get contextEpoch(): number {
    return this.#contextEpoch
  }

  get hotPathProgramCompileCount(): number {
    return this.#hotPathProgramCompileCount
  }

  get hotPathProcessStartCount(): number {
    // 상주 엔진은 프로세스를 띄우지 않는다. 이 값이 0이 아니면 상주가 아니다.
    return this.#hotPathProcessStartCount
  }

  get refusals(): readonly ResidentPlanRefusal[] {
    return this.#refusals
  }

  get gpuVendor(): string {
    return this.#deps.gpuVendor ?? 'unknown'
  }

  get gpuRenderer(): string {
    return this.#deps.gpuRenderer ?? 'unknown'
  }

  /**
   * 촬영 **전에** 부른다. context를 만들고 모든 program과 preset 계획을 준비한다.
   *
   * 실패해도 예외를 던지지 않는다. 상주 경로가 없을 뿐이고 제품은 정확 경로로 동작한다.
   */
  prewarm(
    recipes: readonly ResidentRecipe[],
    surface: ResidentSurfaceSpec,
  ): ResidentEngineStatus {
    const created = this.#deps.createContext()

    if (created === null) {
      this.#status = 'unavailable'
      return this.#status
    }

    this.#gl = created.gl
    this.#textures = created.textures
    this.#contextEpoch += 1
    this.#contextInitializedAtMicros = this.#deps.nowMicros()
    this.#programs.clear()
    this.#plans.clear()
    this.#refusals = []

    for (const name of Object.keys(
      RESIDENT_PROGRAM_SOURCES,
    ) as ResidentProgramName[]) {
      const record = this.#buildProgram(created.gl, name)

      if (record === null) {
        this.#status = 'unavailable'
        return this.#status
      }

      this.#programs.set(name, record)
    }

    for (const recipe of recipes) {
      const result = compileResidentPlan(recipe)

      if (result.ok) {
        // 계획 키는 preset identity + version이다. 세션 중 preset이 바뀌어도
        // 이전 preset의 계획을 실수로 다시 쓰지 않는다.
        this.#plans.set(planKey(recipe.presetId, recipe.presetVersion), result.plan)
      } else {
        this.#refusals.push({
          presetId: recipe.presetId,
          presetVersion: recipe.presetVersion,
          reason: result.reason,
        })
      }
    }

    this.#allocateSurface(created.gl, created.textures, surface)
    this.#status = 'ready'

    return this.#status
  }

  /**
   * 화면 크기나 DPR이 바뀌었을 때 출력 surface만 다시 잡는다.
   *
   * program은 다시 컴파일하지 않는다 — 화면이 바뀌었다고 shader가 바뀌지는 않는다.
   */
  resizeSurface(surface: ResidentSurfaceSpec): boolean {
    if (this.#gl === null || this.#textures === null || this.#status !== 'ready') {
      return false
    }

    this.#allocateSurface(this.#gl, this.#textures, surface)

    return true
  }

  /** WebView2 재생성이나 device loss 뒤 안전하게 다시 세운다. */
  handleContextLost(): void {
    this.#status = 'context-lost'
    this.#gl = null
    this.#textures = null
    this.#programs.clear()
    this.#surface = null
    // 계획은 GPU 자원이 아니다. 하지만 새 context의 program과 짝이 맞아야 하므로 함께 버린다.
    this.#plans.clear()
    this.#refusals = []
  }

  /** 지금 이 preset을 상주 경로로 그릴 수 있는가? **렌더도 할당도 하지 않는다.** */
  canRender(presetId: string, presetVersion: string): boolean {
    return (
      this.#status === 'ready' &&
      this.#surface !== null &&
      this.#plans.has(planKey(presetId, presetVersion))
    )
  }

  /**
   * hot path. **이 안에서는 어떤 자원도 새로 만들지 않는다.**
   *
   * 컴파일이 필요하다는 것을 발견하면 렌더하지 않고 거절한다. 여기서 컴파일해 버리면
   * 측정값이 "상주 renderer의 성능"이 아니게 되고, AC 1이 금지한 바로 그 일이 된다.
   */
  render(
    presetId: string,
    presetVersion: string,
    recipeHash: string,
    source: TexImageSource,
  ): ResidentRenderResult {
    if (this.#status === 'unavailable') {
      return { ok: false, reason: 'resident-capability-unavailable' }
    }

    const gl = this.#gl
    const textures = this.#textures

    if (gl === null || textures === null || this.#status !== 'ready') {
      return { ok: false, reason: 'resident-context-not-ready' }
    }

    if (gl.isContextLost()) {
      this.handleContextLost()
      return { ok: false, reason: 'resident-context-not-ready' }
    }

    const surface = this.#surface

    if (surface === null || surface.spec.widthPx === 0 || surface.spec.heightPx === 0) {
      return { ok: false, reason: 'resident-viewer-not-ready' }
    }

    const plan = this.#plans.get(planKey(presetId, presetVersion))

    if (plan === undefined) {
      return { ok: false, reason: 'resident-operation-unsupported' }
    }

    if (plan.recipeHash !== recipeHash) {
      // host가 들고 있는 계획과 엔진이 편 계획이 다르다. 둘 중 하나는 낡았다.
      return { ok: false, reason: 'resident-recipe-hash-mismatch' }
    }

    const compilesBefore = this.#totalProgramCompileCount
    const renderStartedAtMicros = this.#deps.nowMicros()

    this.#renderInFlight = true

    let executed = false

    try {
      executed = this.#execute(gl, textures, plan, surface, source)
    } finally {
      this.#renderInFlight = false
    }

    if (!executed) {
      return { ok: false, reason: 'resident-capability-unavailable' }
    }

    const pixels = textures.readBack(
      gl,
      surface.spec.widthPx,
      surface.spec.heightPx,
    )
    const renderCompletedAtMicros = this.#deps.nowMicros()

    if (pixels === null) {
      return { ok: false, reason: 'resident-context-not-ready' }
    }

    // 예산을 넘긴 프레임은 **버린다.** 늦게 도착한 상주 결과를 그대로 쓰면
    // 고객은 정확 경로보다 더 오래 기다리고, 그 시간이 evidence에는 성공으로 남는다.
    if (
      renderCompletedAtMicros - renderStartedAtMicros >
      (this.#deps.renderBudgetMicros ?? RESIDENT_RENDER_BUDGET_MICROS)
    ) {
      return { ok: false, reason: 'resident-render-timeout' }
    }

    return {
      ok: true,
      pixels,
      widthPx: surface.spec.widthPx,
      heightPx: surface.spec.heightPx,
      renderStartedAtMicros,
      renderCompletedAtMicros,
      hotPathProgramCompileCount: this.#totalProgramCompileCount - compilesBefore,
    }
  }

  #execute(
    gl: ResidentGl,
    textures: ResidentGlTextures,
    plan: CompiledResidentPlan,
    surface: SurfaceRecord,
    source: TexImageSource,
  ): boolean {
    const { widthPx, heightPx } = surface.spec

    gl.activeTexture(gl.TEXTURE0)
    gl.bindTexture(gl.TEXTURE_2D, surface.textures[0])
    textures.upload(gl, source)

    const pointwise = this.#programs.get('pointwise')

    if (pointwise === undefined) {
      return false
    }

    gl.bindFramebuffer(gl.FRAMEBUFFER, surface.framebuffers[1])
    gl.viewport(0, 0, widthPx, heightPx)
    gl.useProgram(pointwise.program)
    this.#applyPointwiseUniforms(gl, pointwise, plan)
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4)

    // bloom / sharpen은 이웃 픽셀을 읽으므로 별도 pass가 필요하다.
    // 두 연산 모두 같은 blur program을 쓴다 — preset이 늘어도 program 수는 그대로다.
    for (const step of plan.steps) {
      if (step.kind === 'bloom') {
        if (
          !this.#runBlurPair(gl, surface, step.size, step.threshold / 255) ||
          !this.#composite(gl, 'bloomComposite', surface, {
            uStrength: step.strength / 100,
          })
        ) {
          return false
        }
      }

      if (step.kind === 'sharpen') {
        if (
          !this.#runBlurPair(gl, surface, step.radius, null) ||
          !this.#composite(gl, 'sharpen', surface, {
            uAmount: step.amount,
            uThreshold: step.threshold,
          })
        ) {
          return false
        }
      }
    }

    gl.bindFramebuffer(gl.FRAMEBUFFER, null)
    gl.viewport(0, 0, widthPx, heightPx)

    const blit = this.#programs.get('blit')

    if (blit === undefined) {
      return false
    }

    gl.useProgram(blit.program)
    gl.activeTexture(gl.TEXTURE0)
    gl.bindTexture(gl.TEXTURE_2D, surface.textures[1])
    gl.uniform1i(blit.uniforms.get('uSource'), 0)
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4)

    gl.finish()
    return true
  }

  #applyPointwiseUniforms(
    gl: ResidentGl,
    record: ProgramRecord,
    plan: CompiledResidentPlan,
  ): void {
    const uniform = (name: string) => record.uniforms.get(name)

    gl.uniform1i(uniform('uSource'), 0)
    gl.uniform1i(uniform('uTemperatureEnabled'), 0)
    gl.uniform1i(uniform('uExposureEnabled'), 0)
    gl.uniform1i(uniform('uSigmoidEnabled'), 0)
    gl.uniform1i(uniform('uMonochromeEnabled'), 0)

    for (const step of plan.steps) {
      switch (step.kind) {
        case 'temperature':
          gl.uniform1i(uniform('uTemperatureEnabled'), 1)
          gl.uniform3f(
            uniform('uTemperatureCoeffs'),
            step.red,
            step.green,
            step.blue,
          )
          break

        case 'exposure':
          // deflicker mode는 촬영 통계가 필요하다. 상주 엔진은 manual만 구현했고,
          // 계획 단계에서 이미 걸러졌어야 한다.
          gl.uniform1i(uniform('uExposureEnabled'), step.mode === 0 ? 1 : 0)
          gl.uniform1f(uniform('uExposureBlack'), step.black)
          gl.uniform1f(uniform('uExposureEv'), step.exposureEv)
          break

        case 'sigmoid':
          gl.uniform1i(uniform('uSigmoidEnabled'), 1)
          gl.uniform1f(uniform('uSigmoidContrast'), step.middleGreyContrast)
          gl.uniform1f(uniform('uSigmoidSkew'), step.contrastSkewness)
          gl.uniform1f(uniform('uSigmoidWhiteTarget'), step.displayWhiteTarget)
          gl.uniform1f(uniform('uSigmoidBlackTarget'), step.displayBlackTarget)
          break

        case 'monochrome':
          gl.uniform1i(uniform('uMonochromeEnabled'), 1)
          gl.uniform1f(uniform('uMonochromeHighlights'), step.highlights)
          break

        default:
          break
      }
    }
  }

  #runBlurPair(
    gl: ResidentGl,
    surface: SurfaceRecord,
    radius: number,
    threshold: number | null,
  ): boolean {
    const blur = this.#programs.get('blur')

    if (blur === undefined) {
      return false
    }

    const { widthPx, heightPx } = surface.spec

    gl.useProgram(blur.program)
    gl.uniform1i(blur.uniforms.get('uSource'), 0)
    gl.uniform1f(blur.uniforms.get('uRadius'), radius)
    gl.uniform1i(
      blur.uniforms.get('uThresholdEnabled'),
      threshold === null ? 0 : 1,
    )
    gl.uniform1f(blur.uniforms.get('uThreshold'), threshold ?? 0)

    // 가로 방향 → 임시 버퍼
    gl.bindFramebuffer(gl.FRAMEBUFFER, surface.framebuffers[2])
    gl.viewport(0, 0, widthPx, heightPx)
    gl.activeTexture(gl.TEXTURE0)
    gl.bindTexture(gl.TEXTURE_2D, surface.textures[1])
    gl.uniform2f(blur.uniforms.get('uTexelStep'), 1 / widthPx, 0)
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4)

    // 세로 방향 → blur 결과 버퍼. 임계값은 첫 pass에서 이미 적용됐다.
    gl.bindFramebuffer(gl.FRAMEBUFFER, surface.framebuffers[3])
    gl.activeTexture(gl.TEXTURE0)
    gl.bindTexture(gl.TEXTURE_2D, surface.textures[2])
    gl.uniform1i(blur.uniforms.get('uThresholdEnabled'), 0)
    gl.uniform2f(blur.uniforms.get('uTexelStep'), 0, 1 / heightPx)
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4)
    return true
  }

  #composite(
    gl: ResidentGl,
    programName: ResidentProgramName,
    surface: SurfaceRecord,
    uniforms: Record<string, number>,
  ): boolean {
    const record = this.#programs.get(programName)

    if (record === undefined) {
      return false
    }

    const { widthPx, heightPx } = surface.spec

    // 합성 결과를 읽고 있는 텍스처에 바로 쓰면 정의되지 않은 동작이다.
    // 임시 버퍼로 쓰고 다시 되돌린다.
    gl.bindFramebuffer(gl.FRAMEBUFFER, surface.framebuffers[2])
    gl.viewport(0, 0, widthPx, heightPx)
    gl.useProgram(record.program)

    gl.activeTexture(gl.TEXTURE0)
    gl.bindTexture(gl.TEXTURE_2D, surface.textures[1])
    gl.uniform1i(record.uniforms.get('uSource'), 0)

    gl.activeTexture(gl.TEXTURE0 + 1)
    gl.bindTexture(gl.TEXTURE_2D, surface.textures[3])
    gl.uniform1i(
      record.uniforms.get(programName === 'sharpen' ? 'uBlurred' : 'uBloom'),
      1,
    )

    for (const [name, value] of Object.entries(uniforms)) {
      gl.uniform1f(record.uniforms.get(name), value)
    }

    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4)

    const blit = this.#programs.get('blit')

    if (blit === undefined) {
      return false
    }

    gl.bindFramebuffer(gl.FRAMEBUFFER, surface.framebuffers[1])
    gl.useProgram(blit.program)
    gl.activeTexture(gl.TEXTURE0)
    gl.bindTexture(gl.TEXTURE_2D, surface.textures[2])
    gl.uniform1i(blit.uniforms.get('uSource'), 0)
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4)
    return true
  }

  #buildProgram(gl: ResidentGl, name: ResidentProgramName): ProgramRecord | null {
    if (this.#renderInFlight) {
      // 여기 도달했다는 것은 hot path가 자원을 만들려 했다는 뜻이다.
      // 세어 두고 실패시킨다. 조용히 컴파일하면 측정값이 거짓말을 한다.
      this.#hotPathProgramCompileCount += 1
      return null
    }

    const vertex = this.#compileShader(gl, gl.VERTEX_SHADER, RESIDENT_VERTEX_SHADER)
    const fragment = this.#compileShader(
      gl,
      gl.FRAGMENT_SHADER,
      RESIDENT_PROGRAM_SOURCES[name],
    )

    if (vertex === null || fragment === null) {
      return null
    }

    const program = gl.createProgram()

    if (program === null) {
      return null
    }

    gl.attachShader(program, vertex)
    gl.attachShader(program, fragment)
    gl.linkProgram(program)
    gl.deleteShader(vertex)
    gl.deleteShader(fragment)

    if (gl.getProgramParameter(program, gl.LINK_STATUS) !== true) {
      gl.deleteProgram(program)
      return null
    }

    const uniforms = new Map<string, unknown>()

    for (const uniformName of collectUniformNames(
      RESIDENT_PROGRAM_SOURCES[name],
    )) {
      uniforms.set(uniformName, gl.getUniformLocation(program, uniformName))
    }

    return { program, uniforms }
  }

  #compileShader(gl: ResidentGl, type: number, source: string): unknown {
    const shader = gl.createShader(type)

    if (shader === null) {
      return null
    }

    this.#totalProgramCompileCount += 1
    gl.shaderSource(shader, source)
    gl.compileShader(shader)

    if (gl.getShaderParameter(shader, gl.COMPILE_STATUS) !== true) {
      gl.deleteShader(shader)
      return null
    }

    return shader
  }

  /**
   * 출력 surface를 미리 잡는다.
   *
   * 4장이 필요하다: 입력, 작업 결과, 임시, blur. 촬영 중에 늘리지 않는다.
   */
  #allocateSurface(
    gl: ResidentGl,
    textures: ResidentGlTextures,
    spec: ResidentSurfaceSpec,
  ): void {
    this.#releaseSurface(gl)

    const textureHandles: unknown[] = []
    const framebuffers: unknown[] = []

    for (let index = 0; index < 4; index += 1) {
      const texture = gl.createTexture()
      gl.bindTexture(gl.TEXTURE_2D, texture)
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR)
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR)
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE)
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE)
      textures.allocate(gl, spec.widthPx, spec.heightPx)

      const framebuffer = gl.createFramebuffer()
      gl.bindFramebuffer(gl.FRAMEBUFFER, framebuffer)
      gl.framebufferTexture2D(
        gl.FRAMEBUFFER,
        gl.COLOR_ATTACHMENT0,
        gl.TEXTURE_2D,
        texture,
        0,
      )

      textureHandles.push(texture)
      framebuffers.push(framebuffer)
    }

    gl.bindFramebuffer(gl.FRAMEBUFFER, null)

    this.#surface = { spec, textures: textureHandles, framebuffers }
  }

  #releaseSurface(gl: ResidentGl): void {
    if (this.#surface === null) {
      return
    }

    for (const framebuffer of this.#surface.framebuffers) {
      gl.deleteFramebuffer(framebuffer)
    }

    for (const texture of this.#surface.textures) {
      gl.deleteTexture(texture)
    }

    this.#surface = null
  }
}

export function planKey(presetId: string, presetVersion: string): string {
  return `${presetId}@${presetVersion}`
}

/** shader 원문에서 uniform 이름을 뽑는다. 손으로 적은 목록이 원문과 어긋나는 것을 막는다. */
export function collectUniformNames(source: string): string[] {
  const names = new Set<string>()
  const pattern = /uniform\s+\w+\s+(\w+)\s*;/g
  let match = pattern.exec(source)

  while (match !== null) {
    names.add(match[1])
    match = pattern.exec(source)
  }

  return [...names]
}
