import { z } from 'zod'

export const branchConfigDefaults = {
  branchId: 'branch-unconfigured',
  branchPhoneNumber: '',
  operationalToggles: {
    enablePhoneEscalation: false,
  },
} as const

export const branchConfigSchema = z
  .object({
    branchId: z.string().trim().min(1).default(branchConfigDefaults.branchId),
    branchPhoneNumber: z.string().trim().default(branchConfigDefaults.branchPhoneNumber),
    operationalToggles: z
      .object({
        enablePhoneEscalation: z.boolean().default(branchConfigDefaults.operationalToggles.enablePhoneEscalation),
      })
      .strip()
      .default(branchConfigDefaults.operationalToggles),
  })
  .strip()

export type BranchConfig = z.infer<typeof branchConfigSchema>
