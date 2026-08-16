import { describe, expect, it } from 'vitest'

import {
  APPROVED_DISPLAY_PROFILES,
  FIXED_THUMBNAIL_MAX_EDGE_PX,
  classifyDisplayProfile,
  computePhotoRect,
  computeRequiredSourceDimensions,
  isDisplayFitSource,
} from './display-fit'

describe('computePhotoRect', () => {
  it('3:2 사진을 wide 컨테이너에 contain으로 맞춘다 (높이 기준)', () => {
    expect(computePhotoRect({ containerWidth: 1920, containerHeight: 1080 })).toEqual({
      cssWidth: 1620,
      cssHeight: 1080,
    })
  })

  it('3:2보다 좁은 컨테이너에서는 너비 기준으로 맞춘다', () => {
    expect(computePhotoRect({ containerWidth: 1200, containerHeight: 1080 })).toEqual({
      cssWidth: 1200,
      cssHeight: 800,
    })
  })

  it('컨테이너가 아직 측정되지 않았으면 null을 반환한다', () => {
    expect(computePhotoRect({ containerWidth: 0, containerHeight: 1080 })).toBeNull()
    expect(computePhotoRect({ containerWidth: 1920, containerHeight: 0 })).toBeNull()
  })
})

describe('computeRequiredSourceDimensions', () => {
  const profiles = [
    {
      label: '1080p',
      monitorWidthPx: 1920,
      monitorHeightPx: 1080,
      expectedRequired: { requiredSourceWidthPx: 1620, requiredSourceHeightPx: 1080 },
    },
    {
      label: '1440p',
      monitorWidthPx: 2560,
      monitorHeightPx: 1440,
      expectedRequired: { requiredSourceWidthPx: 2160, requiredSourceHeightPx: 1440 },
    },
    {
      label: '4K',
      monitorWidthPx: 3840,
      monitorHeightPx: 2160,
      expectedRequired: { requiredSourceWidthPx: 3240, requiredSourceHeightPx: 2160 },
    },
  ] as const
  const devicePixelRatios = [1, 1.25, 1.5, 2] as const

  for (const profile of profiles) {
    for (const devicePixelRatio of devicePixelRatios) {
      it(`${profile.label} × DPR ${devicePixelRatio}에서 physical source 크기를 고정한다`, () => {
        const rect = computePhotoRect({
          containerWidth: profile.monitorWidthPx / devicePixelRatio,
          containerHeight: profile.monitorHeightPx / devicePixelRatio,
        })

        expect(
          computeRequiredSourceDimensions({
            ...rect!,
            devicePixelRatio,
          }),
        ).toEqual(profile.expectedRequired)
      })
    }
  }

  it('소수 픽셀은 올림한다 (내림하면 upscale이 허용된다)', () => {
    expect(
      computeRequiredSourceDimensions({
        cssWidth: 1620.4,
        cssHeight: 1080.2,
        devicePixelRatio: 1.5,
      }),
    ).toEqual({ requiredSourceWidthPx: 2431, requiredSourceHeightPx: 1621 })
  })

  it('DPR 1.5에서도 고정 크기가 아니라 rect에서 파생한다', () => {
    expect(
      computeRequiredSourceDimensions({
        cssWidth: 1620,
        cssHeight: 1080,
        devicePixelRatio: 1.5,
      }),
    ).toEqual({ requiredSourceWidthPx: 2430, requiredSourceHeightPx: 1620 })
  })

  it.each([
    { cssWidth: 0, cssHeight: 1080, devicePixelRatio: 1 },
    { cssWidth: -1, cssHeight: 1080, devicePixelRatio: 1 },
    { cssWidth: 1620, cssHeight: Number.POSITIVE_INFINITY, devicePixelRatio: 1 },
    { cssWidth: 1620, cssHeight: 1080, devicePixelRatio: Number.NaN },
  ])('유효하지 않은 rect 또는 DPR은 0px 계약으로 내린다', (input) => {
    expect(computeRequiredSourceDimensions(input)).toEqual({
      requiredSourceWidthPx: 0,
      requiredSourceHeightPx: 0,
    })
  })
})

