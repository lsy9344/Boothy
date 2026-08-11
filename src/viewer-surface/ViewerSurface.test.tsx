import { render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import type { ViewerReadinessSnapshot } from '../shared-contracts'
import { ViewerSurface } from './ViewerSurface'
import type { ViewerHostService } from './services/viewer-host-adapter'

const READY_SNAPSHOT: ViewerReadinessSnapshot = {
  schemaVersion: 'viewer-readiness/v1',
  sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
  viewerEpoch: 1,
  revision: 10,
  windowState: 'open',
  listenerReady: true,
  layoutReady: true,
  monitorTargeting: 'approved-customer-monitor',
  displayProfile: {
    profileId: '1080p',
    monitorName: 'BOOTH-CUSTOMER',
    monitorWidthPx: 1920,
    monitorHeightPx: 1080,
    monitorScaleFactor: 1,
  },
  photoRect: {
    cssWidth: 1620,
    cssHeight: 1080,
    devicePixelRatio: 1,
    requiredSourceWidthPx: 1620,
    requiredSourceHeightPx: 1080,
  },
  viewerReady: true,
  reasonCode: 'viewer-ready',
  observedAtMs: 1_000_000,
  lastReportAtMs: 1_000_000,
}

function createStubHost(
  snapshot: ViewerReadinessSnapshot = READY_SNAPSHOT,
): ViewerHostService {
  return {
    async getViewerReadiness() {
      return snapshot
    },
    async reportListenerReady() {
      return snapshot
    },
    async reportLayout() {
      return snapshot
    },
    async subscribeToViewerReadiness() {
      return () => undefined
    },
  }
}

describe('ViewerSurface read-only 계약', () => {
  it('조작 요소를 전혀 렌더링하지 않는다', async () => {
    render(<ViewerSurface hostService={createStubHost()} />)

    await waitFor(() => {
      expect(screen.getByRole('main')).toBeInTheDocument()
    })

    for (const role of [
      'button',
      'link',
      'textbox',
      'checkbox',
      'radio',
      'menuitem',
      'combobox',
      'slider',
      'tab',
      'switch',
    ] as const) {
      expect(screen.queryAllByRole(role)).toHaveLength(0)
    }
  })

  it('포커스 가능한 요소를 두지 않는다', async () => {
    const { container } = render(<ViewerSurface hostService={createStubHost()} />)

    await waitFor(() => {
      expect(screen.getByRole('main')).toBeInTheDocument()
    })

    expect(
      container.querySelectorAll(
        'a[href], button, input, select, textarea, [tabindex], [contenteditable]',
      ),
    ).toHaveLength(0)
  })

  it('고객에게 진단 용어나 내부 상태를 노출하지 않는다', async () => {
    const { container } = render(
      <ViewerSurface
        hostService={createStubHost({
          ...READY_SNAPSHOT,
          viewerReady: false,
          reasonCode: 'monitor-not-approved',
          monitorTargeting: 'monitor-unavailable',
        })}
      />,
    )

    await waitFor(() => {
      expect(screen.getByRole('main')).toBeInTheDocument()
    })

    const visibleText = container.textContent ?? ''

    for (const forbidden of [
      'epoch',
      'revision',
      'monitor',
      'listener',
      'layout',
      'session',
      'DPR',
      'snapshot',
      'reasonCode',
      'viewer-ready',
      'stale',
    ]) {
      expect(visibleText.toLowerCase()).not.toContain(forbidden.toLowerCase())
    }
  })

  it('준비 실패를 관람 화면에서 에러로 표시하지 않고 차분한 대기 문구만 유지한다', async () => {
    render(
      <ViewerSurface
        hostService={createStubHost({
          ...READY_SNAPSHOT,
          viewerReady: false,
          reasonCode: 'viewer-window-closed',
        })}
      />,
    )

    await waitFor(() => {
      expect(
        screen.getByText('사진이 준비되면 여기에 보여드릴게요.'),
      ).toBeInTheDocument()
    })

    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
  })

  it('사진이 놓일 자리를 항상 유지한다', async () => {
    const { container } = render(<ViewerSurface hostService={createStubHost()} />)

    await waitFor(() => {
      expect(screen.getByRole('main')).toBeInTheDocument()
    })

    expect(container.querySelector('.viewer-surface__photo')).not.toBeNull()
  })
})
