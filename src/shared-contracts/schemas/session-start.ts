import { z } from 'zod'

import { customerNameSchema, phoneLastFourSchema } from './session-manifest'

export const sessionStartInputSchema = z.object({
  name: customerNameSchema,
  phoneLastFour: phoneLastFourSchema,
})

