import { useEffect, useEffectEvent, useState } from 'react'

import { SurfaceLayout } from '../../shared-ui/layout/SurfaceLayout'
import type { HostErrorEnvelope, PublishedPresetSummary } from '../../shared-contracts'
import { useSessionState } from '../../session-domain/state/use-session-state'
import { PresetCard } from '../components/PresetCard'
import { presetSelectCopy } from '../copy/presetSelectCopy'

export function PresetSelectScreen() {
  const {
    isLoadingPresetCatalog,
    isSelectingPreset,
    loadPresetCatalog,
    selectActivePreset,
    sessionDraft,
  } = useSessionState()
  const [errorMessage, setErrorMessage] = useState<string | null>(null)

  const activeSessionId = sessionDraft.sessionId
  const activePreset = sessionDraft.selectedPreset

  const selectedPreset =
    sessionDraft.presetCatalog.find(
      (preset) =>
        preset.presetId === activePreset?.presetId &&
        preset.publishedVersion === activePreset?.publishedVersion,
    ) ?? null

  function shouldSuppressPresetError(error: unknown) {
    const hostError = error as Partial<HostErrorEnvelope>

    return (
      hostError.code === 'session-not-found' || hostError.code === 'preset-not-available'
    )
  }

  async function reloadCatalog() {
    if (activeSessionId === null) {
      return
    }

    setErrorMessage(null)

    try {
      await loadPresetCatalog({ sessionId: activeSessionId })
    } catch (error) {
      if (!shouldSuppressPresetError(error)) {
        setErrorMessage(presetSelectCopy.loadErrorDescription)
      }
    }
  }

  const syncCatalog = useEffectEvent(async () => {
    if (activeSessionId === null || sessionDraft.presetCatalogState !== 'idle') {
      return
    }

    await reloadCatalog()
  })

  useEffect(() => {
    void syncCatalog()
  }, [activeSessionId, sessionDraft.presetCatalogState])

  async function handleSelect(preset: PublishedPresetSummary) {
    if (activeSessionId === null) {
      return
    }

    if (
      activePreset?.presetId === preset.presetId &&
      activePreset.publishedVersion === preset.publishedVersion
    ) {
      return
    }

    setErrorMessage(null)

    try {
      await selectActivePreset({
        sessionId: activeSessionId,
        preset: {
          presetId: preset.presetId,
          publishedVersion: preset.publishedVersion,
        },
      })
    } catch (error) {
      if (!shouldSuppressPresetError(error)) {
        setErrorMessage(presetSelectCopy.saveErrorDescription)
      }
    }
  }

  return (
    <SurfaceLayout
      eyebrow={presetSelectCopy.eyebrow}
      title={presetSelectCopy.title}
      description={presetSelectCopy.description}
    >
      <article className="surface-card">
        <h2>{presetSelectCopy.sessionLabel}</h2>
        <p>{sessionDraft.boothAlias}</p>
        <p>{presetSelectCopy.sessionDescription}</p>
      </article>

      {isLoadingPresetCatalog ? (
        <article className="surface-card">
          <h2>{presetSelectCopy.loadingTitle}</h2>
          <p>{presetSelectCopy.loadingDescription}</p>
        </article>
      ) : null}

      {!isLoadingPresetCatalog &&
      sessionDraft.presetCatalogState === 'empty' ? (
        <article className="surface-card">
          <h2>{presetSelectCopy.emptyTitle}</h2>
          <p>{presetSelectCopy.emptyDescription}</p>
        </article>
      ) : null}

      {sessionDraft.presetCatalog.length > 0 ? (
        <div className="preset-card-grid">
          {sessionDraft.presetCatalog.map((preset) => (
            <PresetCard
              key={`${preset.presetId}:${preset.publishedVersion}`}
              preset={preset}
              isSelected={
                preset.presetId === activePreset?.presetId &&
                preset.publishedVersion === activePreset?.publishedVersion
              }
              disabled={isSelectingPreset}
              onSelect={handleSelect}
            />
          ))}
        </div>
      ) : null}

      {sessionDraft.presetCatalogState === 'error' ? (
        <article className="surface-card">
          <h2>{presetSelectCopy.errorTitle}</h2>
          <p>{errorMessage ?? presetSelectCopy.loadErrorDescription}</p>
          <button
            type="button"
            className="surface-card__action"
            disabled={isLoadingPresetCatalog}
            onClick={() => void reloadCatalog()}
          >
            {presetSelectCopy.retryLabel}
          </button>
        </article>
      ) : null}

      <article className="surface-card">
        <h2>{presetSelectCopy.guidanceTitle}</h2>
        <p>
          {selectedPreset === null
            ? presetSelectCopy.guidanceDescription
            : `${selectedPreset.displayName} ${presetSelectCopy.selectedDescription}`}
        </p>
      </article>

      {errorMessage !== null ? (
        <p className="preset-select-screen__error">{errorMessage}</p>
      ) : null}
    </SurfaceLayout>
  )
}
