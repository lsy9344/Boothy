export const DEFAULT_OPERATIONAL_BRANCH_ID = 'branch-unconfigured'

export function resolveOperationalBranchId(branchId: string | null | undefined): string {
  const normalizedBranchId = branchId?.trim()
  return normalizedBranchId && normalizedBranchId.length > 0
    ? normalizedBranchId
    : DEFAULT_OPERATIONAL_BRANCH_ID
}
