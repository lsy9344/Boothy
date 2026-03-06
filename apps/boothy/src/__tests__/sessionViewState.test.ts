import { describe, expect, it } from 'vitest';

import { getLibraryViewIntentOnBackToLibrary, getSessionViewIntentOnApply } from '../sessionViewState';

describe('getSessionViewIntentOnApply', () => {
  it('does not re-open the editor for a duplicate session event', () => {
    expect(
      getSessionViewIntentOnApply({
        sessionKey: 'session-raw-path',
        lastAppliedSessionKey: 'session-raw-path',
      }),
    ).toEqual({
      isDuplicateSession: true,
      shouldActivateEditor: false,
      shouldQueueAutoOpenEditor: false,
    });
  });

  it('auto-opens the editor when a new session is applied', () => {
    expect(
      getSessionViewIntentOnApply({
        sessionKey: 'new-session-raw-path',
        lastAppliedSessionKey: 'previous-session-raw-path',
      }),
    ).toEqual({
      isDuplicateSession: false,
      shouldActivateEditor: true,
      shouldQueueAutoOpenEditor: true,
    });
  });
});

describe('getLibraryViewIntentOnBackToLibrary', () => {
  it('cancels any pending auto-open when the user returns to the library', () => {
    expect(getLibraryViewIntentOnBackToLibrary()).toEqual({
      activeView: 'library',
      shouldQueueAutoOpenEditor: false,
    });
  });
});
