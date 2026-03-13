import { z } from 'zod'

export const sessionTypeSchema = z.enum(['standard', 'couponExtended'])

export const sessionTimingInitializationSchema = z
  .object({
    reservationStartAt: z.iso.datetime(),
    sessionType: sessionTypeSchema,
    updatedAt: z.iso.datetime(),
  })
  .strict()

export const sessionTimingStateSchema = z
  .object({
    reservationStartAt: z.iso.datetime(),
    actualShootEndAt: z.iso.datetime(),
    sessionType: sessionTypeSchema,
    operatorExtensionCount: z.number().int().nonnegative(),
    lastTimingUpdateAt: z.iso.datetime(),
  })
  .strict()

export type SessionType = z.infer<typeof sessionTypeSchema>
export type SessionTimingInitialization = z.infer<typeof sessionTimingInitializationSchema>
export type SessionTimingState = z.infer<typeof sessionTimingStateSchema>
