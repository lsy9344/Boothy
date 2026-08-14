import { z } from 'zod'

import { captureIdSchema, captureRequestIdSchema } from './session-capture'
import { sessionIdSchema } from './ids'

export const viewerDisplaySchemaVersion = 'viewer-display/v1' as const
export const viewerDisplayUpdateSchemaVersion =
  'viewer-display-update/v1' as const

/**
 * 승인된 display tier.
 *
 * Story 7.2는 `sample` 하나만 등록한다. `displayFitPresetProxy`는 Story 7.4가,
 * `rawRefinedDisplay`와 `final`은 Story 7.6이 추가한다. 순서 비교 함수는 지금 만들어 두고
 * tier가 늘어날 때 값만 추가한다 — tier downgrade 방어가 처음부터 계약에 있어야 한다.
 */
export const displayTierSchema = z.enum(['sample'])

export type DisplayTier = z.infer<typeof displayTierSchema>

/** 낮을수록 낮은 tier. 같은 tier는 generationSeq로만 전진한다. */
export const DISPLAY_TIER_ORDER: Record<DisplayTier, number> = {
  sample: 0,
}

export function compareDisplayTier(left: DisplayTier, right: DisplayTier) {
  return DISPLAY_TIER_ORDER[left] - DISPLAY_TIER_ORDER[right]
}

/**
 * generation이 활성 display truth가 되지 못한 이유.
 * 조용한 무시를 금지하기 위해 모든 거부 경로가 고유 코드를 가진다.
 */
export const displayRejectReasonSchema = z.enum([
  'partial-file',
  'undecodable',
  'insufficient-dimensions',
  'stale-epoch',
  'lower-generation',
  'older-request',
  'session-mismatch',
  'orientation-unsupported',
  'unknown-generation',
  'tier-downgrade',
  'viewer-not-ready',
  'decode-failed',
])

/**
 * host만 기록할 수 있는 거부 사유.
 *
 * `present-unreported`는 commit·notify까지 끝난 generation에 유예 시간 안에 viewer의 terminal
 * 보고가 도착하지 않았다는 **host의 관측**이다. viewer가 스스로 주장할 수 있는 결론이 아니므로
 * `displayRejectReasonSchema`(viewer → host)에는 넣지 않는다. 이 값이 없으면 그런 generation은
 * 계측에 아무 행도 남기지 않아 "표시되지 않았다"와 "보고가 유실됐다"를 구분할 수 없다.
 */
export const displayHostRejectReasonSchema = z.enum([
  ...displayRejectReasonSchema.options,
  'present-unreported',
])

export const displayPresentOutcomeSchema = z.enum([
  'presented',
  'rejected',
  'decode-failed',
])

/**
 * 호스트가 원자적으로 commit한 immutable display generation.
 *
 * 이 레코드가 존재한다는 것은 파일이 완전히 write·close·fsync되고, 구조 decode와 크기·correlation
 * 검증을 통과했다는 뜻이다. commit 전에는 어떤 경로로도 이 레코드가 밖으로 나가지 않는다.
 */
export const displayGenerationSchema = z.object({
  generationId: z.string().trim().min(1),
  /** 세션 안에서 단조 증가. 되돌아가지 않는다. */
  generationSeq: z.number().int().positive(),
  sessionId: sessionIdSchema,
  requestId: captureRequestIdSchema,
  captureId: captureIdSchema.nullable(),
  /** 이 generation이 검증된 viewer window 세대. epoch가 바뀌면 재검증 대상이다. */
  viewerEpoch: z.number().int().nonnegative(),
  tier: displayTierSchema,
  assetPath: z.string().trim().min(1),
  sourceWidthPx: z.number().int().positive(),
  sourceHeightPx: z.number().int().positive(),
  byteSize: z.number().int().positive(),
  /** `fnv1a64:<hex>` 형식. 보안용 해시가 아니라 provenance/상관관계 확인용이다. */
  sourceHash: z.string().trim().min(1),
  sampleVariant: z.enum(['a', 'b']),
  committedAtHostMicros: z.number().int().nonnegative(),
})

/**
 * display pointer snapshot. durable recovery 경계이며 live event보다 우선한다.
 *
 * `requiredSource*`는 Story 7.1의 viewer photoRect에서 온다. 0이면 viewer가 아직 크기를
 * 보고하지 않은 것이고, 그 상태에서는 어떤 generation도 활성화될 수 없다.
 */
