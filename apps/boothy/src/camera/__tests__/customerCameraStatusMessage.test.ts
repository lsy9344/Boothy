import { describe, expect, it } from 'vitest';

import type { BoothyCameraStatusReport, BoothyCameraStatusSnapshot } from '../../components/ui/AppProperties';
import {
  CAMERA_MESSAGE_PREPARING,
  CAMERA_MESSAGE_UNAVAILABLE,
  getCustomerCameraStatusMessage,
} from '../customerCameraStatusMessage';

function makeStatus(overrides?: Partial<BoothyCameraStatusReport>): BoothyCameraStatusReport {
  return {
    ipcState: 'connected',
    protocolVersion: '1.0.0',
    status: {
      connected: true,
      cameraDetected: true,
      cameraModel: 'EOS',
      sessionDestination: null,
    },
    ...overrides,
  };
}

function makeSnapshot(overrides?: Partial<BoothyCameraStatusSnapshot>): BoothyCameraStatusSnapshot {
  return {
    seq: 1,
    observedAt: new Date().toISOString(),
    reason: 'test',
    mode: 'real',
    sdk: { initialized: true, diagnostic: null, resolvedPath: null, platform: 'win32' },
    state: 'ready',
    connected: true,
    cameraDetected: true,
    cameraReady: true,
    cameraCount: 1,
    cameraModel: 'EOS',
    ...overrides,
  };
}

describe('getCustomerCameraStatusMessage', () => {
  it('hides unavailable/preparing message while lamp is connected (green)', () => {
    const message = getCustomerCameraStatusMessage({
      isCustomerMode: true,
      hasBoothySession: true,
      customerCameraConnectionStateForLamp: 'connected',
      cameraStatus: makeStatus(),
      cameraStatusSnapshotFresh: makeSnapshot({ state: 'ready' }),
      cameraStatusLoading: false,
      isCameraReconnecting: false,
      cameraStatusError: 'temporary getStatus failure',
    });

    expect(message).toBeNull();
  });

  it('treats transient status error as preparing when lamp is disconnected', () => {
    const message = getCustomerCameraStatusMessage({
      isCustomerMode: true,
      hasBoothySession: true,
      customerCameraConnectionStateForLamp: 'disconnected',
      cameraStatus: makeStatus({
        ipcState: 'connected',
        status: {
          connected: true,
          cameraDetected: false,
          cameraModel: null,
          sessionDestination: null,
        },
      }),
      cameraStatusSnapshotFresh: makeSnapshot({ state: 'noCamera', cameraDetected: false, cameraReady: false }),
      cameraStatusLoading: false,
      isCameraReconnecting: false,
      cameraStatusError: 'IPC timeout',
    });

    expect(message).toBe(CAMERA_MESSAGE_PREPARING);
    expect(message).not.toBe(CAMERA_MESSAGE_UNAVAILABLE);
  });
});
