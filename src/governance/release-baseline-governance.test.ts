import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

import { describe, expect, it } from 'vitest'

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..')

const readRepoFile = (...segments: string[]) =>
  readFileSync(resolve(repoRoot, ...segments), 'utf8')

const extractStepBlock = (workflow: string, stepName: string) => {
  const escapedStepName = stepName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  const stepPattern = new RegExp(
    `- name: ${escapedStepName}[\\s\\S]*?(?=\\n\\s*- name: |$)`,
  )
  const stepMatch = workflow.match(stepPattern)

  expect(stepMatch?.[0]).toBeTruthy()
  return stepMatch![0]
}

describe('release baseline governance', () => {
  it('keeps package scripts, release docs, workflow, and tauri config aligned', () => {
    const packageJson = JSON.parse(readRepoFile('package.json')) as {
      scripts: Record<string, string>
    }
    const workflow = readRepoFile('.github', 'workflows', 'release-windows.yml')
    const releaseBaseline = readRepoFile('docs', 'release-baseline.md')
    const legacyReleaseBaseline = readRepoFile('release-baseline.md')
    const tauriConfig = JSON.parse(readRepoFile('src-tauri', 'tauri.conf.json')) as {
      bundle?: Record<string, unknown>
    }

    expect(packageJson.scripts['build:desktop']).toBe(
      'pnpm build && pnpm tauri build --debug --no-sign',
    )
    expect(packageJson.scripts['release:desktop']).toBe(
      'pnpm build && node scripts/prepare-windows-signing.mjs --allow-unsigned -- pnpm tauri build',
    )

    expect(releaseBaseline).toContain('Node.js 20.19+ or 22.12+')
    expect(releaseBaseline).toContain('Rust 1.77.2+')
    expect(releaseBaseline).toContain('Visual Studio C++ Build Tools')
    expect(releaseBaseline).toContain('Microsoft Edge WebView2 runtime')
    expect(releaseBaseline).toContain('src-tauri/target/debug/bundle/')
    expect(releaseBaseline).toContain('src-tauri/target/release/bundle/')
    expect(releaseBaseline).toContain('windows-2025')
    expect(releaseBaseline).toContain('BOOTHY_WINDOWS_CERT_PATH')
    expect(releaseBaseline).toContain('BOOTHY_WINDOWS_CERT_BASE64')
    expect(releaseBaseline).toContain('unsigned draft proof')
    expect(releaseBaseline).toContain('input validation only')
    expect(releaseBaseline).toContain('GITHUB_STEP_SUMMARY')
    expect(releaseBaseline).toContain('`workflow_dispatch` from `main`')
    expect(releaseBaseline).toContain('runs only the draft release proof path')
    expect(releaseBaseline).toContain('Promotion state')
    expect(releaseBaseline).toContain('release hold')
    expect(releaseBaseline).toContain('branch rollout safety tests')
    expect(releaseBaseline).toContain('release-baseline-governance.test.ts')
    expect(releaseBaseline).toContain('Hosted GitHub Actions draft proof consumes `BOOTHY_WINDOWS_CERT_BASE64`')
    expect(legacyReleaseBaseline).toBe(releaseBaseline)

    expect(workflow).toContain('runs-on: windows-2025')
    expect(workflow).toContain("github.event_name == 'pull_request'")
    expect(workflow).toContain("github.event_name == 'push' && github.ref == 'refs/heads/main'")
    expect(workflow).toContain("startsWith(github.ref, 'refs/tags/boothy-v')")
    expect(workflow).toContain('pnpm build:desktop')
    expect(workflow).toContain('pnpm release:desktop')
    expect(workflow).toContain('workflow_dispatch release proof is only supported from refs/heads/main')
    expect(workflow).toContain('pnpm test:run src/governance/release-baseline-governance.test.ts')
    expect(workflow).toContain('cargo test --test branch_rollout')
    expect(workflow).toContain('BOOTHY_WINDOWS_CERT_BASE64')
    expect(workflow).toContain('BOOTHY_WINDOWS_CERT_PASSWORD')
    expect(workflow).toContain('BOOTHY_WINDOWS_TIMESTAMP_URL')
    expect(workflow).toContain('Proof path')
    expect(workflow).toContain('Proof outcome')
    expect(workflow).toContain('Promotion state')
    expect(workflow).toContain("github.event_name != 'workflow_dispatch' || github.ref == 'refs/heads/main'")
    expect(workflow).toContain('actions/upload-artifact@v4')
    expect(workflow).toContain('GITHUB_STEP_SUMMARY')

    expect(tauriConfig.bundle?.createUpdaterArtifacts).toBe(false)
  })

  it('keeps the reviewed release workflow guards and evidence lanes intact', () => {
    const workflow = readRepoFile('.github', 'workflows', 'release-windows.yml')
    const signingScript = readRepoFile('scripts', 'prepare-windows-signing.mjs')

    const unsignedStep = extractStepBlock(workflow, 'Run unsigned Windows baseline build')
    const releaseStep = extractStepBlock(workflow, 'Run signing-ready draft build')
    const captureStep = extractStepBlock(workflow, 'Capture release proof summary')
    const uploadProofStep = extractStepBlock(workflow, 'Upload release proof')
    const uploadBundleStep = extractStepBlock(workflow, 'Upload Windows bundle outputs')

    expect(unsignedStep).toContain(
      "if: github.event_name == 'pull_request' || (github.event_name == 'push' && github.ref == 'refs/heads/main')",
    )
    expect(releaseStep).toContain(
      "if: (github.event_name == 'workflow_dispatch' && github.ref == 'refs/heads/main') || startsWith(github.ref, 'refs/tags/boothy-v')",
    )
    expect(releaseStep).toContain(
      'BOOTHY_WINDOWS_CERT_BASE64: ${{ secrets.BOOTHY_WINDOWS_CERT_BASE64 }}',
    )
    expect(releaseStep).toContain(
      'BOOTHY_WINDOWS_CERT_PASSWORD: ${{ secrets.BOOTHY_WINDOWS_CERT_PASSWORD }}',
    )
    expect(releaseStep).toContain(
      'BOOTHY_WINDOWS_TIMESTAMP_URL: ${{ secrets.BOOTHY_WINDOWS_TIMESTAMP_URL }}',
    )

    expect(captureStep).toContain(
      "if: always() && (github.event_name != 'workflow_dispatch' || github.ref == 'refs/heads/main')",
    )
    expect(captureStep).toContain('Proof path')
    expect(captureStep).toContain('Proof outcome')
    expect(captureStep).toContain('Hardware gate status')
    expect(captureStep).toContain('Promotion state')
    expect(captureStep).toContain("$promotionState = 'release-hold'")

    expect(uploadProofStep).toContain(
      "if: always() && (github.event_name != 'workflow_dispatch' || github.ref == 'refs/heads/main')",
    )
    expect(uploadBundleStep).toContain(
      "if: always() && (github.event_name != 'workflow_dispatch' || github.ref == 'refs/heads/main')",
    )

    expect(signingScript).toContain('BOOTHY_WINDOWS_CERT_BASE64')
    expect(signingScript).toContain('BOOTHY_WINDOWS_CERT_PASSWORD')
    expect(signingScript).toContain('BOOTHY_WINDOWS_SIGNING_MODE')
    expect(signingScript).toContain('BOOTHY_WINDOWS_SIGNING_SOURCE')
  })

  it('normalizes signing-ready inputs into the env shape consumed by the release path', async () => {
    const scriptUrl = pathToFileURL(
      resolve(repoRoot, 'scripts', 'prepare-windows-signing.mjs'),
    )
    const { main, resolveSigningInputs } = (await import(scriptUrl.href)) as {
      main: (
        argv?: string[],
        env?: Record<string, string | undefined>,
      ) => Promise<{
        certificateBase64: string | null
        certificatePassword: string | null
        mode: 'unsigned-draft' | 'signing-inputs-present'
        source: 'path' | 'base64' | null
        timestampUrl: string | null
      }>
      resolveSigningInputs: (
        env?: Record<string, string | undefined>,
        options?: {
          allowUnsigned?: boolean
          readFileSync?: (path: string) => string | Uint8Array
        },
      ) => {
        certificateBase64: string | null
        certificatePassword: string | null
        mode: 'unsigned-draft' | 'signing-inputs-present'
        source: 'path' | 'base64' | null
        timestampUrl: string | null
      }
    }

    const resolvedUnsignedDraft = resolveSigningInputs({}, { allowUnsigned: true })

    expect(resolvedUnsignedDraft).toEqual({
      certificateBase64: null,
      certificatePassword: null,
      mode: 'unsigned-draft',
      source: null,
      timestampUrl: null,
    })

    const resolvedFromPath = resolveSigningInputs(
      {
        BOOTHY_WINDOWS_CERT_PATH: 'C:/signing/boothy.pfx',
        BOOTHY_WINDOWS_CERT_PASSWORD: 'super-secret',
        BOOTHY_WINDOWS_TIMESTAMP_URL: 'https://timestamp.example.test',
      },
      {
        allowUnsigned: true,
        readFileSync: () => Buffer.from('boothy-cert-bytes', 'utf8'),
      },
    )

    expect(resolvedFromPath).toEqual({
      certificateBase64: Buffer.from('boothy-cert-bytes', 'utf8').toString('base64'),
      certificatePassword: 'super-secret',
      mode: 'signing-inputs-present',
      source: 'path',
      timestampUrl: 'https://timestamp.example.test/',
    })

    const resolvedFromBase64 = resolveSigningInputs(
      {
        BOOTHY_WINDOWS_CERT_BASE64: 'Ym9vdGh5LWNlcnQ=\n',
        BOOTHY_WINDOWS_CERT_PASSWORD: 'super-secret',
      },
      { allowUnsigned: true },
    )

    expect(resolvedFromBase64).toEqual({
      certificateBase64: 'Ym9vdGh5LWNlcnQ=',
      certificatePassword: 'super-secret',
      mode: 'signing-inputs-present',
      source: 'base64',
      timestampUrl: null,
    })

    expect(() =>
      resolveSigningInputs({
        BOOTHY_WINDOWS_CERT_PATH: 'C:/signing/boothy.pfx',
        BOOTHY_WINDOWS_CERT_BASE64: 'Ym9vdGh5LWNlcnQ=',
        BOOTHY_WINDOWS_CERT_PASSWORD: 'super-secret',
      }, { allowUnsigned: true }),
    ).toThrow(/exactly one/i)

    expect(() =>
      resolveSigningInputs(
        {
          BOOTHY_WINDOWS_CERT_BASE64: 'not-valid-base64!',
          BOOTHY_WINDOWS_CERT_PASSWORD: 'super-secret',
        },
        { allowUnsigned: true },
      ),
    ).toThrow(/base64/i)

    expect(() =>
      resolveSigningInputs(
        {
          BOOTHY_WINDOWS_CERT_BASE64: 'Ym9vdGh5LWNlcnQ=',
          BOOTHY_WINDOWS_CERT_PASSWORD: 'super-secret',
          BOOTHY_WINDOWS_TIMESTAMP_URL: 'ftp://timestamp.example.test',
        },
        { allowUnsigned: true },
      ),
    ).toThrow(/http or https/i)

    const githubEnvPath = resolve(repoRoot, 'tmp-release-baseline-github-env.txt')
    const githubOutputPath = resolve(repoRoot, 'tmp-release-baseline-github-output.txt')
    const githubSummaryPath = resolve(repoRoot, 'tmp-release-baseline-summary.txt')

    try {
      const fs = await import('node:fs')
      fs.writeFileSync(githubEnvPath, '')
      fs.writeFileSync(githubOutputPath, '')
      fs.writeFileSync(githubSummaryPath, '')

      await main(
        [],
        {
          BOOTHY_WINDOWS_CERT_BASE64: 'Ym9vdGh5LWNlcnQ=',
          BOOTHY_WINDOWS_CERT_PASSWORD: 'super-secret',
          GITHUB_ENV: githubEnvPath,
          GITHUB_OUTPUT: githubOutputPath,
          GITHUB_STEP_SUMMARY: githubSummaryPath,
        },
      )

      const githubEnv = fs.readFileSync(githubEnvPath, 'utf8')

      expect(githubEnv).toContain('BOOTHY_WINDOWS_SIGNING_MODE=signing-inputs-present')
      expect(githubEnv).toContain('BOOTHY_WINDOWS_SIGNING_SOURCE=base64')
    } finally {
      const fs = await import('node:fs')
      ;[githubEnvPath, githubOutputPath, githubSummaryPath].forEach((path) => {
        if (fs.existsSync(path)) {
          fs.rmSync(path)
        }
      })
    }
  })
})
