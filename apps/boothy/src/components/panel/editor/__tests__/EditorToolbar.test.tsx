import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import EditorToolbar from '../EditorToolbar';
import { SelectedImage } from '../../../ui/AppProperties';

const createSelectedImage = (): SelectedImage => ({
  exif: {},
  height: 900,
  isRaw: false,
  isReady: true,
  originalUrl: null,
  path: 'C:\\boothy\\image-001.jpg',
  thumbnailUrl: '',
  width: 1200,
});

describe('EditorToolbar', () => {
  it('renders the session countdown when remaining seconds are provided', () => {
    render(
      <EditorToolbar
        canRedo={false}
        canUndo={false}
        isFullScreenLoading={false}
        isLoading={false}
        isWaveformVisible={false}
        onBackToLibrary={() => {}}
        onRedo={() => {}}
        onToggleFullScreen={() => {}}
        onToggleShowOriginal={() => {}}
        onToggleWaveform={() => {}}
        onUndo={() => {}}
        selectedImage={createSelectedImage()}
        showOriginal={false}
        showDateView={false}
        onToggleDateView={() => {}}
        sessionRemainingSeconds={300}
      />,
    );

    const timer = screen.getByRole('timer');
    expect(timer).toHaveTextContent('05:00');
    expect(timer).toHaveAttribute('aria-label', expect.stringContaining('Session time remaining'));
  });
});
