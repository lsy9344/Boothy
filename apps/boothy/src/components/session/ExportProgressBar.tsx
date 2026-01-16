import Button from '../ui/Button';
import { ExportProgressState } from '../../hooks/useExportProgress';

interface ExportProgressBarProps {
  onDismissError(): void;
  state: ExportProgressState;
}

const formatCurrentFile = (path: string) => {
  if (!path) {
    return '';
  }
  return path.split(/[\\/]/).pop() || path;
};

export default function ExportProgressBar({ onDismissError, state }: ExportProgressBarProps) {
  if (state.status === 'idle') {
    return null;
  }

  const { completed, total, currentPath, errorMessage, status } = state;
  const percent = total > 0 ? Math.min(100, Math.round((completed / total) * 100)) : 0;
  const isError = status === 'error';
  const isComplete = status === 'complete';

  return (
    <div className="fixed top-16 left-1/2 z-[55] w-[min(720px,92vw)] -translate-x-1/2">
      <div
        className={`rounded-lg border px-4 py-3 shadow-lg ${
          isError ? 'border-red-500/60 bg-red-900/30' : 'border-surface bg-surface'
        }`}
      >
        <div className="flex items-center justify-between">
          <div className="text-sm font-semibold text-text-primary">
            {isError ? 'Export Failed' : isComplete ? 'Export Complete' : 'Exporting Photos'}
          </div>
          {!isError && (
            <div className="text-xs text-text-tertiary">
              {total > 0 ? `${completed}/${total} photos` : 'Preparing export'}
            </div>
          )}
        </div>
        {!isError && (
          <>
            <div className="mt-2 h-2 w-full rounded-full bg-bg-primary">
              <div
                className="h-2 rounded-full bg-accent transition-all"
                style={{ width: `${percent}%` }}
              />
            </div>
            {currentPath && !isComplete && (
              <div className="mt-2 text-xs text-text-secondary">
                Current: {formatCurrentFile(currentPath)}
              </div>
            )}
          </>
        )}
        {isError && (
          <div className="mt-2 text-sm text-red-200">
            {errorMessage || 'Export failed. Please try again.'}
          </div>
        )}
        {isError && (
          <div className="mt-3 flex justify-end">
            <Button className="bg-surface text-text-primary" onClick={onDismissError}>
              Dismiss
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
