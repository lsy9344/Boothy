import { z } from 'zod'

import { captureIdSchema, captureRequestIdSchema } from './session-capture'
import { sessionIdSchema } from './ids'

/**
 * Story 7.4가 `v1` → `v2`로, Story 7.5가 `v2` → `v3`로, Story 7.6이 `v3` → `v4`로 올렸다.
 *
 * v2는 generation이 tier별로 다른 모양을 갖게 됐기 때문이고
 * (`sampleVariant`가 nullable이 되고 `proxyProvenance`가 추가됐다),
 * v3는 **프레임을 실제로 만든 renderer와 참조 renderer를 분리해서 기록**해야 하기 때문이다.
 * 상주 renderer가 만든 프레임에 `referenceRenderer: 'darktable'`만 남기면 evidence가 거짓말을 한다.
 *
 * v4는 `rawRefinedDisplay` tier가 생기면서 **generation만 보고 두 tier를 구분할 수 있어야**
 * 하기 때문이다 (`renderQuality`). 두 tier는 같은 RAW·같은 XMP·같은 renderer를 쓰고
 * `--hq`만 다르므로, 이 필드가 없으면 evidence에서 어느 쪽이 화면에 있었는지 알 수 없다.
 *
 * **읽기는 v1·v2·v3도 받는다.** `displayPointerSnapshotCompatSchema`를 참조하라.
 * 하나의 HV 회차가 빌드 경계를 걸칠 수 있고, evidence 도구가 여러 버전의
 * `pointer.json` / `generations.jsonl`을 함께 읽는다. Story 7.9의 pre-upgrade session
 * 호환과 old generation pointer 복구가 이 규칙 위에 선다.
 */
export const viewerDisplaySchemaVersion = 'viewer-display/v4' as const
export const viewerDisplaySchemaVersionV1 = 'viewer-display/v1' as const
export const viewerDisplaySchemaVersionV2 = 'viewer-display/v2' as const
export const viewerDisplaySchemaVersionV3 = 'viewer-display/v3' as const
/** 이벤트 봉투 자체의 모양은 바뀌지 않았다. 안의 pointer가 자기 버전을 들고 다닌다. */
export const viewerDisplayUpdateSchemaVersion =
  'viewer-display-update/v1' as const

/**
 * 승인된 display tier.
 *
 * Story 7.2가 `sample`을, Story 7.4가 `displayFitPresetProxy`를,
 * Story 7.6이 `rawRefinedDisplay`를 등록했다. tier가 늘어날 때 값만 추가한다 —
 * tier downgrade 방어는 처음부터 계약에 있어야 한다.
 *
 * **`final`은 display tier가 아니다.** Story 7.6이 이 판단을 확정했다:
 * - PRD FR-010이 약속한 승급은 `display-fit preset 이미지 → RAW 정밀본` 둘뿐이다
 * - `final`은 5184×3456 전체 해상도 handoff 산출물이다. 이것을 1429×953 사진 영역에 올리면
 *   축소를 **브라우저가** 하게 되어 darktable 축소보다 품질이 낮으면서 decode 비용은 훨씬 크다.
 *   "더 높은 tier"라는 주장이 화면에서 성립하지 않는다
 * - `--upscale false` 기반 display-fit 계약과 정면으로 충돌한다
 */
export const displayTierSchema = z.enum([
  'sample',
  'displayFitPresetProxy',
  'rawRefinedDisplay',
])

export type DisplayTier = z.infer<typeof displayTierSchema>

/** 낮을수록 낮은 tier. 같은 tier는 generationSeq로만 전진한다. */
export const DISPLAY_TIER_ORDER: Record<DisplayTier, number> = {
  sample: 0,
  displayFitPresetProxy: 1,
  rawRefinedDisplay: 2,
}

/**
 * proxy generation의 확정 파일 경로에 들어가는 variant 자리 문자열.
 *
 * `sampleVariant`가 `null`이어도 경로는 사람과 스크립트가 읽을 수 있어야 한다.
 * 빈 문자열을 넘기면 `000001-.jpg`가 만들어진다.
 */
export const DISPLAY_PROXY_PATH_VARIANT = 'proxy' as const

/**
 * Story 7.6. RAW 정밀본 generation의 확정 파일 경로 variant 자리 문자열.
 *
 * `<seq>-refined.jpg`가 된다. proxy와 같은 함정을 반복하지 않도록 상수로 둔다.
 */
