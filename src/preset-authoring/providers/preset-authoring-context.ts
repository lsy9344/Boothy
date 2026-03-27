import { createContext } from 'react'

import type { PresetAuthoringService } from '../services/preset-authoring-service'

export const PresetAuthoringContext =
  createContext<PresetAuthoringService | null>(null)
