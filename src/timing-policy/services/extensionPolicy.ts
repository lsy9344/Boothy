import { z } from 'zod'

import { sessionTimingStateSchema, type SessionTimingState } from '../../shared-contracts/dto/sessionTiming.js'
import { shiftIsoUtcMinutes } from './shootEndCalculator.js'

const operatorExtensionInputSchema = z
  .object({
    updatedAt: z.iso.datetime(),
  })
  .strict()

type OperatorExtensionInput = z.infer<typeof operatorExtensionInputSchema>

export function applyOperatorSessionExtension(
  state: SessionTimingState,
  input: OperatorExtensionInput,
): SessionTimingState {
  const parsedState = sessionTimingStateSchema.parse(state)
  const parsedInput = operatorExtensionInputSchema.parse(input)

  return sessionTimingStateSchema.parse({
    ...parsedState,
    actualShootEndAt: shiftIsoUtcMinutes(parsedState.actualShootEndAt, 60),
    operatorExtensionCount: parsedState.operatorExtensionCount + 1,
    lastTimingUpdateAt: parsedInput.updatedAt,
  })
}
