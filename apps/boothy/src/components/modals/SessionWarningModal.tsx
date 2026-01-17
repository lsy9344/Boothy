import { useCallback, useEffect, useState } from 'react';
import Button from '../ui/Button';

interface SessionWarningModalProps {
  isBlocking: boolean;
  isOpen: boolean;
  message?: string | null;
  onClose(): void;
}

const FALLBACK_MESSAGE = '세션 종료가 5분 남았습니다.';

export default function SessionWarningModal({ isBlocking, isOpen, message, onClose }: SessionWarningModalProps) {
  const [isMounted, setIsMounted] = useState(false);
  const [show, setShow] = useState(false);

  useEffect(() => {
    if (isOpen) {
      setIsMounted(true);
      const timer = setTimeout(() => setShow(true), 10);
      return () => clearTimeout(timer);
    }
    setShow(false);
    const timer = setTimeout(() => setIsMounted(false), 300);
    return () => clearTimeout(timer);
  }, [isOpen]);

  const handleClose = useCallback(() => {
    onClose();
  }, [onClose]);

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLDivElement>) => {
      if (event.key === 'Enter') {
        event.preventDefault();
        handleClose();
      } else if (!isBlocking && event.key === 'Escape') {
        event.preventDefault();
        handleClose();
      }
    },
    [handleClose, isBlocking],
  );

  if (!isMounted) {
    return null;
  }

  const displayMessage = message && message.trim() ? message : FALLBACK_MESSAGE;

  if (!isBlocking) {
    return (
      <div
        className={`
          fixed top-6 left-1/2 -translate-x-1/2 z-50
          transition-all duration-300 ease-out
          ${show ? 'opacity-100 translate-y-0' : 'opacity-0 -translate-y-2'}
        `}
      >
        <div
          className="bg-surface/95 backdrop-blur-sm rounded-lg shadow-xl border border-border-color px-5 py-4 w-[320px]"
          onKeyDown={handleKeyDown}
          role="dialog"
          aria-modal="false"
        >
          <h3 className="text-sm font-semibold text-text-primary mb-1">Session Ending Soon</h3>
          <p className="text-xs text-text-secondary mb-3">{displayMessage}</p>
          <div className="flex justify-end">
            <Button
              className="focus:outline-none focus:ring-0 focus:ring-offset-0"
              onClick={handleClose}
              variant="ghost"
            >
              Dismiss
            </Button>
          </div>
        </div>
      </div>
    );
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
      role="dialog"
    >
      <div
        className={`
          bg-surface rounded-lg shadow-xl p-6 w-full max-w-sm 
          transform transition-all duration-300 ease-out
          ${show ? 'scale-100 opacity-100 translate-y-0' : 'scale-95 opacity-0 -translate-y-4'}
        `}
        onKeyDown={handleKeyDown}
      >
        <h3 className="text-lg font-semibold text-text-primary mb-2">Session Ending Soon</h3>
        <p className="text-sm text-text-secondary mb-6 whitespace-pre-wrap">{displayMessage}</p>
        <div className="flex justify-end gap-3">
          <Button
            className="focus:outline-none focus:ring-0 focus:ring-offset-0"
            onClick={handleClose}
            variant="primary"
            autoFocus={true}
          >
            확인
          </Button>
        </div>
      </div>
    </div>
  );
}
