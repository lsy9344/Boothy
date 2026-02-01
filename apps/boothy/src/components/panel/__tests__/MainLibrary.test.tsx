import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import MainLibrary from '../MainLibrary';
import {
  LibraryViewMode,
  RawStatus,
  SortDirection,
  ThumbnailAspectRatio,
  ThumbnailSize,
  Theme,
} from '../../ui/AppProperties';
import { Status } from '../right/ExportImportProperties';

vi.mock('@tauri-apps/api/app', () => ({
  getVersion: vi.fn().mockResolvedValue('0.0.0-test'),
}));

const noop = () => {};

describe('MainLibrary', () => {
  afterEach(() => cleanup());

  const baseProps = {
    activePath: null,
    appSettings: { lastRootPath: 'C:\\boothy', theme: Theme.Dark },
    boothySessionName: 'Session-01',
    sessionRemainingSeconds: 1800,
    currentFolderPath: 'C:\\boothy',
    filterCriteria: { colors: [], rating: 0, rawStatus: RawStatus.All },
    imageList: [],
    imageRatings: {},
    importState: { status: Status.Idle, errorMessage: '' },
    isCustomerMode: true,
    isLoading: false,
    libraryScrollTop: 0,
    libraryViewMode: LibraryViewMode.Flat,
    multiSelectedPaths: [],
    onClearSelection: noop,
    onContextMenu: noop,
    onContinueSession: noop,
    onEmptyAreaContextMenu: noop,
    onGoEditor: noop,
    onExitToHome: noop,
    onImageClick: noop,
    onImageDoubleClick: noop,
    onLibraryRefresh: noop,
    onOpenFolder: noop,
    onSettingsChange: noop,
    onThumbnailAspectRatioChange: noop,
    onThumbnailSizeChange: noop,
    rootPath: 'C:\\boothy',
    searchCriteria: { tags: [], text: '', mode: 'AND' as const },
    setFilterCriteria: noop,
    setLibraryScrollTop: noop,
    setLibraryViewMode: noop,
    setSearchCriteria: noop,
    setSortCriteria: noop,
    sortCriteria: { key: 'name', order: SortDirection.Ascending },
    theme: Theme.Dark,
    thumbnailAspectRatio: ThumbnailAspectRatio.Cover,
    thumbnails: {},
    thumbnailSize: ThumbnailSize.Medium,
  };

  it('renders the session countdown in the library header', async () => {
    render(
      <MainLibrary
        {...baseProps}
      />,
    );

    await waitFor(() => {
      expect(screen.getByRole('timer')).toBeInTheDocument();
    });
  });

  it('shows green lamp when camera is ready', () => {
    render(
      <MainLibrary
        {...baseProps}
        cameraStatusReport={{
          ipcState: 'connected',
          protocolVersion: '1.0.0',
          lastError: null,
          requestId: null,
          correlationId: null,
          status: { connected: true, cameraDetected: true, cameraModel: 'EOS', sessionDestination: null },
        }}
      />,
    );

    expect(screen.getByTestId('camera-lamp-dot').className).toContain('bg-green-400');
  });

  it('shows yellow lamp while reconnecting', () => {
    render(
      <MainLibrary
        {...baseProps}
        cameraStatusReport={{
          ipcState: 'reconnecting',
          protocolVersion: '1.0.0',
          lastError: null,
          requestId: null,
          correlationId: null,
          status: { connected: true, cameraDetected: true, cameraModel: 'EOS', sessionDestination: null },
        }}
      />,
    );

    expect(screen.getByTestId('camera-lamp-dot').className).toContain('bg-yellow-400');
  });

  it('shows red lamp when camera is unavailable', () => {
    render(
      <MainLibrary
        {...baseProps}
        isCameraUnavailable={true}
        cameraStatusReport={{
          ipcState: 'reconnecting',
          protocolVersion: '1.0.0',
          lastError: 'Sidecar start failed',
          requestId: null,
          correlationId: null,
          status: null,
        }}
      />,
    );

    expect(screen.getByTestId('camera-lamp-dot').className).toContain('bg-red-500');
  });
});
