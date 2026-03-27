import type { z } from 'zod'

import {
  sessionPostEndCompletionVariantSchema,
  sessionManifestSchema,
  sessionPostEndSchema,
  sessionPostEndStateSchema,
  sessionStartInputSchema,
  sessionStartResultSchema,
} from '../schemas'

export type SessionManifest = z.infer<typeof sessionManifestSchema>
export type SessionPostEndRecord = z.infer<typeof sessionPostEndSchema>
export type SessionPostEndState = z.infer<typeof sessionPostEndStateSchema>
export type SessionPostEndCompletionVariant = z.infer<
  typeof sessionPostEndCompletionVariantSchema
>
export type ExportWaitingPostEndRecord = Extract<
  SessionPostEndRecord,
  { state: 'export-waiting' }
>
export type CompletedPostEndRecord = Extract<
  SessionPostEndRecord,
  { state: 'completed' }
>
export type PhoneRequiredPostEndRecord = Extract<
  SessionPostEndRecord,
  { state: 'phone-required' }
>
export type SessionStartInput = z.infer<typeof sessionStartInputSchema>
export type SessionStartResult = z.infer<typeof sessionStartResultSchema>
