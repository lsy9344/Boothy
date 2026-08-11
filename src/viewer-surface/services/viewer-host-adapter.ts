import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

import {
  hostErrorEnvelopeSchema,
  viewerLayoutReportSchema,
  viewerReadinessSnapshotSchema,
  viewerReadinessUpdateEvent,
  viewerReadinessUpdateSchema,
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
