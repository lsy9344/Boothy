import type { ReactNode } from 'react'
import { startTransition, useEffect, useEffectEvent, useRef, useState } from 'react'

import type {
  HostErrorEnvelope,
  OperatorAuditQueryResult,
  OperatorRecoveryAction,
  OperatorRecoveryActionResult,
  OperatorRecoverySummary,
} from '../../shared-contracts'
import {
  createOperatorDiagnosticsService,
  type OperatorDiagnosticsService,
} from '../services/operator-diagnostics-service'
import { OperatorDiagnosticsContext } from './operator-diagnostics-context'

type OperatorDiagnosticsProviderProps = {
  children: ReactNode
  operatorDiagnosticsService?: OperatorDiagnosticsService
}

export function OperatorDiagnosticsProvider({
  children,
  operatorDiagnosticsService,
}: OperatorDiagnosticsProviderProps) {
  const operatorDiagnosticsServiceRef = useRef(
    operatorDiagnosticsService ?? createOperatorDiagnosticsService(),
  )
  const requestVersionRef = useRef(0)
  const actionRequestVersionRef = useRef(0)
  const [summary, setSummary] = useState<OperatorRecoverySummary | null>(null)
  const [auditHistory, setAuditHistory] = useState<OperatorAuditQueryResult | null>(null)
  const [error, setError] = useState<HostErrorEnvelope | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [isActing, setIsActing] = useState(false)
  const [lastActionResult, setLastActionResult] =
    useState<OperatorRecoveryActionResult | null>(null)

  function commitSuccess(
    requestVersion: number,
    nextSummary: OperatorRecoverySummary,
    nextAuditHistory: OperatorAuditQueryResult | null,
  ) {
    if (requestVersion !== requestVersionRef.current) {
      return
    }

    startTransition(() => {
      setSummary(nextSummary)
      setAuditHistory(nextAuditHistory)
      setLastActionResult(null)
      setError(null)
      setIsLoading(false)
    })
  }

  function commitFailure(requestVersion: number, nextError: HostErrorEnvelope) {
    if (requestVersion !== requestVersionRef.current) {
      return
    }

    startTransition(() => {
      setSummary(null)
      setAuditHistory(null)
      setLastActionResult(null)
      setError(nextError)
      setIsLoading(false)
    })
  }

  async function refresh() {
    const requestVersion = requestVersionRef.current + 1

    requestVersionRef.current = requestVersion
    startTransition(() => {
      setError(null)
      setIsLoading(true)
      setLastActionResult(null)
    })

    try {
      const nextSummary =
        await operatorDiagnosticsServiceRef.current.loadOperatorRecoverySummary()
      const nextAuditHistory =
        await operatorDiagnosticsServiceRef.current.loadOperatorAuditHistory({
          sessionId: nextSummary.sessionId ?? null,
          eventCategories: [],
          limit: 10,
        })

      commitSuccess(requestVersion, nextSummary, nextAuditHistory)
    } catch (error) {
      commitFailure(requestVersion, error as HostErrorEnvelope)
    }
  }

  async function runAction(action: OperatorRecoveryAction) {
    const sessionId = summary?.sessionId

    if (sessionId === null || sessionId === undefined) {
      return
    }

    const requestVersion = actionRequestVersionRef.current + 1

    actionRequestVersionRef.current = requestVersion
    startTransition(() => {
      setIsActing(true)
      setError(null)
    })

    try {
      const result = await operatorDiagnosticsServiceRef.current.runOperatorRecoveryAction({
        sessionId,
        action,
      })
      const nextAuditHistory =
        await operatorDiagnosticsServiceRef.current.loadOperatorAuditHistory({
          sessionId: result.summary.sessionId ?? null,
          eventCategories: [],
          limit: 10,
        })

      if (requestVersion !== actionRequestVersionRef.current) {
        return
      }

      startTransition(() => {
        setSummary(result.summary)
        setAuditHistory(nextAuditHistory)
        setLastActionResult(result)
        setError(null)
        setIsActing(false)
      })
    } catch (error) {
      if (requestVersion !== actionRequestVersionRef.current) {
        return
      }

      startTransition(() => {
        setLastActionResult(null)
        setError(error as HostErrorEnvelope)
        setIsActing(false)
      })
    }
  }

  const refreshOnMount = useEffectEvent(() => {
    void refresh()
  })

  useEffect(() => {
    refreshOnMount()
  }, [])

  return (
    <OperatorDiagnosticsContext.Provider
      value={{
        summary,
        auditHistory,
        error,
        isLoading,
        isActing,
        lastActionResult,
        refresh,
        runAction,
      }}
    >
      {children}
    </OperatorDiagnosticsContext.Provider>
  )
}
