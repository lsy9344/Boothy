import { describe, expect, it } from 'vitest'

import { mapCameraErrorToViewModel } from '../../src/capture-adapter/host/cameraErrorMapping.js'
import { normalizedErrorEnvelopeSchema } from '../../src/shared-contracts/dto/errorEnvelope.js'
import { schemaVersions } from '../../src/shared-contracts/dto/schemaVersion.js'

describe('normalized camera error envelope', () => {
  it('drives both customer-safe copy and operator diagnostics from the same host envelope', () => {
    const envelope = normalizedErrorEnvelopeSchema.parse({
      schemaVersion: schemaVersions.errorEnvelope,
      code: 'camera.reconnecting',
      severity: 'warning',
      retryable: true,
      customerState: 'cameraReconnectNeeded',
      customerCameraConnectionState: 'needsAttention',
      operatorCameraConnectionState: 'reconnecting',
      operatorAction: 'checkCableAndRetry',
      message: 'Camera connection is unstable.',
      details: 'PTP session reopened after an adapter timeout.',
    })

    expect(mapCameraErrorToViewModel(envelope)).toEqual({
      customer: {
        state: 'cameraReconnectNeeded',
        connectionState: 'needsAttention',
        message: '카메라 연결을 다시 확인하고 있어요.',
      },
      operator: {
        connectionState: 'reconnecting',
        action: 'checkCableAndRetry',
        message: 'Camera connection is unstable.',
        details: 'PTP session reopened after an adapter timeout.',
      },
    })
  })

  it('never leaks raw sidecar diagnostics into the customer-facing message', () => {
    const envelope = normalizedErrorEnvelopeSchema.parse({
      schemaVersion: schemaVersions.errorEnvelope,
      code: 'camera.busy',
      severity: 'error',
      retryable: false,
      customerState: 'cameraUnavailable',
      customerCameraConnectionState: 'offline',
      operatorCameraConnectionState: 'disconnected',
      operatorAction: 'restartHelper',
      message: 'Camera helper stopped responding.',
      details: 'PTP_USB_BUSY',
    })

    const viewModel = mapCameraErrorToViewModel(envelope)

    expect(viewModel.customer.message).toBe('지금은 촬영 준비를 계속할 수 없어요. 직원 호출을 부탁드릴게요.')
    expect(viewModel.customer.message).not.toContain('PTP_USB_BUSY')
    expect(viewModel.operator.details).toBe('PTP_USB_BUSY')
  })
})
