export type SessionViewIntent = {
  isDuplicateSession: boolean;
  shouldActivateEditor: boolean;
  shouldQueueAutoOpenEditor: boolean;
};

export type LibraryViewIntent = {
  activeView: 'library';
  shouldQueueAutoOpenEditor: boolean;
};

type SessionViewIntentParams = {
  sessionKey: string | null;
  lastAppliedSessionKey: string | null;
};

export function getSessionViewIntentOnApply(params: SessionViewIntentParams): SessionViewIntent {
  const normalizedSessionKey =
    typeof params.sessionKey === 'string' && params.sessionKey.trim().length > 0 ? params.sessionKey : null;
  const isDuplicateSession =
    normalizedSessionKey !== null && normalizedSessionKey === params.lastAppliedSessionKey;

  return {
    isDuplicateSession,
    shouldActivateEditor: !isDuplicateSession,
    shouldQueueAutoOpenEditor: !isDuplicateSession,
  };
}

export function getLibraryViewIntentOnBackToLibrary(): LibraryViewIntent {
  return {
    activeView: 'library',
    shouldQueueAutoOpenEditor: false,
  };
}
