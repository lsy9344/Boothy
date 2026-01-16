import { AppSettings } from '../components/ui/AppProperties';

export const DEFAULT_END_SCREEN_MESSAGE = '이용해주셔서 감사합니다';
export const DEFAULT_T_MINUS_5_WARNING_MESSAGE = '세션 종료가 5분 남았습니다';
export const DEFAULT_RESET_GRACE_PERIOD_SECONDS = 30;

const isNonEmptyString = (value: unknown): value is string => typeof value === 'string' && value.trim().length > 0;

export const getBoothyEndScreenMessage = (settings?: AppSettings | null) => {
  const value = settings?.boothy_end_screen_message;
  return isNonEmptyString(value) ? value : DEFAULT_END_SCREEN_MESSAGE;
};

export const getBoothyTMinus5WarningMessage = (settings?: AppSettings | null) => {
  const value = settings?.boothy_t_minus_5_warning_message;
  return isNonEmptyString(value) ? value : DEFAULT_T_MINUS_5_WARNING_MESSAGE;
};

export const getBoothyResetGracePeriodSeconds = (settings?: AppSettings | null) => {
  const value = settings?.boothy_reset_grace_period_seconds;
  if (typeof value === 'number' && Number.isFinite(value) && value > 0) {
    return value;
  }
  return DEFAULT_RESET_GRACE_PERIOD_SECONDS;
};
