import { preparationScreenCopy } from '../copy/preparationScreenCopy.js';
import type { CustomerPreparationState } from '../../session-domain/state/customerPreparationState.js';

export type CustomerCameraStatusCopy = {
  kind: CustomerPreparationState['kind'];
  badge: string;
  title: string;
  supporting: string;
  actionHint: string;
  phoneLabel?: string;
  phoneNumber?: string;
};

export function selectCustomerCameraStatusCopy(state: CustomerPreparationState): CustomerCameraStatusCopy {
  if (state.kind === 'ready') {
    return {
      kind: 'ready',
      ...preparationScreenCopy.ready,
      actionHint: '촬영을 시작할 수 있습니다.',
    };
  }

  if (state.kind === 'phone-required') {
    return {
      kind: 'phone-required',
      ...preparationScreenCopy.phoneRequired,
      actionHint: '안내된 매장 연락처로 전화해 주세요.',
      phoneLabel: preparationScreenCopy.phoneLabel,
      phoneNumber: state.branchPhoneNumber,
    };
  }

  if (state.kind === 'waiting') {
    return {
      kind: 'waiting',
      ...preparationScreenCopy.waiting,
      actionHint: preparationScreenCopy.actionHint,
    };
  }

  return {
    kind: 'preparing',
    ...preparationScreenCopy.preparing,
    actionHint: preparationScreenCopy.actionHint,
  };
}
