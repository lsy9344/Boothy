import 'react'

/**
 * Element Timing API의 `elementtiming` 속성. React의 기본 타입에는 없다.
 *
 * Story 7.2는 이 속성을 **보조 진단**으로만 쓴다. asset protocol이 cross-origin이라
 * `renderTime`이 0이 될 수 있으므로 actual-present로 승격하지 않는다.
 * (`after-paint.ts`의 `observeQualifyingFrameTiming` 참조)
 */
declare module 'react' {
  interface ImgHTMLAttributes<T> extends HTMLAttributes<T> {
    elementtiming?: string
  }
}
