import readline from 'node:readline'
import { mkdir, writeFile } from 'node:fs/promises'
import path from 'node:path'

const input = readline.createInterface({
  input: process.stdin,
  crlfDelay: Infinity,
})

const protocolVersion = 'boothy.camera.protocol.v1'
const contractVersion = 'boothy.camera.contract.v1'
const errorEnvelopeVersion = 'boothy.camera.error-envelope.v1'

function emit(message) {
  process.stdout.write(`${JSON.stringify(message)}\n`)
}

async function writeCaptureArtifacts(payload) {
  const originalOutputPath = payload?.originalOutputPath
  const processedOutputPath = payload?.processedOutputPath

  if (!originalOutputPath || !processedOutputPath) {
    return
  }

  await mkdir(path.dirname(originalOutputPath), { recursive: true })
  await mkdir(path.dirname(processedOutputPath), { recursive: true })
  await writeFile(originalOutputPath, Buffer.from('mock-sidecar-original'))
  await writeFile(processedOutputPath, Buffer.from('mock-sidecar-processed'))
}

function handleReadiness(request) {
  const baseEvent = {
    schemaVersion: protocolVersion,
    requestId: request.requestId,
    correlationId: request.correlationId,
    sessionId: request.sessionId,
  }
  const mockScenario = request.payload?.mockScenario ?? 'readinessSuccess'

  if (mockScenario === 'readinessDegraded') {
    emit({
      ...baseEvent,
      event: 'camera.statusChanged',
      payload: {
        connectionState: 'reconnecting',
        readiness: 'degraded',
        lastUpdatedAt: '2026-03-08T09:00:00.000Z',
      },
    })
    emit({
      schemaVersion: protocolVersion,
      requestId: request.requestId,
      correlationId: request.correlationId,
      ok: true,
      status: {
        connectionState: 'reconnecting',
        readiness: 'degraded',
        lastUpdatedAt: '2026-03-08T09:00:00.000Z',
      },
    })
    return
  }

  if (mockScenario === 'normalizedError') {
    emit({
      schemaVersion: protocolVersion,
      requestId: request.requestId,
      correlationId: request.correlationId,
      ok: false,
      error: {
        schemaVersion: errorEnvelopeVersion,
        code: 'camera.reconnecting',
        severity: 'warning',
        retryable: true,
        customerState: 'cameraReconnectNeeded',
        customerCameraConnectionState: 'needsAttention',
        operatorCameraConnectionState: 'reconnecting',
        operatorAction: 'checkCableAndRetry',
        message: 'Camera connection is unstable.',
        details: 'PTP session reopened after an adapter timeout.',
      },
    })
    return
  }

  emit({
    ...baseEvent,
    event: 'camera.statusChanged',
    payload: {
      connectionState: 'connected',
      readiness: 'ready',
      lastUpdatedAt: '2026-03-08T09:00:00.000Z',
    },
  })
  emit({
    schemaVersion: protocolVersion,
    requestId: request.requestId,
    correlationId: request.correlationId,
    ok: true,
    status: {
      connectionState: 'connected',
      readiness: 'ready',
      lastUpdatedAt: '2026-03-08T09:00:00.000Z',
    },
  })
}

function emitReadinessWatchMessage(request) {
  const mockScenario = request.payload?.mockScenario ?? 'readinessSuccess'

  if (mockScenario === 'normalizedError') {
    emit({
      schemaVersion: protocolVersion,
      requestId: request.requestId,
      correlationId: request.correlationId,
      ok: false,
      error: {
        schemaVersion: errorEnvelopeVersion,
        code: 'camera.reconnecting',
        severity: 'warning',
        retryable: true,
        customerState: 'cameraReconnectNeeded',
        customerCameraConnectionState: 'needsAttention',
        operatorCameraConnectionState: 'reconnecting',
        operatorAction: 'checkCableAndRetry',
        message: 'Camera connection is unstable.',
        details: 'PTP session reopened after an adapter timeout.',
      },
    })
    return
  }

  if (mockScenario === 'readinessDegraded') {
    emit({
      schemaVersion: protocolVersion,
      requestId: request.requestId,
      correlationId: request.correlationId,
      ok: true,
      status: {
        connectionState: 'reconnecting',
        readiness: 'degraded',
        lastUpdatedAt: '2026-03-08T09:00:00.000Z',
      },
    })
    return
  }

  emit({
    schemaVersion: protocolVersion,
    requestId: request.requestId,
    correlationId: request.correlationId,
    ok: true,
    status: {
      connectionState: 'connected',
      readiness: 'ready',
      lastUpdatedAt: '2026-03-08T09:00:00.000Z',
    },
  })
}

function handleReadinessWatch(request) {
  emitReadinessWatchMessage(request)
  const timer = setInterval(() => {
    emitReadinessWatchMessage(request)
  }, 750)

  const stopWatching = () => {
    clearInterval(timer)
    process.exit(0)
  }

  input.once('close', stopWatching)
  process.once('SIGTERM', stopWatching)
}

async function handleCapture(request) {
  const captureId = request.payload?.captureId ?? 'capture-001'
  const originalFileName = request.payload?.originalFileName ?? `originals/${captureId}.nef`
  const processedFileName = request.payload?.processedFileName ?? `${captureId}.png`
  const capturedAt = '2026-03-08T09:00:04.000Z'
  const manifestPath = request.payload?.originalOutputPath
    ? path.join(path.dirname(path.dirname(request.payload.originalOutputPath)), 'session.json').replaceAll('\\', '/')
    : `mock://${request.sessionId}/session.json`

  await writeCaptureArtifacts(request.payload)

  emit({
    schemaVersion: protocolVersion,
    requestId: request.requestId,
    correlationId: request.correlationId,
    event: 'capture.progress',
    sessionId: request.sessionId,
    payload: {
      stage: 'captureStarted',
      captureId,
      percentComplete: 0,
      lastUpdatedAt: '2026-03-08T09:00:03.000Z',
    },
  })
  emit({
    schemaVersion: protocolVersion,
    requestId: request.requestId,
    correlationId: request.correlationId,
    event: 'capture.progress',
    sessionId: request.sessionId,
    payload: {
      stage: 'captureCompleted',
      captureId,
      percentComplete: 100,
      lastUpdatedAt: capturedAt,
    },
  })
  emit({
    schemaVersion: contractVersion,
    requestId: request.requestId,
    correlationId: request.correlationId,
    ok: true,
    sessionId: request.sessionId,
    captureId,
    originalFileName,
    processedFileName,
    capturedAt,
    manifestPath,
  })
}

input.on('line', async (line) => {
  if (!line.trim()) {
    return
  }

  const request = JSON.parse(line)

  if (request.method === 'camera.checkReadiness') {
    handleReadiness(request)
    return
  }

  if (request.method === 'camera.watchReadiness') {
    handleReadinessWatch(request)
    return
  }

  if (request.method === 'camera.capture') {
    await handleCapture(request)
  }
})
