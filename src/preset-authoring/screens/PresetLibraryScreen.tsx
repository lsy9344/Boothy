import { startTransition, useEffect, useMemo, useRef, useState } from 'react'

import type {
  AuthoringWorkspaceResult,
  DraftPresetEditPayload,
  DraftPresetSummary,
  DraftValidationFinding,
  DraftValidationReport,
  HostErrorEnvelope,
} from '../../shared-contracts'
import { SurfaceLayout } from '../../shared-ui/layout/SurfaceLayout'
import { usePresetAuthoringService } from '../providers/use-preset-authoring-service'

type ScreenState =
  | {
      tone: 'idle'
      message: string
    }
  | {
      tone:
        | 'loading'
        | 'saving'
        | 'validating'
        | 'success'
        | 'error'
      message: string
    }

type EditorMode = 'create' | 'edit'
type PublicationFormState = {
  publishedVersion: string
  actorId: string
  actorLabel: string
  reviewNote: string
}

const EMPTY_DRAFT_FORM: DraftPresetEditPayload = {
  presetId: 'preset_new-draft',
  displayName: '',
  lifecycleState: 'draft',
  darktableVersion: '5.4.1',
  xmpTemplatePath: 'xmp/template.xmp',
  previewProfile: {
    profileId: 'preview-standard',
    displayName: 'Preview Standard',
    outputColorSpace: 'sRGB',
  },
  finalProfile: {
    profileId: 'final-standard',
    displayName: 'Final Standard',
    outputColorSpace: 'sRGB',
  },
  noisePolicy: {
    policyId: 'balanced-noise',
    displayName: 'Balanced Noise',
    reductionMode: 'balanced',
  },
  preview: {
    assetPath: 'previews/cover.jpg',
    altText: '',
  },
  sampleCut: {
    assetPath: 'samples/sample-cut.jpg',
    altText: '',
  },
  description: '',
  notes: '',
}

function buildSuggestedPublishedVersion(date = new Date()) {
  const year = date.getFullYear()
  const month = `${date.getMonth() + 1}`.padStart(2, '0')
  const day = `${date.getDate()}`.padStart(2, '0')

  return `${year}.${month}.${day}`
}

function createPublicationFormState(): PublicationFormState {
  return {
    publishedVersion: buildSuggestedPublishedVersion(),
    actorId: '',
    actorLabel: '',
    reviewNote: '',
  }
}

function mapDraftToForm(draft: DraftPresetSummary): DraftPresetEditPayload {
  return {
    presetId: draft.presetId,
    displayName: draft.displayName,
    lifecycleState: 'draft',
    darktableVersion: draft.darktableVersion,
    xmpTemplatePath: draft.xmpTemplatePath,
    previewProfile: draft.previewProfile,
    finalProfile: draft.finalProfile,
    noisePolicy: draft.noisePolicy,
    preview: draft.preview,
    sampleCut: draft.sampleCut,
    description: draft.description ?? '',
    notes: draft.notes ?? '',
  }
}

function isMutableAuthoringLifecycle(
  lifecycleState: DraftPresetSummary['lifecycleState'],
) {
  return lifecycleState === 'draft' || lifecycleState === 'validated'
}

function isValidPublishedVersion(value: string) {
  return /^\d{4}\.\d{2}\.\d{2}$/.test(value.trim())
}

function isSameDraftPayload(
  left: DraftPresetEditPayload,
  right: DraftPresetEditPayload,
) {
  return JSON.stringify(left) === JSON.stringify(right)
}

function findDraftById(
  workspace: AuthoringWorkspaceResult | null,
  presetId: string | null,
) {
  if (!workspace || !presetId) {
    return null
  }

  return workspace.drafts.find((draft) => draft.presetId === presetId) ?? null
}

function upsertDraftInWorkspace(
  workspace: AuthoringWorkspaceResult | null,
  draft: DraftPresetSummary,
): AuthoringWorkspaceResult {
  const currentDrafts = workspace?.drafts ?? []
  const nextDrafts = [...currentDrafts]
  const draftIndex = nextDrafts.findIndex((entry) => entry.presetId === draft.presetId)

  if (draftIndex >= 0) {
    nextDrafts[draftIndex] = draft
  } else {
    nextDrafts.unshift(draft)
  }

  nextDrafts.sort((left, right) => {
    if (left.updatedAt !== right.updatedAt) {
      return right.updatedAt.localeCompare(left.updatedAt)
    }

    if (left.displayName !== right.displayName) {
      return left.displayName.localeCompare(right.displayName)
    }

    return left.presetId.localeCompare(right.presetId)
  })

  return {
    schemaVersion: workspace?.schemaVersion ?? 'preset-authoring-workspace/v1',
    supportedLifecycleStates:
      workspace?.supportedLifecycleStates ?? ['draft', 'validated', 'approved', 'published'],
    drafts: nextDrafts,
    invalidDrafts: workspace?.invalidDrafts ?? [],
  }
}

function getLifecycleRank(lifecycleState: DraftPresetSummary['lifecycleState']) {
  switch (lifecycleState) {
    case 'published':
      return 3
    case 'approved':
      return 2
    case 'validated':
      return 1
    default:
      return 0
  }
}

function isDraftAtLeastAsFresh(
  candidate: DraftPresetSummary,
  reference: DraftPresetSummary,
) {
  if (candidate.presetId !== reference.presetId) {
    return false
  }

  if (candidate.draftVersion !== reference.draftVersion) {
    return candidate.draftVersion > reference.draftVersion
  }

  if (candidate.updatedAt !== reference.updatedAt) {
    return candidate.updatedAt >= reference.updatedAt
  }

  const candidateValidationCheckedAt =
    candidate.validation.latestReport?.checkedAt ?? ''
  const referenceValidationCheckedAt =
    reference.validation.latestReport?.checkedAt ?? ''

  if (candidateValidationCheckedAt !== referenceValidationCheckedAt) {
    return candidateValidationCheckedAt >= referenceValidationCheckedAt
  }

  const candidateLifecycleRank = getLifecycleRank(candidate.lifecycleState)
  const referenceLifecycleRank = getLifecycleRank(reference.lifecycleState)

  if (candidateLifecycleRank !== referenceLifecycleRank) {
    return candidateLifecycleRank > referenceLifecycleRank
  }

  return false
}

