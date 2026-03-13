import { SessionFlowProvider } from '../../session-domain/state/SessionFlowProvider.js'
import { CustomerFlowContent } from './CustomerFlowScreen.js'

export function CustomerEntryContent() {
  return <CustomerFlowContent />
}

export function CustomerEntryScreen() {
  return (
    <SessionFlowProvider>
      <CustomerEntryContent />
    </SessionFlowProvider>
  )
}
