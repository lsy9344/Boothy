import { createContext } from 'react'

import type {
  HostErrorEnvelope,
  OperatorAuditQueryResult,
  OperatorRecoveryAction,
  OperatorRecoveryActionResult,
  OperatorRecoverySummary,
} from '../../shared-contracts'

export type OperatorDiagnosticsContextValue = {
  summary: OperatorRecoverySummary | null
  auditHistory: OperatorAuditQueryResult | null
  error: HostErrorEnvelope | null
  isLoading: boolean
  isActing: boolean
  lastActionResult: OperatorRecoveryActionResult | null
  refresh: () => void
  runAction: (action: OperatorRecoveryAction) => void
}

export const OperatorDiagnosticsContext =
  createContext<OperatorDiagnosticsContextValue | null>(null)
