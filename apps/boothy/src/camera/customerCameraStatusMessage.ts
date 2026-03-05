import type { BoothyCameraStatusReport, BoothyCameraStatusSnapshot } from '../components/ui/AppProperties';
import type { CustomerCameraLampConnectionState } from './customerCameraLamp';

export const CAMERA_MESSAGE_NEEDS_CONNECTION = '카메라 연결을 확인해 주세요.';
export const CAMERA_MESSAGE_PREPARING = '촬영을 준비 중입니다. 잠시만 기다려 주세요.';
export const CAMERA_MESSAGE_UNAVAILABLE = '현재 촬영을 사용할 수 없습니다. 관리자에게 문의해 주세요.';

interface GetCustomerCameraStatusMessageParams {
  isCustomerMode: boolean;
  hasBoothySession: boolean;
  cameraStatus: BoothyCameraStatusReport | null;
  cameraStatusSnapshotFresh: BoothyCameraStatusSnapshot | null;
  cameraStatusLoading: boolean;
  isCameraReconnecting: boolean;
  cameraStatusError: string | null;
  customerCameraConnectionStateForLamp: CustomerCameraLampConnectionState;
}

export function getCustomerCameraStatusMessage({
  isCustomerMode,
  hasBoothySession,
  cameraStatus,
  cameraStatusSnapshotFresh,
  cameraStatusLoading,
  isCameraReconnecting,
  cameraStatusError,
  customerCameraConnectionStateForLamp,
}: GetCustomerCameraStatusMessageParams): string | null {
  if (!isCustomerMode || !hasBoothySession) {
    return null;
  }

  if (cameraStatusSnapshotFresh?.mode === 'mock' || cameraStatusSnapshotFresh?.state === 'error') {
    return CAMERA_MESSAGE_UNAVAILABLE;
  }

  // Keep the customer-facing text stable while the lamp is green.
  if (customerCameraConnectionStateForLamp === 'connected') {
    return null;
  }

  if (cameraStatusLoading || isCameraReconnecting || cameraStatus?.ipcState === 'reconnecting') {
    return CAMERA_MESSAGE_PREPARING;
  }

  if (cameraStatus?.ipcState !== 'connected') {
    return CAMERA_MESSAGE_PREPARING;
  }

  if (cameraStatusSnapshotFresh?.state === 'connecting') {
    return CAMERA_MESSAGE_PREPARING;
  }

  if (cameraStatusSnapshotFresh?.state === 'noCamera') {
    if (cameraStatus?.status?.connected === false) {
      return CAMERA_MESSAGE_NEEDS_CONNECTION;
    }
    return CAMERA_MESSAGE_PREPARING;
  }

  if (cameraStatus?.status && !cameraStatus.status.connected) {
    return CAMERA_MESSAGE_NEEDS_CONNECTION;
  }

  if (cameraStatus?.status && !cameraStatus.status.cameraDetected) {
    return CAMERA_MESSAGE_PREPARING;
  }

  // Poll/getStatus transient failures should not escalate to "unavailable" in customer mode.
  if (cameraStatusError || (cameraStatus?.lastError && cameraStatus?.ipcState !== 'connected')) {
    return CAMERA_MESSAGE_PREPARING;
  }

  return null;
}
