import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { releaseInventorySchema } from '../src/shared-contracts/schemas/release-inventory'

import {
  buildComponent,
  buildInventory,
  collectStagedFiles,
  computeTreeDigest,
  NOT_STAGED_MARKER,
  resolveInstallerSigningStatus,
  type BuildOptions,
  type ComponentSpec,
} from './build-inventory'

let workspace: string

beforeEach(() => {
  workspace = mkdtempSync(path.join(tmpdir(), 'boothy-inventory-'))
})

afterEach(() => {
  rmSync(workspace, { recursive: true, force: true })
})

function writeFile(relative: string, content: string): void {
  const absolute = path.join(workspace, relative)
  mkdirSync(path.dirname(absolute), { recursive: true })
  writeFileSync(absolute, content, 'utf8')
}

function buildOptions(): BuildOptions {
  return {
    repoRoot: workspace,
    appVersion: '0.1.0',
    identifier: 'com.boothy.booth',
    installerFileName: 'Boothy_0.1.0_x64-setup.exe',
    installerSigningStatus: 'unsigned',
    generatedAt: '2026-08-17T05:00:00.000Z',
  }
}

function stagedSpec(overrides: Partial<ComponentSpec> = {}): ComponentSpec {
  return {
    name: 'darktable',
    role: 'raw-renderer',
    expectedStatus: 'present',
    version: '5.4.1',
    origin: 'darktable release-5.4.1',
    license: 'GPL-3.0-or-later',
    licenseEvidencePath: 'release/licenses/darktable-5.4.1.md',
    installRelativePath: 'darktable/',
    signingStatus: 'not-applicable',
    rationale: null,
    stage: {
      root: 'payload',
      requiredEntries: ['bin/darktable-cli.exe'],
    },
    pinnedSourceArchiveSha256: null,
    pinnedStagedTreeDigest: null,
    ...overrides,
  }
}

describe('tree digest', () => {
  it('changes when a single byte in a single file changes', () => {
    writeFile('payload/bin/darktable-cli.exe', 'cli')
    writeFile('payload/share/darktable/noiseprofiles.json', '{}')

    const before = computeTreeDigest(collectStagedFiles(path.join(workspace, 'payload'), { root: 'payload' }))

    writeFile('payload/share/darktable/noiseprofiles.json', '{ }')

    const after = computeTreeDigest(collectStagedFiles(path.join(workspace, 'payload'), { root: 'payload' }))

    expect(after).not.toBe(before)
  })

  it('does not change when the same entries arrive in a different order', () => {
    const entries = [
      { relativePath: 'b.txt', sha256: 'b'.repeat(64), sizeBytes: 1 },
      { relativePath: 'a.txt', sha256: 'a'.repeat(64), sizeBytes: 1 },
      { relativePath: 'c/d.txt', sha256: 'c'.repeat(64), sizeBytes: 1 },
    ]

    const forward = computeTreeDigest(entries)
    const reversed = computeTreeDigest([...entries].reverse())

    expect(reversed).toBe(forward)
  })

  it('changes when a file is added and when a file is removed', () => {
    const base = [
      { relativePath: 'a.txt', sha256: 'a'.repeat(64), sizeBytes: 1 },
      { relativePath: 'b.txt', sha256: 'b'.repeat(64), sizeBytes: 1 },
    ]
    const added = [...base, { relativePath: 'c.txt', sha256: 'c'.repeat(64), sizeBytes: 1 }]
    const removed = base.slice(0, 1)

    expect(computeTreeDigest(added)).not.toBe(computeTreeDigest(base))
    expect(computeTreeDigest(removed)).not.toBe(computeTreeDigest(base))
  })

  /**
   * 골든 벡터. Rust self-check(`src-tauri/src/release/inventory.rs`)와 PowerShell 게이트가
   * **같은 값**을 내야 한다. 하나만 달라지면 릴리스가 항상 digest-mismatch로 막힌다.
   */
  it('matches the golden vector shared with the Rust self-check', () => {
    writeFile('golden/a.txt', 'alpha')
    writeFile('golden/dir/b.txt', 'beta')

    const files = collectStagedFiles(path.join(workspace, 'golden'), { root: 'golden' })

    expect(files).toHaveLength(2)
    expect(computeTreeDigest(files)).toBe(
      '467f567c070c84409b3225c48cc0abadf716a1cb5e39bd5be4d720e90dd5e2d6',
    )
  })

  it('is deterministic for an empty tree', () => {
    expect(computeTreeDigest([])).toBe(computeTreeDigest([]))
    expect(computeTreeDigest([])).toBe(
      'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
    )
  })

  it('separates a nested path from a same-named flat path', () => {
    const nested = [{ relativePath: 'bin/cli.exe', sha256: 'a'.repeat(64), sizeBytes: 1 }]
    const flat = [{ relativePath: 'bin-cli.exe', sha256: 'a'.repeat(64), sizeBytes: 1 }]

    expect(computeTreeDigest(nested)).not.toBe(computeTreeDigest(flat))
  })
})

