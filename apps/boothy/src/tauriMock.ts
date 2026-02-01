import { mockConvertFileSrc, mockIPC, mockWindows } from '@tauri-apps/api/mocks';
import { detectTauriRuntime, shouldMockTauri } from './tauriMockConfig';

type MockSession = {
  base_path: string;
  jpg_path: string;
  raw_path: string;
  session_folder_name: string;
  session_name: string;
};

type MockEmit = (event: string, payload?: unknown) => Promise<void>;

declare global {
  interface Window {
    __TAURI_INTERNALS__?: {
      invoke?: (cmd: string, args?: Record<string, unknown>, options?: unknown) => Promise<unknown>;
    };
    __TAURI_MOCK__?: {
      emit: MockEmit;
      session: MockSession;
      startSession: () => Promise<void>;
      triggerExport: (choice: 'overwriteAll' | 'continueFromBackground') => void;
    };
  }
}

// NOTE:
// In Tauri dev, `window.__TAURI_INTERNALS__` can appear to be missing during a WebView reload (e.g. F5),
// which would incorrectly enable mocks and "disconnect" the UI from the real backend/sidecar.
// Mocks are now opt-in only (DEV + env/query flag) to avoid accidentally breaking camera/IPC flows.
const isTauriRuntime = typeof window !== 'undefined' && detectTauriRuntime(window);
const shouldMock = typeof window !== 'undefined'
  ? shouldMockTauri({
      env: import.meta.env,
      search: window.location.search,
      isTauriRuntime,
    })
  : false;

