function normalizeAssetPath(assetPath: string) {
  return assetPath.replace(/\\/g, '/').toLowerCase()
}

const allowFixtureAssetPaths = import.meta.env.MODE === 'test'

function hasPathTraversalSegment(assetPath: string) {
  return assetPath.split('/').some((segment) => segment === '..')
}

export function isSessionScopedAssetPath(
  sessionId: string,
  assetPath: string,
) {
  const normalizedAssetPath = normalizeAssetPath(assetPath)
  const normalizedRuntimeSessionMarker =
    `/pictures/dabi_shoot/sessions/${sessionId.toLowerCase()}/`

  if (hasPathTraversalSegment(normalizedAssetPath)) {
    return false
  }

  if (normalizedAssetPath.startsWith('//')) {
    return false
  }

  if (/^[a-z]:\//.test(normalizedAssetPath) || normalizedAssetPath.startsWith('/')) {
    return normalizedAssetPath.includes(normalizedRuntimeSessionMarker)
  }

  return (
    allowFixtureAssetPaths && normalizedAssetPath.startsWith('fixtures/')
  )
}
