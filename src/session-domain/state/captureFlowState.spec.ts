import { describe, expect, it } from 'vitest';

import {
  applyPresetSelection,
  completeCaptureRequest,
  createCaptureFlowState,
  startCaptureRequest,
} from './captureFlowState.js';

describe('captureFlowState', () => {
  it('applies preset changes forward-only without mutating previously captured photos', () => {
    const initialState = createCaptureFlowState({
      activePresetId: 'warm-tone',
      captures: [
        {
          captureId: 'capture-001',
          capturedAt: '2026-03-08T09:00:00.000Z',
          photoLabel: '첫 번째 촬영',
          presetId: 'warm-tone',
        },
      ],
    });

    const inFlightState = startCaptureRequest(initialState, {
      captureId: 'capture-002',
      requestedAt: '2026-03-08T09:01:00.000Z',
    });
    const changedPresetState = applyPresetSelection(inFlightState, 'background-pink');
    const completedCurrentShot = completeCaptureRequest(changedPresetState, {
      captureId: 'capture-002',
      capturedAt: '2026-03-08T09:01:05.000Z',
      photoLabel: '두 번째 촬영',
    });
    const nextRequestState = startCaptureRequest(completedCurrentShot, {
      captureId: 'capture-003',
      requestedAt: '2026-03-08T09:02:00.000Z',
    });
    const completedNextShot = completeCaptureRequest(nextRequestState, {
      captureId: 'capture-003',
      capturedAt: '2026-03-08T09:02:04.000Z',
      photoLabel: '세 번째 촬영',
    });

    expect(completedNextShot.activePresetId).toBe('background-pink');
    expect(completedNextShot.captures.map((capture) => capture.presetId)).toEqual([
      'background-pink',
      'warm-tone',
      'warm-tone',
    ]);
    expect(completedNextShot.captures.map((capture) => capture.photoLabel)).toEqual([
      '세 번째 촬영',
      '두 번째 촬영',
      '첫 번째 촬영',
    ]);
  });

  it('keeps only the latest selected preset as the session authority after repeated changes', () => {
    const initialState = createCaptureFlowState();

    const nextState = applyPresetSelection(
      applyPresetSelection(initialState, 'cool-tone'),
      'background-ivory',
    );

    expect(nextState.activePresetId).toBe('background-ivory');
    expect(nextState.pendingPresetChangeMessage).toBe('다음 촬영부터 적용됩니다.');
  });

  it('preserves the preset active at request time for each in-flight capture even when completions arrive out of order', () => {
    const initialState = createCaptureFlowState({
      activePresetId: 'warm-tone',
    });

    const firstRequestState = startCaptureRequest(initialState, {
      captureId: 'capture-001',
      requestedAt: '2026-03-08T09:00:00.000Z',
    });
    const changedPresetState = applyPresetSelection(firstRequestState, 'background-pink');
    const secondRequestState = startCaptureRequest(changedPresetState, {
      captureId: 'capture-002',
      requestedAt: '2026-03-08T09:00:01.000Z',
    });
    const completedFirstCapture = completeCaptureRequest(secondRequestState, {
      captureId: 'capture-001',
      capturedAt: '2026-03-08T09:00:03.000Z',
      photoLabel: '첫 번째 촬영',
    });
    const completedSecondCapture = completeCaptureRequest(completedFirstCapture, {
      captureId: 'capture-002',
      capturedAt: '2026-03-08T09:00:04.000Z',
      photoLabel: '두 번째 촬영',
    });

    expect(completedSecondCapture.captures.map((capture) => ({
      captureId: capture.captureId,
      presetId: capture.presetId,
    }))).toEqual([
      {
        captureId: 'capture-002',
        presetId: 'background-pink',
      },
      {
        captureId: 'capture-001',
        presetId: 'warm-tone',
      },
    ]);
  });

  it('treats selecting the active preset as an idempotent no-op without confirmation feedback', () => {
    const initialState = createCaptureFlowState({
      activePresetId: 'background-pink',
    });

    const nextState = applyPresetSelection(initialState, 'background-pink');

    expect(nextState).toBe(initialState);
    expect(nextState.pendingPresetChangeMessage).toBeNull();
  });
});
