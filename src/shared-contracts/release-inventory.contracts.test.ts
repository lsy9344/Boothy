import { describe, expect, it } from 'vitest'

import {
  installSelfCheckReportSchema,
  releaseInventoryComponentSchema,
  releaseInventorySchema,
} from './index'

function createComponent(overrides: Record<string, unknown> = {}) {
  return {
    name: 'darktable',
    role: 'raw-renderer',
    status: 'present',
    version: '5.4.1',
    origin: 'darktable release-5.4.1 / c3f96ca',
    entrySelection: { kind: 'all' },
    license: 'GPL-3.0-or-later',
    licenseEvidencePath: 'release/vendor/darktable-5.4.1/LICENSE-EVIDENCE.md',
    sourceArchiveSha256: 'a'.repeat(64),
    stagedTreeDigest: 'b'.repeat(64),
    installRelativePath: 'darktable/',
    signingStatus: 'not-applicable',
    rationale: null,
    fileCount: 4211,
    totalBytes: 402_653_184,
    ...overrides,
  }
}

function createStagedInventory(overrides: Record<string, unknown> = {}) {
  return {
    schemaVersion: 'release-inventory/v1',
    staging: 'staged',
    generatedAt: '2026-08-17T05:00:00.000Z',
    appVersion: '0.1.0',
    identifier: 'com.boothy.booth',
    installerFileName: 'Boothy_0.1.0_x64-setup.exe',
    signingStatus: 'unsigned',
    installer: null,
    components: [
      createComponent({
        name: 'Boothy',
        role: 'app',
        status: 'embedded',
        version: '0.1.0',
        origin: 'tauri-nsis-bundle',
        license: 'proprietary',
        licenseEvidencePath: null,
        sourceArchiveSha256: null,
        stagedTreeDigest: null,
        entrySelection: null,
        installRelativePath: 'Boothy.exe',
        rationale:
          '설치본 안에 동봉되는 인벤토리 사본은 자기 자신의 해시를 담을 수 없다. 설치본 해시는 봉인 단계의 installer 필드가 기록한다.',
        fileCount: null,
        totalBytes: null,
      }),
      createComponent({
        name: 'canon-helper',
        role: 'camera-helper',
        entrySelection: { kind: 'allExcept', entries: ['EDSDK.dll', 'EdsImage.dll'] },
        version: '0.1.0',
        origin: 'dotnet publish -r win-x64 --self-contained true',
        license: 'proprietary',
        licenseEvidencePath: 'release/vendor/edsdk-13.19.0/LICENSE-EVIDENCE.md',
        sourceArchiveSha256: null,
        installRelativePath: 'sidecar/canon-helper/',
        signingStatus: 'unsigned',
        fileCount: 231,
        totalBytes: 72_351_744,
      }),
      createComponent({
        name: 'Canon EDSDK',
        role: 'edsdk-runtime',
        entrySelection: { kind: 'only', entries: ['EDSDK.dll', 'EdsImage.dll'] },
        version: '13.19.0',
        origin: 'Canon EDSDK 13.19.0 (2025-02-28)',
        license: 'Canon EDSDK redistribution agreement',
        licenseEvidencePath: 'release/vendor/edsdk-13.19.0/LICENSE-EVIDENCE.md',
        installRelativePath: 'sidecar/canon-helper/',
        fileCount: 2,
        totalBytes: 8_388_608,
      }),
      createComponent({
        name: 'raw-original source adapter',
        role: 'source-adapter',
        status: 'embedded',
        version: '0.1.0',
        origin: 'embedded-in-app-binary',
        license: 'proprietary',
        licenseEvidencePath: null,
        sourceArchiveSha256: null,
        stagedTreeDigest: null,
        entrySelection: null,
        installRelativePath: 'Boothy.exe',
        rationale:
          'HV-14 Technology No-Go 이후 승인된 source는 raw-original 하나이고 별도 라이브러리가 없다.',
        fileCount: null,
        totalBytes: null,
      }),
      createComponent({
        name: 'resident display renderer',
        role: 'display-renderer',
        status: 'not-applicable',
        version: null,
        origin: 'none',
        license: 'not-applicable',
        licenseEvidencePath: null,
        sourceArchiveSha256: null,
        stagedTreeDigest: null,
        entrySelection: null,
        installRelativePath: null,
        rationale: 'HV-16 Technology No-Go. 상주 renderer 후보가 비활성이라 동봉할 번들이 없다.',
        fileCount: null,
        totalBytes: null,
      }),
      createComponent({
        name: 'darktable built-in sRGB',
        role: 'color-profile',
        status: 'embedded',
        version: '5.4.1',
        origin: 'darktable-builtin-srgb',
        license: 'GPL-3.0-or-later',
        licenseEvidencePath: null,
        sourceArchiveSha256: null,
        stagedTreeDigest: null,
        entrySelection: null,
        installRelativePath: 'darktable/',
        rationale: '별도 ICC 파일이 없다. --icc-type SRGB 로 darktable 내장 프로파일을 쓴다.',
        fileCount: null,
        totalBytes: null,
      }),
      createComponent({
        name: 'proxy recipes',
        role: 'proxy-recipes',
        status: 'embedded',
        version: '0.1.0',
        origin: 'embedded-in-app-binary',
        license: 'proprietary',
        licenseEvidencePath: null,
        sourceArchiveSha256: null,
        stagedTreeDigest: null,
        entrySelection: null,
        installRelativePath: 'Boothy.exe',
        rationale: 'preset XMP 템플릿은 include_str! 로 앱 바이너리에 컴파일된다.',
        fileCount: null,
        totalBytes: null,
      }),
      createComponent(),
      createComponent({
        name: 'Microsoft Edge WebView2 Runtime',
        role: 'webview2-runtime',
        status: 'embedded',
        version: 'offlineInstaller',
        origin: 'tauri bundle.windows.webviewInstallMode=offlineInstaller',
        license: 'Microsoft WebView2 Runtime redistribution terms',
        licenseEvidencePath: null,
        sourceArchiveSha256: null,
        stagedTreeDigest: null,
        entrySelection: null,
        installRelativePath: 'Boothy_0.1.0_x64-setup.exe',
        rationale:
          'NSIS 설치본이 오프라인 설치본을 자체 포함한다. v2 스키마에 fixedRuntime 이 없어 버전을 설치본으로 고정할 수 없고, 회차마다 실제 버전을 기록한다.',
        fileCount: null,
        totalBytes: null,
      }),
    ],
    ...overrides,
  }
}

