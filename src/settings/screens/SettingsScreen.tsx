import { BranchRolloutPanel } from '../../branch-config/components/BranchRolloutPanel'
import { PreviewRouteGovernancePanel } from '../../branch-config/components/PreviewRouteGovernancePanel'
import {
  createBranchRolloutService,
  type BranchRolloutService,
} from '../../branch-config/services/branch-rollout-service'
import { SurfaceLayout } from '../../shared-ui/layout/SurfaceLayout'

type SettingsScreenProps = {
  branchRolloutService?: BranchRolloutService
}

export function SettingsScreen({
  branchRolloutService = createBranchRolloutService(),
}: SettingsScreenProps) {
  return (
    <SurfaceLayout
      eyebrow="Settings"
      title="Settings Governance"
      description="지점별 rollout과 preview route 승격을 settings surface 안에서만 승인하고, 진행 중인 세션은 안전한 전환 시점까지 보호합니다."
    >
      <BranchRolloutPanel branchRolloutService={branchRolloutService} />
      <PreviewRouteGovernancePanel branchRolloutService={branchRolloutService} />
    </SurfaceLayout>
  )
}
