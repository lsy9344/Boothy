import type { BoothyCameraStatusReport } from '../components/ui/AppProperties';

export type CustomerCameraLampConnectionState = 'connected' | 'disconnected';

export function nextCustomerCameraLampConnectionState({
  prev,
  report,
}: {
  prev: CustomerCameraLampConnectionState;
  report: BoothyCameraStatusReport | null;
}): CustomerCameraLampConnectionState {
  if (!report) {
    return prev;
  }

  if (report.ipcState === 'disconnected') {
    return 'disconnected';
  }

  if (report.status?.connected === false) {
    return 'disconnected';
  }

  // NOTE: In the sidecar, `status.connected` can mean "SDK is up" rather than "camera is usable".
  // For the customer lamp we only want to show green when the camera is actually detected/usable.
  if (
    report.ipcState === 'connected' &&
    report.status?.connected === true &&
    report.status?.cameraDetected === true
  ) {
    return 'connected';
  }

  return prev;
}
