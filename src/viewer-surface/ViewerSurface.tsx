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
  const pendingReportsRef = useRef<
    Array<{ report: DisplayPresentReport; sessionId: string | null }>
  >([])

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
      if (!pointer.measurementLaneEnabled) {
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

        const withoutDuplicate = pendingReportsRef.current.filter(
          (pending) =>
            pending.report.generationId !== report.generationId ||
            pending.report.outcome !== report.outcome,
        )
        pendingReportsRef.current = [
          ...withoutDuplicate,
          { report, sessionId: pointer.sessionId },
        ].slice(-16)
        return
      }

      sendCalibratedReport(report)
    },
    [
      pointer.measurementLaneEnabled,
      pointer.sessionId,
      calibration,
      reportPresent,
      sendCalibratedReport,
    ],
  )

  useEffect(() => {
    if (!pointer.measurementLaneEnabled) {
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
    pointer.measurementLaneEnabled,
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
