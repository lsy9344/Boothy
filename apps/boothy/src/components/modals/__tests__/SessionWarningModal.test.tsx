import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import SessionWarningModal from '../SessionWarningModal';

describe('SessionWarningModal', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it('blocks escape when in customer mode and closes on enter', () => {
    const onClose = vi.fn();
    render(<SessionWarningModal isBlocking={true} isOpen={true} message="Test" onClose={onClose} />);

    const dialog = screen.getByRole('dialog');
    expect(dialog).toHaveAttribute('aria-modal', 'true');

    const heading = screen.getByText('Session Ending Soon');
    fireEvent.keyDown(heading, { key: 'Escape' });
    expect(onClose).not.toHaveBeenCalled();

    fireEvent.keyDown(heading, { key: 'Enter' });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('allows escape dismissal in admin mode', () => {
    const onClose = vi.fn();
    render(<SessionWarningModal isBlocking={false} isOpen={true} message="Test" onClose={onClose} />);

    const dialog = screen.getByRole('dialog');
    expect(dialog).toHaveAttribute('aria-modal', 'false');

    fireEvent.keyDown(dialog, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('becomes visible within 500ms after opening', async () => {
    render(<SessionWarningModal isBlocking={true} isOpen={true} message="Test" onClose={() => {}} />);

    const dialog = screen.getByRole('dialog');
    expect(dialog.className).toContain('opacity-0');

    await act(async () => {
      vi.advanceTimersByTime(10);
    });
    expect(dialog.className).toContain('opacity-100');
  });
});
