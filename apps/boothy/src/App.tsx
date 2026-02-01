import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { homeDir } from '@tauri-apps/api/path';
import { getCurrentWindow } from '@tauri-apps/api/window';
import debounce from 'lodash.debounce';
import throttle from 'lodash.throttle';
import clsx from 'clsx';
import {
  Aperture,
  Check,
  ClipboardPaste,
  Copy,
  CopyPlus,
  Edit,
  FileEdit,
  Folder,
  FolderInput,
  FolderPlus,
  Images,
  LayoutTemplate,
  Redo,
  RotateCcw,
  Star,
  Save,
  Palette,
  Tag,
  Trash2,
  Undo,
  X,
  Pin,
  PinOff,
  Gauge,
  Grip,
} from 'lucide-react';
import TitleBar from './window/TitleBar';
import MainLibrary from './components/panel/MainLibrary';
import FolderTree from './components/panel/FolderTree';
import Editor from './components/panel/Editor';
import Controls from './components/panel/right/ControlsPanel';
import type { CopiedSectionAdjustments } from './components/panel/right/ControlsPanel';
import { useThumbnails } from './hooks/useThumbnails';
import { ImageDimensions } from './hooks/useImageRenderSize';
import type { UserPreset } from './hooks/usePresets';
import RightPanelSwitcher from './components/panel/right/RightPanelSwitcher';
import MetadataPanel from './components/panel/right/MetadataPanel';
import CropPanel from './components/panel/right/CropPanel';
import PresetsPanel from './components/panel/right/PresetsPanel';
import ExportPanel from './components/panel/right/ExportPanel';
import LibraryExportPanel from './components/panel/right/LibraryExportPanel';
import MasksPanel from './components/panel/right/MasksPanel';
import CameraControlsPanel from './components/panel/CameraControlsPanel';
import BottomBar from './components/panel/BottomBar';
import ExportDecisionModal from './components/session/ExportDecisionModal';
import ExportProgressBar from './components/session/ExportProgressBar';
import SessionCountdown from './components/session/SessionCountdown';
import { ContextMenuProvider, useContextMenu } from './context/ContextMenuContext';
import TaggingSubMenu from './context/TaggingSubMenu';
import AdminModeModal from './components/modals/AdminModeModal';
import CreateFolderModal from './components/modals/CreateFolderModal';
import RenameFolderModal from './components/modals/RenameFolderModal';
import ConfirmModal from './components/modals/ConfirmModal';
import ImportSettingsModal from './components/modals/ImportSettingsModal';
import RenameFileModal from './components/modals/RenameFileModal';
import PanoramaModal from './components/modals/PanoramaModal';
import DenoiseModal from './components/modals/DenoiseModal';
import CollageModal from './components/modals/CollageModal';
import CopyPasteSettingsModal from './components/modals/CopyPasteSettingsModal';
import CullingModal from './components/modals/CullingModal';
import EndScreenModal from './components/modals/EndScreenModal';
import TimelineLockoutModal from './components/modals/TimelineLockoutModal';
import TimelineResetModal from './components/modals/TimelineResetModal';
import SessionWarningModal from './components/modals/SessionWarningModal';
import { useHistoryState } from './hooks/useHistoryState';
import Resizer from './components/ui/Resizer';
import {
  Adjustments,
  Color,
  COLOR_LABELS,
  COPYABLE_ADJUSTMENT_KEYS,
  INITIAL_ADJUSTMENTS,
  MaskContainer,
  normalizeLoadedAdjustments,
  PasteMode,
  CopyPasteSettings,
} from './utils/adjustments';
import { generatePaletteFromImage } from './utils/palette';
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts';
import { useExportProgress } from './hooks/useExportProgress';
import { THEMES, DEFAULT_THEME_ID, ThemeProps } from './utils/themes';
import {
  getBoothyEndScreenMessage,
  getBoothyResetGracePeriodSeconds,
  getBoothyTMinus5WarningMessage,
} from './utils/boothySettings';
import { SubMask, ToolType } from './components/panel/right/Masks';
import { ExportState, IMPORT_TIMEOUT, ImportState, Status } from './components/panel/right/ExportImportProperties';
import {
  AppSettings,
  BrushSettings,
  FilterCriteria,
  Invokes,
  ImageFile,
  Option,
  OPTION_SEPARATOR,
  LibraryViewMode,
  Panel,
  Preset,
  RawStatus,
  SelectedImage,
  SortCriteria,
  SortDirection,
  SupportedTypes,
  Theme,
  TransformState,
  UiVisibility,
  WaveformData,
  Orientation,
  ThumbnailSize,
  ThumbnailAspectRatio,
  CullingSuggestions,
  BoothyCameraStatusReport,
  BoothyCameraStatusSnapshot,
  BoothyCameraReconnectResult,
  BoothyCaptureStatus,
} from './components/ui/AppProperties';
import { nextCustomerCameraLampConnectionState } from './camera/customerCameraLamp';
import { ChannelConfig } from './components/adjustments/Curves';

interface CollapsibleSectionsState {
  basic: boolean;
  color: boolean;
  curves: boolean;
  details: boolean;
  effects: boolean;
}

interface ConfirmModalState {
  confirmText?: string;
  confirmVariant?: string;
  isOpen: boolean;
  message?: string;
  onConfirm?(): void;
  title?: string;
}

interface Metadata {
  adjustments: Adjustments;
  rating: number;
  tags: Array<string> | null;
  version: number;
}

interface MultiSelectOptions {
  onSimpleClick(p: any): void;
  updateLibraryActivePath: boolean;
  shiftAnchor: string | null;
}

interface CollageModalState {
  isOpen: boolean;
  sourceImages: ImageFile[];
}

interface PanoramaModalState {
  error: string | null;
  finalImageBase64: string | null;
  isOpen: boolean;
  progressMessage: string | null;
  stitchingSourcePaths: Array<string>;
}

interface DenoiseModalState {
  isOpen: boolean;
  isProcessing: boolean;
  previewBase64: string | null;
  originalBase64?: string | null;
  error: string | null;
  targetPath: string | null;
  progressMessage: string | null;
}

interface CullingModalState {
  isOpen: boolean;
  suggestions: CullingSuggestions | null;
  progress: { current: number; total: number; stage: string } | null;
  error: string | null;
  pathsToCull: Array<string>;
}

interface LutData {
  size: number;
}

interface SearchCriteria {
  tags: string[];
  text: string;
  mode: 'AND' | 'OR';
}

interface BoothySession {
  base_path: string;
  jpg_path: string;
  raw_path: string;
  session_folder_name: string;
  session_name: string;
}

interface BoothyModeState {
  mode: 'customer' | 'admin';
  has_admin_password: boolean;
}

interface BoothySessionTimerTickPayload {
  remaining_seconds: number;
}

interface BoothyCameraErrorPayload {
  code?: string;
  message?: string;
  diagnostic?: string | null;
  correlationId?: string;
  context?: Record<string, any>;
  severity?: string;
}

type StorageHealthStatus = 'healthy' | 'warning' | 'critical' | 'unknown';

interface BoothyStorageHealthPayload {
  status: StorageHealthStatus;
  freeBytes: number;
  totalBytes: number;
  warningThresholdBytes: number;
  criticalThresholdBytes: number;
  sampledAt: string;
  diagnostic?: string | null;
}

const DEBUG = false;
const STORAGE_CRITICAL_MESSAGE = '디스크 공간이 부족합니다. 직원에게 문의해 주세요.';
const STORAGE_WARNING_MESSAGE = '저장 공간이 부족해지고 있습니다. 직원에게 문의해 주세요.';
const REVOCATION_DELAY = 5000;
const CAMERA_MESSAGE_NEEDS_CONNECTION = '카메라 연결을 확인해 주세요.';
const CAMERA_MESSAGE_PREPARING = '촬영을 준비 중입니다. 잠시만 기다려 주세요.';
const CAMERA_MESSAGE_UNAVAILABLE = '현재 촬영을 사용할 수 없습니다. 관리자에게 문의해 주세요.';
const CAMERA_CUSTOMER_MESSAGES: Record<string, string> = {
  CAMERA_NOT_CONNECTED: CAMERA_MESSAGE_NEEDS_CONNECTION,
  CAMERA_DISCONNECT: CAMERA_MESSAGE_NEEDS_CONNECTION,
  CAMERA_NOT_FOUND: CAMERA_MESSAGE_NEEDS_CONNECTION,
  CAMERA_SETUP_FAILED: CAMERA_MESSAGE_PREPARING,
  CAPTURE_FAILED: CAMERA_MESSAGE_PREPARING,
  FILE_TRANSFER_FAILED: CAMERA_MESSAGE_PREPARING,
  IPC_TIMEOUT: CAMERA_MESSAGE_PREPARING,
  IPC_DISCONNECT: CAMERA_MESSAGE_UNAVAILABLE,
  SIDECAR_START_FAILED: CAMERA_MESSAGE_UNAVAILABLE,
  SIDECAR_CRASH: CAMERA_MESSAGE_UNAVAILABLE,
};

const useDelayedRevokeBlobUrl = (url: string | null | undefined) => {
  const previousUrlRef = useRef<string | null | undefined>(null);

  useEffect(() => {
    if (previousUrlRef.current && previousUrlRef.current !== url) {
      const urlToRevoke = previousUrlRef.current;
      if (urlToRevoke && urlToRevoke.startsWith('blob:')) {
        setTimeout(() => {
          URL.revokeObjectURL(urlToRevoke);
        }, REVOCATION_DELAY);
      }
    }
    previousUrlRef.current = url;
  }, [url]);

  useEffect(() => {
    return () => {
      const finalUrl = previousUrlRef.current;
      if (finalUrl && finalUrl.startsWith('blob:')) {
        URL.revokeObjectURL(finalUrl);
      }
    };
  }, []);
};

const getParentDir = (filePath: string): string => {
  const separator = filePath.includes('/') ? '/' : '\\';
  const lastSeparatorIndex = filePath.lastIndexOf(separator);
  if (lastSeparatorIndex === -1) {
    return '';
  }
  return filePath.substring(0, lastSeparatorIndex);
};

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

const sanitizeSettingsForSave = (settings: any) => {
  if (!settings || typeof settings !== 'object') {
    return settings;
  }

  const isNonEmptyString = (value: unknown): value is string => typeof value === 'string' && value.trim().length > 0;

  const sanitizeStringArray = (value: unknown): string[] =>
    Array.isArray(value) ? value.filter(isNonEmptyString) : [];

  const next: any = { ...settings };

  next.pinnedFolders = sanitizeStringArray(next.pinnedFolders);

  if (next.filterCriteria && typeof next.filterCriteria === 'object') {
    next.filterCriteria = {
      ...next.filterCriteria,
      colors: sanitizeStringArray(next.filterCriteria.colors),
    };
  }

  if (Array.isArray(next.taggingShortcuts)) {
    next.taggingShortcuts = sanitizeStringArray(next.taggingShortcuts);
  }

  if (next.copyPasteSettings && typeof next.copyPasteSettings === 'object') {
    const includedAdjustments = (next.copyPasteSettings as any).includedAdjustments;
    if (Array.isArray(includedAdjustments)) {
      next.copyPasteSettings = {
        ...next.copyPasteSettings,
        includedAdjustments: sanitizeStringArray(includedAdjustments),
      };
    }
  }

  if (next.lastFolderState && typeof next.lastFolderState === 'object') {
    const currentFolderPath = (next.lastFolderState as any).currentFolderPath;
    const expandedFolders = (next.lastFolderState as any).expandedFolders;

    if (!isNonEmptyString(currentFolderPath)) {
      next.lastFolderState = null;
    } else {
      next.lastFolderState = {
        currentFolderPath,
        expandedFolders: sanitizeStringArray(expandedFolders),
      };
    }
  }

  if (next.sortCriteria && typeof next.sortCriteria === 'object') {
    const key = (next.sortCriteria as any).key;
    const order = (next.sortCriteria as any).order;
    if (!isNonEmptyString(key) || !isNonEmptyString(order)) {
      next.sortCriteria = null;
    }
  }

  if (typeof next.boothy_end_screen_message === 'string') {
    next.boothy_end_screen_message = next.boothy_end_screen_message.trim();
  }

  if (typeof next.boothy_t_minus_5_warning_message === 'string') {
    next.boothy_t_minus_5_warning_message = next.boothy_t_minus_5_warning_message.trim();
  }

  return next;
};

type SettingsSaveMeta = {
  reason?: string;
};

const useAsyncThrottle = <T extends unknown[]>(fn: (...args: T) => Promise<void>, deps: any[] = []) => {
  const isProcessing = useRef(false);
  const nextArgs = useRef<T | null>(null);
  const mounted = useRef(true);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      nextArgs.current = null;
    };
  }, []);

  const trigger = useCallback((...args: T) => {
    if (isProcessing.current) {
      nextArgs.current = args;
      return;
    }

    isProcessing.current = true;
    fn(...args).finally(() => {
      if (!mounted.current) return;
      isProcessing.current = false;
      if (nextArgs.current) {
        const argsToProcess = nextArgs.current;
        nextArgs.current = null;
        trigger(...argsToProcess);
      }
    });
  }, deps);

  return useMemo(() => {
    const func: any = trigger;
    func.cancel = () => {
      nextArgs.current = null;
    };
    return func as ((...args: T) => void) & { cancel: () => void };
  }, [trigger]);
};

const isPlainObject = (value: any): value is Record<string, any> => {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
};

const deepEqual = (a: any, b: any): boolean => {
  if (a === b) return true;
  if (typeof a !== typeof b) return false;
  if (a === null || b === null) return a === b;

  if (Array.isArray(a) || Array.isArray(b)) {
    if (!Array.isArray(a) || !Array.isArray(b)) return false;
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i += 1) {
      if (!deepEqual(a[i], b[i])) return false;
    }
    return true;
  }

  if (!isPlainObject(a) || !isPlainObject(b)) {
    return false;
  }

  const aKeys = Object.keys(a);
  const bKeys = Object.keys(b);
  if (aKeys.length !== bKeys.length) return false;

  for (const key of aKeys) {
    if (!Object.prototype.hasOwnProperty.call(b, key)) return false;
    if (!deepEqual(a[key], b[key])) return false;
  }

  return true;
};

const NON_VISUAL_ADJUSTMENT_KEYS = new Set([
  'aspectRatio',
  'lastPresetId',
  'lastPresetName',
  'rating',
  'tags',
  'version',
  'sectionVisibility',
]);

const hasRenderableAdjustments = (loadedAdjustments: Adjustments | null | undefined) => {
  if (!loadedAdjustments) {
    return false;
  }
  if (!isPlainObject(loadedAdjustments)) {
    return true;
  }
  if (loadedAdjustments.is_null) {
    return false;
  }
  const normalized = normalizeLoadedAdjustments(loadedAdjustments);
  return Object.keys(INITIAL_ADJUSTMENTS).some((key) => {
    if (NON_VISUAL_ADJUSTMENT_KEYS.has(key)) {
      return false;
    }
    return !deepEqual(normalized[key as keyof Adjustments], INITIAL_ADJUSTMENTS[key as keyof Adjustments]);
  });
};

const findBestMatchingPreset = (adjustments: any, presets: Array<Preset>): Preset | null => {
  if (!adjustments || presets.length === 0) return null;

  let best: Preset | null = null;
  let bestScore = -1;

  for (const preset of presets) {
    const presetAdjustments = preset?.adjustments || {};
    const keys = Object.keys(presetAdjustments);
    if (keys.length === 0) continue;

    let ok = true;
    for (const key of keys) {
      if (!deepEqual(adjustments[key], presetAdjustments[key])) {
        ok = false;
        break;
      }
    }

    if (ok && keys.length > bestScore) {
      best = preset;
      bestScore = keys.length;
    }
  }

  return best;
};

