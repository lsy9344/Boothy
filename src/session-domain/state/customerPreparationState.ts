import type { CameraReadinessStatus } from '../../shared-contracts/dto/cameraStatus.js';
import { schemaVersions } from '../../shared-contracts/dto/schemaVersion.js';

export type CustomerPreparationState =
  | {
      kind: 'preparing';
      captureEnabled: false;
      branchPhoneNumber: string;
      hostStatus: CameraReadinessStatus;
    }
  | {
      kind: 'waiting';
      captureEnabled: false;
      branchPhoneNumber: string;
      hostStatus: CameraReadinessStatus;
    }
  | {
      kind: 'ready';
      captureEnabled: true;
      branchPhoneNumber: string;
      hostStatus: CameraReadinessStatus;
    }
  | {
      kind: 'phone-required';
      captureEnabled: false;
      branchPhoneNumber: string;
      hostStatus: CameraReadinessStatus;
    };

export function deriveCustomerPreparationState(
  hostStatus: CameraReadinessStatus,
  branchPhoneNumber: string,
): CustomerPreparationState {
  assertNormalizedReadinessPayload(hostStatus)

  switch (hostStatus.connectionState) {
    case 'phone-required':
      return {
        kind: 'phone-required',
        captureEnabled: false,
        branchPhoneNumber,
        hostStatus,
      };
    case 'ready':
      return {
        kind: 'ready',
        captureEnabled: true,
        branchPhoneNumber,
        hostStatus,
      };
    case 'waiting':
      return {
        kind: 'waiting',
        captureEnabled: false,
        branchPhoneNumber,
        hostStatus,
      };
    case 'preparing':
    default:
      return {
        kind: 'preparing',
        captureEnabled: false,
        branchPhoneNumber,
        hostStatus,
      };
  }
}

export function createInitialCustomerPreparationState(sessionId: string, branchPhoneNumber: string): CustomerPreparationState {
  return deriveCustomerPreparationState(
    createReadinessStatus(sessionId, {
      connectionState: 'preparing',
    }),
    branchPhoneNumber,
  );
}

export function createContractFailurePreparationState(
  sessionId: string,
  branchPhoneNumber: string,
  technicalMessage: string,
): CustomerPreparationState {
  return deriveCustomerPreparationState(
    createReadinessStatus(sessionId, {
      connectionState: 'phone-required',
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
    }),
    branchPhoneNumber,
  );
}

export function createThresholdEscalatedPreparationState(
  sessionId: string,
  branchPhoneNumber: string,
): CustomerPreparationState {
  return deriveCustomerPreparationState(
    createReadinessStatus(sessionId, {
      connectionState: 'phone-required',
      error: {
        schemaVersion: schemaVersions.errorEnvelope,
        code: 'camera.preparation.timeout',
        severity: 'error',
        retryable: false,
        customerState: 'cameraUnavailable',
        customerCameraConnectionState: 'offline',
        operatorCameraConnectionState: 'disconnected',
        operatorAction: 'contactSupport',
        message: 'Preparation exceeded the approved escalation threshold.',
      },
    }),
    branchPhoneNumber,
  );
}

function assertNormalizedReadinessPayload(hostStatus: CameraReadinessStatus): void {
  const shouldAllowCapture = hostStatus.connectionState === 'ready'

  if (hostStatus.captureEnabled !== shouldAllowCapture) {
    throw new Error(
      `Invalid normalized readiness payload for session ${hostStatus.sessionId}: ` +
        `connectionState=${hostStatus.connectionState} requires captureEnabled=${shouldAllowCapture}, got ${hostStatus.captureEnabled}`,
    )
  }
}

function createReadinessStatus(
  sessionId: string,
  overrides: Partial<CameraReadinessStatus>,
): CameraReadinessStatus {
  return {
    sessionId,
    connectionState: 'preparing',
    captureEnabled: false,
    lastStableCustomerState: null,
    error: null,
    emittedAt: new Date().toISOString(),
    ...overrides,
  };
}
