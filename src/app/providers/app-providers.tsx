import { useRef, type ReactNode } from 'react'

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
  const capabilityServiceRef = useRef<CapabilityService | null>(null)
  const presetAuthoringServiceRef = useRef<PresetAuthoringService | null>(null)

  if (capabilityServiceRef.current === null) {
    capabilityServiceRef.current = capabilityService ?? createCapabilityService()
  }

  if (presetAuthoringServiceRef.current === null) {
    presetAuthoringServiceRef.current =
      presetAuthoringService ?? createPresetAuthoringService()
  }

  const resolvedCapabilityService = capabilityService ?? capabilityServiceRef.current
  const resolvedPresetAuthoringService =
    presetAuthoringService ?? presetAuthoringServiceRef.current

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
