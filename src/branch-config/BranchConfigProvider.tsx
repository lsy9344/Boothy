import { startTransition, useEffect, useState, type ReactNode } from 'react'

import { BranchConfigContext, branchConfigDefaultState } from './BranchConfigContext.js'
import { loadBranchConfig, type BranchConfig } from './services/branchConfigStore.js'

export function BranchConfigProvider({ children }: { children: ReactNode }) {
  const [value, setValue] = useState(branchConfigDefaultState)

  useEffect(() => {
    let isMounted = true

    void loadBranchConfig().then((config: BranchConfig) => {
      if (!isMounted) {
        return
      }

      startTransition(() => {
        setValue({
          config,
          status: 'ready',
        })
      })
    })

    return () => {
      isMounted = false
    }
  }, [])

  return <BranchConfigContext.Provider value={value}>{children}</BranchConfigContext.Provider>
}
