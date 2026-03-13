import { convertFileSrc, invoke, isTauri, type Channel } from '@tauri-apps/api/core';
import { z } from 'zod';

import {
  createCameraStatusChannel,
  createCaptureConfidenceChannel,
  type CameraStatusParseErrorHandler,
  type CameraStatusHandler,
  type CaptureConfidenceHandler,
} from './cameraChannels.js';
import { readFallbackSessionPreset } from './fallbackPresetSessionState.js';
import {
  captureConfidenceSnapshotSchema,
  type CaptureConfidenceSnapshot,
} from '../../shared-contracts/dto/captureConfidence.js';
import { cameraReadinessStatusSchema, type CameraReadinessStatus } from '../../shared-contracts/dto/cameraStatus.js';
import {
  buildCaptureCommandRequest,
  buildReadinessCommandRequest,
  cameraCommandResultSchema,
  cameraStatusChangedEventSchema,
  captureCommandResultSchema,
  captureProgressEventSchema,
  type CameraCommandResult,
  type CaptureCommandResult,
} from '../../shared-contracts/dto/cameraContract.js';
import { schemaVersions } from '../../shared-contracts/dto/schemaVersion.js';
import { defaultPresetId } from '../../shared-contracts/presets/presetCatalog.js';
import { cameraCommandNames } from './cameraCommands.js';

type InvokeFn = (command: string, args?: Record<string, unknown>) => Promise<unknown>;
type CreateChannelFn = (
  onStatus: CameraStatusHandler,
  onParseError?: CameraStatusParseErrorHandler,
) => Channel<unknown>;
type CreateCaptureChannelFn = (onSnapshot: CaptureConfidenceHandler) => Channel<unknown>;
type CreateGenericChannelFn = <T>(onMessage: (message: T) => void) => Channel<unknown>;

type CameraAdapterDependencies = {
  convertFileSrcFn?: (filePath: string) => string;
  invokeFn?: InvokeFn;
  invokeCommand?: InvokeFn;
  isTauriFn?: () => boolean;
  createChannelFn?: CreateChannelFn;
  createCaptureChannelFn?: CreateCaptureChannelFn;
  createChannel?: CreateGenericChannelFn;
};

type ReadinessRequest = {
  sessionId: string;
};

const fallbackTimestamp = '2026-03-08T00:00:00.000Z';
const fallbackShootEndsAt = '2026-03-08T10:50:00.000Z';
const fallbackPhoneRequiredSessionToken = 'phone-required';

function isPreviewablePath(path: string): boolean {
  return /^(asset|blob|data|https?):/i.test(path);
}

function toPreviewablePath(path: string, convertFileSrcFn: (filePath: string) => string): string {
  return isPreviewablePath(path) ? path : convertFileSrcFn(path);
}

function normalizeCaptureConfidenceSnapshot(
  snapshot: CaptureConfidenceSnapshot,
  convertFileSrcFn: (filePath: string) => string,
): CaptureConfidenceSnapshot {
  if (snapshot.latestPhoto.kind === 'ready') {
    return {
      ...snapshot,
      latestPhoto: {
        ...snapshot.latestPhoto,
        photo: {
          ...snapshot.latestPhoto.photo,
          assetUrl: toPreviewablePath(snapshot.latestPhoto.photo.assetUrl, convertFileSrcFn),
        },
      },
    };
  }

  if (snapshot.latestPhoto.kind === 'updating' && snapshot.latestPhoto.preview) {
    return {
      ...snapshot,
      latestPhoto: {
        ...snapshot.latestPhoto,
        preview: {
          ...snapshot.latestPhoto.preview,
          assetUrl: toPreviewablePath(snapshot.latestPhoto.preview.assetUrl, convertFileSrcFn),
        },
      },
    };
  }

  return snapshot;
}

function createFallbackCaptureConfidenceSnapshot(
  sessionId: string,
  revision = 1,
  latestPhoto: CaptureConfidenceSnapshot['latestPhoto'] = {
    kind: 'empty',
  },
): CaptureConfidenceSnapshot {
  const fallbackPreset = readFallbackSessionPreset(sessionId);

  return {
    sessionId,
    revision,
    updatedAt: fallbackTimestamp,
    shootEndsAt: fallbackShootEndsAt,
    activePreset: {
      presetId: fallbackPreset.id ?? defaultPresetId,
      label: fallbackPreset.name,
    },
    latestPhoto,
  };
}

