export type SessionTimeDisplay = {
  label: string
  value: string
  supporting: string
}

export const sessionTimeDisplayLabel = '촬영 종료 시간'

const sessionTimeFormatter = new Intl.DateTimeFormat('ko-KR', {
  hour: 'numeric',
  minute: '2-digit',
  hour12: true,
  timeZone: 'Asia/Seoul',
})

export function selectSessionTimeDisplay(shootEndsAt: string): SessionTimeDisplay {
  return {
    label: sessionTimeDisplayLabel,
    value: sessionTimeFormatter.format(new Date(shootEndsAt)),
    supporting: '이 시간까지 촬영할 수 있어요.',
  }
}
