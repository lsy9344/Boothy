import Button from '../ui/Button';

interface TimelineResetModalProps {
  graceSeconds: number;
  isExporting: boolean;
  isOpen: boolean;
  onPostpone(): void;
  onResetNow(): void;
}

export default function TimelineResetModal({
  graceSeconds,
  isExporting,
  isOpen,
  onPostpone,
  onResetNow,
}: TimelineResetModalProps) {
  if (!isOpen) {
    return null;
  }

  return (
    <div
      aria-modal="true"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm"
      role="dialog"
      onClick={onPostpone}
    >
      <div className="bg-surface rounded-lg shadow-xl p-6 w-full max-w-md" onClick={(e) => e.stopPropagation()}>
        <h3 className="text-lg font-semibold text-text-primary mb-2">Session Reset</h3>
        <p className="text-sm text-text-secondary mb-4">
          The session has reached N:59. You can reset now or postpone for troubleshooting.
        </p>
        {isExporting && (
          <p className="text-xs text-text-secondary mb-4">
            Export is active. Reset will wait for completion (up to {graceSeconds}s) before proceeding.
          </p>
        )}
        <div className="flex justify-end gap-3">
          <Button
            className="bg-bg-primary shadow-transparent hover:bg-bg-primary text-white shadow-none focus:outline-none focus:ring-0"
            onClick={onPostpone}
            variant="ghost"
          >
            Postpone
          </Button>
          <Button onClick={onResetNow}>Reset Now</Button>
        </div>
      </div>
    </div>
  );
}
