import { useCallback, useEffect, useRef, useState } from 'react';
import { ArrowLeft, Cpu, Info, Trash2, Plus, X, SlidersHorizontal, Keyboard, RefreshCw, FolderOpen, CheckSquare } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { relaunch } from '@tauri-apps/plugin-process';
import { motion, AnimatePresence } from 'framer-motion';
import clsx from 'clsx';
import Button from '../ui/Button';
import ConfirmModal from '../modals/ConfirmModal';
import Dropdown, { OptionItem } from '../ui/Dropdown';
import Switch from '../ui/Switch';
import Input from '../ui/Input';
import Slider from '../ui/Slider';
import { ThemeProps, THEMES, DEFAULT_THEME_ID } from '../../utils/themes';
import { Invokes } from '../ui/AppProperties';
import {
  DEFAULT_END_SCREEN_MESSAGE,
  DEFAULT_T_MINUS_5_WARNING_MESSAGE,
  getBoothyEndScreenMessage,
  getBoothyTMinus5WarningMessage,
} from '../../utils/boothySettings';

interface ConfirmModalState {
  confirmText: string;
  confirmVariant: string;
  confirmRequiredText?: string;
  confirmInputLabel?: string;
  confirmInputPlaceholder?: string;
  isOpen: boolean;
  message: string;
  onConfirm(): void;
  title: string;
}

interface DataActionItemProps {
  buttonAction(): void;
  buttonText: string;
  description: any;
  disabled?: boolean;
  icon: any;
  isProcessing: boolean;
  message: string;
  title: string;
}

interface KeybindItemProps {
  description: string;
  keys: Array<string>;
}

interface SettingItemProps {
  children: any;
  description?: string;
  label: string;
}

interface SettingsPanelProps {
  appSettings: any;
  isAdmin?: boolean;
  onBack(): void;
  onLibraryRefresh(): void;
  onSettingsChange(settings: any, meta?: { reason?: string }): void;
  rootPath: string | null;
}

interface StorageDiagnosticsSession {
  base_path: string;
  raw_path: string;
  jpg_path: string;
  session_name?: string;
  session_folder_name?: string;
}

interface StorageDiagnosticsData {
  sessions_root: string;
  active_session: StorageDiagnosticsSession | null;
  drive_free_bytes: number;
  drive_total_bytes: number;
  warning_threshold_bytes: number | null;
  critical_threshold_bytes: number | null;
  captured_at: string;
}

interface StorageDiagnosticsError {
  message?: string;
  diagnostic?: string | null;
  correlationId?: string;
}

interface StorageDiagnosticsResponse {
  ok: boolean;
  data: StorageDiagnosticsData | null;
  error: StorageDiagnosticsError | null;
}

interface CleanupSessionEntry {
  name: string;
  path: string;
  lastModified?: string | null;
  sizeBytes?: number | null;
  isActive: boolean;
  diagnostic?: string | null;
}

interface CleanupSessionsResponse {
  ok: boolean;
  data: CleanupSessionEntry[] | null;
  error: StorageDiagnosticsError | null;
}

interface CleanupDeleteFailure {
  name: string;
  diagnostic: string;
}

interface CleanupDeleteSummary {
  deleted: string[];
  skippedActive: string[];
  skippedInvalid: string[];
  failed: CleanupDeleteFailure[];
}

interface CleanupDeleteResponse {
  ok: boolean;
  data: CleanupDeleteSummary | null;
  error: StorageDiagnosticsError | null;
}

const EXECUTE_TIMEOUT = 3000;

const formatBytes = (value: number | null | undefined) => {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    return 'N/A';
  }

  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let index = 0;
  let result = value;

  while (result >= 1024 && index < units.length - 1) {
    result /= 1024;
    index += 1;
  }

  const precision = result >= 10 || index === 0 ? 0 : 1;
  return `${result.toFixed(precision)} ${units[index]}`;
};

const formatThreshold = (value: number | null | undefined) => {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    return 'Unset (configure in settings.json)';
  }
  return formatBytes(value);
};

const formatTimestamp = (value: string | null | undefined) => {
  if (!value) {
    return 'Not yet refreshed';
  }
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  return parsed.toLocaleString();
};

const normalizeDiagnosticsError = (error: any): StorageDiagnosticsError => {
  if (!error) {
    return { message: 'Request failed.' };
  }
  if (typeof error === 'string') {
    return { message: error };
  }
  if (typeof error.message === 'string') {
    return {
      message: error.message,
      diagnostic: typeof error.diagnostic === 'string' ? error.diagnostic : null,
      correlationId: typeof error.correlationId === 'string' ? error.correlationId : error.correlation_id,
    };
  }
  if (typeof error.error === 'string') {
    return { message: error.error };
  }
  return { message: 'Request failed.' };
};

const adjustmentVisibilityDefaults = {
  sharpening: true,
  presence: true,
  noiseReduction: true,
  chromaticAberration: false,
  negativeConversion: false,
  vignette: true,
  colorCalibration: false,
  grain: true,
};

const resolutions: Array<OptionItem> = [
  { value: 720, label: '720px' },
  { value: 1280, label: '1280px' },
  { value: 1920, label: '1920px' },
  { value: 2560, label: '2560px' },
  { value: 3840, label: '3840px' },
];

const backendOptions: OptionItem[] = [
  { value: 'auto', label: 'Auto' },
  { value: 'vulkan', label: 'Vulkan' },
  { value: 'dx12', label: 'DirectX 12' },
  { value: 'metal', label: 'Metal' },
  { value: 'gl', label: 'OpenGL' },
];

const settingCategories = [
  { id: 'general', label: 'General', icon: SlidersHorizontal },
  { id: 'processing', label: 'Processing', icon: Cpu },
  { id: 'shortcuts', label: 'Shortcuts', icon: Keyboard },
];

const KeybindItem = ({ keys, description }: KeybindItemProps) => (
  <div className="flex justify-between items-center py-2">
    <span className="text-text-secondary text-sm">{description}</span>
    <div className="flex items-center gap-1">
      {keys.map((key: string, index: number) => (
        <kbd
          key={index}
          className="px-2 py-1 text-xs font-sans font-semibold text-text-primary bg-bg-primary border border-border-color rounded-md"
        >
          {key}
        </kbd>
      ))}
    </div>
  </div>
);

const SettingItem = ({ children, description, label }: SettingItemProps) => (
  <div>
    <label className="block text-sm font-medium text-text-primary mb-2">{label}</label>
    {children}
    {description && <p className="text-xs text-text-secondary mt-2">{description}</p>}
  </div>
);

const DataActionItem = ({
  buttonAction,
  buttonText,
  description,
  disabled = false,
  icon,
  isProcessing,
  message,
  title,
}: DataActionItemProps) => (
  <div className="pb-6 border-b border-border-color last:border-b-0 last:pb-0">
    <h3 className="text-sm font-medium text-text-primary mb-2">{title}</h3>
    <p className="text-xs text-text-secondary mb-3">{description}</p>
    <Button variant="destructive" onClick={buttonAction} disabled={isProcessing || disabled}>
      {icon}
      {isProcessing ? 'Processing...' : buttonText}
    </Button>
    {message && <p className="text-sm text-accent mt-3">{message}</p>}
  </div>
);

