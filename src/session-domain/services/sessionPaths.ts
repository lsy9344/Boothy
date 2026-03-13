type ResolveSessionPathsInput = {
  sessionRootBase: string
  sessionId: string
}

export type SessionPaths = {
  eventsPath: string
  exportStatusPath: string
  sessionDir: string
  manifestPath: string
  processedDir: string
}

function joinPath(...parts: string[]) {
  return parts
    .filter(Boolean)
    .map((part, index) => {
      if (index === 0) {
        return part.replace(/[\\/]+$/, '')
      }

      return part.replace(/^[\\/]+|[\\/]+$/g, '')
    })
    .join('/')
}

export function resolveSessionPaths({ sessionRootBase, sessionId }: ResolveSessionPathsInput): SessionPaths {
  const sessionDir = joinPath(sessionRootBase, sessionId)

  return {
    sessionDir,
    manifestPath: joinPath(sessionDir, 'session.json'),
    eventsPath: joinPath(sessionDir, 'events.ndjson'),
    exportStatusPath: joinPath(sessionDir, 'export-status.json'),
    processedDir: joinPath(sessionDir, 'processed'),
  }
}