export const DISPLAY_RAW_REFINED_PATH_VARIANT = 'refined' as const

/**
 * Story 7.6. 이 프레임을 만든 darktable pixelpipe 품질.
 *
 * `displayFitPresetProxy`와 `rawRefinedDisplay`는 **같은 RAW·같은 XMP·같은 pinned renderer·
 * 같은 목표 크기**를 쓴다. 유일한 차이가 `--hq`이므로, 이 필드가 없으면 evidence에서
 * 두 tier의 프레임을 구분할 수 없다.
 *
 * - `fast`: `--hq false`. 먼저 축소하고 처리한다 (Story 7.4 proxy lane)
 * - `high`: `--hq true`. 전체 해상도로 처리한 뒤 축소한다 (Story 7.6 정밀본 lane)
 */
export const displayRenderQualitySchema = z.enum(['fast', 'high'])

export type DisplayRenderQuality = z.infer<typeof displayRenderQualitySchema>

/** tier가 요구하는 렌더 품질. 계약이 한 곳에서 강제한다. */
export const DISPLAY_TIER_REQUIRED_RENDER_QUALITY: Partial<
  Record<DisplayTier, DisplayRenderQuality>
> = {
  displayFitPresetProxy: 'fast',
  rawRefinedDisplay: 'high',
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
  /**
   * Story 7.4. capture-bound preset identity 또는 version이 현재 활성 generation과 다르다.
   *
   * `older-request`와 뭉뚱그리면 안 된다. 세션 중 preset 변경(Story 2.3) 뒤 이전 preset의
   * 늦은 proxy가 화면을 덮는 것은 **순서 문제가 아니라 정체성 문제**이고,
   * 회차 분석에서 두 원인이 구분되어야 한다.
   */
  'preset-mismatch',
  /** Story 7.4. 같은 request 안에서 더 오래된 capture의 generation이 늦게 도착했다. */
  'older-capture',
  /**
   * Story 7.6. RAW 정밀본의 실측 픽셀 크기가 현재 활성 proxy와 정확히 같지 않다.
   *
   * **이것이 AC 4의 crop/scale 점프 0을 만드는 기계적 장치다.** 정밀본은 크기를
   * proxy generation의 provenance에서 상속하지만, 렌더 결과가 한 픽셀이라도 다르면
   * `object-fit: contain` 박스가 달라져 교체 순간 사진이 튄다.
   */
  'refined-dimension-mismatch',
  /**
   * Story 7.6. AC 6의 detail 축(MTF50) gate가 `justified`를 기록하지 않았다.
   *
   * 측정되지 않은 tier 차이는 게시하지 않는다. 없는 차이를 만들어 내면
   * 고객에게 아무 가치가 없으면서 darktable 부하만 두 배가 된다.
   *
   * **`refined-look-drift`는 여기 없다.** ΔE00은 게시 경로에서 계산할 수 없는 값이고
   * (두 raster를 픽셀 단위로 비교해야 한다), 런타임 거부가 아니라 T6 evidence의
   * AC 4 판정 값이다. 계약 enum에 넣으면 절대 발생하지 않는 코드가 매트릭스에 남는다.
   */
  'refined-tier-not-justified',
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
/**
 * Story 7.4. display-fit preset proxy generation의 출처.
 *
 * AC 1이 요구하는 결속을 **하나도 빠짐없이 자산에 실어 둔다.** 이 값들이 없으면
 * "화면에 올라간 이 사진이 정말 이 촬영·이 프리셋·이 화면 크기의 결과인가"를
 * 나중에 evidence로 증명할 수 없다.
 */
/**
 * Story 7.5. 상주 renderer 실행 mode.
 *
 * **`off`가 기본이고 알 수 없는 값도 `off`다.** HV-16이 `Go`를 기록해도
 * Story 7.6 전에는 production 기본값을 바꾸지 않는다.
 */
export const residentRendererModeSchema = z.enum(['off', 'shadow', 'evidence'])

export type ResidentRendererMode = z.infer<typeof residentRendererModeSchema>

export function parseResidentRendererMode(raw: unknown): ResidentRendererMode {
  const parsed = residentRendererModeSchema.safeParse(
    typeof raw === 'string' ? raw.trim() : raw,
  )

  return parsed.success ? parsed.data : 'off'
}

/**
 * 상주 후보가 무엇을 입력으로 받았는지.
 *
 * - `predecoded-fixture`: 오프라인에서 미리 디코드한 raster. **engine feasibility 자료 전용**
 * - `real-capture-direct`: 상주 프로세스가 실제 촬영 원본을 직접 읽었다
 * - `real-capture-via-one-shot`: 실제 촬영이지만 별도 one-shot process가 raster를 먼저 만들었다.
 *   그 startup 비용은 hot path에 그대로 남아 있다
 */
export const residentInputProvenanceSchema = z.enum([
  'predecoded-fixture',
  'real-capture-direct',
  'real-capture-via-one-shot',
])

/**
 * **현재 승인된 direct real-capture decoder는 하나도 없다.**
 *
 * Story 7.3(HV-14)이 embedded JPEG / paired JPEG / Windows shell thumbnail을 전부
 * `Technology No-Go`로 닫았고 그 뒤 승인된 대체 route가 없다. 이 목록이 비어 있는 한
 * 어떤 상주 후보도 `productionEligible`을 주장할 수 없다 (Story 7.5 AC 6).
 */
export const RESIDENT_APPROVED_DIRECT_DECODERS: readonly string[] = []

/**
 * Story 7.5. 상주 renderer가 만든 generation의 producer 신원.
 *
 * `referenceRenderer`와 **다른 질문에 답한다.** 참조 renderer는 "무엇과 비교해서
 * 정확한가"이고, 이 블록은 "누가 이 픽셀을 만들었는가"다. 두 값을 하나로 합치면
 * evidence에서 darktable이 만든 프레임과 다른 엔진이 만든 프레임을 구분할 수 없다.
 */
export const residentProducerProvenanceSchema = z
  .object({
    producerRenderer: z.string().trim().min(1),
    producerRendererVersion: z.string().trim().min(1),
    producerBuildId: z.string().trim().min(1),
    /** `off`는 렌더 자체를 하지 않으므로 게시된 프레임에 나타날 수 없다. */
    executionMode: z.enum(['shadow', 'evidence']),
    recipeSchemaVersion: z.string().trim().min(1),
    compiledRecipeHash: z.string().trim().min(1),
    programHash: z.string().trim().min(1),
    /** 상주 context가 만들어진 시각. 촬영보다 앞서야 한다. */
    contextInitializedAtMicros: z.number().int().nonnegative(),
    /** 측정 대상 hot path의 shader/program compile 횟수. 0이 아니면 AC 1 실패다. */
    hotPathProgramCompileCount: z.number().int().nonnegative(),
    /** 측정 대상 hot path의 renderer process 시작 횟수. 0이 아니면 AC 1 실패다. */
    hotPathProcessStartCount: z.number().int().nonnegative(),
    inputProvenance: residentInputProvenanceSchema,
    inputProducer: z.string().trim().min(1),
    inputProducerVersion: z.string().trim().min(1),
    /**
     * 입력 raster가 상주 renderer에게 **읽을 수 있는 상태로 도착한** 시각.
     *
     * 지연 구간의 시작점이다. 이 값이 없으면 "렌더러가 빠르다"와
     * "촬영부터 화면까지 빠르다"를 분리해서 볼 수 없다.
     */
    sourceReadyAtMicros: z.number().int().nonnegative(),
    /** 입력 raster를 만든 one-shot process의 startup 비용. 숨기지 않고 여기 남긴다. */
    inputStartupCostMicros: z.number().int().nonnegative(),
    /** production 채택 자격. spike에서는 승인된 direct decoder가 없어 항상 false다. */
    productionEligible: z.boolean(),
    adoptionBlockReason: z.string().trim().min(1).nullable(),
    gpuVendor: z.string(),
    gpuRenderer: z.string(),
    fallbackReason: z.string().trim().min(1).nullable(),
  })
  .superRefine((provenance, context) => {
    if (!provenance.productionEligible) {
      // 이유 없는 강등은 evidence에서 조용한 무시와 구분되지 않는다.
      if (provenance.adoptionBlockReason === null) {
        context.addIssue({
          code: 'custom',
          path: ['adoptionBlockReason'],
          message: 'A non-eligible resident frame must carry its block reason.',
        })
      }

      return
    }

    // fixture로 얻은 빠른 결과에 production 자격을 붙이는 것이
    // 이 Story에서 가장 쉬운 자기기만이다. 계약 경계에서 막는다.
    if (provenance.inputProvenance !== 'real-capture-direct') {
      context.addIssue({
        code: 'custom',
        path: ['inputProvenance'],
        message:
          'Production eligibility requires a direct real-capture input.',
      })
    }

    if (!RESIDENT_APPROVED_DIRECT_DECODERS.includes(provenance.inputProducer)) {
      context.addIssue({
        code: 'custom',
        path: ['inputProducer'],
        message: 'The input decoder is not on the approved direct decoder list.',
      })
    }

    if (
      provenance.hotPathProgramCompileCount > 0 ||
      provenance.hotPathProcessStartCount > 0
    ) {
      context.addIssue({
        code: 'custom',
        path: ['hotPathProgramCompileCount'],
        message: 'Production eligibility requires an empty hot path.',
      })
    }

    if (provenance.adoptionBlockReason !== null) {
      context.addIssue({
        code: 'custom',
        path: ['adoptionBlockReason'],
        message: 'An eligible resident frame cannot also carry a block reason.',
      })
    }
  })

export const displayProxyProvenanceSchema = z.object({
  /** capture record에 고정된 preset identity. live catalog pointer가 아니다. */
  presetId: z.string().trim().min(1),
  presetVersion: z.string().trim().min(1),
  /**
   * 이 proxy가 proxy lane에 들어간 **근거**.
   *
   * - `exact-reference-renderer`: 참조 렌더러(darktable)가 capture-bound XMP로 RAW 원본을
   *   렌더했다. 오늘의 정확 경로와 엔진·recipe·source가 같고 출력 크기만 화면에 맞춘 것이라
   *   **근사가 아니다.** 별도 시각 승인 대상이 아니다
   * - `visual-approval`: source나 엔진이 정확 경로와 달라 결과가 달라질 수 있고,
   *   게시 시점의 시각 승인 증거로 자격을 얻었다
   *
   * evidence에서 두 종류의 프레임은 반드시 구분되어야 한다.
   */
  approvalBasis: z.enum(['exact-reference-renderer', 'visual-approval']),
  /** 승인된 versioned proxy recipe. 정확 경로에서는 번들의 게시 버전이다. */
  proxyRecipeVersion: z.string().trim().min(1),
  referenceRenderer: z.string().trim().min(1),
  referenceRendererVersion: z.string().trim().min(1),
  renderProfileId: z.string().trim().min(1),
  outputColorSpace: z.string().trim().min(1),
  jpegQuality: z.number().int().min(1).max(100),
  /** 이 proxy가 어느 source에서 렌더됐는지. Story 7.3의 route 결정과 연결된다. */
  sourceRoute: z.string().trim().min(1),
  /**
   * **입력** source의 내용 해시. generation의 `sourceHash`는 게시된 결과 파일의 해시라
   * 서로 다른 값이며 둘 다 필요하다.
   */
  sourceAssetHash: z.string().trim().min(1),
  /** 렌더 요청 시점의 목표 크기. viewer photoRect × DPR에서 나온다. */
  targetWidthPx: z.number().int().positive(),
  targetHeightPx: z.number().int().positive(),
  displayProfileId: z.string().trim().min(1),
  devicePixelRatio: z.number().positive(),
  /**
   * Story 7.5. **이 프레임을 실제로 만든 renderer.**
   * `null`이면 참조 렌더러(darktable)가 직접 만들었다는 뜻이다.
   */
  residentProvenance: residentProducerProvenanceSchema.nullable(),
  /**
   * Story 7.6. 이 프레임을 만든 darktable pixelpipe 품질.
   *
   * proxy는 `fast`, 정밀본은 `high`다. **generation만 보고 두 tier를 구분할 수 있어야 한다.**
   * 두 tier가 같은 source·같은 recipe·같은 크기를 쓰므로 다른 필드로는 구분되지 않는다.
   */
  renderQuality: displayRenderQualitySchema,
})

export const displayGenerationSchema = z
  .object({
    generationId: z.string().trim().min(1),
    /** 세션 안에서 단조 증가. 되돌아가지 않는다. */
    generationSeq: z.number().int().positive(),
    /** host가 request를 처음 관측한 순서. 이전 pointer에서는 생략될 수 있다. */
    requestOrder: z.number().int().nonnegative().nullable().optional(),
    /** 촬영 확정 시점 순서. 렌더 완료 시각이 아니라 이 값으로 역순을 판정한다. */
    captureOrder: z.number().int().nonnegative().nullable().optional(),
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
    /** 계측 fixture에서만 의미가 있다. proxy generation에서는 `null`이다. */
    sampleVariant: z.enum(['a', 'b']).nullable(),
    /** proxy generation에서만 존재한다. sample generation에서는 `null`이다. */
    proxyProvenance: displayProxyProvenanceSchema.nullable(),
    committedAtHostMicros: z.number().int().nonnegative(),
  })
  .superRefine((generation, context) => {
    // tier마다 필요한 필드가 다르다. 비어 있어 기본값으로 통과하는 경로를 만들지 않는다.
    if (generation.tier === 'sample') {
      if (generation.sampleVariant === null) {
        context.addIssue({
          code: 'custom',
          path: ['sampleVariant'],
          message: 'A sample generation must record its fixture variant.',
        })
      }

      if (generation.proxyProvenance !== null) {
        context.addIssue({
          code: 'custom',
          path: ['proxyProvenance'],
          message: 'A sample generation has no preset proxy provenance.',
        })
      }

      return
    }

    if (generation.proxyProvenance === null) {
      context.addIssue({
        code: 'custom',
        path: ['proxyProvenance'],
        message: 'A display-fit preset proxy must record its provenance.',
      })

      return
    }

    if (generation.sampleVariant !== null) {
      context.addIssue({
        code: 'custom',
        path: ['sampleVariant'],
        message: 'A display-fit preset proxy is not a measurement fixture.',
      })
    }

    // Story 7.6. tier와 렌더 품질이 어긋난 generation은 evidence에서 거짓말을 한다.
    // `--hq false`로 만든 프레임이 `rawRefinedDisplay`로 게시되면 "정밀본이 떴다"가 되고,
    // AC 6의 tier 정당성 측정 자체가 무의미해진다.
    const requiredQuality =
      DISPLAY_TIER_REQUIRED_RENDER_QUALITY[generation.tier]

    if (
      requiredQuality !== undefined &&
      generation.proxyProvenance.renderQuality !== requiredQuality
    ) {
      context.addIssue({
        code: 'custom',
        path: ['proxyProvenance', 'renderQuality'],
        message: `A ${generation.tier} generation must record renderQuality '${requiredQuality}'.`,
      })
    }
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
     * **표시되는 이미지가 계측용 fixture인지.** 기본은 false다.
     *
     * Story 7.2의 sample lane 상태 그대로이며, Story 7.4의 proxy lane은 이 값을 켜지 않는다 —
     * proxy는 실제 제품 결과이지 fixture가 아니기 때문이다.
     * 계측 IPC 여부를 이 값으로 판단하지 마라. 그건 `presentTelemetryEnabled`다.
     */
    measurementLaneEnabled: z.boolean(),
    /**
     * **두 surface가 present 계측 IPC를 수행해야 하는지.** `sample lane || proxy lane`이다.
     *
     * Story 7.4가 분리했다. 이전에는 booth/viewer가 `measurementLaneEnabled`로 계측을
     * gate했는데, proxy lane만 켜면 그 값이 false라 **actual-present 행이 0건**이 된다.
     * HV-13B 1차가 정확히 그 실패 모드였다.
     */
    presentTelemetryEnabled: z.boolean(),
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
      (active.sourceWidthPx < pointer.requiredSourceWidthPx &&
        active.sourceHeightPx < pointer.requiredSourceHeightPx)
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
 * durable generation journal row(v1 / v2 / v3 / v4)를 현재 generation 모양으로 정규화한다.
 * journal row에는 schemaVersion이 없으므로 compat reader에서만 사용한다.
 */
export function normalizeDisplayGenerationInput(raw: unknown): unknown {
  if (raw === null || typeof raw !== 'object' || Array.isArray(raw)) {
    return raw
  }

  const normalized = { ...(raw as Record<string, unknown>) }

  // v1은 sample generation만 만들 수 있었다.
  normalized.tier ??= 'sample'
  normalized.requestOrder ??= null
  normalized.captureOrder ??= null
  normalized.proxyProvenance ??= null
  normalized.sampleVariant ??= null

  const proxy = normalized.proxyProvenance

  if (proxy !== null && typeof proxy === 'object' && !Array.isArray(proxy)) {
    const normalizedProxy = { ...(proxy as Record<string, unknown>) }
    normalizedProxy.residentProvenance ??= null

    // v2/v3에는 proxy lane 하나만 있었고 그 lane은 항상 `--hq false`였다.
    // 새 raw refined tier에는 이 추론을 적용하지 않아 malformed v4 행을 숨기지 않는다.
    if (normalized.tier === 'displayFitPresetProxy') {
      normalizedProxy.renderQuality ??= 'fast'
    }

    normalized.proxyProvenance = normalizedProxy
  }

  return normalized
}

/** durable generations.jsonl 전용 reader. 결과는 항상 현재 v4 generation이다. */
export const displayGenerationCompatSchema = z.preprocess(
  normalizeDisplayGenerationInput,
  displayGenerationSchema,
)

/**
 * durable `pointer.json`(v1 / v2 / v3 / v4)을 현재 모양으로 정규화한다.
 *
 * 한 HV 회차가 빌드 경계를 걸칠 수 있고, evidence 도구는 여러 버전이 섞인 디렉터리를 읽는다.
 * 정규화 없이 파싱을 실패시키면 **오래된 표본이 조용히 분모에서 사라진다.**
 * Story 7.9의 pre-upgrade session 호환과 old generation pointer 복구도 이 함수 위에 선다.
 */
export function normalizeDisplayPointerSnapshotInput(raw: unknown): unknown {
  if (raw === null || typeof raw !== 'object' || Array.isArray(raw)) {
    return raw
  }

  const pointer = { ...(raw as Record<string, unknown>) }

  if (
    pointer.schemaVersion === viewerDisplaySchemaVersionV1 ||
    pointer.schemaVersion === viewerDisplaySchemaVersionV2 ||
    pointer.schemaVersion === viewerDisplaySchemaVersionV3
  ) {
    pointer.schemaVersion = viewerDisplaySchemaVersion
  }

  if (pointer.presentTelemetryEnabled === undefined) {
    pointer.presentTelemetryEnabled = pointer.measurementLaneEnabled === true
  }

  if (pointer.activeGeneration !== null) {
    pointer.activeGeneration = normalizeDisplayGenerationInput(
      pointer.activeGeneration,
    )
  }

  return pointer
}

/**
 * durable pointer 파일 전용 reader. 쓰기는 항상 `displayPointerSnapshotSchema`(v4)로 한다.
 */
export const displayPointerSnapshotCompatSchema = z.preprocess(
  normalizeDisplayPointerSnapshotInput,
  displayPointerSnapshotSchema,
)

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

export const displayPresentReportSchema = z
  .object({
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
  .superRefine((report, context) => {
    if (
      report.outcome === 'presented' &&
      report.spans.actualPresentAtMicros === null
    ) {
      context.addIssue({
        code: 'custom',
        path: ['spans', 'actualPresentAtMicros'],
        message: 'A presented report requires an actual present timestamp.',
      })
    }

    if (report.outcome === 'presented' && report.rejectReason !== null) {
      context.addIssue({
        code: 'custom',
        path: ['rejectReason'],
        message: 'A presented report cannot carry a reject reason.',
      })
    }

    if (report.outcome !== 'presented' && report.rejectReason === null) {
      context.addIssue({
        code: 'custom',
        path: ['rejectReason'],
        message: 'A rejected report requires a reject reason.',
      })
    }

    if (
      report.outcome === 'decode-failed' &&
      report.rejectReason !== 'decode-failed'
    ) {
      context.addIssue({
        code: 'custom',
        path: ['rejectReason'],
        message: 'A decode-failed report must carry the decode-failed reason.',
      })
    }

    if (
      report.outcome === 'rejected' &&
      report.rejectReason === 'decode-failed'
    ) {
      context.addIssue({
        code: 'custom',
        path: ['rejectReason'],
        message: 'A rejected report cannot carry the decode-failed reason.',
      })
    }

    if (
      report.outcome === 'presented' &&
      (report.naturalWidthPx === 0 || report.naturalHeightPx === 0)
    ) {
      context.addIssue({
        code: 'custom',
        path: ['naturalWidthPx'],
        message: 'A presented report requires positive natural dimensions.',
      })
    }
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
