import type { ActiveSessionPreset } from '../../shared-contracts/dto/presetCatalog.js'
import type { CaptureConfidenceSnapshot } from '../../shared-contracts/dto/captureConfidence.js'
import type { PostEndOutcome } from '../../shared-contracts/dto/postEndOutcome.js'
import type { SessionTimingState } from '../../shared-contracts/dto/sessionTiming.js'
import type {
  DeleteSessionPhotoResponse,
  SessionGallerySnapshot,
} from '../../shared-contracts/dto/sessionGallery.js'
import type { SessionErrorCode, SessionStartPayload, SessionStartResult } from '../../shared-contracts/dto/session.js'
import { getPresetCatalogEntryById, type PresetId } from '../../shared-contracts/presets/presetCatalog.js'
import type { SessionPresetSelectionResult } from '../../shared-contracts/schemas/presetSchemas.js'

import type { CustomerPreparationState } from './customerPreparationState.js'
import {
  initialSessionTimingAlertState,
  type SessionTimingAlertState,
} from './sessionTimingAlertState.js'

export type SessionPhase =
  | 'start'
  | 'idle'
  | 'validating'
  | 'provisioning'
  | 'preparing'
  | 'preset-selection'
  | 'capture-loading'
  | 'capture-ready'
  | 'post-end'

export type PresetSelectionStatus = 'idle' | 'applying'
export type CaptureRequestStatus = 'idle' | 'requesting'
export type ReviewStatus = 'idle' | 'loading' | 'ready' | 'empty' | 'deleting'
export type PresetSelectionFailure = Extract<SessionPresetSelectionResult, { ok: false }>

export type SessionState = {
  phase: SessionPhase
  pendingSessionName: string | null
  fields: SessionStartPayload
  fieldErrors: Partial<Record<keyof SessionStartPayload, SessionErrorCode>>
  formErrorCode?: SessionErrorCode
  activeSession: SessionStartResult | null
  sessionTiming: SessionTimingState | null
  timingAlert: SessionTimingAlertState
  readiness: CustomerPreparationState | null
  captureConfidence: CaptureConfidenceSnapshot | null
  captureRequestStatus: CaptureRequestStatus
  selectedPresetId: string | null
  presetSelectionFeedback: string | null
  presetSelectionFailure: PresetSelectionFailure | null
  presetSelectionStatus: PresetSelectionStatus
  activePreset: ActiveSessionPreset | null
  pendingActivePresetId: PresetId | null
  postEndOutcome: PostEndOutcome | null
  reviewStatus: ReviewStatus
  reviewGallery: SessionGallerySnapshot | null
  selectedReviewCaptureId: string | null
  isReviewExpanded: boolean
  pendingDeleteCaptureId: string | null
  reviewFeedback: string | null
}

export type SessionAction =
  | { type: 'journey_started'; sessionName?: string }
  | { type: 'field_changed'; field: keyof SessionStartPayload; value: string }
  | { type: 'validation_started' }
  | {
      type: 'validation_failed'
      fieldErrors: Partial<Record<keyof SessionStartPayload, SessionErrorCode>>
    }
  | { type: 'provisioning_started' }
  | { type: 'provisioning_failed'; errorCode: SessionErrorCode }
  | { type: 'provisioning_succeeded'; session: SessionStartResult }
  | { type: 'session_timing_loaded'; sessionId: string; timing: SessionTimingState }
  | { type: 'timing_alert_changed'; timingAlert: SessionTimingAlertState }
  | { type: 'readiness_changed'; readiness: CustomerPreparationState }
  | { type: 'capture_surface_resumed' }
  | { type: 'preset_selection_started'; selectedPresetId: string | null }
  | { type: 'preset_candidate_selected'; selectedPresetId: string }
  | { type: 'preset_selection_applying'; sessionId: string; selectedPresetId: string }
  | {
      type: 'preset_selection_succeeded'
      sessionId: string
      activePreset: ActiveSessionPreset
      selectedPresetId: string
    }
  | {
      type: 'active_preset_changed'
      activePreset: ActiveSessionPreset
    }
  | { type: 'capture_request_started'; captureId: string; requestedAt: string }
  | { type: 'capture_request_finished' }
  | {
      type: 'preset_selection_failed'
      sessionId: string
      failure: PresetSelectionFailure
      feedback: string
    }
  | { type: 'review_gallery_loading' }
  | { type: 'review_gallery_loaded'; gallery: SessionGallerySnapshot }
  | { type: 'review_gallery_failed' }
  | { type: 'review_capture_selected'; captureId: string }
  | { type: 'review_expanded_opened' }
  | { type: 'review_expanded_closed' }
  | { type: 'review_delete_requested'; captureId: string }
  | { type: 'review_delete_cancelled' }
  | { type: 'review_delete_started' }
  | { type: 'review_delete_succeeded'; sessionId: string; response: DeleteSessionPhotoResponse }
  | { type: 'review_delete_failed'; sessionId: string; feedback: string }
  | { type: 'review_feedback_cleared' }
  | { type: 'capture_confidence_updated'; snapshot: CaptureConfidenceSnapshot }
  | { type: 'post_end_started'; outcome: PostEndOutcome }
  | { type: 'active_session_cleared' }

