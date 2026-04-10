import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, statSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)
const repoRoot = path.resolve(__dirname, '..')
const sourcePath = path.join(repoRoot, 'sidecar', 'dedicated-renderer', 'main.rs')

function resolveTargetArtifact() {
  if (process.platform === 'win32' && process.arch === 'x64') {
    return 'boothy-dedicated-renderer-x86_64-pc-windows-msvc.exe'
  }

  if (process.platform === 'linux' && process.arch === 'x64') {
    return 'boothy-dedicated-renderer-x86_64-unknown-linux-gnu'
  }

  if (process.platform === 'darwin' && process.arch === 'arm64') {
    return 'boothy-dedicated-renderer-aarch64-apple-darwin'
  }

  if (process.platform === 'darwin' && process.arch === 'x64') {
    return 'boothy-dedicated-renderer-x86_64-apple-darwin'
  }

  throw new Error(
    `Unsupported host for dedicated renderer stub: ${process.platform} ${process.arch}`,
  )
}

const outputPath = path.join(
  repoRoot,
  'sidecar',
  'dedicated-renderer',
  resolveTargetArtifact(),
)

const sourceStat = statSync(sourcePath)
const outputStat = existsSync(outputPath) ? statSync(outputPath) : null

if (outputStat && outputStat.mtimeMs >= sourceStat.mtimeMs) {
  console.log(`dedicated renderer stub is ready: ${path.relative(repoRoot, outputPath)}`)
  process.exit(0)
}

mkdirSync(path.dirname(outputPath), { recursive: true })

const compile = spawnSync(
  'rustc',
  [sourcePath, '--edition=2021', '-C', 'opt-level=0', '-o', outputPath],
  {
    cwd: repoRoot,
    stdio: 'inherit',
  },
)

if (compile.status !== 0) {
  process.exit(compile.status ?? 1)
}

console.log(`prepared dedicated renderer stub: ${path.relative(repoRoot, outputPath)}`)
