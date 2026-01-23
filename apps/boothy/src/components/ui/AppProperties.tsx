import { Adjustments, CopyPasteSettings } from '../../utils/adjustments';
import { ToolType } from '../panel/right/Masks';

export const GLOBAL_KEYS = [' ', 'ArrowUp', 'ArrowDown', 'f', 'b', 'w'];
export const OPTION_SEPARATOR = 'separator';

export enum Invokes {
  AddTagForPaths = 'add_tag_for_paths',
  ApplyAdjustments = 'apply_adjustments',
  ApplyAdjustmentsToPaths = 'apply_adjustments_to_paths',
  ApplyAutoAdjustmentsToPaths = 'apply_auto_adjustments_to_paths',
  ApplyDenoising = 'apply_denoising',
  BatchExportImages = 'batch_export_images',
  CalculateAutoAdjustments = 'calculate_auto_adjustments',
  CancelExport = 'cancel_export',
  ClearAllSidecars = 'clear_all_sidecars',
  ClearAllTags = 'clear_all_tags',
  ClearThumbnailCache = 'clear_thumbnail_cache',
  CopyFiles = 'copy_files',
  CreateFolder = 'create_folder',
  CreateVirtualCopy = 'create_virtual_copy',
  CullImages = 'cull_images',
  DeleteFolder = 'delete_folder',
  DuplicateFile = 'duplicate_file',
  EstimateBatchExportSize = 'estimate_batch_export_size',
  EstimateExportSize = 'estimate_export_size',
  ExportImage = 'export_image',
  GenerateFullscreenPreview = 'generate_fullscreen_preview',
  GeneratePreviewForPath = 'generate_preview_for_path',
  GenerateHistogram = 'generate_histogram',
  GenerateMaskOverlay = 'generate_mask_overlay',
  GeneratePresetPreview = 'generate_preset_preview',
  GenerateThumbnailsProgressive = 'generate_thumbnails_progressive',
  GenerateUncroppedPreview = 'generate_uncropped_preview',
  GenerateWaveform = 'image_processing::generate_waveform',
  GetFolderTree = 'get_folder_tree',
  GetLogFilePath = 'get_log_file_path',
  GetPinnedFolderTrees = 'get_pinned_folder_trees',
  GetSupportedFileTypes = 'get_supported_file_types',
  HandleExportPresetsToFile = 'handle_export_presets_to_file',
  HandleImportPresetsFromFile = 'handle_import_presets_from_file',
  HandleImportLegacyPresetsFromFile = 'handle_import_legacy_presets_from_file',
  ImportFiles = 'import_files',
  ListImagesInDir = 'list_images_in_dir',
  ListImagesRecursive = 'list_images_recursive',
  LoadImage = 'load_image',
  LoadMetadata = 'load_metadata',
  LoadPresets = 'load_presets',
  LoadSettings = 'load_settings',
  MoveFiles = 'move_files',
  ReadExifForPaths = 'read_exif_for_paths',
  RemoveTagForPaths = 'remove_tag_for_paths',
  RenameFiles = 'rename_files',
  RenameFolder = 'rename_folder',
  ResetAdjustmentsForPaths = 'reset_adjustments_for_paths',
  SaveMetadataAndUpdateThumbnail = 'save_metadata_and_update_thumbnail',
  SaveCollage = 'save_collage',
  SaveDenoisedImage = 'save_denoised_image',
  SavePanorama = 'save_panorama',
  SavePresets = 'save_presets',
  SaveSettings = 'save_settings',
  BoothyCreateOrOpenSession = 'boothy_create_or_open_session',
  BoothyGetStorageDiagnostics = 'boothy_get_storage_diagnostics',
  BoothyOpenSessionsRootInExplorer = 'boothy_open_sessions_root_in_explorer',
  BoothyGetActiveSession = 'boothy_get_active_session',
  BoothyGetModeState = 'boothy_get_mode_state',
  BoothySetAdminPassword = 'boothy_set_admin_password',
  BoothyAuthenticateAdmin = 'boothy_authenticate_admin',
  BoothySwitchToCustomerMode = 'boothy_switch_to_customer_mode',
  BoothyHandlePhotoTransferred = 'boothy_handle_photo_transferred',
  BoothyHandleExportDecision = 'boothy_handle_export_decision',
  BoothyGetExportedCount = 'boothy_get_exported_count',
  BoothySetCurrentPreset = 'boothy_set_current_preset',
  BoothyLogFrontend = 'boothy_log_frontend',
  SetColorLabelForPaths = 'set_color_label_for_paths',
  ShowInFinder = 'show_in_finder',
  StartFolderWatcher = 'start_folder_watcher',
  StitchPanorama = 'stitch_panorama',
  UpdateWindowEffect = 'update_window_effect',
  SaveTempFile = 'save_temp_file',
}

