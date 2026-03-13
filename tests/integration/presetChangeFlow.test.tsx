import { useState } from 'react';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';

import { CaptureScreen } from '../../src/customer-flow/screens/CaptureScreen.js';
import {
  applyPresetSelection,
  closePresetSelector,
  createCaptureFlowState,
  dismissPresetChangeFeedback,
  openPresetSelector,
  type CaptureFlowState,
} from '../../src/session-domain/state/captureFlowState.js';

function CaptureScreenHarness() {
  const [captureState, setCaptureState] = useState<CaptureFlowState>(() =>
    createCaptureFlowState({
      activePresetId: 'warm-tone',
      sessionEndTimeLabel: '오후 7:40',
    }),
  );

  return (
    <CaptureScreen
      captureState={captureState}
      onCapture={() => undefined}
      onClosePresetSelector={() => setCaptureState((current) => closePresetSelector(current))}
      onDismissPresetChangeFeedback={() => setCaptureState((current) => dismissPresetChangeFeedback(current))}
      onOpenPresetSelector={() => setCaptureState((current) => openPresetSelector(current))}
      onSelectPreset={(presetId) =>
        setCaptureState((current) => closePresetSelector(applyPresetSelection(current, presetId)))
      }
      sessionName="홍길동1234"
      view={{
        endTime: {
          label: '촬영 종료',
          value: '오후 7:40',
          supporting: '이 시간까지 촬영할 수 있어요.',
        },
        preset: {
          label: '현재 프리셋',
          value: '웜톤',
        },
        latestPhoto: {
          kind: 'ready',
          title: '첫 번째 촬영',
          supporting: '웜톤으로 저장된 이전 촬영입니다.',
          assetUrl: 'asset://session-local/capture-001',
          alt: '현재 세션의 최신 촬영 사진 미리보기',
        },
        guidance: '리모컨으로 촬영을 계속해 주세요.',
      }}
    />
  );
}

describe('preset change capture flow', () => {
  it('highlights the active preset, returns immediately after change, announces lightweight feedback, and preserves earlier photo metadata', async () => {
    const user = userEvent.setup();

    render(<CaptureScreenHarness />);

    expect(screen.getByRole('heading', { name: '촬영을 이어서 진행해 주세요.' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '프리셋 변경' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '촬영하기' })).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: '프리셋 변경' }));

    const dialog = screen.getByRole('dialog', { name: '프리셋 변경' });
    expect(within(dialog).getByRole('button', { name: /웜톤/i })).toHaveAttribute('aria-pressed', 'true');

    await user.click(within(dialog).getByRole('button', { name: /배경지 - 핑크/i }));

    expect(screen.queryByRole('dialog', { name: '프리셋 변경' })).not.toBeInTheDocument();
    expect(screen.getByText('배경지 - 핑크')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveTextContent('다음 촬영부터 적용됩니다.');

    const latestPhotoPanel = screen.getByRole('region', { name: '최근 사진' });
    expect(within(latestPhotoPanel).getByText('첫 번째 촬영')).toBeInTheDocument();
    expect(within(latestPhotoPanel).getByText('웜톤으로 저장된 이전 촬영입니다.')).toBeInTheDocument();
    expect(within(latestPhotoPanel).getByRole('img', { name: '현재 세션의 최신 촬영 사진 미리보기' })).toHaveAttribute(
      'src',
      'asset://session-local/capture-001',
    );
    expect(screen.getByRole('button', { name: '촬영하기' })).toBeInTheDocument();
  });
});