describe('isDisplayFitSource', () => {
  const required = { requiredSourceWidthPx: 1620, requiredSourceHeightPx: 1080 }

  it('필요 픽셀 이상이면 적합하다', () => {
    expect(isDisplayFitSource({ naturalWidth: 1920, naturalHeight: 1280 }, required)).toBe(true)
  })

  it('정확히 필요 픽셀이면 적합하다', () => {
    expect(isDisplayFitSource({ naturalWidth: 1620, naturalHeight: 1080 }, required)).toBe(true)
  })

  it('세로 원본을 contain으로 표시할 때 한 축이 목표에 닿으면 확대 없이 적합하다', () => {
    expect(isDisplayFitSource({ naturalWidth: 720, naturalHeight: 1080 }, required)).toBe(true)
    expect(isDisplayFitSource({ naturalWidth: 1620, naturalHeight: 720 }, required)).toBe(true)
  })

  it('두 축이 모두 모자라 확대가 필요하면 부적합하다', () => {
    expect(isDisplayFitSource({ naturalWidth: 1619, naturalHeight: 1079 }, required)).toBe(false)
  })

  it('고정 384px thumbnail은 승인된 어떤 profile에서도 적합하지 않다', () => {
    const thumbnail = {
      naturalWidth: FIXED_THUMBNAIL_MAX_EDGE_PX,
      naturalHeight: FIXED_THUMBNAIL_MAX_EDGE_PX,
    }

    for (const profile of Object.values(APPROVED_DISPLAY_PROFILES)) {
      const rect = computePhotoRect({
        containerWidth: profile.monitorWidthPx / profile.referenceDevicePixelRatio,
        containerHeight: profile.monitorHeightPx / profile.referenceDevicePixelRatio,
      })
      const requiredForProfile = computeRequiredSourceDimensions({
        ...rect!,
        devicePixelRatio: profile.referenceDevicePixelRatio,
      })

      expect(isDisplayFitSource(thumbnail, requiredForProfile)).toBe(false)
    }
  })

  it('upscale이 필요한 review-rail asset은 부적합하다', () => {
    expect(isDisplayFitSource({ naturalWidth: 384, naturalHeight: 256 }, required)).toBe(false)
    expect(isDisplayFitSource({ naturalWidth: 800, naturalHeight: 533 }, required)).toBe(false)
  })

  it('디코드되지 않은 asset(0px)은 부적합하다', () => {
    expect(isDisplayFitSource({ naturalWidth: 0, naturalHeight: 0 }, required)).toBe(false)
  })

  it('유효하지 않은 required dimension은 어떤 asset도 통과시키지 않는다', () => {
    expect(
      isDisplayFitSource(
        { naturalWidth: 1920, naturalHeight: 1280 },
        { requiredSourceWidthPx: -1, requiredSourceHeightPx: 0 },
      ),
    ).toBe(false)
  })
})

describe('classifyDisplayProfile', () => {
  it('승인된 해상도를 profile로 분류한다', () => {
    expect(classifyDisplayProfile({ monitorWidthPx: 1920, monitorHeightPx: 1080 })).toBe('1080p')
    expect(classifyDisplayProfile({ monitorWidthPx: 2560, monitorHeightPx: 1440 })).toBe('1440p')
    expect(classifyDisplayProfile({ monitorWidthPx: 3840, monitorHeightPx: 2160 })).toBe('4k')
  })

  it('승인 목록 밖의 해상도는 null이다', () => {
    expect(classifyDisplayProfile({ monitorWidthPx: 1280, monitorHeightPx: 720 })).toBeNull()
    expect(classifyDisplayProfile({ monitorWidthPx: 3440, monitorHeightPx: 1440 })).toBeNull()
  })
})
