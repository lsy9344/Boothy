import { useContext } from 'react'

import { BranchConfigContext } from './BranchConfigContext.js'

export function useBranchConfig() {
  return useContext(BranchConfigContext)
}
