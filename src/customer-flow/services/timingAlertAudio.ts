export type TimingAlertAudioKind = 'warning' | 'ended'

export type TimingAlertAudioService = {
  play(kind: TimingAlertAudioKind): Promise<void>
}

type AudioLike = {
  play(): Promise<void>
  preload: string
}

type CreateAudioFn = (src: string) => AudioLike

function writeAscii(view: DataView, offset: number, value: string) {
  for (let index = 0; index < value.length; index += 1) {
    view.setUint8(offset + index, value.charCodeAt(index))
  }
}

function createToneWaveBlobUrl(frequencyHz: number, durationMs: number) {
  const sampleRate = 8_000
  const sampleCount = Math.max(1, Math.round((sampleRate * durationMs) / 1_000))
  const bytesPerSample = 2
  const channelCount = 1
  const dataSize = sampleCount * bytesPerSample
  const buffer = new ArrayBuffer(44 + dataSize)
  const view = new DataView(buffer)

  writeAscii(view, 0, 'RIFF')
  view.setUint32(4, 36 + dataSize, true)
  writeAscii(view, 8, 'WAVE')
  writeAscii(view, 12, 'fmt ')
  view.setUint32(16, 16, true)
  view.setUint16(20, 1, true)
  view.setUint16(22, channelCount, true)
  view.setUint32(24, sampleRate, true)
  view.setUint32(28, sampleRate * channelCount * bytesPerSample, true)
  view.setUint16(32, channelCount * bytesPerSample, true)
  view.setUint16(34, 16, true)
  writeAscii(view, 36, 'data')
  view.setUint32(40, dataSize, true)

  for (let index = 0; index < sampleCount; index += 1) {
    const sample =
      Math.sin((2 * Math.PI * frequencyHz * index) / sampleRate) *
      (1 - index / sampleCount) *
      0.35
    view.setInt16(44 + index * bytesPerSample, Math.round(sample * 32_767), true)
  }

  return URL.createObjectURL(new Blob([buffer], { type: 'audio/wav' }))
}

function resolveTone(kind: TimingAlertAudioKind) {
  if (kind === 'warning') {
    return {
      durationMs: 180,
      frequencyHz: 784,
    }
  }

  return {
    durationMs: 320,
    frequencyHz: 440,
  }
}

export function createTimingAlertAudio(
  createAudio: CreateAudioFn = (src) => new Audio(src),
): TimingAlertAudioService {
  return {
    async play(kind) {
      if ((globalThis.navigator?.userAgent ?? '').toLowerCase().includes('jsdom')) {
        return
      }

      const tone = resolveTone(kind)
      const src = createToneWaveBlobUrl(tone.frequencyHz, tone.durationMs)
      const audio = createAudio(src)
      audio.preload = 'auto'

      try {
        await audio.play()
      } finally {
        globalThis.setTimeout(() => {
          URL.revokeObjectURL(src)
        }, Math.max(1_000, tone.durationMs * 4))
      }
    },
  }
}

export const timingAlertAudio = createTimingAlertAudio()
