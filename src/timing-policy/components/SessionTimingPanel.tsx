import { useEffect, useEffectEvent, useState } from 'react'

import type { SessionTimingSnapshot } from '../../shared-contracts'

type SessionTimingPanelProps = {
  timing: SessionTimingSnapshot
  canCapture: boolean
}

function formatRemaining(totalSeconds: number) {
  const safeSeconds = Math.max(0, totalSeconds)
  const hours = Math.floor(safeSeconds / 3600)
  const minutes = Math.floor((safeSeconds % 3600) / 60)
  const seconds = safeSeconds % 60

  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`
  }

  return `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`
}

function formatEndTime(adjustedEndAt: string) {
  const parsed = new Date(adjustedEndAt)

  if (Number.isNaN(parsed.getTime())) {
    return '종료 시각 확인 중'
  }

  return new Intl.DateTimeFormat('ko-KR', {
    hour: 'numeric',
    minute: '2-digit',
  }).format(parsed)
}

function selectTimingCopy(
  timing: SessionTimingSnapshot,
  canCapture: boolean,
) {
  switch (timing.phase) {
    case 'warning':
      return {
        badge: '곧 종료돼요',
        headline: '종료 5분 전이에요.',
        detail: canCapture
          ? '남은 시간 안에는 계속 촬영할 수 있어요.'
          : '지금 상태를 마무리한 뒤 다음 안내를 확인해 주세요.',
      }
    case 'ended':
      return {
        badge: '촬영 종료',
        headline: '촬영 시간이 끝났어요.',
        detail: '이제는 새 촬영이 멈춰요. 다음 안내가 나올 때까지 잠시만 기다려 주세요.',
      }
    case 'active':
    default:
      return {
        badge: '세션 진행 중',
        headline: '종료 시각이 고정되어 있어요.',
        detail: '남은 시간을 보면서 여유 있게 촬영을 마무리할 수 있어요.',
      }
  }
}

export function SessionTimingPanel({
  timing,
  canCapture,
}: SessionTimingPanelProps) {
  const [nowMs, setNowMs] = useState(() => Date.now())
  const syncNow = useEffectEvent(() => {
    setNowMs(Date.now())
  })

  useEffect(() => {
    syncNow()

    const intervalId = globalThis.setInterval(() => {
      syncNow()
    }, 1000)

    return () => {
      globalThis.clearInterval(intervalId)
    }
  }, [timing.sessionId, timing.adjustedEndAt])

  const endAtMs = Date.parse(timing.adjustedEndAt)
  const remainingSeconds = Number.isNaN(endAtMs)
    ? 0
    : Math.max(0, Math.ceil((endAtMs - nowMs) / 1000))
  const copy = selectTimingCopy(timing, canCapture)

  return (
    <article
      className={`surface-card session-timing-panel session-timing-panel--${timing.phase}`}
    >
      <div className="session-timing-panel__header">
        <p className="session-timing-panel__badge">{copy.badge}</p>
        <div>
          <h2>{copy.headline}</h2>
          <p>{copy.detail}</p>
        </div>
      </div>

      <div className="session-timing-panel__grid">
        <div>
          <p className="session-timing-panel__label">종료 시각</p>
          <p className="session-timing-panel__value">{formatEndTime(timing.adjustedEndAt)}</p>
        </div>
        <div>
          <p className="session-timing-panel__label">남은 시간</p>
          <p
            aria-live="polite"
            className="session-timing-panel__value session-timing-panel__value--countdown"
          >
            {formatRemaining(remainingSeconds)}
          </p>
        </div>
      </div>
    </article>
  )
}
