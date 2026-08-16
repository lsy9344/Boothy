import type { DisplayPresentReport } from '../shared-contracts'

export type PendingPresentReport = {
  report: DisplayPresentReport
  sessionId: string | null
}

export function enqueuePendingPresentReport(
  pendingReports: PendingPresentReport[],
  nextReport: PendingPresentReport,
): PendingPresentReport[] {
  return [
    ...pendingReports.filter(
      (pending) =>
        pending.report.generationId !== nextReport.report.generationId ||
        pending.report.outcome !== nextReport.report.outcome,
    ),
    nextReport,
  ]
}
