import { useContext, useRef } from 'react'

import { PresetAuthoringContext } from './preset-authoring-context'
import { createPresetAuthoringService } from '../services/preset-authoring-service'

export function usePresetAuthoringService() {
  const presetAuthoringService = useContext(PresetAuthoringContext)
  const fallbackServiceRef = useRef<ReturnType<typeof createPresetAuthoringService> | null>(null)

  if (fallbackServiceRef.current === null) {
    fallbackServiceRef.current = createPresetAuthoringService()
  }

  return presetAuthoringService ?? fallbackServiceRef.current
}
