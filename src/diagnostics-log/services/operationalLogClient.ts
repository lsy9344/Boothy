import { invoke } from '@tauri-apps/api/core'
import { z } from 'zod'

import {
  lifecycleEventWriteSchema,
  operatorInterventionWriteSchema,
  type LifecycleEventWrite,
  type OperatorInterventionWrite,
} from '../../shared-contracts/logging/operationalEvents.js'

export const operationalLogErrorSchema = z
  .object({
    code: z.string().trim().min(1),
    message: z.string().trim().min(1),
    severity: z.enum(['error']),
    retryable: z.boolean(),
    surface: z.enum(['silent']),
  })
  .strict()

export type OperationalLogError = z.infer<typeof operationalLogErrorSchema>

export type OperationalLogInvoke = <T>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T>

type OperationalLogClientOptions = {
  invoke?: OperationalLogInvoke
}

const fallbackOperationalLogError: OperationalLogError = {
  code: 'diagnostics.unknownFailure',
  message: 'Unknown operational log failure',
  severity: 'error',
  retryable: false,
  surface: 'silent',
}

export function normalizeOperationalLogError(error: unknown): OperationalLogError {
  const result = operationalLogErrorSchema.safeParse(error)
  if (result.success) {
    return result.data
  }

  if (error instanceof Error && error.message.trim().length > 0) {
    return {
      ...fallbackOperationalLogError,
      message: error.message,
    }
  }

  return fallbackOperationalLogError
}

export async function recordLifecycleEvent(
  event: LifecycleEventWrite,
  options: OperationalLogClientOptions = {},
): Promise<void> {
  const payload = lifecycleEventWriteSchema.parse(event)

  try {
    await (options.invoke ?? invoke)('record_lifecycle_event', { event: payload })
  } catch (error) {
    throw normalizeOperationalLogError(error)
  }
}

export async function recordOperatorIntervention(
  intervention: OperatorInterventionWrite,
  options: OperationalLogClientOptions = {},
): Promise<void> {
  const payload = operatorInterventionWriteSchema.parse(intervention)

  try {
    await (options.invoke ?? invoke)('record_operator_intervention', { intervention: payload })
  } catch (error) {
    throw normalizeOperationalLogError(error)
  }
}