describe('staged file collection', () => {
  it('walks nested directories and reports stable relative paths', () => {
    writeFile('payload/bin/darktable-cli.exe', 'cli')
    writeFile('payload/lib/plugin.dll', 'dll')
    writeFile('payload/share/darktable/noiseprofiles.json', '{}')

    const files = collectStagedFiles(path.join(workspace, 'payload'), { root: 'payload' })

    expect(files.map((file) => file.relativePath)).toEqual([
      'bin/darktable-cli.exe',
      'lib/plugin.dll',
      'share/darktable/noiseprofiles.json',
    ])
  })

  it('keeps only the explicitly included entries when include is given', () => {
    writeFile('payload/EDSDK.dll', 'edsdk')
    writeFile('payload/EdsImage.dll', 'edsimage')
    writeFile('payload/canon-helper.exe', 'helper')

    const files = collectStagedFiles(path.join(workspace, 'payload'), {
      root: 'payload',
      include: ['EDSDK.dll', 'EdsImage.dll'],
    })

    expect(files.map((file) => file.relativePath)).toEqual(['EDSDK.dll', 'EdsImage.dll'])
  })

  it('removes excluded entries so two components never claim the same file', () => {
    writeFile('payload/EDSDK.dll', 'edsdk')
    writeFile('payload/canon-helper.exe', 'helper')

    const files = collectStagedFiles(path.join(workspace, 'payload'), {
      root: 'payload',
      exclude: ['EDSDK.dll'],
    })

    expect(files.map((file) => file.relativePath)).toEqual(['canon-helper.exe'])
  })

  it('returns nothing for a directory that does not exist', () => {
    expect(collectStagedFiles(path.join(workspace, 'absent'), { root: 'absent' })).toEqual([])
  })
})

describe('component build', () => {
  it('records a staged component with its digest, file count, and size', () => {
    writeFile('payload/bin/darktable-cli.exe', 'cli')
    writeFile('payload/lib/plugin.dll', 'dll')

    const { component } = buildComponent(stagedSpec(), buildOptions())

    expect(component.status).toBe('present')
    expect(component.fileCount).toBe(2)
    expect(component.totalBytes).toBe(6)
    expect(component.stagedTreeDigest).toMatch(/^[0-9a-f]{64}$/)
    expect(component.rationale).toBeNull()
  })

  it('records an absent payload as missing, with the reason spelled out', () => {
    const { component, note } = buildComponent(stagedSpec(), buildOptions())

    expect(component.status).toBe('missing')
    expect(component.stagedTreeDigest).toBeNull()
    expect(String(component.rationale)).toContain('조달되지 않았다')
    expect(note).toContain('MISSING')
  })

  it('treats a not-staged marker as missing even though files are present', () => {
    writeFile('payload/bin/darktable-cli.exe', 'cli')
    writeFile(`payload/${NOT_STAGED_MARKER}`, '# 이 자리는 아직 채워지지 않았습니다')

    const { component } = buildComponent(stagedSpec(), buildOptions())

    expect(component.status).toBe('missing')
    expect(String(component.rationale)).toContain(NOT_STAGED_MARKER)
  })

  it('records a missing required entry as missing and names the entry', () => {
    writeFile('payload/lib/plugin.dll', 'dll')

    const { component } = buildComponent(stagedSpec(), buildOptions())

    expect(component.status).toBe('missing')
    expect(String(component.rationale)).toContain('bin/darktable-cli.exe')
  })

  it('carries a declared-only component straight through without inventing a digest', () => {
    const { component } = buildComponent(
      stagedSpec({
        name: 'resident display renderer',
        role: 'display-renderer',
        expectedStatus: 'not-applicable',
        version: null,
        licenseEvidencePath: null,
        installRelativePath: null,
        rationale: 'HV-16 Technology No-Go.',
        stage: null,
      }),
      buildOptions(),
    )

    expect(component.status).toBe('not-applicable')
    expect(component.stagedTreeDigest).toBeNull()
    expect(component.rationale).toBe('HV-16 Technology No-Go.')
  })

  it('resolves from-tauri-conf version and {version} install paths', () => {
    const { component } = buildComponent(
      stagedSpec({
        role: 'webview2-runtime',
        expectedStatus: 'embedded',
        version: 'from-tauri-conf',
        installRelativePath: 'Boothy_{version}_x64-setup.exe',
        rationale: '설치본이 오프라인 설치본을 자체 포함한다.',
        stage: null,
      }),
      buildOptions(),
    )

    expect(component.version).toBe('0.1.0')
    expect(component.installRelativePath).toBe('Boothy_0.1.0_x64-setup.exe')
  })

  it('inherits the installer signing status where the spec says inherit', () => {
    const options = { ...buildOptions(), installerSigningStatus: 'signed' }
    const { component } = buildComponent(
      stagedSpec({ signingStatus: 'inherit', stage: null, rationale: '내장' , expectedStatus: 'embedded' }),
      options,
    )

    expect(component.signingStatus).toBe('signed')
  })
})

