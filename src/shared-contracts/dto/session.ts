import type { z } from 'zod'

import {
  sessionManifestSchema,
  sessionStartInputSchema,
  sessionStartResultSchema,
} from '../schemas'

export type SessionManifest = z.infer<typeof sessionManifestSchema>
export type SessionStartInput = z.infer<typeof sessionStartInputSchema>
export type SessionStartResult = z.infer<typeof sessionStartResultSchema>