export const initialSessionState: SessionState = {
  phase: 'start',
  pendingSessionName: null,
  fields: {
    sessionName: '',
  },
  fieldErrors: {},
  activeSession: null,
  sessionTiming: null,
  timingAlert: initialSessionTimingAlertState,
  readiness: null,
  captureConfidence: null,
  captureRequestStatus: 'idle',
  selectedPresetId: null,
  presetSelectionFeedback: null,
  presetSelectionFailure: null,
  presetSelectionStatus: 'idle',
  activePreset: null,
  pendingActivePresetId: null,
  postEndOutcome: null,
  reviewStatus: 'idle',
  reviewGallery: null,
  selectedReviewCaptureId: null,
  isReviewExpanded: false,
  pendingDeleteCaptureId: null,
  reviewFeedback: null,
}

function resolvePendingPresetId(presetId: string): PresetId | null {
  return getPresetCatalogEntryById(presetId)?.id ?? null
}

function reconcileSessionTimingFromSnapshot(
  currentTiming: SessionTimingState | null,
  snapshot: CaptureConfidenceSnapshot,
  activeSession: SessionStartResult | null,
): SessionTimingState | null {
  if (currentTiming) {
    if (Date.parse(snapshot.updatedAt) < Date.parse(currentTiming.lastTimingUpdateAt)) {
      return currentTiming
    }

    if (currentTiming.actualShootEndAt === snapshot.shootEndsAt) {
      return currentTiming
    }

    return {
      ...currentTiming,
      actualShootEndAt: snapshot.shootEndsAt,
      lastTimingUpdateAt: snapshot.updatedAt,
    }
  }

  if (!activeSession) {
    return null
  }

  return {
    reservationStartAt: activeSession.createdAt,
    actualShootEndAt: snapshot.shootEndsAt,
    sessionType: 'standard',
    operatorExtensionCount: 0,
    lastTimingUpdateAt: snapshot.updatedAt,
  }
}

function shouldApplyTimingUpdate(currentTiming: SessionTimingState | null, nextTiming: SessionTimingState): boolean {
  if (!currentTiming) {
    return true
  }

  return Date.parse(nextTiming.lastTimingUpdateAt) >= Date.parse(currentTiming.lastTimingUpdateAt)
}

function clearSessionScopedState(): Omit<
  SessionState,
  'phase' | 'pendingSessionName' | 'fields' | 'fieldErrors' | 'formErrorCode'
> {
  return {
    activeSession: null,
    sessionTiming: null,
    timingAlert: initialSessionTimingAlertState,
    readiness: null,
    captureConfidence: null,
    captureRequestStatus: 'idle',
    selectedPresetId: null,
    presetSelectionFeedback: null,
    presetSelectionFailure: null,
    presetSelectionStatus: 'idle',
    activePreset: null,
    pendingActivePresetId: null,
    postEndOutcome: null,
    reviewStatus: 'idle',
    reviewGallery: null,
    selectedReviewCaptureId: null,
    isReviewExpanded: false,
    pendingDeleteCaptureId: null,
    reviewFeedback: null,
  }
}

