import { z } from 'zod'

/**
 * Story 7.7 — `release-inventory/v1` / `install-self-check/v1`.
 *
 * **인벤토리는 문장이 아니라 대조 가능한 파일이다.** 사람이 유지하는 표는 다음 빌드에서
 * 조용히 거짓이 되므로, 무엇이 들어갔는지를 기계가 읽을 수 있는 형태로만 적는다.
 *
 * 이 계약은 **없는 것을 없다고 적을 수 있어야** 성립한다. 비활성 후보를 빈 문자열이나
 * 누락으로 표현하면 다음 회차가 "왜 비었지"를 다시 묻는다. 그래서 `status`가 네 값을 갖고,
 * `present`가 아닌 모든 값은 `rationale`을 **요구**한다.
 */

const sha256Pattern = /^[0-9a-f]{64}$/

const nonEmptyText = (max: number) => z.string().trim().min(1).max(max)

export const releaseInventorySchemaVersion = 'release-inventory/v1' as const
export const installSelfCheckSchemaVersion = 'install-self-check/v1' as const

/** 설치본이 담아야 하는 구성요소 종류. AC 1이 열거한 항목과 1:1로 대응한다. */
export const releaseInventoryRoleSchema = z.enum([
  'app',
  'camera-helper',
  'edsdk-runtime',
  'source-adapter',
  'display-renderer',
  'color-profile',
  'proxy-recipes',
  'raw-renderer',
  'webview2-runtime',
])

/**
 * - `present`: 별도 페이로드가 staged 트리에 실제로 있다. 해시가 있어야 한다.
 * - `embedded`: 별도 페이로드가 없고 앱 바이너리 안에 있다 (proxy recipe XMP, source adapter).
 * - `not-applicable`: 후보가 비활성이다 (HV-16 `Technology No-Go`의 상주 renderer).
 * - `missing`: 있어야 하는데 없다. **릴리스를 막는 상태다.**
 */
export const releaseInventoryComponentStatusSchema = z.enum([
  'present',
  'embedded',
  'not-applicable',
  'missing',
])

export const releaseSigningStatusSchema = z.enum(['signed', 'unsigned', 'not-applicable'])

export const sha256DigestSchema = z
  .string()
  .trim()
  .toLowerCase()
  .regex(sha256Pattern, 'sha256 해시 형식이 아니에요.')

/**
 * 어떤 파일이 이 구성요소의 것인가.
 *
 * **설치된 앱이 인벤토리만 보고 해시를 다시 계산할 수 있어야 한다.** 명세는 설치본에
 * 들어가지 않으므로, 파일 선택 규칙이 인벤토리 안에 있어야 한다. glob은 쓰지 않는다 —
 * 무엇이 담겼는지가 문서를 읽는 것만으로 결정되어야 한다.
 */
export const componentEntrySelectionSchema = z.discriminatedUnion('kind', [
  z.object({ kind: z.literal('all') }),
  z.object({
    kind: z.literal('only'),
    entries: z.array(nonEmptyText(400)).min(1).max(4096),
  }),
  z.object({
    kind: z.literal('allExcept'),
    entries: z.array(nonEmptyText(400)).min(1).max(4096),
  }),
])

