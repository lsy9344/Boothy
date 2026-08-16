import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

import {
  clockProbeResultSchema,
  displayPointerSnapshotSchema,
  displayPresentReportSchema,
  displayUpdateSchema,
  hostErrorEnvelopeSchema,
  viewerDisplayUpdateEvent,
  viewerLayoutReportSchema,
  viewerReadinessSnapshotSchema,
  viewerReadinessUpdateEvent,
  viewerReadinessUpdateSchema,
  type ClockProbeResult,
  type DisplayPointerSnapshot,
  type DisplayPresentReport,
  type ViewerLayoutReport,
  type ViewerReadinessSnapshot,
  type HostErrorEnvelope,
} from '../../shared-contracts'
import { isTauriRuntime } from '../../shared/runtime/is-tauri'

export function normalizeViewerHostError(error: unknown): HostErrorEnvelope {
  const parsed = hostErrorEnvelopeSchema.safeParse(error)

  if (parsed.success) {
    return parsed.data
  }

  return {
    code: 'host-unavailable',
    message: '관람 화면 준비 상태를 다시 확인하고 있어요.',
  }
}

/**
 * viewer surface의 유일한 host 경계. React 컴포넌트는 `invoke`/`listen`을 직접 호출하지 않는다.
 */
export interface ViewerHostService {
  getViewerReadiness(): Promise<ViewerReadinessSnapshot>
  reportListenerReady(input: { viewerEpoch: number }): Promise<ViewerReadinessSnapshot>
  reportLayout(input: ViewerLayoutReport): Promise<ViewerReadinessSnapshot>
  subscribeToViewerReadiness(input: {
    onReadiness(readiness: ViewerReadinessSnapshot): void
  }): Promise<() => void>
}

/**
 * Tauri 밖(브라우저 dev/테스트)에서 쓰는 정직한 미준비 snapshot.
 * 관람 창은 데스크톱 런타임 없이는 존재할 수 없으므로 ready를 주장하지 않는다.
 */
export function buildAbsentViewerReadiness(): ViewerReadinessSnapshot {
  return viewerReadinessSnapshotSchema.parse({
    sessionId: null,
    viewerEpoch: 0,
    revision: 0,
    windowState: 'absent',
    listenerReady: false,
    layoutReady: false,
    monitorTargeting: 'unresolved',
    displayProfile: null,
    photoRect: null,
    viewerReady: false,
    reasonCode: 'viewer-absent',
    observedAtMs: 0,
    lastReportAtMs: null,
  })
}

class TauriViewerHostService implements ViewerHostService {
  async getViewerReadiness() {
    try {
      return viewerReadinessSnapshotSchema.parse(await invoke('get_viewer_readiness'))
    } catch (error) {
      throw normalizeViewerHostError(error)
    }
  }

  async reportListenerReady(input: { viewerEpoch: number }) {
    try {
      return viewerReadinessSnapshotSchema.parse(
        await invoke('report_viewer_listener_ready', { input }),
      )
    } catch (error) {
      throw normalizeViewerHostError(error)
    }
  }

  async reportLayout(input: ViewerLayoutReport) {
    try {
      return viewerReadinessSnapshotSchema.parse(
        await invoke('report_viewer_layout', {
          input: viewerLayoutReportSchema.parse(input),
        }),
      )
    } catch (error) {
      throw normalizeViewerHostError(error)
    }
  }

  async subscribeToViewerReadiness(input: {
    onReadiness(readiness: ViewerReadinessSnapshot): void
  }) {
    try {
      return await listen(viewerReadinessUpdateEvent, (event) => {
        const parsed = viewerReadinessUpdateSchema.safeParse(event.payload)

        if (parsed.success) {
          input.onReadiness(parsed.data.readiness)
        }
      })
    } catch (error) {
      throw normalizeViewerHostError(error)
    }
  }
}

class AbsentViewerHostService implements ViewerHostService {
  async getViewerReadiness() {
    return buildAbsentViewerReadiness()
  }

  async reportListenerReady() {
    return buildAbsentViewerReadiness()
  }

  async reportLayout() {
    return buildAbsentViewerReadiness()
  }

  async subscribeToViewerReadiness() {
    return () => undefined
  }
}

export function createViewerHostService(): ViewerHostService {
  return isTauriRuntime()
    ? new TauriViewerHostService()
    : new AbsentViewerHostService()
}

/**
 * Story 7.2: display pointer 경계. readiness와 같은 규칙을 따른다 —
 * React 컴포넌트는 `invoke`/`listen`을 직접 호출하지 않는다.
 */
export interface DisplayHostService {
  getViewerDisplayState(): Promise<DisplayPointerSnapshot>
  reportDisplayPresent(report: DisplayPresentReport): Promise<void>
  stampClockProbe(input: { clientSentMicros: number }): Promise<ClockProbeResult>
  subscribeToDisplayUpdates(input: {
    onPointer(pointer: DisplayPointerSnapshot): void
  }): Promise<() => void>
}

/** Tauri 밖에서는 표시할 자산이 존재할 수 없다. 정직한 빈 pointer를 돌려준다. */
export function buildAbsentDisplayPointer(): DisplayPointerSnapshot {
  return displayPointerSnapshotSchema.parse({
    sessionId: null,
    revision: 0,
    activeGeneration: null,
    requiredSourceWidthPx: 0,
    requiredSourceHeightPx: 0,
    measurementLaneEnabled: false,
    presentTelemetryEnabled: false,
    observedAtHostMicros: 0,
  })
}

class TauriDisplayHostService implements DisplayHostService {
  async getViewerDisplayState() {
    try {
      return displayPointerSnapshotSchema.parse(
        await invoke('get_viewer_display_state'),
      )
    } catch (error) {
      throw normalizeViewerHostError(error)
    }
  }

  async reportDisplayPresent(report: DisplayPresentReport) {
    try {
      await invoke('report_display_present', {
        input: displayPresentReportSchema.parse(report),
      })
    } catch (error) {
      throw normalizeViewerHostError(error)
    }
  }

  async stampClockProbe() {
    try {
      return clockProbeResultSchema.parse(
        await invoke('stamp_clock_probe'),
      )
    } catch (error) {
      throw normalizeViewerHostError(error)
    }
  }

  async subscribeToDisplayUpdates(input: {
    onPointer(pointer: DisplayPointerSnapshot): void
  }) {
    try {
      return await listen(viewerDisplayUpdateEvent, (event) => {
        const parsed = displayUpdateSchema.safeParse(event.payload)

        if (parsed.success) {
          input.onPointer(parsed.data.pointer)
        }
      })
    } catch (error) {
      throw normalizeViewerHostError(error)
    }
  }
}

class AbsentDisplayHostService implements DisplayHostService {
  async getViewerDisplayState() {
    return buildAbsentDisplayPointer()
  }

  async reportDisplayPresent() {
    return undefined
  }

  async stampClockProbe() {
    return { hostMonotonicMicros: 0, hostEpochMicros: 0 }
  }

  async subscribeToDisplayUpdates() {
    return () => undefined
  }
}

export function createDisplayHostService(): DisplayHostService {
  return isTauriRuntime()
    ? new TauriDisplayHostService()
    : new AbsentDisplayHostService()
}
