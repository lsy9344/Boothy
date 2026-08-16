import { z } from 'zod'

import { captureIdSchema, captureRequestIdSchema } from './session-capture'
import { sessionIdSchema } from './ids'

export const captureSourceSchemaVersion = 'capture-source/v1' as const
export const sourceComparisonSchemaVersion = 'source-comparison/v1' as const

/**
 * 같은 촬영의 JPEG source를 얻는 경로.
 *
 * **incumbent(`windows-shell-thumbnail`)가 1급 값으로 등록되어 있다.** 기준선이 계약에
 * 없으면 "새 route가 더 빠르다"는 주장에 비교 대상이 없다. Story 7.3은 세 route를 같은
 * 조건에서 재고, 그 결과로만 primary/fallback/No-Go를 정한다.
 *
 * - `embedded-jpeg` — RAW 컨테이너에 내장된 full-size JPEG을 추출한다 (Route A).
 *   CR2의 TIFF IFD를 직접 읽으며 **새 의존성을 쓰지 않는다.**
 * - `camera-paired-jpeg` — 카메라가 RAW와 함께 만든 별도 JPEG object를 받는다 (Route B).
 * - `windows-shell-thumbnail` — 현재 제품에 살아 있는 incumbent (Route C, 기준선).
 */
export const sourceRouteSchema = z.enum([
  'embedded-jpeg',
  'camera-paired-jpeg',
  'windows-shell-thumbnail',
])

/**
 * source 후보가 승격되지 못한 이유.
 *
 * `displayRejectReasonSchema`와 같은 원칙이다 — 모든 거부 경로가 고유 코드를 가진다.
 * 조용한 무시를 허용하면 성공률 분모가 소리 없이 줄어 결과가 실제보다 좋아 보인다.
 */
export const sourceRejectReasonSchema = z.enum([
  'absent',
  'partial',
  'corrupt',
  'undecodable',
  'orientation-unsupported',
  'wrong-session',
  'wrong-request',
  'wrong-capture',
  'stale',
  'unsupported-combination',
  'extraction-failed',
  'cancelled',
])

/** transfer object가 RAW original인지 부가 JPEG인지. 확장자가 아니라 SDK 정보로 판정한다. */
export const sourceObjectRoleSchema = z.enum(['raw', 'jpeg'])

/**
 * 한 route가 만들어 낸 source 후보 1건.
 *
 * `assetPath`가 null이면 후보 자체가 생기지 않은 것이고, 그때도 표본 행은 남는다.
 * **행이 없는 시도를 만들지 않는다** — Story 7.2가 이 결함으로 한 회차를 잃었다.
 */
export const sourceCandidateSchema = z
  .object({
    captureId: captureIdSchema.nullable(),
    requestId: captureRequestIdSchema,
    sessionId: sessionIdSchema,
    route: sourceRouteSchema,
    assetPath: z.string().trim().min(1).nullable(),
    widthPx: z.number().int().nonnegative().nullable(),
    heightPx: z.number().int().nonnegative().nullable(),
    byteSize: z.number().int().nonnegative().nullable(),
    exifOrientation: z.number().int().min(1).max(8).nullable(),
    decodeValid: z.boolean(),
    /** `fnv1a64:<16자리 소문자 hex>`. 보안 해시가 아니라 provenance/상관관계 확인용이다. */
    sourceHash: z.string().regex(/^fnv1a64:[0-9a-f]{16}$/).nullable(),
    /** 같은 촬영 안에서 이 transfer object가 몇 번째로 도착했는가. 0부터. */
    objectIndex: z.number().int().nonnegative().nullable(),
    /** `EdsDirectoryItemInfo.groupID`. 없으면 보조 correlation을 쓰고 그 사실을 남긴다. */
    groupId: z.number().int().nonnegative().nullable(),
    readyAtHostMicros: z.number().int().nonnegative().nullable(),
    extractionCostMicros: z.number().int().nonnegative().nullable(),
  })
  .superRefine((candidate, context) => {
    const measurements = [
      candidate.widthPx,
      candidate.heightPx,
      candidate.byteSize,
      candidate.objectIndex,
    ]
    const hasNoMeasurements = measurements.every((value) => value === null)

    if (candidate.assetPath === null && !hasNoMeasurements) {
      context.addIssue({
        code: 'custom',
        path: ['assetPath'],
        message: 'assetPath와 source 계측값의 존재 여부가 일치해야 합니다.',
      })
    }

    if (
      candidate.assetPath === null &&
      (candidate.decodeValid || candidate.sourceHash !== null)
    ) {
      context.addIssue({
        code: 'custom',
        path: ['decodeValid'],
        message: '없는 source는 decode 결과나 hash를 가질 수 없습니다.',
      })
    }
  })

