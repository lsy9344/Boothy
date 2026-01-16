import Button from '../ui/Button';

export type ExportDecisionChoice = 'overwriteAll' | 'continueFromBackground';

interface ExportDecisionModalProps {
  isOpen: boolean;
  onSelect(choice: ExportDecisionChoice): void;
}

export default function ExportDecisionModal({ isOpen, onSelect }: ExportDecisionModalProps) {
  if (!isOpen) {
    return null;
  }

  return (
    <div
      aria-modal="true"
      className="fixed inset-0 z-[60] flex items-center justify-center bg-black/40 backdrop-blur-sm"
      role="dialog"
    >
      <div className="w-full max-w-md rounded-lg bg-surface p-6 shadow-xl">
        <h3 className="text-lg font-semibold text-text-primary">Export Options</h3>
        <p className="mt-2 text-sm text-text-secondary">
          Choose how to export this session&apos;s photos.
        </p>
        <div className="mt-6 space-y-3">
          <Button
            className="w-full justify-center"
            onClick={() => onSelect('overwriteAll')}
          >
            모두 내보내기
          </Button>
          <Button
            className="w-full justify-center bg-surface text-text-primary"
            onClick={() => onSelect('continueFromBackground')}
          >
            이어서 내보내기
          </Button>
        </div>
      </div>
    </div>
  );
}
