function normalizeSessionName(sessionName: string): string {
  return sessionName.trim().replace(/\s+/g, ' ')
}

export function buildSessionName(sessionName: string): string {
  return normalizeSessionName(sessionName)
}

export function resolveSameDaySessionName(sessionName: string, existingSessionNames: Iterable<string>): string {
  const baseSessionName = buildSessionName(sessionName)
  const usedSessionNames = new Set(existingSessionNames)

  if (!usedSessionNames.has(baseSessionName)) {
    return baseSessionName
  }

  let suffix = 2
  while (usedSessionNames.has(`${baseSessionName}_${suffix}`)) {
    suffix += 1
  }

  return `${baseSessionName}_${suffix}`
}