function mergeRefreshedWorkspace(
  refreshed: AuthoringWorkspaceResult,
  optimisticDraft: DraftPresetSummary,
) {
  const refreshedDraft =
    refreshed.drafts.find((draft) => draft.presetId === optimisticDraft.presetId) ?? null

  if (refreshedDraft && isDraftAtLeastAsFresh(refreshedDraft, optimisticDraft)) {
    return refreshed
  }

  return upsertDraftInWorkspace(refreshed, optimisticDraft)
}

function normalizeHostMessage(error: unknown) {
  const hostError = error as HostErrorEnvelope | undefined

  return (
    hostError?.message ??
    '지금은 draft 작업을 완료하지 못했어요. 잠시 후 다시 시도해 주세요.'
  )
}

function isCapabilityDenied(error: unknown) {
  const hostError = error as HostErrorEnvelope | undefined

  return hostError?.code === 'capability-denied'
}

function formatLifecycleLabel(draft: DraftPresetSummary) {
  switch (draft.lifecycleState) {
    case 'validated':
      return 'approval 준비 완료'
    case 'published':
      return '게시 완료'
    case 'approved':
      return '승인 완료'
    default:
      return 'draft'
  }
}

function formatValidationStatus(report: DraftValidationReport | null) {
  if (!report) {
    return '검증 전'
  }

  return report.status === 'passed' ? '통과' : '수정 필요'
}

function formatFindingTone(finding: DraftValidationFinding) {
  return finding.severity === 'error' ? '조치 필요' : '참고'
}

type PublicationAuditRecord = DraftPresetSummary['publicationHistory'][number]

function findLatestPublicationRecord(
  draft: DraftPresetSummary | null,
): PublicationAuditRecord | null {
  if (!draft || draft.publicationHistory.length === 0) {
    return null
  }

  return draft.publicationHistory[draft.publicationHistory.length - 1] ?? null
}

function hasBlockingFollowUpReview(draft: DraftPresetSummary | null) {
  const latestRecord = findLatestPublicationRecord(draft)
  const latestValidationCheckedAt = draft?.validation.latestReport?.checkedAt ?? ''

  if (!latestRecord || latestRecord.action !== 'rejected') {
    return false
  }

  if (latestValidationCheckedAt.length === 0) {
    return true
  }

  return latestRecord.notedAt >= latestValidationCheckedAt
}

function formatPublicationActionLabel(action: PublicationAuditRecord['action']) {
  switch (action) {
    case 'approved':
      return '승인 완료'
    case 'published':
      return '게시 완료'
    default:
      return action
  }
}

