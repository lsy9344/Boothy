import { useContext } from 'react'

import { OperatorDiagnosticsContext } from './operator-diagnostics-context'

export function useOperatorDiagnostics() {
  const value = useContext(OperatorDiagnosticsContext)

  if (value === null) {
    throw new Error('OperatorDiagnosticsProvider가 필요해요.')
  }

  return value
}