export enum Panel {
  Adjustments = 'adjustments',
  CameraControls = 'cameraControls',
  Crop = 'crop',
  Export = 'export',
  Masks = 'masks',
  Metadata = 'metadata',
  Presets = 'presets',
}

export enum RawStatus {
  All = 'all',
  NonRawOnly = 'nonRawOnly',
  RawOnly = 'rawOnly',
  RawOverNonRaw = 'rawOverNonRaw',
}

export enum SortDirection {
  Ascending = 'asc',
  Descening = 'desc',
}

export enum Theme {
  Arctic = 'arctic',
  Blue = 'blue',
  Dark = 'dark',
  Grey = 'grey',
  Light = 'light',
  MutedGreen = 'muted-green',
  Sepia = 'sepia',
  Snow = 'snow',
}

export enum ThumbnailAspectRatio {
  Cover = 'cover',
  Contain = 'contain',
}

export interface AppSettings {
  adaptiveEditorTheme?: Theme;
  copyPasteSettings?: CopyPasteSettings;
  decorations?: any;
  editorPreviewResolution?: number;
  enableZoomHifi?: boolean;
  enableLivePreviews?: boolean;
  enableHighQualityLivePreviews?: boolean;
  enableExifReading?: boolean;
  filterCriteria?: FilterCriteria;
  lastFolderState?: any;
  pinnedFolders?: any;
  lastRootPath: string | null;
  sortCriteria?: SortCriteria;
  taggingShortcuts?: string[];
  theme: Theme;
  thumbnailSize?: ThumbnailSize;
  thumbnailAspectRatio?: ThumbnailAspectRatio;
  uiVisibility?: UiVisibility;
  adjustmentVisibility?: { [key: string]: boolean };
  activeTreeSection?: string | null;
  rawHighlightCompression?: number;
  processingBackend?: string;
  linuxGpuOptimization?: boolean;
  boothy_end_screen_message?: string;
  boothy_t_minus_5_warning_message?: string;
  boothy_reset_grace_period_seconds?: number;
  boothy_storage_health_enabled?: boolean;
  boothy_storage_warning_threshold_bytes?: number;
  boothy_storage_critical_threshold_bytes?: number;
  boothy_storage_poll_interval_seconds?: number;
}

export interface BrushSettings {
  feather: number;
  size: number;
  tool: ToolType;
}

export enum LibraryViewMode {
  Flat = 'flat',
  Recursive = 'recursive',
}

export interface FilterCriteria {
  colors: Array<string>;
  rating: number;
  rawStatus: RawStatus;
}

export interface Folder {
  children: any;
  id?: string | undefined;
  name?: string | undefined;
}

export interface ImageFile {
  is_edited: boolean;
  modified: number;
  path: string;
  tags: Array<string> | null;
  exif: { [key: string]: string } | null;
  is_virtual_copy: boolean;
}

export interface Option {
  color?: string;
  disabled?: boolean;
  icon?: any;
  isDestructive?: boolean;
  label?: string;
  onClick?(): void;
  submenu?: any;
  type?: string;
}

export enum Orientation {
  Horizontal = 'horizontal',
  Vertical = 'vertical',
}

export interface Preset {
  adjustments: Partial<Adjustments>;
  folder?: Folder;
  id: string;
  name: string;
}

export interface Progress {
  completed?: number;
  current?: number;
  total: number;
}

export interface SelectedImage {
  exif: any;
  height: number;
  isRaw: boolean;
  isReady: boolean;
  metadata?: any;
  original_base64?: string;
  originalUrl: string | null;
  path: string;
  thumbnailUrl: string;
  width: number;
}

export interface SortCriteria {
  key: string;
  label?: string;
  order: string;
}

export interface SupportedTypes {
  nonRaw: Array<string>;
  raw: Array<string>;
}

export enum ThumbnailSize {
  Large = 'large',
  Medium = 'medium',
  Small = 'small',
}

export interface TransformState {
  positionX: number;
  positionY: number;
  scale: number;
}

export interface UiVisibility {
  folderTree: boolean;
  filmstrip: boolean;
}

export interface WaveformData {
  [index: string]: Array<number> | number;
  blue: Array<number>;
  green: Array<number>;
  height: number;
  luma: Array<number>;
  red: Array<number>;
  width: number;
}

export interface CullingSettings {
  similarityThreshold: number;
  blurThreshold: number;
  groupSimilar: boolean;
  filterBlurry: boolean;
}

export interface ImageAnalysisResult {
  path: string;
  qualityScore: number;
  sharpnessMetric: number;
  centerFocusMetric: number;
  exposureMetric: number;
  width: number;
  height: number;
}

export interface CullGroup {
  representative: ImageAnalysisResult;
  duplicates: ImageAnalysisResult[];
}

export interface CullingSuggestions {
  similarGroups: CullGroup[];
  blurryImages: ImageAnalysisResult[];
  failedPaths: string[];
}
