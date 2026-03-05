import { describe, expect, it } from 'vitest';
import { detectTauriRuntime, shouldMockTauri } from '../tauriMockConfig';

describe('tauriMockConfig', () => {
  it('detects tauri runtime via __TAURI__ or __TAURI_INTERNALS__', () => {
    expect(detectTauriRuntime({})).toBe(false);
    expect(detectTauriRuntime({ __TAURI_INTERNALS__: {} })).toBe(true);
    expect(detectTauriRuntime({ __TAURI__: {} })).toBe(true);
  });

  it('enables tauri mocks by default in non-tauri runtime', () => {
    const env = { DEV: true, VITE_BOOTHY_ENABLE_TAURI_MOCKS: '' };
    expect(shouldMockTauri({ env, search: '', isTauriRuntime: false })).toBe(true);
    expect(shouldMockTauri({ env: { DEV: false }, search: '', isTauriRuntime: false })).toBe(true);
  });

  it('allows explicit env opt-in', () => {
    const env = { DEV: true, VITE_BOOTHY_ENABLE_TAURI_MOCKS: 'true' };
    expect(shouldMockTauri({ env, search: '', isTauriRuntime: false })).toBe(true);
  });

  it('allows explicit env opt-out', () => {
    const env = { DEV: true, VITE_BOOTHY_ENABLE_TAURI_MOCKS: 'false' };
    expect(shouldMockTauri({ env, search: '', isTauriRuntime: false })).toBe(false);
  });

  it('enables tauri mocks via query opt-in', () => {
    const env = { DEV: true };
    expect(shouldMockTauri({ env, search: '?boothyMockTauri=1', isTauriRuntime: false })).toBe(true);
  });

  it('never enables tauri mocks when tauri runtime is detected', () => {
    const env = { DEV: true, VITE_BOOTHY_ENABLE_TAURI_MOCKS: 'true' };
    expect(shouldMockTauri({ env, search: '?boothyMockTauri=1', isTauriRuntime: true })).toBe(false);
  });
});