export const releaseInventoryComponentSchema = z
  .object({
    name: nonEmptyText(80),
    role: releaseInventoryRoleSchema,
    status: releaseInventoryComponentStatusSchema,
    version: nonEmptyText(60).nullable(),
    origin: nonEmptyText(160),
    entrySelection: componentEntrySelectionSchema.nullable(),
    /** AC 1의 licensing 증거는 별도 문서가 아니라 **인벤토리 자체의 일부**다. */
    license: nonEmptyText(120),
    licenseEvidencePath: nonEmptyText(240).nullable(),
    sourceArchiveSha256: sha256DigestSchema.nullable(),
    stagedTreeDigest: sha256DigestSchema.nullable(),
    installRelativePath: nonEmptyText(240).nullable(),
    signingStatus: releaseSigningStatusSchema,
    rationale: nonEmptyText(400).nullable(),
    fileCount: z.number().int().min(0).nullable(),
    totalBytes: z.number().int().min(0).nullable(),
  })
  .superRefine((value, context) => {
    if (value.status === 'present') {
      if (value.stagedTreeDigest === null) {
        context.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'staged 구성요소는 트리 해시가 있어야 해요.',
          path: ['stagedTreeDigest'],
        })
      }

      if (value.installRelativePath === null) {
        context.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'staged 구성요소는 설치 경로가 있어야 해요.',
          path: ['installRelativePath'],
        })
      }

      if (value.licenseEvidencePath === null) {
        context.addIssue({
          code: z.ZodIssueCode.custom,
          message: '동봉하는 구성요소는 라이선스 증거 경로가 있어야 해요.',
          path: ['licenseEvidencePath'],
        })
      }

      if (value.version === null) {
        context.addIssue({
          code: z.ZodIssueCode.custom,
          message: '동봉하는 구성요소는 버전이 있어야 해요.',
          path: ['version'],
        })
      }

      if (value.fileCount === null || value.totalBytes === null) {
        context.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'staged 구성요소는 파일 수와 크기를 함께 적어야 해요.',
          path: ['fileCount'],
        })
      }

      if (value.entrySelection === null) {
        context.addIssue({
          code: z.ZodIssueCode.custom,
          message: '설치된 앱이 해시를 다시 계산하려면 파일 선택 규칙이 있어야 해요.',
          path: ['entrySelection'],
        })
      }
    }

    if (value.status !== 'present' && value.entrySelection !== null) {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        message: '동봉하지 않은 구성요소에 파일 선택 규칙을 적을 수 없어요.',
        path: ['entrySelection'],
      })
    }

    if (value.status !== 'present' && value.rationale === null) {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        message: '동봉하지 않은 구성요소는 그 이유를 함께 적어야 해요.',
        path: ['rationale'],
      })
    }

    if (value.status !== 'present' && value.stagedTreeDigest !== null) {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        message: '동봉하지 않은 구성요소에 트리 해시를 적을 수 없어요.',
        path: ['stagedTreeDigest'],
      })
    }

    if (value.status === 'embedded' && value.installRelativePath === null) {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        message: '앱 바이너리에 내장된 구성요소도 어느 실행 파일에 있는지 적어야 해요.',
        path: ['installRelativePath'],
      })
    }
  })

export const releaseInstallerIdentitySchema = z.object({
  fileName: nonEmptyText(160),
  sha256: sha256DigestSchema,
  sizeBytes: z.number().int().min(1),
})

/**
 * staged 인벤토리.
 *
 * `installer`는 **봉인 단계에서만** 채워진다. 설치본 안에 동봉되는 사본은 자기 자신의 해시를
 * 담을 수 없으므로 `null`이다. 그 한계를 감추지 않고 계약에 적는다.
 */
export const stagedReleaseInventorySchema = z
  .object({
    schemaVersion: z.literal(releaseInventorySchemaVersion),
    staging: z.literal('staged'),
    generatedAt: z.string().datetime(),
    appVersion: nonEmptyText(40),
    identifier: nonEmptyText(120),
    installerFileName: nonEmptyText(160),
    signingStatus: releaseSigningStatusSchema,
    installer: releaseInstallerIdentitySchema.nullable(),
    components: z.array(releaseInventoryComponentSchema).min(1).max(32),
  })
  .superRefine((value, context) => {
    const roles = value.components.map((component) => component.role)
    const uniqueRoles = new Set(roles)

    if (uniqueRoles.size !== roles.length) {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        message: '같은 구성요소 종류를 두 번 적을 수 없어요.',
        path: ['components'],
      })
    }

    for (const role of releaseInventoryRoleSchema.options) {
      if (!uniqueRoles.has(role)) {
        context.addIssue({
          code: z.ZodIssueCode.custom,
          message: `인벤토리에 \`${role}\` 항목이 없어요.`,
          path: ['components'],
        })
      }
    }
  })

/**
 * staged 되지 않은 인벤토리.
 *
 * **자리를 비워 두지 않고 결손을 스스로 신고한다.** 이 문서를 담은 빌드는 release candidate가
 * 아니며, `release:verify`와 `--self-check`가 각각 막는다.
 */
export const notStagedReleaseInventorySchema = z.object({
  schemaVersion: z.literal(releaseInventorySchemaVersion),
  staging: z.literal('not-staged'),
  reason: nonEmptyText(400),
})

