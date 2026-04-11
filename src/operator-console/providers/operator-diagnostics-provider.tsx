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
  const [auditError, setAuditError] = useState<HostErrorEnvelope | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [isActing, setIsActing] = useState(false)
  const [lastActionResult, setLastActionResult] =
    useState<OperatorRecoveryActionResult | null>(null)

  function commitSuccess(
    requestVersion: number,
    nextSummary: OperatorRecoverySummary,
    nextAuditHistory: OperatorAuditQueryResult | null,
    nextAuditError: HostErrorEnvelope | null,
  ) {
    if (requestVersion !== requestVersionRef.current) {
      return
    }

    startTransition(() => {
      setSummary(nextSummary)
      setAuditHistory(nextAuditHistory)
      setLastActionResult(null)
      setError(null)
      setAuditError(nextAuditError)
      setIsLoading(false)
    })
  }

  function commitFailure(
    requestVersion: number,
    nextError: HostErrorEnvelope,
    preserveCurrentView: boolean,
  ) {
    if (requestVersion !== requestVersionRef.current) {
      return
    }

    startTransition(() => {
      if (!preserveCurrentView) {
        setSummary(null)
        setAuditHistory(null)
      }
      setLastActionResult(null)
      setError(nextError)
      setAuditError(null)
      setIsLoading(false)
    })
  }

  async function loadAuditHistory(sessionId: string | null) {
    try {
      const nextAuditHistory =
        await operatorDiagnosticsServiceRef.current.loadOperatorAuditHistory({
          sessionId,
          eventCategories: [],
          limit: 10,
        })

      return {
        auditHistory: nextAuditHistory,
        auditError: null,
      }
    } catch (error) {
      return {
        auditHistory: null,
        auditError: error as HostErrorEnvelope,
      }
    }
  }

  async function refresh() {
    const requestVersion = requestVersionRef.current + 1

    requestVersionRef.current = requestVersion
    startTransition(() => {
      setError(null)
      setAuditError(null)
      setIsLoading(true)
      setLastActionResult(null)
    })

    try {
      const nextSummary =
        await operatorDiagnosticsServiceRef.current.loadOperatorRecoverySummary()
      const { auditHistory: nextAuditHistory, auditError: nextAuditError } =
        await loadAuditHistory(nextSummary.sessionId ?? null)

      commitSuccess(
        requestVersion,
        nextSummary,
        nextAuditHistory,
        nextAuditError,
      )
    } catch (error) {
      commitFailure(requestVersion, error as HostErrorEnvelope, summary !== null)
    }
  }

  async function runAction(action: OperatorRecoveryAction) {
    const sessionId = summary?.sessionId

    if (
      sessionId === null ||
      sessionId === undefined ||
      isLoading ||
      isActing
    ) {
      return
    }

    const requestVersion = actionRequestVersionRef.current + 1

    actionRequestVersionRef.current = requestVersion
    startTransition(() => {
      setIsActing(true)
      setError(null)
      setAuditError(null)
    })

    try {
      const result = await operatorDiagnosticsServiceRef.current.runOperatorRecoveryAction({
        sessionId,
        action,
      })
      const { auditHistory: nextAuditHistory, auditError: nextAuditError } =
        await loadAuditHistory(result.summary.sessionId ?? null)

      if (requestVersion !== actionRequestVersionRef.current) {
        return
      }

      startTransition(() => {
        setSummary(result.summary)
        setAuditHistory(nextAuditHistory)
        setLastActionResult(result)
        setError(null)
        setAuditError(nextAuditError)
        setIsActing(false)
      })
    } catch (error) {
      if (requestVersion !== actionRequestVersionRef.current) {
        return
      }

      startTransition(() => {
        setLastActionResult(null)
        setError(error as HostErrorEnvelope)
        setAuditError(null)
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
        auditError,
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
