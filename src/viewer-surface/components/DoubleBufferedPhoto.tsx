import { useEffect, useRef, useState } from 'react'

import type {
  DisplayGeneration,
  DisplayPresentReport,
  DisplayPresentSpans,
} from '../../shared-contracts'
import { resolveDisplayAssetSrc } from '../services/display-asset-src'
import { isDisplayFitSource } from '../services/display-fit'
import {
  QUALIFYING_FRAME_ELEMENT_TIMING,
  stampAfterPaint,
} from '../telemetry/after-paint'
import { ceilMicros, floorMicros } from '../telemetry/present-clock'

/**
 * 두 레이어를 겹쳐 두고 **완전히 decode된 뒤에만** 앞뒤를 바꾼다.
 *
 * opaque swap 계약:
 * - 두 레이어는 같은 box, 같은 `object-fit`을 쓴다. 하나라도 다르면 scale jump가 생긴다.
 * - transition / opacity fade / spinner / placeholder를 쓰지 않는다. 나가는 이미지는
 *   한 프레임도 반투명해지지 않는다.
 * - 다음 이미지가 decode를 끝내기 전까지 현재 이미지를 절대 내리지 않는다.
 *
 * 이전 레이어의 `src`를 비우지 않는 이유: 레이어가 정확히 두 개이므로 다음 교체에서 어차피
 * 덮어써진다. 중간에 비우면 잠깐 빈 레이어가 생길 위험만 늘고 얻는 것이 없다.
 */
export type DoubleBufferedPhotoProps = {
  generation: DisplayGeneration | null
  requiredSourceWidthPx: number
  requiredSourceHeightPx: number
  /** 고객 안전 문구 하나. 진단 용어를 넣지 않는다. */
  alt?: string
  onPresented?(report: DisplayPresentReport): void
  onRejected?(report: DisplayPresentReport): void
  resolveSrc?: (assetPath: string) => string
  stampAfterPaintFn?: typeof stampAfterPaint
}

type LayerState = {
  src: string
  generationId: string
}

const EMPTY_LAYER: LayerState = { src: '', generationId: '' }

function buildSpans(overrides: Partial<DisplayPresentSpans>): DisplayPresentSpans {
  return {
    viewerReceiptAtMicros: null,
    decodeStartAtMicros: null,
    decodeEndAtMicros: null,
    swapCommittedAtMicros: null,
    imgOnLoadAtMicros: null,
    actualPresentAtMicros: null,
    elementTimingRenderAtMicros: null,
    isElementRenderTime: false,
    ...overrides,
  }
}