export const releaseInventorySchema = z.discriminatedUnion('staging', [
  stagedReleaseInventorySchema,
  notStagedReleaseInventorySchema,
])

/**
 * 인벤토리 대조 실패 사유.
 *
 * **한 덩어리 "검증 실패"로 뭉치지 않는다.** 운영자가 무엇을 해야 할지 사유마다 다르다.
 */
export const releaseInventoryReasonCodeSchema = z.enum([
  'inventory-not-staged',
  'inventory-component-missing',
  'inventory-digest-mismatch',
  'inventory-unexpected-component',
  'digest-tool-unavailable',
  'darktable-not-bundled',
  'darktable-tree-incomplete',
  'helper-binary-missing',
  'helper-version-check-failed',
  'webview2-runtime-unreadable',
  'inventory-manifest-unreadable',
  'session-manifest-unreadable',
])

export const installSelfCheckComponentStatusSchema = z.enum(['pass', 'fail', 'skipped'])

export const installSelfCheckComponentSchema = z
  .object({
    name: nonEmptyText(80),
    expectedDigest: sha256DigestSchema.nullable(),
    actualDigest: sha256DigestSchema.nullable(),
    status: installSelfCheckComponentStatusSchema,
    reasonCode: releaseInventoryReasonCodeSchema.nullable(),
  })
  .superRefine((value, context) => {
    if (value.status === 'fail' && value.reasonCode === null) {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        message: '실패 항목은 사유 코드가 있어야 해요.',
        path: ['reasonCode'],
      })
    }

    if (value.status === 'pass' && value.reasonCode !== null) {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        message: '통과 항목에는 사유 코드를 적지 않아요.',
        path: ['reasonCode'],
      })
    }
  })

export const darktableResolutionSchema = z.object({
  binary: nonEmptyText(400),
  /** `bundled-resource`가 아니면 **그 회차는 핀이 깨진 회차다.** */
  source: z.enum([
    'env-override',
    'bundled-resource',
    'program-files-bin',
    'program-w6432-bin',
    'localappdata-programs-bin',
    'path',
  ]),
})

export const installSelfCheckOverallSchema = z.enum(['pass', 'fail'])

export const installSelfCheckReportSchema = z
  .object({
    schemaVersion: z.literal(installSelfCheckSchemaVersion),
    checkedAt: z.string().datetime(),
    appVersion: nonEmptyText(40),
    identifier: nonEmptyText(120),
    installRoot: nonEmptyText(400),
    components: z.array(installSelfCheckComponentSchema).min(1).max(64),
    webview2Version: nonEmptyText(60).nullable(),
    darktableResolution: darktableResolutionSchema.nullable(),
    helperVersion: nonEmptyText(120).nullable(),
    overall: installSelfCheckOverallSchema,
  })
  .superRefine((value, context) => {
    const hasFailure = value.components.some((component) => component.status === 'fail')

    if (hasFailure && value.overall === 'pass') {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        message: '실패 항목이 있으면 전체 결과를 통과로 적을 수 없어요.',
        path: ['overall'],
      })
    }

    if (!hasFailure && value.overall === 'fail') {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        message: '실패 항목이 없는데 전체 결과만 실패로 적을 수 없어요.',
        path: ['overall'],
      })
    }
  })

export type ReleaseInventoryRole = z.infer<typeof releaseInventoryRoleSchema>
export type ReleaseInventoryComponentStatus = z.infer<
  typeof releaseInventoryComponentStatusSchema
>
export type ComponentEntrySelection = z.infer<typeof componentEntrySelectionSchema>
export type ReleaseInventoryComponent = z.infer<typeof releaseInventoryComponentSchema>
export type StagedReleaseInventory = z.infer<typeof stagedReleaseInventorySchema>
export type NotStagedReleaseInventory = z.infer<typeof notStagedReleaseInventorySchema>
export type ReleaseInventory = z.infer<typeof releaseInventorySchema>
export type ReleaseInventoryReasonCode = z.infer<typeof releaseInventoryReasonCodeSchema>
export type InstallSelfCheckReport = z.infer<typeof installSelfCheckReportSchema>
