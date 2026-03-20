import { z } from 'zod'

export const sessionIdSchema = z
  .string()
  .regex(/^session_[a-z0-9]{26}$/i, '유효한 세션 식별자가 아니에요.')
