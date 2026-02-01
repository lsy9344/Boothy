type TauriMockEnv = {
  DEV?: boolean;
  VITE_BOOTHY_ENABLE_TAURI_MOCKS?: string;
};

export const TAURI_MOCK_QUERY_PARAM = 'boothyMockTauri';

export const detectTauriRuntime = (win: unknown): boolean => {
  if (!win || typeof win !== 'object') return false;
  const w = win as Record<string, unknown> & { __TAURI_INTERNALS__?: unknown };
  return '__TAURI__' in w || typeof w.__TAURI_INTERNALS__ !== 'undefined';
};

export const shouldMockTauri = (params: { env: TauriMockEnv; search: string; isTauriRuntime: boolean }): boolean => {
  if (params.isTauriRuntime) return false;
  if (!params.env?.DEV) return false;

  const envEnabled = String(params.env?.VITE_BOOTHY_ENABLE_TAURI_MOCKS ?? '').trim().toLowerCase() === 'true';
  if (envEnabled) return true;

  try {
    return new URLSearchParams(params.search ?? '').has(TAURI_MOCK_QUERY_PARAM);
  } catch {
    return false;
  }
};

