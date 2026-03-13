import { describe, expect, it } from 'vitest'

describe('selectCaptureConfidenceView', () => {
  it('projects the authoritative end time, active preset, and first-photo waiting state', async () => {
    const module = await import('./captureConfidenceView.js').catch(() => null)

    expect(module).not.toBeNull()

    expect(
      module?.selectCaptureConfidenceView({
        sessionId: 'session-24',
        revision: 1,
        updatedAt: '2026-03-08T10:00:00.000Z',
        shootEndsAt: '2026-03-08T10:50:00.000Z',
        activePreset: {
          presetId: 'preset-noir',
          label: 'Soft Noir',
        },
        latestPhoto: {
          kind: 'empty',
        },
      }),
    ).toEqual({
      endTime: {
        label: '촬영 종료 시간',
        value: '오후 7:50',
        supporting: '이 시간까지 촬영할 수 있어요.',
      },
      preset: {
        label: '현재 프리셋',
        value: 'Soft Noir',
      },
      latestPhoto: {
        kind: 'empty',
        title: '첫 사진을 기다리고 있어요.',
        supporting: '촬영이 저장되면 바로 여기에 보여드릴게요.',
        assetUrl: null,
        alt: null,
      },
      guidance: '리모컨으로 촬영을 계속해 주세요.',
      timingAlert: {
        kind: 'none',
      },
    })
  })

  it('keeps the previous preview visible while a new session-scoped photo is updating', async () => {
    const module = await import('./captureConfidenceView.js').catch(() => null)

    expect(module).not.toBeNull()

    expect(
      module?.selectCaptureConfidenceView({
        sessionId: 'session-24',
        revision: 4,
        updatedAt: '2026-03-08T10:06:00.000Z',
        shootEndsAt: '2026-03-08T10:50:00.000Z',
        activePreset: {
          presetId: 'preset-noir',
          label: 'Soft Noir',
        },
        latestPhoto: {
          kind: 'updating',
          nextCaptureId: 'capture-3',
          preview: {
            sessionId: 'session-24',
            captureId: 'capture-2',
            sequence: 2,
            assetUrl: 'asset://session-24/capture-2',
            capturedAt: '2026-03-08T10:04:00.000Z',
          },
        },
      }),
    ).toEqual({
      endTime: {
        label: '촬영 종료 시간',
        value: '오후 7:50',
        supporting: '이 시간까지 촬영할 수 있어요.',
      },
      preset: {
        label: '현재 프리셋',
        value: 'Soft Noir',
      },
      latestPhoto: {
        kind: 'updating',
        title: '방금 찍은 사진을 불러오고 있어요.',
        supporting: '촬영 흐름은 그대로 유지됩니다.',
        assetUrl: 'asset://session-24/capture-2',
        alt: '현재 세션의 최신 촬영 사진 미리보기',
      },
      guidance: '리모컨으로 촬영을 계속해 주세요.',
      timingAlert: {
        kind: 'none',
      },
    })
  })

  it('projects a ready latest-photo snapshot without changing the authoritative end-time contract', async () => {
    const module = await import('./captureConfidenceView.js').catch(() => null)

    expect(module).not.toBeNull()

    expect(
      module?.selectCaptureConfidenceView({
        sessionId: 'session-24',
        revision: 5,
        updatedAt: '2026-03-08T10:07:00.000Z',
        shootEndsAt: '2026-03-08T10:50:00.000Z',
        activePreset: {
          presetId: 'preset-noir',
          label: 'Soft Noir',
        },
        latestPhoto: {
          kind: 'ready',
          photo: {
            sessionId: 'session-24',
            captureId: 'capture-3',
            sequence: 3,
            assetUrl: 'asset://session-24/capture-3',
            capturedAt: '2026-03-08T10:07:00.000Z',
          },
        },
      }),
    ).toEqual({
      endTime: {
        label: '촬영 종료 시간',
        value: '오후 7:50',
        supporting: '이 시간까지 촬영할 수 있어요.',
      },
      preset: {
        label: '현재 프리셋',
        value: 'Soft Noir',
      },
      latestPhoto: {
        kind: 'ready',
        title: '방금 저장된 사진이에요.',
        supporting: '현재 세션에서 가장 최근에 저장된 사진입니다.',
        assetUrl: 'asset://session-24/capture-3',
        alt: '현재 세션의 최신 촬영 사진 미리보기',
      },
      guidance: '리모컨으로 촬영을 계속해 주세요.',
      timingAlert: {
        kind: 'none',
      },
    })
  })

  it('keeps Story 4.1 capture copy stable even if later-story warning data is present', async () => {
    const module = await import('./captureConfidenceView.js').catch(() => null)

    expect(module).not.toBeNull()

    expect(
      module?.selectCaptureConfidenceView(
        {
          sessionId: 'session-24',
          revision: 6,
          updatedAt: '2026-03-08T10:45:00.000Z',
          shootEndsAt: '2026-03-08T10:50:00.000Z',
          activePreset: {
            presetId: 'preset-noir',
            label: 'Soft Noir',
          },
          latestPhoto: {
            kind: 'empty',
          },
        },
        undefined,
        {
          kind: 'warning',
          effectiveTimingRevision: 'session-24:2026-03-08T10:45:00.000Z',
          actualShootEndAt: '2026-03-08T10:50:00.000Z',
          warningAt: '2026-03-08T10:45:00.000Z',
        },
      ),
    ).toEqual({
      endTime: {
        label: '촬영 종료 시간',
        value: '오후 7:50',
        supporting: '이 시간까지 촬영할 수 있어요.',
      },
      preset: {
        label: '현재 프리셋',
        value: 'Soft Noir',
      },
      latestPhoto: {
        kind: 'empty',
        title: '첫 사진을 기다리고 있어요.',
        supporting: '촬영이 저장되면 바로 여기에 보여드릴게요.',
        assetUrl: null,
        alt: null,
      },
      guidance: '리모컨으로 촬영을 계속해 주세요.',
      timingAlert: {
        kind: 'none',
      },
    })
  })

  it('keeps Story 4.1 capture copy stable even if later-story ended data is present', async () => {
    const module = await import('./captureConfidenceView.js').catch(() => null)

    expect(module).not.toBeNull()

    expect(
      module?.selectCaptureConfidenceView(
        {
          sessionId: 'session-24',
          revision: 7,
          updatedAt: '2026-03-08T10:50:00.000Z',
          shootEndsAt: '2026-03-08T10:50:00.000Z',
          activePreset: {
            presetId: 'preset-noir',
            label: 'Soft Noir',
          },
          latestPhoto: {
            kind: 'ready',
            photo: {
              sessionId: 'session-24',
              captureId: 'capture-3',
              sequence: 3,
              assetUrl: 'asset://session-24/capture-3',
              capturedAt: '2026-03-08T10:49:50.000Z',
            },
          },
        },
        undefined,
        {
          kind: 'ended',
          effectiveTimingRevision: 'session-24:2026-03-08T10:50:00.000Z',
          actualShootEndAt: '2026-03-08T10:50:00.000Z',
          warningAt: '2026-03-08T10:45:00.000Z',
        },
      ),
    ).toEqual({
      endTime: {
        label: '촬영 종료 시간',
        value: '오후 7:50',
        supporting: '이 시간까지 촬영할 수 있어요.',
      },
      preset: {
        label: '현재 프리셋',
        value: 'Soft Noir',
      },
      latestPhoto: {
        kind: 'ready',
        title: '방금 저장된 사진이에요.',
        supporting: '현재 세션에서 가장 최근에 저장된 사진입니다.',
        assetUrl: 'asset://session-24/capture-3',
        alt: '현재 세션의 최신 촬영 사진 미리보기',
      },
      guidance: '리모컨으로 촬영을 계속해 주세요.',
      timingAlert: {
        kind: 'none',
      },
    })
  })
})
