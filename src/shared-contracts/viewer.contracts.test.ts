import { describe, expect, it } from 'vitest'

import {
  captureReasonCodeSchema,
  viewerLayoutReportSchema,
  viewerReadinessSnapshotSchema,
  viewerReadinessUpdateSchema,
  viewerSessionBindingUpdateSchema,
  viewerSessionBindingUpdateEvent,
} from './index'

const SESSION_ID = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'

/** Rust `ViewerReadinessSnapshotDto`가 실제로 직렬화하는 형태 */
const HOST_SNAPSHOT_PAYLOAD = {
  schemaVersion: 'viewer-readiness/v1',
  sessionId: SESSION_ID,
  viewerEpoch: 1,
  revision: 4,
  windowState: 'open',
  listenerReady: true,
  layoutReady: true,
  monitorTargeting: 'approved-customer-monitor',
  displayProfile: {
    profileId: '1080p',
    monitorName: 'BOOTH-CUSTOMER',
    monitorWidthPx: 1920,
    monitorHeightPx: 1080,
    monitorScaleFactor: 1,
  },
  photoRect: {
    cssWidth: 1620,
    cssHeight: 1080,
    devicePixelRatio: 1,
    requiredSourceWidthPx: 1620,
    requiredSourceHeightPx: 1080,
  },
  viewerReady: true,
  reasonCode: 'viewer-ready',
  observedAtMs: 1_775_168_754_531,
  lastReportAtMs: 1_775_168_754_531,
} as const

