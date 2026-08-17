/**
 * Story 7.7 — release-inventory/v1 생성기.
 *
 * staged 트리를 훑어 구성요소마다 해시를 계산하고 `release/dist/inventory.json`을 쓴다.
 *
 * **이 파일이 인벤토리를 판정하지 않는다.** 여기는 "실제로 무엇이 들어갔는가"만 기록하고,
 * 명세와의 대조는 `release/verify-inventory.ps1`이 한다. 생성과 판정을 한 곳에 두면
 * 생성기가 자기 산출물을 통과시키는 게이트가 된다.
 *
 * 실행: `node release/build-inventory.ts`
 * 봉인: `node release/build-inventory.ts --seal <설치본 경로>`
 *
 * 타입 표기는 Node가 지워 낼 수 있는 문법만 쓴다 (enum/namespace/parameter property 금지).
 */

import { createHash } from 'node:crypto'
import { existsSync, readdirSync, readFileSync, statSync, mkdirSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { pathToFileURL } from 'node:url'

export const NOT_STAGED_MARKER = 'PAYLOAD-NOT-STAGED.md'

export type StagedFileEntry = {
  /** 루트 기준 상대 경로. 항상 `/` 구분자다 — 플랫폼이 digest를 바꾸면 안 된다. */
  relativePath: string
  sha256: string
  sizeBytes: number
}

export type ComponentStageSpec = {
  root: string
  include?: string[]
  exclude?: string[]
  requiredEntries?: string[]
}

export type ComponentSpec = {
  name: string
  role: string
  expectedStatus: string
  version: string | null
  origin: string
  license: string
  licenseEvidencePath: string | null
  installRelativePath: string | null
  signingStatus: string
  rationale: string | null
  stage: ComponentStageSpec | null
  pinnedSourceArchiveSha256: string | null
  pinnedStagedTreeDigest: string | null
}

export type InventorySpec = {
  schemaVersion: string
  components: ComponentSpec[]
}

export function sha256OfBuffer(buffer: Buffer): string {
  return createHash('sha256').update(buffer).digest('hex')
}

export function sha256OfFile(filePath: string): string {
  return sha256OfBuffer(readFileSync(filePath))
}

/**
 * 트리 해시.
 *
 * 정렬된 `<상대경로>:<sha256>` 목록을 `\n`으로 이어 붙여 다시 sha256 한다.
 *
 * - **디렉터리 나열 순서가 달라져도 같은 값**이 나와야 한다. 그래서 정렬한다.
 * - **파일 하나만 바뀌어도 다른 값**이 나와야 한다. 그래서 내용 해시를 포함한다.
 * - 빈 트리는 빈 문자열의 sha256이다. 결정적이되, 빈 트리는 `missing`으로 기록되므로
 *   이 값이 인벤토리에 그대로 실리는 일은 없다.
 */
export function compareUtf8(left: string, right: string): number {
  return Buffer.compare(Buffer.from(left, 'utf8'), Buffer.from(right, 'utf8'))
}

export function computeTreeDigest(entries: StagedFileEntry[]): string {
  const lines = entries.map((entry) => `${entry.relativePath}:${entry.sha256}`).sort(compareUtf8)

  return sha256OfBuffer(Buffer.from(lines.join('\n'), 'utf8'))
}

function walkFiles(root: string, current: string, collected: string[]): void {
  const absolute = path.join(root, current)

  for (const entry of readdirSync(absolute, { withFileTypes: true })) {
    const relative = current === '' ? entry.name : `${current}/${entry.name}`

    if (entry.isDirectory()) {
      walkFiles(root, relative, collected)
      continue
    }

    if (entry.isFile()) {
      collected.push(relative)
    }
  }
}

/**
 * staged 트리의 파일 목록.
 *
 * `include`가 있으면 그 목록만, 없으면 전부. `exclude`는 언제나 뺀다.
 * **glob을 쓰지 않는다** — 무엇이 담겼는지가 명세를 읽는 것만으로 결정되어야 한다.
 */
export function collectStagedFiles(root: string, stage: ComponentStageSpec): StagedFileEntry[] {
  if (!existsSync(root) || !statSync(root).isDirectory()) {
    return []
  }

  const excluded = new Set(stage.exclude ?? [])
  let relativePaths: string[] = []

  if (stage.include && stage.include.length > 0) {
    relativePaths = stage.include.filter((relative) => existsSync(path.join(root, relative)))
  } else {
    walkFiles(root, '', relativePaths)
  }

  return relativePaths
    .filter((relative) => !excluded.has(relative))
    .map((relative) => {
      const absolute = path.join(root, relative)

      return {
        relativePath: relative,
        sha256: sha256OfFile(absolute),
        sizeBytes: statSync(absolute).size,
      }
    })
    .sort((left, right) => compareUtf8(left.relativePath, right.relativePath))
}

/**
 * 파일 선택 규칙을 인벤토리가 담을 수 있는 형태로 옮긴다.
 *
 * 설치된 앱은 명세를 갖고 있지 않다. 이 규칙이 인벤토리에 없으면 self-check가
 * 어떤 파일을 해시해야 하는지 알 수 없다.
 */
export function describeEntrySelection(stage: ComponentStageSpec): Record<string, unknown> {
  if (stage.include && stage.include.length > 0) {
    return { kind: 'only', entries: [...stage.include].sort(compareUtf8) }
  }

  if (stage.exclude && stage.exclude.length > 0) {
    return { kind: 'allExcept', entries: [...stage.exclude].sort(compareUtf8) }
  }

  return { kind: 'all' }
}

export function missingRequiredEntries(root: string, stage: ComponentStageSpec): string[] {
  return (stage.requiredEntries ?? []).filter(
    (entry) => !existsSync(path.join(root, ...entry.split('/'))),
  )
}

export type BuildOptions = {
  repoRoot: string
  appVersion: string
  identifier: string
  installerFileName: string
  installerSigningStatus: string
  generatedAt: string
}

export type ComponentBuildResult = {
  component: Record<string, unknown>
  /** 사람이 읽는 한 줄. 조용히 넘어가는 결손이 없도록 stage 로그에 그대로 찍는다. */
  note: string
}

function resolveVersion(spec: ComponentSpec, options: BuildOptions): string | null {
  return spec.version === 'from-tauri-conf' ? options.appVersion : spec.version
}

function resolveSigningStatus(spec: ComponentSpec, options: BuildOptions): string {
  return spec.signingStatus === 'inherit' ? options.installerSigningStatus : spec.signingStatus
}

function resolveInstallRelativePath(
  spec: ComponentSpec,
  options: BuildOptions,
): string | null {
  if (spec.installRelativePath === null) {
    return null
  }

  return spec.installRelativePath.replace('{version}', options.appVersion)
}

export function buildComponent(spec: ComponentSpec, options: BuildOptions): ComponentBuildResult {
  const version = resolveVersion(spec, options)
  const signingStatus = resolveSigningStatus(spec, options)
  const installRelativePath = resolveInstallRelativePath(spec, options)

  const declared = {
    name: spec.name,
    role: spec.role,
    version,
    origin: spec.origin,
    entrySelection: null as Record<string, unknown> | null,
    license: spec.license,
    licenseEvidencePath: spec.licenseEvidencePath,
    sourceArchiveSha256: spec.pinnedSourceArchiveSha256,
    installRelativePath,
    signingStatus,
  }

  if (spec.stage === null) {
    return {
      component: {
        ...declared,
        status: spec.expectedStatus,
        stagedTreeDigest: null,
        rationale: spec.rationale,
        fileCount: null,
        totalBytes: null,
      },
      note: `${spec.role}: ${spec.expectedStatus} (별도 페이로드 없음)`,
    }
  }

  const root = path.join(options.repoRoot, ...spec.stage.root.split('/'))
  const missingEntries = missingRequiredEntries(root, spec.stage)
  const files = collectStagedFiles(root, spec.stage)
  const notStaged = existsSync(path.join(root, NOT_STAGED_MARKER))

  if (notStaged || files.length === 0 || missingEntries.length > 0) {
    const reason = notStaged
      ? `${spec.stage.root} 에 ${NOT_STAGED_MARKER} 표시만 있다. 페이로드가 조달되지 않았다.`
      : files.length === 0
        ? `${spec.stage.root} 에 파일이 없다. 페이로드가 조달되지 않았다.`
        : `${spec.stage.root} 에 필수 항목이 없다: ${missingEntries.join(', ')}`

    return {
      component: {
        ...declared,
        status: 'missing',
        stagedTreeDigest: null,
        rationale: reason,
        fileCount: null,
        totalBytes: null,
      },
      note: `${spec.role}: MISSING — ${reason}`,
    }
  }

  const stagedTreeDigest = computeTreeDigest(files)
  const totalBytes = files.reduce((sum, entry) => sum + entry.sizeBytes, 0)

  return {
    component: {
      ...declared,
      entrySelection: describeEntrySelection(spec.stage),
      status: 'present',
      stagedTreeDigest,
      rationale: null,
      fileCount: files.length,
      totalBytes,
    },
    note: `${spec.role}: present, ${files.length} files, ${totalBytes} bytes, digest ${stagedTreeDigest}`,
  }
}

export function buildInventory(
  spec: InventorySpec,
  options: BuildOptions,
): { inventory: Record<string, unknown>; notes: string[] } {
  const results = spec.components.map((component) => buildComponent(component, options))

  return {
    inventory: {
      schemaVersion: 'release-inventory/v1',
      staging: 'staged',
      generatedAt: options.generatedAt,
      appVersion: options.appVersion,
      identifier: options.identifier,
      installerFileName: options.installerFileName,
      signingStatus: options.installerSigningStatus,
      installer: null,
      components: results.map((result) => result.component),
    },
    notes: results.map((result) => result.note),
  }
}

export function readJsonFile(filePath: string): Record<string, unknown> {
  // Windows 도구들이 UTF-8 BOM을 붙여 쓴다. BOM 하나 때문에 릴리스가 멈추면 안 된다.
  const raw = readFileSync(filePath, 'utf8')
  const text = raw.charCodeAt(0) === 0xfeff ? raw.slice(1) : raw

  return JSON.parse(text) as Record<string, unknown>
}

export function resolveInstallerSigningStatus(
  tauriConf: Record<string, unknown>,
  env: Record<string, string | undefined>,
): string {
  const bundle = (tauriConf.bundle ?? {}) as Record<string, unknown>
  const windows = (bundle.windows ?? {}) as Record<string, unknown>
  const thumbprintFromConf = windows.certificateThumbprint
  const thumbprintFromEnv = env.BOOTHY_WINDOWS_CERT_THUMBPRINT
  const signCommand = windows.signCommand

  const hasThumbprint =
    (typeof thumbprintFromConf === 'string' && thumbprintFromConf.trim() !== '') ||
    (typeof thumbprintFromEnv === 'string' && thumbprintFromEnv.trim() !== '')
  const hasSignCommand = typeof signCommand === 'string' && signCommand.trim() !== ''

  return hasThumbprint || hasSignCommand ? 'signed' : 'unsigned'
}

export const NSIS_BUNDLE_DIR = 'src-tauri/target/release/bundle/nsis'

/** 봉인 대상 설치본을 찾는다. 인자로 경로를 주면 그쪽이 우선한다. */
export function discoverInstaller(repoRoot: string): string | undefined {
  const bundleDir = path.join(repoRoot, ...NSIS_BUNDLE_DIR.split('/'))

  if (!existsSync(bundleDir)) {
    return undefined
  }

  const candidates = readdirSync(bundleDir)
    .filter((entry) => entry.toLowerCase().endsWith('.exe'))
    .sort(compareUtf8)

  return candidates.length === 0 ? undefined : path.join(bundleDir, candidates[0]!)
}

function readOption(argv: string[], name: string): string | undefined {
  const index = argv.indexOf(name)

  return index === -1 ? undefined : argv[index + 1]
}

function main(argv: string[]): number {
  // `--repo-root` / `--spec` / `--out` 은 게이트 자체를 시험하기 위한 입구다
  // (`release/test-verify-inventory.ps1`). 실제 릴리스 경로는 기본값만 쓴다.
  const repoRoot = readOption(argv, '--repo-root') ?? path.resolve(import.meta.dirname, '..')
  const specPath = readOption(argv, '--spec') ?? path.join(repoRoot, 'release', 'inventory-spec.json')
  const distDir = path.join(repoRoot, 'release', 'dist')
  const inventoryPath = readOption(argv, '--out') ?? path.join(distDir, 'inventory.json')
  const sealedPath = path.join(distDir, 'inventory.release.json')

  const sealIndex = argv.indexOf('--seal')

  if (sealIndex !== -1) {
    if (!existsSync(inventoryPath)) {
      console.error('release/dist/inventory.json 이 없습니다. 먼저 release:stage 를 실행하세요.')
      return 1
    }

    const inventory = readJsonFile(inventoryPath)

    if (inventory.staging !== 'staged') {
      console.error('staged 되지 않은 인벤토리는 봉인할 수 없습니다.')
      return 1
    }

    const explicitPath = argv[sealIndex + 1]
    const installerPath = explicitPath ?? discoverInstaller(repoRoot)

    if (!installerPath) {
      console.error(
        `설치본을 찾지 못했습니다. ${NSIS_BUNDLE_DIR} 아래에 .exe 가 없습니다. 먼저 tauri build 를 실행하세요.`,
      )
      return 1
    }

    if (!existsSync(installerPath)) {
      console.error(`설치본을 찾지 못했습니다: ${installerPath}`)
      return 1
    }

    // **산출물 이름 검증.** `release-baseline.md` 가 이미 있다고 주장했던 그 검사다.
    // 이름이 다르면 배포 절차와 증거가 가리키는 파일이 서로 달라진다.
    const actualName = path.basename(installerPath)
    const expectedName = String(inventory.installerFileName)

    if (actualName !== expectedName) {
      console.error(`설치본 이름이 인벤토리와 다릅니다. 기대 ${expectedName}, 실제 ${actualName}`)
      return 1
    }

    const sealed = {
      ...inventory,
      installer: {
        fileName: path.basename(installerPath),
        sha256: sha256OfFile(installerPath),
        sizeBytes: statSync(installerPath).size,
      },
    }

    writeFileSync(sealedPath, `${JSON.stringify(sealed, null, 2)}\n`, 'utf8')
    console.log(`봉인했습니다: ${sealedPath}`)
    console.log(`  설치본: ${sealed.installer.fileName}`)
    console.log(`  sha256: ${sealed.installer.sha256}`)
    console.log(`  크기:   ${sealed.installer.sizeBytes} bytes`)
    return 0
  }

  const spec = readJsonFile(specPath) as unknown as InventorySpec
  // 앱 버전과 identifier는 언제나 **실제 제품 설정**에서 온다. `--repo-root` 로 staging
  // 트리를 옮겨도 인벤토리가 가리키는 제품이 달라지면 안 된다.
  const tauriConf = readJsonFile(
    path.resolve(import.meta.dirname, '..', 'src-tauri', 'tauri.conf.json'),
  )
  const appVersion = String(tauriConf.version)
  const productName = String(tauriConf.productName)

  const { inventory, notes } = buildInventory(spec, {
    repoRoot,
    appVersion,
    identifier: String(tauriConf.identifier),
    installerFileName: `${productName}_${appVersion}_x64-setup.exe`,
    installerSigningStatus: resolveInstallerSigningStatus(tauriConf, process.env),
    generatedAt: new Date().toISOString(),
  })

  mkdirSync(path.dirname(inventoryPath), { recursive: true })
  writeFileSync(inventoryPath, `${JSON.stringify(inventory, null, 2)}\n`, 'utf8')

  console.log(`인벤토리를 썼습니다: ${inventoryPath}`)
  for (const note of notes) {
    console.log(`  ${note}`)
  }

  const missing = (inventory.components as Array<Record<string, unknown>>).filter(
    (component) => component.status === 'missing',
  )

  if (missing.length > 0) {
    console.log('')
    console.log(
      `결손 ${missing.length}건이 인벤토리에 기록되었습니다. release:verify 가 이 빌드를 막습니다.`,
    )
  }

  return 0
}

const invokedDirectly =
  process.argv[1] !== undefined && pathToFileURL(process.argv[1]).href === import.meta.url

if (invokedDirectly) {
  process.exit(main(process.argv.slice(2)))
}