export function sessionReducer(state: SessionState, action: SessionAction): SessionState {
  switch (action.type) {
    case 'journey_started':
      return {
        ...initialSessionState,
        phase: 'idle',
        pendingSessionName:
          typeof action.sessionName === 'string' ? action.sessionName.trim() || null : null,
        fields: {
          ...initialSessionState.fields,
          sessionName: typeof action.sessionName === 'string' ? action.sessionName.trim() : '',
        },
      }
    case 'field_changed': {
      const fieldErrors = { ...state.fieldErrors }
      delete fieldErrors[action.field]

      return {
        ...state,
        phase:
          state.phase === 'preparing' ||
          state.phase === 'capture-loading' ||
          state.phase === 'capture-ready'
            ? state.phase
            : 'idle',
        fields: {
          ...state.fields,
          [action.field]: action.value,
        },
        fieldErrors,
        formErrorCode: undefined,
        postEndOutcome: state.phase === 'post-end' ? state.postEndOutcome : null,
        timingAlert:
          state.phase === 'capture-loading' || state.phase === 'capture-ready'
            ? state.timingAlert
            : initialSessionTimingAlertState,
        readiness:
          state.phase === 'preparing' ||
          state.phase === 'capture-loading' ||
          state.phase === 'capture-ready' ||
          state.phase === 'post-end'
            ? state.readiness
            : null,
        captureConfidence:
          state.phase === 'capture-loading' || state.phase === 'capture-ready' || state.phase === 'post-end'
            ? state.captureConfidence
            : null,
        captureRequestStatus:
          state.phase === 'capture-loading' || state.phase === 'capture-ready'
            ? state.captureRequestStatus
            : 'idle',
        reviewFeedback:
          state.phase === 'capture-loading' || state.phase === 'capture-ready'
            ? state.reviewFeedback
            : null,
      }
    }
    case 'validation_started':
      return {
        ...state,
        phase: 'validating',
        formErrorCode: undefined,
        ...clearSessionScopedState(),
      }
    case 'validation_failed':
      return {
        ...state,
        phase: 'idle',
        fieldErrors: action.fieldErrors,
        formErrorCode: undefined,
        ...clearSessionScopedState(),
      }
    case 'provisioning_started':
      return {
        ...state,
        phase: 'provisioning',
        fieldErrors: {},
        formErrorCode: undefined,
        ...clearSessionScopedState(),
      }
    case 'provisioning_failed':
      return {
        ...state,
        phase: 'idle',
        pendingSessionName: null,
        formErrorCode: action.errorCode,
        ...clearSessionScopedState(),
      }
    case 'provisioning_succeeded':
      return {
        ...state,
        phase: 'preparing',
        pendingSessionName: null,
        fieldErrors: {},
        formErrorCode: undefined,
        activeSession: action.session,
        sessionTiming: null,
        timingAlert: initialSessionTimingAlertState,
        captureConfidence: null,
        captureRequestStatus: 'idle',
        selectedPresetId: null,
        presetSelectionFeedback: null,
        presetSelectionFailure: null,
        presetSelectionStatus: 'idle',
        activePreset: null,
        pendingActivePresetId: null,
        postEndOutcome: null,
        reviewStatus: 'idle',
        reviewGallery: null,
        selectedReviewCaptureId: null,
        isReviewExpanded: false,
        pendingDeleteCaptureId: null,
        reviewFeedback: null,
      }
    case 'session_timing_loaded':
      if (state.activeSession?.sessionId !== action.sessionId) {
        return state
      }

      if (!shouldApplyTimingUpdate(state.sessionTiming, action.timing)) {
        return state
      }

      return {
        ...state,
        sessionTiming: action.timing,
      }
    case 'timing_alert_changed':
      return {
        ...state,
        timingAlert: action.timingAlert,
      }
    case 'readiness_changed':
      if (
        (
          state.phase === 'preset-selection' ||
          state.phase === 'capture-loading' ||
          state.phase === 'capture-ready'
        ) &&
        action.readiness.kind !== 'ready'
      ) {
        return {
          ...state,
          phase: 'preparing',
          readiness: action.readiness,
          sessionTiming: state.sessionTiming,
          timingAlert: initialSessionTimingAlertState,
          captureConfidence: null,
          captureRequestStatus: 'idle',
          selectedPresetId: state.selectedPresetId,
          presetSelectionFeedback: null,
          presetSelectionFailure: null,
          presetSelectionStatus: 'idle',
          activePreset: state.activePreset,
          pendingActivePresetId: state.pendingActivePresetId,
          reviewStatus: 'idle',
          reviewGallery: null,
          selectedReviewCaptureId: null,
          isReviewExpanded: false,
          pendingDeleteCaptureId: null,
          reviewFeedback: null,
        }
      }

      if (
        state.phase === 'preparing' &&
        action.readiness.kind === 'ready' &&
        state.activePreset !== null
      ) {
        return {
          ...state,
          phase: 'capture-ready',
          readiness: action.readiness,
          captureRequestStatus: 'idle',
          postEndOutcome: null,
        }
      }

      return {
        ...state,
        readiness: action.readiness,
      }
    case 'capture_surface_resumed':
      return {
        ...state,
        phase: 'capture-ready',
        captureRequestStatus: 'idle',
        postEndOutcome: null,
      }
    case 'preset_selection_started':
      return {
        ...state,
        phase: 'preset-selection',
        selectedPresetId: action.selectedPresetId,
        presetSelectionFeedback: null,
        presetSelectionFailure: null,
        presetSelectionStatus: 'idle',
      }
    case 'preset_candidate_selected':
      return {
        ...state,
        selectedPresetId: action.selectedPresetId,
        presetSelectionFeedback: null,
        presetSelectionFailure: null,
        presetSelectionStatus: 'idle',
      }
    case 'preset_selection_applying':
      if (state.activeSession?.sessionId !== action.sessionId) {
        return state
      }

      return {
        ...state,
        selectedPresetId: action.selectedPresetId,
        presetSelectionFeedback: null,
        presetSelectionFailure: null,
        presetSelectionStatus: 'applying',
      }
    case 'preset_selection_succeeded':
      if (state.activeSession?.sessionId !== action.sessionId) {
        return state
      }

      return {
        ...state,
        phase: 'capture-loading',
        selectedPresetId: action.selectedPresetId,
        presetSelectionFeedback: null,
        presetSelectionFailure: null,
        presetSelectionStatus: 'idle',
        activePreset: action.activePreset,
        pendingActivePresetId: resolvePendingPresetId(action.activePreset.presetId),
        timingAlert: initialSessionTimingAlertState,
        captureConfidence: null,
      }
    case 'active_preset_changed':
      return {
        ...state,
        activePreset: action.activePreset,
        pendingActivePresetId: resolvePendingPresetId(action.activePreset.presetId),
        postEndOutcome: null,
      }
    case 'capture_request_started': {
      if (!state.captureConfidence) {
        return {
          ...state,
          captureRequestStatus: 'requesting',
        }
      }

      const preview =
        state.captureConfidence.latestPhoto.kind === 'ready'
          ? state.captureConfidence.latestPhoto.photo
          : state.captureConfidence.latestPhoto.kind === 'updating'
            ? state.captureConfidence.latestPhoto.preview
            : undefined

      return {
        ...state,
        captureRequestStatus: 'requesting',
        captureConfidence: {
          ...state.captureConfidence,
          revision: state.captureConfidence.revision + 1,
          updatedAt: action.requestedAt,
          latestPhoto: {
            kind: 'updating',
            nextCaptureId: action.captureId,
            ...(preview ? { preview } : {}),
          },
        },
      }
    }
    case 'capture_request_finished':
      return {
        ...state,
        captureRequestStatus: 'idle',
      }
    case 'preset_selection_failed':
      if (state.activeSession?.sessionId !== action.sessionId) {
        return state
      }

      return {
        ...state,
        presetSelectionFeedback: action.feedback,
        presetSelectionFailure: action.failure,
        presetSelectionStatus: 'idle',
      }
    case 'review_gallery_loading':
      return {
        ...state,
        reviewStatus: 'loading',
      }
    case 'review_gallery_loaded':
      {
        const preservedSelection = state.selectedReviewCaptureId
          ? action.gallery.items.find((item) => item.captureId === state.selectedReviewCaptureId)?.captureId ?? null
          : null

      return {
        ...state,
        reviewStatus: action.gallery.items.length > 0 ? 'ready' : 'empty',
        reviewGallery: action.gallery,
        selectedReviewCaptureId: preservedSelection ?? action.gallery.selectedCaptureId,
        pendingDeleteCaptureId: null,
      }
      }
    case 'review_gallery_failed':
      return {
        ...state,
        reviewStatus: state.reviewGallery?.items.length ? 'ready' : 'empty',
        pendingDeleteCaptureId: null,
      }
    case 'review_capture_selected':
      return {
        ...state,
        selectedReviewCaptureId: action.captureId,
      }
    case 'review_expanded_opened':
      return {
        ...state,
        isReviewExpanded: true,
      }
    case 'review_expanded_closed':
      return {
        ...state,
        isReviewExpanded: false,
      }
    case 'review_delete_requested':
      return {
        ...state,
        isReviewExpanded: false,
        pendingDeleteCaptureId: action.captureId,
      }
    case 'review_delete_cancelled':
      return {
        ...state,
        pendingDeleteCaptureId: null,
      }
    case 'review_delete_started':
      return {
        ...state,
        reviewStatus: 'deleting',
      }
    case 'review_delete_succeeded':
      if (state.activeSession?.sessionId !== action.sessionId) {
        return state
      }

      return {
        ...state,
        reviewStatus: action.response.gallery.items.length > 0 ? 'ready' : 'empty',
        reviewGallery: action.response.gallery,
        selectedReviewCaptureId: action.response.gallery.selectedCaptureId,
        isReviewExpanded: false,
        pendingDeleteCaptureId: null,
        reviewFeedback: action.response.confirmationMessage,
      }
    case 'review_delete_failed':
      if (state.activeSession?.sessionId !== action.sessionId) {
        return state
      }

      return {
        ...state,
        reviewStatus: state.reviewGallery?.items.length ? 'ready' : 'empty',
        pendingDeleteCaptureId: null,
        reviewFeedback: action.feedback,
      }
    case 'review_feedback_cleared':
      return {
        ...state,
        reviewFeedback: null,
      }
    case 'capture_confidence_updated':
      {
        if (state.phase === 'preset-selection' || state.readiness?.kind === 'phone-required') {
          return {
            ...state,
            sessionTiming: reconcileSessionTimingFromSnapshot(
              state.sessionTiming,
              action.snapshot,
              state.activeSession,
            ),
            captureConfidence: action.snapshot,
          }
        }

        const nextPhase = state.readiness?.kind === 'ready' ? 'capture-ready' : 'preparing'

      if (state.pendingActivePresetId && action.snapshot.activePreset.presetId !== state.pendingActivePresetId) {
        return {
          ...state,
          phase: nextPhase,
          postEndOutcome: null,
          sessionTiming: reconcileSessionTimingFromSnapshot(
            state.sessionTiming,
            action.snapshot,
            state.activeSession,
          ),
          captureRequestStatus: 'idle',
          captureConfidence: action.snapshot,
        }
      }

      {
        const resolvedSnapshotPreset = getPresetCatalogEntryById(action.snapshot.activePreset.presetId)

        if (!resolvedSnapshotPreset) {
          return {
            ...state,
            phase: nextPhase,
            pendingActivePresetId: null,
            postEndOutcome: null,
            captureConfidence: action.snapshot,
          }
        }

        return {
          ...state,
          phase: nextPhase,
          activePreset: {
            presetId: resolvedSnapshotPreset.id,
            displayName: resolvedSnapshotPreset.name,
          },
          pendingActivePresetId: null,
          postEndOutcome: null,
          sessionTiming: reconcileSessionTimingFromSnapshot(state.sessionTiming, action.snapshot, state.activeSession),
          captureRequestStatus: 'idle',
          captureConfidence: action.snapshot,
        }
      }
      }
    case 'post_end_started':
      return {
        ...state,
        phase: 'post-end',
        postEndOutcome: action.outcome,
        captureRequestStatus: 'idle',
        selectedPresetId: null,
        presetSelectionFeedback: null,
        presetSelectionFailure: null,
        presetSelectionStatus: 'idle',
        pendingActivePresetId: null,
        isReviewExpanded: false,
        pendingDeleteCaptureId: null,
        reviewFeedback: null,
      }
    case 'active_session_cleared':
      return {
        ...initialSessionState,
      }
    default:
      return state
  }
}

export function selectCaptureActionEnabled(state: Pick<SessionState, 'readiness'>) {
  return state.readiness?.kind === 'ready'
}
