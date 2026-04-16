import { useState, type ReactNode } from 'react'

import type { CaptureRuntimeService } from '../../capture-adapter/services/capture-runtime'
import { PresetAuthoringProvider } from '../../preset-authoring/providers/preset-authoring-provider'
import {
  createPresetAuthoringService,
  type PresetAuthoringService,
} from '../../preset-authoring/services/preset-authoring-service'
import type { PresetCatalogService } from '../../preset-catalog/services/preset-catalog-service'
import type { ActivePresetService } from '../../session-domain/services/active-preset'
import { SessionProvider } from '../../session-domain/state/session-provider'
import type { StartSessionService } from '../../session-domain/services/start-session'
import { CapabilityProvider } from './capability-provider'
import { createCapabilityService, type CapabilityService } from '../services/capability-service'

type AppProvidersProps = {
  children: ReactNode
  capabilityService?: CapabilityService
  sessionService?: StartSessionService
  presetCatalogService?: PresetCatalogService
  presetAuthoringService?: PresetAuthoringService
  activePresetService?: ActivePresetService
  captureRuntimeService?: CaptureRuntimeService
}

export function AppProviders({
  children,
  capabilityService,
  sessionService,
  presetCatalogService,
  presetAuthoringService,
  activePresetService,
  captureRuntimeService,
}: AppProvidersProps) {
  const [fallbackCapabilityService] = useState<CapabilityService>(() =>
    createCapabilityService(),
  )
  const [fallbackPresetAuthoringService] = useState<PresetAuthoringService>(() =>
    createPresetAuthoringService(),
  )

  const resolvedCapabilityService =
    capabilityService ?? fallbackCapabilityService
  const resolvedPresetAuthoringService =
    presetAuthoringService ?? fallbackPresetAuthoringService

  return (
    <CapabilityProvider capabilityService={resolvedCapabilityService}>
      <PresetAuthoringProvider presetAuthoringService={resolvedPresetAuthoringService}>
        <SessionProvider
          sessionService={sessionService}
          presetCatalogService={presetCatalogService}
          activePresetService={activePresetService}
          captureRuntimeService={captureRuntimeService}
        >
          {children}
        </SessionProvider>
      </PresetAuthoringProvider>
    </CapabilityProvider>
  )
}