export function DoubleBufferedPhoto({
  generation,
  requiredSourceWidthPx,
  requiredSourceHeightPx,
  alt = '방금 촬영한 사진',
  onPresented,
  onRejected,
  resolveSrc = resolveDisplayAssetSrc,
  stampAfterPaintFn = stampAfterPaint,
}: DoubleBufferedPhotoProps) {
  const [layers, setLayers] = useState<[LayerState, LayerState]>([
    EMPTY_LAYER,
    EMPTY_LAYER,
  ])
  const [frontIndex, setFrontIndex] = useState(0)
  const frontIndexRef = useRef(0)
  const layerRefs = useRef<[HTMLImageElement | null, HTMLImageElement | null]>([
    null,
    null,
  ])
  const imgLoadMicrosRef = useRef<number | null>(null)
  /** 현재 화면에 실제로 올라간 generation. decode가 끝나기 전에는 갱신하지 않는다. */
  const presentedGenerationIdRef = useRef<string>('')
  const pendingGenerationIdRef = useRef<string>('')
  /**
   * swap을 커밋한 뒤 after-paint 스탬프를 기다리는 generation들.
   *
   * **effect 정리 시점에 취소하지 않는다.** 교체는 이미 일어났고 프레임은 곧 표시된다.
   * 여기서 취소하면 다음 generation이나 photo rect 변화가 직전 표본의 terminal 보고를
   * 조용히 지운다 (HV-13B 2026-08-12 회차에서 10건 중 1건이 이렇게 사라졌다).
   * 취소는 언마운트에서만 한다.
   */
  const pendingStampsRef = useRef<Map<string, () => void>>(new Map())

  useEffect(
    () => () => {
      for (const cancel of pendingStampsRef.current.values()) {
        cancel()
      }

      pendingStampsRef.current.clear()
    },
    [],
  )

  // 새 generation이 오면 뒤 레이어에만 실어 둔다. 앞 레이어는 그대로 보인다.
  useEffect(() => {
    if (generation === null) {
      presentedGenerationIdRef.current = ''
      pendingGenerationIdRef.current = ''
      imgLoadMicrosRef.current = null
      setLayers([EMPTY_LAYER, EMPTY_LAYER])
      frontIndexRef.current = 0
      setFrontIndex(0)
      return
    }

    if (
      generation.generationId === presentedGenerationIdRef.current ||
      generation.generationId === pendingGenerationIdRef.current
    ) {
      return
    }

    pendingGenerationIdRef.current = generation.generationId
    const backIndex = frontIndexRef.current === 0 ? 1 : 0

    setLayers((current) => {
      const next: [LayerState, LayerState] = [current[0], current[1]]
      next[backIndex] = {
        src: resolveSrc(generation.assetPath),
        generationId: generation.generationId,
      }

      return next
    })
  }, [generation, resolveSrc])

  // 뒤 레이어가 DOM에 반영된 뒤에 decode를 기다린다.
  useEffect(() => {
    if (generation === null) {
      return
    }

    const backIndex = frontIndexRef.current === 0 ? 1 : 0
    const backLayer = layers[backIndex]

    if (backLayer.generationId !== generation.generationId) {
      return
    }

    const element = layerRefs.current[backIndex]

    if (element === null) {
      return
    }

    let isDisposed = false
    const decodeStartAtMicros = floorMicros(performance.now())

    const report = (
      outcome: DisplayPresentReport['outcome'],
      rejectReason: DisplayPresentReport['rejectReason'],
      spans: DisplayPresentSpans,
      element: HTMLImageElement | null,
    ): DisplayPresentReport => ({
      generationId: generation.generationId,
      viewerEpoch: generation.viewerEpoch,
      outcome,
      rejectReason,
      naturalWidthPx: element?.naturalWidth ?? 0,
      naturalHeightPx: element?.naturalHeight ?? 0,
      spans,
      // 보정값은 상위(useDisplayPointer)가 채워 넣는다.
      clockOffsetMicros: 0,
      clockUncertaintyMicros: 0,
    })

    // decode()를 쓸 수 없는 런타임에서는 **교체하지 않는다.**
    // 완전 decode를 증명하지 못한 자산을 올리는 것은 opaque swap 계약 위반이다.
    const decodePromise =
      typeof element.decode === 'function'
        ? element.decode()
        : Promise.reject(new Error('image-decode-unavailable'))

    void decodePromise
      .then(() => {
        if (isDisposed) {
          return
        }

        const decodeEndAtMicros = ceilMicros(performance.now())

        // host가 통과시켰더라도 contain 표시에서 확대가 필요하면 여기서 다시 막는다.
        if (!isDisplayFitSource(
          { naturalWidth: element.naturalWidth, naturalHeight: element.naturalHeight },
          { requiredSourceWidthPx, requiredSourceHeightPx },
        )) {
          pendingGenerationIdRef.current = ''
          onRejected?.(
            report(
              'rejected',
              'insufficient-dimensions',
              buildSpans({
                decodeStartAtMicros,
                decodeEndAtMicros,
                imgOnLoadAtMicros: imgLoadMicrosRef.current,
              }),
              element,
            ),
          )

          return
        }

        // 여기서 한 번의 style commit으로 앞뒤가 바뀐다. 중간 상태가 없다.
        frontIndexRef.current = backIndex
        setFrontIndex(backIndex)
        presentedGenerationIdRef.current = generation.generationId
        pendingGenerationIdRef.current = ''
        const swapCommittedAtMicros = ceilMicros(performance.now())
        const stampKey = generation.generationId

        if (pendingStampsRef.current.has(stampKey)) {
          return
        }

        // 여기서 만든 스탬프는 effect 정리와 수명을 공유하지 않는다.
        // 교체가 커밋된 이상 이 표본의 terminal 보고는 반드시 나가야 한다.
        const cancelStamp = stampAfterPaintFn((nowMs) => {
          pendingStampsRef.current.delete(stampKey)

          onPresented?.(
            report(
              'presented',
              null,
              buildSpans({
                decodeStartAtMicros,
                decodeEndAtMicros,
                swapCommittedAtMicros,
                imgOnLoadAtMicros: imgLoadMicrosRef.current,
                actualPresentAtMicros: ceilMicros(nowMs),
              }),
              element,
            ),
          )
        })

        pendingStampsRef.current.set(stampKey, cancelStamp)
      })
      .catch(() => {
        if (isDisposed) {
          return
        }

        // 현재 이미지를 유지한 채 실패만 보고한다. 빈 화면으로 떨어지지 않는다.
        pendingGenerationIdRef.current = ''
        onRejected?.(
          report(
            'decode-failed',
            'decode-failed',
            buildSpans({
              decodeStartAtMicros,
              decodeEndAtMicros: ceilMicros(performance.now()),
            }),
            element,
          ),
        )
      })

    // decode 단계만 정리한다. 커밋된 교체의 after-paint 스탬프는 언마운트에서만 취소된다.
    return () => {
      isDisposed = true
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [generation, layers, requiredSourceWidthPx, requiredSourceHeightPx])

  return (
    <>
      {[0, 1].map((index) => {
        const layer = layers[index]
        const isFront = index === frontIndex

        return (
          <img
            key={index}
            ref={(element) => {
              layerRefs.current[index as 0 | 1] = element
            }}
            className="viewer-photo__layer"
            data-front={isFront ? 'true' : 'false'}
            // 빈 문자열 src는 브라우저가 현재 문서를 다시 내려받게 만든다.
            src={layer.src === '' ? undefined : layer.src}
            alt={generation !== null && isFront && layer.src !== '' ? alt : ''}
            elementtiming={QUALIFYING_FRAME_ELEMENT_TIMING}
            decoding="sync"
            loading="eager"
            fetchPriority="high"
            draggable={false}
            onLoad={() => {
              imgLoadMicrosRef.current = ceilMicros(performance.now())
            }}
            style={{
              position: 'absolute',
              inset: 0,
              width: '100%',
              height: '100%',
              objectFit: 'contain',
              // 두 레이어 모두 완전 불투명하다. z-index만 바꿔 교체한다.
              zIndex: isFront ? 2 : 1,
              visibility:
                generation === null || layer.src === '' ? 'hidden' : 'visible',
            }}
          />
        )
      })}
    </>
  )
}