export const displayPointerSnapshotSchema = z
  .object({
    schemaVersion: z
      .literal(viewerDisplaySchemaVersion)
      .default(viewerDisplaySchemaVersion),
    sessionId: sessionIdSchema.nullable(),
    /** 모든 pointer 변경마다 단조 증가. */
    revision: z.number().int().nonnegative(),
    activeGeneration: displayGenerationSchema.nullable(),
    requiredSourceWidthPx: z.number().int().nonnegative(),
    requiredSourceHeightPx: z.number().int().nonnegative(),
    /**
     * 계측 lane이 켜져 있는지. **기본은 false다.**
     *
     * true면 표시되는 이미지는 preset 적용 결과가 아니라 계측용 fixture이며,
     * 두 surface는 이 값이 true일 때만 present 계측 IPC를 수행한다.
     */
    measurementLaneEnabled: z.boolean(),
    observedAtHostMicros: z.number().int().nonnegative(),
  })
  .superRefine((pointer, context) => {
    const active = pointer.activeGeneration

    if (active === null) {
      return
    }

    if (pointer.sessionId === null || active.sessionId !== pointer.sessionId) {
      context.addIssue({
        code: 'custom',
        path: ['activeGeneration'],
        message: 'An active generation must belong to the bound session.',
      })
    }

    if (
      pointer.requiredSourceWidthPx === 0 ||
      pointer.requiredSourceHeightPx === 0 ||
      active.sourceWidthPx < pointer.requiredSourceWidthPx ||
      active.sourceHeightPx < pointer.requiredSourceHeightPx
    ) {
      context.addIssue({
        code: 'custom',
        path: ['activeGeneration'],
        message:
          'An active generation must satisfy the current physical display-fit contract.',
      })
    }
  })

export const displayUpdateSchema = z.object({
  schemaVersion: z
    .literal(viewerDisplayUpdateSchemaVersion)
    .default(viewerDisplayUpdateSchemaVersion),
  pointer: displayPointerSnapshotSchema,
})

/**
 * viewer가 관측한 span. 전부 **host monotonic clock micros로 변환된 값**이다.
 *
 * 이 중 어느 것도 KPI 종료점이 아니다. 공식 종료점은 `actualPresentAtMicros` 하나이며
 * 그마저도 물리 프레임의 소프트웨어 추정치다 (HV-13B가 물리 오프셋을 측정한다).
 */
export const displayPresentSpansSchema = z.object({
  viewerReceiptAtMicros: z.number().int().nonnegative().nullable(),
  decodeStartAtMicros: z.number().int().nonnegative().nullable(),
  decodeEndAtMicros: z.number().int().nonnegative().nullable(),
  swapCommittedAtMicros: z.number().int().nonnegative().nullable(),
  imgOnLoadAtMicros: z.number().int().nonnegative().nullable(),
  actualPresentAtMicros: z.number().int().nonnegative().nullable(),
  elementTimingRenderAtMicros: z.number().int().nonnegative().nullable(),
  /**
   * Element Timing `renderTime`이 실제 render time인지 여부.
   * asset protocol은 cross-origin이라 `Timing-Allow-Origin`이 없으면 `renderTime === 0`이 되고
   * `startTime`이 `loadTime`으로 대체된다. false면 actual-present로 승격할 수 없다.
   */
  isElementRenderTime: z.boolean(),
})

export const displayPresentReportSchema = z.object({
  generationId: z.string().trim().min(1),
  viewerEpoch: z.number().int().nonnegative(),
  outcome: displayPresentOutcomeSchema,
  rejectReason: displayRejectReasonSchema.nullable(),
  naturalWidthPx: z.number().int().nonnegative(),
  naturalHeightPx: z.number().int().nonnegative(),
  spans: displayPresentSpansSchema,
  /** viewer document의 `performance.now()` → host clock 보정값과 그 불확실도. */
  clockOffsetMicros: z.number().int(),
  clockUncertaintyMicros: z.number().int().nonnegative(),
})

/** clock 보정 왕복. host는 수신 즉시 자기 monotonic clock을 찍어 돌려준다. */
export const clockProbeSchema = z.object({
  clientSentMicros: z.number().int().nonnegative(),
})

export const clockProbeResultSchema = z.object({
  hostMonotonicMicros: z.number().int().nonnegative(),
  hostEpochMicros: z.number().int().nonnegative(),
})

/**
 * booth → host. 공식 KPI 시작점.
 *
 * `clientInputMicros`는 촬영 컨트롤의 `pointerup` 핸들러 첫 줄에서 읽은 값이다.
 * host 수신 시각을 시작점으로 쓰면 IPC 지연이 시작점 뒤로 밀려 KPI가 실제보다 빨라 보인다.
 */
export const trustedInputReportSchema = z.object({
  sessionId: sessionIdSchema,
  requestId: captureRequestIdSchema,
  clientInputMicros: z.number().int().nonnegative(),
  clockOffsetMicros: z.number().int(),
  clockUncertaintyMicros: z.number().int().nonnegative(),
  /** 합성 이벤트(`isTrusted === false`)는 qualifying 표본이 아니다. */
  isTrusted: z.boolean(),
})
