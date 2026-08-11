import { useRef } from 'react'

import { useViewerReadiness } from './state/use-viewer-readiness'
import type { ViewerHostService } from './services/viewer-host-adapter'

type ViewerSurfaceProps = {
  hostService?: ViewerHostService
}

/**
 * 전용 관람 화면.
 *
 * Story 7.1 범위에서는 사진을 표시하지 않는다. 사진이 놓일 자리(photo rectangle)를 확정하고
 * 그 자리에 필요한 픽셀 수를 host 계약으로 보고하는 것까지만 소유한다.
 *
 * read-only 계약: 이 surface에는 조작 요소(button/link/textbox/menuitem)와 진단 표시가 없어야 한다.
 * 준비 실패는 booth control surface의 wait/call guidance로만 전달한다.
 */
export function ViewerSurface({ hostService }: ViewerSurfaceProps) {
  const stageRef = useRef<HTMLDivElement>(null)
  const photoRef = useRef<HTMLDivElement>(null)

  useViewerReadiness({ stageRef, photoRef, hostService })

  return (
    <main className="viewer-surface">
      <div className="viewer-surface__stage" ref={stageRef}>
        <div className="viewer-surface__photo" ref={photoRef} />
      </div>
      <p className="viewer-surface__standby">사진이 준비되면 여기에 보여드릴게요.</p>
    </main>
  )
}
