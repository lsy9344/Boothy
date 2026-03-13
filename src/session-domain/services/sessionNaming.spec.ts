import { describe, expect, it } from 'vitest'

import { buildSessionName, resolveSameDaySessionName } from './sessionNaming.js'

describe('session naming', () => {
  it('builds the normalized session name from the customer-provided session name', () => {
    expect(buildSessionName('  김보라 오후 세션  ')).toBe('김보라 오후 세션')
  })

  it('adds the next available same-day suffix when the base name already exists', () => {
    expect(resolveSameDaySessionName('김보라 오후 세션', ['김보라 오후 세션'])).toBe('김보라 오후 세션_2')
    expect(resolveSameDaySessionName('김보라 오후 세션', ['김보라 오후 세션', '김보라 오후 세션_2'])).toBe(
      '김보라 오후 세션_3',
    )
  })
})
