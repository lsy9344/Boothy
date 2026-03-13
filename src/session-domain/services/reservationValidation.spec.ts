import { describe, expect, it } from 'vitest'

import {
  validateSessionName,
  validateSessionStartInput,
} from './reservationValidation.js'

describe('session start validation', () => {
  it('trims and accepts a non-empty session name', () => {
    expect(validateSessionName('  김보라 오후 세션  ')).toEqual({
      ok: true,
      value: '김보라 오후 세션',
    })
  })

  it('rejects an empty session name after trimming', () => {
    expect(validateSessionName('   ')).toEqual({
      ok: false,
      errorCode: 'session_name.required',
    })
  })

  it('returns a normalized session-start payload for a valid session name', () => {
    expect(
      validateSessionStartInput({
        sessionName: '  김보라 오후 세션  ',
      }),
    ).toEqual({
      ok: true,
      value: {
        sessionName: '김보라 오후 세션',
      },
    })
  })

  it('returns a typed field error when the session name is blank', () => {
    expect(
      validateSessionStartInput({
        sessionName: '   ',
      }),
    ).toEqual({
      ok: false,
      fieldErrors: {
        sessionName: 'session_name.required',
      },
    })
  })
})
