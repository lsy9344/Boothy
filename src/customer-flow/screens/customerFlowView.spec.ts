import { describe, expect, it } from 'vitest';

import { createCaptureReadyView } from '../selectors/captureConfidenceView.js';
import { selectSessionTimeDisplay } from '../../timing-policy/selectors/sessionTimeDisplay.js';
import { resolveCaptureView } from './customerFlowView.js';

describe('resolveCaptureView', () => {
  it('uses the trust copy once the authoritative end time is already available', () => {
    const resolvedView = resolveCaptureView(
      createCaptureReadyView('배경지 - 핑크'),
      selectSessionTimeDisplay('2026-03-08T10:50:00.000Z'),
    );

    expect(resolvedView.endTime).toMatchObject({
      label: '촬영 종료 시간',
      value: '오후 7:50',
      supporting: '이 시간까지 촬영할 수 있어요.',
    });
  });

  it('keeps the trust copy even if later-story alert data is present', () => {
    const resolvedView = resolveCaptureView(
      createCaptureReadyView('배경지 - 핑크', {
        kind: 'warning',
        effectiveTimingRevision: 'session-24:2026-03-08T10:50:00.000Z',
        actualShootEndAt: '2026-03-08T10:50:00.000Z',
        warningAt: '2026-03-08T10:45:00.000Z',
      }),
      selectSessionTimeDisplay('2026-03-08T10:50:00.000Z'),
    );

    expect(resolvedView.endTime).toMatchObject({
      label: '촬영 종료 시간',
      value: '오후 7:50',
      supporting: '이 시간까지 촬영할 수 있어요.',
    });
  });
});
