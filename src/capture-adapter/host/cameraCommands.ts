import { invoke } from '@tauri-apps/api/core'

export type InvokeCommand = <TResult>(command: string, args: Record<string, unknown>) => Promise<TResult>

export const cameraCommandNames = {
  runReadinessFlow: 'camera_run_readiness_flow',
  getReadinessSnapshot: 'get_camera_readiness_snapshot',
  watchReadiness: 'watch_camera_readiness',
  unwatchReadiness: 'unwatch_camera_readiness',
  getCaptureConfidenceSnapshot: 'get_capture_confidence_snapshot',
  watchCaptureConfidence: 'watch_capture_confidence',
  unwatchCaptureConfidence: 'unwatch_capture_confidence',
  requestCapture: 'request_capture',
} as const

export function invokeHostCommand<TResult>(command: string, args: Record<string, unknown>) {
  return invoke<TResult>(command, args)
}
