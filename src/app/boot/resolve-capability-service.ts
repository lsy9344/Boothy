import {
  createCapabilityService,
  type CapabilityService,
} from '../services/capability-service'
import type { CapabilitySnapshot } from '../../shared-contracts'

/**
 * host가 응답하지 않아도 UI는 반드시 그려져야 한다. 이 시간이 지나면 기본 권한으로 계속 진행한다.
 */
export const CAPABILITY_SNAPSHOT_TIMEOUT_MS = 3_000

export const VIEWER_WINDOW_LABEL = 'viewer-window'

export function withTimeout<T>(
  promise: Promise<T>,
  timeoutMs: number,
): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timeoutId = globalThis.setTimeout(() => {
      reject(new Error('capability-snapshot-timeout'))
    }, timeoutMs)

    promise.then(
      (value) => {
        globalThis.clearTimeout(timeoutId)
        resolve(value)
      },
      (error) => {
        globalThis.clearTimeout(timeoutId)
        reject(error)
      },
    )
  })
}

/**
 * 부팅 시점의 capability service를 결정한다.
 *
 * 관람 창은 고객 surface라 창 label만으로 접근이 결정되고 privileged snapshot이 필요 없다.
 * 따라서 host IPC를 아예 기다리지 않는다. 기다리면 host가 느리거나 멈췄을 때
 * 고객이 흰 화면을 보게 되고, 이는 어떤 진단 정보보다도 나쁜 결과다.
 *
 * 나머지 창도 무한정 기다리지 않는다. 시간이 지나면 기본 권한으로 렌더링을 진행한다.
 */
export async function resolveBootCapabilityService({
  currentWindowLabel,
  readSnapshot,
  timeoutMs = CAPABILITY_SNAPSHOT_TIMEOUT_MS,
}: {
  currentWindowLabel: string | null
  readSnapshot: () => Promise<CapabilitySnapshot>
  timeoutMs?: number
}): Promise<CapabilityService> {
  if (currentWindowLabel === VIEWER_WINDOW_LABEL) {
    return createCapabilityService({ currentWindowLabel })
  }

  try {
    const capabilitySnapshot = await withTimeout(readSnapshot(), timeoutMs)

    return createCapabilityService({
      ...capabilitySnapshot,
      currentWindowLabel,
    })
  } catch {
    return createCapabilityService({ currentWindowLabel })
  }
}
