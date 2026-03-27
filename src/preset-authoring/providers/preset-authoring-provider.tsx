import type { ReactNode } from 'react'

import type { PresetAuthoringService } from '../services/preset-authoring-service'
import { PresetAuthoringContext } from './preset-authoring-context'

type PresetAuthoringProviderProps = {
  children: ReactNode
  presetAuthoringService: PresetAuthoringService
}

export function PresetAuthoringProvider({
  children,
  presetAuthoringService,
}: PresetAuthoringProviderProps) {
  return (
    <PresetAuthoringContext.Provider value={presetAuthoringService}>
      {children}
    </PresetAuthoringContext.Provider>
  )
}