describe('release-inventory/v1 contract', () => {
  it('accepts a staged inventory that names every required component role', () => {
    const parsed = releaseInventorySchema.parse(createStagedInventory())

    expect(parsed.staging).toBe('staged')
    expect(parsed.staging === 'staged' && parsed.components).toHaveLength(9)
  })

  it('accepts a not-staged inventory so an unstaged build declares its own gap', () => {
    const parsed = releaseInventorySchema.parse({
      schemaVersion: 'release-inventory/v1',
      staging: 'not-staged',
      reason: 'release:stage 가 실행되지 않았습니다.',
    })

    expect(parsed.staging).toBe('not-staged')
  })

  it('rejects a component without licensing evidence of its own', () => {
    const withoutLicense: Record<string, unknown> = createComponent()
    delete withoutLicense.license

    expect(releaseInventoryComponentSchema.safeParse(withoutLicense).success).toBe(false)
    expect(
      releaseInventoryComponentSchema.safeParse(createComponent({ license: '   ' })).success,
    ).toBe(false)
  })

  it('rejects an inactive component that does not say why it is inactive', () => {
    const result = releaseInventoryComponentSchema.safeParse(
      createComponent({
        role: 'display-renderer',
        status: 'not-applicable',
        version: null,
        stagedTreeDigest: null,
        entrySelection: null,
        sourceArchiveSha256: null,
        licenseEvidencePath: null,
        installRelativePath: null,
        rationale: null,
        fileCount: null,
        totalBytes: null,
      }),
    )

    expect(result.success).toBe(false)
  })

  it('rejects an unknown component role', () => {
    expect(
      releaseInventoryComponentSchema.safeParse(createComponent({ role: 'gpu-shader-bundle' }))
        .success,
    ).toBe(false)
  })

  it('rejects a staged component that does not say which files are its own', () => {
    expect(
      releaseInventoryComponentSchema.safeParse(createComponent({ entrySelection: null })).success,
    ).toBe(false)
  })

  it('rejects a file selection rule on a component that was never staged', () => {
    expect(
      releaseInventoryComponentSchema.safeParse(
        createComponent({
          status: 'embedded',
          stagedTreeDigest: null,
          rationale: '앱 바이너리에 내장된다.',
          fileCount: null,
          totalBytes: null,
          entrySelection: { kind: 'all' },
        }),
      ).success,
    ).toBe(false)
  })

  it('rejects an unknown file selection kind', () => {
    expect(
      releaseInventoryComponentSchema.safeParse(
        createComponent({ entrySelection: { kind: 'glob', entries: ['**/*'] } }),
      ).success,
    ).toBe(false)
  })

  it('rejects a staged component without a tree digest', () => {
    expect(
      releaseInventoryComponentSchema.safeParse(createComponent({ stagedTreeDigest: null }))
        .success,
    ).toBe(false)
  })

  it('rejects a tree digest on a component that was never staged', () => {
    expect(
      releaseInventoryComponentSchema.safeParse(
        createComponent({
          status: 'not-applicable',
          rationale: '후보가 비활성이다.',
          stagedTreeDigest: 'c'.repeat(64),
        }),
      ).success,
    ).toBe(false)
  })

  it('rejects a malformed digest', () => {
    expect(
      releaseInventoryComponentSchema.safeParse(createComponent({ stagedTreeDigest: 'nothex' }))
        .success,
    ).toBe(false)
  })

  it('rejects an inventory that is missing a required role', () => {
    const inventory = createStagedInventory()
    const withoutRawRenderer = {
      ...inventory,
      components: inventory.components.filter((component) => component.role !== 'raw-renderer'),
    }

    expect(releaseInventorySchema.safeParse(withoutRawRenderer).success).toBe(false)
  })

  it('rejects an inventory that names the same role twice', () => {
    const inventory = createStagedInventory()
    const duplicated = {
      ...inventory,
      components: [...inventory.components, createComponent()],
    }

    expect(releaseInventorySchema.safeParse(duplicated).success).toBe(false)
  })
})

