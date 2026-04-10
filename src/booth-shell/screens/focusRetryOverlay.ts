export function shouldShowFocusRetryOverlay(input: {
  previousReasonCode: string | null
  nextReasonCode: string
  wasRequestingCapture: boolean
  isRequestingCapture: boolean
  alreadyShownForCurrentRequest: boolean
}) {
  const isRetryRequired = input.nextReasonCode === 'capture-retry-required'
  const enteredRetryStateOutsideRequest =
    isRetryRequired &&
    !input.wasRequestingCapture &&
    !input.isRequestingCapture &&
    input.previousReasonCode !== 'capture-retry-required'
  const completedRequestIntoRetryState =
    isRetryRequired &&
    input.wasRequestingCapture &&
    !input.isRequestingCapture &&
    !input.alreadyShownForCurrentRequest

  return {
    shouldShow: enteredRetryStateOutsideRequest || completedRequestIntoRetryState,
    markShownForCurrentRequest: completedRequestIntoRetryState,
    resetShownForCurrentRequest:
      !input.wasRequestingCapture && input.isRequestingCapture,
  }
}
