import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

function readJson<T>(relativePath: string): T {
  return JSON.parse(readFileSync(resolve(process.cwd(), relativePath), 'utf8')) as T
}

function normalizeTargets(targets: string | string[]): string[] {
  return Array.isArray(targets) ? targets : [targets]
}

describe('release baseline', () => {
  it('pins the Boothy NSIS installer baseline and keeps updater artifacts off', () => {
    const packageJson = readJson<{ version: string }>('package.json')
    const tauriConfig = readJson<{
      productName: string
      version: string
      identifier: string
      bundle: {
        active: boolean
        targets: string | string[]
        createUpdaterArtifacts?: boolean
      }
    }>('src-tauri/tauri.conf.json')

    expect(tauriConfig.productName).toBe('Boothy')
    expect(tauriConfig.version).toBe(packageJson.version)
    expect(tauriConfig.identifier).toBe('com.boothy.app')
    expect(tauriConfig.bundle.active).toBe(true)
    expect(normalizeTargets(tauriConfig.bundle.targets)).toEqual(['nsis'])
    expect(tauriConfig.bundle.createUpdaterArtifacts).toBe(false)
  })

  it('keeps the Rust host metadata aligned with the Boothy app identity', () => {
    const cargoToml = readFileSync(resolve(process.cwd(), 'src-tauri/Cargo.toml'), 'utf8')

    expect(cargoToml).toContain('name = "boothy"')
    expect(cargoToml).toContain('name = "boothy_lib"')
    expect(cargoToml).toContain('edition = "2021"')
    expect(cargoToml).toContain('rust-version = "1.77.2"')
    expect(cargoToml).not.toContain('description = "A Tauri App"')
    expect(cargoToml).not.toContain('authors = ["you"]')
  })

  it('provides stable desktop build commands and a signing-ready Windows release overlay', () => {
    const packageJson = readJson<{
      scripts: Record<string, string>
    }>('package.json')
    const releaseConfig = readJson<{
      bundle: {
        targets: string[]
        createUpdaterArtifacts: boolean
        windows: {
          signCommand: {
            cmd: string
            args: string[]
          }
        }
      }
    }>('src-tauri/tauri.windows-release.conf.json')
    const signScript = readFileSync(resolve(process.cwd(), 'src-tauri/scripts/sign-windows.ps1'), 'utf8')

    expect(packageJson.scripts['build:desktop']).toBe('pnpm tauri build --bundles nsis')
    expect(packageJson.scripts['release:desktop']).toBe(
      'pnpm tauri build --bundles nsis --config src-tauri/tauri.windows-release.conf.json',
    )

    expect(releaseConfig.bundle.targets).toEqual(['nsis'])
    expect(releaseConfig.bundle.createUpdaterArtifacts).toBe(false)
    expect(releaseConfig.bundle.windows.signCommand.cmd).toBe(
      'C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe',
    )
    expect(releaseConfig.bundle.windows.signCommand.args).toEqual([
      '-NoLogo',
      '-NoProfile',
      '-ExecutionPolicy',
      'Bypass',
      '-File',
      './src-tauri/scripts/sign-windows.ps1',
      '%1',
    ])

    expect(signScript).toContain('BOOTHY_WINDOWS_CERT_PATH')
    expect(signScript).toContain('BOOTHY_WINDOWS_CERT_BASE64')
    expect(signScript).toContain('BOOTHY_WINDOWS_CERT_PASSWORD')
    expect(signScript).toContain('FromBase64String')
    expect(signScript).toContain('Signing-ready blocker')
    expect(signScript).toContain('GetEnvironmentVariable')
  })

  it('defines an explicit Windows release workflow and baseline verification documentation', () => {
    const workflow = readFileSync(resolve(process.cwd(), '.github/workflows/release-windows.yml'), 'utf8')
    const releaseDocs = readFileSync(resolve(process.cwd(), 'docs/release-baseline.md'), 'utf8')

    expect(workflow).toContain('pull_request:')
    expect(workflow).toContain('branches:')
    expect(workflow).toContain("- 'main'")
    expect(workflow).toContain('runs-on: windows-latest')
    expect(workflow).toContain('tauri-apps/tauri-action@v1')
    expect(workflow).toContain('pnpm build:desktop')
    expect(workflow).toContain('args: --config src-tauri/tauri.windows-release.conf.json --bundles nsis')
    expect(workflow).toContain('releaseDraft: true')
    expect(workflow).toContain('BOOTHY_WINDOWS_CERT_BASE64')
    expect(workflow).toContain('BOOTHY_WINDOWS_CERT_PASSWORD')
    expect(workflow).toContain('Signing-ready blocker')

    expect(releaseDocs).toContain('pnpm build:desktop')
    expect(releaseDocs).toContain('pnpm release:desktop')
    expect(releaseDocs).toContain('.github/workflows/release-windows.yml')
    expect(releaseDocs).toContain('BOOTHY_WINDOWS_CERT_BASE64')
    expect(releaseDocs).toContain('release/bundle/nsis')
    expect(releaseDocs).toContain('Signing-ready blocker')
  })

  it('documents the Windows release prerequisites and verifies the expected installer identity in CI', () => {
    const packageJson = readJson<{ version: string }>('package.json')
    const workflow = readFileSync(resolve(process.cwd(), '.github/workflows/release-windows.yml'), 'utf8')
    const releaseDocs = readFileSync(resolve(process.cwd(), 'docs/release-baseline.md'), 'utf8')
    const expectedInstaller = `Boothy_${packageJson.version}_x64-setup.exe`

    expect(workflow).toContain('Verify installer artifact identity')
    expect(workflow).toContain('Boothy_${version}_x64-setup.exe')

    expect(releaseDocs).toContain('Node.js 20.19+')
    expect(releaseDocs).toContain('Windows environment')
    expect(releaseDocs).toContain(expectedInstaller)
    expect(releaseDocs).toContain('createUpdaterArtifacts')
  })
})
