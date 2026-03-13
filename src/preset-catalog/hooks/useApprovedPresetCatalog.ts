import { useEffect, useState } from 'react'

import {
  presetCatalogService as defaultPresetCatalogService,
  type PresetCatalogLoadState,
  type PresetCatalogService,
} from '../services/presetCatalogService.js'

export function useApprovedPresetCatalog(
  presetCatalogService: PresetCatalogService = defaultPresetCatalogService,
) {
  const [catalogState, setCatalogState] = useState<PresetCatalogLoadState>({
    status: 'loading',
  })

  useEffect(() => {
    let isActive = true

    void presetCatalogService.loadApprovedPresetCatalog().then((result) => {
      if (!isActive) {
        return
      }

      setCatalogState(result)
    })

    return () => {
      isActive = false
    }
  }, [presetCatalogService])

  return catalogState
}