describe('install-self-check/v1 contract', () => {
  function createReport(overrides: Record<string, unknown> = {}) {
    return {
      schemaVersion: 'install-self-check/v1',
      checkedAt: '2026-08-17T05:10:00.000Z',
      appVersion: '0.1.0',
      identifier: 'com.boothy.booth',
      installRoot: 'C:\\Program Files\\Boothy',
      components: [
        {
          name: 'darktable',
          expectedDigest: 'b'.repeat(64),
          actualDigest: 'b'.repeat(64),
          status: 'pass',
          reasonCode: null,
        },
      ],
      webview2Version: '141.0.3537.85',
      darktableResolution: {
        binary: 'C:\\Program Files\\Boothy\\darktable\\bin\\darktable-cli.exe',
        source: 'bundled-resource',
      },
      helperVersion: 'canon-helper 0.1.0',
      overall: 'pass',
      ...overrides,
    }
  }

  it('accepts a passing report', () => {
    expect(installSelfCheckReportSchema.parse(createReport()).overall).toBe('pass')
  })

  it('refuses to call a report passing while a component failed', () => {
    const result = installSelfCheckReportSchema.safeParse(
      createReport({
        components: [
          {
            name: 'darktable',
            expectedDigest: 'b'.repeat(64),
            actualDigest: 'c'.repeat(64),
            status: 'fail',
            reasonCode: 'inventory-digest-mismatch',
          },
        ],
      }),
    )

    expect(result.success).toBe(false)
  })

  it('requires a reason code on every failed component', () => {
    const result = installSelfCheckReportSchema.safeParse(
      createReport({
        overall: 'fail',
        components: [
          {
            name: 'darktable',
            expectedDigest: 'b'.repeat(64),
            actualDigest: null,
            status: 'fail',
            reasonCode: null,
          },
        ],
      }),
    )

    expect(result.success).toBe(false)
  })

  it('rejects an unknown darktable resolution source', () => {
    const result = installSelfCheckReportSchema.safeParse(
      createReport({
        darktableResolution: {
          binary: 'C:\\somewhere\\darktable-cli.exe',
          source: 'guessed',
        },
      }),
    )

    expect(result.success).toBe(false)
  })
})