if (shouldMock) {
  mockWindows('main');
  mockConvertFileSrc('windows');

  const defaultSession: MockSession = {
    base_path: 'C:\\Mock\\Session',
    jpg_path: 'C:\\Mock\\Session\\Jpg',
    raw_path: 'C:\\Mock\\Session\\Raw',
    session_folder_name: 'MockSession',
    session_name: 'Mock Session',
  };
  const storageDiagnosticsMock = {
    ok: true,
    data: {
      sessions_root: 'C:\\Mock\\dabi_shoot',
      active_session: defaultSession,
      drive_free_bytes: 128 * 1024 * 1024 * 1024,
      drive_total_bytes: 512 * 1024 * 1024 * 1024,
      warning_threshold_bytes: 50 * 1024 * 1024 * 1024,
      critical_threshold_bytes: 20 * 1024 * 1024 * 1024,
      captured_at: '2026-01-01T00:00:00.000Z',
    },
    error: null,
  };
  const cameraStatusReportMock = {
    ipcState: 'connected',
    lastError: null,
    protocolVersion: '1.0.0',
    requestId: 'req-mock',
    correlationId: 'corr-mock',
    status: {
      connected: true,
      cameraDetected: true,
      sessionDestination: defaultSession.raw_path,
      cameraModel: 'Mock Canon EOS R5',
    },
  };
  const cleanupSessionsMock = {
    ok: true,
    data: [
      {
        name: 'MockSession',
        path: 'C:\\Mock\\dabi_shoot\\MockSession',
        lastModified: '2026-01-01T00:00:00.000Z',
        sizeBytes: 1024 * 1024 * 128,
        isActive: true,
        diagnostic: null,
      },
      {
        name: 'OldSession-2024-12',
        path: 'C:\\Mock\\dabi_shoot\\OldSession-2024-12',
        lastModified: '2025-12-31T23:00:00.000Z',
        sizeBytes: 1024 * 1024 * 512,
        isActive: false,
        diagnostic: null,
      },
    ],
    error: null,
  };

  const emitEvent: MockEmit = async (event, payload) => {
    await window.__TAURI_INTERNALS__?.invoke?.('plugin:event|emit', { event, payload });
  };

  let exportTimeouts: Array<number> = [];
  const clearExportTimeouts = () => {
    exportTimeouts.forEach((id) => window.clearTimeout(id));
    exportTimeouts = [];
  };

  const scheduleExport = (choice: 'overwriteAll' | 'continueFromBackground') => {
    clearExportTimeouts();
    const total = choice === 'overwriteAll' ? 5 : 2;
    const filenames = Array.from({ length: total }, (_, index) => `photo-${index + 1}.CR3`);

    void emitEvent('boothy-export-progress', { completed: 0, total, current_path: '' });

    const step = (completed: number) => {
      if (completed >= total) {
        void emitEvent('boothy-export-complete');
        return;
      }
      const current_path = filenames[completed];
      void emitEvent('boothy-export-progress', { completed: completed + 1, total, current_path });
      exportTimeouts.push(window.setTimeout(() => step(completed + 1), 150));
    };

    exportTimeouts.push(window.setTimeout(() => step(0), 150));
  };

  const triggerMockCapture = () => {
    const correlationId = `corr-mock-${Date.now()}`;
    const filename = `MOCK_${Date.now()}.CR3`;
    const path = `${defaultSession.raw_path}\\${filename}`;

    void emitEvent('boothy-capture-started');

    window.setTimeout(() => {
      void emitEvent('boothy-photo-transferred', {
        path,
        filename,
        fileSize: 2048,
        transferredAt: new Date().toISOString(),
        correlationId,
      });
    }, 300);

    window.setTimeout(() => {
      void emitEvent('boothy-new-photo', { path, correlationId });
    }, 800);
  };

  mockIPC(
    (cmd, args) => {
      switch (cmd) {
        case 'plugin:app|version':
          return '0.0.0-mock';
        case 'plugin:app|name':
          return 'Boothy';
        case 'plugin:app|tauri_version':
          return '2.9.0';
        case 'plugin:os|platform':
          return 'windows';
        case 'plugin:path|home_dir':
          return 'C:\\Users\\Mock';
        case 'plugin:dialog|open':
        case 'plugin:dialog|save':
        case 'plugin:process|relaunch':
          return null;
        case 'plugin:window|is_fullscreen':
        case 'plugin:window|is_maximized':
        case 'plugin:window|is_minimized':
          return false;
        case 'plugin:window|toggle_maximize':
        case 'plugin:window|minimize':
        case 'plugin:window|close':
        case 'plugin:window|set_fullscreen':
        case 'plugin:window|maximize':
        case 'plugin:window|unmaximize':
          return null;
        case 'load_settings':
          return {
            lastRootPath: null,
            theme: 'dark',
            pinnedFolders: [],
          };
        case 'get_folder_tree': {
          const path = (args?.path as string) ?? defaultSession.raw_path;
          return { children: [], is_dir: true, name: 'Session', path };
        }
        case 'get_supported_file_types':
          return { raw: ['.CR3'], nonRaw: ['.JPG'] };
        case 'get_pinned_folder_trees':
          return [];
        case 'list_images_in_dir':
        case 'list_images_recursive':
          return [];
        case 'read_exif_for_paths':
          return {};
        case 'save_settings':
        case 'cancel_thumbnail_generation':
        case 'start_folder_watcher':
          return null;
        case 'boothy_get_mode_state':
          return { mode: 'customer', has_admin_password: false };
        case 'boothy_camera_get_status':
          return cameraStatusReportMock;
        case 'boothy_camera_reconnect':
          return { ok: true, attempts: 1, lastError: null };
        case 'boothy_trigger_capture':
          triggerMockCapture();
          return null;
        case 'boothy_handle_export_decision': {
          const choice = (args?.choice as 'overwriteAll' | 'continueFromBackground') ?? 'continueFromBackground';
          scheduleExport(choice);
          return null;
        }
        case 'boothy_get_storage_diagnostics':
          return storageDiagnosticsMock;
        case 'boothy_open_sessions_root_in_explorer':
          return null;
        case 'boothy_list_cleanup_sessions':
          return cleanupSessionsMock;
        case 'boothy_delete_cleanup_sessions':
          return {
            ok: true,
            data: {
              deleted: (args?.sessionNames as string[]) ?? [],
              skippedActive: [],
              skippedInvalid: [],
              failed: [],
            },
            error: null,
          };
        case 'boothy_log_frontend':
          return null;
        default:
          return null;
      }
    },
    { shouldMockEvents: true },
  );

  window.__TAURI_MOCK__ = {
    emit: emitEvent,
    session: defaultSession,
    startSession: async () => emitEvent('boothy-session-changed', defaultSession),
    triggerExport: scheduleExport,
  };
}