function App() {
  // Keep snapshot dominance extremely short so transient "connecting/noCamera" snapshots
  // don't linger visually when pull(getStatus) already indicates the camera is ready.
  const CAMERA_SNAPSHOT_TTL_MS = 500;

  const [rootPath, setRootPath] = useState<string | null>(null);
  const [appSettings, setAppSettings] = useState<AppSettings | null>(null);
  const appSettingsRef = useRef<AppSettings | null>(null);
  const settingsSaveSeqRef = useRef(0);
  const [boothySession, setBoothySession] = useState<BoothySession | null>(null);
  const [sessionRemainingSeconds, setSessionRemainingSeconds] = useState<number | null>(null);
  const [boothyMode, setBoothyMode] = useState<'customer' | 'admin'>('customer');
  const [boothyHasAdminPassword, setBoothyHasAdminPassword] = useState(false);
  const [storageHealth, setStorageHealth] = useState<BoothyStorageHealthPayload | null>(null);
  const [cameraStatus, setCameraStatus] = useState<BoothyCameraStatusReport | null>(null);
  const [cameraStatusSnapshot, setCameraStatusSnapshot] = useState<BoothyCameraStatusSnapshot | null>(null);
  const [cameraStatusLoading, setCameraStatusLoading] = useState(false);
  const [cameraStatusError, setCameraStatusError] = useState<string | null>(null);
  const cameraStatusLastOkAtRef = useRef<number | null>(null);
  const cameraStatusFailureStreakRef = useRef(0);
  const cameraStatusLastBriefRef = useRef<string | null>(null);
  const cameraStatusRequestInFlightRef = useRef(false);
  const cameraStatusRefreshQueuedRef = useRef(false);
  const cameraStatusLastRequestAtRef = useRef<number | null>(null);
  const cameraStatusRequestSeqRef = useRef(0);
  const lastCameraSnapshotAtRef = useRef<number | null>(null);
  const statusHintDebounceRef = useRef<number | null>(null);
  const cameraStatusRecoveryPollIntervalRef = useRef<number | null>(null);
  const cameraStatusRecoveryPollSlowSwitchTimeoutRef = useRef<number | null>(null);
  const [cameraReconnectResult, setCameraReconnectResult] = useState<BoothyCameraReconnectResult | null>(null);
  const [isCameraReconnecting, setIsCameraReconnecting] = useState(false);
  const [captureStatus, setCaptureStatus] = useState<BoothyCaptureStatus>('idle');
  const [captureErrorMessage, setCaptureErrorMessage] = useState<string | null>(null);
  const captureStatusTimeoutRef = useRef<number | null>(null);
  const hasBoothySession = Boolean(boothySession?.session_name || boothySession?.session_folder_name);
  const [isAdminModalOpen, setIsAdminModalOpen] = useState(false);
  const [isAdminActionRunning, setIsAdminActionRunning] = useState(false);
  const [adminModalError, setAdminModalError] = useState<string | null>(null);
  const [isStartingSession, setIsStartingSession] = useState(false);
  const [activeView, setActiveView] = useState<'editor' | 'library'>('library');
  const [shouldAutoOpenEditor, setShouldAutoOpenEditor] = useState(false);
  const [isWindowFullScreen, setIsWindowFullScreen] = useState(false);
  const [currentFolderPath, setCurrentFolderPath] = useState<string | null>(null);
  const [expandedFolders, setExpandedFolders] = useState(new Set<string>());
  const [folderTree, setFolderTree] = useState<any>(null);
  const [pinnedFolderTrees, setPinnedFolderTrees] = useState<any[]>([]);
  const [imageList, setImageList] = useState<Array<ImageFile>>([]);
  const [imageRatings, setImageRatings] = useState<Record<string, number>>({});
  const [sortCriteria, setSortCriteria] = useState<SortCriteria>({ key: 'name', order: SortDirection.Ascending });
  const [filterCriteria, setFilterCriteria] = useState<FilterCriteria>({
    colors: [],
    rating: 0,
    rawStatus: RawStatus.All,
  });
  const [supportedTypes, setSupportedTypes] = useState<SupportedTypes | null>(null);
  const [selectedImage, setSelectedImage] = useState<SelectedImage | null>(null);
  const [currentPreset, setCurrentPreset] = useState<Preset | null>(null);
  const currentPresetRef = useRef<Preset | null>(null);
  const defaultPresetLoadedRef = useRef(false);
  const defaultPresetRef = useRef<Preset | null>(null);
  const presetsIndexRef = useRef<Array<Preset>>([]);
  const imageListRef = useRef<Array<ImageFile>>([]);
  const [multiSelectedPaths, setMultiSelectedPaths] = useState<Array<string>>([]);
  const [libraryActivePath, setLibraryActivePath] = useState<string | null>(null);
  const [libraryActiveAdjustments, setLibraryActiveAdjustments] = useState<Adjustments>(INITIAL_ADJUSTMENTS);
  const [finalPreviewUrl, setFinalPreviewUrl] = useState<string | null>(null);
  const [uncroppedAdjustedPreviewUrl, setUncroppedAdjustedPreviewUrl] = useState<string | null>(null);
  const {
    state: historyAdjustments,
    setState: setHistoryAdjustments,
    undo: undoAdjustments,
    redo: redoAdjustments,
    canUndo,
    canRedo,
    resetHistory: resetAdjustmentsHistory,
  } = useHistoryState(INITIAL_ADJUSTMENTS);
  const [adjustments, setLiveAdjustments] = useState<Adjustments>(INITIAL_ADJUSTMENTS);
  const [showOriginal, setShowOriginal] = useState(false);
  const [isTreeLoading, setIsTreeLoading] = useState(false);
  const [isViewLoading, setIsViewLoading] = useState(false);
  const [initialFileToOpen, setInitialFileToOpen] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [histogram, setHistogram] = useState<ChannelConfig | null>(null);
  const [waveform, setWaveform] = useState<WaveformData | null>(null);
  const [isWaveformVisible, setIsWaveformVisible] = useState(false);
  const [uiVisibility, setUiVisibility] = useState<UiVisibility>({
    folderTree: true,
    filmstrip: true,
  });
  const [isSliderDragging, setIsSliderDragging] = useState(false);
  const dragIdleTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [isFullScreen, setIsFullScreen] = useState(false);
  const [isFullScreenLoading, setIsFullScreenLoading] = useState(false);
  const [fullScreenUrl, setFullScreenUrl] = useState<string | null>(null);
  const [isAnimatingTheme, setIsAnimatingTheme] = useState(false);
  const isInitialThemeMount = useRef(true);
  const [theme, setTheme] = useState(DEFAULT_THEME_ID);
  const [adaptivePalette, setAdaptivePalette] = useState<any>(null);
  const [activeRightPanel, setActiveRightPanel] = useState<Panel | null>(Panel.Adjustments);
  const [activeMaskContainerId, setActiveMaskContainerId] = useState<string | null>(null);
  const [activeMaskId, setActiveMaskId] = useState<string | null>(null);
  const [zoom, setZoom] = useState(1);
  const [displaySize, setDisplaySize] = useState<ImageDimensions>({ width: 0, height: 0 });
  const [previewSize, setPreviewSize] = useState<ImageDimensions>({ width: 0, height: 0 });
  const [baseRenderSize, setBaseRenderSize] = useState<ImageDimensions>({ width: 0, height: 0 });
  const [originalSize, setOriginalSize] = useState<ImageDimensions>({ width: 0, height: 0 });
  const [isFullResolution, setIsFullResolution] = useState(false);
  const [fullResolutionUrl, setFullResolutionUrl] = useState<string | null>(null);
  const [isLoadingFullRes, setIsLoadingFullRes] = useState(false);
  const [transformedOriginalUrl, setTransformedOriginalUrl] = useState<string | null>(null);
  const fullResRequestRef = useRef<any>(null);
  const fullResCacheKeyRef = useRef<string | null>(null);

  useDelayedRevokeBlobUrl(finalPreviewUrl);
  useDelayedRevokeBlobUrl(uncroppedAdjustedPreviewUrl);
  useDelayedRevokeBlobUrl(fullScreenUrl);
  useDelayedRevokeBlobUrl(transformedOriginalUrl);
  useDelayedRevokeBlobUrl(selectedImage?.originalUrl);
  currentPresetRef.current = currentPreset;
  useEffect(() => {
    imageListRef.current = imageList;
  }, [imageList]);

  const findFirstPreset = useCallback((presetList: Array<UserPreset>): Preset | null => {
    if (!Array.isArray(presetList)) {
      return null;
    }
    for (const item of presetList) {
      if (item.preset) {
        return item.preset;
      }
      if (item.folder?.children?.length) {
        return item.folder.children[0];
      }
    }
    return null;
  }, []);

  const flattenPresets = useCallback((presetList: Array<UserPreset>): Array<Preset> => {
    if (!Array.isArray(presetList)) {
      return [];
    }
    const flat: Array<Preset> = [];
    for (const item of presetList) {
      if (item.preset) {
        flat.push(item.preset);
      } else if (item.folder?.children?.length) {
        flat.push(...item.folder.children);
      }
    }
    return flat;
  }, []);

  const cachePresets = useCallback(
    (loadedPresets: Array<UserPreset>) => {
      presetsIndexRef.current = flattenPresets(loadedPresets);
      if (!defaultPresetRef.current) {
        defaultPresetRef.current = findFirstPreset(loadedPresets);
      }
    },
    [findFirstPreset, flattenPresets],
  );

  const ensurePresetCache = useCallback(async () => {
    if (defaultPresetRef.current && presetsIndexRef.current.length > 0) {
      return;
    }
    try {
      const loadedPresets: Array<UserPreset> = await invoke(Invokes.LoadPresets);
      cachePresets(loadedPresets);
    } catch (err) {
      console.error('[Presets] Failed to load presets for cache:', err);
    }
  }, [cachePresets]);

  const findNewestAddedPath = useCallback((files: ImageFile[], prevFiles: ImageFile[]) => {
    const prevPaths = new Set(prevFiles.map((file) => file.path));
    const addedFiles = files.filter((file) => !prevPaths.has(file.path));
    if (addedFiles.length === 0) {
      return null;
    }
    let newest = addedFiles[0];
    for (const file of addedFiles) {
      if ((file.modified ?? 0) >= (newest.modified ?? 0)) {
        newest = file;
      }
    }
    return newest.path;
  }, []);

  const handleDisplaySizeChange = useCallback((size: ImageDimensions & { scale?: number }) => {
    setDisplaySize({ width: size.width, height: size.height });

    if (size.scale) {
      const baseWidth = size.width / size.scale;
      const baseHeight = size.height / size.scale;
      setBaseRenderSize({ width: baseWidth, height: baseHeight });
    }
  }, []);

  useEffect(() => {
    if (defaultPresetLoadedRef.current) {
      return;
    }
    defaultPresetLoadedRef.current = true;

    const loadDefaultPreset = async () => {
      try {
        const loadedPresets: Array<UserPreset> = await invoke(Invokes.LoadPresets);
        cachePresets(loadedPresets);
        const firstPreset = findFirstPreset(loadedPresets);
        if (!firstPreset) {
          return;
        }
        if (currentPresetRef.current) {
          return;
        }
        setCurrentPreset(firstPreset);
        currentPresetRef.current = firstPreset;
        defaultPresetRef.current = firstPreset;
        await invoke(Invokes.BoothySetCurrentPreset, {
          presetId: firstPreset.id,
          presetName: firstPreset.name,
          presetAdjustments: firstPreset.adjustments,
        });
        console.log('[PresetsPanel] Default preset set:', firstPreset.name);
      } catch (err) {
        console.error('[PresetsPanel] Failed to set default preset:', err);
      }
    };

    loadDefaultPreset();
  }, [findFirstPreset, cachePresets]);

  const [initialFitScale, setInitialFitScale] = useState<number | null>(null);
  const [renderedRightPanel, setRenderedRightPanel] = useState<Panel | null>(activeRightPanel);
  const [collapsibleSectionsState, setCollapsibleSectionsState] = useState<CollapsibleSectionsState>({
    basic: true,
    color: false,
    curves: true,
    details: false,
    effects: false,
  });
  const [isLibraryExportPanelVisible, setIsLibraryExportPanelVisible] = useState(false);
  const [isExportDecisionModalOpen, setIsExportDecisionModalOpen] = useState(false);
  const [libraryViewMode, setLibraryViewMode] = useState<LibraryViewMode>(LibraryViewMode.Flat);
  const [leftPanelWidth, setLeftPanelWidth] = useState<number>(256);
  const [rightPanelWidth, setRightPanelWidth] = useState<number>(320);
  const [bottomPanelHeight, setBottomPanelHeight] = useState<number>(144);
  const [activeTreeSection, setActiveTreeSection] = useState<string | null>('current');
  const [isResizing, setIsResizing] = useState(false);
  const [thumbnailSize, setThumbnailSize] = useState(ThumbnailSize.Medium);
  const [thumbnailAspectRatio, setThumbnailAspectRatio] = useState(ThumbnailAspectRatio.Cover);
  const [copiedAdjustments, setCopiedAdjustments] = useState<Adjustments | null>(null);
  const [isStraightenActive, setIsStraightenActive] = useState(false);
  const [isWbPickerActive, setIsWbPickerActive] = useState(false);
  const [isSessionLocked, setIsSessionLocked] = useState(false);
  const [isLockoutModalOpen, setIsLockoutModalOpen] = useState(false);
  const [isResetModalOpen, setIsResetModalOpen] = useState(false);
  const [isEndScreenVisible, setIsEndScreenVisible] = useState(false);
  const [isTMinus5ModalOpen, setIsTMinus5ModalOpen] = useState(false);
  const [tMinus5Message, setTMinus5Message] = useState<string | null>(null);
  const [adminOverrideActive, setAdminOverrideActive] = useState(false);
  const [pendingReset, setPendingReset] = useState(false);
  const resetTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isEditingLockedRef = useRef(false);
  const pendingResetRef = useRef(false);
  const resetPostponedRef = useRef(false);
  const storageLockoutRef = useRef(false);

  const sendFrontendLog = useCallback(
    (level: 'debug' | 'info' | 'warn' | 'error', message: string, context?: Record<string, any>) => {
      invoke(Invokes.BoothyLogFrontend, {
        level,
        message,
        context: context ?? null,
      }).catch(() => { });
    },
    [],
  );
  const logAdminOverride = useCallback(
    (action: string) => {
      sendFrontendLog('info', 'timeline-admin-override', {
        action,
        timestamp: new Date().toISOString(),
      });
      setAdminOverrideActive(true);
    },
    [sendFrontendLog],
  );
  const reportLockoutBlockedAction = useCallback(() => {
    setError('Session is locked. Editing is disabled.');
  }, []);
  const [copiedFilePaths, setCopiedFilePaths] = useState<Array<string>>([]);
  const [copiedSectionAdjustments, setCopiedSectionAdjustments] = useState<CopiedSectionAdjustments | null>(null);
  const [copiedMask, setCopiedMask] = useState<MaskContainer | null>(null);
  const [isCopied, setIsCopied] = useState(false);
  const [isPasted, setIsPasted] = useState(false);
  const [searchCriteria, setSearchCriteria] = useState<SearchCriteria>({
    tags: [],
    text: '',
    mode: 'OR',
  });
  const [brushSettings, setBrushSettings] = useState<BrushSettings | null>({
    size: 50,
    feather: 50,
    tool: ToolType.Brush,
  });
  const [isCreateFolderModalOpen, setIsCreateFolderModalOpen] = useState(false);
  const [isRenameFolderModalOpen, setIsRenameFolderModalOpen] = useState(false);
  const [isRenameFileModalOpen, setIsRenameFileModalOpen] = useState(false);
  const [renameTargetPaths, setRenameTargetPaths] = useState<Array<string>>([]);
  const [isImportModalOpen, setIsImportModalOpen] = useState(false);
  const [isCopyPasteSettingsModalOpen, setIsCopyPasteSettingsModalOpen] = useState(false);
  const [importTargetFolder, setImportTargetFolder] = useState<string | null>(null);
  const [importSourcePaths, setImportSourcePaths] = useState<Array<string>>([]);
  const [folderActionTarget, setFolderActionTarget] = useState<string | null>(null);
  const [confirmModalState, setConfirmModalState] = useState<ConfirmModalState>({ isOpen: false });
  const [panoramaModalState, setPanoramaModalState] = useState<PanoramaModalState>({
    error: null,
    finalImageBase64: null,
    isOpen: false,
    progressMessage: '',
    stitchingSourcePaths: [],
  });
  const [denoiseModalState, setDenoiseModalState] = useState<DenoiseModalState>({
    isOpen: false,
    isProcessing: false,
    previewBase64: null,
    error: null,
    targetPath: null,
    progressMessage: null,
  });
  const [cullingModalState, setCullingModalState] = useState<CullingModalState>({
    isOpen: false,
    suggestions: null,
    progress: null,
    error: null,
    pathsToCull: [],
  });
  const [collageModalState, setCollageModalState] = useState<CollageModalState>({
    isOpen: false,
    sourceImages: [],
  });
  const [customEscapeHandler, setCustomEscapeHandler] = useState<(() => void) | null>(null);
  const [isMaskControlHovered, setIsMaskControlHovered] = useState(false);
  const [libraryScrollTop, setLibraryScrollTop] = useState<number>(0);
  const { showContextMenu } = useContextMenu();
  const [thumbnails, setThumbnails] = useState<Record<string, string>>({});
  const { loading: isThumbnailsLoading } = useThumbnails(imageList, setThumbnails);
  const transformWrapperRef = useRef<any>(null);
  const isProgrammaticZoom = useRef(false);
  const isInitialMount = useRef(true);
  const currentFolderPathRef = useRef<string>(currentFolderPath);
  const hasBoothySessionRef = useRef(hasBoothySession);
  const pendingSessionRef = useRef<BoothySession | null>(null);
  const lastAppliedSessionRef = useRef<string | null>(null);
  const selectSubfolderRequestRef = useRef(0);
  const isCustomerMode = boothyMode === 'customer';
  const isStorageLockout = storageHealth?.status === 'critical';
  const isEditingLocked = isSessionLocked || (isCustomerMode && isTMinus5ModalOpen);
  const {
    state: boothyExportProgress,
    reset: resetBoothyExportProgress,
    setError: setBoothyExportError,
  } = useExportProgress();

  const [exportState, setExportState] = useState<ExportState>({
    errorMessage: '',
    progress: { current: 0, total: 0 },
    status: Status.Idle,
  });

  const [importState, setImportState] = useState<ImportState>({
    errorMessage: '',
    path: '',
    progress: { current: 0, total: 0 },
    status: Status.Idle,
  });

  useEffect(() => {
    currentFolderPathRef.current = currentFolderPath;
  }, [currentFolderPath]);

  useEffect(() => {
    hasBoothySessionRef.current = hasBoothySession;
  }, [hasBoothySession]);

  useEffect(() => {
    if (!isCopied) {
      return;
    }
    const timer = setTimeout(() => setIsCopied(false), 1000);
    return () => clearTimeout(timer);
  }, [isCopied]);
  useEffect(() => {
    if (!isPasted) {
      return;
    }
    const timer = setTimeout(() => setIsPasted(false), 1000);
    return () => clearTimeout(timer);
  }, [isPasted]);

  useEffect(() => {
    isEditingLockedRef.current = isEditingLocked;
  }, [isEditingLocked]);

  useEffect(() => {
    storageLockoutRef.current = isStorageLockout;
  }, [isStorageLockout]);

  useEffect(() => {
    pendingResetRef.current = pendingReset;
  }, [pendingReset]);

  const refreshCameraStatus = useCallback(async () => {
    if (cameraStatusRequestInFlightRef.current) {
      const startedAt = cameraStatusLastRequestAtRef.current;
      if (typeof startedAt === 'number' && Date.now() - startedAt > 10000) {
        console.warn('Camera status request appears stuck; allowing a new request.');
        cameraStatusRequestInFlightRef.current = false;
        cameraStatusLastRequestAtRef.current = null;
        setCameraStatusLoading(false);
        setCameraStatusError('카메라 상태 요청이 지연되어 재시도합니다.');
      } else {
        cameraStatusRefreshQueuedRef.current = true;
        return;
      }
    }
    cameraStatusLastRequestAtRef.current = Date.now();
    cameraStatusRequestInFlightRef.current = true;
    const requestSeq = (cameraStatusRequestSeqRef.current += 1);

    setCameraStatusLoading(true);
    try {
      const invokePromise = invoke<BoothyCameraStatusReport>(Invokes.BoothyCameraGetStatus);
      const timeoutMs = 9000;
      const report = await Promise.race<BoothyCameraStatusReport>([
        invokePromise,
        new Promise<BoothyCameraStatusReport>((_resolve, reject) => {
          window.setTimeout(() => reject(new Error('invoke-timeout')), timeoutMs);
        }),
      ]);
      if (cameraStatusRequestSeqRef.current === requestSeq) {
        cameraStatusLastOkAtRef.current = Date.now();
        cameraStatusFailureStreakRef.current = 0;

        const brief = JSON.stringify({
          ipcState: report?.ipcState ?? null,
          connected: report?.status?.connected ?? null,
          cameraDetected: report?.status?.cameraDetected ?? null,
          hasLastError: Boolean(report?.lastError),
        });
        if (cameraStatusLastBriefRef.current !== brief) {
          cameraStatusLastBriefRef.current = brief;
          sendFrontendLog('debug', 'camera-status-refresh-success', JSON.parse(brief));
        }

        setCameraStatus(report);
        setCameraStatusError(null);
        if (
          report?.ipcState !== 'connected' ||
          report?.status?.connected === false ||
          report?.status?.cameraDetected === false
        ) {
          setCameraStatusSnapshot(null);
        }
      }
    } catch (err) {
      console.warn('Failed to fetch camera status:', err);
      if (cameraStatusRequestSeqRef.current === requestSeq) {
        cameraStatusFailureStreakRef.current += 1;
        const now = Date.now();
        const lastOkAt = cameraStatusLastOkAtRef.current;
        const okAgeMs = typeof lastOkAt === 'number' ? now - lastOkAt : null;
        const hardFail = okAgeMs == null || okAgeMs > 15000 || cameraStatusFailureStreakRef.current >= 3;

        sendFrontendLog('warn', 'camera-status-refresh-failed', {
          error: String(err),
          streak: cameraStatusFailureStreakRef.current,
          lastOkAgeMs: okAgeMs,
          hardFail,
        });

        if (hardFail) {
          setCameraStatus(null);
          setCameraStatusSnapshot(null);
          if (err instanceof Error && err.message === 'invoke-timeout') {
            setCameraStatusError('카메라 상태 요청이 시간 초과되었습니다. 잠시 후 다시 시도합니다.');
          } else {
            setCameraStatusError(typeof err === 'string' ? err : '카메라 상태를 가져오지 못했습니다.');
          }
        } else {
          // Soft-fail: keep last known status to avoid red lamp flicker during transient reload/session transitions.
          setCameraStatusError(null);
        }
      }
    } finally {
      if (cameraStatusRequestSeqRef.current === requestSeq) {
        setCameraStatusLoading(false);
        cameraStatusRequestInFlightRef.current = false;
        cameraStatusLastRequestAtRef.current = null;
        if (cameraStatusRefreshQueuedRef.current) {
          cameraStatusRefreshQueuedRef.current = false;
          window.setTimeout(() => {
            void refreshCameraStatus();
          }, 0);
        }
      }
    }
  }, [sendFrontendLog]);

  const clearCaptureStatusTimeout = useCallback(() => {
    if (captureStatusTimeoutRef.current !== null) {
      window.clearTimeout(captureStatusTimeoutRef.current);
      captureStatusTimeoutRef.current = null;
    }
  }, []);

  const setCaptureStatusState = useCallback(
    (
      status: BoothyCaptureStatus,
      options?: { errorMessage?: string | null; resetAfterMs?: number },
    ) => {
      clearCaptureStatusTimeout();
      setCaptureStatus(status);
      if (typeof options?.errorMessage !== 'undefined') {
        setCaptureErrorMessage(options.errorMessage);
      } else if (status !== 'error') {
        setCaptureErrorMessage(null);
      }
      if (options?.resetAfterMs) {
        captureStatusTimeoutRef.current = window.setTimeout(() => {
          setCaptureStatus('idle');
          setCaptureErrorMessage(null);
          captureStatusTimeoutRef.current = null;
        }, options.resetAfterMs);
      }
    },
    [clearCaptureStatusTimeout],
  );

  useEffect(() => () => clearCaptureStatusTimeout(), [clearCaptureStatusTimeout]);

  useEffect(() => {
    if (!hasBoothySession) {
      setCaptureStatus('idle');
      setCaptureErrorMessage(null);
      clearCaptureStatusTimeout();
    }
  }, [hasBoothySession, clearCaptureStatusTimeout]);

  useEffect(() => {
    refreshCameraStatus();
  }, [refreshCameraStatus]);

  const debouncedSetHistory = useMemo(
    () => debounce((newAdjustments) => setHistoryAdjustments(newAdjustments), 300),
    [setHistoryAdjustments],
  );

  const setAdjustments = useCallback(
    (value: any) => {
      setLiveAdjustments((prevAdjustments: Adjustments) => {
        const newAdjustments = typeof value === 'function' ? value(prevAdjustments) : value;
        debouncedSetHistory(newAdjustments);
        return newAdjustments;
      });
    },
    [debouncedSetHistory],
  );

  const handlePresetApplied = useCallback(
    (preset: Preset) => {
      setCurrentPreset(preset);
    },
    [],
  );

  const handleStraighten = useCallback(
    (angleCorrection: number) => {
      if (isEditingLockedRef.current) {
        reportLockoutBlockedAction();
        return;
      }
      setAdjustments((prev: Partial<Adjustments>) => {
        const newRotation = (prev.rotation || 0) + angleCorrection;
        return { ...prev, rotation: newRotation, crop: null };
      });

      setIsStraightenActive(false);
    },
    [setAdjustments, reportLockoutBlockedAction],
  );

  const toggleWbPicker = useCallback(() => {
    setIsWbPickerActive((prev) => !prev);
  }, []);

  const handleWbPicked = useCallback(() => {
    //setIsWbPickerActive(false); // lets keep it active
  }, []);

  useEffect(() => {
    setLiveAdjustments(historyAdjustments);
  }, [historyAdjustments]);

  useEffect(() => {
    if (activeRightPanel !== Panel.Masks || !activeMaskContainerId) {
      setIsMaskControlHovered(false);
    }
  }, [activeRightPanel, activeMaskContainerId]);

  const geometricAdjustmentsKey = useMemo(() => {
    if (!adjustments) return '';
    const { crop, rotation, flipHorizontal, flipVertical, orientationSteps } = adjustments;
    return JSON.stringify({ crop, rotation, flipHorizontal, flipVertical, orientationSteps });
  }, [
    adjustments?.crop,
    adjustments?.rotation,
    adjustments?.flipHorizontal,
    adjustments?.flipVertical,
    adjustments?.orientationSteps,
  ]);

  const visualAdjustmentsKey = useMemo(() => {
    if (!adjustments) return '';
    const { rating: _rating, sectionVisibility: _sectionVisibility, ...visualAdjustments } = adjustments;
    return JSON.stringify(visualAdjustments);
  }, [adjustments]);

  const undo = useCallback(() => {
    if (canUndo) {
      undoAdjustments();
      debouncedSetHistory.cancel();
    }
  }, [canUndo, undoAdjustments, debouncedSetHistory]);
  const redo = useCallback(() => {
    if (canRedo) {
      redoAdjustments();
      debouncedSetHistory.cancel();
    }
  }, [canRedo, redoAdjustments, debouncedSetHistory]);

  useEffect(() => {
    setTransformedOriginalUrl(null);
  }, [geometricAdjustmentsKey, selectedImage?.path]);

  useEffect(() => {
    let isEffectActive = true;
    let objectUrl: string | null = null;

    const generate = async () => {
      if (showOriginal && selectedImage?.path && !transformedOriginalUrl) {
        try {
          const imageData: Uint8Array = await invoke('generate_original_transformed_preview', {
            jsAdjustments: adjustments,
          });
          if (isEffectActive) {
            const blob = new Blob([imageData], { type: 'image/jpeg' });
            objectUrl = URL.createObjectURL(blob);
            setTransformedOriginalUrl(objectUrl);
          }
        } catch (e) {
          if (isEffectActive) {
            console.error('Failed to generate original preview:', e);
            setError('Failed to show original image.');
            setShowOriginal(false);
          }
        }
      }
    };

    generate();

    return () => {
      isEffectActive = false;
    };
  }, [showOriginal, selectedImage?.path, adjustments, transformedOriginalUrl]);

  useEffect(() => {
    if (currentFolderPath) {
      refreshImageList();
    }
  }, [libraryViewMode]);

  const updateSubMask = (subMaskId: string, updatedData: any) => {
    setAdjustments((prev: Adjustments) => ({
      ...prev,
      masks: prev.masks.map((c: MaskContainer) => ({
        ...c,
        subMasks: c.subMasks.map((sm: SubMask) => (sm.id === subMaskId ? { ...sm, ...updatedData } : sm)),
      })),
    }));
  };

  const handleDeleteMaskContainer = useCallback(
    (containerId: string) => {
      setAdjustments((prev: Adjustments) => ({
        ...prev,
        masks: (prev.masks || []).filter((c) => c.id !== containerId),
      }));
      if (activeMaskContainerId === containerId) {
        setActiveMaskContainerId(null);
        setActiveMaskId(null);
      }
    },
    [setAdjustments, activeMaskContainerId],
  );

  const sortedImageList = useMemo(() => {
    let processedList = imageList;

    if (filterCriteria.rawStatus === RawStatus.RawOverNonRaw && supportedTypes) {
      const rawBaseNames = new Set<string>();

      for (const image of imageList) {
        const pathWithoutVC = image.path.split('?vc=')[0];
        const filename = pathWithoutVC.split(/[\\/]/).pop() || '';
        const lastDotIndex = filename.lastIndexOf('.');
        const extension = lastDotIndex !== -1 ? filename.substring(lastDotIndex + 1).toLowerCase() : '';

        if (extension && supportedTypes.raw.includes(extension)) {
          const baseName = lastDotIndex !== -1 ? filename.substring(0, lastDotIndex) : filename;
          const parentDir = getParentDir(pathWithoutVC);
          const uniqueKey = `${parentDir}/${baseName}`;
          rawBaseNames.add(uniqueKey);
        }
      }

      if (rawBaseNames.size > 0) {
        processedList = imageList.filter((image) => {
          const pathWithoutVC = image.path.split('?vc=')[0];
          const filename = pathWithoutVC.split(/[\\/]/).pop() || '';
          const lastDotIndex = filename.lastIndexOf('.');
          const extension = lastDotIndex !== -1 ? filename.substring(lastDotIndex + 1).toLowerCase() : '';

          const isNonRaw = extension && supportedTypes.nonRaw.includes(extension);

          if (isNonRaw) {
            const baseName = lastDotIndex !== -1 ? filename.substring(0, lastDotIndex) : filename;
            const parentDir = getParentDir(pathWithoutVC);
            const uniqueKey = `${parentDir}/${baseName}`;

            if (rawBaseNames.has(uniqueKey)) {
              return false;
            }
          }

          return true;
        });
      }
    }

    const filteredList = processedList.filter((image) => {
      if (filterCriteria.rating > 0) {
        const rating = imageRatings[image.path] || 0;
        if (filterCriteria.rating === 5) {
          if (rating !== 5) return false;
        } else {
          if (rating < filterCriteria.rating) return false;
        }
      }

      if (
        filterCriteria.rawStatus &&
        filterCriteria.rawStatus !== RawStatus.All &&
        filterCriteria.rawStatus !== RawStatus.RawOverNonRaw &&
        supportedTypes
      ) {
        const extension = image.path.split('.').pop()?.toLowerCase() || '';
        const isRaw = supportedTypes.raw?.includes(extension);

        if (filterCriteria.rawStatus === RawStatus.RawOnly && !isRaw) {
          return false;
        }
        if (filterCriteria.rawStatus === RawStatus.NonRawOnly && isRaw) {
          return false;
        }
      }

      if (filterCriteria.colors && filterCriteria.colors.length > 0) {
        const imageColor = (image.tags || []).find((tag: string) => tag.startsWith('color:'))?.substring(6);

        const hasMatchingColor = imageColor && filterCriteria.colors.includes(imageColor);
        const matchesNone = !imageColor && filterCriteria.colors.includes('none');

        if (!hasMatchingColor && !matchesNone) {
          return false;
        }
      }

      return true;
    });

    const { tags: searchTags, text: searchText, mode: searchMode } = searchCriteria;
    const lowerCaseSearchText = searchText.trim().toLowerCase();

    const filteredBySearch =
      searchTags.length === 0 && lowerCaseSearchText === ''
        ? filteredList
        : filteredList.filter((image: ImageFile) => {
          const lowerCaseImageTags = (image.tags || []).map((t) => t.toLowerCase().replace('user:', ''));
          const filename = image?.path?.split(/[\\/]/)?.pop()?.toLowerCase() || '';

          let tagsMatch = true;
          if (searchTags.length > 0) {
            const lowerCaseSearchTags = searchTags.map((t) => t.toLowerCase());
            if (searchMode === 'OR') {
              tagsMatch = lowerCaseSearchTags.some((searchTag) =>
                lowerCaseImageTags.some((imgTag) => imgTag.includes(searchTag)),
              );
            } else {
              tagsMatch = lowerCaseSearchTags.every((searchTag) =>
                lowerCaseImageTags.some((imgTag) => imgTag.includes(searchTag)),
              );
            }
          }

          let textMatch = true;
          if (lowerCaseSearchText !== '') {
            textMatch =
              filename.includes(lowerCaseSearchText) ||
              lowerCaseImageTags.some((t) => t.includes(lowerCaseSearchText));
          }

          return tagsMatch && textMatch;
        });

    const list = [...filteredBySearch];

    const parseShutter = (val: string | undefined): number | null => {
      if (!val) return null;
      const cleanVal = val.replace(/s/i, '').trim();
      const parts = cleanVal.split('/');
      if (parts.length === 2) {
        const num = parseFloat(parts[0]);
        const den = parseFloat(parts[1]);
        return den !== 0 ? num / den : null;
      }
      const numVal = parseFloat(cleanVal);
      return isNaN(numVal) ? null : numVal;
    };

    const parseAperture = (val: string | undefined): number | null => {
      if (!val) return null;
      const match = val.match(/(\d+(\.\d+)?)/);
      const numVal = match ? parseFloat(match[0]) : null;
      return numVal === null || isNaN(numVal) ? null : numVal;
    };

    const parseFocalLength = (val: string | undefined): number | null => {
      if (!val) return null;
      const match = val.match(/(\d+(\.\d+)?)/);
      if (!match) return null;
      const numVal = parseFloat(match[0]);
      return isNaN(numVal) ? null : numVal;
    };

    list.sort((a, b) => {
      const { key, order } = sortCriteria;
      let comparison = 0;

      const compareNullable = (valA: any, valB: any) => {
        if (valA !== null && valB !== null) {
          if (valA < valB) return -1;
          if (valA > valB) return 1;
          return 0;
        }
        if (valA !== null) return -1;
        if (valB !== null) return 1;
        return 0;
      };

      switch (key) {
        case 'date_taken': {
          const dateA = a.exif?.DateTimeOriginal;
          const dateB = b.exif?.DateTimeOriginal;
          comparison = compareNullable(dateA, dateB);
          if (comparison === 0) comparison = a.modified - b.modified;
          break;
        }
        case 'iso': {
          const getIso = (exif: { [key: string]: string } | null): number | null => {
            if (!exif) return null;
            const isoStr = exif.PhotographicSensitivity || exif.ISOSpeedRatings;
            if (!isoStr) return null;
            const isoNum = parseInt(isoStr, 10);
            return isNaN(isoNum) ? null : isoNum;
          };
          const isoA = getIso(a.exif);
          const isoB = getIso(b.exif);
          comparison = compareNullable(isoA, isoB);
          break;
        }
        case 'shutter_speed': {
          const shutterA = parseShutter(a.exif?.ExposureTime);
          const shutterB = parseShutter(b.exif?.ExposureTime);
          comparison = compareNullable(shutterA, shutterB);
          break;
        }
        case 'aperture': {
          const apertureA = parseAperture(a.exif?.FNumber);
          const apertureB = parseAperture(b.exif?.FNumber);
          comparison = compareNullable(apertureA, apertureB);
          break;
        }
        case 'focal_length': {
          const focalA = parseFocalLength(a.exif?.FocalLength);
          const focalB = parseFocalLength(b.exif?.FocalLength);
          comparison = compareNullable(focalA, focalB);
          break;
        }
        case 'date':
          comparison = a.modified - b.modified;
          break;
        case 'rating':
          comparison = (imageRatings[a.path] || 0) - (imageRatings[b.path] || 0);
          break;
        default:
          comparison = a.path.localeCompare(b.path);
          break;
      }

      return order === SortDirection.Ascending ? comparison : -comparison;
    });
    return list;
  }, [imageList, sortCriteria, imageRatings, filterCriteria, supportedTypes, searchCriteria, appSettings]);

  const applyAdjustments = useAsyncThrottle(
    async (currentAdjustments: Adjustments, dragging: boolean = false) => {
      if (!selectedImage?.isReady) return;

      const payload = JSON.parse(JSON.stringify(currentAdjustments));

      try {
        await invoke(Invokes.ApplyAdjustments, { jsAdjustments: payload, isInteractive: dragging });
      } catch (err) {
        console.error('Failed to invoke apply_adjustments:', err);
      }
    },
    [selectedImage?.isReady],
  );

  const debouncedApplyAdjustments = useCallback(
    debounce((currentAdjustments) => {
      applyAdjustments(currentAdjustments, false);
    }, 50),
    [applyAdjustments],
  );

  const debouncedGenerateUncroppedPreview = useCallback(
    debounce((currentAdjustments) => {
      if (!selectedImage?.isReady) {
        return;
      }
      invoke(Invokes.GenerateUncroppedPreview, { jsAdjustments: currentAdjustments }).catch((err) =>
        console.error('Failed to generate uncropped preview:', err),
      );
    }, 50),
    [selectedImage?.isReady],
  );

  const debouncedSave = useCallback(
    debounce((path, adjustmentsToSave) => {
      invoke(Invokes.SaveMetadataAndUpdateThumbnail, { path, adjustments: adjustmentsToSave }).catch((err) => {
        console.error('Auto-save failed:', err);
        setError(`Failed to save changes: ${err}`);
      });
    }, 300),
    [],
  );

  const createResizeHandler = (setter: any, startSize: number) => (e: any) => {
    e.preventDefault();
    setIsResizing(true);
    const startX = e.clientX;
    const startY = e.clientY;
    const doDrag = (moveEvent: any) => {
      if (setter === setLeftPanelWidth) {
        setter(Math.max(200, Math.min(startSize + (moveEvent.clientX - startX), 500)));
      } else if (setter === setRightPanelWidth) {
        setter(Math.max(280, Math.min(startSize - (moveEvent.clientX - startX), 600)));
      } else if (setter === setBottomPanelHeight) {
        setter(Math.max(100, Math.min(startSize - (moveEvent.clientY - startY), 400)));
      }
    };
    const stopDrag = () => {
      document.documentElement.style.cursor = '';
      window.removeEventListener('mousemove', doDrag);
      window.removeEventListener('mouseup', stopDrag);
      setIsResizing(false);
    };
    document.documentElement.style.cursor = setter === setBottomPanelHeight ? 'row-resize' : 'col-resize';
    window.addEventListener('mousemove', doDrag);
    window.addEventListener('mouseup', stopDrag);
  };

  useEffect(() => {
    const appWindow = getCurrentWindow();
    const checkFullscreen = async () => {
      setIsWindowFullScreen(await appWindow.isFullscreen());
    };
    checkFullscreen();

    const unlistenPromise = appWindow.onResized(checkFullscreen);

    return () => {
      unlistenPromise.then((unlisten: any) => unlisten());
    };
  }, []);

  const handleLutSelect = useCallback(
    async (path: string) => {
      try {
        const result: LutData = await invoke('load_and_parse_lut', { path });
        const name = path.split(/[\\/]/).pop() || 'LUT';
        setAdjustments((prev: Partial<Adjustments>) => ({
          ...prev,
          lutPath: path,
          lutName: name,
          lutSize: result.size,
          lutIntensity: 100,
          sectionVisibility: {
            ...(prev.sectionVisibility || INITIAL_ADJUSTMENTS.sectionVisibility),
            effects: true,
          },
        }));
      } catch (err) {
        console.error('Failed to load or parse LUT:', err);
        setError(`Failed to load LUT: ${err}`);
      }
    },
    [setAdjustments],
  );

  const handleRightPanelSelect = useCallback(
    (panelId: Panel) => {
      if (panelId === activeRightPanel) {
        setActiveRightPanel(null);
      } else {
        setActiveRightPanel(panelId);
        setRenderedRightPanel(panelId);
      }
      setActiveMaskId(null);
    },
    [activeRightPanel],
  );

  const allowedRightPanels = useMemo(
    () => (isCustomerMode ? [Panel.Presets, Panel.Export] : null), // null = all panels including CameraControls
    [isCustomerMode],
  );

  useEffect(() => {
    appSettingsRef.current = appSettings;
  }, [appSettings]);

  useEffect(() => {
    if (!isCustomerMode || !activeRightPanel || !allowedRightPanels) {
      return;
    }
    if (!allowedRightPanels.includes(activeRightPanel)) {
      setActiveRightPanel(Panel.Presets);
      setRenderedRightPanel(Panel.Presets);
    }
  }, [isCustomerMode, activeRightPanel, allowedRightPanels]);

  const handleSettingsChange = useCallback(
    (newSettings: AppSettings, meta?: SettingsSaveMeta) => {
      if (!newSettings) {
        console.error('handleSettingsChange was called with null settings. Aborting save operation.');
        return;
      }
      const sanitizedSettings = sanitizeSettingsForSave(newSettings);
      if (!sanitizedSettings) {
        console.error('handleSettingsChange was called with invalid settings. Aborting save operation.');
        return;
      }

      if (sanitizedSettings.theme && sanitizedSettings.theme !== theme) {
        setTheme(sanitizedSettings.theme);
      }

      const { searchCriteria: _searchCriteria, ...settingsToSave } = sanitizedSettings as any;
      const saveSeq = (settingsSaveSeqRef.current += 1);
      const previousSettings = appSettingsRef.current;

      setAppSettings(sanitizedSettings);
      invoke(Invokes.SaveSettings, { settings: settingsToSave })
        .then(() => {
          if (typeof window !== 'undefined') {
            window.dispatchEvent(
              new CustomEvent('boothy:settings-save-result', {
                detail: { ok: true, reason: meta?.reason ?? 'unknown' },
              }),
            );
          }
        })
        .catch((err) => {
          console.error('Failed to save settings:', err);
          if (settingsSaveSeqRef.current === saveSeq && previousSettings) {
            setAppSettings(previousSettings);
          }
          if (typeof window !== 'undefined') {
            window.dispatchEvent(
              new CustomEvent('boothy:settings-save-result', {
                detail: { ok: false, reason: meta?.reason ?? 'unknown', error: `${err}` },
              }),
            );
          }
        });
    },
    [theme],
  );

  useEffect(() => {
    sendFrontendLog('info', 'settings-load-start');
    invoke(Invokes.LoadSettings)
      .then(async (settings: any) => {
        if (
          !settings.copyPasteSettings ||
          !settings.copyPasteSettings.includedAdjustments ||
          settings.copyPasteSettings.includedAdjustments.length === 0
        ) {
          settings.copyPasteSettings = {
            mode: 'merge',
            includedAdjustments: COPYABLE_ADJUSTMENT_KEYS,
          };
        }
        setAppSettings(settings);
        if (settings?.sortCriteria) setSortCriteria(settings.sortCriteria);
        if (settings?.filterCriteria) {
          setFilterCriteria((prev: FilterCriteria) => ({
            ...prev,
            ...settings.filterCriteria,
            rawStatus: settings.filterCriteria.rawStatus || RawStatus.All,
            colors: settings.filterCriteria.colors || [],
          }));
        }
        if (settings?.theme) {
          setTheme(settings.theme);
        }
        if (settings?.uiVisibility) {
          setUiVisibility((prev) => ({ ...prev, ...settings.uiVisibility }));
        }
        if (settings?.thumbnailSize) {
          setThumbnailSize(settings.thumbnailSize);
        }
        if (settings?.thumbnailAspectRatio) {
          setThumbnailAspectRatio(settings.thumbnailAspectRatio);
        }
        if (settings?.activeTreeSection) {
          setActiveTreeSection(settings.activeTreeSection);
        }
        if (settings?.pinnedFolders && settings.pinnedFolders.length > 0) {
          try {
            const trees = await invoke(Invokes.GetPinnedFolderTrees, { paths: settings.pinnedFolders });
            setPinnedFolderTrees(trees);
          } catch (err) {
            console.error('Failed to load pinned folder trees:', err);
          }
        }
        sendFrontendLog('info', 'settings-load-success', {
          lastRootPath: settings?.lastRootPath ?? null,
          theme: settings?.theme ?? null,
          pinnedFolderCount: settings?.pinnedFolders?.length ?? 0,
        });
        invoke('frontend_ready')
          .then(() => {
            sendFrontendLog('info', 'frontend-ready');
            // F5/WebView reload can restart the frontend while the Rust backend + sidecar remain connected.
            // In that case, we may not receive boothy-camera-connected/statusHint events, so explicitly pull once.
            refreshCameraStatus();
          })
          .catch((e) => {
            console.error('Failed to notify backend of readiness:', e);
            sendFrontendLog('error', 'frontend-ready-failed', { error: String(e) });
          });
      })
      .catch((err) => {
        console.error('Failed to load settings:', err);
        sendFrontendLog('error', 'settings-load-failed', { error: String(err) });
        setAppSettings({ lastRootPath: null, theme: DEFAULT_THEME_ID });
      })
      .finally(() => {
        isInitialMount.current = false;
      });
  }, [sendFrontendLog, refreshCameraStatus]);

  useEffect(() => {
    sendFrontendLog('info', 'app-mounted', { location: window.location.href });
  }, [sendFrontendLog]);

  useEffect(() => {
    // Ensure we pull camera status at least once on startup so the Library lamp doesn't remain red
    // after push snapshots become stale (snapshots are emitted on change, not continuously).
    refreshCameraStatus();
  }, [refreshCameraStatus]);

  useEffect(() => {
    if (isInitialMount.current || !appSettings) {
      return;
    }
    if (JSON.stringify(appSettings.uiVisibility) !== JSON.stringify(uiVisibility)) {
      handleSettingsChange({ ...appSettings, uiVisibility });
    }
  }, [uiVisibility, appSettings, handleSettingsChange]);

  const handleToggleWaveform = useCallback(() => {
    setIsWaveformVisible((prev: boolean) => !prev);
  }, []);

  useEffect(() => {
    if (isInitialMount.current || !appSettings) {
      return;
    }
    if (appSettings.thumbnailSize !== thumbnailSize) {
      handleSettingsChange({ ...appSettings, thumbnailSize });
    }
  }, [thumbnailSize, appSettings, handleSettingsChange]);

  useEffect(() => {
    if (isInitialMount.current || !appSettings) {
      return;
    }
    if (appSettings.thumbnailAspectRatio !== thumbnailAspectRatio) {
      handleSettingsChange({ ...appSettings, thumbnailAspectRatio });
    }
  }, [thumbnailAspectRatio, appSettings, handleSettingsChange]);

  useEffect(() => {
    invoke(Invokes.GetSupportedFileTypes)
      .then((types: any) => setSupportedTypes(types))
      .catch((err) => console.error('Failed to load supported file types:', err));
  }, []);

  useEffect(() => {
    if (isInitialMount.current || !appSettings) {
      return;
    }
    if (JSON.stringify(appSettings.sortCriteria) !== JSON.stringify(sortCriteria)) {
      handleSettingsChange({ ...appSettings, sortCriteria });
    }
  }, [sortCriteria, appSettings, handleSettingsChange]);

  useEffect(() => {
    if (isInitialMount.current || !appSettings) {
      return;
    }
    if (JSON.stringify(appSettings.filterCriteria) !== JSON.stringify(filterCriteria)) {
      handleSettingsChange({ ...appSettings, filterCriteria });
    }
  }, [filterCriteria, appSettings, handleSettingsChange]);

  useEffect(() => {
    if (appSettings?.adaptiveEditorTheme && selectedImage && finalPreviewUrl) {
      generatePaletteFromImage(finalPreviewUrl)
        .then(setAdaptivePalette)
        .catch((_err) => {
          const darkTheme = THEMES.find((t) => t.id === Theme.Dark);
          setAdaptivePalette(darkTheme ? darkTheme.cssVariables : null);
        });
    } else if (!appSettings?.adaptiveEditorTheme || !selectedImage) {
      setAdaptivePalette(null);
    }
  }, [appSettings?.adaptiveEditorTheme, selectedImage, finalPreviewUrl]);

  useEffect(() => {
    const root = document.documentElement;
    const currentThemeId = theme || DEFAULT_THEME_ID;

    const baseTheme =
      THEMES.find((t: ThemeProps) => t.id === currentThemeId) ||
      THEMES.find((t: ThemeProps) => t.id === DEFAULT_THEME_ID);
    if (!baseTheme) {
      return;
    }

    let finalCssVariables: any = { ...baseTheme.cssVariables };
    const effectThemeForWindow = baseTheme.id;

    if (adaptivePalette) {
      finalCssVariables = { ...finalCssVariables, ...adaptivePalette };
    }

    Object.entries(finalCssVariables).forEach(([key, value]) => {
      root.style.setProperty(key, value as string);
    });

    const isLight = [Theme.Light, Theme.Snow, Theme.Arctic].includes(effectThemeForWindow);
    invoke(Invokes.UpdateWindowEffect, { theme: isLight ? Theme.Light : Theme.Dark });
  }, [theme, adaptivePalette]);

  useEffect(() => {
    if (isInitialThemeMount.current) {
      isInitialThemeMount.current = false;
      return;
    }

    setIsAnimatingTheme(true);
    const timer = setTimeout(() => setIsAnimatingTheme(false), 500);

    return () => clearTimeout(timer);
  }, [theme]);

  const refreshAllFolderTrees = useCallback(async () => {
    if (rootPath) {
      try {
        const treeData = await invoke(Invokes.GetFolderTree, { path: rootPath });
        setFolderTree(treeData);
      } catch (err) {
        console.error('Failed to refresh main folder tree:', err);
        setError(`Failed to refresh folder tree: ${err}.`);
      }
    }

    const currentPins = appSettings?.pinnedFolders || [];
    if (currentPins.length > 0) {
      try {
        const trees = await invoke(Invokes.GetPinnedFolderTrees, { paths: currentPins });
        setPinnedFolderTrees(trees);
      } catch (err) {
        console.error('Failed to refresh pinned folder trees:', err);
      }
    }
  }, [rootPath, appSettings?.pinnedFolders]);

  const pinnedFolders = useMemo(() => appSettings?.pinnedFolders || [], [appSettings]);

  const handleTogglePinFolder = useCallback(
    async (path: string) => {
      if (!appSettings) return;
      const currentPins = appSettings.pinnedFolders || [];
      const isPinned = currentPins.includes(path);
      const newPins = isPinned
        ? currentPins.filter((p) => p !== path)
        : [...currentPins, path].sort((a, b) => a.localeCompare(b));

      if (!isPinned && path === currentFolderPath) {
        handleActiveTreeSectionChange('pinned');
      }

      handleSettingsChange({ ...appSettings, pinnedFolders: newPins });

      try {
        const trees = await invoke(Invokes.GetPinnedFolderTrees, { paths: newPins });
        setPinnedFolderTrees(trees);
      } catch (err) {
        console.error('Failed to refresh pinned folders:', err);
      }
    },
    [appSettings, handleSettingsChange],
  );

  const handleActiveTreeSectionChange = (section: string | null) => {
    setActiveTreeSection(section);
    if (appSettings) {
      handleSettingsChange({ ...appSettings, activeTreeSection: section });
    }
  };

  const handleSelectSubfolder = useCallback(
    async (path: string | null, isNewRoot = false) => {
      const requestId = selectSubfolderRequestRef.current + 1;
      selectSubfolderRequestRef.current = requestId;
      const isLatestRequest = () => selectSubfolderRequestRef.current === requestId;

      await invoke('cancel_thumbnail_generation');
      if (!isLatestRequest()) {
        return;
      }
      setIsViewLoading(true);
      setSearchCriteria({ tags: [], text: '', mode: 'OR' });
      setLibraryScrollTop(0);
      setThumbnails({});
      try {
        setCurrentFolderPath(path);

        if (isNewRoot) {
          setExpandedFolders(new Set([path]));
        } else if (path) {
          setExpandedFolders((prev) => {
            const newSet = new Set(prev);
            const allRoots = [rootPath, ...pinnedFolders].filter(Boolean) as string[];
            const relevantRoot = allRoots.find((r) => path.startsWith(r));

            if (relevantRoot) {
              const separator = path.includes('/') ? '/' : '\\';
              const parentSeparatorIndex = path.lastIndexOf(separator);

              if (parentSeparatorIndex > -1 && path.length > relevantRoot.length) {
                let current = path.substring(0, parentSeparatorIndex);
                while (current && current.length >= relevantRoot.length) {
                  newSet.add(current);
                  const nextParentIndex = current.lastIndexOf(separator);
                  if (nextParentIndex === -1 || current === relevantRoot) {
                    break;
                  }
                  current = current.substring(0, nextParentIndex);
                }
              }
              newSet.add(relevantRoot);
            }
            return newSet;
          });
        }

        if (isNewRoot) {
          if (path && !pinnedFolders.includes(path)) {
            handleActiveTreeSectionChange('current');
          }
          setIsTreeLoading(true);
          handleSettingsChange({ ...appSettings, lastRootPath: path } as AppSettings);
          try {
            const treeData = await invoke(Invokes.GetFolderTree, { path });
            if (!isLatestRequest()) {
              return;
            }
            setFolderTree(treeData);
          } catch (err) {
            if (isLatestRequest()) {
              console.error('Failed to load folder tree:', err);
              setError(`Failed to load folder tree: ${err}. Some sub-folders might be inaccessible.`);
            }
          } finally {
            if (isLatestRequest()) {
              setIsTreeLoading(false);
            }
          }
          // Note: File watcher is started by backend when session is created/restored
          // No need to call StartFolderWatcher from frontend
        }

        setImageList([]);
        setImageRatings({});
        setMultiSelectedPaths([]);
        setLibraryActivePath(null);
        if (selectedImage) {
          setSelectedImage(null);
          setFinalPreviewUrl(null);
          setUncroppedAdjustedPreviewUrl(null);
          setHistogram(null);
        }

        const command =
          libraryViewMode === LibraryViewMode.Recursive ? Invokes.ListImagesRecursive : Invokes.ListImagesInDir;

        const files: ImageFile[] = await invoke(command, { path });
        if (!isLatestRequest()) {
          return;
        }
        const exifSortKeys = ['date_taken', 'iso', 'shutter_speed', 'aperture', 'focal_length'];
        const isExifSortActive = exifSortKeys.includes(sortCriteria.key);
        const shouldReadExif = appSettings?.enableExifReading ?? false;

        if (shouldReadExif && files.length > 0) {
          const paths = files.map((f: ImageFile) => f.path);

          if (isExifSortActive) {
            const exifDataMap: Record<string, any> = await invoke(Invokes.ReadExifForPaths, { paths });
            if (!isLatestRequest()) {
              return;
            }
            const finalImageList = files.map((image) => ({
              ...image,
              exif: exifDataMap[image.path] || image.exif || null,
            }));
            setImageList(finalImageList);
          } else {
            setImageList(files);
            invoke(Invokes.ReadExifForPaths, { paths })
              .then((exifDataMap: any) => {
                if (isLatestRequest()) {
                  setImageList((currentImageList) =>
                    currentImageList.map((image) => ({
                      ...image,
                      exif: exifDataMap[image.path] || image.exif || null,
                    })),
                  );
                }
              })
              .catch((err) => {
                console.error('Failed to read EXIF data in background:', err);
              });
          }
        } else {
          setImageList(files);
        }
      } catch (err) {
        if (isLatestRequest()) {
          console.error('Failed to load folder contents:', err);
          setError('Failed to load images from the selected folder.');
          setIsTreeLoading(false);
        }
      } finally {
        if (isLatestRequest()) {
          setIsViewLoading(false);
        }
      }
    },
    [appSettings, handleSettingsChange, selectedImage, rootPath, sortCriteria.key, pinnedFolders, libraryViewMode],
  );

  const handleLibraryRefresh = useCallback(() => {
    if (currentFolderPath) handleSelectSubfolder(currentFolderPath, false);
  }, [currentFolderPath, handleSelectSubfolder]);

  const applyBoothySession = useCallback(
    (session: BoothySession) => {
      if (!session) {
        return;
      }
      setBoothySession(session);
      setShouldAutoOpenEditor(true);
      setActiveView('editor');

      const rawPath = (session as any).raw_path || (session as any).rawPath;
      if (!rawPath) {
        return;
      }

      if (!appSettings) {
        pendingSessionRef.current = session;
        return;
      }

      const sessionKey = session.session_folder_name || rawPath;
      if (sessionKey && lastAppliedSessionRef.current === sessionKey) {
        pendingSessionRef.current = null;
        return;
      }

      lastAppliedSessionRef.current = sessionKey || null;
      pendingSessionRef.current = null;
      if (resetTimerRef.current) {
        clearTimeout(resetTimerRef.current);
        resetTimerRef.current = null;
      }
      pendingResetRef.current = false;
      resetPostponedRef.current = false;
      setPendingReset(false);
      setIsSessionLocked(false);
      setIsLockoutModalOpen(false);
      setIsResetModalOpen(false);
      setIsTMinus5ModalOpen(false);
      setIsEndScreenVisible(false);
      setAdminOverrideActive(false);
      setRootPath(rawPath);
      handleSelectSubfolder(rawPath, true);
    },
    [appSettings, handleSelectSubfolder],
  );

  useEffect(() => {
    if (!appSettings || !pendingSessionRef.current) {
      return;
    }
    applyBoothySession(pendingSessionRef.current);
  }, [appSettings, applyBoothySession]);

  useEffect(() => {
    if (!appSettings) {
      return;
    }
    invoke(Invokes.BoothyGetModeState)
      .then((state: BoothyModeState) => {
        if (state?.mode) {
          setBoothyMode(state.mode);
        }
        setBoothyHasAdminPassword(!!state?.has_admin_password);
      })
      .catch((err) => {
        console.error('Failed to load Boothy mode state:', err);
      });

    // Disabled: Auto-restore session on startup
    // Users must always enter a session name to begin
    // invoke(Invokes.BoothyGetActiveSession)
    //   .then((session: BoothySession | null) => {
    //     if (session) {
    //       applyBoothySession(session);
    //     }
    //   })
    //   .catch((err) => {
    //     console.error('Failed to load active Boothy session:', err);
    //   });
  }, [appSettings, applyBoothySession]);

  const isMissingFileError = (err: unknown) => {
    const message = String(err);
    return (
      message.includes('os error 2') ||
      message.includes('ENOENT') ||
      message.includes('The system cannot find the file specified') ||
      message.toLowerCase().includes('not found')
    );
  };

  const refreshImageList = useCallback(async () => {
    if (!currentFolderPath) return null;
    try {
      const command =
        libraryViewMode === LibraryViewMode.Recursive ? Invokes.ListImagesRecursive : Invokes.ListImagesInDir;

      const files: ImageFile[] = await invoke(command, { path: currentFolderPath });
      const exifSortKeys = ['date_taken', 'iso', 'shutter_speed', 'aperture', 'focal_length'];
      const isExifSortActive = exifSortKeys.includes(sortCriteria.key);
      const shouldReadExif = appSettings?.enableExifReading ?? false;

      let freshExifData: Record<string, any> | null = null;

      if (shouldReadExif && files.length > 0 && isExifSortActive) {
        const paths = files.map((f: ImageFile) => f.path);
        freshExifData = await invoke(Invokes.ReadExifForPaths, { paths });
      }

      setImageList((prevList) => {
        const prevMap = new Map(prevList.map((img) => [img.path, img]));

        return files.map((newFile) => {
          if (freshExifData && freshExifData[newFile.path]) {
            newFile.exif = freshExifData[newFile.path];
            return newFile;
          }
          const existing = prevMap.get(newFile.path);
          if (existing && existing.modified === newFile.modified) {
            return existing;
          }

          return newFile;
        });
      });

      if (shouldReadExif && files.length > 0 && !isExifSortActive) {
        const paths = files.map((f: ImageFile) => f.path);
        invoke(Invokes.ReadExifForPaths, { paths })
          .then((exifDataMap: any) => {
            setImageList((currentImageList) =>
              currentImageList.map((image) => {
                if (exifDataMap[image.path] && !image.exif) {
                  return { ...image, exif: exifDataMap[image.path] };
                }
                return image;
              }),
            );
          })
          .catch((err) => {
            console.error('Failed to read EXIF data in background:', err);
          });
      }
      return files;
    } catch (err) {
      console.error('Failed to refresh image list:', err);
      setError('Failed to refresh image list.');
    }
    return null;
  }, [currentFolderPath, sortCriteria.key, appSettings?.enableExifReading, libraryViewMode]);

  const clearEditorSelection = useCallback((nextLibraryPath: string | null) => {
    setSelectedImage(null);
    setFinalPreviewUrl(null);
    setUncroppedAdjustedPreviewUrl(null);
    setHistogram(null);
    setWaveform(null);
    setIsWaveformVisible(false);
    setActiveMaskId(null);
    setActiveMaskContainerId(null);
    setIsWbPickerActive(false);
    setLibraryActivePath(nextLibraryPath);
  }, []);

  const reconcileSelectionAfterRefresh = useCallback(
    (files: ImageFile[] | null) => {
      if (!files) {
        return;
      }
      const fileSet = new Set(files.map((file) => file.path));

      setMultiSelectedPaths((prev) => prev.filter((path) => fileSet.has(path)));
      setLibraryActivePath((prev) => (prev && !fileSet.has(prev) ? null : prev));

      if (selectedImage && !fileSet.has(selectedImage.path)) {
        clearEditorSelection(null);
      }
    },
    [selectedImage, clearEditorSelection],
  );

  const refreshSessionFiles = useCallback(async () => {
    const files = await refreshImageList();
    reconcileSelectionAfterRefresh(files);
  }, [refreshImageList, reconcileSelectionAfterRefresh]);

  const handleToggleFolder = useCallback((path: string) => {
    setExpandedFolders((prev) => {
      const newSet = new Set(prev);
      if (newSet.has(path)) {
        newSet.delete(path);
      } else {
        newSet.add(path);
      }
      return newSet;
    });
  }, []);

  useEffect(() => {
    if (isInitialMount.current || !appSettings || !rootPath || !currentFolderPath) {
      return;
    }

    const newFolderState = {
      currentFolderPath,
      expandedFolders: Array.from(expandedFolders),
    };

    if (JSON.stringify(appSettings.lastFolderState) === JSON.stringify(newFolderState)) {
      return;
    }

    handleSettingsChange({ ...appSettings, lastFolderState: newFolderState });
  }, [currentFolderPath, expandedFolders, rootPath, appSettings, handleSettingsChange]);

  useEffect(() => {
    const handleGlobalContextMenu = (event: any) => {
      if (!DEBUG) event.preventDefault();
    };
    window.addEventListener('contextmenu', handleGlobalContextMenu);
    return () => window.removeEventListener('contextmenu', handleGlobalContextMenu);
  }, []);

  const handleBackToLibrary = useCallback(() => {
    const lastActivePath = selectedImage?.path ?? null;
    setActiveView('library');
    clearEditorSelection(lastActivePath);
  }, [selectedImage?.path, clearEditorSelection]);

  const handleImageSelect = useCallback(
    (path: string) => {
      if (selectedImage?.path === path) {
        return;
      }
      setActiveView('editor');
      applyAdjustments.cancel();
      debouncedSave.cancel();

      setSelectedImage({
        exif: null,
        height: 0,
        isRaw: false,
        isReady: false,
        metadata: null,
        originalUrl: null,
        path,
        thumbnailUrl: thumbnails[path],
        width: 0,
      });
      setOriginalSize({ width: 0, height: 0 });
      setPreviewSize({ width: 0, height: 0 });
      setMultiSelectedPaths([path]);
      setLibraryActivePath(null);
      setIsViewLoading(true);
      setError(null);
      setHistogram(null);
      setFinalPreviewUrl(null);
      setUncroppedAdjustedPreviewUrl(null);
      setFullScreenUrl(null);
      setFullResolutionUrl(null);
      setTransformedOriginalUrl(null);
      setLiveAdjustments(INITIAL_ADJUSTMENTS);
      resetAdjustmentsHistory(INITIAL_ADJUSTMENTS);
      setShowOriginal(false);
      setActiveMaskId(null);
      setActiveMaskContainerId(null);
      setIsWbPickerActive(false);

      if (transformWrapperRef.current) {
        transformWrapperRef.current.resetTransform(0);
      }

      setZoom(1);
      setIsLibraryExportPanelVisible(false);
    },
    [selectedImage?.path, applyAdjustments, debouncedSave, thumbnails, resetAdjustmentsHistory],
  );

  const pickNewestImagePath = useCallback((files: ImageFile[]) => {
    if (!files || files.length === 0) {
      return null;
    }
    let newest = files[0];
    for (const file of files) {
      if ((file.modified ?? 0) >= (newest.modified ?? 0)) {
        newest = file;
      }
    }
    return newest.path;
  }, []);

  const selectEditorTargetPath = useCallback(() => {
    if (libraryActivePath) {
      return libraryActivePath;
    }
    if (multiSelectedPaths.length > 0) {
      return multiSelectedPaths[0];
    }
    return pickNewestImagePath(imageList);
  }, [libraryActivePath, multiSelectedPaths, imageList, pickNewestImagePath]);

  const handleReturnToEditor = useCallback(() => {
    setActiveView('editor');
    const targetPath = selectEditorTargetPath();
    if (!targetPath) {
      return;
    }
    handleImageSelect(targetPath);
  }, [handleImageSelect, selectEditorTargetPath]);

  useEffect(() => {
    if (!shouldAutoOpenEditor) {
      return;
    }
    if (selectedImage) {
      setShouldAutoOpenEditor(false);
      return;
    }
    const targetPath = selectEditorTargetPath();
    if (!targetPath) {
      return;
    }
    handleImageSelect(targetPath);
    setShouldAutoOpenEditor(false);
  }, [shouldAutoOpenEditor, selectedImage, selectEditorTargetPath, handleImageSelect]);

  const refreshSessionFilesWithAutoSelect = useCallback(async () => {
    const previousFiles = imageListRef.current;
    const files = await refreshImageList();
    reconcileSelectionAfterRefresh(files);

    if (!files || isEditingLockedRef.current) {
      return;
    }

    const newestPath = findNewestAddedPath(files, previousFiles);
    if (newestPath) {
      handleImageSelect(newestPath);
    }
  }, [refreshImageList, reconcileSelectionAfterRefresh, findNewestAddedPath, handleImageSelect]);

  const throttledSessionRefresh = useMemo(
    () =>
      throttle(() => {
        void refreshSessionFilesWithAutoSelect();
      }, 500),
    [refreshSessionFilesWithAutoSelect],
  );

  useEffect(() => {
    return () => {
      throttledSessionRefresh.cancel();
    };
  }, [throttledSessionRefresh]);

  const executeDelete = useCallback(
    async (pathsToDelete: Array<string>, options = { includeAssociated: false }) => {
      if (isEditingLockedRef.current) {
        reportLockoutBlockedAction();
        return;
      }
      if (!pathsToDelete || pathsToDelete.length === 0) {
        return;
      }

      const activePath = selectedImage ? selectedImage.path : libraryActivePath;
      let nextImagePath: string | null = null;

      if (activePath) {
        const physicalPath = activePath.split('?vc=')[0];
        const isActiveImageDeleted = pathsToDelete.some((p) => p === activePath || p === physicalPath);

        if (isActiveImageDeleted) {
          const currentIndex = sortedImageList.findIndex((img) => img.path === activePath);
          if (currentIndex !== -1) {
            const nextCandidate = sortedImageList
              .slice(currentIndex + 1)
              .find((img) => !pathsToDelete.includes(img.path));

            if (nextCandidate) {
              nextImagePath = nextCandidate.path;
            } else {
              const prevCandidate = sortedImageList
                .slice(0, currentIndex)
                .reverse()
                .find((img) => !pathsToDelete.includes(img.path));

              if (prevCandidate) {
                nextImagePath = prevCandidate.path;
              }
            }
          }
        } else {
          nextImagePath = activePath;
        }
      }

      try {
        const command = options.includeAssociated ? 'delete_files_with_associated' : 'delete_files_from_disk';
        await invoke(command, { paths: pathsToDelete });

        await refreshImageList();

        if (selectedImage) {
          const physicalPath = selectedImage.path.split('?vc=')[0];
          const isFileBeingEditedDeleted = pathsToDelete.some((p) => p === selectedImage.path || p === physicalPath);

          if (isFileBeingEditedDeleted) {
            if (nextImagePath) {
              handleImageSelect(nextImagePath);
            } else {
              handleBackToLibrary();
            }
          }
        } else {
          if (nextImagePath) {
            setMultiSelectedPaths([nextImagePath]);
            setLibraryActivePath(nextImagePath);
          } else {
            setMultiSelectedPaths([]);
            setLibraryActivePath(null);
          }
        }
      } catch (err) {
        if (isMissingFileError(err)) {
          await refreshSessionFiles();
          return;
        }
        console.error('Failed to delete files:', err);
        setError(`Failed to delete files: ${err}`);
      }
    },
    [
      refreshImageList,
      refreshSessionFiles,
      isMissingFileError,
      selectedImage,
      handleBackToLibrary,
      libraryActivePath,
      sortedImageList,
      handleImageSelect,
      reportLockoutBlockedAction,
    ],
  );

  const handleDeleteSelected = useCallback(() => {
    if (isEditingLockedRef.current) {
      reportLockoutBlockedAction();
      return;
    }
    const pathsToDelete = multiSelectedPaths;
    if (pathsToDelete.length === 0) {
      return;
    }

    const isSingle = pathsToDelete.length === 1;

    const selectionHasVirtualCopies =
      isSingle &&
      !pathsToDelete[0].includes('?vc=') &&
      imageList.some((image) => image.path.startsWith(`${pathsToDelete[0]}?vc=`));

    let modalTitle = 'Confirm Delete';
    let modalMessage = '';
    let confirmText = 'Delete';

    if (selectionHasVirtualCopies) {
      modalTitle = 'Delete Image and All Virtual Copies?';
      modalMessage =
        'Are you sure you want to permanently delete this image and all of its virtual copies? This action cannot be undone.';
      confirmText = 'Delete All';
    } else if (isSingle) {
      modalMessage =
        'Are you sure you want to permanently delete this image? This action cannot be undone. Right-click for more options (e.g., deleting associated files).';
      confirmText = 'Delete Selected Only';
    } else {
      modalMessage = `Are you sure you want to permanently delete these ${pathsToDelete.length} images? This action cannot be undone. Right-click for more options (e.g., deleting associated files).`;
      confirmText = 'Delete Selected Only';
    }

    setConfirmModalState({
      confirmText,
      confirmVariant: 'destructive',
      isOpen: true,
      message: modalMessage,
      onConfirm: () => executeDelete(pathsToDelete, { includeAssociated: false }),
      title: modalTitle,
    });
  }, [multiSelectedPaths, executeDelete, imageList, reportLockoutBlockedAction]);

  const handleToggleFullScreen = useCallback(() => {
    if (isFullScreen) {
      setIsFullScreen(false);
      setFullScreenUrl(null);
    } else {
      if (!selectedImage) {
        return;
      }
      setIsFullScreen(true);
    }
  }, [isFullScreen, selectedImage]);

  useEffect(() => {
    if (!isFullScreen || !selectedImage?.isReady) {
      return;
    }

    let url: string | null = null;
    const generate = async () => {
      setIsFullScreenLoading(true);
      try {
        const imageData: Uint8Array = await invoke(Invokes.GenerateFullscreenPreview, { jsAdjustments: adjustments });
        const blob = new Blob([imageData], { type: 'image/jpeg' });
        url = URL.createObjectURL(blob);
        setFullScreenUrl(url);
      } catch (e) {
        console.error('Failed to generate fullscreen preview:', e);
        setError('Failed to generate full screen preview.');
      } finally {
        setIsFullScreenLoading(false);
      }
    };
    generate();
  }, [isFullScreen, selectedImage?.path, selectedImage?.isReady, adjustments]);

  const handleCopyAdjustments = useCallback(() => {
    const sourceAdjustments = selectedImage ? adjustments : libraryActiveAdjustments;
    const adjustmentsToCopy: any = {};
    for (const key of COPYABLE_ADJUSTMENT_KEYS) {
      if (Object.prototype.hasOwnProperty.call(sourceAdjustments, key)) {
        adjustmentsToCopy[key] = sourceAdjustments[key];
      }
    }
    setCopiedAdjustments(adjustmentsToCopy);
    setIsCopied(true);
  }, [selectedImage, adjustments, libraryActiveAdjustments]);

  const handlePasteAdjustments = useCallback(
    (paths?: Array<string>) => {
      if (!copiedAdjustments || !appSettings) {
        return;
      }

      const { mode, includedAdjustments } = appSettings.copyPasteSettings;

      const adjustmentsToApply: Partial<Adjustments> = {};

      for (const key of includedAdjustments) {
        if (Object.prototype.hasOwnProperty.call(copiedAdjustments, key)) {
          const value = copiedAdjustments[key as keyof Adjustments];

          if (mode === PasteMode.Merge) {
            const defaultValue = INITIAL_ADJUSTMENTS[key as keyof Adjustments];
            if (JSON.stringify(value) !== JSON.stringify(defaultValue)) {
              adjustmentsToApply[key as keyof Adjustments] = value;
            }
          } else {
            adjustmentsToApply[key as keyof Adjustments] = value;
          }
        }
      }

      if (Object.keys(adjustmentsToApply).length === 0) {
        setIsPasted(true);
        return;
      }

      const pathsToUpdate =
        paths || (multiSelectedPaths.length > 0 ? multiSelectedPaths : selectedImage ? [selectedImage.path] : []);
      if (pathsToUpdate.length === 0) {
        return;
      }

      if (selectedImage && pathsToUpdate.includes(selectedImage.path)) {
        const newAdjustments = { ...adjustments, ...adjustmentsToApply };
        setAdjustments(newAdjustments);
      }

      invoke(Invokes.ApplyAdjustmentsToPaths, { paths: pathsToUpdate, adjustments: adjustmentsToApply }).catch(
        (err) => {
          console.error('Failed to paste adjustments to multiple images:', err);
          setError(`Failed to paste adjustments: ${err}`);
        },
      );
      setIsPasted(true);
    },
    [copiedAdjustments, appSettings, multiSelectedPaths, selectedImage, adjustments, setAdjustments],
  );

  const handleAutoAdjustments = async () => {
    if (!selectedImage) {
      return;
    }
    try {
      const autoAdjustments: Adjustments = await invoke(Invokes.CalculateAutoAdjustments);
      setAdjustments((prev: Adjustments) => {
        const newAdjustments = { ...prev, ...autoAdjustments };
        newAdjustments.sectionVisibility = {
          ...prev.sectionVisibility,
          ...autoAdjustments.sectionVisibility,
        };

        return newAdjustments;
      });
    } catch (err) {
      console.error('Failed to calculate auto adjustments:', err);
      setError(`Failed to apply auto adjustments: ${err}`);
    }
  };

  const handleRate = useCallback(
    (newRating: number, paths?: Array<string>) => {
      const pathsToRate =
        paths || (multiSelectedPaths.length > 0 ? multiSelectedPaths : selectedImage ? [selectedImage.path] : []);
      if (pathsToRate.length === 0) {
        return;
      }

      let currentRating = 0;
      if (selectedImage && pathsToRate.includes(selectedImage.path)) {
        currentRating = adjustments.rating;
      } else if (libraryActivePath && pathsToRate.includes(libraryActivePath)) {
        currentRating = libraryActiveAdjustments.rating;
      }

      const finalRating = newRating === currentRating ? 0 : newRating;

      setImageRatings((prev: Record<string, number>) => {
        const newRatings = { ...prev };
        pathsToRate.forEach((path: string) => {
          newRatings[path] = finalRating;
        });
        return newRatings;
      });

      if (selectedImage && pathsToRate.includes(selectedImage.path)) {
        setAdjustments((prev: Adjustments) => ({ ...prev, rating: finalRating }));
      }

      if (libraryActivePath && pathsToRate.includes(libraryActivePath)) {
        setLibraryActiveAdjustments((prev) => ({ ...prev, rating: finalRating }));
      }

      invoke(Invokes.ApplyAdjustmentsToPaths, { paths: pathsToRate, adjustments: { rating: finalRating } }).catch(
        (err) => {
          console.error('Failed to apply rating to paths:', err);
          setError(`Failed to apply rating: ${err}`);
        },
      );
    },
    [
      multiSelectedPaths,
      selectedImage,
      libraryActivePath,
      adjustments.rating,
      libraryActiveAdjustments.rating,
      setAdjustments,
    ],
  );

  const handleSetColorLabel = useCallback(
    async (color: string | null, paths?: Array<string>) => {
      const pathsToUpdate =
        paths || (multiSelectedPaths.length > 0 ? multiSelectedPaths : selectedImage ? [selectedImage.path] : []);
      if (pathsToUpdate.length === 0) {
        return;
      }
      const primaryPath = selectedImage?.path || libraryActivePath;
      const primaryImage = imageList.find((img: ImageFile) => img.path === primaryPath);
      let currentColor = null;
      if (primaryImage && primaryImage.tags) {
        const colorTag = primaryImage.tags.find((tag: string) => tag.startsWith('color:'));
        if (colorTag) {
          currentColor = colorTag.substring(6);
        }
      }
      const finalColor = color !== null && color === currentColor ? null : color;
      try {
        await invoke(Invokes.SetColorLabelForPaths, { paths: pathsToUpdate, color: finalColor });

        setImageList((prevList: Array<ImageFile>) =>
          prevList.map((image: ImageFile) => {
            if (pathsToUpdate.includes(image.path)) {
              const otherTags = (image.tags || []).filter((tag: string) => !tag.startsWith('color:'));
              const newTags = finalColor ? [...otherTags, `color:${finalColor}`] : otherTags;
              return { ...image, tags: newTags };
            }
            return image;
          }),
        );
      } catch (err) {
        console.error('Failed to set color label:', err);
        setError(`Failed to set color label: ${err}`);
      }
    },
    [multiSelectedPaths, selectedImage, libraryActivePath, imageList],
  );

  const getCommonTags = useCallback(
    (paths: string[]): { tag: string; isUser: boolean }[] => {
      if (paths.length === 0) return [];
      const imageFiles = imageList.filter((img) => paths.includes(img.path));
      if (imageFiles.length === 0) return [];

      const allTagsSets = imageFiles.map((img) => {
        const tagsWithPrefix = (img.tags || []).filter((t) => !t.startsWith('color:'));
        return new Set(tagsWithPrefix);
      });

      if (allTagsSets.length === 0) return [];

      const commonTagsWithPrefix = allTagsSets.reduce((intersection, currentSet) => {
        return new Set([...intersection].filter((tag) => currentSet.has(tag)));
      });

      return Array.from(commonTagsWithPrefix)
        .map((tag) => ({
          tag: tag.startsWith('user:') ? tag.substring(5) : tag,
          isUser: tag.startsWith('user:'),
        }))
        .sort((a, b) => a.tag.localeCompare(b.tag));
    },
    [imageList],
  );

  const handleTagsChanged = useCallback(
    (changedPaths: string[], newTags: { tag: string; isUser: boolean }[]) => {
      setImageList((prevList) =>
        prevList.map((image) => {
          if (changedPaths.includes(image.path)) {
            const colorTags = (image.tags || []).filter((t) => t.startsWith('color:'));
            const prefixedNewTags = newTags.map((t) => (t.isUser ? `user:${t.tag}` : t.tag));
            const finalTags = [...colorTags, ...prefixedNewTags].sort();
            return { ...image, tags: finalTags.length > 0 ? finalTags : null };
          }
          return image;
        }),
      );
    },
    [setImageList],
  );

  const closeConfirmModal = () => setConfirmModalState({ ...confirmModalState, isOpen: false });

  const handlePasteFiles = useCallback(
    async (mode = 'copy') => {
      if (copiedFilePaths.length === 0 || !currentFolderPath) {
        return;
      }
      try {
        if (mode === 'copy')
          await invoke(Invokes.CopyFiles, { sourcePaths: copiedFilePaths, destinationFolder: currentFolderPath });
        else {
          await invoke(Invokes.MoveFiles, { sourcePaths: copiedFilePaths, destinationFolder: currentFolderPath });
          setCopiedFilePaths([]);
        }
        await refreshImageList();
      } catch (err) {
        setError(`Failed to ${mode} files: ${err}`);
      }
    },
    [copiedFilePaths, currentFolderPath, refreshImageList],
  );

  const requestFullResolution = useCallback(
    debounce((currentAdjustments: any, key: string) => {
      if (!selectedImage?.path) return;

      if (fullResRequestRef.current) {
        fullResRequestRef.current.cancelled = true;
      }

      const request = { cancelled: false };
      fullResRequestRef.current = request;

      invoke(Invokes.GenerateFullscreenPreview, {
        jsAdjustments: currentAdjustments,
      })
        .then((imageData: Uint8Array) => {
          if (!request.cancelled) {
            const blob = new Blob([imageData], { type: 'image/jpeg' });
            const url = URL.createObjectURL(blob);
            setFullResolutionUrl(url);
            fullResCacheKeyRef.current = key;
            setIsFullResolution(true);
            setIsLoadingFullRes(false);
          }
        })
        .catch((error: any) => {
          if (!request.cancelled) {
            console.error('Failed to generate full resolution preview:', error);
            setIsFullResolution(false);
            setFullResolutionUrl(null);
            fullResCacheKeyRef.current = null;
            setIsLoadingFullRes(false);
          }
        });
    }, 300),
    [selectedImage?.path],
  );

  useEffect(() => {
    if (isFullResolution && selectedImage?.path) {
      if (fullResCacheKeyRef.current !== visualAdjustmentsKey) {
        setIsLoadingFullRes(true);
        requestFullResolution(adjustments, visualAdjustmentsKey);
      }
    }
  }, [adjustments, isFullResolution, selectedImage?.path, requestFullResolution, visualAdjustmentsKey]);

  const handleFullResolutionLogic = useCallback(
    (targetZoomPercent: number) => {
      if (appSettings?.enableZoomHifi === false) {
        return;
      }

      if (!initialFitScale) {
        return;
      }
      const highResThreshold = Math.max(initialFitScale * 2, 0.5);
      const needsFullRes = targetZoomPercent > highResThreshold;
      const previewIsAlreadyFullRes = previewSize.width >= originalSize.width;
      if (needsFullRes && !previewIsAlreadyFullRes) {
        if (isFullResolution) {
          return;
        }
        if (fullResolutionUrl && fullResCacheKeyRef.current === visualAdjustmentsKey) {
          setIsFullResolution(true);
          return;
        }
        if (!isLoadingFullRes) {
          setIsLoadingFullRes(true);
          requestFullResolution(adjustments, visualAdjustmentsKey);
        }
      } else {
        if (fullResRequestRef.current) {
          fullResRequestRef.current.cancelled = true;
        }
        if (requestFullResolution.cancel) {
          requestFullResolution.cancel();
        }
        if (isFullResolution) {
          setIsFullResolution(false);
        }
        if (isLoadingFullRes) {
          setIsLoadingFullRes(false);
        }
      }
    },
    [
      initialFitScale,
      previewSize.width,
      originalSize.width,
      isFullResolution,
      isLoadingFullRes,
      requestFullResolution,
      adjustments,
      fullResolutionUrl,
      visualAdjustmentsKey,
      appSettings,
    ],
  );

  const handleZoomChange = useCallback(
    (zoomValue: number, fitToWindow: boolean = false) => {
      let targetZoomPercent: number;
      const orientationSteps = adjustments.orientationSteps || 0;
      const isSwapped = orientationSteps === 1 || orientationSteps === 3;
      const effectiveOriginalWidth = isSwapped ? originalSize.height : originalSize.width;
      const effectiveOriginalHeight = isSwapped ? originalSize.width : originalSize.height;
      if (fitToWindow) {
        if (
          effectiveOriginalWidth > 0 &&
          effectiveOriginalHeight > 0 &&
          baseRenderSize.width > 0 &&
          baseRenderSize.height > 0
        ) {
          const originalAspect = effectiveOriginalWidth / effectiveOriginalHeight;
          const baseAspect = baseRenderSize.width / baseRenderSize.height;
          if (originalAspect > baseAspect) {
            targetZoomPercent = baseRenderSize.width / effectiveOriginalWidth;
          } else {
            targetZoomPercent = baseRenderSize.height / effectiveOriginalHeight;
          }
        } else {
          targetZoomPercent = 1.0;
        }
      } else {
        targetZoomPercent = zoomValue;
      }
      targetZoomPercent = Math.max(0.1, Math.min(2.0, targetZoomPercent));
      let transformZoom = 1.0;
      if (
        effectiveOriginalWidth > 0 &&
        effectiveOriginalHeight > 0 &&
        baseRenderSize.width > 0 &&
        baseRenderSize.height > 0
      ) {
        const originalAspect = effectiveOriginalWidth / effectiveOriginalHeight;
        const baseAspect = baseRenderSize.width / baseRenderSize.height;
        if (originalAspect > baseAspect) {
          transformZoom = (targetZoomPercent * effectiveOriginalWidth) / baseRenderSize.width;
        } else {
          transformZoom = (targetZoomPercent * effectiveOriginalHeight) / baseRenderSize.height;
        }
      }
      isProgrammaticZoom.current = true;
      setZoom(transformZoom);
      handleFullResolutionLogic(targetZoomPercent);
    },
    [originalSize, baseRenderSize, handleFullResolutionLogic, adjustments.orientationSteps],
  );

  const handleUserTransform = useCallback(
    (transformState: TransformState) => {
      if (isProgrammaticZoom.current) {
        isProgrammaticZoom.current = false;
        return;
      }

      setZoom(transformState.scale);

      if (originalSize.width > 0 && baseRenderSize.width > 0) {
        const orientationSteps = adjustments.orientationSteps || 0;
        const isSwapped = orientationSteps === 1 || orientationSteps === 3;
        const effectiveOriginalWidth = isSwapped ? originalSize.height : originalSize.width;

        const targetZoomPercent = (baseRenderSize.width * transformState.scale) / effectiveOriginalWidth;
        handleFullResolutionLogic(targetZoomPercent);
      }
    },
    [originalSize, baseRenderSize, handleFullResolutionLogic, adjustments.orientationSteps],
  );

  const isAnyModalOpen =
    isCreateFolderModalOpen ||
    isRenameFolderModalOpen ||
    isRenameFileModalOpen ||
    isImportModalOpen ||
    isCopyPasteSettingsModalOpen ||
    confirmModalState.isOpen ||
    panoramaModalState.isOpen ||
    cullingModalState.isOpen ||
    collageModalState.isOpen ||
    isLockoutModalOpen ||
    isResetModalOpen ||
    isEndScreenVisible ||
    isEditingLocked ||
    (isCustomerMode && isTMinus5ModalOpen);

  useKeyboardShortcuts({
    isModalOpen: isAnyModalOpen,
    activeMaskContainerId,
    activeMaskId,
    activeRightPanel,
    canRedo,
    canUndo,
    copiedFilePaths,
    customEscapeHandler,
    handleBackToLibrary,
    handleCopyAdjustments,
    handleDeleteMaskContainer,
    handleDeleteSelected,
    handleImageSelect,
    handlePasteAdjustments,
    handlePasteFiles,
    handleRate,
    handleRightPanelSelect,
    handleSetColorLabel,
    handleToggleFullScreen,
    handleZoomChange,
    isFullScreen,
    isStraightenActive,
    isViewLoading,
    libraryActivePath,
    multiSelectedPaths,
    redo,
    selectedImage,
    setActiveMaskContainerId,
    setActiveMaskId,
    setCopiedFilePaths,
    setIsStraightenActive,
    setIsWaveformVisible,
    setLibraryActivePath,
    setMultiSelectedPaths,
    setShowOriginal,
    sortedImageList,
    undo,
    zoom,
    displaySize,
    baseRenderSize,
    originalSize,
  });

  const clearResetTimer = useCallback(() => {
    if (resetTimerRef.current) {
      clearTimeout(resetTimerRef.current);
      resetTimerRef.current = null;
    }
  }, []);

  const resetGraceSeconds = useMemo(() => {
    const configuredSeconds = getBoothyResetGracePeriodSeconds(appSettings);
    if (typeof sessionRemainingSeconds === 'number' && Number.isFinite(sessionRemainingSeconds)) {
      return Math.min(configuredSeconds, Math.max(0, sessionRemainingSeconds));
    }
    return configuredSeconds;
  }, [appSettings, sessionRemainingSeconds]);

  const resetSessionContext = useCallback(() => {
    clearResetTimer();
    setPendingReset(false);
    pendingResetRef.current = false;
    resetPostponedRef.current = false;
    setIsSessionLocked(false);
    setIsLockoutModalOpen(false);
    setIsResetModalOpen(false);
    setIsTMinus5ModalOpen(false);
    setIsEndScreenVisible(false);
    setAdminOverrideActive(false);
    setBoothySession(null);
    setSessionRemainingSeconds(null);
    pendingSessionRef.current = null;
    lastAppliedSessionRef.current = null;
    setRootPath(null);
    setCurrentFolderPath(null);
    setImageList([]);
    setImageRatings({});
    setFolderTree(null);
    setMultiSelectedPaths([]);
    setLibraryActivePath(null);
    setIsLibraryExportPanelVisible(false);
    setExpandedFolders(new Set());
    setIsStartingSession(false);
  }, [clearResetTimer]);

  const startResetFlow = useCallback(() => {
    const remainingSeconds =
      typeof sessionRemainingSeconds === 'number' && Number.isFinite(sessionRemainingSeconds)
        ? Math.max(0, sessionRemainingSeconds)
        : null;
    const effectiveGraceSeconds = resetGraceSeconds;
    const isBoothyExporting = boothyExportProgress.status === 'exporting';
    const isBusy =
      exportState.status === Status.Exporting || importState.status === Status.Importing || isBoothyExporting;

    resetPostponedRef.current = false;

    if (!isBusy) {
      resetSessionContext();
      return;
    }

    setPendingReset(true);
    clearResetTimer();
    resetTimerRef.current = setTimeout(() => {
      if (resetPostponedRef.current || !pendingResetRef.current) {
        return;
      }
      sendFrontendLog('warn', 'timeline-reset-grace-exceeded', {
        graceSeconds: effectiveGraceSeconds,
        remainingSeconds,
      });
      invoke(Invokes.CancelExport).catch(() => { });
      resetSessionContext();
    }, effectiveGraceSeconds * 1000);
  }, [
    boothyExportProgress.status,
    clearResetTimer,
    exportState.status,
    importState.status,
    resetSessionContext,
    resetGraceSeconds,
    sessionRemainingSeconds,
    sendFrontendLog,
  ]);

  const handleSessionLockout = useCallback(() => {
    setIsSessionLocked(true);
    if (!isCustomerMode) {
      setIsLockoutModalOpen(true);
    }
  }, [isCustomerMode]);

  const handleAdminContinueWorking = useCallback(() => {
    setIsSessionLocked(false);
    setIsLockoutModalOpen(false);
    logAdminOverride('t_zero_continue_working');
  }, [logAdminOverride]);

  const handleAdminDismissLockout = useCallback(() => {
    setIsLockoutModalOpen(false);
  }, []);

  const handleAdminResetNow = useCallback(() => {
    setIsResetModalOpen(false);
    startResetFlow();
  }, [startResetFlow]);

  const handleAdminPostponeReset = useCallback(() => {
    resetPostponedRef.current = true;
    setPendingReset(false);
    clearResetTimer();
    setIsResetModalOpen(false);
    logAdminOverride('n_59_reset_postpone');
  }, [clearResetTimer, logAdminOverride]);

  const handleTMinus5Confirm = useCallback(() => {
    setIsTMinus5ModalOpen(false);
  }, []);

  const handleTMinus5Dismiss = useCallback(() => {
    setIsTMinus5ModalOpen(false);
    logAdminOverride('t_minus_5_dismiss');
  }, [logAdminOverride]);

  const handleTMinus5Event = useCallback(
    (payload: any) => {
      const fallbackMessage = getBoothyTMinus5WarningMessage(appSettings);
      const message =
        typeof payload?.message === 'string' && payload.message.trim().length > 0 ? payload.message : fallbackMessage;
      setTMinus5Message(message);
      setIsTMinus5ModalOpen(true);
    },
    [appSettings],
  );

  const handleShowEndScreen = useCallback(() => {
    setIsEndScreenVisible(true);
  }, []);

  const handleAutoExportDecision = useCallback(async () => {
    if (storageLockoutRef.current) {
      setBoothyExportError(STORAGE_CRITICAL_MESSAGE);
      return;
    }
    // Check backend active session instead of frontend state to avoid stale closure issues
    try {
      const session = await invoke(Invokes.BoothyGetActiveSession);
      if (!session) {
        return;
      }
    } catch {
      return;
    }

    setIsLibraryExportPanelVisible(false);

    try {
      // Check if there are already exported files in Jpg folder
      const exportedCount = await invoke<number>(Invokes.BoothyGetExportedCount);
      const choice = exportedCount > 0 ? 'continueFromBackground' : 'overwriteAll';

      invoke(Invokes.BoothyHandleExportDecision, { choice }).catch((err) => {
        console.error('Failed to start Boothy export:', err);
        setBoothyExportError(typeof err === 'string' ? err : 'Failed to start export.');
      });
    } catch (err) {
      console.error('Failed to check exported count, defaulting to overwriteAll:', err);
      invoke(Invokes.BoothyHandleExportDecision, { choice: 'overwriteAll' }).catch((exportErr) => {
        setBoothyExportError(typeof exportErr === 'string' ? exportErr : 'Failed to start export.');
      });
    }
  }, [setBoothyExportError]);

  const handleExportDecisionOpen = useCallback(() => {
    if (storageLockoutRef.current) {
      setError(STORAGE_CRITICAL_MESSAGE);
      return;
    }
    setIsExportDecisionModalOpen(true);
  }, [setError]);

  const handleExportDecisionSelect = useCallback(
    (choice: 'overwriteAll' | 'continueFromBackground') => {
      if (storageLockoutRef.current) {
        setBoothyExportError(STORAGE_CRITICAL_MESSAGE);
        return;
      }
      setIsExportDecisionModalOpen(false);
      invoke(Invokes.BoothyHandleExportDecision, { choice }).catch((err) => {
        console.error('Failed to start Boothy export:', err);
        setBoothyExportError(typeof err === 'string' ? err : 'Failed to start export.');
      });
    },
    [setBoothyExportError],
  );

  useEffect(() => {
    let isEffectActive = true;
    const listeners = [
      listen('preview-update-final', (event: any) => {
        if (isEffectActive) {
          const imageData = new Uint8Array(event.payload);
          const blob = new Blob([imageData], { type: 'image/jpeg' });
          const url = URL.createObjectURL(blob);
          setFinalPreviewUrl(url);
        }
      }),
      listen('preview-update-uncropped', (event: any) => {
        if (isEffectActive) {
          const imageData = new Uint8Array(event.payload);
          const blob = new Blob([imageData], { type: 'image/jpeg' });
          const url = URL.createObjectURL(blob);
          setUncroppedAdjustedPreviewUrl(url);
        }
      }),
      listen('histogram-update', (event: any) => {
        if (isEffectActive) {
          setHistogram(event.payload);
        }
      }),
      listen('open-with-file', (event: any) => {
        if (isEffectActive) {
          setInitialFileToOpen(event.payload as string);
        }
      }),
      listen('waveform-update', (event: any) => {
        if (isEffectActive) {
          setWaveform(event.payload);
        }
      }),
      listen('thumbnail-generated', (event: any) => {
        if (isEffectActive) {
          const { path, data, rating } = event.payload;
          if (data) {
            setThumbnails((prev) => ({ ...prev, [path]: data }));
          }
          if (rating !== undefined) {
            setImageRatings((prev) => ({ ...prev, [path]: rating }));
          }
        }
      }),
      listen('batch-export-progress', (event: any) => {
        if (isEffectActive) {
          setExportState((prev: ExportState) => ({ ...prev, progress: event.payload }));
        }
      }),
      listen('export-complete', () => {
        if (isEffectActive) {
          setExportState((prev: ExportState) => ({ ...prev, status: Status.Success }));
        }
      }),
      listen('export-error', (event) => {
        if (isEffectActive) {
          setExportState((prev: ExportState) => ({
            ...prev,
            status: Status.Error,
            errorMessage: typeof event.payload === 'string' ? event.payload : 'An unknown export error occurred.',
          }));
        }
      }),
      listen('export-cancelled', () => {
        if (isEffectActive) {
          setExportState((prev: ExportState) => ({ ...prev, status: Status.Cancelled }));
        }
      }),
      listen('import-start', (event: any) => {
        if (isEffectActive) {
          setImportState({
            errorMessage: '',
            path: '',
            progress: { current: 0, total: event.payload.total },
            status: Status.Importing,
          });
        }
      }),
      listen('import-progress', (event: any) => {
        if (isEffectActive) {
          setImportState((prev: ImportState) => ({
            ...prev,
            path: event.payload.path,
            progress: { current: event.payload.current, total: event.payload.total },
          }));
        }
      }),
      listen('import-complete', () => {
        if (isEffectActive) {
          setImportState((prev: ImportState) => ({ ...prev, status: Status.Success }));
          refreshAllFolderTrees();
          if (currentFolderPathRef.current) {
            handleSelectSubfolder(currentFolderPathRef.current, false);
          }
        }
      }),
      listen('import-error', (event) => {
        if (isEffectActive) {
          setImportState((prev: ImportState) => ({
            ...prev,
            errorMessage: typeof event.payload === 'string' ? event.payload : 'An unknown import error occurred.',
            status: Status.Error,
          }));
        }
      }),
      listen('denoise-progress', (event: any) => {
        if (isEffectActive) {
          setDenoiseModalState((prev) => ({ ...prev, progressMessage: event.payload as string }));
        }
      }),
      listen('denoise-complete', (event: any) => {
        if (isEffectActive) {
          const payload = event.payload;
          const isObject = typeof payload === 'object' && payload !== null;

          setDenoiseModalState((prev) => ({
            ...prev,
            isProcessing: false,
            previewBase64: isObject ? payload.denoised : payload,
            originalBase64: isObject ? payload.original : null,
            progressMessage: null,
          }));
        }
      }),
      listen('denoise-error', (event: any) => {
        if (isEffectActive) {
          setDenoiseModalState((prev) => ({
            ...prev,
            isProcessing: false,
            error: String(event.payload),
            progressMessage: null,
          }));
        }
      }),
      // Boothy session and mode event listeners
      listen('boothy-session-changed', (event: any) => {
        if (isEffectActive) {
          const session = event.payload as BoothySession;
          if (session) {
            applyBoothySession(session);
            setSessionRemainingSeconds(null);
            setStorageHealth(null);
          }
        }
      }),
      listen('boothy-session-timer-tick', (event: any) => {
        if (isEffectActive) {
          const payload = event.payload as BoothySessionTimerTickPayload;
          if (typeof payload?.remaining_seconds === 'number') {
            setSessionRemainingSeconds(payload.remaining_seconds);
          }
        }
      }),
      listen('boothy-session-t-minus-5', (event: any) => {
        if (isEffectActive) {
          handleTMinus5Event(event.payload);
        }
      }),
      listen('boothy-session-t-zero', () => {
        if (isEffectActive) {
          handleSessionLockout();
          handleAutoExportDecision();
        }
      }),
      listen('boothy-session-reset', () => {
        if (isEffectActive) {
          if (isCustomerMode) {
            startResetFlow();
          } else {
            setIsResetModalOpen(true);
          }
        }
      }),
      listen('boothy-show-end-screen', () => {
        if (isEffectActive) {
          handleShowEndScreen();
        }
      }),
      listen('boothy-capture-started', () => {
        if (isEffectActive) {
          setCaptureStatusState('capturing');
        }
      }),
      listen('boothy-photo-transferred', (event: any) => {
        if (isEffectActive) {
          if (isEditingLockedRef.current) {
            return;
          }
          if (storageLockoutRef.current) {
            setError(STORAGE_CRITICAL_MESSAGE);
            return;
          }
          const payload = event.payload;
          const path = typeof payload === 'string' ? payload : payload?.path;
          const correlationId = typeof payload === 'object' ? payload?.correlationId : undefined;

          if (!path) {
            return;
          }

          if (isCustomerMode && hasBoothySessionRef.current) {
            setCaptureStatusState('transferring');
          }

          invoke(Invokes.BoothyHandlePhotoTransferred, {
            path,
            correlationId: correlationId || '',
          })
            .then(() => {
              if (isEffectActive && isCustomerMode && hasBoothySessionRef.current) {
                setCaptureStatusState('stabilizing');
              }
            })
            .catch((err) => {
              const message = typeof err === 'string' ? err : '최신 촬영을 처리하지 못했습니다. 다시 시도해 주세요.';
              console.error('Failed to handle photo transfer:', err);
              setError(message);
              if (isCustomerMode && hasBoothySessionRef.current) {
                setCaptureStatusState('error', { errorMessage: message, resetAfterMs: 5000 });
              }
            });
        }
      }),
      listen('boothy-new-photo', (event: any) => {
        if (isEffectActive) {
          if (isEditingLockedRef.current) {
            return;
          }
          const payload = event.payload;
          const path = typeof payload === 'string' ? payload : payload?.path;
          if (!path) {
            return;
          }

          if (isCustomerMode && hasBoothySessionRef.current) {
            setCaptureStatusState('importing');
          }

          void (async () => {
            try {
              const files = await refreshImageList();
              reconcileSelectionAfterRefresh(files);
              if (files?.some((file) => file.path === path)) {
                handleImageSelect(path);
              }
              if (isCustomerMode && hasBoothySessionRef.current) {
                setCaptureStatusState('ready', { resetAfterMs: 3000 });
              }
            } catch (err) {
              console.error('Failed to refresh library for new photo:', err);
              const message = '최신 촬영 사진을 불러오지 못했습니다.';
              setError(message);
              if (isCustomerMode && hasBoothySessionRef.current) {
                setCaptureStatusState('error', { errorMessage: message, resetAfterMs: 5000 });
              }
            }
          })();
        }
      }),
      listen('boothy-session-files-changed', () => {
        if (isEffectActive) {
          throttledSessionRefresh();
        }
      }),
      listen('boothy-camera-error', (event: any) => {
        if (isEffectActive) {
          const payload = event.payload as BoothyCameraErrorPayload | string;
          let message = CAMERA_MESSAGE_UNAVAILABLE;

          if (typeof payload === 'string') {
            message = payload;
          } else if (typeof payload?.code === 'string' && CAMERA_CUSTOMER_MESSAGES[payload.code]) {
            message = CAMERA_CUSTOMER_MESSAGES[payload.code];
          } else if (typeof payload?.message === 'string') {
            message = payload.message;
            if (boothyMode === 'admin' && typeof payload.diagnostic === 'string' && payload.diagnostic.trim()) {
              const details: string[] = [payload.diagnostic];
              if (typeof payload.correlationId === 'string' && payload.correlationId.trim()) {
                details.push(`Correlation ID: ${payload.correlationId}`);
              }
              message = `${message} (Details: ${details.join(' | ')})`;
            }
          }

          setError(message);
          if (isCustomerMode && hasBoothySessionRef.current) {
            setCaptureStatusState('error', { errorMessage: message, resetAfterMs: 5000 });
          }
          refreshCameraStatus();
        }
      }),
      listen('boothy-camera-connected', () => {
        if (isEffectActive) {
          refreshCameraStatus();
        }
      }),
      listen('boothy-camera-status', (event: any) => {
        if (!isEffectActive) return;
        const payload = event.payload as BoothyCameraStatusSnapshot | string;
        if (!payload || typeof payload !== 'object') {
          return;
        }
        setCameraStatusSnapshot(payload);
        lastCameraSnapshotAtRef.current = Date.now();
      }),
      listen('boothy-camera-status-hint', (event: any) => {
        if (!isEffectActive) return;

        const payload = event?.payload as any;
        const hintSource = payload && typeof payload === 'object' ? payload.source : null;
        const forceRefresh = hintSource === 'stopSidecar' || hintSource === 'backendPollError';

        // If we already received a snapshot very recently, skip the pull fallback.
        const lastSnapshotAt = lastCameraSnapshotAtRef.current;
        if (!forceRefresh && typeof lastSnapshotAt === 'number' && Date.now() - lastSnapshotAt < 1000) {
          return;
        }

        // Debounce: prevent duplicate refreshCameraStatus calls within 200ms
        // This fixes the issue where multiple getStatus requests are triggered
        // simultaneously when a single statusHint event is received
        if (statusHintDebounceRef.current !== null) {
          window.clearTimeout(statusHintDebounceRef.current);
        }

        // Keep this very small to minimize how long the customer lamp stays yellow/red.
        const debounceMs = hintSource === 'stopSidecar' ? 150 : 50;
        statusHintDebounceRef.current = window.setTimeout(() => {
          statusHintDebounceRef.current = null;
          refreshCameraStatus();
        }, debounceMs);
      }),
      listen('boothy-storage-health', (event: any) => {
        if (isEffectActive) {
          const payload = event.payload as BoothyStorageHealthPayload;
          if (payload?.status) {
            setStorageHealth(payload);
          }
        }
      }),
      listen('boothy-mode-changed', (event: any) => {
        if (isEffectActive) {
          const payload = event.payload as BoothyModeState;
          if (payload?.mode) {
            setBoothyMode(payload.mode);
          }
          setBoothyHasAdminPassword(!!payload?.has_admin_password);
        }
      }),
    ];
    return () => {
      isEffectActive = false;
      listeners.forEach((p) => p.then((unlisten) => unlisten()));

      // Clear statusHint debounce timer on cleanup
      if (statusHintDebounceRef.current !== null) {
        window.clearTimeout(statusHintDebounceRef.current);
        statusHintDebounceRef.current = null;
      }
    };
  }, [
    refreshAllFolderTrees,
    handleSelectSubfolder,
    refreshImageList,
    reconcileSelectionAfterRefresh,
    handleImageSelect,
    applyBoothySession,
    handleSessionLockout,
    handleAutoExportDecision,
    handleTMinus5Event,
    handleShowEndScreen,
    startResetFlow,
    throttledSessionRefresh,
    refreshCameraStatus,
    setCaptureStatusState,
    boothyMode,
    isCustomerMode,
  ]);

  useEffect(() => {
    const isBoothyExporting = boothyExportProgress.status === 'exporting';
    const isBusy =
      exportState.status === Status.Exporting || importState.status === Status.Importing || isBoothyExporting;
    if (!pendingResetRef.current || resetPostponedRef.current || isBusy) {
      return;
    }
    resetSessionContext();
  }, [boothyExportProgress.status, exportState.status, importState.status, resetSessionContext]);

  useEffect(() => {
    if ([Status.Success, Status.Error, Status.Cancelled].includes(exportState.status)) {
      const timeoutDuration = exportState.status === Status.Success ? 5000 : 3000;

      const timer = setTimeout(() => {
        setExportState({ status: Status.Idle, progress: { current: 0, total: 0 }, errorMessage: '' });
      }, timeoutDuration);
      return () => clearTimeout(timer);
    }
  }, [exportState.status]);

  useEffect(() => {
    if (boothyExportProgress.status !== 'complete') {
      return;
    }
    handleShowEndScreen();
    const timer = setTimeout(() => {
      resetBoothyExportProgress();
    }, 1000);
    return () => clearTimeout(timer);
  }, [boothyExportProgress.status, handleShowEndScreen, resetBoothyExportProgress]);

  useEffect(() => {
    if ([Status.Success, Status.Error].includes(importState.status)) {
      const timer = setTimeout(() => {
        setImportState({ status: Status.Idle, progress: { current: 0, total: 0 }, path: '', errorMessage: '' });
      }, IMPORT_TIMEOUT);

      return () => clearTimeout(timer);
    }
  }, [importState.status]);

  useEffect(() => {
    if (libraryActivePath) {
      invoke(Invokes.LoadMetadata, { path: libraryActivePath })
        .then((metadata: any) => {
          if (metadata.adjustments && !metadata.adjustments.is_null) {
            const normalized: Adjustments = normalizeLoadedAdjustments(metadata.adjustments);
            setLibraryActiveAdjustments(normalized);
          } else {
            setLibraryActiveAdjustments(INITIAL_ADJUSTMENTS);
          }
        })
        .catch((err) => {
          console.error('Failed to load metadata for library active image', err);
          setLibraryActiveAdjustments(INITIAL_ADJUSTMENTS);
        });
    } else {
      setLibraryActiveAdjustments(INITIAL_ADJUSTMENTS);
    }
  }, [libraryActivePath]);

  useEffect(() => {
    let isEffectActive = true;

    const unlistenProgress = listen('panorama-progress', (event: any) => {
      if (isEffectActive) {
        setPanoramaModalState((prev: PanoramaModalState) => ({
          ...prev,
          error: null,
          finalImageBase64: null,
          isOpen: true,
          progressMessage: event.payload,
        }));
      }
    });

    const unlistenComplete = listen('panorama-complete', (event: any) => {
      if (isEffectActive) {
        const { base64 } = event.payload;
        setPanoramaModalState((prev: PanoramaModalState) => ({
          ...prev,
          error: null,
          finalImageBase64: base64,
          progressMessage: 'Panorama Ready',
        }));
      }
    });

    const unlistenError = listen('panorama-error', (event: any) => {
      if (isEffectActive) {
        setPanoramaModalState((prev: PanoramaModalState) => ({
          ...prev,
          error: String(event.payload),
          finalImageBase64: null,
          progressMessage: 'An error occurred.',
        }));
      }
    });

    return () => {
      isEffectActive = false;
      unlistenProgress.then((f: any) => f());
      unlistenComplete.then((f: any) => f());
      unlistenError.then((f: any) => f());
    };
  }, []);

  useEffect(() => {
    let isEffectActive = true;

    const unlistenStart = listen('culling-start', (event: any) => {
      if (isEffectActive) {
        setCullingModalState((prev) => ({
          ...prev,
          isOpen: true,
          progress: { current: 0, total: event.payload, stage: 'Initializing...' },
          suggestions: null,
          error: null,
        }));
      }
    });

    const unlistenProgress = listen('culling-progress', (event: any) => {
      if (isEffectActive) {
        setCullingModalState((prev) => ({ ...prev, progress: event.payload }));
      }
    });

    const unlistenComplete = listen('culling-complete', (event: any) => {
      if (isEffectActive) {
        setCullingModalState((prev) => ({ ...prev, progress: null, suggestions: event.payload }));
      }
    });

    const unlistenError = listen('culling-error', (event: any) => {
      if (isEffectActive) {
        setCullingModalState((prev) => ({ ...prev, progress: null, error: String(event.payload) }));
      }
    });

    return () => {
      isEffectActive = false;
      unlistenStart.then((f) => f());
      unlistenProgress.then((f) => f());
      unlistenComplete.then((f) => f());
      unlistenError.then((f) => f());
    };
  }, []);

  const handleSavePanorama = async (): Promise<string> => {
    if (panoramaModalState.stitchingSourcePaths.length === 0) {
      const err = 'Source paths for panorama not found.';
      setPanoramaModalState((prev: PanoramaModalState) => ({ ...prev, error: err }));
      throw new Error(err);
    }

    try {
      const savedPath: string = await invoke(Invokes.SavePanorama, {
        firstPathStr: panoramaModalState.stitchingSourcePaths[0],
      });
      await refreshImageList();
      return savedPath;
    } catch (err) {
      console.error('Failed to save panorama:', err);
      setPanoramaModalState((prev: PanoramaModalState) => ({ ...prev, error: String(err) }));
      throw err;
    }
  };

  const handleApplyDenoise = useCallback(
    async (intensity: number) => {
      if (!denoiseModalState.targetPath) return;

      setDenoiseModalState((prev) => ({
        ...prev,
        isProcessing: true,
        error: null,
        progressMessage: 'Starting engine...',
      }));

      try {
        await invoke(Invokes.ApplyDenoising, {
          path: denoiseModalState.targetPath,
          intensity: intensity,
        });
      } catch (err) {
        setDenoiseModalState((prev) => ({
          ...prev,
          isProcessing: false,
          error: String(err),
        }));
      }
    },
    [denoiseModalState.targetPath],
  );

  const handleSaveDenoisedImage = async (): Promise<string> => {
    if (!denoiseModalState.targetPath) throw new Error('No target path');
    const savedPath = await invoke<string>(Invokes.SaveDenoisedImage, {
      originalPathStr: denoiseModalState.targetPath,
    });
    await refreshImageList();
    return savedPath;
  };

  const handleSaveCollage = async (base64Data: string, firstPath: string): Promise<string> => {
    try {
      const savedPath: string = await invoke(Invokes.SaveCollage, {
        base64Data,
        firstPathStr: firstPath,
      });
      await refreshImageList();
      return savedPath;
    } catch (err) {
      console.error('Failed to save collage:', err);
      setError(`Failed to save collage: ${err}`);
      throw err;
    }
  };

  const throttledInteractiveUpdate = useMemo(
    () =>
      throttle(
        (currentAdjustments) => {
          applyAdjustments(currentAdjustments, true);
        },
        120,
        { leading: true, trailing: true },
      ),
    [applyAdjustments],
  );

  useEffect(() => {
    return () => {
      throttledInteractiveUpdate.cancel();
    };
  }, [throttledInteractiveUpdate]);

  useEffect(() => {
    if (!selectedImage?.isReady) return;

    if (dragIdleTimer.current) {
      clearTimeout(dragIdleTimer.current);
    }

    if (isSliderDragging) {
      debouncedApplyAdjustments.cancel();

      const livePreviewsEnabled = appSettings?.enableLivePreviews !== false;
      const idleTimeoutDuration = livePreviewsEnabled ? 150 : 50;

      if (livePreviewsEnabled) {
        throttledInteractiveUpdate(adjustments);
      }

      dragIdleTimer.current = setTimeout(() => {
        applyAdjustments(adjustments, false);
      }, idleTimeoutDuration);
    } else {
      throttledInteractiveUpdate.cancel();
      debouncedApplyAdjustments(adjustments);
      debouncedSave(selectedImage.path, adjustments);
    }

    return () => {
      if (dragIdleTimer.current) clearTimeout(dragIdleTimer.current);
      debouncedApplyAdjustments.cancel();
    };
  }, [
    adjustments,
    selectedImage?.path,
    selectedImage?.isReady,
    isSliderDragging,
    applyAdjustments,
    debouncedApplyAdjustments,
    throttledInteractiveUpdate,
    debouncedSave,
    appSettings?.enableLivePreviews,
  ]);

  useEffect(() => {
    if (activeRightPanel === Panel.Crop && selectedImage?.isReady) {
      debouncedGenerateUncroppedPreview(adjustments);
    }

    return () => debouncedGenerateUncroppedPreview.cancel();
  }, [adjustments, activeRightPanel, selectedImage?.isReady, debouncedGenerateUncroppedPreview]);

  const handleOpenFolder = async () => {
    if (isCustomerMode) {
      setError('Folder browsing is disabled in customer mode.');
      return;
    }
    try {
      const selected = await open({ directory: true, multiple: false, defaultPath: await homeDir() });
      if (typeof selected === 'string') {
        setRootPath(selected);
        await handleSelectSubfolder(selected, true);
      }
    } catch (err) {
      console.error('Failed to open directory dialog:', err);
      setError('Failed to open folder selection dialog.');
    }
  };

  const handleStartSession = useCallback(
    async (sessionName: string) => {
      if (!sessionName.trim()) {
        return;
      }
      setIsStartingSession(true);
      setError(null);
      try {
        const session = await invoke(Invokes.BoothyCreateOrOpenSession, { sessionName });
        setShouldAutoOpenEditor(true);
        applyBoothySession(session as BoothySession);
      } catch (err) {
        console.error('Failed to start session:', err);
        setError(`Failed to start session: ${err}`);
      } finally {
        setIsStartingSession(false);
      }
    },
    [applyBoothySession],
  );

  const syncBoothyModeState = useCallback(() => {
    return invoke(Invokes.BoothyGetModeState)
      .then((state: BoothyModeState) => {
        if (state?.mode) {
          setBoothyMode(state.mode);
        }
        setBoothyHasAdminPassword(!!state?.has_admin_password);
      })
      .catch((err) => {
        console.error('Failed to sync Boothy mode state:', err);
      });
  }, []);

  const handleCameraReconnect = useCallback(async () => {
    if (isCameraReconnecting) {
      return;
    }
    setIsCameraReconnecting(true);
    setCameraReconnectResult(null);
    try {
      const result = await invoke<BoothyCameraReconnectResult>(Invokes.BoothyCameraReconnect);
      setCameraReconnectResult(result);
      await refreshCameraStatus();
    } catch (err) {
      setCameraReconnectResult({
        ok: false,
        attempts: 0,
        lastError: typeof err === 'string' ? err : '재연결에 실패했습니다.',
      });
    } finally {
      setIsCameraReconnecting(false);
    }
  }, [isCameraReconnecting, refreshCameraStatus]);

  const handleTriggerCapture = useCallback(async () => {
    if (!hasBoothySession) {
      setError('세션을 시작해 주세요.');
      return;
    }

    if (isEditingLocked) {
      setError('현재 촬영할 수 없습니다. 잠시 후 다시 시도해 주세요.');
      return;
    }

    if (isStorageLockout) {
      setError(STORAGE_CRITICAL_MESSAGE);
      return;
    }

    if (!cameraStatus?.status?.connected || !cameraStatus?.status?.cameraDetected) {
      setError(CAMERA_MESSAGE_NEEDS_CONNECTION);
      return;
    }

    try {
      await invoke(Invokes.BoothyTriggerCapture);
    } catch (err) {
      const message = typeof err === 'string' ? err : CAMERA_MESSAGE_UNAVAILABLE;
      setError(message);
    }
  }, [
    cameraStatus?.status?.cameraDetected,
    cameraStatus?.status?.connected,
    hasBoothySession,
    isEditingLocked,
    isStorageLockout,
  ]);

  const handleAdminToggle = useCallback(() => {
    if (boothyMode === 'admin') {
      invoke(Invokes.BoothySwitchToCustomerMode)
        .then(() => {
          setBoothyMode('customer');
          void syncBoothyModeState();
        })
        .catch((err) => {
          console.error('Failed to switch to customer mode:', err);
          setError('Failed to switch to customer mode.');
        });
      return;
    }
    setAdminModalError(null);
    setIsAdminModalOpen(true);
  }, [boothyMode, syncBoothyModeState]);

  const handleSetAdminPassword = useCallback(
    async (password: string) => {
      setIsAdminActionRunning(true);
      setAdminModalError(null);
      try {
        await invoke(Invokes.BoothySetAdminPassword, { password });
        setBoothyMode('admin');
        setBoothyHasAdminPassword(true);
        await syncBoothyModeState();
        setIsAdminModalOpen(false);
      } catch (err) {
        console.error('Failed to set admin password:', err);
        setAdminModalError('Failed to set admin password.');
      } finally {
        setIsAdminActionRunning(false);
      }
    },
    [syncBoothyModeState],
  );

  const handleUnlockAdmin = useCallback(
    async (password: string) => {
      setIsAdminActionRunning(true);
      setAdminModalError(null);
      try {
        const result = await invoke(Invokes.BoothyAuthenticateAdmin, { password });
        if (!result) {
          setAdminModalError('Incorrect password.');
          return;
        }
        setBoothyMode('admin');
        setBoothyHasAdminPassword(true);
        await syncBoothyModeState();
        setIsAdminModalOpen(false);
      } catch (err) {
        console.error('Failed to unlock admin mode:', err);
        setAdminModalError('Failed to unlock admin mode.');
      } finally {
        setIsAdminActionRunning(false);
      }
    },
    [syncBoothyModeState],
  );

  const handleContinueSession = () => {
    const restore = async () => {
      if (!appSettings?.lastRootPath) {
        return;
      }

      const root = appSettings.lastRootPath;
      const folderState = appSettings.lastFolderState;
      const pathToSelect = folderState?.currentFolderPath || root;

      setRootPath(root);

      if (folderState?.expandedFolders) {
        const newExpandedFolders = new Set(folderState.expandedFolders);
        newExpandedFolders.add(root);
        setExpandedFolders(newExpandedFolders);
      } else {
        setExpandedFolders(new Set([root]));
      }

      setIsTreeLoading(true);
      try {
        const treeData = await invoke(Invokes.GetFolderTree, { path: root });
        setFolderTree(treeData);
      } catch (err) {
        console.error('Failed to restore folder tree:', err);
      } finally {
        setIsTreeLoading(false);
      }

      await handleSelectSubfolder(pathToSelect, false);
    };
    restore().catch((err) => {
      console.error('Failed to restore session, folder might be missing:', err);
      setError('Failed to restore session. The last used folder may have been moved or deleted.');
      if (appSettings) {
        handleSettingsChange({ ...appSettings, lastRootPath: null, lastFolderState: null });
      }
      handleExitToHome();
      setIsTreeLoading(false);
    });
  };

  useEffect(() => {
    if (!initialFileToOpen || !appSettings) {
      return;
    }
    const parentDir = getParentDir(initialFileToOpen);
    if (currentFolderPath !== parentDir) {
      setRootPath(parentDir);
      handleSelectSubfolder(parentDir, true);
      return;
    }
    const isImageInList = imageList.some((image) => image.path === initialFileToOpen);
    if (isImageInList) {
      handleImageSelect(initialFileToOpen);
      setInitialFileToOpen(null);
    } else if (!isViewLoading) {
      console.warn(`'open-with-file' target ${initialFileToOpen} not found in its directory after loading. Aborting.`);
      setInitialFileToOpen(null);
    }
  }, [
    initialFileToOpen,
    appSettings,
    currentFolderPath,
    imageList,
    isViewLoading,
    handleSelectSubfolder,
    handleImageSelect,
  ]);

  const handleExitToHome = useCallback(() => {
    setActiveView('library');
    setShouldAutoOpenEditor(false);
    setIsStartingSession(false);
    setRootPath(null);
    setCurrentFolderPath(null);
    setImageList([]);
    setImageRatings({});
    setFolderTree(null);
    setMultiSelectedPaths([]);
    setLibraryActivePath(null);
    setIsLibraryExportPanelVisible(false);
    setExpandedFolders(new Set());
  }, []);

  const handleMultiSelectClick = (path: string, event: any, options: MultiSelectOptions) => {
    const { ctrlKey, metaKey, shiftKey } = event;
    const isCtrlPressed = ctrlKey || metaKey;
    const { shiftAnchor, onSimpleClick, updateLibraryActivePath } = options;

    if (shiftKey && shiftAnchor) {
      const lastIndex = sortedImageList.findIndex((f) => f.path === shiftAnchor);
      const currentIndex = sortedImageList.findIndex((f) => f.path === path);

      if (lastIndex !== -1 && currentIndex !== -1) {
        const start = Math.min(lastIndex, currentIndex);
        const end = Math.max(lastIndex, currentIndex);
        const range = sortedImageList.slice(start, end + 1).map((f: ImageFile) => f.path);
        const baseSelection = isCtrlPressed ? multiSelectedPaths : [shiftAnchor];
        const newSelection = Array.from(new Set([...baseSelection, ...range]));

        setMultiSelectedPaths(newSelection);
        if (updateLibraryActivePath) {
          setLibraryActivePath(path);
        }
      }
    } else if (isCtrlPressed) {
      const newSelection = new Set(multiSelectedPaths);
      if (newSelection.has(path)) {
        newSelection.delete(path);
      } else {
        newSelection.add(path);
      }

      const newSelectionArray = Array.from(newSelection);
      setMultiSelectedPaths(newSelectionArray);

      if (updateLibraryActivePath) {
        if (newSelectionArray.includes(path)) {
          setLibraryActivePath(path);
        } else if (newSelectionArray.length > 0) {
          setLibraryActivePath(newSelectionArray[newSelectionArray.length - 1]);
        } else {
          setLibraryActivePath(null);
        }
      }
    } else {
      onSimpleClick(path);
    }
  };

  const handleLibraryImageSingleClick = (path: string, event: any) => {
    handleMultiSelectClick(path, event, {
      shiftAnchor: libraryActivePath,
      updateLibraryActivePath: true,
      onSimpleClick: (p: any) => {
        setMultiSelectedPaths([p]);
        setLibraryActivePath(p);
      },
    });
  };

  const handleImageClick = (path: string, event: any) => {
    const inEditor = !!selectedImage;
    handleMultiSelectClick(path, event, {
      shiftAnchor: inEditor ? selectedImage.path : libraryActivePath,
      updateLibraryActivePath: !inEditor,
      onSimpleClick: handleImageSelect,
    });
  };

  useEffect(() => {
    const invokeWaveForm = async () => {
      const waveForm: any = await invoke(Invokes.GenerateWaveform).catch((err) =>
        console.error('Failed to generate waveform:', err),
      );
      if (waveForm) {
        setWaveform(waveForm);
      }
    };

    if (isWaveformVisible && selectedImage?.isReady && !waveform) {
      invokeWaveForm();
    }
  }, [isWaveformVisible, selectedImage?.isReady, waveform]);

  useEffect(() => {
    if (selectedImage && !selectedImage.isReady && selectedImage.path) {
      let isEffectActive = true;
      const loadFullImageData = async () => {
        try {
          const loadImageResult: any = await invoke(Invokes.LoadImage, { path: selectedImage.path });
          if (!isEffectActive) {
            return;
          }
          if (!isEffectActive) {
            return;
          }

          const { width, height } = loadImageResult;
          setOriginalSize({ width, height });

          if (appSettings?.editorPreviewResolution) {
            const maxSize = appSettings.editorPreviewResolution;
            const aspectRatio = width / height;

            if (width > height) {
              const pWidth = Math.min(width, maxSize);
              const pHeight = Math.round(pWidth / aspectRatio);
              setPreviewSize({ width: pWidth, height: pHeight });
            } else {
              const pHeight = Math.min(height, maxSize);
              const pWidth = Math.round(pHeight * aspectRatio);
              setPreviewSize({ width: pWidth, height: pHeight });
            }
          } else {
            setPreviewSize({ width: 0, height: 0 });
          }

          setIsFullResolution(false);
          setFullResolutionUrl(null);
          fullResCacheKeyRef.current = null;

          const blob = new Blob([loadImageResult.original_image_bytes], { type: 'image/jpeg' });
          const originalUrl = URL.createObjectURL(blob);

          setSelectedImage((currentSelected: SelectedImage | null) => {
            if (currentSelected && currentSelected.path === selectedImage.path) {
              return {
                ...currentSelected,
                exif: loadImageResult.exif,
                height: loadImageResult.height,
                isRaw: loadImageResult.is_raw,
                isReady: true,
                metadata: loadImageResult.metadata,
                originalUrl: originalUrl,
                width: loadImageResult.width,
              };
            }
            return currentSelected;
          });

          let initialAdjusts: Adjustments;
          const savedAdjustments = loadImageResult?.metadata?.adjustments;
          const hasSavedAdjustments = hasRenderableAdjustments(savedAdjustments);

          await ensurePresetCache();
          if (!isEffectActive) {
            return;
          }

          const isSessionActive = hasBoothySessionRef.current;
          const fallbackPreset = defaultPresetRef.current;
          const activePreset = currentPresetRef.current ?? fallbackPreset;

          if (hasSavedAdjustments) {
            initialAdjusts = normalizeLoadedAdjustments(savedAdjustments);
          } else {
            const shouldApplyPreset =
              isSessionActive && activePreset && isPlainObject(activePreset.adjustments);
            const presetAdjustments = shouldApplyPreset ? activePreset.adjustments : null;

            initialAdjusts = {
              ...INITIAL_ADJUSTMENTS,
              ...(presetAdjustments ?? {}),
              aspectRatio: loadImageResult.width / loadImageResult.height,
              lastPresetId: activePreset?.id ?? null,
              lastPresetName: activePreset?.name ?? null,
            };
          }

          if (!initialAdjusts.lastPresetId) {
            const matchedPreset = findBestMatchingPreset(initialAdjusts, presetsIndexRef.current);
            if (matchedPreset) {
              initialAdjusts.lastPresetId = matchedPreset.id;
              initialAdjusts.lastPresetName = matchedPreset.name;
            } else if (fallbackPreset) {
              initialAdjusts.lastPresetId = fallbackPreset.id;
              initialAdjusts.lastPresetName = fallbackPreset.name;
            }
          }
          setLiveAdjustments(initialAdjusts);
          resetAdjustmentsHistory(initialAdjusts);
        } catch (err) {
          if (isEffectActive) {
            if (isMissingFileError(err)) {
              await refreshSessionFiles();
              clearEditorSelection(null);
              return;
            }
            console.error('Failed to load image:', err);
            setError(`Failed to load image: ${err}`);
            clearEditorSelection(null);
          }
        } finally {
          if (isEffectActive) {
            setIsViewLoading(false);
          }
        }
      };
      loadFullImageData();
      return () => {
        isEffectActive = false;
      };
    }
  }, [
    selectedImage?.path,
    selectedImage?.isReady,
    resetAdjustmentsHistory,
    appSettings?.editorPreviewResolution,
    ensurePresetCache,
  ]);

  const handleClearSelection = () => {
    if (selectedImage) {
      setMultiSelectedPaths([selectedImage.path]);
    } else {
      setMultiSelectedPaths([]);
      setLibraryActivePath(null);
    }
  };

  const handleRenameFiles = useCallback(async (paths: Array<string>) => {
    if (paths && paths.length > 0) {
      setRenameTargetPaths(paths);
      setIsRenameFileModalOpen(true);
    }
  }, []);

  const handleSaveRename = useCallback(
    async (nameTemplate: string) => {
      if (renameTargetPaths.length > 0 && nameTemplate) {
        try {
          const newPaths: Array<string> = await invoke(Invokes.RenameFiles, {
            nameTemplate,
            paths: renameTargetPaths,
          });

          await refreshImageList();

          if (selectedImage && renameTargetPaths.includes(selectedImage.path)) {
            const oldPathIndex = renameTargetPaths.indexOf(selectedImage.path);

            if (newPaths[oldPathIndex]) {
              handleImageSelect(newPaths[oldPathIndex]);
            } else {
              handleBackToLibrary();
            }
          }

          if (libraryActivePath && renameTargetPaths.includes(libraryActivePath)) {
            const oldPathIndex = renameTargetPaths.indexOf(libraryActivePath);

            if (newPaths[oldPathIndex]) {
              setLibraryActivePath(newPaths[oldPathIndex]);
            } else {
              setLibraryActivePath(null);
            }
          }

          setMultiSelectedPaths(newPaths);
        } catch (err) {
          setError(`Failed to rename files: ${err}`);
        }
      }

      setRenameTargetPaths([]);
    },
    [renameTargetPaths, refreshImageList, selectedImage, libraryActivePath, handleImageSelect, handleBackToLibrary],
  );

  const handleStartImport = async (settings: AppSettings) => {
    if (importSourcePaths.length > 0 && importTargetFolder) {
      invoke(Invokes.ImportFiles, {
        destinationFolder: importTargetFolder,
        settings: settings,
        sourcePaths: importSourcePaths,
      }).catch((err) => {
        console.error('Failed to start import:', err);
        setImportState({ status: Status.Error, errorMessage: `Failed to start import: ${err}` });
      });
    }
  };

  const handleResetAdjustments = useCallback(
    (paths?: Array<string>) => {
      const pathsToReset = paths || multiSelectedPaths;
      if (pathsToReset.length === 0) {
        return;
      }

      debouncedSetHistory.cancel();

      invoke(Invokes.ResetAdjustmentsForPaths, { paths: pathsToReset })
        .then(() => {
          if (libraryActivePath && pathsToReset.includes(libraryActivePath)) {
            setLibraryActiveAdjustments((prev: Adjustments) => ({ ...INITIAL_ADJUSTMENTS, rating: prev.rating }));
          }
          if (selectedImage && pathsToReset.includes(selectedImage.path)) {
            const currentRating = adjustments.rating;
            resetAdjustmentsHistory({ ...INITIAL_ADJUSTMENTS, rating: currentRating });
          }
        })
        .catch((err) => {
          console.error('Failed to reset adjustments:', err);
          setError(`Failed to reset adjustments: ${err}`);
        });
    },
    [
      multiSelectedPaths,
      libraryActivePath,
      selectedImage,
      adjustments.rating,
      resetAdjustmentsHistory,
      debouncedSetHistory,
    ],
  );

  const handleImportClick = useCallback(
    async (targetPath: string) => {
      try {
        const nonRaw = supportedTypes?.nonRaw || [];
        const raw = supportedTypes?.raw || [];

        const expandExtensions = (exts: string[]) => {
          return Array.from(new Set(exts.flatMap((ext) => [ext.toLowerCase(), ext.toUpperCase()])));
        };

        const processedNonRaw = expandExtensions(nonRaw);
        const processedRaw = expandExtensions(raw);
        const allImageExtensions = [...processedNonRaw, ...processedRaw];

        const selected = await open({
          filters: [
            {
              name: 'All Supported Images',
              extensions: allImageExtensions,
            },
            {
              name: 'RAW Images',
              extensions: processedRaw,
            },
            {
              name: 'Standard Images (JPEG, PNG, etc.)',
              extensions: processedNonRaw,
            },
            {
              name: 'All Files',
              extensions: ['*'],
            },
          ],
          multiple: true,
          title: 'Select files to import',
        });

        if (Array.isArray(selected) && selected.length > 0) {
          setImportSourcePaths(selected);
          setImportTargetFolder(targetPath);
          setIsImportModalOpen(true);
        }
      } catch (err) {
        console.error('Failed to open file dialog for import:', err);
      }
    },
    [supportedTypes],
  );

  const handleEditorContextMenu = (event: any) => {
    event.preventDefault();
    event.stopPropagation();
    if (!selectedImage) return;

    const commonTags = getCommonTags([selectedImage.path]);

    const options: Array<Option> = [
      {
        label: 'Export Image',
        icon: Save,
        onClick: () => {
          setRenderedRightPanel(Panel.Export);
          setActiveRightPanel(Panel.Export);
        },
      },
      { type: OPTION_SEPARATOR },
      { label: 'Undo', icon: Undo, onClick: undo, disabled: !canUndo },
      { label: 'Redo', icon: Redo, onClick: redo, disabled: !canRedo },
      { type: OPTION_SEPARATOR },
      { label: 'Copy Adjustments', icon: Copy, onClick: handleCopyAdjustments },
      {
        label: 'Paste Adjustments',
        icon: ClipboardPaste,
        onClick: handlePasteAdjustments,
        disabled: copiedAdjustments === null,
      },
      { type: OPTION_SEPARATOR },
      { label: 'Auto Adjust Image', icon: Aperture, onClick: handleAutoAdjustments },
      {
        label: 'Rating',
        icon: Star,
        submenu: [0, 1, 2, 3, 4, 5].map((rating: number) => ({
          label: rating === 0 ? 'No Rating' : `${rating} Star${rating !== 1 ? 's' : ''}`,
          onClick: () => handleRate(rating),
        })),
      },
      {
        label: 'Color Label',
        icon: Palette,
        submenu: [
          { label: 'No Label', onClick: () => handleSetColorLabel(null) },
          ...COLOR_LABELS.map((label: Color) => ({
            label: label.name.charAt(0).toUpperCase() + label.name.slice(1),
            color: label.color,
            onClick: () => handleSetColorLabel(label.name),
          })),
        ],
      },
      {
        label: 'Tagging',
        icon: Tag,
        submenu: [
          {
            customComponent: TaggingSubMenu,
            customProps: {
              paths: [selectedImage.path],
              initialTags: commonTags,
              onTagsChanged: handleTagsChanged,
              appSettings,
            },
          },
        ],
      },
      { type: OPTION_SEPARATOR },
      {
        label: 'Reset Adjustments',
        icon: RotateCcw,
        onClick: () => {
          debouncedSetHistory.cancel();
          const currentRating = adjustments.rating;
          resetAdjustmentsHistory({ ...INITIAL_ADJUSTMENTS, rating: currentRating });
        },
      },
    ];
    showContextMenu(event.clientX, event.clientY, options);
  };

  const handleThumbnailContextMenu = (event: any, path: string) => {
    event.preventDefault();
    event.stopPropagation();

    const isTargetInSelection = multiSelectedPaths.includes(path);
    let finalSelection;

    if (!isTargetInSelection) {
      finalSelection = [path];
      setMultiSelectedPaths([path]);
      if (!selectedImage) {
        setLibraryActivePath(path);
      }
    } else {
      finalSelection = multiSelectedPaths;
    }

    const commonTags = getCommonTags(finalSelection);

    const selectionCount = finalSelection.length;
    const isSingleSelection = selectionCount === 1;
    const isEditingThisImage = selectedImage?.path === path;
    const deleteLabel = isSingleSelection ? 'Delete Image' : `Delete ${selectionCount} Images`;
    const exportLabel = isSingleSelection ? 'Export Image' : `Export ${selectionCount} Images`;

    const selectionHasVirtualCopies =
      isSingleSelection &&
      !finalSelection[0].includes('?vc=') &&
      imageList.some((image) => image.path.startsWith(`${finalSelection[0]}?vc=`));

    const hasAssociatedFiles = finalSelection.some((selectedPath) => {
      const lastDotIndex = selectedPath.lastIndexOf('.');
      if (lastDotIndex === -1) return false;
      const basePath = selectedPath.substring(0, lastDotIndex);
      return imageList.some((image) => image.path.startsWith(basePath + '.') && image.path !== selectedPath);
    });

    let deleteSubmenu;
    if (selectionHasVirtualCopies) {
      deleteSubmenu = [
        { label: 'Cancel', icon: X, onClick: () => { } },
        {
          label: 'Confirm Delete + Virtual Copies',
          icon: Check,
          isDestructive: true,
          onClick: () => executeDelete(finalSelection, { includeAssociated: false }),
        },
      ];
    } else if (hasAssociatedFiles) {
      deleteSubmenu = [
        { label: 'Cancel', icon: X, onClick: () => { } },
        {
          label: 'Delete Selected Only',
          icon: Check,
          isDestructive: true,
          onClick: () => executeDelete(finalSelection, { includeAssociated: false }),
        },
        {
          label: 'Delete + Associated',
          icon: Check,
          isDestructive: true,
          onClick: () => executeDelete(finalSelection, { includeAssociated: true }),
        },
      ];
    } else {
      deleteSubmenu = [
        { label: 'Cancel', icon: X, onClick: () => { } },
        {
          label: 'Confirm',
          icon: Check,
          isDestructive: true,
          onClick: () => executeDelete(finalSelection, { includeAssociated: false }),
        },
      ];
    }

    const deleteOption = {
      label: deleteLabel,
      icon: Trash2,
      isDestructive: true,
      submenu: deleteSubmenu,
    };

    const pasteLabel = isSingleSelection ? 'Paste Adjustments' : `Paste Adjustments to ${selectionCount} Images`;
    const resetLabel = isSingleSelection ? 'Reset Adjustments' : `Reset Adjustments on ${selectionCount} Images`;
    const copyLabel = isSingleSelection ? 'Copy Image' : `Copy ${selectionCount} Images`;
    const autoAdjustLabel = isSingleSelection ? 'Auto Adjust Image' : `Auto Adjust ${selectionCount} Images`;
    const renameLabel = isSingleSelection ? 'Rename Image' : `Rename ${selectionCount} Images`;
    const cullLabel = isSingleSelection ? 'Cull Image' : `Cull ${selectionCount} Images`;
    const collageLabel = isSingleSelection ? 'Create Collage' : 'Create Collage';
    const stitchLabel = 'Stitch Panorama';

    const handleCreateVirtualCopy = async (sourcePath: string) => {
      try {
        await invoke(Invokes.CreateVirtualCopy, { sourceVirtualPath: sourcePath });
        await refreshImageList();
      } catch (err) {
        console.error('Failed to create virtual copy:', err);
        setError(`Failed to create virtual copy: ${err}`);
      }
    };

    const handleApplyAutoAdjustmentsToSelection = () => {
      if (finalSelection.length === 0) return;

      invoke(Invokes.ApplyAutoAdjustmentsToPaths, { paths: finalSelection })
        .then(async () => {
          if (selectedImage && finalSelection.includes(selectedImage.path)) {
            const metadata: Metadata = await invoke(Invokes.LoadMetadata, { path: selectedImage.path });
            if (metadata.adjustments && !metadata.adjustments.is_null) {
              const normalized = normalizeLoadedAdjustments(metadata.adjustments);
              setLiveAdjustments(normalized);
              resetAdjustmentsHistory(normalized);
            }
          }
          if (libraryActivePath && finalSelection.includes(libraryActivePath)) {
            const metadata: Metadata = await invoke(Invokes.LoadMetadata, { path: libraryActivePath });
            if (metadata.adjustments && !metadata.adjustments.is_null) {
              const normalized = normalizeLoadedAdjustments(metadata.adjustments);
              setLibraryActiveAdjustments(normalized);
            }
          }
        })
        .catch((err) => {
          console.error('Failed to apply auto adjustments to paths:', err);
          setError(`Failed to apply auto adjustments: ${err}`);
        });
    };

    const onExportClick = () => {
      if (hasBoothySession) {
        handleExportDecisionOpen();
        return;
      }
      if (selectedImage) {
        if (selectedImage.path !== path) {
          handleImageSelect(path);
        }
        setRenderedRightPanel(Panel.Export);
        setActiveRightPanel(Panel.Export);
      } else {
        setIsLibraryExportPanelVisible(true);
      }
    };

    const options = [
      ...(!isEditingThisImage
        ? [
          {
            disabled: !isSingleSelection,
            icon: Edit,
            label: 'Edit Image',
            onClick: () => handleImageSelect(finalSelection[0]),
          },
          {
            icon: Save,
            label: exportLabel,
            onClick: onExportClick,
          },
          { type: OPTION_SEPARATOR },
        ]
        : [
          {
            icon: Save,
            label: exportLabel,
            onClick: onExportClick,
          },
          { type: OPTION_SEPARATOR },
        ]),
      {
        disabled: !isSingleSelection,
        icon: Copy,
        label: 'Copy Adjustments',
        onClick: async () => {
          try {
            const metadata: any = await invoke(Invokes.LoadMetadata, { path: finalSelection[0] });
            const sourceAdjustments =
              metadata.adjustments && !metadata.adjustments.is_null
                ? { ...INITIAL_ADJUSTMENTS, ...metadata.adjustments }
                : INITIAL_ADJUSTMENTS;
            const adjustmentsToCopy: any = {};
            for (const key of COPYABLE_ADJUSTMENT_KEYS) {
              if (Object.prototype.hasOwnProperty.call(sourceAdjustments, key)) {
                adjustmentsToCopy[key] = sourceAdjustments[key];
              }
            }
            setCopiedAdjustments(adjustmentsToCopy);
            setIsCopied(true);
          } catch (err) {
            console.error('Failed to load metadata for copy:', err);
            setError(`Failed to copy adjustments: ${err}`);
          }
        },
      },
      {
        disabled: copiedAdjustments === null,
        icon: ClipboardPaste,
        label: pasteLabel,
        onClick: handlePasteAdjustments,
      },
      {
        label: 'Productivity',
        icon: Gauge,
        submenu: [
          {
            label: autoAdjustLabel,
            icon: Aperture,
            onClick: handleApplyAutoAdjustmentsToSelection,
          },
          {
            disabled: !isSingleSelection,
            icon: CopyPlus,
            label: 'Create Virtual Copy',
            onClick: () => handleCreateVirtualCopy(finalSelection[0]),
          },
          {
            label: 'Denoise',
            icon: Grip,
            disabled: !isSingleSelection,
            onClick: () => {
              setDenoiseModalState({
                isOpen: true,
                isProcessing: false,
                previewBase64: null,
                error: null,
                targetPath: finalSelection[0],
                progressMessage: null,
              });
            },
          },
          {
            disabled: selectionCount < 2 || selectionCount > 30,
            icon: Images,
            label: stitchLabel,
            onClick: () => {
              setPanoramaModalState({
                error: null,
                finalImageBase64: null,
                isOpen: true,
                progressMessage: 'Starting panorama process...',
                stitchingSourcePaths: finalSelection,
              });
              invoke(Invokes.StitchPanorama, { paths: finalSelection }).catch((err) => {
                setPanoramaModalState((prev: PanoramaModalState) => ({
                  ...prev,
                  error: String(err),
                  isOpen: true,
                  progressMessage: 'Failed to start.',
                }));
              });
            },
          },
          {
            icon: LayoutTemplate,
            label: collageLabel,
            onClick: () => {
              const imagesForCollage = imageList.filter((img) => finalSelection.includes(img.path));
              setCollageModalState({
                isOpen: true,
                sourceImages: imagesForCollage,
              });
            },
            disabled: selectionCount === 0 || selectionCount > 9,
          },
          {
            label: cullLabel,
            icon: Star,
            onClick: () =>
              setCullingModalState({
                isOpen: true,
                progress: null,
                suggestions: null,
                error: null,
                pathsToCull: finalSelection,
              }),
            disabled: imageList.length < 2,
          },
        ],
      },
      { type: OPTION_SEPARATOR },
      {
        label: copyLabel,
        icon: Copy,
        onClick: () => {
          setCopiedFilePaths(finalSelection);
          setIsCopied(true);
        },
      },
      {
        disabled: !isSingleSelection,
        icon: CopyPlus,
        label: 'Duplicate Image',
        onClick: async () => {
          try {
            await invoke(Invokes.DuplicateFile, { path: finalSelection[0] });
            await refreshImageList();
          } catch (err) {
            console.error('Failed to duplicate file:', err);
            setError(`Failed to duplicate file: ${err}`);
          }
        },
      },
      { icon: FileEdit, label: renameLabel, onClick: () => handleRenameFiles(finalSelection) },
      { type: OPTION_SEPARATOR },
      {
        icon: Star,
        label: 'Rating',
        submenu: [0, 1, 2, 3, 4, 5].map((rating: number) => ({
          label: rating === 0 ? 'No Rating' : `${rating} Star${rating !== 1 ? 's' : ''}`,
          onClick: () => handleRate(rating, finalSelection),
        })),
      },
      {
        label: 'Color Label',
        icon: Palette,
        submenu: [
          { label: 'No Label', onClick: () => handleSetColorLabel(null, finalSelection) },
          ...COLOR_LABELS.map((label: Color) => ({
            label: label.name.charAt(0).toUpperCase() + label.name.slice(1),
            color: label.color,
            onClick: () => handleSetColorLabel(label.name, finalSelection),
          })),
        ],
      },
      {
        label: 'Tagging',
        icon: Tag,
        submenu: [
          {
            customComponent: TaggingSubMenu,
            customProps: {
              paths: finalSelection,
              initialTags: commonTags,
              onTagsChanged: handleTagsChanged,
              appSettings,
            },
          },
        ],
      },
      { type: OPTION_SEPARATOR },
      {
        disabled: !isSingleSelection,
        icon: Folder,
        label: 'Show in File Explorer',
        onClick: () => {
          invoke(Invokes.ShowInFinder, { path: finalSelection[0] }).catch((err) =>
            setError(`Could not show file in explorer: ${err}`),
          );
        },
      },
      { label: resetLabel, icon: RotateCcw, onClick: () => handleResetAdjustments(finalSelection) },
      deleteOption,
    ];
    showContextMenu(event.clientX, event.clientY, options);
  };

  const handleCreateFolder = async (folderName: string) => {
    if (folderName && folderName.trim() !== '' && folderActionTarget) {
      try {
        await invoke(Invokes.CreateFolder, { path: `${folderActionTarget}/${folderName.trim()}` });
        refreshAllFolderTrees();
      } catch (err) {
        setError(`Failed to create folder: ${err}`);
      }
    }
  };

  const handleRenameFolder = async (newName: string) => {
    if (newName && newName.trim() !== '' && folderActionTarget) {
      try {
        const oldPath = folderActionTarget;
        const trimmedNewName = newName.trim();

        await invoke(Invokes.RenameFolder, { path: oldPath, newName: trimmedNewName });

        const parentDir = getParentDir(oldPath);
        const separator = oldPath.includes('/') ? '/' : '\\';
        const newPath = parentDir ? `${parentDir}${separator}${trimmedNewName}` : trimmedNewName;

        const newAppSettings = { ...appSettings } as AppSettings;
        let settingsChanged = false;

        if (rootPath === oldPath) {
          setRootPath(newPath);
          newAppSettings.lastRootPath = newPath;
          settingsChanged = true;
        }
        if (currentFolderPath?.startsWith(oldPath)) {
          const newCurrentPath = currentFolderPath.replace(oldPath, newPath);
          setCurrentFolderPath(newCurrentPath);
        }

        const currentPins = appSettings?.pinnedFolders || [];
        if (currentPins.includes(oldPath)) {
          const newPins = currentPins.map((p) => (p === oldPath ? newPath : p)).sort((a, b) => a.localeCompare(b));
          newAppSettings.pinnedFolders = newPins;
          settingsChanged = true;
        }

        if (settingsChanged) {
          handleSettingsChange(newAppSettings);
        }

        await refreshAllFolderTrees();
      } catch (err) {
        setError(`Failed to rename folder: ${err}`);
      }
    }
  };

  const handleFolderTreeContextMenu = (event: any, path: string, isCurrentlyPinned?: boolean) => {
    event.preventDefault();
    event.stopPropagation();
    const targetPath = path || rootPath;
    if (!targetPath) {
      return;
    }
    const isRoot = targetPath === rootPath;
    const numCopied = copiedFilePaths.length;
    const copyPastedLabel = numCopied === 1 ? 'Copy image here' : `Copy ${numCopied} images here`;
    const movePastedLabel = numCopied === 1 ? 'Move image here' : `Move ${numCopied} images here`;

    const pinOption = isCurrentlyPinned
      ? {
        icon: PinOff,
        label: 'Unpin Folder',
        onClick: () => handleTogglePinFolder(targetPath),
      }
      : {
        icon: Pin,
        label: 'Pin Folder',
        onClick: () => handleTogglePinFolder(targetPath),
      };

    const options = [
      pinOption,
      { type: OPTION_SEPARATOR },
      {
        icon: FolderPlus,
        label: 'New Folder',
        onClick: () => {
          setFolderActionTarget(targetPath);
          setIsCreateFolderModalOpen(true);
        },
      },
      {
        disabled: isRoot,
        icon: FileEdit,
        label: 'Rename Folder',
        onClick: () => {
          setFolderActionTarget(targetPath);
          setIsRenameFolderModalOpen(true);
        },
      },
      { type: OPTION_SEPARATOR },
      {
        disabled: copiedFilePaths.length === 0,
        icon: ClipboardPaste,
        label: 'Paste',
        submenu: [
          {
            label: copyPastedLabel,
            onClick: async () => {
              try {
                await invoke(Invokes.CopyFiles, { sourcePaths: copiedFilePaths, destinationFolder: targetPath });
                if (targetPath === currentFolderPath) handleLibraryRefresh();
              } catch (err) {
                setError(`Failed to copy files: ${err}`);
              }
            },
          },
          {
            label: movePastedLabel,
            onClick: async () => {
              try {
                await invoke(Invokes.MoveFiles, { sourcePaths: copiedFilePaths, destinationFolder: targetPath });
                setCopiedFilePaths([]);
                setMultiSelectedPaths([]);
                refreshAllFolderTrees();
                handleLibraryRefresh();
              } catch (err) {
                setError(`Failed to move files: ${err}`);
              }
            },
          },
        ],
      },
      { icon: FolderInput, label: 'Import Images', onClick: () => handleImportClick(targetPath) },
      { type: OPTION_SEPARATOR },
      {
        icon: Folder,
        label: 'Show in File Explorer',
        onClick: () =>
          invoke(Invokes.ShowInFinder, { path: targetPath }).catch((err) => setError(`Could not show folder: ${err}`)),
      },
      ...(path
        ? [
          {
            disabled: isRoot,
            icon: Trash2,
            isDestructive: true,
            label: 'Delete Folder',
            submenu: [
              { label: 'Cancel', icon: X, onClick: () => { } },
              {
                label: 'Confirm',
                icon: Check,
                isDestructive: true,
                onClick: async () => {
                  try {
                    await invoke(Invokes.DeleteFolder, { path: targetPath });
                    if (currentFolderPath?.startsWith(targetPath)) await handleSelectSubfolder(rootPath);
                    refreshAllFolderTrees();
                  } catch (err) {
                    setError(`Failed to delete folder: ${err}`);
                  }
                },
              },
            ],
          },
        ]
        : []),
    ];
    showContextMenu(event.clientX, event.clientY, options);
  };

  const handleMainLibraryContextMenu = (event: any) => {
    event.preventDefault();
    event.stopPropagation();
    const numCopied = copiedFilePaths.length;
    const copyPastedLabel = numCopied === 1 ? 'Copy image here' : `Copy ${numCopied} images here`;
    const movePastedLabel = numCopied === 1 ? 'Move image here' : `Move ${numCopied} images here`;

    const options = [
      {
        label: 'Paste',
        icon: ClipboardPaste,
        disabled: copiedFilePaths.length === 0,
        submenu: [
          {
            label: copyPastedLabel,
            onClick: async () => {
              try {
                await invoke(Invokes.CopyFiles, { sourcePaths: copiedFilePaths, destinationFolder: currentFolderPath });
                handleLibraryRefresh();
              } catch (err) {
                setError(`Failed to copy files: ${err}`);
              }
            },
          },
          {
            label: movePastedLabel,
            onClick: async () => {
              try {
                await invoke(Invokes.MoveFiles, { sourcePaths: copiedFilePaths, destinationFolder: currentFolderPath });
                setCopiedFilePaths([]);
                setMultiSelectedPaths([]);
                refreshAllFolderTrees();
                handleLibraryRefresh();
              } catch (err) {
                setError(`Failed to move files: ${err}`);
              }
            },
          },
        ],
      },
      {
        icon: FolderInput,
        label: 'Import Images',
        onClick: () => handleImportClick(currentFolderPath as string),
        disabled: !currentFolderPath,
      },
    ];
    showContextMenu(event.clientX, event.clientY, options);
  };

  const memoizedFolderTree = useMemo(
    () =>
      rootPath &&
      !isCustomerMode && (
        <>
          <FolderTree
            expandedFolders={expandedFolders}
            isLoading={isTreeLoading}
            isResizing={isResizing}
            isVisible={uiVisibility.folderTree}
            onContextMenu={handleFolderTreeContextMenu}
            onFolderSelect={(path) => handleSelectSubfolder(path, false)}
            onToggleFolder={handleToggleFolder}
            selectedPath={currentFolderPath}
            setIsVisible={(value: boolean) => setUiVisibility((prev: UiVisibility) => ({ ...prev, folderTree: value }))}
            style={{ width: uiVisibility.folderTree ? `${leftPanelWidth}px` : '32px' }}
            tree={folderTree}
            pinnedFolderTrees={pinnedFolderTrees}
            pinnedFolders={pinnedFolders}
            activeSection={activeTreeSection}
            onActiveSectionChange={handleActiveTreeSectionChange}
          />
          <Resizer
            direction={Orientation.Vertical}
            onMouseDown={createResizeHandler(setLeftPanelWidth, leftPanelWidth)}
          />
        </>
      ),
    [
      rootPath,
      isCustomerMode,
      expandedFolders,
      isTreeLoading,
      isResizing,
      handleSelectSubfolder,
      uiVisibility.folderTree,
      currentFolderPath,
      leftPanelWidth,
      folderTree,
      pinnedFolderTrees,
      pinnedFolders,
      activeTreeSection,
    ],
  );

  const boothySessionLabel = boothySession?.session_name || boothySession?.session_folder_name || null;

  const isCameraReady = Boolean(
    cameraStatus?.ipcState === 'connected' &&
      (((() => {
        if (!cameraStatusSnapshot) return false;
        const observedAtMs = Date.parse(cameraStatusSnapshot.observedAt);
        if (!Number.isFinite(observedAtMs)) return false;
        // Prefer push snapshots, but treat them as stale quickly so pull(getStatus) can take over
        // when sidecar stops emitting events (power-cycle edge cases).
        return (
          Date.now() - observedAtMs <= CAMERA_SNAPSHOT_TTL_MS && cameraStatusSnapshot.state === 'ready'
        );
      })()) ||
        (cameraStatusSnapshot == null &&
          cameraStatus?.status?.connected &&
          cameraStatus?.status?.cameraDetected)),
  );

  const cameraStatusSnapshotFresh: BoothyCameraStatusSnapshot | null = (() => {
    if (!cameraStatusSnapshot) return null;
    const observedAtMs = Date.parse(cameraStatusSnapshot.observedAt);
    if (!Number.isFinite(observedAtMs)) return null;
    return Date.now() - observedAtMs <= CAMERA_SNAPSHOT_TTL_MS ? cameraStatusSnapshot : null;
  })();

  const [customerCameraConnectionStateForLamp, setCustomerCameraConnectionStateForLamp] = useState<
    'connected' | 'disconnected'
  >(() => {
    if (!isCustomerMode) return 'connected';
    if (cameraStatus?.ipcState === 'disconnected') return 'disconnected';
    if (cameraStatus?.status?.connected === false) return 'disconnected';
    if (
      cameraStatus?.ipcState === 'connected' &&
      cameraStatus?.status?.connected === true &&
      cameraStatus?.status?.cameraDetected === true
    ) {
      return 'connected';
    }
    return 'disconnected';
  });

  useEffect(() => {
    if (!isCustomerMode) {
      setCustomerCameraConnectionStateForLamp('connected');
      return;
    }
    setCustomerCameraConnectionStateForLamp((prev) =>
      nextCustomerCameraLampConnectionState({ prev, report: cameraStatus }),
    );
  }, [cameraStatus, isCustomerMode]);

  const customerCameraStatusMessage = useMemo(() => {
    if (!isCustomerMode || !hasBoothySession) {
      return null;
    }
    if (cameraStatusError) {
      return CAMERA_MESSAGE_UNAVAILABLE;
    }
    if (cameraStatus?.lastError && cameraStatus?.ipcState !== 'connected') {
      return CAMERA_MESSAGE_UNAVAILABLE;
    }
    if (cameraStatusLoading || isCameraReconnecting || cameraStatus?.ipcState === 'reconnecting') {
      return CAMERA_MESSAGE_PREPARING;
    }
    if (cameraStatus?.ipcState !== 'connected') {
      return CAMERA_MESSAGE_PREPARING;
    }
    if (cameraStatusSnapshotFresh?.mode === 'mock') {
      return CAMERA_MESSAGE_UNAVAILABLE;
    }
    if (cameraStatusSnapshotFresh?.state === 'connecting') {
      return CAMERA_MESSAGE_PREPARING;
    }
    if (cameraStatusSnapshotFresh?.state === 'noCamera') {
      // In customer mode we only want to communicate "connected vs not connected".
      // Treat transient "noCamera" snapshots as "preparing" when the backend still reports the camera as connected.
      if (cameraStatus?.status?.connected === false) {
        return CAMERA_MESSAGE_NEEDS_CONNECTION;
      }
      return CAMERA_MESSAGE_PREPARING;
    }
    if (cameraStatusSnapshotFresh?.state === 'error') {
      return CAMERA_MESSAGE_UNAVAILABLE;
    }
    if (cameraStatus?.status && !cameraStatus.status.cameraDetected) {
      return CAMERA_MESSAGE_PREPARING;
    }
    if (cameraStatus?.status && !cameraStatus.status.connected) {
      return CAMERA_MESSAGE_NEEDS_CONNECTION;
    }
    return null;
  }, [
    cameraStatus,
    cameraStatusError,
    cameraStatusLoading,
    cameraStatusSnapshotFresh,
    hasBoothySession,
    isCameraReconnecting,
    isCustomerMode,
  ]);

  useEffect(() => {
    const shouldPoll =
      isCustomerMode &&
      hasBoothySession &&
      (cameraStatusLoading || cameraStatusError != null || customerCameraStatusMessage != null);

    const clearRecoveryPoll = () => {
      if (cameraStatusRecoveryPollIntervalRef.current !== null) {
        window.clearInterval(cameraStatusRecoveryPollIntervalRef.current);
        cameraStatusRecoveryPollIntervalRef.current = null;
      }
      if (cameraStatusRecoveryPollSlowSwitchTimeoutRef.current !== null) {
        window.clearTimeout(cameraStatusRecoveryPollSlowSwitchTimeoutRef.current);
        cameraStatusRecoveryPollSlowSwitchTimeoutRef.current = null;
      }
    };

    if (!shouldPoll) {
      clearRecoveryPoll();
      return;
    }

    // Fast recovery polling: when the lamp is yellow/red, refresh more aggressively so it can
    // return to green quickly as soon as the backend/sidecar recovers.
    //
    // To avoid spamming IPC when the camera is truly disconnected for a long time, we switch to a
    // slower interval after a short window.
    const startInterval = (intervalMs: number) => {
      if (cameraStatusRecoveryPollIntervalRef.current !== null) {
        window.clearInterval(cameraStatusRecoveryPollIntervalRef.current);
      }
      cameraStatusRecoveryPollIntervalRef.current = window.setInterval(() => {
        void refreshCameraStatus();
      }, intervalMs);
    };

    startInterval(250);
    cameraStatusRecoveryPollSlowSwitchTimeoutRef.current = window.setTimeout(() => {
      startInterval(1000);
    }, 30000);

    return clearRecoveryPoll;
  }, [
    isCustomerMode,
    hasBoothySession,
    cameraStatusLoading,
    cameraStatusError,
    customerCameraStatusMessage,
    refreshCameraStatus,
  ]);

  const captureStatusMessage = useMemo(() => {
    if (!isCustomerMode || !hasBoothySession) {
      return null;
    }
    if (captureStatus === 'error') {
      return captureErrorMessage || '촬영 중 오류가 발생했습니다.';
    }
    switch (captureStatus) {
      case 'capturing':
        return '촬영 중...';
      case 'transferring':
        return '전송 중...';
      case 'stabilizing':
        return '정리 중...';
      case 'importing':
        return '가져오는 중...';
      case 'ready':
        return '촬영 완료';
      default:
        return null;
    }
  }, [captureErrorMessage, captureStatus, hasBoothySession, isCustomerMode]);

  const isCaptureDisabled =
    !hasBoothySession ||
    isEditingLocked ||
    isStorageLockout ||
    !isCameraReady ||
    isCameraReconnecting ||
    cameraStatusLoading;

  const handleLibraryExportClick = useCallback(() => {
    if (hasBoothySession) {
      handleExportDecisionOpen();
      return;
    }
    setIsLibraryExportPanelVisible((prev) => !prev);
  }, [handleExportDecisionOpen, hasBoothySession]);

  const memoizedLibraryView = useMemo(
    () => (
      <div className="flex flex-row flex-grow h-full min-h-0">
        <div className="flex-1 flex flex-col min-w-0 gap-2">
             <MainLibrary
                activePath={libraryActivePath}
                appSettings={appSettings}
                boothySessionName={boothySessionLabel}
                cameraStatusReport={cameraStatus}
                cameraStatusSnapshot={cameraStatusSnapshotFresh}
                isCameraStatusLoading={cameraStatusLoading}
                isCameraReconnecting={isCameraReconnecting}
                cameraStatusMessage={customerCameraStatusMessage}
                isCameraUnavailable={customerCameraStatusMessage === CAMERA_MESSAGE_UNAVAILABLE}
                customerCameraConnectionState={customerCameraConnectionStateForLamp}
                captureStatus={captureStatus}
                captureStatusMessage={captureStatusMessage}
               sessionRemainingSeconds={sessionRemainingSeconds}
             currentFolderPath={currentFolderPath}
            filterCriteria={filterCriteria}
            imageList={sortedImageList}
            imageRatings={imageRatings}
            importState={importState}
            isAdmin={boothyMode === 'admin'}
            isCustomerMode={isCustomerMode}
            isThumbnailsLoading={isThumbnailsLoading}
            isLoading={isViewLoading}
            isStartingSession={isStartingSession}
            libraryScrollTop={libraryScrollTop}
            libraryViewMode={libraryViewMode}
            multiSelectedPaths={multiSelectedPaths}
            onClearSelection={handleClearSelection}
            onContextMenu={handleThumbnailContextMenu}
            onContinueSession={handleContinueSession}
            onEmptyAreaContextMenu={handleMainLibraryContextMenu}
            onGoEditor={handleReturnToEditor}
            onExitToHome={handleExitToHome}
            onImageClick={handleLibraryImageSingleClick}
            onImageDoubleClick={handleImageSelect}
            onLibraryRefresh={handleLibraryRefresh}
            onOpenFolder={handleOpenFolder}
            onStartSession={handleStartSession}
            onTriggerCapture={handleTriggerCapture}
            onSettingsChange={handleSettingsChange}
            onThumbnailAspectRatioChange={setThumbnailAspectRatio}
            onThumbnailSizeChange={setThumbnailSize}
            isCaptureDisabled={isCaptureDisabled}
            rootPath={rootPath}
            searchCriteria={searchCriteria}
            setFilterCriteria={setFilterCriteria}
            setLibraryScrollTop={setLibraryScrollTop}
            setLibraryViewMode={setLibraryViewMode}
            setSearchCriteria={setSearchCriteria}
            setSortCriteria={setSortCriteria}
            sortCriteria={sortCriteria}
            theme={theme}
            thumbnailAspectRatio={thumbnailAspectRatio}
            thumbnails={thumbnails}
            thumbnailSize={thumbnailSize}
          />
          {rootPath && (
            <BottomBar
              isCopied={isCopied}
              isCopyDisabled={isEditingLocked || multiSelectedPaths.length !== 1}
              isExportDisabled={isEditingLocked || multiSelectedPaths.length === 0}
              isCustomerMode={isCustomerMode}
              isLibraryView={true}
              isPasted={isPasted}
              isPasteDisabled={isEditingLocked || copiedAdjustments === null || multiSelectedPaths.length === 0}
              isRatingDisabled={isEditingLocked || multiSelectedPaths.length === 0}
              isResetDisabled={isEditingLocked || multiSelectedPaths.length === 0}
              multiSelectedPaths={multiSelectedPaths}
              onCopy={handleCopyAdjustments}
              onExportClick={handleLibraryExportClick}
              onOpenCopyPasteSettings={() => setIsCopyPasteSettingsModalOpen(true)}
              onPaste={() => handlePasteAdjustments()}
              onRate={handleRate}
              onReset={() => handleResetAdjustments()}
              rating={libraryActiveAdjustments.rating || 0}
              thumbnailAspectRatio={thumbnailAspectRatio}
            />
          )}
        </div>
      </div>
    ),
    [
      sortedImageList,
      currentFolderPath,
      libraryActivePath,
      appSettings,
      boothySessionLabel,
      cameraStatus,
      cameraStatusSnapshotFresh,
      cameraStatusLoading,
      customerCameraStatusMessage,
      captureStatus,
      captureStatusMessage,
      isCaptureDisabled,
      filterCriteria,
      imageRatings,
      importState,
      isThumbnailsLoading,
      isViewLoading,
      libraryScrollTop,
      libraryViewMode,
      multiSelectedPaths,
      rootPath,
      searchCriteria,
      sortCriteria,
      theme,
      thumbnailAspectRatio,
      thumbnails,
      thumbnailSize,
      sessionRemainingSeconds,
      isCopied,
      isPasted,
      copiedAdjustments,
      libraryActiveAdjustments,
      isCustomerMode,
      isCameraReconnecting,
      isEditingLocked,
      isStartingSession,
      boothyMode,
      handleClearSelection,
      handleThumbnailContextMenu,
      handleContinueSession,
      handleMainLibraryContextMenu,
      handleLibraryExportClick,
      handleCopyAdjustments,
      handlePasteAdjustments,
      handleRate,
      handleResetAdjustments,
      handleLibraryRefresh,
      handleOpenFolder,
      handleReturnToEditor,
      handleExitToHome,
      handleLibraryImageSingleClick,
      handleImageSelect,
      handleStartSession,
      handleTriggerCapture,
      handleSettingsChange,
      setFilterCriteria,
      setLibraryScrollTop,
      setLibraryViewMode,
      setSearchCriteria,
      setSortCriteria,
      setThumbnailAspectRatio,
      setThumbnailSize,
      setIsCopyPasteSettingsModalOpen,
    ],
  );

  const renderEditorEmptyState = () => (
    <div className="flex-1 bg-bg-secondary rounded-lg flex flex-col relative overflow-hidden p-2 gap-2 min-h-0">
      <div className="flex items-center justify-end px-4 h-14">
        <SessionCountdown remainingSeconds={sessionRemainingSeconds ?? null} size="sm" />
      </div>
      <div className="flex-1 flex items-center justify-center text-text-secondary">
        <p>Select an image from the library to begin editing.</p>
      </div>
    </div>
  );

  const renderEditorView = () => {
    const panelVariants: any = {
      animate: { opacity: 1, y: 0, transition: { duration: 0.2, ease: 'circOut' } },
      exit: { opacity: 0.4, y: -20, transition: { duration: 0.1, ease: 'circIn' } },
      initial: { opacity: 0.4, y: 20 },
    };
    const hasSelection = !!selectedImage;

    return (
      <div className="flex flex-row flex-grow h-full min-h-0">
        <div className="flex-1 flex flex-col min-w-0">
          {hasSelection ? (
            <Editor
              activeMaskContainerId={activeMaskContainerId}
              activeMaskId={activeMaskId}
              activeRightPanel={activeRightPanel}
              adjustments={adjustments}
              brushSettings={brushSettings}
              canRedo={canRedo}
              canUndo={canUndo}
              finalPreviewUrl={finalPreviewUrl}
              fullScreenUrl={fullScreenUrl}
              isFullScreen={isFullScreen}
              isFullScreenLoading={isFullScreenLoading}
              isLoading={isViewLoading}
              isMaskControlHovered={isMaskControlHovered}
              isStraightenActive={isStraightenActive}
              isWaveformVisible={isWaveformVisible}
              sessionRemainingSeconds={sessionRemainingSeconds}
              onBackToLibrary={handleBackToLibrary}
              onCloseWaveform={() => setIsWaveformVisible(false)}
              onContextMenu={handleEditorContextMenu}
              onRedo={redo}
              onSelectMask={setActiveMaskId}
              onStraighten={handleStraighten}
              onToggleFullScreen={handleToggleFullScreen}
              onToggleWaveform={handleToggleWaveform}
              onUndo={undo}
              onZoomed={handleUserTransform}
              renderedRightPanel={renderedRightPanel}
              selectedImage={selectedImage}
              isWbPickerActive={isWbPickerActive}
              onWbPicked={handleWbPicked}
              setAdjustments={setAdjustments}
              setShowOriginal={setShowOriginal}
              showOriginal={showOriginal}
              targetZoom={zoom}
              thumbnails={thumbnails}
              transformWrapperRef={transformWrapperRef}
              transformedOriginalUrl={transformedOriginalUrl}
              uncroppedAdjustedPreviewUrl={uncroppedAdjustedPreviewUrl}
              updateSubMask={updateSubMask}
              waveform={waveform}
              onDisplaySizeChange={handleDisplaySizeChange}
              onInitialFitScale={setInitialFitScale}
              originalSize={originalSize}
              isFullResolution={isFullResolution}
              fullResolutionUrl={fullResolutionUrl}
              isLoadingFullRes={isLoadingFullRes}
            />
          ) : (
            renderEditorEmptyState()
          )}
          <Resizer
            direction={Orientation.Horizontal}
            onMouseDown={createResizeHandler(setBottomPanelHeight, bottomPanelHeight)}
          />
          <BottomBar
            filmstripHeight={bottomPanelHeight}
            imageList={sortedImageList}
            imageRatings={imageRatings}
            isCopied={isCopied}
            isCopyDisabled={isEditingLocked || !selectedImage}
            isCustomerMode={isCustomerMode}
            isFilmstripVisible={uiVisibility.filmstrip}
            isLoading={isViewLoading}
            isPasted={isPasted}
            isPasteDisabled={isEditingLocked || copiedAdjustments === null}
            isRatingDisabled={isEditingLocked || !selectedImage}
            isResizing={isResizing}
            multiSelectedPaths={multiSelectedPaths}
            displaySize={displaySize}
            originalSize={originalSize}
            onClearSelection={handleClearSelection}
            onContextMenu={handleThumbnailContextMenu}
            onCopy={handleCopyAdjustments}
            onOpenCopyPasteSettings={() => setIsCopyPasteSettingsModalOpen(true)}
            onImageSelect={handleImageClick}
            onPaste={() => handlePasteAdjustments()}
            onRate={handleRate}
            onZoomChange={handleZoomChange}
            rating={adjustments.rating || 0}
            selectedImage={selectedImage}
            setIsFilmstripVisible={(value: boolean) =>
              setUiVisibility((prev: UiVisibility) => ({ ...prev, filmstrip: value }))
            }
            thumbnailAspectRatio={thumbnailAspectRatio}
            thumbnails={thumbnails}
          />
        </div>

        <Resizer
          onMouseDown={createResizeHandler(setRightPanelWidth, rightPanelWidth)}
          direction={Orientation.Vertical}
        />
        <div className="flex bg-bg-secondary rounded-lg h-full">
          <div
            className={clsx('h-full overflow-hidden', !isResizing && 'transition-all duration-300 ease-in-out')}
            style={{ width: activeRightPanel && hasSelection ? `${rightPanelWidth}px` : '0px' }}
          >
            <div style={{ width: `${rightPanelWidth}px` }} className="h-full min-h-0 flex flex-col">
              {hasSelection && (
                <>
                  <div className="px-4 py-2 border-b border-surface text-xs text-text-secondary flex items-center gap-2 flex-shrink-0">
                    <span className="uppercase tracking-wider">Preset</span>
                    <div
                      className={clsx(
                        'min-w-0 flex-1 inline-flex items-center gap-2',
                        'px-2.5 py-1 rounded-xl border bg-surface/50',
                        selectedImagePresetName ? 'border-accent/30' : 'border-border-color/30',
                      )}
                      title={selectedImagePresetName || 'None'}
                    >
                      <span
                        className={clsx('h-2 w-2 rounded-full flex-shrink-0 ring-2 ring-transparent', [
                          selectedImagePresetName ? 'bg-accent ring-accent/50' : 'bg-text-secondary/40',
                        ])}
                        aria-hidden="true"
                      />
                      <span
                        className={clsx(
                          'min-w-0 truncate font-medium',
                          selectedImagePresetName ? 'text-text-primary' : 'text-text-secondary',
                        )}
                      >
                        {selectedImagePresetName || 'None'}
                      </span>
                    </div>
                  </div>
                  <AnimatePresence mode="wait">
                    {activeRightPanel && (
                      <motion.div
                        animate="animate"
                        className="flex-1 min-h-0 w-full"
                        exit="exit"
                        initial="initial"
                        key={renderedRightPanel}
                        variants={panelVariants}
                      >
                        {renderedRightPanel === Panel.Adjustments && (
                          <Controls
                            adjustments={adjustments}
                            collapsibleState={collapsibleSectionsState}
                            copiedSectionAdjustments={copiedSectionAdjustments}
                            handleAutoAdjustments={handleAutoAdjustments}
                            histogram={histogram}
                            selectedImage={selectedImage}
                            setAdjustments={setAdjustments}
                            setCollapsibleState={setCollapsibleSectionsState}
                            setCopiedSectionAdjustments={setCopiedSectionAdjustments}
                            theme={theme}
                            handleLutSelect={handleLutSelect}
                            appSettings={appSettings}
                            isWbPickerActive={isWbPickerActive}
                            toggleWbPicker={toggleWbPicker}
                            onDragStateChange={setIsSliderDragging}
                          />
                        )}
                        {renderedRightPanel === Panel.Metadata && <MetadataPanel selectedImage={selectedImage} />}
                        {renderedRightPanel === Panel.Crop && (
                          <CropPanel
                            adjustments={adjustments}
                            isStraightenActive={isStraightenActive}
                            isSessionLocked={isEditingLocked}
                            selectedImage={selectedImage}
                            setAdjustments={setAdjustments}
                            setIsStraightenActive={setIsStraightenActive}
                          />
                        )}
                        {renderedRightPanel === Panel.Masks && (
                          <MasksPanel
                            activeMaskContainerId={activeMaskContainerId}
                            activeMaskId={activeMaskId}
                            adjustments={adjustments}
                            appSettings={appSettings}
                            brushSettings={brushSettings}
                            copiedMask={copiedMask}
                            histogram={histogram}
                            onSelectContainer={setActiveMaskContainerId}
                            onSelectMask={setActiveMaskId}
                            selectedImage={selectedImage}
                            setAdjustments={setAdjustments}
                            setBrushSettings={setBrushSettings}
                            setCopiedMask={setCopiedMask}
                            setCustomEscapeHandler={setCustomEscapeHandler}
                            onDragStateChange={setIsSliderDragging}
                            setIsMaskControlHovered={setIsMaskControlHovered}
                          />
                        )}
                        {renderedRightPanel === Panel.Presets && (
                          <PresetsPanel
                            activePanel={activeRightPanel}
                            adjustments={adjustments}
                            isSessionLocked={isEditingLocked}
                            onBlockedAction={reportLockoutBlockedAction}
                            onPresetApplied={handlePresetApplied}
                            selectedPresetId={selectedImagePresetId}
                            selectedImage={selectedImage}
                            setAdjustments={setAdjustments}
                          />
                        )}
                        {renderedRightPanel === Panel.Export && (
                          <ExportPanel
                            adjustments={adjustments}
                            exportState={exportState}
                            multiSelectedPaths={multiSelectedPaths}
                            selectedImage={selectedImage}
                            setExportState={setExportState}
                          />
                        )}
                        {renderedRightPanel === Panel.CameraControls && (
                          <CameraControlsPanel
                            status={cameraStatus}
                            snapshot={cameraStatusSnapshotFresh}
                            isLoading={cameraStatusLoading}
                            isReconnecting={isCameraReconnecting}
                            reconnectResult={cameraReconnectResult}
                            onReconnect={handleCameraReconnect}
                          />
                        )}
                      </motion.div>
                    )}
                  </AnimatePresence>
                </>
              )}
            </div>
          </div>
          <div
            className={clsx(
              'h-full border-l transition-colors relative',
              activeRightPanel && hasSelection ? 'border-surface' : 'border-transparent',
            )}
          >
            <RightPanelSwitcher
              activePanel={activeRightPanel}
              allowedPanels={allowedRightPanels}
              onOpenLibrary={handleBackToLibrary}
              onPanelSelect={handleRightPanelSelect}
            />
            {!activeRightPanel && hasSelection && (
              <div
                className="absolute top-2 right-full mr-2 max-w-[14rem]"
                title={selectedImagePresetName || 'None'}
                aria-label={`Current preset: ${selectedImagePresetName || 'None'}`}
              >
                <div
                  className={clsx(
                    'min-w-0 inline-flex items-center gap-2',
                    'px-2.5 py-1 rounded-xl border bg-surface/50',
                    selectedImagePresetName ? 'border-accent/30' : 'border-border-color/30',
                  )}
                >
                  <span
                    className={clsx('h-2 w-2 rounded-full flex-shrink-0 ring-2 ring-transparent', [
                      selectedImagePresetName ? 'bg-accent ring-accent/50' : 'bg-text-secondary/40',
                    ])}
                    aria-hidden="true"
                  />
                  <span
                    className={clsx(
                      'min-w-0 truncate text-xs font-medium',
                      selectedImagePresetName ? 'text-text-primary' : 'text-text-secondary',
                    )}
                  >
                    {selectedImagePresetName || 'None'}
                  </span>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    );
  };

  const renderMainView = () => {
    const isEditorView = activeView === 'editor' || !!selectedImage;
    if (isEditorView) {
      return renderEditorView();
    }
    return memoizedLibraryView;
  };

  const selectedImagePresetId = adjustments?.lastPresetId || null;
  const selectedImagePresetName = adjustments?.lastPresetName || null;

  const renderContent = () => {
    return renderMainView();
  };

  const storageStatus = storageHealth?.status;
  const shouldShowStorageWarning =
    !!storageStatus &&
    (isCustomerMode
      ? storageStatus === 'warning' || storageStatus === 'critical'
      : storageStatus !== 'healthy');

  const storageHealthMessage = isCustomerMode
    ? storageStatus === 'critical'
      ? STORAGE_CRITICAL_MESSAGE
      : STORAGE_WARNING_MESSAGE
    : storageStatus === 'unknown'
      ? '스토리지 상태를 확인할 수 없습니다.'
      : storageStatus === 'critical'
        ? '저장 공간이 매우 부족합니다.'
        : '저장 공간이 부족해지고 있습니다.';

  const storageHealthDetails =
    !isCustomerMode && storageHealth
      ? (() => {
          const details: string[] = [];
          if (storageHealth.totalBytes > 0) {
            details.push(
              `남은 공간: ${formatBytes(storageHealth.freeBytes)} / 전체: ${formatBytes(storageHealth.totalBytes)}`,
            );
          }
          details.push(`경고 임계값: ${formatBytes(storageHealth.warningThresholdBytes)}`);
          details.push(`위험 임계값: ${formatBytes(storageHealth.criticalThresholdBytes)}`);
          return details;
        })()
      : [];

  const shouldShowTitleBar = !appSettings?.decorations && !isWindowFullScreen;

  return (
    <div
      className={clsx(
        'flex flex-col h-screen bg-bg-primary font-sans text-text-primary overflow-hidden select-none relative',
        (appSettings?.adaptiveEditorTheme || isAnimatingTheme) && 'enable-color-transitions',
      )}
    >
      {shouldShowTitleBar && (
        <TitleBar
          boothyMode={boothyMode}
          boothyHasAdminPassword={boothyHasAdminPassword}
          adminOverrideActive={adminOverrideActive}
          isAdminActionRunning={isAdminActionRunning}
          onAdminToggle={handleAdminToggle}
        />
      )}

      {isSessionLocked && (
        <div className="fixed inset-0 z-[45] flex items-center justify-center bg-black/70 backdrop-blur-sm">
          <div className="text-center bg-surface/80 rounded-2xl px-10 py-8 shadow-2xl">
            <h2 className="text-2xl font-semibold text-text-primary mb-2">Session Locked</h2>
            <p className="text-sm text-text-secondary">Editing is disabled until the next session.</p>
          </div>
        </div>
      )}

      <div
        className={clsx('flex-1 flex flex-col min-h-0', [
          rootPath && 'p-2 gap-2',
          shouldShowTitleBar && 'pt-12',
        ])}
      >
        {error && (
          <div className="absolute top-12 left-1/2 transform -translate-x-1/2 bg-red-600 text-white px-4 py-2 rounded-lg z-50">
            {error}
            <button
              type="button"
              onClick={() => setError(null)}
              className="ml-4 font-bold hover:text-gray-200"
              aria-label="Dismiss error"
            >
              x
            </button>
          </div>
        )}
        {shouldShowStorageWarning && (
          <div className="absolute top-24 left-1/2 transform -translate-x-1/2 bg-amber-500/90 text-amber-50 px-4 py-2 rounded-lg z-50 shadow-lg border border-amber-200/40">
            <div className="text-sm font-semibold">{storageHealthMessage}</div>
            {!isCustomerMode && storageHealthDetails.length > 0 && (
              <div className="mt-1 text-xs text-amber-100/90 space-y-0.5">
                {storageHealthDetails.map((line) => (
                  <div key={line}>{line}</div>
                ))}
              </div>
            )}
            {!isCustomerMode && storageHealth?.diagnostic && (
              <div className="mt-1 text-[11px] text-amber-100/80">{storageHealth.diagnostic}</div>
            )}
          </div>
        )}
        <ExportProgressBar onDismissError={resetBoothyExportProgress} state={boothyExportProgress} />
        <div className="flex flex-row flex-grow h-full min-h-0">
          {memoizedFolderTree}
          <div className="flex-1 flex flex-col min-w-0">{renderContent()}</div>
          {!selectedImage && isLibraryExportPanelVisible && (
            <Resizer
              direction={Orientation.Vertical}
              onMouseDown={createResizeHandler(setRightPanelWidth, rightPanelWidth)}
            />
          )}
          <div
            className={clsx('flex-shrink-0 overflow-hidden', !isResizing && 'transition-all duration-300 ease-in-out')}
            style={{ width: isLibraryExportPanelVisible ? `${rightPanelWidth}px` : '0px' }}
          >
            <LibraryExportPanel
              exportState={exportState}
              imageList={sortedImageList}
              isVisible={isLibraryExportPanelVisible}
              multiSelectedPaths={multiSelectedPaths}
              onClose={() => setIsLibraryExportPanelVisible(false)}
              setExportState={setExportState}
            />
          </div>
        </div>
      </div>
      <SessionWarningModal
        isBlocking={isCustomerMode}
        isOpen={isTMinus5ModalOpen}
        message={tMinus5Message || getBoothyTMinus5WarningMessage(appSettings)}
        onClose={isCustomerMode ? handleTMinus5Confirm : handleTMinus5Dismiss}
      />
      <TimelineLockoutModal
        isOpen={isLockoutModalOpen}
        onContinue={handleAdminContinueWorking}
        onDismiss={handleAdminDismissLockout}
      />
      <TimelineResetModal
        graceSeconds={resetGraceSeconds}
        isExporting={exportState.status === Status.Exporting || importState.status === Status.Importing}
        isOpen={isResetModalOpen}
        onPostpone={handleAdminPostponeReset}
        onResetNow={handleAdminResetNow}
      />
      <ExportDecisionModal isOpen={isExportDecisionModalOpen} onSelect={handleExportDecisionSelect} />
      <EndScreenModal
        isAdmin={boothyMode === 'admin'}
        isOpen={isEndScreenVisible}
        message={getBoothyEndScreenMessage(appSettings)}
        onExit={() => setIsEndScreenVisible(false)}
      />
      <CopyPasteSettingsModal
        isOpen={isCopyPasteSettingsModalOpen}
        onClose={() => setIsCopyPasteSettingsModalOpen(false)}
        settings={appSettings?.copyPasteSettings as CopyPasteSettings}
        onSave={(newSettings) =>
          handleSettingsChange({ ...appSettings, copyPasteSettings: newSettings } as AppSettings)
        }
      />
      <PanoramaModal
        error={panoramaModalState.error}
        finalImageBase64={panoramaModalState.finalImageBase64}
        isOpen={panoramaModalState.isOpen}
        onClose={() =>
          setPanoramaModalState({
            isOpen: false,
            progressMessage: '',
            finalImageBase64: null,
            error: null,
            stitchingSourcePaths: [],
          })
        }
        onOpenFile={(path: string) => {
          handleImageSelect(path);
        }}
        onSave={handleSavePanorama}
        progressMessage={panoramaModalState.progressMessage}
      />
      <DenoiseModal
        isOpen={denoiseModalState.isOpen}
        onClose={() => setDenoiseModalState((prev) => ({ ...prev, isOpen: false }))}
        onDenoise={handleApplyDenoise}
        onSave={handleSaveDenoisedImage}
        onOpenFile={handleImageSelect}
        previewBase64={denoiseModalState.previewBase64}
        originalBase64={denoiseModalState.originalBase64 || null}
        isProcessing={denoiseModalState.isProcessing}
        error={denoiseModalState.error}
        progressMessage={denoiseModalState.progressMessage}
      />
      <AdminModeModal
        errorMessage={adminModalError}
        hasAdminPassword={boothyHasAdminPassword}
        isOpen={isAdminModalOpen}
        isProcessing={isAdminActionRunning}
        onClose={() => setIsAdminModalOpen(false)}
        onSetPassword={handleSetAdminPassword}
        onUnlock={handleUnlockAdmin}
      />
      <CreateFolderModal
        isOpen={isCreateFolderModalOpen}
        onClose={() => setIsCreateFolderModalOpen(false)}
        onSave={handleCreateFolder}
      />
      <RenameFolderModal
        currentName={folderActionTarget ? folderActionTarget.split(/[\\/]/).pop() : ''}
        isOpen={isRenameFolderModalOpen}
        onClose={() => setIsRenameFolderModalOpen(false)}
        onSave={handleRenameFolder}
      />
      <RenameFileModal
        filesToRename={renameTargetPaths}
        isOpen={isRenameFileModalOpen}
        onClose={() => setIsRenameFileModalOpen(false)}
        onSave={handleSaveRename}
      />
      <ConfirmModal {...confirmModalState} onClose={closeConfirmModal} />
      <ImportSettingsModal
        fileCount={importSourcePaths.length}
        isOpen={isImportModalOpen}
        onClose={() => setIsImportModalOpen(false)}
        onSave={handleStartImport}
      />
      <CullingModal
        isOpen={cullingModalState.isOpen}
        onClose={() =>
          setCullingModalState({ isOpen: false, progress: null, suggestions: null, error: null, pathsToCull: [] })
        }
        progress={cullingModalState.progress}
        suggestions={cullingModalState.suggestions}
        error={cullingModalState.error}
        imagePaths={cullingModalState.pathsToCull}
        thumbnails={thumbnails}
        onApply={(action, paths) => {
          if (action === 'reject') {
            handleSetColorLabel('red', paths);
          } else if (action === 'rate_zero') {
            handleRate(1, paths);
          } else if (action === 'delete') {
            executeDelete(paths, { includeAssociated: false });
          }
          setCullingModalState({ isOpen: false, progress: null, suggestions: null, error: null, pathsToCull: [] });
        }}
        onError={(err) => {
          setCullingModalState((prev) => ({ ...prev, error: err, progress: null }));
        }}
      />
      <CollageModal
        isOpen={collageModalState.isOpen}
        onClose={() => setCollageModalState({ isOpen: false, sourceImages: [] })}
        onSave={handleSaveCollage}
        sourceImages={collageModalState.sourceImages}
        thumbnails={thumbnails}
      />
    </div>
  );
}

const AppWrapper = () => (
  <ContextMenuProvider>
    <App />
  </ContextMenuProvider>
);

export default AppWrapper;
