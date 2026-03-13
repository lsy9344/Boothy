import { z } from 'zod'

const nonEmptyStringSchema = z.string().trim().min(1)

export const postEndOutcomeKindSchema = z.enum(['export-waiting', 'completed', 'handoff'])
export const postEndGuidanceModeSchema = z.enum(['standard', 'wait-or-call'])

export const postEndOutcomeSchema = z
  .object({
    sessionId: nonEmptyStringSchema,
    actualShootEndAt: z.iso.datetime(),
    outcomeKind: postEndOutcomeKindSchema,
    guidanceMode: postEndGuidanceModeSchema,
    sessionName: nonEmptyStringSchema.nullable(),
    showSessionName: z.boolean(),
    handoffTargetLabel: nonEmptyStringSchema.nullable(),
  })
  .strict()

export type PostEndOutcome = z.infer<typeof postEndOutcomeSchema>
export type PostEndOutcomeKind = z.infer<typeof postEndOutcomeKindSchema>
export type PostEndGuidanceMode = z.infer<typeof postEndGuidanceModeSchema>
