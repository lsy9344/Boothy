import { useCallback, useEffect, useState } from 'react';
import Button from '../ui/Button';
import Input from '../ui/Input';

interface AdminModeModalProps {
  errorMessage?: string | null;
  hasAdminPassword: boolean;
  isOpen: boolean;
  isProcessing?: boolean;
  onClose(): void;
  onSetPassword(password: string): void;
  onUnlock(password: string): void;
}

export default function AdminModeModal({
  errorMessage,
  hasAdminPassword,
  isOpen,
  isProcessing = false,
  onClose,
  onSetPassword,
  onUnlock,
}: AdminModeModalProps) {
  const [isMounted, setIsMounted] = useState(false);
  const [show, setShow] = useState(false);
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');

  useEffect(() => {
    if (isOpen) {
      setIsMounted(true);
      const timer = setTimeout(() => setShow(true), 10);
      return () => clearTimeout(timer);
    }
    setShow(false);
    const timer = setTimeout(() => {
      setIsMounted(false);
      setPassword('');
      setConfirmPassword('');
    }, 300);
    return () => clearTimeout(timer);
  }, [isOpen]);

  const isSetupMode = !hasAdminPassword;
  const passwordsMatch = !isSetupMode || password === confirmPassword;
  const canSubmit = !!password.trim() && passwordsMatch && !isProcessing;

  const handleSubmit = useCallback(() => {
    if (!canSubmit) {
      return;
    }
    if (isSetupMode) {
      onSetPassword(password.trim());
    } else {
      onUnlock(password.trim());
    }
  }, [canSubmit, isSetupMode, onSetPassword, onUnlock, password]);

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLDivElement>) => {
      if (event.key === 'Enter') {
        event.preventDefault();
        handleSubmit();
      } else if (event.key === 'Escape') {
        event.preventDefault();
        onClose();
      }
    },
    [handleSubmit, onClose],
  );

  if (!isMounted) {
    return null;
  }

  return (
    <div
      aria-modal="true"
      className={`
        fixed inset-0 flex items-center justify-center z-50 
        bg-black/30 backdrop-blur-sm 
        transition-opacity duration-300 ease-in-out
        ${show ? 'opacity-100' : 'opacity-0'}
      `}
      onClick={onClose}
      role="dialog"
    >
      <div
        className={`
          bg-surface rounded-lg shadow-xl p-6 w-full max-w-sm 
          transform transition-all duration-300 ease-out
          ${show ? 'scale-100 opacity-100 translate-y-0' : 'scale-95 opacity-0 -translate-y-4'}
        `}
        onClick={(event: any) => event.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        <h3 className="text-lg font-semibold text-text-primary mb-2">
          {isSetupMode ? 'Set Admin Password' : 'Unlock Admin Mode'}
        </h3>
        <p className="text-sm text-text-secondary mb-4">
          {isSetupMode
            ? 'Create a password to unlock admin controls for this booth.'
            : 'Enter the admin password to unlock advanced controls.'}
        </p>
        <div className="flex flex-col gap-3">
          <Input
            autoFocus
            onChange={(event: any) => setPassword(event.target.value)}
            placeholder="Password"
            type="password"
            value={password}
          />
          {isSetupMode && (
            <Input
              onChange={(event: any) => setConfirmPassword(event.target.value)}
              placeholder="Confirm password"
              type="password"
              value={confirmPassword}
            />
          )}
        </div>
        {!passwordsMatch && (
          <p className="text-xs text-red-400 mt-2">Passwords do not match.</p>
        )}
        {errorMessage && (
          <p className="text-xs text-red-400 mt-2">{errorMessage}</p>
        )}
        <div className="flex justify-end gap-3 mt-6">
          <Button
            className="bg-bg-primary shadow-transparent hover:bg-bg-primary text-white shadow-none focus:outline-none focus:ring-0"
            onClick={onClose}
            variant="ghost"
            tabIndex={0}
          >
            Cancel
          </Button>
          <Button
            className="focus:outline-none focus:ring-0 focus:ring-offset-0"
            disabled={!canSubmit}
            onClick={handleSubmit}
            variant="primary"
          >
            {isProcessing ? 'Working...' : isSetupMode ? 'Set Password' : 'Unlock'}
          </Button>
        </div>
      </div>
    </div>
  );
}
