import { appendFileSync, readFileSync } from 'node:fs'
import { spawn } from 'node:child_process'
import { EOL } from 'node:os'
import { fileURLToPath } from 'node:url'

const normalizeString = (value) => {
  if (typeof value !== 'string') {
    return ''
  }

  return value.trim()
}

const normalizeBase64 = (value) => normalizeString(value).replace(/\s+/g, '')

const validateBase64Certificate = (value) => {
  if (!value) {
    return value
  }

  const base64Pattern =
    /^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/

  if (!base64Pattern.test(value)) {
    throw new Error('BOOTHY_WINDOWS_CERT_BASE64 must be valid base64-encoded PFX data.')
  }

  const decoded = Buffer.from(value, 'base64')

  if (decoded.length === 0) {
    throw new Error('BOOTHY_WINDOWS_CERT_BASE64 must decode to non-empty PFX data.')
  }

  return value
}

const validateTimestampUrl = (value) => {
  if (!value) {
    return null
  }

  let parsed

  try {
    parsed = new URL(value)
  } catch {
    throw new Error('BOOTHY_WINDOWS_TIMESTAMP_URL must be a valid absolute URL.')
  }

  if (!['http:', 'https:'].includes(parsed.protocol)) {
    throw new Error('BOOTHY_WINDOWS_TIMESTAMP_URL must use http or https.')
  }

  return parsed.toString()
}

export function resolveSigningInputs(env = process.env, options = {}) {
  const { allowUnsigned = false, readFileSync: readCertificate = readFileSync } = options
  const certificatePath = normalizeString(env.BOOTHY_WINDOWS_CERT_PATH)
  const certificateBase64 = validateBase64Certificate(
    normalizeBase64(env.BOOTHY_WINDOWS_CERT_BASE64),
  )
  const certificatePassword = normalizeString(env.BOOTHY_WINDOWS_CERT_PASSWORD)
  const timestampUrl = validateTimestampUrl(
    normalizeString(env.BOOTHY_WINDOWS_TIMESTAMP_URL),
  )

  if (!certificatePath && !certificateBase64) {
    if (allowUnsigned) {
      return {
        certificateBase64: null,
        certificatePassword: null,
        mode: 'unsigned-draft',
        source: null,
        timestampUrl: null,
      }
    }

    throw new Error(
      'Expected exactly one signing source: BOOTHY_WINDOWS_CERT_PATH or BOOTHY_WINDOWS_CERT_BASE64.',
    )
  }

  if (certificatePath && certificateBase64) {
    throw new Error(
      'Expected exactly one signing source: BOOTHY_WINDOWS_CERT_PATH or BOOTHY_WINDOWS_CERT_BASE64.',
    )
  }

  if (!certificatePassword) {
    throw new Error(
      'BOOTHY_WINDOWS_CERT_PASSWORD is required whenever signing inputs are supplied.',
    )
  }

  if (certificatePath) {
    const certificateBytes = Buffer.from(readCertificate(certificatePath))

    if (certificateBytes.length === 0) {
      throw new Error('BOOTHY_WINDOWS_CERT_PATH must point to a non-empty PFX file.')
    }

    return {
      certificateBase64: certificateBytes.toString('base64'),
      certificatePassword,
      mode: 'signing-inputs-present',
      source: 'path',
      timestampUrl,
    }
  }

  return {
    certificateBase64,
    certificatePassword,
    mode: 'signing-inputs-present',
    source: 'base64',
    timestampUrl,
  }
}

const buildSummary = (resolved) => {
  const lines = [
    '### release:desktop signing context',
    `- mode: ${resolved.mode}`,
    `- source: ${resolved.source ?? 'none'}`,
    `- timestamp URL: ${resolved.timestampUrl ?? 'not provided'}`,
  ]

  if (resolved.mode === 'unsigned-draft') {
    lines.push('- note: unsigned draft proof continues because no signing input was supplied.')
  } else {
    lines.push('- note: signing inputs were validated, but this baseline still treats release proof as input validation only.')
  }

  return `${lines.join(EOL)}${EOL}`
}

const appendGithubStepSummary = (summary, env = process.env) => {
  if (!env.GITHUB_STEP_SUMMARY) {
    return
  }

  appendFileSync(env.GITHUB_STEP_SUMMARY, summary, 'utf8')
}

const appendGithubOutput = (resolved, env = process.env) => {
  if (!env.GITHUB_OUTPUT) {
    return
  }

  const output = [
    `mode=${resolved.mode}`,
    `source=${resolved.source ?? 'none'}`,
    `timestamp_url=${resolved.timestampUrl ?? ''}`,
  ]

  appendFileSync(env.GITHUB_OUTPUT, `${output.join(EOL)}${EOL}`, 'utf8')
}

const appendGithubEnv = (resolved, env = process.env) => {
  if (!env.GITHUB_ENV) {
    return
  }

  const output = [
    `BOOTHY_WINDOWS_SIGNING_MODE=${resolved.mode}`,
    `BOOTHY_WINDOWS_SIGNING_SOURCE=${resolved.source ?? 'none'}`,
    `BOOTHY_WINDOWS_SIGNING_TIMESTAMP_URL=${resolved.timestampUrl ?? ''}`,
  ]

  appendFileSync(env.GITHUB_ENV, `${output.join(EOL)}${EOL}`, 'utf8')
}

const parseCliArgs = (argv) => {
  const separatorIndex = argv.indexOf('--')
  const flagArgs = separatorIndex >= 0 ? argv.slice(0, separatorIndex) : argv
  const commandArgs = separatorIndex >= 0 ? argv.slice(separatorIndex + 1) : []

  return {
    allowUnsigned: flagArgs.includes('--allow-unsigned'),
    command: commandArgs.join(' '),
  }
}

const runCommand = (command, env) =>
  new Promise((resolve, reject) => {
    const child = spawn(command, {
      env,
      shell: true,
      stdio: 'inherit',
    })

    child.on('error', reject)
    child.on('exit', (code) => {
      if (code === 0) {
        resolve()
        return
      }

      reject(new Error(`Command failed with exit code ${code ?? 'unknown'}.`))
    })
  })

export async function main(argv = process.argv.slice(2), env = process.env) {
  const { allowUnsigned, command } = parseCliArgs(argv)
  const resolved = resolveSigningInputs(env, { allowUnsigned })

  env.BOOTHY_WINDOWS_SIGNING_MODE = resolved.mode
  env.BOOTHY_WINDOWS_SIGNING_SOURCE = resolved.source ?? 'none'

  if (resolved.timestampUrl) {
    env.BOOTHY_WINDOWS_SIGNING_TIMESTAMP_URL = resolved.timestampUrl
  }

  appendGithubStepSummary(buildSummary(resolved), env)
  appendGithubOutput(resolved, env)
  appendGithubEnv(resolved, env)

  if (!command) {
    return resolved
  }

  await runCommand(command, env)
  return resolved
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error))
    process.exit(1)
  })
}
