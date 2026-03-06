import type { BoothyCameraStatusReport, BoothyCameraStatusSnapshot } from './components/ui/AppProperties';

interface IsCameraReadyForCaptureParams {
  cameraStatus: BoothyCameraStatusReport | null;
  cameraStatusSnapshot: BoothyCameraStatusSnapshot | null;
  cameraStatusSnapshotFresh: BoothyCameraStatusSnapshot | null;
}

export function isCameraReadyForCapture({
  cameraStatus,
  cameraStatusSnapshotFresh,
  cameraStatusSnapshot,
}: IsCameraReadyForCaptureParams): boolean {
  if (cameraStatus?.ipcState !== 'connected') {
    return false;
  }

  if (cameraStatusSnapshotFresh) {
    return cameraStatusSnapshotFresh.state === 'ready';
  }

  // Keep capture available across transient getStatus soft-fail reports. The customer lamp already
  // holds its previous green state in this case, so the button should stay aligned with that.
  if (!cameraStatus?.status && cameraStatusSnapshot) {
    return cameraStatusSnapshot.state === 'ready';
  }

  return Boolean(cameraStatus?.status?.connected && cameraStatus?.status?.cameraDetected);
}
