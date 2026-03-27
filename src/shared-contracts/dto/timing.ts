import type { z } from 'zod'

import { sessionTimingSnapshotSchema } from '../schemas'

export type SessionTimingSnapshot = z.infer<typeof sessionTimingSnapshotSchema>
