import { describe, expect, it } from 'vitest';

import { getCaptureProgressTimeoutMs, isCaptureInFlight } from '../captureStatus';
import type { BoothyCaptureStatus } from '../components/ui/AppProperties';

describe('isCaptureInFlight', () => {
  it.each<BoothyCaptureStatus>(['capturing', 'transferring', 'stabilizing', 'importing'])(
    'returns true while capture is busy: %s',
    (status) => {
      expect(isCaptureInFlight(status)).toBe(true);
    },
  );

  it.each<BoothyCaptureStatus>(['idle', 'ready', 'error'])('returns false for non-busy states: %s', (status) => {
    expect(isCaptureInFlight(status)).toBe(false);
  });
});

describe('getCaptureProgressTimeoutMs', () => {
  it.each<BoothyCaptureStatus>(['capturing', 'transferring', 'stabilizing', 'importing'])(
    'arms a watchdog timeout for in-flight capture state: %s',
    (status) => {
      expect(getCaptureProgressTimeoutMs(status)).toBeGreaterThan(0);
    },
  );

  it.each<BoothyCaptureStatus>(['idle', 'ready', 'error'])(
    'does not arm a watchdog timeout for terminal capture state: %s',
    (status) => {
      expect(getCaptureProgressTimeoutMs(status)).toBeNull();
    },
  );
});
