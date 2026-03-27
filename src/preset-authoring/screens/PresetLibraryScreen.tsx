import { startTransition, useEffect, useMemo, useRef, useState } from 'react'

import type {
  AuthoringWorkspaceResult,
  CatalogStateResult,
  CatalogStateSummary,
  CatalogVersionHistoryItem,
  PublicationAuditRecord,
  DraftPresetEditPayload,
  DraftPresetSummary,
  DraftValidationFinding,
  DraftValidationReport,
  HostErrorEnvelope,
  RollbackPresetCatalogInput,
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

type RollbackFormState = {
  targetPublishedVersion: string
  actorId: string
  actorLabel: string
}

const EMPTY_DRAFT_FORM: DraftPresetEditPayload = {
  presetId: 'preset_new-draft',
  displayName: '',
  lifecycleState: 'draft',
  darktableVersion: '5.4.1',
  darktableProjectPath: 'darktable/project.dtpreset',
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

function createRollbackFormState(
  summary: CatalogStateSummary | null,
  previous?: RollbackFormState,
): RollbackFormState {
  const defaultTarget =
    summary?.publishedPresets.find(
      (preset) => preset.publishedVersion !== summary.livePublishedVersion,
    )?.publishedVersion ?? ''

  return {
    targetPublishedVersion:
      previous?.targetPublishedVersion && previous.targetPublishedVersion.length > 0
        ? previous.targetPublishedVersion
        : defaultTarget,
    actorId: previous?.actorId ?? '',
    actorLabel: previous?.actorLabel ?? '',
  }
}

function mapDraftToForm(draft: DraftPresetSummary): DraftPresetEditPayload {
  return {
    presetId: draft.presetId,
    displayName: draft.displayName,
    lifecycleState: 'draft',
    darktableVersion: draft.darktableVersion,
    darktableProjectPath: draft.darktableProjectPath,
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

function findLatestPublicationRecord(draft: DraftPresetSummary | null) {
  if (!draft || draft.publicationHistory.length === 0) {
    return null
  }

  return draft.publicationHistory[draft.publicationHistory.length - 1] ?? null
}

function formatPublicationActionLabel(action: PublicationAuditRecord['action']) {
  switch (action) {
    case 'approved':
      return '승인 완료'
    case 'published':
      return '게시 완료'
    case 'rejected':
      return '게시 거절'
    default:
      return action
  }
}

function formatPublicationReasonLabel(
  reasonCode: PublicationAuditRecord['reasonCode'],
) {
  switch (reasonCode) {
    case 'draft-not-validated':
      return '검증 상태 확인 필요'
    case 'stale-validation':
      return '검증 결과가 오래됨'
    case 'metadata-mismatch':
      return '승인 메타데이터 불일치'
    case 'duplicate-version':
      return '중복 게시 버전'
    case 'path-escape':
      return '작업공간 바깥 경로 차단'
    case 'future-session-only-violation':
      return 'future-session-only 규칙 위반'
    default:
      return null
  }
}

function formatCatalogActionLabel(actionType: CatalogVersionHistoryItem['actionType']) {
  return actionType
}

export function PresetLibraryScreen() {
  const presetAuthoringService = usePresetAuthoringService()
  const loadRequestVersionRef = useRef(0)
  const [workspace, setWorkspace] = useState<AuthoringWorkspaceResult | null>(null)
  const [catalogState, setCatalogState] = useState<CatalogStateResult | null>(null)
  const [rollbackForms, setRollbackForms] = useState<Record<string, RollbackFormState>>({})
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
        const [workspaceResult, catalogStateResult] = await Promise.all([
          presetAuthoringService.loadAuthoringWorkspace(),
          presetAuthoringService.loadPresetCatalogState(),
        ])

        if (!active || loadRequestVersionRef.current !== requestVersion) {
          return
        }

        setHostAccessDenied(false)
        setWorkspace(workspaceResult)
        setCatalogState(catalogStateResult)
        setRollbackForms((current) =>
          Object.fromEntries(
            catalogStateResult.presets.map((summary) => [
              summary.presetId,
              createRollbackFormState(summary, current[summary.presetId]),
            ]),
          ),
        )

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
          setCatalogState(null)
          setRollbackForms({})
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
  const latestValidation = selectedDraft?.validation.latestReport ?? null
  const latestPublicationRecord = findLatestPublicationRecord(selectedDraft)
  const canEditSelectedDraft =
    selectedDraft !== null && isMutableAuthoringLifecycle(selectedDraft.lifecycleState)
  const canEditDraftForm = mode === 'create' || canEditSelectedDraft
  const canRunValidation = mode === 'edit' && canEditSelectedDraft
  const canPublishValidatedDraft =
    selectedDraft !== null &&
    selectedDraft.lifecycleState === 'validated' &&
    latestValidation?.status === 'passed' &&
    !hasUnsavedChanges
  const isBusy =
    screenState.tone === 'loading' ||
    screenState.tone === 'saving' ||
    screenState.tone === 'validating'
  const hasPublishedCatalogEntries = (catalogState?.presets.length ?? 0) > 0

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

  function updateRollbackForm<K extends keyof RollbackFormState>(
    presetId: string,
    key: K,
    value: RollbackFormState[K],
  ) {
    setRollbackForms((current) => ({
      ...current,
      [presetId]: {
        ...createRollbackFormState(
          catalogState?.presets.find((summary) => summary.presetId === presetId) ?? null,
          current[presetId],
        ),
        [key]: value,
      },
    }))
  }

  function handleCreateDraft() {
    if (isBusy) {
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
      const [refreshedWorkspace, refreshedCatalogState] = await Promise.all([
        presetAuthoringService.loadAuthoringWorkspace(),
        presetAuthoringService.loadPresetCatalogState(),
      ])

      setHostAccessDenied(false)
      setWorkspace(refreshedWorkspace)
      setCatalogState(refreshedCatalogState)
      setRollbackForms((current) =>
        Object.fromEntries(
          refreshedCatalogState.presets.map((summary) => [
            summary.presetId,
            createRollbackFormState(summary, current[summary.presetId]),
          ]),
        ),
      )
      setMode('edit')
      setSelectedDraftId(savedDraft.presetId)
      setDraftForm(mapDraftToForm(savedDraft))
      setScreenState({
        tone: 'success',
        message: `${savedDraft.displayName} draft가 저장되었어요. booth catalog는 계속 unchanged 상태예요.`,
      })
    } catch (error) {
      const denied = isCapabilityDenied(error)
      setHostAccessDenied(denied)
      if (denied) {
        setWorkspace(null)
        setCatalogState(null)
        setRollbackForms({})
        setSelectedDraftId(null)
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
        publishedVersion: publicationForm.publishedVersion,
        actorId: publicationForm.actorId,
        actorLabel: publicationForm.actorLabel,
        scope: 'future-sessions-only',
        reviewNote: publicationForm.reviewNote,
      })
      const [refreshedWorkspace, refreshedCatalogState] = await Promise.all([
        presetAuthoringService.loadAuthoringWorkspace(),
        presetAuthoringService.loadPresetCatalogState(),
      ])

      setHostAccessDenied(false)
      setWorkspace(refreshedWorkspace)
      setCatalogState(refreshedCatalogState)
      setRollbackForms((current) =>
        Object.fromEntries(
          refreshedCatalogState.presets.map((summary) => [
            summary.presetId,
            createRollbackFormState(summary, current[summary.presetId]),
          ]),
        ),
      )
      setSelectedDraftId(result.draft.presetId)
      setDraftForm(mapDraftToForm(result.draft))
      setMode('edit')

      if (result.status === 'published') {
        setPublicationForm(createPublicationFormState())
        setScreenState({
          tone: 'success',
          message: `${result.draft.displayName} 승인 게시가 완료되었어요. 새 버전은 미래 세션 catalog에만 반영되고 현재 세션은 그대로 유지돼요.`,
        })
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
        setCatalogState(null)
        setRollbackForms({})
        setSelectedDraftId(null)
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
      const [refreshedWorkspace, refreshedCatalogState] = await Promise.all([
        presetAuthoringService.loadAuthoringWorkspace(),
        presetAuthoringService.loadPresetCatalogState(),
      ])

      setHostAccessDenied(false)
      setWorkspace(refreshedWorkspace)
      setCatalogState(refreshedCatalogState)
      setRollbackForms((current) =>
        Object.fromEntries(
          refreshedCatalogState.presets.map((summary) => [
            summary.presetId,
            createRollbackFormState(summary, current[summary.presetId]),
          ]),
        ),
      )
      setSelectedDraftId(result.draft.presetId)
      setDraftForm(mapDraftToForm(result.draft))
      setMode('edit')
      setScreenState({
        tone: result.report.status === 'passed' ? 'success' : 'error',
        message:
          result.report.status === 'passed'
            ? `${result.draft.displayName} draft가 approval 준비 완료 상태로 전환되었어요. published booth catalog와 active session은 그대로 유지돼요.`
            : `${result.draft.displayName} draft는 아직 draft 상태예요. 아래 수정 가이드를 확인해 주세요.`,
      })
    } catch (error) {
      const denied = isCapabilityDenied(error)
      setHostAccessDenied(denied)
      if (denied) {
        setWorkspace(null)
        setCatalogState(null)
        setRollbackForms({})
        setSelectedDraftId(null)
      }
      setScreenState({
        tone: 'error',
        message: normalizeHostMessage(error),
      })
    }
  }

  async function handleRollbackPresetCatalog(summary: CatalogStateSummary) {
    if (isBusy) {
      return
    }

    const rollbackForm = createRollbackFormState(
      summary,
      rollbackForms[summary.presetId],
    )
    if (!catalogState || rollbackForm.targetPublishedVersion.length === 0) {
      setScreenState({
        tone: 'error',
        message: 'rollback target version을 먼저 선택해 주세요.',
      })
      return
    }

    setScreenState({
      tone: 'saving',
      message: '선택한 승인 버전으로 future-session live catalog를 되돌리고 있어요.',
    })

    try {
      const result = await presetAuthoringService.rollbackPresetCatalog({
        presetId: summary.presetId,
        targetPublishedVersion: rollbackForm.targetPublishedVersion,
        expectedCatalogRevision: catalogState.catalogRevision,
        actorId: rollbackForm.actorId,
        actorLabel: rollbackForm.actorLabel,
      } satisfies RollbackPresetCatalogInput)

      const [workspaceResult, catalogStateResult] = await Promise.all([
        presetAuthoringService.loadAuthoringWorkspace(),
        presetAuthoringService.loadPresetCatalogState(),
      ])

      setHostAccessDenied(false)
      setWorkspace(workspaceResult)
      setCatalogState(catalogStateResult)
      setRollbackForms((current) =>
        Object.fromEntries(
          catalogStateResult.presets.map((nextSummary) => [
            nextSummary.presetId,
            createRollbackFormState(nextSummary, current[nextSummary.presetId]),
          ]),
        ),
      )

      if (result.status === 'rolled-back') {
        setScreenState({
          tone: 'success',
          message: result.message,
        })
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
        setCatalogState(null)
        setRollbackForms({})
        setSelectedDraftId(null)
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
      description="내부 프리셋 저작 작업공간에서 draft를 준비하고, host 검증 뒤 approval-ready 상태 확인과 승인/게시 검토까지 진행할 수 있어요. 이 단계에서도 booth catalog와 현재 세션 binding은 즉시 바뀌지 않아요."
    >
      <section className="authoring-shell">
        <article className="surface-card authoring-card authoring-card--emphasis">
          <p className="authoring-card__eyebrow">Internal Only</p>
          <h2>Draft Validation Boundary</h2>
          <p>
            이 화면은 <strong>draft 작성</strong>, <strong>booth compatibility 검증</strong>,
            <strong>approval 준비 완료 확인</strong>, 그리고 <strong>승인/게시 검토</strong>를
            함께 다뤄요. 그래도 여기서 실행하는 게시 작업은 진행 중인 booth session과 이미
            저장된 capture binding을 바꾸지 않고 미래 세션 catalog에만 반영돼요.
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

            <article className="surface-card authoring-card">
              <div className="authoring-card__header">
                <div>
                  <p className="authoring-card__eyebrow">Catalog Versions</p>
                  <h2>미래 세션 live catalog 관리</h2>
                </div>
                {catalogState ? (
                  <p className="authoring-card__badge">
                    revision {catalogState.catalogRevision}
                  </p>
                ) : null}
              </div>

              {hasPublishedCatalogEntries ? (
                <div className="authoring-stack">
                  {catalogState?.presets.map((summary) => {
                    const rollbackForm = createRollbackFormState(
                      summary,
                      rollbackForms[summary.presetId],
                    )
                    const livePreset =
                      summary.publishedPresets.find(
                        (preset) =>
                          preset.publishedVersion === summary.livePublishedVersion,
                      ) ?? summary.publishedPresets[0]
                    const rollbackCandidates = summary.publishedPresets.filter(
                      (preset) =>
                        preset.publishedVersion !== summary.livePublishedVersion,
                    )

                    return (
                      <div key={summary.presetId} className="authoring-validation">
                        <p className="authoring-validation__summary">
                          <strong>{livePreset.displayName}</strong>
                        </p>
                        <p className="authoring-card__support">
                          현재 future session live version: {summary.livePublishedVersion}
                        </p>
                        <p className="authoring-card__support">
                          진행 중인 세션은 지금 바인딩된 preset version을 그대로 유지하고, 새로
                          시작한 세션부터만 이 live catalog를 봐요.
                        </p>

                        {rollbackCandidates.length > 0 ? (
                          <>
                            <div className="authoring-form__group">
                              <label className="session-start-form__field">
                                <span className="session-start-form__label">
                                  Rollback target version
                                </span>
                                <select
                                  className="session-start-form__input"
                                  value={rollbackForm.targetPublishedVersion}
                                  onChange={(event) =>
                                    updateRollbackForm(
                                      summary.presetId,
                                      'targetPublishedVersion',
                                      event.target.value,
                                    )
                                  }
                                  disabled={isBusy}
                                >
                                  <option value="">선택해 주세요</option>
                                  {rollbackCandidates.map((preset) => (
                                    <option
                                      key={preset.publishedVersion}
                                      value={preset.publishedVersion}
                                    >
                                      {preset.publishedVersion}
                                    </option>
                                  ))}
                                </select>
                              </label>

                              <label className="session-start-form__field">
                                <span className="session-start-form__label">롤백 승인자 ID</span>
                                <input
                                  className="session-start-form__input"
                                  value={rollbackForm.actorId}
                                  onChange={(event) =>
                                    updateRollbackForm(
                                      summary.presetId,
                                      'actorId',
                                      event.target.value,
                                    )
                                  }
                                  disabled={isBusy}
                                />
                              </label>
                            </div>

                            <div className="authoring-form__group">
                              <label className="session-start-form__field">
                                <span className="session-start-form__label">롤백 승인자 이름</span>
                                <input
                                  className="session-start-form__input"
                                  value={rollbackForm.actorLabel}
                                  onChange={(event) =>
                                    updateRollbackForm(
                                      summary.presetId,
                                      'actorLabel',
                                      event.target.value,
                                    )
                                  }
                                  disabled={isBusy}
                                />
                              </label>
                            </div>

                            <div className="authoring-form__actions">
                              <button
                                className="surface-card__action surface-card__action--secondary"
                                type="button"
                                onClick={() => void handleRollbackPresetCatalog(summary)}
                                disabled={
                                  isBusy ||
                                  rollbackForm.targetPublishedVersion.length === 0
                                }
                              >
                                선택한 버전으로 롤백
                              </button>
                            </div>
                          </>
                        ) : (
                          <p className="authoring-card__support">
                            이 preset은 아직 rollback 가능한 이전 승인 버전이 없어요.
                          </p>
                        )}

                        {summary.versionHistory.length > 0 ? (
                          <ul className="authoring-validation__list">
                            {summary.versionHistory
                              .slice()
                              .reverse()
                              .map((entry) => (
                                <li
                                  key={`${entry.actionType}-${entry.toPublishedVersion}-${entry.happenedAt}`}
                                  className="authoring-validation__item"
                                >
                                  <p className="authoring-validation__meta">
                                    {formatCatalogActionLabel(entry.actionType)} ·{' '}
                                    {entry.toPublishedVersion} · {entry.actorLabel}
                                  </p>
                                  <p className="authoring-validation__message">
                                    {entry.happenedAt}
                                  </p>
                                </li>
                              ))}
                          </ul>
                        ) : null}
                      </div>
                    )
                  })}
                </div>
              ) : (
                <p className="authoring-card__support">
                  아직 live catalog를 관리할 published preset이 없어요. 승인 게시가 끝나면 여기에서
                  현재 live version과 rollback 가능한 승인 버전을 함께 볼 수 있어요.
                </p>
              )}
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

                  <label className="session-start-form__field">
                    <span className="session-start-form__label">darktable project 참조</span>
                    <input
                      className="session-start-form__input"
                      name="darktableProjectPath"
                      value={draftForm.darktableProjectPath}
                      onChange={(event) =>
                        updateForm('darktableProjectPath', event.target.value)
                      }
                      disabled={isBusy || !canEditDraftForm}
                    />
                  </label>

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
                    <h2>승인 및 게시 검토</h2>
                  </div>
                  {selectedDraft ? (
                    <p className="authoring-card__badge">
                      {selectedDraft.lifecycleState === 'validated'
                        ? 'handoff 준비'
                        : formatLifecycleLabel(selectedDraft)}
                    </p>
                  ) : null}
                </div>

                {selectedDraft ? (
                  <div className="authoring-form">
                    <p className="authoring-card__support">
                      승인 게시는 future session catalog에만 반영되고, 현재 세션과 이미 저장된
                      capture binding은 계속 그대로 유지돼요.
                    </p>

                    {canPublishValidatedDraft ? (
                      <>
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
                          승인 후 host가 {'approved -> published'} 전이를 잠그고 immutable
                          bundle을 생성해요. 이 작업은 미래 세션 catalog에만 반영돼요.
                        </p>

                        <div className="authoring-form__actions">
                          <button
                            className="surface-card__action"
                            type="button"
                            onClick={handlePublishValidatedDraft}
                            disabled={isBusy}
                          >
                            게시 승인 실행
                          </button>
                        </div>
                      </>
                    ) : selectedDraft.lifecycleState !== 'validated' ? (
                      selectedDraft.lifecycleState === 'approved' ||
                      selectedDraft.lifecycleState === 'published' ? (
                        <p className="authoring-card__support">
                          이 기록은 이미 승인/게시 단계를 지난 상태예요. 이 화면에서는 읽기 전용으로만
                          확인하고, 후속 수정이 필요하면 새 draft를 만들어 별도 검증 흐름으로 이어가
                          주세요.
                        </p>
                      ) : (
                        <p className="authoring-card__support">
                          승인 게시를 시작하려면 host validation을 통과해 approval-ready 상태를
                          먼저 만들어 주세요.
                        </p>
                      )
                    ) : hasUnsavedChanges ? (
                      <p className="authoring-card__support">
                        저장되지 않은 변경이 있어요. approval-ready 상태를 신뢰하려면 최신 draft를
                        먼저 저장해 주세요.
                      </p>
                    ) : (
                      <p className="authoring-card__support">
                        이 draft는 approval 준비 완료 상태예요. 승인자 정보와 버전을 확인한 뒤
                        게시를 실행할 수 있어요.
                      </p>
                    )}

                    {latestPublicationRecord?.action === 'rejected' ? (
                      <div className="authoring-validation">
                        <p className="authoring-validation__summary">
                          최근 게시 시도는 거절되었어요.
                          {formatPublicationReasonLabel(latestPublicationRecord.reasonCode)
                            ? ` ${formatPublicationReasonLabel(latestPublicationRecord.reasonCode)} 사유를 먼저 해결해 주세요.`
                            : ''}
                        </p>
                        <ul className="authoring-validation__list">
                          <li className="authoring-validation__item authoring-validation__item--error">
                            <p className="authoring-validation__meta">
                              게시 거절
                              {latestPublicationRecord.reasonCode
                                ? ` · ${latestPublicationRecord.reasonCode}`
                                : ''}
                            </p>
                            <p className="authoring-validation__message">
                              {screenState.tone === 'error'
                                ? screenState.message
                                : '최근 승인 게시 요청이 거절되었어요.'}
                            </p>
                            <p className="authoring-validation__guidance">
                              {latestPublicationRecord.guidance}
                            </p>
                          </li>
                        </ul>
                      </div>
                    ) : null}
                  </div>
                ) : (
                  <p className="authoring-card__support">
                    승인 게시 검토를 보려면 저장된 draft를 먼저 선택해 주세요.
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
                          필수 booth compatibility rule을 모두 통과했어요. 이번 단계에서는
                          approval 준비 완료까지만 확인하고 다음 승인 단계로 넘길 수 있어요.
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

              <article className="surface-card authoring-card">
                <div className="authoring-card__header">
                  <div>
                    <p className="authoring-card__eyebrow">Publication History</p>
                    <h2>승인/게시 이력</h2>
                  </div>
                  {selectedDraft ? (
                    <p className="authoring-card__badge">
                      {selectedDraft.publicationHistory.length} entries
                    </p>
                  ) : null}
                </div>

                {selectedDraft?.publicationHistory.length ? (
                  <ul className="authoring-validation__list">
                    {selectedDraft.publicationHistory.map((record) => (
                      <li
                        key={`${record.action}-${record.publishedVersion}-${record.notedAt}`}
                        className={`authoring-validation__item authoring-validation__item--${
                          record.action === 'rejected' ? 'error' : 'warning'
                        }`}
                      >
                        <p className="authoring-validation__meta">
                          {formatPublicationActionLabel(record.action)} · {record.publishedVersion} ·{' '}
                          {record.actorLabel}
                        </p>
                        <p className="authoring-validation__message">{record.notedAt}</p>
                        <p className="authoring-validation__guidance">{record.guidance}</p>
                        {record.reviewNote ? (
                          <p className="authoring-card__support">
                            검토 메모: {record.reviewNote}
                          </p>
                        ) : null}
                        {formatPublicationReasonLabel(record.reasonCode) ? (
                          <p className="authoring-card__support">
                            사유: {formatPublicationReasonLabel(record.reasonCode)}
                          </p>
                        ) : null}
                      </li>
                    ))}
                  </ul>
                ) : (
                  <p className="authoring-card__support">
                    아직 승인/게시 이력이 없어요. approval-ready draft를 게시하면 여기에서 승인과
                    게시 경계를 추적할 수 있어요.
                  </p>
                )}
              </article>
            </div>
          </div>
        )}
      </section>
    </SurfaceLayout>
  )
}
