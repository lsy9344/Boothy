import type { SessionErrorCode } from '../../shared-contracts/dto/session.js'

export const sessionStartErrorCopy: Record<SessionErrorCode, string> = {
  'session_name.required': '세션 이름을 입력해 주세요.',
  'session.validation_failed': '세션 정보를 확인한 뒤 다시 시도해 주세요.',
  'session.provisioning_failed': '세션을 시작하지 못했어요. 잠시 후 다시 시도해 주세요.',
}
