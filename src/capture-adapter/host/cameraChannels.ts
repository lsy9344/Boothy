import { Channel } from '@tauri-apps/api/core';

import {
  captureConfidenceSnapshotSchema,
  type CaptureConfidenceSnapshot,
} from '../../shared-contracts/dto/captureConfidence.js';
import { cameraReadinessStatusSchema, type CameraReadinessStatus } from '../../shared-contracts/dto/cameraStatus.js';

export type CameraStatusHandler = (status: CameraReadinessStatus) => void;
export type CameraStatusParseErrorHandler = (error: Error) => void;
export type CaptureConfidenceHandler = (snapshot: CaptureConfidenceSnapshot) => void;

export function createCameraStatusChannel(
  onStatus: CameraStatusHandler,
  onParseError?: CameraStatusParseErrorHandler,
) {
  return new Channel<unknown>((payload) => {
    const parsedPayload = cameraReadinessStatusSchema.safeParse(payload);

    if (!parsedPayload.success) {
      const parseError = new Error(`Invalid watched camera readiness payload: ${parsedPayload.error.message}`);

      if (onParseError) {
        onParseError(parseError);
        return;
      }

      throw parseError;
    }

    onStatus(parsedPayload.data);
  });
}

export function createCaptureConfidenceChannel(onSnapshot: CaptureConfidenceHandler) {
  return new Channel<unknown>((payload) => {
    onSnapshot(captureConfidenceSnapshotSchema.parse(payload));
  });
}
