type CueKind = 'warning' | 'ended'

function getAudioContextConstructor() {
  if (typeof window === 'undefined') {
    return null
  }

  return window.AudioContext ?? (window as typeof window & {
    webkitAudioContext?: typeof AudioContext
  }).webkitAudioContext ?? null
}

export async function playTimingCue(kind: CueKind) {
  const AudioContextConstructor = getAudioContextConstructor()

  if (AudioContextConstructor === null) {
    return
  }

  const context = new AudioContextConstructor()
  const tones =
    kind === 'warning'
      ? [
          { frequency: 880, durationMs: 120, delayMs: 0 },
          { frequency: 1046, durationMs: 140, delayMs: 170 },
        ]
      : [
          { frequency: 523, durationMs: 180, delayMs: 0 },
          { frequency: 392, durationMs: 220, delayMs: 230 },
          { frequency: 262, durationMs: 320, delayMs: 520 },
        ]

  try {
    if (context.state === 'suspended') {
      await context.resume()
    }

    const baseTime = context.currentTime

    for (const tone of tones) {
      const oscillator = context.createOscillator()
      const gainNode = context.createGain()
      const startAt = baseTime + tone.delayMs / 1000
      const endAt = startAt + tone.durationMs / 1000

      oscillator.type = 'sine'
      oscillator.frequency.value = tone.frequency
      gainNode.gain.setValueAtTime(0.0001, startAt)
      gainNode.gain.exponentialRampToValueAtTime(0.18, startAt + 0.02)
      gainNode.gain.exponentialRampToValueAtTime(0.0001, endAt)
      oscillator.connect(gainNode)
      gainNode.connect(context.destination)
      oscillator.start(startAt)
      oscillator.stop(endAt)
    }

    const totalDurationMs = Math.max(
      ...tones.map((tone) => tone.delayMs + tone.durationMs),
      0,
    )

    globalThis.setTimeout(() => {
      void context.close().catch(() => undefined)
    }, totalDurationMs + 120)
  } catch {
    void context.close().catch(() => undefined)
  }
}
