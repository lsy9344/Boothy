/**
 * Story 7.5. 관람 창의 수명주기에 상주 renderer를 붙인다.
 *
 * 목적은 하나다: **촬영이 시작되기 전에 준비를 끝내는 것.** 관람 창은 Story 7.1이
 * 촬영보다 먼저 띄우도록 만들었으므로, 그 창이 사진 자리 크기를 보고한 순간이
 * 상주 context를 세울 수 있는 가장 이른 시점이다.
 *
 * **기본은 아무것도 하지 않는 것이다.** mode가 `off`(무설정 포함)이면 context조차
 * 만들지 않는다. HV-16이 `Go`를 기록해도 Story 7.6 전에는 이 기본값을 바꾸지 않는다.
 */

import { useEffect, useRef, useState } from 'react'

import {
  Webgl2ResidentEngine,
  type ResidentEngineStatus,
} from '../engine/webgl2-resident-engine'
import type { ResidentRecipe } from '../engine/resident-recipe'
import { createBrowserResidentContext } from '../services/browser-resident-context'
import { createResidentRendererHostService } from '../services/resident-renderer-host-adapter'

export type ResidentRendererPlan = {
  readonly schemaVersion: string
  readonly mode: 'off' | 'shadow' | 'evidence'
  readonly engine: string
  readonly engineVersion: string
  readonly recipes: readonly ResidentRecipe[]
  readonly refusals: readonly {
    readonly presetId: string
    readonly presetVersion: string
    readonly reason: string
  }[]
}

export type ResidentRendererHostService = {
  getResidentRendererPlan(): Promise<ResidentRendererPlan>
}

export type ResidentSurfaceInput = {
  readonly requiredSourceWidthPx: number
  readonly requiredSourceHeightPx: number
  readonly devicePixelRatio: number
  readonly displayProfileId: string
}

export type ResidentRendererState = {
  readonly mode: 'off' | 'shadow' | 'evidence'
  readonly status: ResidentEngineStatus
  readonly engine: Webgl2ResidentEngine | null
  readonly readyPresetKeys: readonly string[]
  readonly refusals: ResidentRendererPlan['refusals']
}

const IDLE: ResidentRendererState = {
  mode: 'off',
  status: 'uninitialized',
  engine: null,
  readyPresetKeys: [],
  refusals: [],
}

export type UseResidentRendererOptions = {
  readonly surface: ResidentSurfaceInput | null
  readonly viewerEpoch: number
  readonly hostService?: ResidentRendererHostService
  readonly nowMicros?: () => number
}

/**
 * 관람 창이 사는 동안 상주 엔진을 유지한다.
 *
 * 이 hook은 **렌더를 트리거하지 않는다.** 준비 상태만 소유한다. 실제 렌더는
 * HV-16 실행 도구가 `state.engine`을 통해 명시적으로 호출한다 — 실험이 제품 촬영
 * 경로에 스스로 끼어들지 않게 하려는 것이다.
 */
