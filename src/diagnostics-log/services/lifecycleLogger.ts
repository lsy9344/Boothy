import { invoke } from '@tauri-apps/api/core'
import type { PresetCatalogAuditReason } from '../../preset-catalog/services/presetCatalogService.js'
import type { PostEndOutcomeKind } from '../../shared-contracts/dto/postEndOutcome.js'

import {
  recordLifecycleEvent,
  type OperationalLogInvoke,
} from './operationalLogClient.js'

import { resolveOperationalBranchId } from './operationalLogContext.js'

type ReadinessReachedInput = {
  branchId: string
  sessionId: string
  sessionName: string
}

type PostEndLifecycleInput = ReadinessReachedInput & {
  actualShootEndAt: string
}

type ExportStateChangedInput = PostEndLifecycleInput & {
  outcomeKind: PostEndOutcomeKind
}

type PresetCatalogFallbackInput = {
  branchId: string
  reason: PresetCatalogAuditReason
  sessionId?: string
  sessionName?: string
}

export type LifecycleLogger = {
  recordActualShootEnd?(input: PostEndLifecycleInput): Promise<void>
  recordExportStateChanged?(input: ExportStateChangedInput): Promise<void>
  recordPhoneRequired?(input: PostEndLifecycleInput): Promise<void>
  recordReadinessReached(input: ReadinessReachedInput): Promise<void>
  recordPresetCatalogFallback?(input: PresetCatalogFallbackInput): Promise<void>
  recordSessionCompleted?(input: PostEndLifecycleInput): Promise<void>
  recordWarningShown?(input: PostEndLifecycleInput): Promise<void>
}

export function createLifecycleLogger(invokeClient: OperationalLogInvoke = invoke): LifecycleLogger {
  return {
    async recordWarningShown(input) {
      await recordLifecycleEvent(
        {
          payloadVersion: 1,
          eventType: 'warning_shown',
          occurredAt: new Date().toISOString(),
          branchId: resolveOperationalBranchId(input.branchId),
          sessionId: input.sessionId,
          sessionName: input.sessionName,
          currentStage: 'captureActive',
          actualShootEndAt: input.actualShootEndAt,
        },
        { invoke: invokeClient },
      )
    },
    async recordActualShootEnd(input) {
      await recordLifecycleEvent(
        {
          payloadVersion: 1,
          eventType: 'actual_shoot_end',
          occurredAt: new Date().toISOString(),
          branchId: resolveOperationalBranchId(input.branchId),
          sessionId: input.sessionId,
          sessionName: input.sessionName,
          currentStage: 'captureActive',
          actualShootEndAt: input.actualShootEndAt,
        },
        { invoke: invokeClient },
      )
    },
    async recordExportStateChanged(input) {
      await recordLifecycleEvent(
        {
          payloadVersion: 1,
          eventType: 'export_state_changed',
          occurredAt: new Date().toISOString(),
          branchId: resolveOperationalBranchId(input.branchId),
          sessionId: input.sessionId,
          sessionName: input.sessionName,
          currentStage: `postEnd:${input.outcomeKind}`,
          actualShootEndAt: input.actualShootEndAt,
        },
        { invoke: invokeClient },
      )
    },
    async recordPhoneRequired(input) {
      await recordLifecycleEvent(
        {
          payloadVersion: 1,
          eventType: 'phone_required',
          occurredAt: new Date().toISOString(),
          branchId: resolveOperationalBranchId(input.branchId),
          sessionId: input.sessionId,
          sessionName: input.sessionName,
          currentStage: 'postEnd:phoneRequired',
          actualShootEndAt: input.actualShootEndAt,
        },
        { invoke: invokeClient },
      )
    },
    async recordReadinessReached(input) {
      await recordLifecycleEvent(
        {
          payloadVersion: 1,
          eventType: 'readiness_reached',
          occurredAt: new Date().toISOString(),
          branchId: resolveOperationalBranchId(input.branchId),
          sessionId: input.sessionId,
          sessionName: input.sessionName,
          currentStage: 'cameraReady',
        },
        { invoke: invokeClient },
      )
    },
    async recordSessionCompleted(input) {
      await recordLifecycleEvent(
        {
          payloadVersion: 1,
          eventType: 'session_completed',
          occurredAt: new Date().toISOString(),
          branchId: resolveOperationalBranchId(input.branchId),
          sessionId: input.sessionId,
          sessionName: input.sessionName,
          currentStage: 'postEnd:completed',
          actualShootEndAt: input.actualShootEndAt,
        },
        { invoke: invokeClient },
      )
    },
    async recordPresetCatalogFallback(input) {
      await recordLifecycleEvent(
        {
          payloadVersion: 1,
          eventType: 'preset_catalog_fallback',
          occurredAt: new Date().toISOString(),
          branchId: resolveOperationalBranchId(input.branchId),
          sessionId: input.sessionId,
          sessionName: input.sessionName,
          currentStage: 'presetSelection',
          catalogFallbackReason: input.reason,
        },
        { invoke: invokeClient },
      )
    },
  }
}
