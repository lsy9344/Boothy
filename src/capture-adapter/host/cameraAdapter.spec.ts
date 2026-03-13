import { describe, expect, it, vi } from 'vitest';

import { createCameraAdapter } from './cameraAdapter.js';
import { schemaVersions } from '../../shared-contracts/dto/schemaVersion.js';

describe('createCameraAdapter', () => {
  it('parses the readiness snapshot through the shared contract schema', async () => {
    const invokeFn = vi.fn(async () => ({
      sessionId: 'session-1',
      connectionState: 'waiting',
      captureEnabled: false,
      lastStableCustomerState: null,
      error: null,
      emittedAt: '2026-03-08T00:00:00.000Z',
    }));

    const adapter = createCameraAdapter({
      createChannelFn: () =>
        ({
          onmessage: () => undefined,
        }) as never,
      invokeFn,
      isTauriFn: () => true,
    });

    await expect(adapter.getReadinessSnapshot({ sessionId: 'session-1' })).resolves.toMatchObject({
      sessionId: 'session-1',
      connectionState: 'waiting',
    });
    expect(invokeFn).toHaveBeenCalledWith('get_camera_readiness_snapshot', {
      sessionId: 'session-1',
    });
  });

  it('parses the capture-confidence snapshot through the shared contract schema', async () => {
    const invokeFn = vi.fn(async () => ({
      sessionId: 'session-1',
      revision: 2,
      updatedAt: '2026-03-08T10:06:00.000Z',
      shootEndsAt: '2026-03-08T10:50:00.000Z',
      activePreset: {
        presetId: 'preset-noir',
        label: 'Soft Noir',
      },
      latestPhoto: {
        kind: 'ready',
        photo: {
          sessionId: 'session-1',
          captureId: 'capture-2',
          sequence: 2,
          assetUrl: 'asset://session-1/capture-2',
          capturedAt: '2026-03-08T10:05:00.000Z',
        },
      },
    }));

    const adapter = createCameraAdapter({
      createCaptureChannelFn: () =>
        ({
          onmessage: () => undefined,
        }) as never,
      invokeFn,
      isTauriFn: () => true,
    });

    expect('getCaptureConfidenceSnapshot' in adapter).toBe(true);

    if (!('getCaptureConfidenceSnapshot' in adapter)) {
      return;
    }

    await expect(adapter.getCaptureConfidenceSnapshot({ sessionId: 'session-1' })).resolves.toMatchObject({
      sessionId: 'session-1',
      activePreset: {
        label: 'Soft Noir',
      },
    });
    expect(invokeFn).toHaveBeenCalledWith('get_capture_confidence_snapshot', {
      sessionId: 'session-1',
    });
  });

  it('converts raw latest-photo filesystem paths into previewable asset URLs', async () => {
    const invokeFn = vi.fn(async () => ({
      sessionId: 'session-1',
      revision: 2,
      updatedAt: '2026-03-08T10:06:00.000Z',
      shootEndsAt: '2026-03-08T10:50:00.000Z',
      activePreset: {
        presetId: 'preset-noir',
        label: 'Soft Noir',
      },
      latestPhoto: {
        kind: 'ready',
        photo: {
          sessionId: 'session-1',
          captureId: 'capture-2',
          sequence: 2,
          assetUrl: 'C:/Boothy/Sessions/session-1/processed/capture-2.png',
          capturedAt: '2026-03-08T10:05:00.000Z',
        },
      },
    }));

    const adapter = createCameraAdapter({
      convertFileSrcFn: (filePath) => `asset://preview/${encodeURIComponent(filePath)}`,
      createCaptureChannelFn: () =>
        ({
          onmessage: () => undefined,
        }) as never,
      invokeFn,
      isTauriFn: () => true,
    });

    await expect(adapter.getCaptureConfidenceSnapshot({ sessionId: 'session-1' })).resolves.toMatchObject({
      latestPhoto: {
        kind: 'ready',
        photo: {
          assetUrl:
            'asset://preview/C%3A%2FBoothy%2FSessions%2Fsession-1%2Fprocessed%2Fcapture-2.png',
        },
      },
    });
  });

  it('requests capture through the typed host boundary and forwards capture progress events', async () => {
    const progressEvents: unknown[] = [];
    const invokeFn = vi.fn(async (_command: string, args?: Record<string, unknown>) => {
      const channel = args?.channel as {
        onmessage: (payload: unknown) => void
      };

      channel.onmessage({
        schemaVersion: schemaVersions.protocol,
        requestId: 'req-capture-001',
        correlationId: 'corr-session-001',
        event: 'capture.progress',
        sessionId: 'session-1',
        payload: {
          stage: 'captureStarted',
          captureId: 'capture-7',
          percentComplete: 0,
          lastUpdatedAt: '2026-03-08T10:06:00.000Z',
        },
      });
      channel.onmessage({
        schemaVersion: schemaVersions.protocol,
        requestId: 'req-capture-001',
        correlationId: 'corr-session-001',
        event: 'capture.progress',
        sessionId: 'session-1',
        payload: {
          stage: 'captureCompleted',
          captureId: 'capture-7',
          percentComplete: 100,
          lastUpdatedAt: '2026-03-08T10:06:04.000Z',
        },
      });

      return {
        schemaVersion: 'boothy.camera.contract.v1',
        requestId: 'req-capture-001',
        correlationId: 'corr-session-001',
        ok: true,
        sessionId: 'session-1',
        captureId: 'capture-7',
        originalFileName: 'originals/capture-7.nef',
        processedFileName: 'capture-7.png',
        capturedAt: '2026-03-08T10:06:04.000Z',
        manifestPath: 'C:/Boothy/Sessions/session-1/session.json',
      };
    });

    const adapter = createCameraAdapter({
      createChannel: <T,>(onMessage: (message: T) => void) => ({
        onmessage: (payload: unknown) => {
          onMessage(payload as T);
        },
      }) as never,
      invokeFn,
      isTauriFn: () => true,
    });

    await expect(
      adapter.requestCapture(
        {
          requestId: 'req-capture-001',
          correlationId: 'corr-session-001',
          sessionId: 'session-1',
          activePreset: {
            presetId: 'background-pink',
            label: '배경지 - 핑크',
          },
        },
        (event) => {
          progressEvents.push(event);
        },
      ),
    ).resolves.toMatchObject({
      sessionId: 'session-1',
      captureId: 'capture-7',
    });

    expect(invokeFn).toHaveBeenCalledWith(
      'request_capture',
      expect.objectContaining({
        payload: expect.objectContaining({
          method: 'camera.capture',
          sessionId: 'session-1',
          payload: {
            activePreset: {
              presetId: 'background-pink',
              label: '배경지 - 핑크',
            },
          },
        }),
        channel: expect.any(Object),
      }),
    );
    expect(progressEvents).toEqual([
      {
        schemaVersion: schemaVersions.protocol,
        requestId: 'req-capture-001',
        correlationId: 'corr-session-001',
        event: 'capture.progress',
        sessionId: 'session-1',
        payload: {
          stage: 'captureStarted',
          captureId: 'capture-7',
          percentComplete: 0,
          lastUpdatedAt: '2026-03-08T10:06:00.000Z',
        },
      },
      {
        schemaVersion: schemaVersions.protocol,
        requestId: 'req-capture-001',
        correlationId: 'corr-session-001',
        event: 'capture.progress',
        sessionId: 'session-1',
        payload: {
          stage: 'captureCompleted',
          captureId: 'capture-7',
          percentComplete: 100,
          lastUpdatedAt: '2026-03-08T10:06:04.000Z',
        },
      },
    ]);
  });

  it('forwards ordered readiness updates from the channel boundary', async () => {
    const updates: Array<{
      sessionId: string
      connectionState: 'preparing' | 'waiting' | 'ready' | 'phone-required'
      captureEnabled: boolean
      lastStableCustomerState: 'preparing' | 'ready' | null
      error: {
        schemaVersion: 'boothy.camera.error-envelope.v1'
        code: string
        severity: 'info' | 'warning' | 'error' | 'critical'
        customerState: 'cameraReconnectNeeded' | 'cameraUnavailable'
        customerCameraConnectionState: 'connected' | 'needsAttention' | 'offline'
        operatorCameraConnectionState: 'connected' | 'reconnecting' | 'disconnected' | 'offline'
        operatorAction: 'checkCableAndRetry' | 'restartHelper' | 'contactSupport'
        retryable: boolean
        message: string
        details?: string
      } | null
      emittedAt: string
    }> = []
    const invokeFn = vi.fn(async (_command: string, args?: Record<string, unknown>) => {
      const statusChannel = args?.statusChannel as {
        onmessage: (payload: unknown) => void
      }

      statusChannel.onmessage({
        sessionId: 'session-1',
        connectionState: 'waiting',
        captureEnabled: false,
        lastStableCustomerState: null,
        error: null,
        emittedAt: '2026-03-08T00:00:00.000Z',
      })
      statusChannel.onmessage({
        sessionId: 'session-1',
        connectionState: 'ready',
        captureEnabled: true,
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
          details: 'Transient timeout',
        },
        emittedAt: '2026-03-08T00:00:05.000Z',
      })
    })

    const adapter = createCameraAdapter({
      createChannelFn: (onStatus) => ({
        onmessage: (payload: unknown) => {
          onStatus(payload as never)
        },
      }) as never,
      invokeFn,
      isTauriFn: () => true,
    })

    await adapter.watchReadiness({
      sessionId: 'session-1',
      onStatus: (status) => {
        updates.push(status)
      },
    })

    expect(updates).toMatchObject([
      {
        connectionState: 'waiting',
        captureEnabled: false,
        error: null,
      },
      {
        connectionState: 'ready',
        captureEnabled: true,
        error: {
          code: 'camera.poll.retryable',
          details: 'Transient timeout',
        },
      },
    ])
  });

  it('normalizes malformed watched readiness payloads into a contract-failure phone-required state', async () => {
    const updates: Array<{
      sessionId: string
      connectionState: 'preparing' | 'waiting' | 'ready' | 'phone-required'
      captureEnabled: boolean
      lastStableCustomerState: 'preparing' | 'ready' | null
      error: {
        code: string
        details?: string
      } | null
    }> = [];
    const invokeFn = vi.fn(async (_command: string, args?: Record<string, unknown>) => {
      const statusChannel = args?.statusChannel as {
        onmessage: (payload: unknown) => void
      };

      statusChannel.onmessage({
        sessionId: 'session-contract-failure',
        connectionState: 'ready',
        captureEnabled: false,
        lastStableCustomerState: 'ready',
        error: null,
        emittedAt: '2026-03-08T00:00:05.000Z',
      });
    });

    const adapter = createCameraAdapter({
      createChannelFn: (_onStatus, onParseError) =>
        ({
          onmessage: () => {
            onParseError?.(new Error('Invalid watched camera readiness payload: mocked schema failure'));
          },
        }) as never,
      invokeFn,
      isTauriFn: () => true,
    });

    await adapter.watchReadiness({
      sessionId: 'session-contract-failure',
      onStatus: (status) => {
        updates.push(status);
      },
    });

    expect(updates).toEqual([
      expect.objectContaining({
        sessionId: 'session-contract-failure',
        connectionState: 'phone-required',
        captureEnabled: false,
        error: expect.objectContaining({
          code: 'camera.contract.unavailable',
        }),
      }),
    ]);
    expect(updates[0]?.error?.details).toContain('Invalid watched camera readiness payload');
  });

  it('stops the native readiness watch when the unsubscribe function runs', async () => {
    const invokeFn = vi.fn(async () => undefined);
    const adapter = createCameraAdapter({
      createChannelFn: (onStatus) =>
        ({
          onmessage: (payload: unknown) => {
            onStatus(payload as never);
          },
        }) as never,
      invokeFn,
      isTauriFn: () => true,
    });

    const stopWatching = await adapter.watchReadiness({
      sessionId: 'session-native-readiness-stop',
      onStatus: () => undefined,
    });

    stopWatching();
    await Promise.resolve();

    const watchCall = invokeFn.mock.calls[0];
    const watchArgs = watchCall?.[1] as {
      watchId: string;
    };

    expect(invokeFn).toHaveBeenNthCalledWith(
      1,
      'watch_camera_readiness',
      expect.objectContaining({
        sessionId: 'session-native-readiness-stop',
        watchId: expect.any(String),
        statusChannel: expect.any(Object),
      }),
    );
    expect(invokeFn).toHaveBeenNthCalledWith(2, 'unwatch_camera_readiness', {
      watchId: watchArgs.watchId,
    });
  });

  it('normalizes watched capture-confidence snapshots before delivering them to the UI', async () => {
    const updates: unknown[] = [];
    const invokeFn = vi.fn(async (_command: string, args?: Record<string, unknown>) => {
      const captureChannel = args?.captureChannel as {
        onmessage: (payload: unknown) => void;
      };

      captureChannel.onmessage({
        sessionId: 'session-1',
        revision: 3,
        updatedAt: '2026-03-08T10:06:04.000Z',
        shootEndsAt: '2026-03-08T10:50:00.000Z',
        activePreset: {
          presetId: 'preset-noir',
          label: 'Soft Noir',
        },
        latestPhoto: {
          kind: 'ready',
          photo: {
            sessionId: 'session-1',
            captureId: 'capture-3',
            sequence: 3,
            assetUrl: 'C:/Boothy/Sessions/session-1/processed/capture-3.png',
            capturedAt: '2026-03-08T10:06:04.000Z',
          },
        },
      });
    });

    const adapter = createCameraAdapter({
      convertFileSrcFn: (filePath) => `asset://preview/${encodeURIComponent(filePath)}`,
      createCaptureChannelFn: (onSnapshot) =>
        ({
          onmessage: (payload: unknown) => {
            onSnapshot(payload as never);
          },
        }) as never,
      invokeFn,
      isTauriFn: () => true,
    });

    await adapter.watchCaptureConfidence({
      sessionId: 'session-1',
      onSnapshot: (snapshot) => {
        updates.push(snapshot);
      },
    });

    expect(updates).toEqual([
      expect.objectContaining({
        latestPhoto: {
          kind: 'ready',
          photo: expect.objectContaining({
            assetUrl:
              'asset://preview/C%3A%2FBoothy%2FSessions%2Fsession-1%2Fprocessed%2Fcapture-3.png',
          }),
        },
      }),
    ]);
  });

  it('stops the native capture-confidence watch when the unsubscribe function runs', async () => {
    const invokeFn = vi.fn(async () => undefined);
    const adapter = createCameraAdapter({
      createCaptureChannelFn: (onSnapshot) =>
        ({
          onmessage: (payload: unknown) => {
            onSnapshot(payload as never);
          },
        }) as never,
      invokeFn,
      isTauriFn: () => true,
    });

    const stopWatching = await adapter.watchCaptureConfidence({
      sessionId: 'session-native-capture-stop',
      onSnapshot: () => undefined,
    });

    stopWatching();
    await Promise.resolve();

    const watchCall = invokeFn.mock.calls[0];
    const watchArgs = watchCall?.[1] as {
      watchId: string;
    };

    expect(invokeFn).toHaveBeenNthCalledWith(
      1,
      'watch_capture_confidence',
      expect.objectContaining({
        sessionId: 'session-native-capture-stop',
        watchId: expect.any(String),
        captureChannel: expect.any(Object),
      }),
    );
    expect(invokeFn).toHaveBeenNthCalledWith(2, 'unwatch_capture_confidence', {
      watchId: watchArgs.watchId,
    });
  });

  it('emits a browser fallback readiness transition so the customer flow can leave preparation', async () => {
    vi.useFakeTimers();

    const onStatus = vi.fn();
    const adapter = createCameraAdapter({
      isTauriFn: () => false,
    });

    const stopWatching = await adapter.watchReadiness({
      sessionId: 'session-1',
      onStatus,
    });

    vi.advanceTimersByTime(1000);

    expect(onStatus).toHaveBeenCalledWith(
      expect.objectContaining({
        sessionId: 'session-1',
        connectionState: 'ready',
        captureEnabled: true,
      }),
    );

    stopWatching();
    vi.useRealTimers();
  });

  it('can escalate the browser fallback readiness path to phone-required for dev and browser flows', async () => {
    vi.useFakeTimers();

    const onStatus = vi.fn();
    const adapter = createCameraAdapter({
      isTauriFn: () => false,
    });

    const stopWatching = await adapter.watchReadiness({
      sessionId: 'session-phone-required-fallback',
      onStatus,
    });

    vi.advanceTimersByTime(1000);

    expect(onStatus).toHaveBeenCalledWith(
      expect.objectContaining({
        sessionId: 'session-phone-required-fallback',
        connectionState: 'phone-required',
        captureEnabled: false,
      }),
    );

    stopWatching();
    vi.useRealTimers();
  });

  it('keeps the browser phone-required fallback deterministic without a transient waiting state', async () => {
    const onStatus = vi.fn();
    const adapter = createCameraAdapter({
      isTauriFn: () => false,
    });

    const stopWatching = await adapter.watchReadiness({
      sessionId: 'session-phone-required-fallback',
      onStatus,
    });

    expect(onStatus).toHaveBeenCalledTimes(1);
    expect(onStatus).toHaveBeenCalledWith(
      expect.objectContaining({
        sessionId: 'session-phone-required-fallback',
        connectionState: 'phone-required',
        captureEnabled: false,
      }),
    );
    expect(onStatus).not.toHaveBeenCalledWith(
      expect.objectContaining({
        connectionState: 'waiting',
      }),
    );

    stopWatching();
  });

  it('returns a phone-required fallback readiness snapshot for dev sessions that intentionally exercise escalation', async () => {
    const adapter = createCameraAdapter({
      isTauriFn: () => false,
    });

    await expect(adapter.getReadinessSnapshot({ sessionId: 'session-phone-required-fallback' })).resolves.toMatchObject({
      sessionId: 'session-phone-required-fallback',
      connectionState: 'phone-required',
      captureEnabled: false,
    });
  });

  it('surfaces missing readiness commands in Tauri instead of fabricating a ready booth', async () => {
    const invokeFn = vi.fn(async () => {
      throw new Error('command get_camera_readiness_snapshot not found');
    });
    const onStatus = vi.fn();
    const adapter = createCameraAdapter({
      createChannelFn: () =>
        ({
          onmessage: () => undefined,
        }) as never,
      invokeFn,
      isTauriFn: () => true,
    });

    await expect(adapter.getReadinessSnapshot({ sessionId: 'session-3' })).rejects.toThrow(
      'command get_camera_readiness_snapshot not found',
    );
    await expect(
      adapter.watchReadiness({
        sessionId: 'session-3',
        onStatus,
      }),
    ).rejects.toThrow('command get_camera_readiness_snapshot not found');
    expect(onStatus).not.toHaveBeenCalled();
  });

  it('uses only approved MVP preset identifiers in fallback capture-confidence snapshots', async () => {
    const adapter = createCameraAdapter({
      isTauriFn: () => false,
    });

    await expect(adapter.getCaptureConfidenceSnapshot({ sessionId: 'session-4' })).resolves.toMatchObject({
      activePreset: {
        presetId: 'warm-tone',
        label: '웜톤',
      },
    });
  });

  it('does not fabricate latest-photo watch updates in the browser fallback path', async () => {
    vi.useFakeTimers();

    const onSnapshot = vi.fn();
    const adapter = createCameraAdapter({
      isTauriFn: () => false,
    });

    const stopWatching = await adapter.watchCaptureConfidence({
      sessionId: 'session-browser-fallback',
      onSnapshot,
    });

    vi.advanceTimersByTime(2000);

    expect(onSnapshot).not.toHaveBeenCalled();

    stopWatching();
    vi.useRealTimers();
  });

  it('does not fabricate a successful capture result in the browser fallback path', async () => {
    const onProgress = vi.fn();
    const adapter = createCameraAdapter({
      isTauriFn: () => false,
    });

    await expect(
      adapter.requestCapture(
        {
          requestId: 'req-browser-capture-001',
          correlationId: 'corr-browser-capture-001',
          sessionId: 'session-browser-capture',
          activePreset: {
            presetId: 'background-pink',
            label: '배경지 - 핑크',
          },
        },
        onProgress,
      ),
    ).rejects.toThrow('Browser capture fallback is unavailable for Story 3.1');

    expect(onProgress).not.toHaveBeenCalled();
  });

  it('surfaces missing capture-confidence commands in Tauri instead of synthesizing photos', async () => {
    const invokeFn = vi.fn(async () => {
      throw new Error('command get_capture_confidence_snapshot not found');
    });
    const onSnapshot = vi.fn();
    const adapter = createCameraAdapter({
      createCaptureChannelFn: () =>
        ({
          onmessage: () => undefined,
        }) as never,
      invokeFn,
      isTauriFn: () => true,
    });

    await expect(adapter.getCaptureConfidenceSnapshot({ sessionId: 'session-5' })).rejects.toThrow(
      'command get_capture_confidence_snapshot not found',
    );
    await expect(
      adapter.watchCaptureConfidence({
        sessionId: 'session-5',
        onSnapshot,
      }),
    ).rejects.toThrow('command get_capture_confidence_snapshot not found');
    expect(onSnapshot).not.toHaveBeenCalled();
  });
});