export default function SettingsPanel({
  appSettings,
  isAdmin,
  onBack,
  onLibraryRefresh,
  onSettingsChange,
  rootPath,
}: SettingsPanelProps) {
  const [isClearing, setIsClearing] = useState(false);
  const [clearMessage, setClearMessage] = useState('');
  const [isClearingCache, setIsClearingCache] = useState(false);
  const [cacheClearMessage, setCacheClearMessage] = useState('');
  const [isClearingTags, setIsClearingTags] = useState(false);
  const [tagsClearMessage, setTagsClearMessage] = useState('');
  const [confirmModalState, setConfirmModalState] = useState<ConfirmModalState>({
    confirmText: 'Confirm',
    confirmVariant: 'primary',
    isOpen: false,
    message: '',
    onConfirm: () => { },
    title: '',
  });
  const [hasInteractedWithLivePreview, setHasInteractedWithLivePreview] = useState(false);
  const [newShortcut, setNewShortcut] = useState('');

  const [processingSettings, setProcessingSettings] = useState({
    editorPreviewResolution: appSettings?.editorPreviewResolution || 1920,
    rawHighlightCompression: appSettings?.rawHighlightCompression ?? 2.5,
    processingBackend: appSettings?.processingBackend || 'auto',
    linuxGpuOptimization: appSettings?.linuxGpuOptimization ?? false,
  });
  const [timelineSettings, setTimelineSettings] = useState({
    endScreenMessage: getBoothyEndScreenMessage(appSettings),
    tMinus5WarningMessage: getBoothyTMinus5WarningMessage(appSettings),
  });
  const [timelineSaveStatus, setTimelineSaveStatus] = useState<'idle' | 'saving' | 'success' | 'error'>('idle');
  const [timelineSaveMessage, setTimelineSaveMessage] = useState('');
  const timelineSaveTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const storageActionTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const cleanupMessageTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [restartRequired, setRestartRequired] = useState(false);
  const [activeCategory, setActiveCategory] = useState('general');
  const [logPath, setLogPath] = useState('');
  const [storageDiagnostics, setStorageDiagnostics] = useState<StorageDiagnosticsData | null>(null);
  const [storageDiagnosticsStatus, setStorageDiagnosticsStatus] = useState<'idle' | 'loading'>('idle');
  const [storageDiagnosticsError, setStorageDiagnosticsError] = useState<StorageDiagnosticsError | null>(null);
  const [storageActionMessage, setStorageActionMessage] = useState('');
  const [storageActionError, setStorageActionError] = useState<StorageDiagnosticsError | null>(null);
  const [cleanupSessions, setCleanupSessions] = useState<CleanupSessionEntry[]>([]);
  const [cleanupStatus, setCleanupStatus] = useState<'idle' | 'loading' | 'deleting'>('idle');
  const [cleanupError, setCleanupError] = useState<StorageDiagnosticsError | null>(null);
  const [cleanupMessage, setCleanupMessage] = useState('');
  const [selectedCleanupSessions, setSelectedCleanupSessions] = useState<string[]>([]);

  useEffect(() => {
    return () => {
      if (storageActionTimeoutRef.current) {
        clearTimeout(storageActionTimeoutRef.current);
        storageActionTimeoutRef.current = null;
      }
      if (cleanupMessageTimeoutRef.current) {
        clearTimeout(cleanupMessageTimeoutRef.current);
        cleanupMessageTimeoutRef.current = null;
      }
    };
  }, []);

  useEffect(() => {
    setProcessingSettings({
      editorPreviewResolution: appSettings?.editorPreviewResolution || 1920,
      rawHighlightCompression: appSettings?.rawHighlightCompression ?? 2.5,
      processingBackend: appSettings?.processingBackend || 'auto',
      linuxGpuOptimization: appSettings?.linuxGpuOptimization ?? false,
    });
    setTimelineSettings({
      endScreenMessage: getBoothyEndScreenMessage(appSettings),
      tMinus5WarningMessage: getBoothyTMinus5WarningMessage(appSettings),
    });
    setTimelineSaveStatus('idle');
    setTimelineSaveMessage('');
    setRestartRequired(false);
  }, [appSettings]);

  useEffect(() => {
    const handleSettingsSaveResult = (event: Event) => {
      const detail = (event as CustomEvent<any>).detail;
      if (!detail || detail.reason !== 'timeline-messages') {
        return;
      }

      if (timelineSaveTimeoutRef.current) {
        clearTimeout(timelineSaveTimeoutRef.current);
        timelineSaveTimeoutRef.current = null;
      }

      if (detail.ok) {
        setTimelineSaveStatus('success');
        setTimelineSaveMessage('Saved.');
      } else {
        setTimelineSaveStatus('error');
        setTimelineSaveMessage(`Save failed: ${detail.error ?? 'Unknown error'}`);
      }

      timelineSaveTimeoutRef.current = setTimeout(() => {
        setTimelineSaveStatus('idle');
        setTimelineSaveMessage('');
        timelineSaveTimeoutRef.current = null;
      }, EXECUTE_TIMEOUT);
    };

    window.addEventListener('boothy:settings-save-result', handleSettingsSaveResult as EventListener);
    return () => {
      window.removeEventListener('boothy:settings-save-result', handleSettingsSaveResult as EventListener);
      if (timelineSaveTimeoutRef.current) {
        clearTimeout(timelineSaveTimeoutRef.current);
        timelineSaveTimeoutRef.current = null;
      }
    };
  }, []);

  useEffect(() => {
    const fetchLogPath = async () => {
      try {
        const path: string = await invoke(Invokes.GetLogFilePath);
        setLogPath(path);
      } catch (error) {
        console.error('Failed to get log file path:', error);
        setLogPath('Could not retrieve log file path.');
      }
    };
    fetchLogPath();
  }, []);

  const refreshStorageDiagnostics = useCallback(async () => {
    if (!isAdmin) {
      return;
    }
    setStorageDiagnosticsStatus('loading');
    setStorageDiagnosticsError(null);
    setStorageActionMessage('');
    setStorageActionError(null);

    try {
      const response = await invoke<StorageDiagnosticsResponse>(Invokes.BoothyGetStorageDiagnostics);
      if (response?.ok && response.data) {
        setStorageDiagnostics(response.data);
        setStorageDiagnosticsError(null);
      } else {
        setStorageDiagnosticsError(normalizeDiagnosticsError(response?.error));
      }
    } catch (error) {
      setStorageDiagnosticsError(normalizeDiagnosticsError(error));
    } finally {
      setStorageDiagnosticsStatus('idle');
    }
  }, [isAdmin]);

  const refreshCleanupSessions = useCallback(async () => {
    if (!isAdmin) {
      return;
    }
    setCleanupStatus('loading');
    setCleanupError(null);

    try {
      const response = await invoke<CleanupSessionsResponse>(Invokes.BoothyListCleanupSessions);
      if (response?.ok && Array.isArray(response.data)) {
        setCleanupSessions(response.data);
        setCleanupError(null);
        setSelectedCleanupSessions((prev) =>
          prev.filter((name) => response.data?.some((entry) => entry.name === name && !entry.isActive)),
        );
      } else {
        setCleanupSessions([]);
        setCleanupError(normalizeDiagnosticsError(response?.error));
      }
    } catch (error) {
      setCleanupSessions([]);
      setCleanupError(normalizeDiagnosticsError(error));
    } finally {
      setCleanupStatus('idle');
    }
  }, [isAdmin]);

  const setCleanupMessageWithTimeout = (message: string) => {
    setCleanupMessage(message);

    if (cleanupMessageTimeoutRef.current) {
      clearTimeout(cleanupMessageTimeoutRef.current);
      cleanupMessageTimeoutRef.current = null;
    }

    if (message) {
      cleanupMessageTimeoutRef.current = setTimeout(() => {
        setCleanupMessage('');
        cleanupMessageTimeoutRef.current = null;
      }, EXECUTE_TIMEOUT);
    }
  };

  const handleOpenSessionsRoot = useCallback(async () => {
    setStorageActionMessage('');
    setStorageActionError(null);

    if (storageActionTimeoutRef.current) {
      clearTimeout(storageActionTimeoutRef.current);
      storageActionTimeoutRef.current = null;
    }

    try {
      await invoke(Invokes.BoothyOpenSessionsRootInExplorer);
      setStorageActionMessage('Explorer opened.');

      storageActionTimeoutRef.current = setTimeout(() => {
        setStorageActionMessage('');
        storageActionTimeoutRef.current = null;
      }, EXECUTE_TIMEOUT);

    } catch (error) {
      setStorageActionError(normalizeDiagnosticsError(error));
    }
  }, []);

  const handleSelectAllCleanupSessions = () => {
    const selectable = cleanupSessions
      .filter((entry) => !entry.isActive)
      .map((entry) => entry.name);
    setSelectedCleanupSessions(selectable);
  };

  const toggleCleanupSelection = (sessionName: string) => {
    setSelectedCleanupSessions((prev) =>
      prev.includes(sessionName)
        ? prev.filter((name) => name !== sessionName)
        : [...prev, sessionName],
    );
  };

  const buildCleanupSummaryMessage = (summary: CleanupDeleteSummary) => {
    const parts: string[] = [];
    if (summary.deleted.length > 0) {
      parts.push(`Deleted ${summary.deleted.length} session${summary.deleted.length === 1 ? '' : 's'}.`);
    }
    if (summary.skippedActive.length > 0) {
      parts.push(
        `Skipped active session${summary.skippedActive.length === 1 ? '' : 's'}: ${summary.skippedActive.join(', ')}.`,
      );
    }
    if (summary.skippedInvalid.length > 0) {
      parts.push(
        `Skipped ${summary.skippedInvalid.length} invalid request${summary.skippedInvalid.length === 1 ? '' : 's'}.`,
      );
    }
    if (summary.failed.length > 0) {
      parts.push(
        `Failed to delete ${summary.failed.length} session${summary.failed.length === 1 ? '' : 's'}.`,
      );
    }
    return parts.join(' ');
  };

  const executeCleanupDelete = async () => {
    if (selectedCleanupSessions.length === 0) {
      return;
    }

    setCleanupStatus('deleting');
    setCleanupError(null);
    setCleanupMessage('');
    if (cleanupMessageTimeoutRef.current) {
      clearTimeout(cleanupMessageTimeoutRef.current);
      cleanupMessageTimeoutRef.current = null;
    }

    try {
      const response = await invoke<CleanupDeleteResponse>(Invokes.BoothyDeleteCleanupSessions, {
        sessionNames: selectedCleanupSessions,
      });
      if (response?.ok && response.data) {
        setCleanupMessageWithTimeout(buildCleanupSummaryMessage(response.data));
        setSelectedCleanupSessions([]);
        await refreshCleanupSessions();
      } else {
        setCleanupError(normalizeDiagnosticsError(response?.error));
      }
    } catch (error) {
      setCleanupError(normalizeDiagnosticsError(error));
    } finally {
      setCleanupStatus('idle');
    }
  };

  const handleCleanupDelete = () => {
    if (selectedCleanupSessions.length === 0) {
      return;
    }
    setConfirmModalState({
      confirmText: 'Delete Sessions',
      confirmVariant: 'destructive',
      confirmRequiredText: 'DELETE',
      confirmInputLabel: 'Type DELETE to confirm.',
      confirmInputPlaceholder: 'DELETE',
      isOpen: true,
      message:
        `You are about to permanently delete ${selectedCleanupSessions.length} session` +
        `${selectedCleanupSessions.length === 1 ? '' : 's'}.\n\n` +
        'This action cannot be undone.',
      onConfirm: executeCleanupDelete,
      title: 'Confirm Session Deletion',
    });
  };

  useEffect(() => {
    if (isAdmin) {
      void refreshStorageDiagnostics();
      void refreshCleanupSessions();
    } else {
      setStorageDiagnostics(null);
      setStorageDiagnosticsError(null);
      setStorageActionMessage('');
      setStorageActionError(null);
      setStorageDiagnosticsStatus('idle');
      setCleanupSessions([]);
      setCleanupError(null);
      setCleanupMessage('');
      setCleanupStatus('idle');
      setSelectedCleanupSessions([]);
    }
  }, [isAdmin, refreshCleanupSessions, refreshStorageDiagnostics]);

  const handleProcessingSettingChange = (key: string, value: any) => {
    setProcessingSettings((prev) => ({ ...prev, [key]: value }));
    if (key === 'processingBackend' || key === 'linuxGpuOptimization') {
      setRestartRequired(true);
    } else {
      onSettingsChange({ ...appSettings, [key]: value });
    }
  };

  const handleSaveAndRelaunch = async () => {
    onSettingsChange({
      ...appSettings,
      ...processingSettings,
    });
    await new Promise((resolve) => setTimeout(resolve, 200));
    await relaunch();
  };

  const effectiveRootPath = rootPath || appSettings?.lastRootPath;

  const executeClearSidecars = async () => {
    setIsClearing(true);
    setClearMessage('Deleting sidecar files, please wait...');
    try {
      const count: number = await invoke(Invokes.ClearAllSidecars, { rootPath: effectiveRootPath });
      setClearMessage(`${count} sidecar files deleted successfully.`);
      onLibraryRefresh();
    } catch (err: any) {
      console.error('Failed to clear sidecars:', err);
      setClearMessage(`Error: ${err}`);
    } finally {
      setTimeout(() => {
        setIsClearing(false);
        setClearMessage('');
      }, EXECUTE_TIMEOUT);
    }
  };

  const handleClearSidecars = () => {
    setConfirmModalState({
      confirmText: 'Delete All Edits',
      confirmVariant: 'destructive',
      isOpen: true,
      message:
        'Are you sure you want to delete all sidecar files?\n\nThis will permanently remove all your edits for all images inside the current base folder and its subfolders.',
      onConfirm: executeClearSidecars,
      title: 'Confirm Deletion',
    });
  };

  const executeClearTags = async () => {
    setIsClearingTags(true);
    setTagsClearMessage('Clearing all tags from sidecar files...');
    try {
      const count: number = await invoke(Invokes.ClearAllTags, { rootPath: effectiveRootPath });
      setTagsClearMessage(`${count} files updated. All non-color tags removed.`);
      onLibraryRefresh();
    } catch (err: any) {
      console.error('Failed to clear tags:', err);
      setTagsClearMessage(`Error: ${err}`);
    } finally {
      setTimeout(() => {
        setIsClearingTags(false);
        setTagsClearMessage('');
      }, EXECUTE_TIMEOUT);
    }
  };

  const handleClearTags = () => {
    setConfirmModalState({
      confirmText: 'Clear Tags',
      confirmVariant: 'destructive',
      isOpen: true,
      message:
        'Are you sure you want to remove all non-color tags from all images in the current base folder?\n\nColor labels will be kept. This action cannot be undone.',
      onConfirm: executeClearTags,
      title: 'Confirm Tag Deletion',
    });
  };

  const shortcutTagVariants = {
    visible: { opacity: 1, scale: 1, transition: { type: 'spring' as const, stiffness: 500, damping: 30 } },
    exit: { opacity: 0, scale: 0.8, transition: { duration: 0.15 } },
  };

  const executeSetTransparent = async (transparent: boolean) => {
    onSettingsChange({ ...appSettings, transparent });
    await relaunch();
  };

  const handleSetTransparent = (transparent: boolean) => {
    setConfirmModalState({
      confirmText: 'Toggle Transparency',
      confirmVariant: 'primary',
      isOpen: true,
      message: `Are you sure you want to ${transparent ? 'enable' : 'disable'} window transparency effects?\n\n${transparent ? 'These effects may reduce application performance.' : ''
        }\n\nThe application will relaunch to make this change.`,
      onConfirm: () => executeSetTransparent(transparent),
      title: 'Confirm Window Transparency',
    });
  };

  const executeClearCache = async () => {
    setIsClearingCache(true);
    setCacheClearMessage('Clearing thumbnail cache...');
    try {
      await invoke(Invokes.ClearThumbnailCache);
      setCacheClearMessage('Thumbnail cache cleared successfully.');
      onLibraryRefresh();
    } catch (err: any) {
      console.error('Failed to clear thumbnail cache:', err);
      setCacheClearMessage(`Error: ${err}`);
    } finally {
      setTimeout(() => {
        setIsClearingCache(false);
        setCacheClearMessage('');
      }, EXECUTE_TIMEOUT);
    }
  };

  const handleClearCache = () => {
    setConfirmModalState({
      confirmText: 'Clear Cache',
      confirmVariant: 'destructive',
      isOpen: true,
      message:
        'Are you sure you want to clear the thumbnail cache?\n\nAll thumbnails will need to be regenerated, which may be slow for large folders.',
      onConfirm: executeClearCache,
      title: 'Confirm Cache Deletion',
    });
  };

  const closeConfirmModal = () => {
    setConfirmModalState({ ...confirmModalState, isOpen: false });
  };

  const handleAddShortcut = () => {
    const shortcuts = appSettings?.taggingShortcuts || [];
    const newTag = newShortcut.trim().toLowerCase();
    if (newTag && !shortcuts.includes(newTag)) {
      const newShortcuts = [...shortcuts, newTag].sort();
      onSettingsChange({ ...appSettings, taggingShortcuts: newShortcuts });
      setNewShortcut('');
    }
  };

  const currentEndMessage = getBoothyEndScreenMessage(appSettings);
  const currentTMinus5Message = getBoothyTMinus5WarningMessage(appSettings);
  const timelineEndMessage = timelineSettings.endScreenMessage ?? '';
  const timelineTMinus5Message = timelineSettings.tMinus5WarningMessage ?? '';
  const trimmedEndMessage = timelineEndMessage.trim();
  const trimmedTMinus5Message = timelineTMinus5Message.trim();
  const isTimelineDirty =
    timelineEndMessage !== currentEndMessage || timelineTMinus5Message !== currentTMinus5Message;
  const isTimelineValid = trimmedEndMessage.length > 0 && trimmedTMinus5Message.length > 0;

  const handleTimelineSave = () => {
    if (!appSettings || !isTimelineValid) {
      return;
    }
    if (timelineSaveTimeoutRef.current) {
      clearTimeout(timelineSaveTimeoutRef.current);
      timelineSaveTimeoutRef.current = null;
    }
    setTimelineSaveStatus('saving');
    setTimelineSaveMessage('Saving...');
    onSettingsChange(
      {
        ...appSettings,
        boothy_end_screen_message: trimmedEndMessage,
        boothy_t_minus_5_warning_message: trimmedTMinus5Message,
      },
      { reason: 'timeline-messages' },
    );
  };

  const handleTimelineCancel = () => {
    setTimelineSettings({
      endScreenMessage: currentEndMessage,
      tMinus5WarningMessage: currentTMinus5Message,
    });
  };

  const handleTimelineRestoreDefaults = () => {
    setTimelineSettings({
      endScreenMessage: DEFAULT_END_SCREEN_MESSAGE,
      tMinus5WarningMessage: DEFAULT_T_MINUS_5_WARNING_MESSAGE,
    });
  };

  const handleRemoveShortcut = (shortcutToRemove: string) => {
    const shortcuts = appSettings?.taggingShortcuts || [];
    const newShortcuts = shortcuts.filter((s: string) => s !== shortcutToRemove);
    onSettingsChange({ ...appSettings, taggingShortcuts: newShortcuts });
  };

  const handleInputKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleAddShortcut();
    }
  };

  const storageSession = storageDiagnostics?.active_session ?? null;
  const storageLastUpdated = formatTimestamp(storageDiagnostics?.captured_at);

  return (
    <>
      <ConfirmModal {...confirmModalState} onClose={closeConfirmModal} />
      <div className="flex flex-col h-full w-full text-text-primary">
        <header className="flex-shrink-0 mb-8 pt-4">
          <div className="flex flex-wrap items-center justify-between gap-y-4 max-w-7xl mx-auto w-full px-6">
            <div className="flex items-center flex-shrink-0">
              <Button
                className="mr-4 hover:bg-surface text-text-primary rounded-full"
                onClick={onBack}
                size="icon"
                variant="ghost"
              >
                <ArrowLeft />
              </Button>
              <h1 className="text-3xl font-bold text-accent whitespace-nowrap">Settings</h1>
            </div>

            <div className="relative flex w-full min-[1200px]:w-[450px] p-2 bg-surface rounded-md">
              {settingCategories.map((category) => (
                <button
                  key={category.id}
                  onClick={() => setActiveCategory(category.id)}
                  className={clsx(
                    'relative flex-1 flex items-center justify-center gap-2 px-3 py-1.5 text-sm font-medium rounded-md transition-colors',
                    {
                      'text-text-primary hover:bg-surface': activeCategory !== category.id,
                      'text-button-text': activeCategory === category.id,
                    },
                  )}
                  style={{ WebkitTapHighlightColor: 'transparent' }}
                >
                  {activeCategory === category.id && (
                    <motion.span
                      layoutId="settings-category-switch-bubble"
                      className="absolute inset-0 z-0 bg-accent"
                      style={{ borderRadius: 6 }}
                      transition={{ type: 'spring', bounce: 0.2, duration: 0.6 }}
                    />
                  )}
                  <span className="relative z-10 flex items-center">
                    <category.icon size={16} className="mr-2 flex-shrink-0" />
                    <span className="truncate">{category.label}</span>
                  </span>
                </button>
              ))}
            </div>
          </div>
        </header>

        <div className="flex-1 overflow-y-auto overflow-x-hidden pr-2 -mr-2 custom-scrollbar">
          <div className="max-w-7xl mx-auto w-full px-6 pb-8">
            <AnimatePresence mode="wait">
              {activeCategory === 'general' && (
                <motion.div
                  key="general"
                  initial={{ opacity: 0, x: 10 }}
                  animate={{ opacity: 1, x: 0 }}
                  exit={{ opacity: 0, x: -10 }}
                  transition={{ duration: 0.2 }}
                  className="grid grid-cols-1 lg:grid-cols-2 gap-6"
                >
                  <div className="p-6 bg-surface rounded-xl shadow-md">
                    <h2 className="text-xl font-semibold mb-6 text-accent">General Settings</h2>
                    <div className="space-y-6">
                      <SettingItem label="Theme" description="Change the look and feel of the application.">
                        <Dropdown
                          onChange={(value: any) => onSettingsChange({ ...appSettings, theme: value })}
                          options={THEMES.map((theme: ThemeProps) => ({ value: theme.id, label: theme.name }))}
                          value={appSettings?.theme || DEFAULT_THEME_ID}
                        />
                      </SettingItem>

                      <SettingItem
                        description="Dynamically changes editor colors based on the current image."
                        label="Editor Theme"
                      >
                        <Switch
                          checked={appSettings?.adaptiveEditorTheme ?? false}
                          id="adaptive-theme-toggle"
                          label="Adaptive Editor Theme"
                          onChange={(checked) => onSettingsChange({ ...appSettings, adaptiveEditorTheme: checked })}
                        />
                      </SettingItem>

                      <SettingItem
                        label="EXIF Library Sorting"
                        description="Read EXIF data (ISO, aperture, etc.) on folder load at the cost of slower folder loading when using EXIF sorting."
                      >
                        <Switch
                          checked={appSettings?.enableExifReading ?? false}
                          id="exif-reading-toggle"
                          label="EXIF Reading"
                          onChange={(checked) => onSettingsChange({ ...appSettings, enableExifReading: checked })}
                        />
                      </SettingItem>

                      <SettingItem
                        description="Enables or disables transparency effects for the application window. Relaunch required."
                        label="Window Effects"
                      >
                        <Switch
                          checked={appSettings?.transparent ?? true}
                          id="window-effects-toggle"
                          label="Transparency"
                          onChange={handleSetTransparent}
                        />
                      </SettingItem>
                    </div>
                  </div>

                  <div className="p-6 bg-surface rounded-xl shadow-md">
                    <h2 className="text-xl font-semibold mb-6 text-accent">Adjustments Visibility</h2>
                    <p className="text-sm text-text-secondary mb-4">
                      Hide adjustment sections you don&apos;t use often to simplify the editing panel. Your settings will
                      be preserved and applied even when hidden.
                    </p>
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-x-6 gap-y-4">
                      {/* Hide noise reduction to stop people from thinking it exists
                    <Switch
                      label="Noise Reduction"
                      checked={appSettings?.adjustmentVisibility?.noiseReduction ?? true}
                      onChange={(checked) =>
                        onSettingsChange({
                          ...appSettings,
                          adjustmentVisibility: {
                            ...(appSettings?.adjustmentVisibility || adjustmentVisibilityDefaults),
                            noiseReduction: checked,
                          },
                        })
                      }
                    /> 
                    */}
                      <Switch
                        label="Chromatic Aberration"
                        checked={appSettings?.adjustmentVisibility?.chromaticAberration ?? false}
                        onChange={(checked) =>
                          onSettingsChange({
                            ...appSettings,
                            adjustmentVisibility: {
                              ...(appSettings?.adjustmentVisibility || adjustmentVisibilityDefaults),
                              chromaticAberration: checked,
                            },
                          })
                        }
                      />
                      <Switch
                        label="Grain"
                        checked={appSettings?.adjustmentVisibility?.grain ?? true}
                        onChange={(checked) =>
                          onSettingsChange({
                            ...appSettings,
                            adjustmentVisibility: {
                              ...(appSettings?.adjustmentVisibility || adjustmentVisibilityDefaults),
                              grain: checked,
                            },
                          })
                        }
                      />
                      <Switch
                        label="Color Calibration"
                        checked={appSettings?.adjustmentVisibility?.colorCalibration ?? true}
                        onChange={(checked) =>
                          onSettingsChange({
                            ...appSettings,
                            adjustmentVisibility: {
                              ...(appSettings?.adjustmentVisibility || adjustmentVisibilityDefaults),
                              colorCalibration: checked,
                            },
                          })
                        }
                      />
                      <Switch
                        label="Negative Conversion"
                        checked={appSettings?.adjustmentVisibility?.negativeConversion ?? false}
                        onChange={(checked) =>
                          onSettingsChange({
                            ...appSettings,
                            adjustmentVisibility: {
                              ...(appSettings?.adjustmentVisibility || adjustmentVisibilityDefaults),
                              negativeConversion: checked,
                            },
                          })
                        }
                      />
                    </div>
                  </div>

                  <div className="p-6 bg-surface rounded-xl shadow-md">
                    <h2 className="text-xl font-semibold mb-6 text-accent">Tagging</h2>
                    <div className="space-y-6">
                      <SettingItem
                        label="Tagging Shortcuts"
                        description="A list of tags that will appear as shortcuts in the tagging context menu."
                      >
                        <div>
                          <div className="flex flex-wrap gap-2 p-2 bg-bg-primary rounded-md min-h-[40px] border border-border-color mb-2 items-center">
                            <AnimatePresence>
                              {(appSettings?.taggingShortcuts || []).length > 0 ? (
                                (appSettings?.taggingShortcuts || []).map((shortcut: string) => (
                                  <motion.div
                                    key={shortcut}
                                    layout
                                    variants={shortcutTagVariants}
                                    initial={false}
                                    animate="visible"
                                    exit="exit"
                                    onClick={() => handleRemoveShortcut(shortcut)}
                                    title={`Remove shortcut "${shortcut}"`}
                                    className="flex items-center gap-1 bg-surface text-text-primary text-sm font-medium px-2 py-1 rounded group cursor-pointer"
                                  >
                                    <span>{shortcut}</span>
                                    <span className="rounded-full group-hover:bg-black/20 p-0.5 transition-colors">
                                      <X size={14} />
                                    </span>
                                  </motion.div>
                                ))
                              ) : (
                                <motion.span
                                  key="no-shortcuts-placeholder"
                                  initial={{ opacity: 0 }}
                                  animate={{ opacity: 1 }}
                                  exit={{ opacity: 0 }}
                                  transition={{ duration: 0.2 }}
                                  className="text-sm text-text-secondary italic px-1 select-none"
                                >
                                  No shortcuts added
                                </motion.span>
                              )}
                            </AnimatePresence>
                          </div>
                          <div className="relative">
                            <Input
                              type="text"
                              value={newShortcut}
                              onChange={(e) => setNewShortcut(e.target.value)}
                              onKeyDown={handleInputKeyDown}
                              placeholder="Add a new shortcut..."
                              className="pr-10"
                            />
                            <button
                              onClick={handleAddShortcut}
                              className="absolute right-1 top-1/2 -translate-y-1/2 p-1.5 rounded-full text-text-secondary hover:text-text-primary hover:bg-surface"
                              title="Add shortcut"
                            >
                              <Plus size={18} />
                            </button>
                          </div>
                        </div>
                      </SettingItem>

                      <div className="pt-6 border-t border-border-color">
                        <div className="space-y-6">
                          <DataActionItem
                            buttonAction={handleClearTags}
                            buttonText="Clear Tags"
                            description="This will remove all non-color tags from your .rrdata files in the current base folder. Color labels will be kept."
                            disabled={!effectiveRootPath}
                            icon={<Trash2 size={16} className="mr-2" />}
                            isProcessing={isClearingTags}
                            message={tagsClearMessage}
                            title="Clear Tags"
                          />
                        </div>
                      </div>
                    </div>
                  </div>

                  <div className="p-6 bg-surface rounded-xl shadow-md">
                    <h2 className="text-xl font-semibold mb-6 text-accent">Session Timeline Messages</h2>
                    <div className="space-y-6">
                      <SettingItem
                        label="End Screen Message"
                        description="Displayed on the end screen after a session finishes."
                      >
                        <Input
                          type="text"
                          value={timelineEndMessage}
                          onChange={(e) => {
                            setTimelineSaveStatus('idle');
                            setTimelineSaveMessage('');
                            setTimelineSettings((prev) => ({ ...prev, endScreenMessage: e.target.value }));
                          }}
                          placeholder="End screen message..."
                        />
                      </SettingItem>

                      <SettingItem
                        label="T-5 Warning Message"
                        description="Shown in the warning modal when five minutes remain."
                      >
                        <Input
                          type="text"
                          value={timelineTMinus5Message}
                          onChange={(e) => {
                            setTimelineSaveStatus('idle');
                            setTimelineSaveMessage('');
                            setTimelineSettings((prev) => ({ ...prev, tMinus5WarningMessage: e.target.value }));
                          }}
                          placeholder="T-5 warning message..."
                        />
                      </SettingItem>

                      {!isTimelineValid && <p className="text-xs text-red-400">Messages cannot be empty.</p>}

                      {timelineSaveMessage && (
                        <div
                          className={clsx(
                            'mb-4 rounded-lg border p-3 text-sm flex items-center gap-2',
                            timelineSaveStatus === 'error' && 'border-red-500/40 bg-red-500/10 text-red-200',
                            timelineSaveStatus === 'success' && 'border-green-500/30 bg-green-500/10 text-green-300',
                            timelineSaveStatus === 'saving' && 'border-blue-500/30 bg-blue-500/10 text-blue-300',
                          )}
                        >
                          <Info size={16} />
                          {timelineSaveMessage}
                        </div>
                      )}

                      <div className="flex flex-wrap items-center justify-end gap-3">
                        <Button
                          className="bg-bg-primary shadow-transparent hover:bg-bg-primary text-white shadow-none focus:outline-none focus:ring-0"
                          disabled={timelineSaveStatus === 'saving'}
                          onClick={handleTimelineRestoreDefaults}
                        >
                          Restore Defaults
                        </Button>
                        <Button
                          className="bg-bg-primary shadow-transparent hover:bg-bg-primary text-white shadow-none focus:outline-none focus:ring-0"
                          disabled={!isTimelineDirty || timelineSaveStatus === 'saving'}
                          onClick={handleTimelineCancel}
                        >
                          Cancel
                        </Button>
                        <Button
                          disabled={!isTimelineDirty || !isTimelineValid || timelineSaveStatus === 'saving'}
                          onClick={handleTimelineSave}
                        >
                          Save
                        </Button>
                      </div>
                    </div>
                  </div>

                  {isAdmin && (
                    <div className="p-6 bg-surface rounded-xl shadow-md lg:col-span-2">
                      <div className="flex items-center justify-between mb-6">
                        <div>
                          <h2 className="text-xl font-semibold text-accent">Admin Storage Diagnostics</h2>
                          <p className="text-sm text-text-secondary mt-1">
                            View disk usage and session paths.
                          </p>
                        </div>
                        <Button
                          onClick={refreshStorageDiagnostics}
                          disabled={storageDiagnosticsStatus === 'loading'}
                          variant="secondary"
                          size="sm"
                        >
                          <RefreshCw
                            size={16}
                            className={clsx('mr-2', storageDiagnosticsStatus === 'loading' && 'animate-spin')}
                          />
                          Refresh
                        </Button>
                      </div>

                      {storageDiagnosticsError && (
                        <div className="mb-6 rounded-lg border border-red-500/40 bg-red-500/10 p-4 text-sm text-red-200">
                          <div className="font-medium flex items-center gap-2">
                            <Info size={16} />
                            {storageDiagnosticsError.message || 'Failed to load storage diagnostics.'}
                          </div>
                          {storageDiagnosticsError.diagnostic && (
                            <div className="mt-2 text-xs font-mono bg-black/20 p-2 rounded">
                              {storageDiagnosticsError.diagnostic}
                            </div>
                          )}
                          {storageDiagnosticsError.correlationId && (
                            <div className="mt-1 text-xs opacity-70">
                              ID: {storageDiagnosticsError.correlationId}
                            </div>
                          )}
                        </div>
                      )}

                      {storageActionError && (
                        <div className="mb-6 rounded-lg border border-red-500/40 bg-red-500/10 p-4 text-sm text-red-200">
                          <div className="font-medium">{storageActionError.message || 'Action failed.'}</div>
                          {storageActionError.diagnostic && (
                            <div className="mt-2 text-xs font-mono bg-black/20 p-2 rounded">
                              {storageActionError.diagnostic}
                            </div>
                          )}
                        </div>
                      )}

                      {!storageActionError && storageActionMessage && (
                        <div className="mb-6 rounded-lg border border-green-500/30 bg-green-500/10 p-3 text-sm text-green-300 flex items-center gap-2">
                          <Info size={16} />
                          {storageActionMessage}
                        </div>
                      )}

                      <div className="space-y-8">
                        {/* Sessions Root */}
                        <SettingItem
                          label="Sessions Root"
                          description={`The base directory where all sessions are stored. Last updated: ${storageLastUpdated}`}
                        >
                          <div className="flex gap-2">
                            <Input
                              type="text"
                              value={storageDiagnostics?.sessions_root || 'Not yet refreshed'}
                              readOnly
                              className="font-mono text-xs flex-1"
                            />
                            <Button onClick={handleOpenSessionsRoot} variant="secondary" title="Open in Explorer">
                              <FolderOpen size={16} />
                            </Button>
                          </div>
                        </SettingItem>

                        {/* Active Session */}
                        <div className="pt-6 border-t border-border-color">
                          <h3 className="text-base font-medium text-text-primary mb-4 flex items-center gap-2">
                            <FolderOpen size={18} className="text-accent" />
                            Active Session
                          </h3>
                          {storageSession ? (
                            <div className="space-y-4">
                              <SettingItem label="Base Path">
                                <Input
                                  value={storageSession.base_path}
                                  readOnly
                                  className="font-mono text-xs"
                                />
                              </SettingItem>
                              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                                <SettingItem label="Raw Path">
                                  <Input
                                    value={storageSession.raw_path}
                                    readOnly
                                    className="font-mono text-xs"
                                  />
                                </SettingItem>
                                <SettingItem label="Jpg Path">
                                  <Input
                                    value={storageSession.jpg_path}
                                    readOnly
                                    className="font-mono text-xs"
                                  />
                                </SettingItem>
                              </div>
                            </div>
                          ) : (
                            <div className="p-4 bg-bg-primary rounded-lg border border-border-color text-center text-text-secondary italic">
                              No active session currently loaded.
                            </div>
                          )}
                        </div>

                        {/* Disk Usage */}
                        <div className="pt-6 border-t border-border-color">
                          <h3 className="text-base font-medium text-text-primary mb-4 flex items-center gap-2">
                            <Cpu size={18} className="text-accent" />
                            Disk Usage
                          </h3>
                          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
                            <SettingItem label="Free Space">
                              <Input
                                value={
                                  storageDiagnostics
                                    ? formatBytes(storageDiagnostics.drive_free_bytes)
                                    : '-'
                                }
                                readOnly
                                className="font-mono text-xs"
                              />
                            </SettingItem>
                            <SettingItem label="Total Space">
                              <Input
                                value={
                                  storageDiagnostics
                                    ? formatBytes(storageDiagnostics.drive_total_bytes)
                                    : '-'
                                }
                                readOnly
                                className="font-mono text-xs"
                              />
                            </SettingItem>
                            <SettingItem
                              label="Warning Threshold"
                              description="Visual warning level"
                            >
                              <Input
                                value={formatThreshold(storageDiagnostics?.warning_threshold_bytes)}
                                readOnly
                                className="font-mono text-xs"
                              />
                            </SettingItem>
                            <SettingItem
                              label="Critical Threshold"
                              description="Lockout threshold"
                            >
                              <Input
                                value={formatThreshold(storageDiagnostics?.critical_threshold_bytes)}
                                readOnly
                                className="font-mono text-xs"
                              />
                            </SettingItem>
                          </div>
                        </div>

                        {/* Cleanup Sessions */}
                        <div className="pt-6 border-t border-border-color">
                          <div className="flex flex-col gap-4 md:flex-row md:items-start md:justify-between mb-4">
                            <div>
                              <h3 className="text-base font-medium text-text-primary mb-1 flex items-center gap-2">
                                <Trash2 size={18} className="text-accent" />
                                Cleanup Sessions
                              </h3>
                              <p className="text-xs text-text-secondary">
                                Select old sessions to permanently remove them from the sessions root.
                              </p>
                            </div>
                            <div className="flex flex-wrap gap-2">
                              <Button
                                onClick={refreshCleanupSessions}
                                disabled={cleanupStatus === 'loading' || cleanupStatus === 'deleting'}
                                variant="secondary"
                              >
                                <RefreshCw
                                  size={16}
                                  className={clsx(
                                    'mr-2',
                                    cleanupStatus === 'loading' && 'animate-spin',
                                  )}
                                />
                                Refresh
                              </Button>
                              <Button
                                onClick={handleSelectAllCleanupSessions}
                                disabled={
                                  cleanupStatus === 'loading' ||
                                  cleanupStatus === 'deleting' ||
                                  cleanupSessions.filter((entry) => !entry.isActive).length === 0
                                }
                                variant="secondary"
                              >
                                <CheckSquare size={16} className="mr-2" />
                                Select All
                              </Button>
                              <Button
                                onClick={handleCleanupDelete}
                                disabled={selectedCleanupSessions.length === 0 || cleanupStatus !== 'idle'}
                                variant="destructive"
                              >
                                <Trash2 size={16} className="mr-2" />
                                Delete Selected
                              </Button>
                            </div>
                          </div>

                          {cleanupError && (
                            <div className="mb-4 rounded-lg border border-red-500/40 bg-red-500/10 p-4 text-sm text-red-200">
                              <div className="font-medium">{cleanupError.message || 'Cleanup failed.'}</div>
                              {cleanupError.diagnostic && (
                                <div className="mt-2 text-xs font-mono bg-black/20 p-2 rounded">
                                  {cleanupError.diagnostic}
                                </div>
                              )}
                            </div>
                          )}

                          {!cleanupError && cleanupMessage && (
                            <div className="mb-4 rounded-lg border border-green-500/30 bg-green-500/10 p-3 text-sm text-green-300 flex items-center gap-2">
                              <Info size={16} />
                              {cleanupMessage}
                            </div>
                          )}

                          {cleanupSessions.length === 0 ? (
                            <div className="p-4 bg-bg-primary rounded-lg border border-border-color text-center text-text-secondary italic">
                              No sessions available for cleanup.
                            </div>
                          ) : (
                            <div className="space-y-3">
                              {cleanupSessions.map((session) => {
                                const isSelected = selectedCleanupSessions.includes(session.name);
                                const isDisabled =
                                  session.isActive || cleanupStatus === 'loading' || cleanupStatus === 'deleting';
                                return (
                                  <div
                                    key={session.name}
                                    className={clsx(
                                      'flex items-start gap-3 rounded-lg border border-border-color bg-bg-primary p-3',
                                      isDisabled && 'opacity-60',
                                    )}
                                  >
                                    <input
                                      type="checkbox"
                                      className="mt-1 h-4 w-4 accent-accent"
                                      checked={isSelected}
                                      disabled={isDisabled}
                                      onChange={() => toggleCleanupSelection(session.name)}
                                    />
                                    <div className="flex-1 space-y-1">
                                      <div className="flex items-center gap-2">
                                        <span className="text-sm font-medium text-text-primary">{session.name}</span>
                                        {session.isActive && (
                                          <span className="text-[10px] uppercase tracking-wide bg-amber-500/20 text-amber-200 px-2 py-0.5 rounded-full">
                                            Active
                                          </span>
                                        )}
                                      </div>
                                      <div className="text-xs text-text-secondary break-all">{session.path}</div>
                                      <div className="text-[11px] text-text-secondary">
                                        {formatTimestamp(session.lastModified ?? null)} · {formatBytes(session.sizeBytes)}
                                      </div>
                                      {session.diagnostic && (
                                        <div className="text-[11px] text-amber-200">{session.diagnostic}</div>
                                      )}
                                    </div>
                                  </div>
                                );
                              })}
                            </div>
                          )}
                        </div>
                      </div>
                    </div>
                  )}
                </motion.div>
              )}

              {activeCategory === 'processing' && (
                <motion.div
                  key="processing"
                  initial={{ opacity: 0, x: 10 }}
                  animate={{ opacity: 1, x: 0 }}
                  exit={{ opacity: 0, x: -10 }}
                  transition={{ duration: 0.2 }}
                  className="grid grid-cols-1 lg:grid-cols-2 gap-6"
                >
                  <div className="p-6 bg-surface rounded-xl shadow-md">
                    <h2 className="text-xl font-semibold mb-6 text-accent">Processing Engine</h2>
                    <div className="space-y-6">
                      <SettingItem
                        description="Higher resolutions provide a sharper preview but may impact performance on less powerful systems."
                        label="Preview Resolution"
                      >
                        <Dropdown
                          onChange={(value: any) => handleProcessingSettingChange('editorPreviewResolution', value)}
                          options={resolutions}
                          value={processingSettings.editorPreviewResolution}
                        />
                      </SettingItem>

                      <SettingItem
                        label="High Quality Zoom"
                        description="Load a higher quality version of the image when zooming in for more detail. Disabling this can improve performance."
                      >
                        <Switch
                          checked={appSettings?.enableZoomHifi ?? true}
                          id="zoom-hifi-toggle"
                          label="Enable High Quality Zoom"
                          onChange={(checked) => onSettingsChange({ ...appSettings, enableZoomHifi: checked })}
                        />
                      </SettingItem>

                      <div className="space-y-4">
                        <SettingItem
                          label="Live Interactive Previews"
                          description="Update the preview immediately while dragging sliders. Disable this if the interface feels laggy during adjustments."
                        >
                          <Switch
                            checked={appSettings?.enableLivePreviews ?? true}
                            id="live-previews-toggle"
                            label="Enable Live Previews"
                            onChange={(checked) => {
                              setHasInteractedWithLivePreview(true);
                              onSettingsChange({ ...appSettings, enableLivePreviews: checked });
                            }}
                          />
                        </SettingItem>

                        <AnimatePresence>
                          {(appSettings?.enableLivePreviews ?? true) && (
                            <motion.div
                              initial={hasInteractedWithLivePreview ? { height: 0, opacity: 0 } : false}
                              animate={{ height: 'auto', opacity: 1 }}
                              exit={{ height: 0, opacity: 0 }}
                              transition={{ duration: 0.3, ease: 'easeInOut' }}
                              className="overflow-hidden"
                            >
                              <div className="pl-4 border-l-2 border-border-color ml-1">
                                <SettingItem
                                  label="High Quality Live Preview"
                                  description="Uses higher resolution and less compression during interaction. Significantly increases GPU load."
                                >
                                  <Switch
                                    checked={appSettings?.enableHighQualityLivePreviews ?? false}
                                    id="hq-live-previews-toggle"
                                    label="Enable High Quality"
                                    onChange={(checked) =>
                                      onSettingsChange({ ...appSettings, enableHighQualityLivePreviews: checked })
                                    }
                                  />
                                </SettingItem>
                              </div>
                            </motion.div>
                          )}
                        </AnimatePresence>
                      </div>

                      <SettingItem
                        label="RAW Highlight Recovery"
                        description="Controls how much detail is recovered from clipped highlights in RAW files. Higher values recover more detail but can introduce purple artefacts."
                      >
                        <Slider
                          label="Amount"
                          min={1}
                          max={10}
                          step={0.1}
                          value={processingSettings.rawHighlightCompression}
                          defaultValue={2.5}
                          onChange={(e: any) =>
                            handleProcessingSettingChange('rawHighlightCompression', parseFloat(e.target.value))
                          }
                        />
                      </SettingItem>

                      <SettingItem
                        label="Processing Backend"
                        description="Select the graphics API. 'Auto' is recommended. May fix crashes on some systems."
                      >
                        <Dropdown
                          onChange={(value: any) => handleProcessingSettingChange('processingBackend', value)}
                          options={backendOptions}
                          value={processingSettings.processingBackend}
                        />
                      </SettingItem>

                      <SettingItem
                        label="Linux Compatibility Mode"
                        description="Enable workarounds for common GPU driver and display server (e.g., Wayland) issues. May improve stability or performance on some systems."
                      >
                        <Switch
                          checked={processingSettings.linuxGpuOptimization}
                          id="gpu-compat-toggle"
                          label="Enable Compatibility Mode"
                          onChange={(checked) => handleProcessingSettingChange('linuxGpuOptimization', checked)}
                        />
                      </SettingItem>

                      {restartRequired && (
                        <>
                          <div className="p-3 bg-blue-900/20 text-blue-300 border border-blue-500/50 rounded-lg text-sm flex items-center gap-3">
                            <Info size={18} />
                            <p>Changes to the processing engine require an application restart to take effect.</p>
                          </div>
                          <div className="flex justify-end">
                            <Button onClick={handleSaveAndRelaunch}>Save & Relaunch</Button>
                          </div>
                        </>
                      )}
                    </div>
                  </div>

                  <div className="p-6 bg-surface rounded-xl shadow-md">
                    <h2 className="text-xl font-semibold mb-6 text-accent">Data Management</h2>
                    <div className="space-y-6">
                      <DataActionItem
                        buttonAction={handleClearSidecars}
                        buttonText="Delete All Edits in Folder"
                        description={
                          <>
                            This will delete all{' '}
                            <code className="bg-bg-primary px-1 rounded text-text-primary">.rrdata</code> files
                            (containing your edits) within the current base folder:
                            <span className="block font-mono text-xs bg-bg-primary p-2 rounded mt-2 break-all border border-border-color">
                              {effectiveRootPath || 'No folder selected'}
                            </span>
                          </>
                        }
                        disabled={!effectiveRootPath}
                        icon={<Trash2 size={16} className="mr-2" />}
                        isProcessing={isClearing}
                        message={clearMessage}
                        title="Clear All Sidecar Files"
                      />

                      <DataActionItem
                        buttonAction={handleClearCache}
                        buttonText="Clear Thumbnail Cache"
                        description="This will delete all cached thumbnail images. They will be regenerated automatically as you browse your library."
                        icon={<Trash2 size={16} className="mr-2" />}
                        isProcessing={isClearingCache}
                        message={cacheClearMessage}
                        title="Clear Thumbnail Cache"
                      />

                      <DataActionItem
                        buttonAction={async () => {
                          if (logPath && !logPath.startsWith('Could not')) {
                            await invoke(Invokes.ShowInFinder, { path: logPath });
                          }
                        }}
                        buttonText="Open Log File"
                        description={
                          <>
                            View the application&apos;s log file for troubleshooting. The log is located at:
                            <span className="block font-mono text-xs bg-bg-primary p-2 rounded mt-2 break-all border border-border-color">
                              {logPath || 'Loading...'}
                            </span>
                          </>
                        }
                        disabled={!logPath || logPath.startsWith('Could not')}
                        icon={<Info size={16} className="mr-2" />}
                        isProcessing={false}
                        message=""
                        title="View Application Logs"
                      />
                    </div>
                  </div>
                </motion.div>
              )}

              {activeCategory === 'shortcuts' && (
                <motion.div
                  key="shortcuts"
                  initial={{ opacity: 0, x: 10 }}
                  animate={{ opacity: 1, x: 0 }}
                  exit={{ opacity: 0, x: -10 }}
                  transition={{ duration: 0.2 }}
                  className="grid grid-cols-1 lg:grid-cols-2 gap-6"
                >
                  <div className="p-6 bg-surface rounded-xl shadow-md h-full">
                    <h2 className="text-xl font-semibold mb-6 text-accent">General Shortcuts</h2>
                    <div className="space-y-4">
                      <div className="divide-y divide-border-color">
                        <KeybindItem keys={['Space', 'Enter']} description="Open selected image" />
                        <KeybindItem keys={['Ctrl/Cmd', '+', 'C']} description="Copy selected adjustments" />
                        <KeybindItem keys={['Ctrl/Cmd', '+', 'V']} description="Paste copied adjustments" />
                        <KeybindItem keys={['Ctrl/Cmd', '+', 'Shift', '+', 'C']} description="Copy selected file(s)" />
                        <KeybindItem
                          description="Paste file(s) to current folder"
                          keys={['Ctrl/Cmd', '+', 'Shift', '+', 'V']}
                        />
                        <KeybindItem keys={['Ctrl/Cmd', '+', 'A']} description="Select all images" />
                        <KeybindItem keys={['Delete']} description="Delete selected file(s)" />
                        <KeybindItem keys={['0-5']} description="Set star rating for selected image(s)" />
                        <KeybindItem keys={['Shift', '+', '0-5']} description="Set color label for selected image(s)" />
                        <KeybindItem keys={['↑', '↓', '←', '→']} description="Navigate images in library" />
                      </div>
                    </div>
                  </div>

                  <div className="p-6 bg-surface rounded-xl shadow-md h-full">
                    <h2 className="text-xl font-semibold mb-6 text-accent">Editor Shortcuts</h2>
                    <div className="space-y-4">
                      <div className="divide-y divide-border-color">
                        <KeybindItem keys={['Esc']} description="Deselect mask, exit crop/fullscreen/editor" />
                        <KeybindItem keys={['Ctrl/Cmd', '+', 'Z']} description="Undo adjustment" />
                        <KeybindItem keys={['Ctrl/Cmd', '+', 'Y']} description="Redo adjustment" />
                        <KeybindItem keys={['Delete']} description="Delete selected mask/patch or image" />
                        <KeybindItem keys={['Space']} description="Cycle zoom (Fit, 2x Fit, 100%)" />
                        <KeybindItem keys={['←', '→']} description="Previous / Next image" />
                        <KeybindItem keys={['↑', '↓']} description="Zoom in / Zoom out (by step)" />
                        <KeybindItem
                          keys={['Shift', '+', 'Mouse Wheel']}
                          description="Adjust slider value by 2 steps"
                        />
                        <KeybindItem keys={['Ctrl/Cmd', '+', '+']} description="Zoom in" />
                        <KeybindItem keys={['Ctrl/Cmd', '+', '-']} description="Zoom out" />
                        <KeybindItem keys={['Ctrl/Cmd', '+', '0']} description="Zoom to fit" />
                        <KeybindItem keys={['Ctrl/Cmd', '+', '1']} description="Zoom to 100%" />
                        <KeybindItem keys={['F']} description="Toggle fullscreen" />
                        <KeybindItem keys={['B']} description="Show original (before/after)" />
                        <KeybindItem keys={['D']} description="Toggle Adjustments panel" />
                        <KeybindItem keys={['R']} description="Toggle Crop panel" />
                        <KeybindItem keys={['M']} description="Toggle Masks panel" />
                        <KeybindItem keys={['P']} description="Toggle Presets panel" />
                        <KeybindItem keys={['I']} description="Toggle Metadata panel" />
                        <KeybindItem keys={['W']} description="Toggle Waveform display" />
                        <KeybindItem keys={['E']} description="Toggle Export panel" />
                      </div>
                    </div>
                  </div>
                </motion.div>
              )}
            </AnimatePresence>
          </div>
        </div>
      </div >
    </>
  );
}
