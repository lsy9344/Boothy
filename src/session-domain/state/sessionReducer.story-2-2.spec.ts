import { describe, expect, it } from 'vitest'

import { initialSessionState, sessionReducer } from './sessionReducer.js'

describe('Story 2.2 preset selection state', () => {
  it('enters preset selection without auto-binding a preset for the new session', () => {
    const nextState = sessionReducer(
      {
        ...initialSessionState,
        phase: 'preparing',
        activeSession: {
          sessionId: 'session-22',
          sessionName: '김보라1234',
          sessionFolder: 'C:/sessions/session-22',
          manifestPath: 'C:/sessions/session-22/session.json',
          createdAt: '2026-03-12T09:00:00.000Z',
          preparationState: 'preparing',
        },
      },
      {
        type: 'preset_selection_started',
        selectedPresetId: null,
      },
    )

    expect(nextState.phase).toBe('preset-selection')
    expect(nextState.selectedPresetId).toBeNull()
    expect(nextState.activePreset).toBeNull()
    expect(nextState.presetSelectionStatus).toBe('idle')
  })

  it('stores a candidate preset without advancing to capture-loading', () => {
    const nextState = sessionReducer(
      {
        ...initialSessionState,
        phase: 'preset-selection',
      },
      {
        type: 'preset_candidate_selected',
        selectedPresetId: 'background-pink',
      },
    )

    expect(nextState.phase).toBe('preset-selection')
    expect(nextState.selectedPresetId).toBe('background-pink')
    expect(nextState.activePreset).toBeNull()
    expect(nextState.presetSelectionStatus).toBe('idle')
  })

  it('advances only after the confirmed preset binding succeeds', () => {
    const nextState = sessionReducer(
      {
        ...initialSessionState,
        phase: 'preset-selection',
        activeSession: {
          sessionId: 'session-22',
          sessionName: '김보라1234',
          sessionFolder: 'C:/sessions/session-22',
          manifestPath: 'C:/sessions/session-22/session.json',
          createdAt: '2026-03-12T09:00:00.000Z',
          preparationState: 'preparing',
        },
        selectedPresetId: 'background-pink',
      },
      {
        type: 'preset_selection_succeeded',
        sessionId: 'session-22',
        activePreset: {
          presetId: 'background-pink',
          displayName: '배경지 - 핑크',
        },
        selectedPresetId: 'background-pink',
      },
    )

    expect(nextState.phase).toBe('capture-loading')
    expect(nextState.selectedPresetId).toBe('background-pink')
    expect(nextState.activePreset).toEqual({
      presetId: 'background-pink',
      displayName: '배경지 - 핑크',
    })
  })

  it('keeps the customer on preset selection with retry guidance when confirmation fails', () => {
    const nextState = sessionReducer(
      {
        ...initialSessionState,
        phase: 'preset-selection',
        activeSession: {
          sessionId: 'session-22',
          sessionName: '김보라1234',
          sessionFolder: 'C:/sessions/session-22',
          manifestPath: 'C:/sessions/session-22/session.json',
          createdAt: '2026-03-12T09:00:00.000Z',
          preparationState: 'preparing',
        },
        selectedPresetId: 'background-pink',
      },
      {
        type: 'preset_selection_failed',
        sessionId: 'session-22',
        failure: {
          ok: false,
          errorCode: 'session.preset_selection_failed',
          message: 'Unknown presetId',
        },
        feedback: '프리셋을 적용하지 못했어요. 다시 선택해 주세요.',
      },
    )

    expect(nextState.phase).toBe('preset-selection')
    expect(nextState.selectedPresetId).toBe('background-pink')
    expect(nextState.presetSelectionStatus).toBe('idle')
    expect(nextState.presetSelectionFeedback).toBe('프리셋을 적용하지 못했어요. 다시 선택해 주세요.')
    expect(nextState.presetSelectionFailure).toEqual({
      ok: false,
      errorCode: 'session.preset_selection_failed',
      message: 'Unknown presetId',
    })
  })

  it('ignores stale delete success actions from a cleared or replaced session', () => {
    const nextState = sessionReducer(
      {
        ...initialSessionState,
        phase: 'capture-ready',
        activeSession: {
          sessionId: 'session-23',
          sessionName: '김보라1235',
          sessionFolder: 'C:/sessions/session-23',
          manifestPath: 'C:/sessions/session-23/session.json',
          createdAt: '2026-03-12T09:00:00.000Z',
          preparationState: 'preparing',
        },
        reviewStatus: 'deleting',
      },
      {
        type: 'review_delete_succeeded',
        sessionId: 'session-22',
        response: {
          schemaVersion: 'boothy.camera.contract.v1',
          deletedCaptureId: 'capture-001',
          confirmationMessage: '사진이 삭제되었습니다.',
          gallery: {
            schemaVersion: 'boothy.camera.contract.v1',
            sessionId: 'session-22',
            sessionName: '김보라1234',
            shootEndsAt: '2026-03-12T09:50:00.000Z',
            activePresetName: '배경지 - 핑크',
            latestCaptureId: null,
            selectedCaptureId: null,
            items: [],
          },
        },
      },
    )

    expect(nextState.activeSession?.sessionId).toBe('session-23')
    expect(nextState.reviewStatus).toBe('deleting')
    expect(nextState.reviewGallery).toBeNull()
    expect(nextState.reviewFeedback).toBeNull()
  })
})
