import type { NormalizedErrorEnvelope } from '../../shared-contracts/dto/errorEnvelope.js'

const customerMessages = {
  cameraReconnectNeeded: '카메라 연결을 다시 확인하고 있어요.',
  cameraUnavailable: '지금은 촬영 준비를 계속할 수 없어요. 직원 호출을 부탁드릴게요.',
} as const

export function mapCameraErrorToViewModel(error: NormalizedErrorEnvelope) {
  return {
    customer: {
      state: error.customerState,
      connectionState: error.customerCameraConnectionState,
      message: customerMessages[error.customerState],
    },
    operator: {
      connectionState: error.operatorCameraConnectionState,
      action: error.operatorAction,
      message: error.message,
      details: error.details,
    },
  }
}
