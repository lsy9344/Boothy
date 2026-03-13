import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import type { CustomerPreparationState } from '../../session-domain/state/customerPreparationState.js'
import { PreparationScreen } from './PreparationScreen.js'

function renderPreparationScreen(readiness: CustomerPreparationState) {
  render(<PreparationScreen readiness={readiness} sessionName="김보라1234" />)
}

describe('PreparationScreen', () => {
  it('shows the waiting state with a disabled capture action', () => {
    renderPreparationScreen({
      kind: 'waiting',
      captureEnabled: false,
      branchPhoneNumber: '010-1234-5678',
      hostStatus: {
        sessionId: 'session-1',
        connectionState: 'waiting',
        captureEnabled: false,
        lastStableCustomerState: null,
        error: null,
        emittedAt: '2026-03-08T00:00:00.000Z',
      },
    })

    expect(screen.getByRole('heading', { name: '아직 촬영할 수 없습니다. 잠시만 기다려 주세요.' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: '촬영 시작' })).toBeDisabled()
  })

  it('shows the ready-to-shoot message when the readiness state is ready', () => {
    renderPreparationScreen({
      kind: 'ready',
      captureEnabled: true,
      branchPhoneNumber: '010-1234-5678',
      hostStatus: {
        sessionId: 'session-1',
        connectionState: 'ready',
        captureEnabled: true,
        lastStableCustomerState: 'ready',
        error: null,
        emittedAt: '2026-03-08T00:00:05.000Z',
      },
    })

    expect(screen.getByRole('heading', { name: '카메라가 연결되어 촬영을 시작할 수 있습니다.' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: '촬영 시작' })).toBeEnabled()
  })

  it('shows the configured branch phone number and hides diagnostics when phone escalation is required', () => {
    renderPreparationScreen({
      kind: 'phone-required',
      captureEnabled: false,
        branchPhoneNumber: '010-1234-5678',
        hostStatus: {
          sessionId: 'session-2',
          connectionState: 'phone-required',
          captureEnabled: false,
          lastStableCustomerState: null,
        error: {
          schemaVersion: 'boothy.camera.error-envelope.v1',
          code: 'camera.unavailable',
          severity: 'error',
          customerState: 'cameraUnavailable',
          customerCameraConnectionState: 'offline',
          operatorCameraConnectionState: 'disconnected',
          operatorAction: 'contactSupport',
          retryable: false,
          message: 'Camera helper stopped responding.',
          details: 'SDK path C:/camera-helper/canon.dll missing',
        },
        emittedAt: '2026-03-08T00:02:05.000Z',
      },
    })

    expect(screen.getByRole('heading', { name: '카메라 연결이 확인되지 않습니다. 전화해 주세요.' })).toBeInTheDocument()
    expect(screen.getByText('010-1234-5678')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: '촬영 시작' })).not.toBeInTheDocument()
    expect(screen.queryByText(/canon\.dll/i)).not.toBeInTheDocument()
    expect(screen.queryByText(/C:\/camera-helper/i)).not.toBeInTheDocument()
  })
})
