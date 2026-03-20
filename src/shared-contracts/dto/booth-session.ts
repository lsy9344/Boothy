import { z } from 'zod'

export const boothSessionStubSchema = z.object({
  sessionId: z.string().min(1),
  boothAlias: z.string().min(1),
  presetId: z.string().min(1),
})

export type BoothSessionStub = z.infer<typeof boothSessionStubSchema>
