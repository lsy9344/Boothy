import { createContext } from 'react'

import { branchConfigDefaults, type BranchConfig } from './services/branchConfigStore.js'

export type BranchConfigContextValue = {
  config: BranchConfig
  status: 'loading' | 'ready'
}

export const branchConfigDefaultState: BranchConfigContextValue = {
  config: branchConfigDefaults,
  status: 'loading',
}

export const BranchConfigContext = createContext<BranchConfigContextValue>(branchConfigDefaultState)