export function PresetLibraryScreen() {
  const presetAuthoringService = usePresetAuthoringService()
  const loadRequestVersionRef = useRef(0)
  const [workspace, setWorkspace] = useState<AuthoringWorkspaceResult | null>(null)
  const [mode, setMode] = useState<EditorMode>('create')
  const [selectedDraftId, setSelectedDraftId] = useState<string | null>(null)
  const [draftForm, setDraftForm] = useState<DraftPresetEditPayload>(EMPTY_DRAFT_FORM)
  const [publicationForm, setPublicationForm] = useState<PublicationFormState>(
    () => createPublicationFormState(),
  )
  const [hostAccessDenied, setHostAccessDenied] = useState(false)
  const [screenState, setScreenState] = useState<ScreenState>({
    tone: 'loading',
    message: 'draft 작업공간을 준비하고 있어요.',
  })

  useEffect(() => {
    let active = true

    async function loadWorkspace() {
      const requestVersion = loadRequestVersionRef.current + 1
      loadRequestVersionRef.current = requestVersion
      setScreenState({
        tone: 'loading',
        message: 'draft 작업공간을 불러오고 있어요.',
      })

      try {
        const workspaceResult = await presetAuthoringService.loadAuthoringWorkspace()

        if (!active || loadRequestVersionRef.current !== requestVersion) {
          return
        }

        setHostAccessDenied(false)
        setWorkspace(workspaceResult)

        const firstDraft = workspaceResult.drafts[0] ?? null

        if (firstDraft) {
          setMode('edit')
          setSelectedDraftId(firstDraft.presetId)
          setDraftForm(mapDraftToForm(firstDraft))
          setPublicationForm(createPublicationFormState())
          setScreenState({
            tone: 'idle',
            message: '저장된 draft를 이어서 검증하거나 수정할 수 있어요.',
          })
          return
        }

        setMode('create')
        setSelectedDraftId(null)
        setDraftForm(EMPTY_DRAFT_FORM)
        setPublicationForm(createPublicationFormState())
        setScreenState({
          tone: 'idle',
          message: '새 draft를 만들면 booth catalog와 현재 세션은 바뀌지 않아요.',
        })
      } catch (error) {
        if (!active) {
          return
        }

        const denied = isCapabilityDenied(error)
        setHostAccessDenied(denied)
        if (denied) {
          setWorkspace(null)
          setSelectedDraftId(null)
          setPublicationForm(createPublicationFormState())
        }
        setScreenState({
          tone: 'error',
          message: normalizeHostMessage(error),
        })
      }
    }

    void loadWorkspace()

    return () => {
      active = false
    }
  }, [presetAuthoringService])

  const selectedDraft = useMemo(
    () => findDraftById(workspace, selectedDraftId),
    [selectedDraftId, workspace],
  )
  const hasUnsavedChanges =
    mode === 'edit' &&
    selectedDraft !== null &&
    !isSameDraftPayload(draftForm, mapDraftToForm(selectedDraft))
  const hasPendingDraftChanges =
    (mode === 'create' && !isSameDraftPayload(draftForm, EMPTY_DRAFT_FORM)) ||
    hasUnsavedChanges
  const latestValidation = selectedDraft?.validation.latestReport ?? null
  const followUpHistory =
    selectedDraft?.publicationHistory.filter((record) => record.action !== 'rejected') ?? []
  const blocksNextStepUntilRevalidated = hasBlockingFollowUpReview(selectedDraft)
  const canEditSelectedDraft =
    selectedDraft !== null && isMutableAuthoringLifecycle(selectedDraft.lifecycleState)
  const canEditDraftForm = mode === 'create' || canEditSelectedDraft
  const canRunValidation = mode === 'edit' && canEditSelectedDraft
  const canPublishValidatedDraft =
    selectedDraft !== null &&
    selectedDraft.lifecycleState === 'validated' &&
    latestValidation?.status === 'passed' &&
    !hasUnsavedChanges &&
    !blocksNextStepUntilRevalidated
  const hasPublicationMetadata =
    isValidPublishedVersion(publicationForm.publishedVersion) &&
    publicationForm.actorId.trim() !== '' &&
    publicationForm.actorLabel.trim() !== ''
  const isBusy =
    screenState.tone === 'loading' ||
    screenState.tone === 'saving' ||
    screenState.tone === 'validating'

  function updateForm<K extends keyof DraftPresetEditPayload>(
    key: K,
    value: DraftPresetEditPayload[K],
  ) {
    setDraftForm((current) => ({
      ...current,
      [key]: value,
    }))
  }

  function updatePreview(
    key: keyof DraftPresetEditPayload['preview'],
    value: string,
  ) {
    setDraftForm((current) => ({
      ...current,
      preview: {
        ...current.preview,
        [key]: value,
      },
    }))
  }

  function updateSampleCut(
    key: keyof DraftPresetEditPayload['sampleCut'],
    value: string,
  ) {
    setDraftForm((current) => ({
      ...current,
      sampleCut: {
        ...current.sampleCut,
        [key]: value,
      },
    }))
  }

  function updatePreviewProfile(
    key: keyof DraftPresetEditPayload['previewProfile'],
    value: string,
  ) {
    setDraftForm((current) => ({
      ...current,
      previewProfile: {
        ...current.previewProfile,
        [key]: value,
      },
    }))
  }

  function updateFinalProfile(
    key: keyof DraftPresetEditPayload['finalProfile'],
    value: string,
  ) {
    setDraftForm((current) => ({
      ...current,
      finalProfile: {
        ...current.finalProfile,
        [key]: value,
      },
    }))
  }

  function updateNoisePolicy(
    key: keyof DraftPresetEditPayload['noisePolicy'],
    value: string,
  ) {
    setDraftForm((current) => ({
      ...current,
      noisePolicy: {
        ...current.noisePolicy,
        [key]: value,
      },
    }))
  }

  function updatePublicationForm<K extends keyof PublicationFormState>(
    key: K,
    value: PublicationFormState[K],
  ) {
    setPublicationForm((current) => ({
      ...current,
      [key]: value,
    }))
  }

  function handleCreateDraft() {
    if (isBusy) {
      return
    }

    if (hasPendingDraftChanges) {
      setScreenState({
        tone: 'error',
        message:
          '저장되지 않은 변경이 있어요. 먼저 draft를 저장하거나 변경을 되돌린 뒤 화면을 전환해 주세요.',
      })
      return
    }

    startTransition(() => {
      setHostAccessDenied(false)
      setMode('create')
      setSelectedDraftId(null)
      setDraftForm(EMPTY_DRAFT_FORM)
      setPublicationForm(createPublicationFormState())
      setScreenState({
        tone: 'idle',
        message: '새 draft baseline을 작성 중이에요. 저장 전까지 booth catalog는 그대로 유지돼요.',
      })
    })
  }

  function handleSelectDraft(draft: DraftPresetSummary) {
    if (isBusy) {
      return
    }

    if (hasPendingDraftChanges) {
      setScreenState({
        tone: 'error',
        message:
          '저장되지 않은 변경이 있어요. 먼저 draft를 저장하거나 변경을 되돌린 뒤 다른 draft를 열어 주세요.',
      })
      return
    }

    startTransition(() => {
      setMode('edit')
      setSelectedDraftId(draft.presetId)
      setDraftForm(mapDraftToForm(draft))
      setPublicationForm(createPublicationFormState())
      setScreenState({
        tone: 'idle',
        message: isMutableAuthoringLifecycle(draft.lifecycleState)
          ? `${draft.displayName} draft를 검토 중이에요.`
          : `${draft.displayName} 기록은 다음 단계 상태라 이 화면에서는 읽기 전용으로 보여 드려요.`,
      })
    })
  }

  function handleRevertDraftChanges() {
    if (isBusy || !hasPendingDraftChanges) {
      return
    }

    if (mode === 'edit' && selectedDraft) {
      startTransition(() => {
        setDraftForm(mapDraftToForm(selectedDraft))
        setScreenState({
          tone: 'idle',
          message:
            '저장 전 변경을 되돌렸어요. 다른 draft를 열거나 현재 draft 검토를 이어갈 수 있어요.',
        })
      })
      return
    }

    startTransition(() => {
      setDraftForm(EMPTY_DRAFT_FORM)
      setScreenState({
        tone: 'idle',
        message:
          '새 draft 입력값을 되돌렸어요. 다른 draft를 열거나 새 baseline 작성을 다시 시작할 수 있어요.',
      })
    })
  }

  async function handleRepairInvalidDraft(draftFolder: string) {
    if (isBusy) {
      return
    }

    setScreenState({
      tone: 'saving',
      message: `${draftFolder} 손상 draft 기록을 정리하고 있어요.`,
    })

    try {
      await presetAuthoringService.repairInvalidDraft({
        draftFolder,
      })
      setWorkspace((current) =>
        current
          ? {
              ...current,
              invalidDrafts: current.invalidDrafts.filter(
                (invalidDraft) => invalidDraft.draftFolder !== draftFolder,
              ),
            }
          : current,
      )
      setHostAccessDenied(false)
      setScreenState({
        tone: 'success',
        message: `${draftFolder} 손상 draft 기록을 정리했어요. 같은 presetId로 새 draft를 다시 만들 수 있어요.`,
      })
    } catch (error) {
      const denied = isCapabilityDenied(error)
      setHostAccessDenied(denied)
      if (denied) {
        setWorkspace(null)
        setSelectedDraftId(null)
        setPublicationForm(createPublicationFormState())
      }
      setScreenState({
        tone: 'error',
        message: normalizeHostMessage(error),
      })
    }
  }

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault()

    if (isBusy) {
      return
    }

    if (!canEditDraftForm) {
      setScreenState({
        tone: 'error',
        message:
          '승인 또는 게시 완료 기록은 이 단계에서 다시 저장하지 않아요. 새 draft를 만들어 후속 작업을 이어가 주세요.',
      })
      return
    }

    setScreenState({
      tone: 'saving',
      message: 'draft artifact candidate를 저장하고 있어요.',
    })

    try {
      const savedDraft =
        mode === 'create'
          ? await presetAuthoringService.createDraftPreset(draftForm)
          : await presetAuthoringService.saveDraftPreset(draftForm)

      setHostAccessDenied(false)
      setWorkspace((current) => upsertDraftInWorkspace(current, savedDraft))
      setMode('edit')
      setSelectedDraftId(savedDraft.presetId)
      setDraftForm(mapDraftToForm(savedDraft))
      setPublicationForm(createPublicationFormState())
      setScreenState({
        tone: 'success',
        message: `${savedDraft.displayName} draft가 저장되었어요. booth catalog는 계속 unchanged 상태예요.`,
      })

      try {
        const refreshedWorkspace = await presetAuthoringService.loadAuthoringWorkspace()
        setHostAccessDenied(false)
        setWorkspace(mergeRefreshedWorkspace(refreshedWorkspace, savedDraft))
      } catch (error) {
        const denied = isCapabilityDenied(error)

        if (denied) {
          setHostAccessDenied(true)
          setWorkspace(null)
          setSelectedDraftId(null)
          setScreenState({
            tone: 'error',
            message: normalizeHostMessage(error),
          })
        }

        // Keep the optimistic draft state when the post-save refresh is temporarily unavailable.
      }
    } catch (error) {
      const denied = isCapabilityDenied(error)
      setHostAccessDenied(denied)
      if (denied) {
        setWorkspace(null)
        setSelectedDraftId(null)
        setPublicationForm(createPublicationFormState())
      }
      setScreenState({
        tone: 'error',
        message: normalizeHostMessage(error),
      })
    }
  }

  async function handleValidateDraft() {
    if (isBusy || !selectedDraftId) {
      return
    }

    if (!canRunValidation) {
      setScreenState({
        tone: 'error',
        message:
          '승인 또는 게시 완료 기록은 이 단계에서 다시 검증하지 않아요. 새 draft를 만들어 host 검증을 이어가 주세요.',
      })
      return
    }

    if (hasUnsavedChanges) {
      setScreenState({
        tone: 'error',
        message: '저장되지 않은 변경이 있어요. 최신 draft를 저장한 뒤 host 검증을 실행해 주세요.',
      })
      return
    }

    setScreenState({
      tone: 'validating',
      message: 'host 기준 booth compatibility 검증을 실행하고 있어요.',
    })

    try {
      const result = await presetAuthoringService.validateDraftPreset({
        presetId: selectedDraftId,
      })

      setHostAccessDenied(false)
      setWorkspace((current) => upsertDraftInWorkspace(current, result.draft))
      setSelectedDraftId(result.draft.presetId)
      setDraftForm(mapDraftToForm(result.draft))
      setMode('edit')
      setPublicationForm(createPublicationFormState())
      setScreenState({
        tone: result.report.status === 'passed' ? 'success' : 'error',
        message:
          result.report.status === 'passed'
            ? `${result.draft.displayName} draft가 approval 준비 완료 상태로 전환되었어요. published booth catalog와 active session은 그대로 유지돼요.`
            : `${result.draft.displayName} draft는 아직 draft 상태예요. 아래 수정 가이드를 확인해 주세요.`,
      })

      try {
        const refreshedWorkspace = await presetAuthoringService.loadAuthoringWorkspace()
        setHostAccessDenied(false)
        setWorkspace(mergeRefreshedWorkspace(refreshedWorkspace, result.draft))
      } catch (error) {
        const denied = isCapabilityDenied(error)

        if (denied) {
          setHostAccessDenied(true)
          setWorkspace(null)
          setSelectedDraftId(null)
          setScreenState({
            tone: 'error',
            message: normalizeHostMessage(error),
          })
        }

        // Keep the optimistic validation result when the post-validate refresh is temporarily unavailable.
      }
    } catch (error) {
      const denied = isCapabilityDenied(error)
      setHostAccessDenied(denied)
      if (denied) {
        setWorkspace(null)
        setSelectedDraftId(null)
        setPublicationForm(createPublicationFormState())
      }
      setScreenState({
        tone: 'error',
        message: normalizeHostMessage(error),
      })
    }
  }

  async function handlePublishValidatedDraft() {
    if (isBusy || !selectedDraft || !latestValidation || !canPublishValidatedDraft) {
      return
    }

    if (!hasPublicationMetadata) {
      setScreenState({
        tone: 'error',
        message: '게시 승인 전에 published version과 승인자 정보를 모두 입력해 주세요.',
      })
      return
    }

    setScreenState({
      tone: 'saving',
      message: '승인 검토를 잠그고 immutable 게시 아티팩트를 만들고 있어요.',
    })

    try {
      const result = await presetAuthoringService.publishValidatedPreset({
        presetId: selectedDraft.presetId,
        draftVersion: selectedDraft.draftVersion,
        validationCheckedAt: latestValidation.checkedAt,
        expectedDisplayName: selectedDraft.displayName,
        publishedVersion: publicationForm.publishedVersion.trim(),
        actorId: publicationForm.actorId.trim(),
        actorLabel: publicationForm.actorLabel.trim(),
        scope: 'future-sessions-only',
        reviewNote: publicationForm.reviewNote.trim(),
      })

      setHostAccessDenied(false)
      setWorkspace((current) => upsertDraftInWorkspace(current, result.draft))
      setSelectedDraftId(result.draft.presetId)
      setDraftForm(mapDraftToForm(result.draft))
      setMode('edit')

      if (result.status === 'published') {
        setPublicationForm(createPublicationFormState())
        setScreenState({
          tone: 'success',
          message:
            `${result.draft.displayName} 승인 게시가 완료되었어요. 새 버전은 미래 세션 catalog에만 반영되고 현재 세션은 그대로 유지돼요.`,
        })

        try {
          const refreshedWorkspace = await presetAuthoringService.loadAuthoringWorkspace()
          setHostAccessDenied(false)
          setWorkspace(mergeRefreshedWorkspace(refreshedWorkspace, result.draft))
        } catch (error) {
          const denied = isCapabilityDenied(error)

          if (denied) {
            setHostAccessDenied(true)
            setWorkspace(null)
            setSelectedDraftId(null)
            setPublicationForm(createPublicationFormState())
            setScreenState({
              tone: 'error',
              message: normalizeHostMessage(error),
            })
          }
        }

        return
      }

      setScreenState({
        tone: 'error',
        message: result.message,
      })
    } catch (error) {
      const denied = isCapabilityDenied(error)
      setHostAccessDenied(denied)
      if (denied) {
        setWorkspace(null)
        setSelectedDraftId(null)
        setPublicationForm(createPublicationFormState())
      }
      setScreenState({
        tone: 'error',
        message: normalizeHostMessage(error),
      })
    }
  }

  return (
    <SurfaceLayout
      eyebrow="Authoring"
      title="Draft Preset Workspace"
      description="내부 프리셋 저작 작업공간에서 draft를 준비하고, host 검증 뒤 future-session용 게시까지 진행할 수 있어요. 이 단계에서도 booth catalog와 현재 세션 binding은 즉시 바뀌지 않아요."
    >
      <section className="authoring-shell">
        <article className="surface-card authoring-card authoring-card--emphasis">
          <p className="authoring-card__eyebrow">Internal Only</p>
          <h2>Draft Validation Boundary</h2>
          <p>
            이 화면은 <strong>draft 작성</strong>, <strong>booth compatibility 검증</strong>,
            <strong>approval 준비 완료 확인</strong>, <strong>future-session 게시</strong>
            까지 다뤄요. 게시를 완료해도 현재 세션과 이미 저장된 capture binding은 그대로
            유지돼요.
          </p>
          <p className={`authoring-status authoring-status--${screenState.tone}`}>
            {screenState.message}
          </p>
        </article>

        {hostAccessDenied ? (
          <article className="surface-card authoring-card">
            <p className="authoring-card__eyebrow">Access Restricted</p>
            <h2>authoring 제어를 표시하지 않았어요</h2>
            <p className="authoring-card__support">
              host 권한 확인이 실패해서 draft 목록과 검증 제어를 숨겼어요. 승인된
              authoring 전용 창과 내부 인증 상태를 다시 확인해 주세요.
            </p>
          </article>
        ) : (
          <div className="authoring-grid">
            <article className="surface-card authoring-card">
              <div className="authoring-card__header">
                <div>
                  <p className="authoring-card__eyebrow">Draft Library</p>
                  <h2>저장된 draft</h2>
                </div>
                <button
                  className="surface-card__action"
                  type="button"
                  onClick={handleCreateDraft}
                  disabled={isBusy}
                >
                  새 draft 만들기
                </button>
              </div>

              <p className="authoring-card__support">
                authoring workspace에 저장된 draft와 approval 준비 완료 상태를 이어서 확인할 수
                있어요.
              </p>

              {workspace?.invalidDrafts.length ? (
                <div className="authoring-stack">
                  {workspace.invalidDrafts.map((invalidDraft) => (
                    <div
                      key={invalidDraft.draftFolder}
                      className="authoring-validation__item authoring-validation__item--error"
                    >
                      <p className="authoring-validation__meta">
                        복구 필요 · {invalidDraft.draftFolder}
                      </p>
                      <p className="authoring-validation__message">{invalidDraft.message}</p>
                      <p className="authoring-validation__guidance">{invalidDraft.guidance}</p>
                      {invalidDraft.canRepair ? (
                        <button
                          className="surface-card__action surface-card__action--secondary"
                          type="button"
                          onClick={() => void handleRepairInvalidDraft(invalidDraft.draftFolder)}
                          disabled={isBusy}
                        >
                          손상 draft 정리
                        </button>
                      ) : null}
                    </div>
                  ))}
                </div>
              ) : null}

              <div className="authoring-draft-list" role="list" aria-label="draft preset 목록">
                {workspace?.drafts.length ? (
                  workspace.drafts.map((draft) => (
                    <button
                      key={draft.presetId}
                      className={`authoring-draft-list__item${
                        selectedDraftId === draft.presetId
                          ? ' authoring-draft-list__item--selected'
                          : ''
                      }`}
                      type="button"
                      onClick={() => handleSelectDraft(draft)}
                      disabled={isBusy}
                    >
                      <span className="authoring-draft-list__name">{draft.displayName}</span>
                      <span className="authoring-draft-list__meta">
                        {draft.presetId} · v{draft.draftVersion}
                      </span>
                      <span className="authoring-draft-list__meta">
                        {formatLifecycleLabel(draft)} ·{' '}
                        {formatValidationStatus(draft.validation.latestReport)}
                      </span>
                      <span className="authoring-draft-list__meta">{draft.updatedAt}</span>
                    </button>
                  ))
                ) : (
                  <p className="authoring-draft-list__empty">
                    아직 저장된 draft가 없어요. 새 draft를 만들어 baseline을 시작하세요.
                  </p>
                )}
              </div>
            </article>

            <div className="authoring-stack">
              <article className="surface-card authoring-card">
                <div className="authoring-card__header">
                  <div>
                    <p className="authoring-card__eyebrow">
                      {mode === 'create' ? 'Create Draft' : 'Edit Draft'}
                    </p>
                    <h2>{mode === 'create' ? '새 draft 작성' : 'draft 상세'}</h2>
                  </div>
                  {selectedDraft ? (
                    <p className="authoring-card__badge">
                      {formatLifecycleLabel(selectedDraft)} · version {selectedDraft.draftVersion}
                    </p>
                  ) : null}
                </div>

                <form className="authoring-form" onSubmit={handleSubmit}>
                  <div className="authoring-form__group">
                    <label className="session-start-form__field">
                      <span className="session-start-form__label">Preset ID</span>
                      <input
                        className="session-start-form__input"
                        name="presetId"
                        value={draftForm.presetId}
                        onChange={(event) => updateForm('presetId', event.target.value)}
                        disabled={mode === 'edit' || isBusy || !canEditDraftForm}
                      />
                    </label>

                    <label className="session-start-form__field">
                      <span className="session-start-form__label">Draft name</span>
                      <input
                        className="session-start-form__input"
                        name="displayName"
                        value={draftForm.displayName}
                        onChange={(event) => updateForm('displayName', event.target.value)}
                        disabled={isBusy || !canEditDraftForm}
                      />
                    </label>
                  </div>

                  <div className="authoring-form__group">
                    <label className="session-start-form__field">
                      <span className="session-start-form__label">darktable version</span>
                      <input
                        className="session-start-form__input"
                        name="darktableVersion"
                        value={draftForm.darktableVersion}
                        onChange={(event) => updateForm('darktableVersion', event.target.value)}
                        disabled={isBusy || !canEditDraftForm}
                      />
                    </label>

                    <label className="session-start-form__field">
                      <span className="session-start-form__label">XMP template 경로</span>
                      <input
                        className="session-start-form__input"
                        name="xmpTemplatePath"
                        value={draftForm.xmpTemplatePath}
                        onChange={(event) => updateForm('xmpTemplatePath', event.target.value)}
                        disabled={isBusy || !canEditDraftForm}
                      />
                    </label>
                  </div>

                  <div className="authoring-form__group">
                    <label className="session-start-form__field">
                      <span className="session-start-form__label">preview profile ID</span>
                      <input
                        className="session-start-form__input"
                        value={draftForm.previewProfile.profileId}
                        onChange={(event) =>
                          updatePreviewProfile('profileId', event.target.value)
                        }
                        disabled={isBusy || !canEditDraftForm}
                      />
                    </label>

                    <label className="session-start-form__field">
                      <span className="session-start-form__label">preview profile 이름</span>
                      <input
                        className="session-start-form__input"
                        value={draftForm.previewProfile.displayName}
                        onChange={(event) =>
                          updatePreviewProfile('displayName', event.target.value)
                        }
                        disabled={isBusy || !canEditDraftForm}
                      />
                    </label>
                  </div>

                  <div className="authoring-form__group">
                    <label className="session-start-form__field">
                      <span className="session-start-form__label">
                        preview output color space
                      </span>
                      <input
                        className="session-start-form__input"
                        value={draftForm.previewProfile.outputColorSpace}
                        onChange={(event) =>
                          updatePreviewProfile('outputColorSpace', event.target.value)
                        }
                        disabled={isBusy || !canEditDraftForm}
                      />
                    </label>

                    <label className="session-start-form__field">
                      <span className="session-start-form__label">final profile ID</span>
                      <input
                        className="session-start-form__input"
                        value={draftForm.finalProfile.profileId}
                        onChange={(event) => updateFinalProfile('profileId', event.target.value)}
                        disabled={isBusy || !canEditDraftForm}
                      />
                    </label>
                  </div>

                  <div className="authoring-form__group">
                    <label className="session-start-form__field">
                      <span className="session-start-form__label">final profile 이름</span>
                      <input
                        className="session-start-form__input"
                        value={draftForm.finalProfile.displayName}
                        onChange={(event) =>
                          updateFinalProfile('displayName', event.target.value)
                        }
                        disabled={isBusy || !canEditDraftForm}
                      />
                    </label>

                    <label className="session-start-form__field">
                      <span className="session-start-form__label">
                        final output color space
                      </span>
                      <input
                        className="session-start-form__input"
                        value={draftForm.finalProfile.outputColorSpace}
                        onChange={(event) =>
                          updateFinalProfile('outputColorSpace', event.target.value)
                        }
                        disabled={isBusy || !canEditDraftForm}
                      />
                    </label>
                  </div>

                  <div className="authoring-form__group">
                    <label className="session-start-form__field">
                      <span className="session-start-form__label">noise policy ID</span>
                      <input
                        className="session-start-form__input"
                        value={draftForm.noisePolicy.policyId}
                        onChange={(event) => updateNoisePolicy('policyId', event.target.value)}
                        disabled={isBusy || !canEditDraftForm}
                      />
                    </label>

                    <label className="session-start-form__field">
                      <span className="session-start-form__label">noise policy 이름</span>
                      <input
                        className="session-start-form__input"
                        value={draftForm.noisePolicy.displayName}
                        onChange={(event) =>
                          updateNoisePolicy('displayName', event.target.value)
                        }
                        disabled={isBusy || !canEditDraftForm}
                      />
                    </label>
                  </div>

                  <label className="session-start-form__field">
                    <span className="session-start-form__label">noise reduction mode</span>
                    <input
                      className="session-start-form__input"
                      value={draftForm.noisePolicy.reductionMode}
                      onChange={(event) =>
                        updateNoisePolicy('reductionMode', event.target.value)
                      }
                      disabled={isBusy || !canEditDraftForm}
                    />
                  </label>

                  <div className="authoring-form__group">
                    <label className="session-start-form__field">
                      <span className="session-start-form__label">대표 preview 경로</span>
                      <input
                        className="session-start-form__input"
                        name="previewAssetPath"
                        value={draftForm.preview.assetPath}
                        onChange={(event) => updatePreview('assetPath', event.target.value)}
                        disabled={isBusy || !canEditDraftForm}
                      />
                    </label>

                    <label className="session-start-form__field">
                      <span className="session-start-form__label">preview 설명</span>
                      <input
                        className="session-start-form__input"
                        name="previewAltText"
                        value={draftForm.preview.altText}
                        onChange={(event) => updatePreview('altText', event.target.value)}
                        disabled={isBusy || !canEditDraftForm}
                      />
                    </label>
                  </div>

                  <div className="authoring-form__group">
                    <label className="session-start-form__field">
                      <span className="session-start-form__label">대표 sample-cut 경로</span>
                      <input
                        className="session-start-form__input"
                        name="sampleCutAssetPath"
                        value={draftForm.sampleCut.assetPath}
                        onChange={(event) => updateSampleCut('assetPath', event.target.value)}
                        disabled={isBusy || !canEditDraftForm}
                      />
                    </label>

                    <label className="session-start-form__field">
                      <span className="session-start-form__label">sample-cut 설명</span>
                      <input
                        className="session-start-form__input"
                        name="sampleCutAltText"
                        value={draftForm.sampleCut.altText}
                        onChange={(event) => updateSampleCut('altText', event.target.value)}
                        disabled={isBusy || !canEditDraftForm}
                      />
                    </label>
                  </div>

                  <label className="session-start-form__field">
                    <span className="session-start-form__label">기본 설명</span>
                    <textarea
                      className="session-start-form__input authoring-form__textarea"
                      name="description"
                      value={draftForm.description ?? ''}
                      onChange={(event) => updateForm('description', event.target.value)}
                      disabled={isBusy || !canEditDraftForm}
                    />
                  </label>

                  <label className="session-start-form__field">
                    <span className="session-start-form__label">내부 메모</span>
                    <textarea
                      className="session-start-form__input authoring-form__textarea"
                      name="notes"
                      value={draftForm.notes ?? ''}
                      onChange={(event) => updateForm('notes', event.target.value)}
                      disabled={isBusy || !canEditDraftForm}
                    />
                  </label>

                  <div className="authoring-form__footer">
                    <p className="authoring-form__note">
                      {canEditDraftForm
                        ? '저장과 검증은 authoring 전용 경계에서만 일어나고, active booth session binding이나 published catalog는 그대로 유지돼요.'
                        : '승인 또는 게시 완료 기록은 여기서 되돌리지 않아요. 후속 수정이 필요하면 새 draft를 만들어 별도 검증 흐름으로 이어가 주세요.'}
                    </p>

                    <div className="authoring-form__actions">
                      {canEditDraftForm ? (
                        <button
                          className="surface-card__action surface-card__action--secondary"
                          type="button"
                          onClick={handleRevertDraftChanges}
                          disabled={isBusy || !hasPendingDraftChanges}
                        >
                          변경 되돌리기
                        </button>
                      ) : null}
                      {canRunValidation ? (
                        <button
                          className="surface-card__action surface-card__action--secondary"
                          type="button"
                          onClick={handleValidateDraft}
                          disabled={isBusy || !selectedDraftId || hasUnsavedChanges}
                        >
                          호환성 검증 실행
                        </button>
                      ) : null}
                      {canEditDraftForm ? (
                        <button className="surface-card__action" type="submit" disabled={isBusy}>
                          {mode === 'create' ? 'draft 저장' : 'draft 업데이트'}
                        </button>
                      ) : null}
                    </div>
                  </div>
                </form>
              </article>

              <article className="surface-card authoring-card">
                <div className="authoring-card__header">
                  <div>
                    <p className="authoring-card__eyebrow">Next Step</p>
                    <h2>다음 승인 단계 안내</h2>
                  </div>
                  {selectedDraft ? (
                    <p className="authoring-card__badge">
                      {selectedDraft.lifecycleState === 'validated'
                        ? 'approval 준비 완료'
                        : formatLifecycleLabel(selectedDraft)}
                    </p>
                  ) : null}
                </div>

                {selectedDraft ? (
                  <div className="authoring-validation">
                    <p className="authoring-card__support">
                      validated draft를 미래 세션 catalog에 게시할 수 있지만, 현재 진행 중인
                      세션과 기존 capture binding은 그대로 유지돼요.
                    </p>
                    {selectedDraft.lifecycleState === 'approved' ||
                    selectedDraft.lifecycleState === 'published' ? (
                      <p className="authoring-card__support">
                        이 기록은 이미 승인/게시 단계를 지난 상태예요. 이 화면에서는 읽기 전용으로만
                        확인하고, 후속 수정이 필요하면 새 draft를 만들어 별도 검증 흐름으로 이어가
                        주세요.
                      </p>
                    ) : selectedDraft.lifecycleState !== 'validated' ? (
                      <p className="authoring-card__support">
                        다음 승인 단계로 넘기려면 먼저 저장된 draft를 host validation으로 통과시켜
                        approval 준비 완료 상태를 만들어 주세요.
                      </p>
                    ) : hasUnsavedChanges ? (
                      <p className="authoring-card__support">
                        저장되지 않은 변경이 있어요. approval 준비 완료 상태를 신뢰하려면 최신
                        draft를 먼저 저장해 주세요.
                      </p>
                    ) : canPublishValidatedDraft ? (
                      <>
                        <p className="authoring-card__support">
                          이 draft는 approval 준비 완료 상태예요. 게시 승인을 실행하면 다음 새
                          세션부터 preset 선택 화면에 나타나요.
                        </p>
                        <div className="authoring-form__group">
                          <label className="session-start-form__field">
                            <span className="session-start-form__label">Published version</span>
                            <input
                              className="session-start-form__input"
                              value={publicationForm.publishedVersion}
                              onChange={(event) =>
                                updatePublicationForm('publishedVersion', event.target.value)
                              }
                              disabled={isBusy}
                            />
                          </label>
                          <label className="session-start-form__field">
                            <span className="session-start-form__label">승인자 ID</span>
                            <input
                              className="session-start-form__input"
                              value={publicationForm.actorId}
                              onChange={(event) =>
                                updatePublicationForm('actorId', event.target.value)
                              }
                              disabled={isBusy}
                            />
                          </label>
                        </div>
                        <div className="authoring-form__group">
                          <label className="session-start-form__field">
                            <span className="session-start-form__label">승인자 이름</span>
                            <input
                              className="session-start-form__input"
                              value={publicationForm.actorLabel}
                              onChange={(event) =>
                                updatePublicationForm('actorLabel', event.target.value)
                              }
                              disabled={isBusy}
                            />
                          </label>
                          <label className="session-start-form__field">
                            <span className="session-start-form__label">검토 메모</span>
                            <input
                              className="session-start-form__input"
                              value={publicationForm.reviewNote}
                              onChange={(event) =>
                                updatePublicationForm('reviewNote', event.target.value)
                              }
                              disabled={isBusy}
                            />
                          </label>
                        </div>
                        <p className="authoring-form__note">
                          게시 성공은 future-session catalog에만 반영돼요. 이미 시작된 세션에는
                          새 preset이 나타나지 않아요.
                        </p>
                        {!hasPublicationMetadata ? (
                          <p className="authoring-form__note">
                            게시 승인을 실행하려면 published version과 승인자 정보를 모두 입력해
                            주세요.
                          </p>
                        ) : null}
                        <div className="authoring-form__actions">
                          <button
                            className="surface-card__action"
                            type="button"
                            onClick={() => void handlePublishValidatedDraft()}
                            disabled={isBusy || !canPublishValidatedDraft}
                          >
                            게시 승인 실행
                          </button>
                        </div>
                      </>
                    ) : blocksNextStepUntilRevalidated ? (
                      <p className="authoring-card__support">
                        후속 승인 단계에서 다시 확인이 필요한 상태예요. 최신 draft를 다시 검토하고
                        host 검증을 실행한 뒤 다음 단계로 넘겨 주세요.
                      </p>
                    ) : (
                      <p className="authoring-card__support">
                        host 검증을 통과한 draft는 approval 준비 완료 상태를 유지하고, 게시에
                        필요한 메타데이터를 다시 확인한 뒤 승인 절차를 이어가면 돼요.
                      </p>
                    )}
                  </div>
                ) : (
                  <p className="authoring-card__support">
                    다음 승인 단계 안내를 보려면 저장된 draft를 먼저 선택해 주세요.
                  </p>
                )}
              </article>

              <article className="surface-card authoring-card">
                <div className="authoring-card__header">
                  <div>
                    <p className="authoring-card__eyebrow">Latest Validation</p>
                    <h2>booth compatibility 결과</h2>
                  </div>
                  {selectedDraft ? (
                    <p className="authoring-card__badge">
                      {formatValidationStatus(latestValidation)} · history{' '}
                      {selectedDraft.validation.history.length}
                    </p>
                  ) : null}
                </div>

                {selectedDraft ? (
                  hasUnsavedChanges ? (
                    <p className="authoring-card__support">
                      저장되지 않은 변경이 있어요. 현재 보이는 값으로 검증하려면 먼저 draft를
                      저장해 주세요.
                    </p>
                  ) : latestValidation ? (
                    <div className="authoring-validation">
                      <p className="authoring-validation__summary">
                        host가 {latestValidation.checkedAt}에 검증을 실행했고,
                        현재 lifecycle은 <strong>{formatLifecycleLabel(selectedDraft)}</strong>
                        예요.
                      </p>
                      {latestValidation.findings.length ? (
                        <ul className="authoring-validation__list">
                          {latestValidation.findings.map((finding) => (
                            <li
                              key={`${finding.ruleCode}-${finding.fieldPath ?? 'artifact'}`}
                              className={`authoring-validation__item authoring-validation__item--${finding.severity}`}
                            >
                              <p className="authoring-validation__meta">
                                {formatFindingTone(finding)} · {finding.ruleCode}
                                {finding.fieldPath ? ` · ${finding.fieldPath}` : ''}
                              </p>
                              <p className="authoring-validation__message">{finding.message}</p>
                              <p className="authoring-validation__guidance">
                                {finding.guidance}
                              </p>
                            </li>
                          ))}
                        </ul>
                      ) : (
                        <p className="authoring-validation__summary">
                          필수 booth compatibility rule을 모두 통과했어요. 이번 화면에서는
                          approval 준비 완료 상태까지 확인했고, 후속 승인 절차는 다음 단계에서
                          이어가면 돼요.
                        </p>
                      )}
                    </div>
                  ) : (
                    <p className="authoring-card__support">
                      아직 검증 기록이 없어요. 저장된 draft를 선택한 뒤 host 검증을 실행하면
                      여기에서 rule별 수정 가이드를 확인할 수 있어요.
                    </p>
                  )
                ) : (
                  <p className="authoring-card__support">
                    검증 결과를 보려면 저장된 draft를 먼저 선택해 주세요.
                  </p>
                )}
              </article>

              {selectedDraft &&
              (selectedDraft.lifecycleState === 'approved' ||
                selectedDraft.lifecycleState === 'published') &&
              followUpHistory.length ? (
                <article className="surface-card authoring-card">
                  <div className="authoring-card__header">
                    <div>
                      <p className="authoring-card__eyebrow">Follow-up History</p>
                      <h2>후속 단계 이력</h2>
                    </div>
                    <p className="authoring-card__badge">{followUpHistory.length} entries</p>
                  </div>

                  <ul className="authoring-validation__list">
                    {followUpHistory.map((record) => (
                      <li
                        key={`${record.action}-${record.publishedVersion}-${record.notedAt}`}
                        className="authoring-validation__item"
                      >
                        <p className="authoring-validation__meta">
                          {formatPublicationActionLabel(record.action)} · {record.publishedVersion}{' '}
                          · {record.actorLabel}
                        </p>
                        <p className="authoring-validation__message">{record.notedAt}</p>
                        {record.reviewNote ? (
                          <p className="authoring-card__support">
                            검토 메모: {record.reviewNote}
                          </p>
                        ) : null}
                      </li>
                    ))}
                  </ul>
                </article>
              ) : null}

            </div>
          </div>
        )}
      </section>
    </SurfaceLayout>
  )
}
