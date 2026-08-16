import type { ViewerDisplayProfile } from '../../shared-contracts'

/**
 * 사진 표시 비율. 승인된 booth 카메라(EOS 700D) 출력과 동일한 3:2를 고정한다.
 */
export const PHOTO_ASPECT_RATIO = 3 / 2

/**
 * `src-tauri/src/render/mod.rs`의 preview cap과 같은 값. review-rail thumbnail이
 * display-fit source로 오인되지 않도록 계약 테스트에서 참조한다.
 * 이 값을 display 경로의 목표 크기로 사용하지 않는다.
 */
export const FIXED_THUMBNAIL_MAX_EDGE_PX = 384

type ApprovedDisplayProfile = {
  profileId: ViewerDisplayProfile['profileId']
  monitorWidthPx: number
  monitorHeightPx: number
  /** HV-13A 기준 환경의 대표 DPR. 실제 판정은 항상 viewer가 보고한 DPR을 쓴다. */
  referenceDevicePixelRatio: number
}

export const APPROVED_DISPLAY_PROFILES: Record<
  ViewerDisplayProfile['profileId'],
  ApprovedDisplayProfile
> = {
  '1080p': {
    profileId: '1080p',
    monitorWidthPx: 1920,
    monitorHeightPx: 1080,
    referenceDevicePixelRatio: 1,
  },
  '1440p': {
    profileId: '1440p',
    monitorWidthPx: 2560,
    monitorHeightPx: 1440,
    referenceDevicePixelRatio: 1.25,
  },
  '4k': {
    profileId: '4k',
    monitorWidthPx: 3840,
    monitorHeightPx: 2160,
    referenceDevicePixelRatio: 2,
  },
}

export type PhotoRectCss = {
  cssWidth: number
  cssHeight: number
}

export type RequiredSourceDimensions = {
  requiredSourceWidthPx: number
  requiredSourceHeightPx: number
}

/**
 * 컨테이너 안에 3:2 사진을 contain으로 배치했을 때의 실제 CSS rectangle.
 * 컨테이너가 아직 측정되지 않았으면 null을 반환해 layout-ready를 주장하지 않는다.
 */
export function computePhotoRect({
  containerWidth,
  containerHeight,
  aspectRatio = PHOTO_ASPECT_RATIO,
}: {
  containerWidth: number
  containerHeight: number
  aspectRatio?: number
}): PhotoRectCss | null {
  if (
    !Number.isFinite(containerWidth) ||
    !Number.isFinite(containerHeight) ||
    containerWidth <= 0 ||
    containerHeight <= 0
  ) {
    return null
  }

  const containerAspect = containerWidth / containerHeight

  if (containerAspect > aspectRatio) {
    return {
      cssWidth: containerHeight * aspectRatio,
      cssHeight: containerHeight,
    }
  }

  return {
    cssWidth: containerWidth,
    cssHeight: containerWidth / aspectRatio,
  }
}

/**
 * physical display-fit 계약의 핵심. 고정 thumbnail 크기가 아니라 실제 CSS rect와 DPR에서
 * 필요한 소스 픽셀 수를 파생한다. 올림을 쓰는 이유는 내림이 1px upscale을 허용하기 때문이다.
 */
export function computeRequiredSourceDimensions({
  cssWidth,
  cssHeight,
  devicePixelRatio,
}: {
  cssWidth: number
  cssHeight: number
  devicePixelRatio: number
}): RequiredSourceDimensions {
  if (
    !Number.isFinite(cssWidth) ||
    !Number.isFinite(cssHeight) ||
    !Number.isFinite(devicePixelRatio) ||
    cssWidth <= 0 ||
    cssHeight <= 0 ||
    devicePixelRatio <= 0
  ) {
    return {
      requiredSourceWidthPx: 0,
      requiredSourceHeightPx: 0,
    }
  }

  return {
    requiredSourceWidthPx: Math.ceil(cssWidth * devicePixelRatio),
    requiredSourceHeightPx: Math.ceil(cssHeight * devicePixelRatio),
  }
}

/**
 * 해당 asset을 photo rectangle에 `contain`으로 놓을 때 upscale이 필요한지 판정한다.
 * 한 축이 경계에 닿으면 다른 축은 letterbox 여백으로 남고 픽셀 확대는 일어나지 않는다.
 * Story 7.1은 이 판정을 계약으로만 소유하고, 실제 asset 게시는 Story 7.2 이후가 소유한다.
 */
export function isDisplayFitSource(
  source: { naturalWidth: number; naturalHeight: number },
  required: RequiredSourceDimensions,
): boolean {
  if (
    !Number.isFinite(source.naturalWidth) ||
    !Number.isFinite(source.naturalHeight) ||
    source.naturalWidth <= 0 ||
    source.naturalHeight <= 0
  ) {
    return false
  }

  if (
    !Number.isFinite(required.requiredSourceWidthPx) ||
    !Number.isFinite(required.requiredSourceHeightPx) ||
    required.requiredSourceWidthPx <= 0 ||
    required.requiredSourceHeightPx <= 0
  ) {
    return false
  }

  return (
    source.naturalWidth >= required.requiredSourceWidthPx ||
    source.naturalHeight >= required.requiredSourceHeightPx
  )
}

/**
 * 모니터 해상도를 승인된 display profile로 분류한다. 승인 목록 밖이면 null이며
 * host는 이를 `unapproved-profile` monitor targeting으로 보고한다.
 */
export function classifyDisplayProfile({
  monitorWidthPx,
  monitorHeightPx,
}: {
  monitorWidthPx: number
  monitorHeightPx: number
}): ViewerDisplayProfile['profileId'] | null {
  const match = Object.values(APPROVED_DISPLAY_PROFILES).find(
    (profile) =>
      profile.monitorWidthPx === monitorWidthPx &&
      profile.monitorHeightPx === monitorHeightPx,
  )

  return match?.profileId ?? null
}
