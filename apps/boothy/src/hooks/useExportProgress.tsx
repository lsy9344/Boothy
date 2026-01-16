import { useCallback, useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';

export type ExportProgressStatus = 'idle' | 'exporting' | 'complete' | 'error';

export interface ExportProgressState {
  completed: number;
  currentPath: string;
  errorMessage: string;
  status: ExportProgressStatus;
  total: number;
}

const initialState: ExportProgressState = {
  completed: 0,
  currentPath: '',
  errorMessage: '',
  status: 'idle',
  total: 0,
};

export function useExportProgress() {
  const [state, setState] = useState<ExportProgressState>(initialState);

  useEffect(() => {
    let isEffectActive = true;
    const listeners = [
      listen('boothy-export-progress', (event: any) => {
        if (!isEffectActive) {
          return;
        }
        const payload = event?.payload || {};
        setState((prev) => ({
          ...prev,
          completed: Number(payload.completed ?? 0),
          currentPath: String(payload.current_path ?? ''),
          errorMessage: '',
          status: 'exporting',
          total: Number(payload.total ?? prev.total ?? 0),
        }));
      }),
      listen('boothy-export-complete', () => {
        if (!isEffectActive) {
          return;
        }
        setState((prev) => ({
          ...prev,
          status: 'complete',
          completed: prev.total,
          currentPath: '',
          errorMessage: '',
        }));
      }),
      listen('boothy-export-error', (event: any) => {
        if (!isEffectActive) {
          return;
        }
        const payload = event?.payload;
        const message =
          typeof payload === 'string'
            ? payload
            : typeof payload?.message === 'string'
              ? payload.message
              : 'An unknown export error occurred.';
        setState((prev) => ({
          ...prev,
          status: 'error',
          errorMessage: message,
        }));
      }),
    ];

    return () => {
      isEffectActive = false;
      listeners.forEach((promise) => {
        promise.then((unlisten) => unlisten());
      });
    };
  }, []);

  const reset = useCallback(() => setState(initialState), []);

  const setError = useCallback((message: string) => {
    setState((prev) => ({
      ...prev,
      status: 'error',
      errorMessage: message,
    }));
  }, []);

  return {
    reset,
    setError,
    state,
  };
}