describe('inventory document', () => {
  it('writes a staged document whose installer identity is filled in only when sealed', () => {
    const { inventory } = buildInventory({ schemaVersion: 'release-inventory-spec/v1', components: [stagedSpec()] }, buildOptions())

    expect(inventory.schemaVersion).toBe('release-inventory/v1')
    expect(inventory.staging).toBe('staged')
    expect(inventory.installer).toBeNull()
    expect(inventory.installerFileName).toBe('Boothy_0.1.0_x64-setup.exe')
  })
})

describe('the real inventory spec', () => {
  const repoRoot = path.resolve(import.meta.dirname, '..')

  /**
   * **staged 트리를 실제로 해시하지 않는다.** 이 테스트가 확인하는 것은 명세가 계약을
   * 만족하는 문서를 만드는가이고, 실제 페이로드를 해시하면 vitest 프로세스가 수백 MB를
   * 읽느라 다른 테스트의 타이밍까지 흔든다. 세 구현의 해시 일치는 PowerShell 게이트
   * 테스트와 Rust 통합 테스트가 실제 트리로 증명한다.
   */
  it('produces a document that satisfies the release-inventory/v1 contract', () => {
    const spec = JSON.parse(
      readFileSync(path.join(repoRoot, 'release', 'inventory-spec.json'), 'utf8'),
    ) as { schemaVersion: string; components: ComponentSpec[] }

    const { inventory } = buildInventory(spec, {
      repoRoot: workspace,
      appVersion: '0.1.0',
      identifier: 'com.boothy.booth',
      installerFileName: 'Boothy_0.1.0_x64-setup.exe',
      installerSigningStatus: 'unsigned',
      generatedAt: '2026-08-17T05:00:00.000Z',
    })

    const parsed = releaseInventorySchema.parse(inventory)

    expect(parsed.staging).toBe('staged')
    expect(parsed.staging === 'staged' && parsed.components).toHaveLength(9)
  })

  it('names every licence evidence file it points at', () => {
    const spec = JSON.parse(
      readFileSync(path.join(repoRoot, 'release', 'inventory-spec.json'), 'utf8'),
    ) as { components: ComponentSpec[] }

    for (const component of spec.components) {
      if (component.licenseEvidencePath === null) {
        continue
      }

      expect(
        existsSync(path.join(repoRoot, ...component.licenseEvidencePath.split('/'))),
        `${component.role} 의 라이선스 증거 파일이 없다: ${component.licenseEvidencePath}`,
      ).toBe(true)
    }
  })
})

describe('installer signing status', () => {
  it('is unsigned while no certificate is configured anywhere', () => {
    expect(
      resolveInstallerSigningStatus({ bundle: { windows: { certificateThumbprint: null } } }, {}),
    ).toBe('unsigned')
  })

  it('is signed when the config carries a thumbprint', () => {
    expect(
      resolveInstallerSigningStatus(
        { bundle: { windows: { certificateThumbprint: 'ABCD1234' } } },
        {},
      ),
    ).toBe('signed')
  })

  it('is signed when CI supplies the thumbprint through the environment', () => {
    expect(
      resolveInstallerSigningStatus(
        { bundle: { windows: { certificateThumbprint: null } } },
        { BOOTHY_WINDOWS_CERT_THUMBPRINT: 'ABCD1234' },
      ),
    ).toBe('signed')
  })

  it('is signed when a custom sign command is configured', () => {
    expect(
      resolveInstallerSigningStatus(
        { bundle: { windows: { signCommand: 'sign-tool %1' } } },
        {},
      ),
    ).toBe('signed')
  })

  it('does not treat an empty string as a certificate', () => {
    expect(
      resolveInstallerSigningStatus(
        { bundle: { windows: { certificateThumbprint: '   ' } } },
        { BOOTHY_WINDOWS_CERT_THUMBPRINT: '' },
      ),
    ).toBe('unsigned')
  })
})
