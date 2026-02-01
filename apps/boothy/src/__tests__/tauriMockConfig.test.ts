import { describe, expect, it } from 'vitest';
import { detectTauriRuntime, shouldMockTauri } from '../tauriMockConfig';

describe('tauriMockConfig', () => {
  it('detects tauri runtime via __TAURI__ or __TAURI_INTERNALS__', () => {
    expect(detectTauriRuntime({})).toBe(false);
    expect(detectTauriRuntime({ __TAURI_INTERNALS__: {} })).toBe(true);
    expect(detectTauriRuntime({ __TAURI__: {} })).toBe(true);
  });

  it('does not enable tauri mocks without explicit opt-in', () => {
    const env = { DEV: true, VITE_BOOTHY_ENABLE_TAURI_MOCKS: '' };
    expect(shouldMockTauri({ env, search: '', isTauriRuntime: false })).toBe(false);
    expect(shouldMockTauri({ env, search: '?something=1', isTauriRuntime: false })).toBe(false);
  });

  it('enables tauri mocks via env opt-in in DEV', () => {
    const env = { DEV: true, VITE_BOOTHY_ENABLE_TAURI_MOCKS: 'true' };
    expect(shouldMockTauri({ env, search: '', isTauriRuntime: false })).toBe(true);
  });

  it('enables tauri mocks via query opt-in in DEV', () => {
    const env = { DEV: true };
    expect(shouldMockTauri({ env, search: '?boothyMockTauri=1', isTauriRuntime: false })).toBe(true);
  });

  it('never enables tauri mocks when tauri runtime is detected', () => {
    const env = { DEV: true, VITE_BOOTHY_ENABLE_TAURI_MOCKS: 'true' };
    expect(shouldMockTauri({ env, search: '?boothyMockTauri=1', isTauriRuntime: true })).toBe(false);
  });
});

