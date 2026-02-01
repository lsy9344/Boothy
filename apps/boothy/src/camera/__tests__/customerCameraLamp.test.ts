import { describe, expect, it } from 'vitest';
import type { BoothyCameraStatusReport } from '../../components/ui/AppProperties';
import { nextCustomerCameraLampConnectionState } from '../customerCameraLamp';

describe('nextCustomerCameraLampConnectionState', () => {
  const baseReport = {
    protocolVersion: '1.0.0',
    lastError: null,
    requestId: null,
    correlationId: null,
  } satisfies Omit<BoothyCameraStatusReport, 'ipcState'>;

  it('keeps disconnected when status becomes temporarily unknown (prevents green/red flapping)', () => {
    let state: 'connected' | 'disconnected' = 'connected';

    state = nextCustomerCameraLampConnectionState({
      prev: state,
      report: {
        ...baseReport,
        ipcState: 'connected',
        status: { connected: false, cameraDetected: false, cameraModel: null, sessionDestination: null },
      },
    });
    expect(state).toBe('disconnected');

    // When the backend temporarily loses status (e.g. IPC restart / write-timeout path),
    // we must NOT default back to "connected" (green).
    state = nextCustomerCameraLampConnectionState({
      prev: state,
      report: null,
    });
    expect(state).toBe('disconnected');

    state = nextCustomerCameraLampConnectionState({
      prev: state,
      report: {
        ...baseReport,
        ipcState: 'connected',
        status: null,
        lastError: 'IPC pipe write timeout',
      },
    });
    expect(state).toBe('disconnected');
  });

  it('switches to connected only on explicit evidence', () => {
    let state: 'connected' | 'disconnected' = 'disconnected';

    state = nextCustomerCameraLampConnectionState({
      prev: state,
      report: {
        ...baseReport,
        ipcState: 'connected',
        status: { connected: true, cameraDetected: false, cameraModel: 'EOS', sessionDestination: null },
      },
    });
    expect(state).toBe('disconnected');

    state = nextCustomerCameraLampConnectionState({
      prev: state,
      report: {
        ...baseReport,
        ipcState: 'connected',
        status: { connected: true, cameraDetected: true, cameraModel: 'EOS', sessionDestination: null },
      },
    });
    expect(state).toBe('connected');
  });
});