export function createFallbackCameraReadinessStatus(sessionId: string): CameraReadinessStatus {
  return {
    sessionId,
    connectionState: shouldEscalateFallbackReadiness(sessionId) ? 'phone-required' : 'preparing',
    captureEnabled: false,
    lastStableCustomerState: null,
    error: null,
    emittedAt: fallbackTimestamp,
  };
}

function createContractFailureReadinessStatus(sessionId: string, technicalMessage: string): CameraReadinessStatus {
  return {
    sessionId,
    connectionState: 'phone-required',
    captureEnabled: false,
    lastStableCustomerState: null,
    error: {
      schemaVersion: schemaVersions.errorEnvelope,
      code: 'camera.contract.unavailable',
      severity: 'critical',
      retryable: false,
      customerState: 'cameraUnavailable',
      customerCameraConnectionState: 'offline',
      operatorCameraConnectionState: 'offline',
      operatorAction: 'contactSupport',
      message: 'Camera readiness could not be confirmed.',
      details: technicalMessage,
    },
    emittedAt: new Date().toISOString(),
  };
}

function shouldEscalateFallbackReadiness(sessionId: string): boolean {
  return sessionId.toLowerCase().includes(fallbackPhoneRequiredSessionToken);
}

function createWatchId(kind: 'readiness' | 'capture-confidence'): string {
  const suffix =
    typeof globalThis.crypto?.randomUUID === 'function'
      ? globalThis.crypto.randomUUID()
      : `${Date.now()}-${Math.random().toString(16).slice(2)}`;

  return `camera-${kind}-${suffix}`;
}

function createNativeWatchStopper(
  invokeCommand: InvokeFn,
  command: string,
  watchId: string,
  clearChannel: () => void,
) {
  let stopped = false;

  return () => {
    if (stopped) {
      return;
    }

    stopped = true;
    clearChannel();
    void invokeCommand(command, { watchId }).catch(() => undefined);
  };
}

function startFallbackReadinessWatch(sessionId: string, onStatus: CameraStatusHandler) {
  if (shouldEscalateFallbackReadiness(sessionId)) {
    onStatus({
      sessionId,
      connectionState: 'phone-required',
      captureEnabled: false,
      lastStableCustomerState: null,
      error: null,
      emittedAt: '2026-03-08T10:00:02.000Z',
    });

    return () => undefined;
  }

  const checkingTimer = globalThis.setTimeout(() => {
    onStatus({
      sessionId,
      connectionState: 'waiting',
      captureEnabled: false,
      lastStableCustomerState: null,
      error: null,
      emittedAt: '2026-03-08T10:00:02.000Z',
    });
  }, 250);
  const finalTimer = globalThis.setTimeout(() => {
    onStatus(
      shouldEscalateFallbackReadiness(sessionId)
        ? {
            sessionId,
            connectionState: 'phone-required',
            captureEnabled: false,
            lastStableCustomerState: null,
            error: null,
            emittedAt: '2026-03-08T10:00:04.000Z',
          }
        : {
            sessionId,
            connectionState: 'ready',
            captureEnabled: true,
            lastStableCustomerState: 'ready',
            error: null,
            emittedAt: '2026-03-08T10:00:04.000Z',
          },
    );
  }, 900);

  return () => {
    globalThis.clearTimeout(checkingTimer);
    globalThis.clearTimeout(finalTimer);
  };
}

function startFallbackCaptureConfidenceWatch(_sessionId: string, _onSnapshot: CaptureConfidenceHandler) {
  return () => undefined;
}

export type CameraReadinessAdapter = ReturnType<typeof createCameraAdapter>;

