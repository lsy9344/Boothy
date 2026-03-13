import backgroundIvoryPreviewUrl from '../../customer-flow/assets/preset-previews/background-ivory.svg'
import backgroundPinkPreviewUrl from '../../customer-flow/assets/preset-previews/background-pink.svg'
import coolTonePreviewUrl from '../../customer-flow/assets/preset-previews/cool-tone.svg'
import warmTonePreviewUrl from '../../customer-flow/assets/preset-previews/warm-tone.svg'
import type { CatalogFallbackReason } from '../../shared-contracts/logging/operationalEvents.js'
import {
  presetCatalog as approvedPresetCatalog,
  presetCatalogSchema as approvedPresetCatalogSchema,
  type PresetCatalogEntry,
} from '../../shared-contracts/presets/presetCatalog.js'
import { presetCatalogSchema, type PresetCatalog } from '../../shared-contracts/schemas/presetSchemas.js'
import { loadApprovedPresetCatalogCandidate } from './presetCatalogCandidateStore.js'

type PreviewAssetDetails = {
  group: 'background' | 'tone'
  previewAssetPath?: string
  previewAssetUrl?: string
}

export type PresetCatalogAuditReason = CatalogFallbackReason

export type ReadyPresetCatalogLoadState = {
  status: 'ready'
  presets: PresetCatalog
  source: 'approved' | 'approved-fallback' | 'verified-candidate'
  auditReason?: PresetCatalogAuditReason
}

export type PresetCatalogLoadState =
  | { status: 'loading' }
  | ReadyPresetCatalogLoadState
  | { status: 'empty' }
  | { status: 'unavailable'; auditReason: PresetCatalogAuditReason }

export type PresetCatalogService = {
  loadApprovedPresetCatalog(): Promise<PresetCatalogLoadState>
}

export type PresetCatalogServiceOptions = {
  loadCatalogSource?: () => Promise<unknown | undefined>
}

const previewAssetMap: Record<PresetCatalogEntry['id'], PreviewAssetDetails> = {
  'background-ivory': {
    group: 'background',
    previewAssetPath: '/src/customer-flow/assets/preset-previews/background-ivory.svg',
    previewAssetUrl: backgroundIvoryPreviewUrl,
  },
  'background-pink': {
    group: 'background',
    previewAssetPath: '/src/customer-flow/assets/preset-previews/background-pink.svg',
    previewAssetUrl: backgroundPinkPreviewUrl,
  },
  'cool-tone': {
    group: 'tone',
    previewAssetPath: '/src/customer-flow/assets/preset-previews/cool-tone.svg',
    previewAssetUrl: coolTonePreviewUrl,
  },
  'warm-tone': {
    group: 'tone',
    previewAssetPath: '/src/customer-flow/assets/preset-previews/warm-tone.svg',
    previewAssetUrl: warmTonePreviewUrl,
  },
}

function mapApprovedCatalogForBoothDisplay(catalog: readonly PresetCatalogEntry[]): PresetCatalog {
  return presetCatalogSchema.parse(
    catalog.map((preset) => ({
      id: preset.id,
      name: preset.name,
      ...previewAssetMap[preset.id],
    })),
  )
}

export const approvedBoothPresetCatalog = mapApprovedCatalogForBoothDisplay(approvedPresetCatalog)

function createApprovedCatalogState(): ReadyPresetCatalogLoadState {
  return {
    status: 'ready',
    presets: approvedBoothPresetCatalog,
    source: 'approved',
  }
}

function createApprovedFallbackCatalogState(
  auditReason: PresetCatalogAuditReason,
): ReadyPresetCatalogLoadState {
  return {
    status: 'ready',
    presets: approvedBoothPresetCatalog,
    source: 'approved-fallback',
    auditReason,
  }
}

function isMissingCatalogInput(value: unknown): boolean {
  return value == null
}

function resolveFallbackReason(candidate: unknown): PresetCatalogAuditReason {
  if (isMissingCatalogInput(candidate)) {
    return 'missing_catalog_input'
  }

  if (!Array.isArray(candidate)) {
    return 'invalid_catalog_shape'
  }

  if (candidate.length === 0) {
    return 'missing_catalog_input'
  }

  if (candidate.length > 6) {
    return 'oversized_catalog'
  }

  for (const [index, entry] of candidate.entries()) {
    if (!entry || typeof entry !== 'object') {
      return 'invalid_catalog_shape'
    }

    const candidateId = 'id' in entry ? entry.id : undefined
    const candidateName = 'name' in entry ? entry.name : undefined
    const approvedEntry = approvedPresetCatalog[index]

    if (typeof candidateId !== 'string') {
      return 'invalid_catalog_shape'
    }

    if (!approvedPresetCatalog.some((preset) => preset.id === candidateId)) {
      return 'invalid_id'
    }

    if (approvedEntry && candidateId !== approvedEntry.id) {
      return 'reordered_catalog'
    }

    if (typeof candidateName === 'string' && approvedEntry && candidateName !== approvedEntry.name) {
      return 'name_mismatch'
    }
  }

  return 'invalid_catalog_shape'
}

function resolveReadyCatalogState(
  candidate: unknown,
): PresetCatalogLoadState {
  if (isMissingCatalogInput(candidate)) {
    return createApprovedFallbackCatalogState('missing_catalog_input')
  }

  const parsedCandidate = approvedPresetCatalogSchema.safeParse(candidate)

  if (!parsedCandidate.success) {
    return createApprovedFallbackCatalogState(resolveFallbackReason(candidate))
  }

  const mappedCandidate = presetCatalogSchema.safeParse(
    parsedCandidate.data.map((preset) => ({
      id: preset.id,
      name: preset.name,
      ...previewAssetMap[preset.id],
    })),
  )

  if (!mappedCandidate.success) {
    return createApprovedFallbackCatalogState(resolveFallbackReason(parsedCandidate.data))
  }

  return {
    status: 'ready',
    presets: mappedCandidate.data,
    source: 'verified-candidate',
  }
}

export function createPresetCatalogService(
  options: PresetCatalogServiceOptions = {},
): PresetCatalogService {
  const loadCatalogSource = options.loadCatalogSource

  return {
    async loadApprovedPresetCatalog() {
      if (!loadCatalogSource) {
        return createApprovedCatalogState()
      }

      try {
        const loadedCatalog = await loadCatalogSource()
        return resolveReadyCatalogState(loadedCatalog)
      } catch {
        return createApprovedFallbackCatalogState('missing_catalog_input')
      }
    },
  }
}

export function createRuntimePresetCatalogService(
  loadCatalogSource: NonNullable<PresetCatalogServiceOptions['loadCatalogSource']> = loadApprovedPresetCatalogCandidate,
): PresetCatalogService {
  return createPresetCatalogService({
    loadCatalogSource,
  })
}

export const presetCatalogService = createRuntimePresetCatalogService()
