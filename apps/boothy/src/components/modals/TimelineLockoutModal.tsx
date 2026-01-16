import Button from '../ui/Button';

interface TimelineLockoutModalProps {
  isOpen: boolean;
  onDismiss(): void;
  onContinue(): void;
}

export default function TimelineLockoutModal({ isOpen, onDismiss, onContinue }: TimelineLockoutModalProps) {
  if (!isOpen) {
    return null;
  }

  return (
    <div
      aria-modal="true"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm"
      role="dialog"
      onClick={onDismiss}
    >
      <div
        className="bg-surface rounded-lg shadow-xl p-6 w-full max-w-md"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="text-lg font-semibold text-text-primary mb-2">Session Locked</h3>
        <p className="text-sm text-text-secondary mb-6">
          The session has reached T-0. Choose whether to keep the workspace locked or continue working in admin mode.
        </p>
        <div className="flex justify-end gap-3">
          <Button
            className="bg-bg-primary shadow-transparent hover:bg-bg-primary text-white shadow-none focus:outline-none focus:ring-0"
            onClick={onDismiss}
            variant="ghost"
          >
            Dismiss
          </Button>
          <Button onClick={onContinue}>Continue Working</Button>
        </div>
      </div>
    </div>
  );
}
