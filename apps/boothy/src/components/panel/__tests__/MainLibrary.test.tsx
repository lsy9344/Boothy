import { render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
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
  it('renders the session countdown in the library header', async () => {
    render(
      <MainLibrary
        activePath={null}
        appSettings={{ lastRootPath: 'C:\\boothy', theme: Theme.Dark }}
        boothySessionName="Session-01"
        sessionRemainingSeconds={1800}
        currentFolderPath="C:\\boothy"
        filterCriteria={{ colors: [], rating: 0, rawStatus: RawStatus.All }}
        imageList={[]}
        imageRatings={{}}
        importState={{ status: Status.Idle, errorMessage: '' }}
        isCustomerMode={true}
        isLoading={false}
        libraryScrollTop={0}
        libraryViewMode={LibraryViewMode.Flat}
        multiSelectedPaths={[]}
        onClearSelection={noop}
        onContextMenu={noop}
        onContinueSession={noop}
        onEmptyAreaContextMenu={noop}
        onGoHome={noop}
        onImageClick={noop}
        onImageDoubleClick={noop}
        onLibraryRefresh={noop}
        onOpenFolder={noop}
        onSettingsChange={noop}
        onThumbnailAspectRatioChange={noop}
        onThumbnailSizeChange={noop}
        rootPath="C:\\boothy"
        searchCriteria={{ tags: [], text: '', mode: 'AND' }}
        setFilterCriteria={noop}
        setLibraryScrollTop={noop}
        setLibraryViewMode={noop}
        setSearchCriteria={noop}
        setSortCriteria={noop}
        sortCriteria={{ key: 'name', order: SortDirection.Ascending }}
        theme={Theme.Dark}
        thumbnailAspectRatio={ThumbnailAspectRatio.Cover}
        thumbnails={{}}
        thumbnailSize={ThumbnailSize.Medium}
      />,
    );

    await waitFor(() => {
      expect(screen.getByRole('timer')).toBeInTheDocument();
    });
  });
});
