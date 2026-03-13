import { describe, expect, it } from 'vitest';

import { selectCustomerCameraStatusCopy } from './customerCameraStatusCopy.js';
import { deriveCustomerPreparationState } from '../../session-domain/state/customerPreparationState.js';

describe('selectCustomerCameraStatusCopy', () => {
  it('returns approved waiting copy from normalized host truth', () => {
    const readinessState = deriveCustomerPreparationState(
      {
        sessionId: 'session-1',
        connectionState: 'waiting',
        captureEnabled: false,
        lastStableCustomerState: null,
        error: null,
        emittedAt: '2026-03-08T00:00:00.000Z',
      },
      '010-1234-5678',
    );

    expect(selectCustomerCameraStatusCopy(readinessState)).toMatchObject({
      kind: 'waiting',
      badge: '잠시 대기',
      title: '아직 촬영할 수 없습니다. 잠시만 기다려 주세요.',
      supporting: '카메라 준비를 다시 확인하고 있습니다.',
      actionHint: '준비가 끝나면 촬영을 시작할 수 있습니다.',
    });
  });

  it('keeps retryable transient polling failures in the waiting state', () => {
    const readinessState = deriveCustomerPreparationState(
      {
        sessionId: 'session-1',
        connectionState: 'waiting',
        captureEnabled: false,
        lastStableCustomerState: 'ready',
        error: {
          schemaVersion: 'boothy.camera.error-envelope.v1',
          code: 'camera.poll.retryable',
          severity: 'warning',
          customerState: 'cameraReconnectNeeded',
          customerCameraConnectionState: 'needsAttention',
          operatorCameraConnectionState: 'reconnecting',
          operatorAction: 'checkCableAndRetry',
          retryable: true,
          message: 'Camera connection is unstable.',
          details: 'Polling call timed out after 500ms',
        },
        emittedAt: '2026-03-08T00:00:03.000Z',
      },
      '010-1234-5678',
    );

    expect(selectCustomerCameraStatusCopy(readinessState)).toMatchObject({
      kind: 'waiting',
      badge: '잠시 대기',
      title: '아직 촬영할 수 없습니다. 잠시만 기다려 주세요.',
    });
  });

  it('returns the approved ready-to-shoot copy when readiness is confirmed', () => {
    const readinessState = deriveCustomerPreparationState(
      {
        sessionId: 'session-1',
        connectionState: 'ready',
        captureEnabled: true,
        lastStableCustomerState: 'ready',
        error: null,
        emittedAt: '2026-03-08T00:00:05.000Z',
      },
      '010-1234-5678',
    );

    expect(selectCustomerCameraStatusCopy(readinessState)).toMatchObject({
      kind: 'ready',
      badge: '촬영 가능',
      title: '카메라가 연결되어 촬영을 시작할 수 있습니다.',
      actionHint: '촬영을 시작할 수 있습니다.',
    });
  });

  it('returns phone-required copy without leaking internal diagnostics', () => {
    const readinessState = deriveCustomerPreparationState(
      {
        sessionId: 'session-1',
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
          details: 'C:/camera-helper/canon.dll missing',
        },
        emittedAt: '2026-03-08T00:02:00.000Z',
      },
      '010-1234-5678',
    );

    expect(selectCustomerCameraStatusCopy(readinessState)).toMatchObject({
      kind: 'phone-required',
      badge: '전화 필요',
      title: '카메라 연결이 확인되지 않습니다. 전화해 주세요.',
      phoneNumber: '010-1234-5678',
    });
    expect(selectCustomerCameraStatusCopy(readinessState).supporting).not.toContain('canon.dll');
    expect(selectCustomerCameraStatusCopy(readinessState).supporting).not.toContain('C:/camera-helper');
  });

  it('keeps a direct phone-required readiness status in the escalation state even without an error envelope', () => {
    const readinessState = deriveCustomerPreparationState(
      {
        sessionId: 'session-1',
        connectionState: 'phone-required',
        captureEnabled: false,
        lastStableCustomerState: null,
        error: null,
        emittedAt: '2026-03-08T00:02:00.000Z',
      },
      '010-1234-5678',
    );

    expect(selectCustomerCameraStatusCopy(readinessState)).toMatchObject({
      kind: 'phone-required',
      phoneNumber: '010-1234-5678',
    });
  });
});