/**
 * helper → host. 카메라가 스스로 보고한 image-quality capability.
 *
 * **지원 여부를 가정하지 않는다.** `EdsGetPropertyDesc`의 목록이 유일한 truth이며,
 * 목록에 없는 조합은 시도조차 하지 않고 `unsupported-combination`으로 기록한다.
 * 시도해서 실패한 것과 지원되지 않아 시도하지 않은 것은 다른 결과다.
 */
export const imageQualityCapabilitySchema = z
  .object({
    descriptorAvailable: z.boolean(),
    currentValue: z.number().int().nullable(),
    supportedValues: z.array(z.number().int()),
    rawPlusJpegSupported: z.boolean(),
    probedAtHostMicros: z.number().int().nonnegative(),
  })
  .superRefine((capability, context) => {
    if (!capability.descriptorAvailable && capability.rawPlusJpegSupported) {
      context.addIssue({
        code: 'custom',
        path: ['rawPlusJpegSupported'],
        message: 'descriptor 없이 RAW+JPEG 지원을 확정할 수 없습니다.',
      })
    }
  })

/** AB/BA block에서 이 표본이 속한 순서. host가 정하고 기록한다. */
export const sourceBlockOrderSchema = z.enum(['AB', 'BA'])

/**
 * `source-comparison.jsonl` 한 행.
 *
 * **Story 7.2의 `viewer-present.jsonl`과 의도적으로 분리된 파일이다.** 두 파일을 합쳐
 * 집계하는 도구를 만들지 않는다 — 합치는 순간 보정되지 않은 JPEG의 빠른 도착 시각이
 * preset-applied KPI를 실제보다 좋아 보이게 만든다.
 */
export const sourceComparisonSampleSchema = z
  .object({
    schemaVersion: z
      .literal(sourceComparisonSchemaVersion)
      .default(sourceComparisonSchemaVersion),
    candidate: sourceCandidateSchema,
    accepted: z.boolean(),
    rejectReason: sourceRejectReasonSchema.nullable(),
    objectRole: sourceObjectRoleSchema.nullable(),
  /**
   * `groupID`가 없어 파일명 stem + 도착 시각 창으로 묶었는가.
   * true인 표본은 correlation 근거가 약하므로 결론에서 그대로 드러내야 한다.
   */
    usedFallbackCorrelation: z.boolean(),
    blockOrder: sourceBlockOrderSchema,
    blockIndex: z.number().int().nonnegative(),
    isWarmUp: z.boolean(),
  /** AB/BA 배치를 재현하기 위한 seed. 사람이 순서를 고르면 편향이 들어간다. */
    randomizationSeed: z.number().int().nonnegative(),
  /**
   * **항상 false다.** 이 lane의 어떤 산출물도 preset이 적용되지 않았다.
   *
   * 필드를 두는 이유는 기본값으로 조용히 통과하는 경로를 없애기 위해서다. FR-010의
   * qualifying frame은 "같은 capture의 **preset이 적용된** 이미지"이며 Story 7.3은
   * 그 정의를 건드리지 않는다.
   */
    isPresetApplied: z.literal(false),
    recordedAtHostMicros: z.number().int().nonnegative(),
  })
  .superRefine((sample, context) => {
    if (sample.accepted !== (sample.rejectReason === null)) {
      context.addIssue({
        code: 'custom',
        path: ['rejectReason'],
        message: '승격 결과와 거부 사유가 일치해야 합니다.',
      })
    }

    if (sample.accepted && sample.candidate.sourceHash === null) {
      context.addIssue({
        code: 'custom',
        path: ['candidate', 'sourceHash'],
        message: '승격된 source는 provenance hash를 가져야 합니다.',
      })
    }

    if (sample.accepted && sample.objectRole !== 'jpeg') {
      context.addIssue({
        code: 'custom',
        path: ['objectRole'],
        message: '승격된 비교 source의 object 역할은 JPEG여야 합니다.',
      })
    }

    if (
      sample.usedFallbackCorrelation &&
      sample.candidate.route !== 'camera-paired-jpeg'
    ) {
      context.addIssue({
        code: 'custom',
        path: ['usedFallbackCorrelation'],
        message: 'fallback correlation은 paired JPEG route에서만 유효합니다.',
      })
    }
  })

/**
 * 측정 lane 스위치. **기본은 off이고, off일 때 제품 경로는 지금과 완전히 동일하다.**
 *
 * Story 7.2의 `BOOTHY_DISPLAY_SAMPLE_MODE`와 독립적인 스위치다. 두 lane을 동시에 켜면
 * 표본이 서로 오염되므로, 동시 활성화 시 경고를 남기고 source lane을 우선한다.
 */
export const sourceCompareModeSchema = z.enum([
  'off',
  'embedded',
  'paired',
  'shell',
  'ab',
])
