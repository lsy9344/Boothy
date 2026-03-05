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
  const envValue = String(params.env?.VITE_BOOTHY_ENABLE_TAURI_MOCKS ?? '').trim().toLowerCase();
  if (envValue === 'false') return false;

  const envEnabled = envValue === 'true';
  if (envEnabled) return true;

  try {
    const hasQueryOptIn = new URLSearchParams(params.search ?? '').has(TAURI_MOCK_QUERY_PARAM);
    if (hasQueryOptIn) return true;
  } catch {
    // no-op
  }

  // In a plain browser runtime we default to mocks so UI/dev flows don't crash on invoke().
  return true;
};
