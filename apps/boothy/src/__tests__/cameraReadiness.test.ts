import { describe, expect, it } from 'vitest';

import type { BoothyCameraStatusReport, BoothyCameraStatusSnapshot } from '../components/ui/AppProperties';
import { isCameraReadyForCapture } from '../cameraReadiness';

function makeReport(overrides?: Partial<BoothyCameraStatusReport>): BoothyCameraStatusReport {
  return {
    ipcState: 'connected',
    protocolVersion: '1.0.0',
    status: {
      connected: true,
      cameraDetected: true,
      sessionDestination: 'C:\\shots\\Raw',
      cameraModel: 'Canon EOS 700D',
    },
    ...overrides,
  };
}

function makeSnapshot(overrides?: Partial<BoothyCameraStatusSnapshot>): BoothyCameraStatusSnapshot {
  return {
    seq: 1,
    observedAt: new Date().toISOString(),
    reason: 'statusChanged',
    mode: 'real',
    sdk: { initialized: true },
    state: 'ready',
    connected: true,
    cameraDetected: true,
    cameraReady: true,
    cameraCount: 1,
    cameraModel: 'Canon EOS 700D',
    ...overrides,
  };
}

describe('isCameraReadyForCapture', () => {
  it('allows capture when a fresh ready snapshot exists', () => {
    expect(
      isCameraReadyForCapture({
        cameraStatus: makeReport(),
        cameraStatusSnapshot: makeSnapshot(),
        cameraStatusSnapshotFresh: makeSnapshot(),
      }),
    ).toBe(true);
  });

  it('falls back to pull status when the snapshot is stale but the camera is still connected', () => {
    expect(
      isCameraReadyForCapture({
        cameraStatus: makeReport(),
        cameraStatusSnapshot: makeSnapshot({
          observedAt: new Date(Date.now() - 10_000).toISOString(),
          state: 'ready',
        }),
        cameraStatusSnapshotFresh: null,
      }),
    ).toBe(true);
  });

  it('keeps capture ready when the pull report soft-fails but the last snapshot still says ready', () => {
    expect(
      isCameraReadyForCapture({
        cameraStatus: makeReport({
          lastError: 'IPC pipe write timeout during camera.getStatus',
          status: null,
        }),
        cameraStatusSnapshot: makeSnapshot({
          observedAt: new Date(Date.now() - 10_000).toISOString(),
          state: 'ready',
        }),
        cameraStatusSnapshotFresh: null,
      }),
    ).toBe(true);
  });

  it('keeps capture disabled when the latest fresh snapshot says no camera', () => {
    expect(
      isCameraReadyForCapture({
        cameraStatus: makeReport(),
        cameraStatusSnapshot: makeSnapshot({ state: 'noCamera', cameraDetected: false, cameraReady: false }),
        cameraStatusSnapshotFresh: makeSnapshot({
          state: 'noCamera',
          cameraDetected: false,
          cameraReady: false,
        }),
      }),
    ).toBe(false);
  });
});
