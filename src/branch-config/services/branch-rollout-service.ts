import { invoke } from '@tauri-apps/api/core'

import {
  branchRollbackInputSchema,
  branchRolloutActionResultSchema,
  branchRolloutInputSchema,
  branchRolloutOverviewResultSchema,
  hostErrorEnvelopeSchema,
  type BranchRollbackInput,
  type BranchRolloutActionResult,
  type BranchRolloutInput,
  type BranchRolloutOverviewResult,
  type HostErrorEnvelope,
} from '../../shared-contracts'
import { isTauriRuntime } from '../../shared/runtime/is-tauri'

export interface BranchRolloutGateway {
  loadOverview(): Promise<unknown>
  applyRollout(input: BranchRolloutInput): Promise<unknown>
  applyRollback(input: BranchRollbackInput): Promise<unknown>
}

type HostBranchRolloutInput = {
  branchIds: string[]
  targetBuildVersion: string
  targetPresetStackVersion: string
  actorId: string
  actorLabel: string
}

export interface BranchRolloutService {
  loadOverview(): Promise<BranchRolloutOverviewResult>
  applyRollout(input: BranchRolloutInput): Promise<BranchRolloutActionResult>
  applyRollback(input: BranchRollbackInput): Promise<BranchRolloutActionResult>
}

class DefaultBranchRolloutService implements BranchRolloutService {
  private readonly gateway: BranchRolloutGateway

  constructor(gateway: BranchRolloutGateway) {
    this.gateway = gateway
  }

  async loadOverview() {
    try {
      return branchRolloutOverviewResultSchema.parse(await this.gateway.loadOverview())
    } catch (error) {
      throw normalizeHostError(error)
    }
  }

  async applyRollout(input: BranchRolloutInput) {
    const parsedInput = branchRolloutInputSchema.parse(input)

    try {
      return branchRolloutActionResultSchema.parse(
        await this.gateway.applyRollout(parsedInput),
      )
    } catch (error) {
      throw normalizeHostError(error)
    }
  }

  async applyRollback(input: BranchRollbackInput) {
    const parsedInput = branchRollbackInputSchema.parse(input)

    try {
      return branchRolloutActionResultSchema.parse(
        await this.gateway.applyRollback(parsedInput),
      )
    } catch (error) {
      throw normalizeHostError(error)
    }
  }
}

function normalizeHostError(error: unknown): HostErrorEnvelope {
  const parsed = hostErrorEnvelopeSchema.safeParse(error)

  if (parsed.success) {
    if (parsed.data.code === 'capability-denied') {
      return {
        code: parsed.data.code,
        message: '승인된 settings surface에서만 지점 배포 거버넌스를 열 수 있어요.',
      }
    }

    return parsed.data
  }

  return {
    code: 'host-unavailable',
    message: '지금은 지점 배포 거버넌스를 불러올 수 없어요. 잠시 후 다시 시도해 주세요.',
  }
}

function readBrowserFixture() {
  const fixture = (
    globalThis as typeof globalThis & {
      __BOOTHY_BROWSER_BRANCH_ROLLOUT_OVERVIEW__?: unknown
    }
  ).__BOOTHY_BROWSER_BRANCH_ROLLOUT_OVERVIEW__

  if (fixture === undefined) {
    return null
  }

  const parsed = branchRolloutOverviewResultSchema.safeParse(fixture)

  if (parsed.success) {
    return parsed.data
  }

  throw {
    code: 'host-unavailable',
    message: '브라우저 rollout fixture 형식이 올바르지 않아요.',
  } satisfies HostErrorEnvelope
}

export function createBrowserBranchRolloutGateway(): BranchRolloutGateway {
  return {
    async loadOverview() {
      const fixture = readBrowserFixture()

      if (fixture !== null) {
        return fixture
      }

      return {
        schemaVersion: 'branch-rollout-overview/v1',
        approvedBaselines: [],
        branches: [],
        recentHistory: [],
      }
    },
    async applyRollout() {
      throw {
        code: 'host-unavailable',
        message: '브라우저 미리보기에서는 rollout 실행을 지원하지 않아요.',
      } satisfies HostErrorEnvelope
    },
    async applyRollback() {
      throw {
        code: 'host-unavailable',
        message: '브라우저 미리보기에서는 rollback 실행을 지원하지 않아요.',
      } satisfies HostErrorEnvelope
    },
  }
}

export function createTauriBranchRolloutGateway(): BranchRolloutGateway {
  return {
    async loadOverview() {
      return invoke('load_branch_rollout_overview')
    },
    async applyRollout(input) {
      const hostInput: HostBranchRolloutInput = {
        branchIds: input.branchIds,
        targetBuildVersion: input.targetBaseline.buildVersion,
        targetPresetStackVersion: input.targetBaseline.presetStackVersion,
        actorId: input.actorId,
        actorLabel: input.actorLabel,
      }

      return invoke('apply_branch_rollout', { input: hostInput })
    },
    async applyRollback(input) {
      return invoke('apply_branch_rollback', { input })
    },
  }
}

export function createDefaultBranchRolloutGateway() {
  return isTauriRuntime()
    ? createTauriBranchRolloutGateway()
    : createBrowserBranchRolloutGateway()
}

type CreateBranchRolloutServiceOptions = {
  gateway?: BranchRolloutGateway
}

export function createBranchRolloutService({
  gateway = createDefaultBranchRolloutGateway(),
}: CreateBranchRolloutServiceOptions = {}) {
  return new DefaultBranchRolloutService(gateway)
}
