import type { BoothyCaptureStatus } from './components/ui/AppProperties';

const CAPTURE_PROGRESS_TIMEOUTS_MS: Partial<Record<BoothyCaptureStatus, number>> = {
  capturing: 15000,
  transferring: 20000,
  stabilizing: 15000,
  importing: 15000,
};

export function isCaptureInFlight(status: BoothyCaptureStatus): boolean {
  return status === 'capturing' || status === 'transferring' || status === 'stabilizing' || status === 'importing';
}

export function getCaptureProgressTimeoutMs(status: BoothyCaptureStatus): number | null {
  return CAPTURE_PROGRESS_TIMEOUTS_MS[status] ?? null;
}
