import { useCallback, useEffect, useRef } from 'react'

import { useDisplayPointer } from '../display-generation/state/use-display-pointer'
import {
  DoubleBufferedPhoto,
  type DoubleBufferedPhotoProps,
} from './components/DoubleBufferedPhoto'
import { useViewerReadiness } from './state/use-viewer-readiness'
import type {
  DisplayHostService,
  ViewerHostService,
} from './services/viewer-host-adapter'
import { useHostClockCalibration } from './telemetry/use-host-clock-calibration'
import { toIntegerClockFields } from './telemetry/present-clock'
import type { DisplayPresentReport } from '../shared-contracts'
import {
  enqueuePendingPresentReport,
  type PendingPresentReport,
} from './pending-present-report'
import {
  useResidentRenderer,
  type ResidentRendererHostService,
} from '../resident-renderer/state/use-resident-renderer'

function withoutUncalibratedViewerSpans(
  report: DisplayPresentReport,
): DisplayPresentReport {
  return {
    ...report,
    spans: {
      viewerReceiptAtMicros: null,
      decodeStartAtMicros: null,
      decodeEndAtMicros: null,
      swapCommittedAtMicros: null,
      imgOnLoadAtMicros: null,
      actualPresentAtMicros: null,
      elementTimingRenderAtMicros: null,
      isElementRenderTime: false,
    },
  }
}

type ViewerSurfaceProps = {
  hostService?: ViewerHostService
  displayHostService?: DisplayHostService
  stampAfterPaintFn?: DoubleBufferedPhotoProps['stampAfterPaintFn']
  /** Story 7.5 spike. 기본은 `off`이므로 주입하지 않으면 아무 일도 하지 않는다. */
  residentRendererHostService?: ResidentRendererHostService
}

/**
 * 전용 관람 화면.
 *
 * Story 7.1이 사진 자리(photo rectangle)와 필요한 픽셀 수를 계약으로 확정했고,
 * Story 7.2가 그 자리에 immutable generation을 opaque double-buffer로 올린다.
 *
 * read-only 계약: 조작 요소(button/link/textbox/menuitem)와 진단 표시가 없어야 한다.
 * 준비 실패와 표시 실패는 booth control surface의 wait/call guidance로만 전달한다.
 *
 * **레이아웃 불변식:** `.viewer-surface__photo`의 기하를 바꾸지 않는다. `use-viewer-readiness`가
 * 이 요소를 실측해 host 계약과 1px 허용오차로 비교하며, 어긋나면 촬영이 막힌다.
 * standby 문구도 `display: none`으로 없애지 않는다 — 행이 사라지면 photo rect가 커져
 * layout-ready가 흔들리고 사진에 scale jump가 생긴다.
 */
export function ViewerSurface({
  hostService,
  displayHostService,
  stampAfterPaintFn,
  residentRendererHostService,
}: ViewerSurfaceProps) {
  const stageRef = useRef<HTMLDivElement>(null)
  const photoRef = useRef<HTMLDivElement>(null)

  const readiness = useViewerReadiness({ stageRef, photoRef, hostService })
  const { pointer, generation, reportPresent } = useDisplayPointer({
    readiness,
    hostService: displayHostService,
  })
  const calibration = useHostClockCalibration({
    viewerEpoch: readiness.viewerEpoch,
    hostService: displayHostService,
  })

  const requiredSourceWidthPx = readiness.photoRect?.requiredSourceWidthPx ?? 0
  const requiredSourceHeightPx = readiness.photoRect?.requiredSourceHeightPx ?? 0
  const pendingReportsRef = useRef<PendingPresentReport[]>([])

  /**
   * Story 7.5 spike. **관람 창이 사진 자리를 보고한 순간이 촬영보다 앞선 가장 이른 시점**이고,
   * 상주 renderer가 준비될 수 있는 유일한 자리다.
   *
   * 기본 mode는 `off`이므로 제품 동작은 바뀌지 않는다. `off`에서는 GPU context조차 만들지 않고,
   * 이 hook은 렌더를 트리거하지도 않는다 — 실험이 제품 촬영 경로에 스스로 끼어들지 않게 하려는 것이다.
   */
  useResidentRenderer({
    surface:
      requiredSourceWidthPx === 0 || requiredSourceHeightPx === 0
        ? null
        : {
            requiredSourceWidthPx,
            requiredSourceHeightPx,
            devicePixelRatio: readiness.photoRect?.devicePixelRatio ?? 1,
            displayProfileId: readiness.displayProfile?.profileId ?? 'unknown',
          },
    viewerEpoch: readiness.viewerEpoch,
    hostService: residentRendererHostService,
  })

  const sendCalibratedReport = useCallback(
    (report: DisplayPresentReport) => {
      if (calibration === null) {
        return
      }

      void reportPresent({
        ...report,
        ...toIntegerClockFields(calibration),
      })
    },
    [reportPresent, calibration],
  )

  // 보정값은 계측 경계에서만 채운다. 컴포넌트는 span만 만들고 변환은 host가 한다.
  const submitReport = useCallback(
    (report: DisplayPresentReport) => {
      if (!pointer.presentTelemetryEnabled) {
        return
      }

      if (calibration === null) {
        if (report.outcome !== 'presented') {
          void reportPresent({
            ...withoutUncalibratedViewerSpans(report),
            clockOffsetMicros: 0,
            clockUncertaintyMicros: Number.MAX_SAFE_INTEGER,
          })
          return
        }

        pendingReportsRef.current = enqueuePendingPresentReport(
          pendingReportsRef.current,
          { report, sessionId: pointer.sessionId },
        )
        return
      }

      sendCalibratedReport(report)
    },
    [
      pointer.presentTelemetryEnabled,
      pointer.sessionId,
      calibration,
      reportPresent,
      sendCalibratedReport,
    ],
  )

  useEffect(() => {
    if (!pointer.presentTelemetryEnabled) {
      pendingReportsRef.current = []
      return
    }

    if (calibration === null) {
      return
    }

    const pending = pendingReportsRef.current
    pendingReportsRef.current = []

    for (const item of pending) {
      if (
        item.sessionId === pointer.sessionId &&
        item.report.viewerEpoch === readiness.viewerEpoch
      ) {
        sendCalibratedReport(item.report)
      }
    }
  }, [
    calibration,
    pointer.presentTelemetryEnabled,
    pointer.sessionId,
    readiness.viewerEpoch,
    sendCalibratedReport,
  ])

  const hasPhoto = generation !== null

  return (
    <main className="viewer-surface">
      <div className="viewer-surface__stage" ref={stageRef}>
        <div className="viewer-surface__photo" ref={photoRef}>
          <DoubleBufferedPhoto
            generation={generation}
            requiredSourceWidthPx={requiredSourceWidthPx}
            requiredSourceHeightPx={requiredSourceHeightPx}
            onPresented={submitReport}
            onRejected={submitReport}
            stampAfterPaintFn={stampAfterPaintFn}
          />
        </div>
      </div>
      {/*
        사진이 보이는 동안에도 이 행의 레이아웃 점유를 유지한다.
        `display: none`으로 바꾸면 stage 높이 → photo rect가 바뀌어 촬영이 막히고 scale jump가 난다.
      */}
      <p
        className="viewer-surface__standby"
        style={{ visibility: hasPhoto ? 'hidden' : 'visible' }}
        aria-hidden={hasPhoto}
      >
        사진이 준비되면 여기에 보여드릴게요.
      </p>
    </main>
  )
}
