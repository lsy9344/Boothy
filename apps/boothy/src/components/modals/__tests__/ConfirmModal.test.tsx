import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import ConfirmModal from '../ConfirmModal';

describe('ConfirmModal', () => {
  afterEach(() => {
    cleanup();
  });

  it('requires typed confirmation before allowing destructive action', () => {
    const onConfirm = vi.fn();
    render(
      <ConfirmModal
        isOpen={true}
        title="Confirm Delete"
        message="Deleting is permanent."
        confirmText="Delete"
        confirmVariant="destructive"
        confirmRequiredText="DELETE"
        onClose={() => {}}
        onConfirm={onConfirm}
      />,
    );

    const confirmButton = screen.getByRole('button', { name: 'Delete' });
    expect(confirmButton).toBeDisabled();

    const input = screen.getByPlaceholderText('DELETE');
    fireEvent.change(input, { target: { value: 'delete' } });
    expect(confirmButton).toBeDisabled();

    fireEvent.change(input, { target: { value: 'DELETE' } });
    expect(confirmButton).not.toBeDisabled();

    fireEvent.click(confirmButton);
    expect(onConfirm).toHaveBeenCalledTimes(1);
  });
});
