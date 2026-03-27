import { useContext } from 'react'

import { PresetAuthoringContext } from './preset-authoring-context'
import { createPresetAuthoringService } from '../services/preset-authoring-service'

export function usePresetAuthoringService() {
  return useContext(PresetAuthoringContext) ?? createPresetAuthoringService()
}