export function createCameraAdapter({
  convertFileSrcFn = convertFileSrc,
  invokeFn = invoke,
  invokeCommand,
  isTauriFn = isTauri,
  createChannelFn = createCameraStatusChannel,
  createCaptureChannelFn = createCaptureConfidenceChannel,
  createChannel,
}: CameraAdapterDependencies = {}) {
  const resolvedInvoke = invokeCommand ?? invokeFn;
  const resolvedCreateChannel =
    createChannel ?? ((onMessage) => createChannelFn(onMessage as CameraStatusHandler));

  return {
    async getReadinessSnapshot({ sessionId }: ReadinessRequest): Promise<CameraReadinessStatus> {
      if (!isTauriFn()) {
        return createFallbackCameraReadinessStatus(sessionId);
      }

      const snapshot = await resolvedInvoke(cameraCommandNames.getReadinessSnapshot, {
        sessionId,
      });

      return cameraReadinessStatusSchema.parse(snapshot);
    },

    async watchReadiness({
      sessionId,
      onStatus,
    }: ReadinessRequest & { onStatus: CameraStatusHandler }): Promise<() => void> {
      if (!isTauriFn()) {
        return startFallbackReadinessWatch(sessionId, onStatus);
      }

      const statusChannel = createChannelFn(onStatus, (error) => {
        onStatus(createContractFailureReadinessStatus(sessionId, error.message));
      });
      const watchId = createWatchId('readiness');

      await resolvedInvoke(cameraCommandNames.watchReadiness, {
        sessionId,
        watchId,
        statusChannel,
      });

      return createNativeWatchStopper(
        resolvedInvoke,
        cameraCommandNames.unwatchReadiness,
        watchId,
        () => {
          statusChannel.onmessage = () => undefined;
        },
      );
    },

    async runReadinessCheck(
      input: Parameters<typeof buildReadinessCommandRequest>[0],
      onProgress?: (event: z.infer<typeof cameraStatusChangedEventSchema>) => void,
    ): Promise<CameraCommandResult> {
      const payload = buildReadinessCommandRequest(input);
      const channel = resolvedCreateChannel<z.infer<typeof cameraStatusChangedEventSchema>>((event) => {
        onProgress?.(cameraStatusChangedEventSchema.parse(event));
      });
      const result = await resolvedInvoke(cameraCommandNames.runReadinessFlow, {
        payload,
        channel,
      });

      return cameraCommandResultSchema.parse(result);
    },

    async requestCapture(
      input: Parameters<typeof buildCaptureCommandRequest>[0],
      onProgress?: (event: z.infer<typeof captureProgressEventSchema>) => void,
    ): Promise<CaptureCommandResult> {
      const payload = buildCaptureCommandRequest(input);

      if (!isTauriFn()) {
        throw new Error('Browser capture fallback is unavailable for Story 3.1');
      }

      const channel = resolvedCreateChannel<z.infer<typeof captureProgressEventSchema>>((event) => {
        onProgress?.(captureProgressEventSchema.parse(event));
      });
      const result = await resolvedInvoke(cameraCommandNames.requestCapture, {
        payload,
        channel,
      });

      return captureCommandResultSchema.parse(result);
    },

    async getCaptureConfidenceSnapshot({ sessionId }: ReadinessRequest): Promise<CaptureConfidenceSnapshot> {
      if (!isTauriFn()) {
        return createFallbackCaptureConfidenceSnapshot(sessionId);
      }

      const snapshot = await resolvedInvoke(cameraCommandNames.getCaptureConfidenceSnapshot, {
        sessionId,
      });

      return normalizeCaptureConfidenceSnapshot(captureConfidenceSnapshotSchema.parse(snapshot), convertFileSrcFn);
    },

    async watchCaptureConfidence({
      sessionId,
      onSnapshot,
    }: ReadinessRequest & { onSnapshot: CaptureConfidenceHandler }): Promise<() => void> {
      if (!isTauriFn()) {
        return startFallbackCaptureConfidenceWatch(sessionId, onSnapshot);
      }

      const captureChannel = createCaptureChannelFn((snapshot) => {
        onSnapshot(normalizeCaptureConfidenceSnapshot(snapshot, convertFileSrcFn));
      });
      const watchId = createWatchId('capture-confidence');

      await resolvedInvoke(cameraCommandNames.watchCaptureConfidence, {
        sessionId,
        watchId,
        captureChannel,
      });

      return createNativeWatchStopper(
        resolvedInvoke,
        cameraCommandNames.unwatchCaptureConfidence,
        watchId,
        () => {
          captureChannel.onmessage = () => undefined;
        },
      );
    },
  };
}

export const cameraAdapter = createCameraAdapter();