describe('viewer readiness 계약', () => {
  it('host snapshot payload를 그대로 파싱한다', () => {
    expect(viewerReadinessSnapshotSchema.parse(HOST_SNAPSHOT_PAYLOAD)).toEqual(
      HOST_SNAPSHOT_PAYLOAD,
    )
  })

  it('세션이 아직 없는 부팅 직후 상태를 허용한다', () => {
    const parsed = viewerReadinessSnapshotSchema.parse({
      ...HOST_SNAPSHOT_PAYLOAD,
      sessionId: null,
      displayProfile: null,
      photoRect: null,
      windowState: 'absent',
      listenerReady: false,
      layoutReady: false,
      viewerReady: false,
      reasonCode: 'viewer-absent',
      lastReportAtMs: null,
    })

    expect(parsed.sessionId).toBeNull()
    expect(parsed.viewerReady).toBe(false)
  })

  it('유효하지 않은 세션 식별자를 거부한다', () => {
    expect(() =>
      viewerReadinessSnapshotSchema.parse({
        ...HOST_SNAPSHOT_PAYLOAD,
        sessionId: 'session_not_valid',
      }),
    ).toThrow()
  })

  it('승인 목록 밖의 display profile을 거부한다', () => {
    expect(() =>
      viewerReadinessSnapshotSchema.parse({
        ...HOST_SNAPSHOT_PAYLOAD,
        displayProfile: {
          ...HOST_SNAPSHOT_PAYLOAD.displayProfile,
          profileId: '720p',
        },
      }),
    ).toThrow()
  })

  it('0 이하의 photo rect를 거부한다', () => {
    expect(() =>
      viewerReadinessSnapshotSchema.parse({
        ...HOST_SNAPSHOT_PAYLOAD,
        photoRect: { ...HOST_SNAPSHOT_PAYLOAD.photoRect, cssWidth: 0 },
      }),
    ).toThrow()
  })

  it('알 수 없는 reason code를 거부한다', () => {
    expect(() =>
      viewerReadinessSnapshotSchema.parse({
        ...HOST_SNAPSHOT_PAYLOAD,
        reasonCode: 'looks-fine',
      }),
    ).toThrow()
  })

  it('필수 ready 상태가 빠진 viewerReady snapshot을 거부한다', () => {
    expect(() =>
      viewerReadinessSnapshotSchema.parse({
        ...HOST_SNAPSHOT_PAYLOAD,
        photoRect: null,
      }),
    ).toThrow()
    expect(() =>
      viewerReadinessSnapshotSchema.parse({
        ...HOST_SNAPSHOT_PAYLOAD,
        monitorTargeting: 'single-monitor-fallback',
      }),
    ).toThrow()
  })

  it('event update envelope을 파싱한다', () => {
    const parsed = viewerReadinessUpdateSchema.parse({
      schemaVersion: 'viewer-readiness-update/v1',
      readiness: HOST_SNAPSHOT_PAYLOAD,
    })

    expect(parsed.readiness.revision).toBe(4)
  })

  it('viewer -> host layout report를 파싱한다', () => {
    const parsed = viewerLayoutReportSchema.parse({
      viewerEpoch: 1,
      sessionId: SESSION_ID,
      cssWidth: 1620,
      cssHeight: 1080,
      devicePixelRatio: 1.25,
      layoutReady: true,
    })

    expect(parsed.devicePixelRatio).toBe(1.25)
  })

  it('DPR이 0 이하인 layout report를 거부한다', () => {
    expect(() =>
      viewerLayoutReportSchema.parse({
        viewerEpoch: 1,
        sessionId: SESSION_ID,
        cssWidth: 1620,
        cssHeight: 1080,
        devicePixelRatio: 0,
        layoutReady: true,
      }),
    ).toThrow()
  })

  it('0px layout은 미준비 보고에서만 허용한다', () => {
    expect(
      viewerLayoutReportSchema.parse({
        viewerEpoch: 1,
        sessionId: SESSION_ID,
        cssWidth: 0,
        cssHeight: 0,
        devicePixelRatio: 1,
        layoutReady: false,
      }).layoutReady,
    ).toBe(false)

    expect(() =>
      viewerLayoutReportSchema.parse({
        viewerEpoch: 1,
        sessionId: SESSION_ID,
        cssWidth: 0,
        cssHeight: 0,
        devicePixelRatio: 1,
        layoutReady: true,
      }),
    ).toThrow()
  })

  it('host 정수 범위를 넘는 source dimension을 거부한다', () => {
    expect(() =>
      viewerLayoutReportSchema.parse({
        viewerEpoch: 1,
        sessionId: SESSION_ID,
        cssWidth: 0xffff_ffff,
        cssHeight: 1080,
        devicePixelRatio: 2,
        layoutReady: true,
      }),
    ).toThrow()
  })

  it('session binding event 이름을 공유 계약으로 고정한다', () => {
    expect(viewerSessionBindingUpdateEvent).toBe(
      'viewer-session-binding-update',
    )
  })

  it('session binding event payload를 durable snapshot envelope로 고정한다', () => {
    const parsed = viewerSessionBindingUpdateSchema.parse({
      schemaVersion: 'viewer-readiness-update/v1',
      readiness: HOST_SNAPSHOT_PAYLOAD,
    })

    expect(parsed.readiness.sessionId).toBe(SESSION_ID)
    expect(parsed.readiness.viewerReady).toBe(true)
  })

  it('capture readiness가 viewer-preparing 차단 사유를 인식한다', () => {
    expect(captureReasonCodeSchema.parse('viewer-preparing')).toBe(
      'viewer-preparing',
    )
  })

  it('host source dimensions match the measured rect and DPR', () => {
    expect(() =>
      viewerReadinessSnapshotSchema.parse({
        ...HOST_SNAPSHOT_PAYLOAD,
        photoRect: {
          ...HOST_SNAPSHOT_PAYLOAD.photoRect,
          requiredSourceWidthPx: 1,
        },
      }),
    ).toThrow()

    expect(
      viewerReadinessSnapshotSchema.parse({
        ...HOST_SNAPSHOT_PAYLOAD,
        photoRect: {
          cssWidth: 1620.4,
          cssHeight: 1080.2,
          devicePixelRatio: 1.5,
          requiredSourceWidthPx: 2431,
          requiredSourceHeightPx: 1621,
        },
      }).photoRect,
    ).toMatchObject({
      requiredSourceWidthPx: 2431,
      requiredSourceHeightPx: 1621,
    })
  })
})
