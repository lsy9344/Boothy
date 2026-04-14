import type { ReactNode } from 'react'
import { Component } from 'react'

import { SurfaceLayout } from '../../shared-ui/layout/SurfaceLayout'

type AppErrorBoundaryProps = {
  children: ReactNode
  onError?: (error: Error) => void
}

type AppErrorBoundaryState = {
  error: Error | null
}

export class AppErrorBoundary extends Component<
  AppErrorBoundaryProps,
  AppErrorBoundaryState
> {
  state: AppErrorBoundaryState = {
    error: null,
  }

  static getDerivedStateFromError(error: Error): AppErrorBoundaryState {
    return {
      error,
    }
  }

  componentDidCatch(error: Error) {
    this.props.onError?.(error)
  }

  render() {
    if (this.state.error !== null) {
      return (
        <SurfaceLayout
          eyebrow="App Status"
          title="앱을 다시 불러오는 중이에요"
          description="초기 화면을 준비하는 중 문제가 생겨 현재 상태를 확인하고 있어요."
        >
          <article className="surface-card">
            <h2>초기화 오류</h2>
            <p>{this.state.error.message}</p>
          </article>
        </SurfaceLayout>
      )
    }

    return this.props.children
  }
}
