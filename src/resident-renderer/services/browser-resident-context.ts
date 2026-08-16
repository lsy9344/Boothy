/**
 * Story 7.5. 실제 브라우저(WebView2)에서 상주 WebGL2 context를 만든다.
 *
 * 엔진은 이 파일에 의존하지 않는다. 엔진은 구조적 타입만 알고, 진짜 GPU와 대역이
 * **같은 경로**를 지나간다. 그래야 "hot path에서 컴파일하지 않는다"를 실장비 없이도
 * 회귀 테스트로 지킬 수 있다.
 */

import type {
  ResidentGl,
  ResidentGlTextures,
} from '../engine/webgl2-resident-engine'

export type BrowserResidentContext = {
  readonly gl: ResidentGl
  readonly textures: ResidentGlTextures
  readonly canvas: HTMLCanvasElement
  readonly gpuVendor: string
  readonly gpuRenderer: string
}

/**
 * WebGL2 context와 그 위의 텍스처 도우미를 만든다.
 *
 * **실패는 `null`이다.** 던지지 않는다 — 상주 경로가 없다는 것은 오류가 아니라
 * "정확 darktable 경로로 간다"는 정상 결과다.
 */
export function createBrowserResidentContext(
  widthPx: number,
  heightPx: number,
): BrowserResidentContext | null {
  if (
    typeof document === 'undefined' ||
    !Number.isInteger(widthPx) ||
    !Number.isInteger(heightPx) ||
    widthPx <= 0 ||
    heightPx <= 0
  ) {
    return null
  }

  const canvas = document.createElement('canvas')
  canvas.width = widthPx
  canvas.height = heightPx

  const context = canvas.getContext('webgl2', {
    alpha: false,
    antialias: false,
    depth: false,
    stencil: false,
    // 화면에 직접 그리지 않고 읽어서 host로 넘긴다. 보존이 필요하다.
    preserveDrawingBuffer: true,
    powerPreference: 'high-performance',
  })

  if (context === null) {
    return null
  }

  const debugInfo = context.getExtension('WEBGL_debug_renderer_info')

  return {
    gl: context as unknown as ResidentGl,
    textures: createTextureHelpers(context),
    canvas,
    gpuVendor:
      debugInfo === null
        ? String(context.getParameter(context.VENDOR) ?? 'unknown')
        : String(
            context.getParameter(debugInfo.UNMASKED_VENDOR_WEBGL) ?? 'unknown',
          ),
    gpuRenderer:
      debugInfo === null
        ? String(context.getParameter(context.RENDERER) ?? 'unknown')
        : String(
            context.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL) ?? 'unknown',
          ),
  }
}

/**
 * `texImage2D`와 `readPixels`는 오버로드가 많아 엔진의 좁은 타입에 담기 어렵다.
 * 진짜 context를 닫아 두고 필요한 두 동작만 노출한다.
 */
function createTextureHelpers(
  context: WebGL2RenderingContext,
): ResidentGlTextures {
  return {
    allocate: (_gl, width, height) => {
      context.texImage2D(
        context.TEXTURE_2D,
        0,
        context.RGBA8,
        width,
        height,
        0,
        context.RGBA,
        context.UNSIGNED_BYTE,
        null,
      )
    },
    upload: (_gl, source) => {
      context.texImage2D(
        context.TEXTURE_2D,
        0,
        context.RGBA8,
        context.RGBA,
        context.UNSIGNED_BYTE,
        source,
      )
    },
    readBack: (_gl, width, height) => {
      if (context.isContextLost()) {
        return null
      }

      const pixels = new Uint8Array(width * height * 4)
      context.readPixels(
        0,
        0,
        width,
        height,
        context.RGBA,
        context.UNSIGNED_BYTE,
        pixels,
      )

      return new Uint8ClampedArray(pixels.buffer)
    },
  }
}