export function useResidentRenderer({
  surface,
  viewerEpoch,
  hostService,
  nowMicros,
}: UseResidentRendererOptions): ResidentRendererState {
  const [state, setState] = useState<ResidentRendererState>(IDLE)
  const engineRef = useRef<Webgl2ResidentEngine | null>(null)
  const contextRef = useRef<ReturnType<
    typeof createBrowserResidentContext
  > | null>(null)

  // 호출자는 보통 `{ getResidentRendererPlan }` 같은 객체 리터럴을 넘긴다.
  // 그 객체를 아래 effect의 의존성에 두면 **매 렌더마다 context를 다시 세운다** —
  // 관람 창이 살아 있는 내내 GPU 자원을 만들고 버리는 루프가 된다.
  // 서비스는 앱에서 모듈 싱글턴이므로 신원이 아니라 최신 참조만 붙잡는다.
  //
  // `useRef` 초기값이 첫 렌더의 값이므로 mount 시점에는 이미 올바르다.
  // 이후 변경만 아래 effect가 따라잡는다 — 렌더 중에 ref를 쓰면 React 규칙 위반이다.
  const hostServiceRef = useRef(hostService)
  const nowMicrosRef = useRef(nowMicros)

  useEffect(() => {
    hostServiceRef.current = hostService
    nowMicrosRef.current = nowMicros
  }, [hostService, nowMicros])

  const widthPx = surface?.requiredSourceWidthPx ?? 0
  const heightPx = surface?.requiredSourceHeightPx ?? 0
  const devicePixelRatio = surface?.devicePixelRatio ?? 0
  const displayProfileId = surface?.displayProfileId ?? ''

  useEffect(() => {
    const hostService =
      hostServiceRef.current ?? createResidentRendererHostService()

    // viewer가 사진 자리를 아직 보고하지 않았으면 준비할 대상 크기가 없다.
    // 이 상태에서 context를 세우면 곧바로 다시 세워야 한다.
    if (widthPx === 0 || heightPx === 0) {
      return
    }

    let cancelled = false

    const prepare = async () => {
      let plan: ResidentRendererPlan

      try {
        plan = await hostService.getResidentRendererPlan()
      } catch {
        // 계획을 못 받는 것은 제품 실패가 아니다. 상주 경로가 없을 뿐이다.
        if (!cancelled) {
          setState(IDLE)
        }
        return
      }

      if (cancelled) {
        return
      }

      if (plan.mode === 'off') {
        // context조차 만들지 않는다. 꺼진 실험이 GPU 자원을 잡고 있으면 안 된다.
        setState({ ...IDLE, refusals: plan.refusals })
        return
      }

      const context = createBrowserResidentContext(widthPx, heightPx)

      if (context === null) {
        setState({
          mode: plan.mode,
          status: 'unavailable',
          engine: null,
          readyPresetKeys: [],
          refusals: plan.refusals,
        })
        return
      }

      contextRef.current = context

      const engine = new Webgl2ResidentEngine({
        createContext: () => ({ gl: context.gl, textures: context.textures }),
        nowMicros: nowMicrosRef.current ?? defaultNowMicros,
        gpuVendor: context.gpuVendor,
        gpuRenderer: context.gpuRenderer,
      })

      const status = engine.prewarm(plan.recipes, {
        widthPx,
        heightPx,
        devicePixelRatio,
        displayProfileId,
      })

      engineRef.current = engine

      if (cancelled) {
        return
      }

      setState({
        mode: plan.mode,
        status,
        engine,
        readyPresetKeys: plan.recipes
          .filter((recipe) =>
            engine.canRender(recipe.presetId, recipe.presetVersion),
          )
          .map((recipe) => `${recipe.presetId}@${recipe.presetVersion}`),
        refusals: [...plan.refusals, ...engine.refusals],
      })
    }

    void prepare()

    return () => {
      cancelled = true
      engineRef.current?.handleContextLost()
      engineRef.current = null
      contextRef.current = null
    }
    // viewerEpoch가 바뀌면 관람 창이 재생성된 것이다. 이전 context는 죽은 세대에 속한다.
  }, [widthPx, heightPx, devicePixelRatio, displayProfileId, viewerEpoch])

  useEffect(() => {
    const canvas = contextRef.current?.canvas

    if (canvas === undefined) {
      return
    }

    const onLost = (event: Event) => {
      // 기본 동작(복구 불가)을 막아야 `webglcontextrestored`가 온다.
      event.preventDefault()
      engineRef.current?.handleContextLost()
      setState((previous) => ({
        ...previous,
        status: 'context-lost',
        readyPresetKeys: [],
      }))
    }

    canvas.addEventListener('webglcontextlost', onLost)

    return () => {
      canvas.removeEventListener('webglcontextlost', onLost)
    }
  }, [state.engine])

  return widthPx === 0 || heightPx === 0 ? IDLE : state
}

function defaultNowMicros(): number {
  return Math.round(performance.now() * 1000)
}
