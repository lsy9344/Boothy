import { useEffect, useEffectEvent, useState } from 'react'

import { SurfaceLayout } from '../../shared-ui/layout/SurfaceLayout'
import type { HostErrorEnvelope, PublishedPresetSummary } from '../../shared-contracts'
import { useSessionState } from '../../session-domain/state/use-session-state'
import { PresetCard } from '../components/PresetCard'
import { getPresetSelectCopy } from '../copy/presetSelectCopy'

export function PresetSelectScreen() {
  const {
    cancelPresetSwitch,
    isLoadingPresetCatalog,
    isSelectingPreset,
    loadPresetCatalog,
    selectActivePreset,
    sessionDraft,
  } = useSessionState()
  const [errorMessage, setErrorMessage] = useState<string | null>(null)

  const activeSessionId = sessionDraft.sessionId
  const activePreset = sessionDraft.selectedPreset
  const copy = getPresetSelectCopy(sessionDraft.presetSelectionMode)

  const selectedPreset =
    sessionDraft.presetCatalog.find(
      (preset) =>
        preset.presetId === activePreset?.presetId &&
        preset.publishedVersion === activePreset?.publishedVersion,
    ) ?? null
  const selectedPresetName =
    selectedPreset?.displayName ?? sessionDraft.manifest?.activePresetDisplayName ?? null

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
        setErrorMessage(copy.loadErrorDescription)
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
      const hostError = error as Partial<HostErrorEnvelope>

      if (hostError.code === 'preset-not-available') {
        await reloadCatalog()
        setErrorMessage(copy.unavailableDescription)
        return
      }

      if (!shouldSuppressPresetError(error)) {
        setErrorMessage(copy.saveErrorDescription)
      }
    }
  }

  return (
    <SurfaceLayout
      eyebrow={copy.eyebrow}
      title={copy.title}
      description={copy.description}
    >
      <article className="surface-card">
        <h2>{copy.sessionLabel}</h2>
        <p>{sessionDraft.boothAlias}</p>
        <p>{copy.sessionDescription}</p>
      </article>

      {isLoadingPresetCatalog ? (
        <article className="surface-card">
          <h2>{copy.loadingTitle}</h2>
          <p>{copy.loadingDescription}</p>
        </article>
      ) : null}

      {!isLoadingPresetCatalog &&
      sessionDraft.presetCatalogState === 'empty' ? (
        <article className="surface-card">
          <h2>{copy.emptyTitle}</h2>
          <p>{copy.emptyDescription}</p>
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
              saveLabel={copy.saveLabel}
              selectedLabel={copy.selectedLabel}
              onSelect={handleSelect}
            />
          ))}
        </div>
      ) : null}

      {sessionDraft.presetCatalogState === 'error' ? (
        <article className="surface-card">
          <h2>{copy.errorTitle}</h2>
          <p>{errorMessage ?? copy.loadErrorDescription}</p>
          <button
            type="button"
            className="surface-card__action"
            disabled={isLoadingPresetCatalog}
            onClick={() => void reloadCatalog()}
          >
            {copy.retryLabel}
          </button>
        </article>
      ) : null}

      <article className="surface-card">
        <h2>{copy.guidanceTitle}</h2>
        <p>
          {selectedPresetName === null
            ? copy.guidanceDescription
            : `${selectedPresetName} ${copy.selectedDescription}`}
        </p>
        {copy.cancelDescription !== null ? <p>{copy.cancelDescription}</p> : null}
        {copy.cancelLabel !== null ? (
          <button
            type="button"
            className="surface-card__action"
            disabled={isSelectingPreset}
            onClick={cancelPresetSwitch}
          >
            {copy.cancelLabel}
          </button>
        ) : null}
      </article>

      {errorMessage !== null ? (
        <p className="preset-select-screen__error">{errorMessage}</p>
      ) : null}
    </SurfaceLayout>
  )
}
