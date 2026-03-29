import { describe, expect, it } from 'vitest'

import type { SessionCaptureRecord, SessionManifest } from '../../shared-contracts'
import { selectCurrentSessionPreviews } from './current-session-previews'

function createCapture(
  overrides: Partial<SessionCaptureRecord> = {},
): SessionCaptureRecord {
  return {
    schemaVersion: 'session-capture/v1',
    sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
    boothAlias: 'Kim 4821',
    activePresetId: 'preset_soft-glow',
    activePresetVersion: '2026.03.20',
    captureId: 'capture_01hs6n1r8b8zc5v4ey2x7b9g1m',
    requestId: 'request_01hs6n1r8b8zc5v4ey2x7b9g1m',
    raw: {
      assetPath: 'fixtures/capture-raw.jpg',
      persistedAtMs: 100,
    },
    preview: {
      assetPath: 'fixtures/capture-preview.jpg',
      enqueuedAtMs: 110,
      readyAtMs: 200,
    },
    final: {
      assetPath: null,
      readyAtMs: null,
    },
    renderStatus: 'previewReady',
    postEndState: 'activeSession',
    timing: {
      captureAcknowledgedAtMs: 100,
      previewVisibleAtMs: 200,
      captureBudgetMs: 1000,
      previewBudgetMs: 5000,
      previewBudgetState: 'withinBudget',
    },
    ...overrides,
  }
}

function createManifest(
  captures: SessionCaptureRecord[],
  sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m',
): SessionManifest {
  return {
    schemaVersion: 'session-manifest/v1',
    sessionId,
    boothAlias: 'Kim 4821',
    customer: {
      name: 'Kim',
      phoneLastFour: '4821',
    },
    createdAt: '2026-03-20T00:00:00.000Z',
    updatedAt: '2026-03-20T00:00:00.000Z',
    lifecycle: {
      status: 'active',
      stage: 'capture-ready',
    },
    activePreset: {
      presetId: 'preset_soft-glow',
      publishedVersion: '2026.03.20',
    },
    activePresetId: 'preset_soft-glow',
    captures,
    postEnd: null,
  }
}

describe('selectCurrentSessionPreviews', () => {
  it('returns only preview-ready captures from the active session and marks the latest item', () => {
    const previews = selectCurrentSessionPreviews(
      createManifest([
        createCapture({
          captureId: 'capture_latest',
          preview: {
            assetPath: 'fixtures/latest.jpg',
            enqueuedAtMs: 150,
            readyAtMs: 500,
          },
        }),
        createCapture({
          captureId: 'capture_older',
          preview: {
            assetPath: 'fixtures/older.jpg',
            enqueuedAtMs: 120,
            readyAtMs: 300,
          },
        }),
        createCapture({
          captureId: 'capture_other_session',
          sessionId: 'session_01hs6n1r8b8zc5v4ey2x7b9g1n',
          boothAlias: 'Lee 1234',
          preview: {
            assetPath: 'fixtures/other.jpg',
            enqueuedAtMs: 160,
            readyAtMs: 700,
          },
        }),
        createCapture({
          captureId: 'capture_waiting',
          renderStatus: 'previewWaiting',
          preview: {
            assetPath: 'fixtures/waiting.jpg',
            enqueuedAtMs: 170,
            readyAtMs: null,
          },
        }),
      ]),
    )

    expect(previews).toEqual([
      {
        captureId: 'capture_latest',
        assetPath: 'fixtures/latest.jpg',
        activePresetId: 'preset_soft-glow',
        activePresetVersion: '2026.03.20',
        presetDisplayName: null,
        isCurrentActivePreset: true,
        postEndState: 'activeSession',
        readyAtMs: 500,
        isLatest: true,
      },
      {
        captureId: 'capture_older',
        assetPath: 'fixtures/older.jpg',
        activePresetId: 'preset_soft-glow',
        activePresetVersion: '2026.03.20',
        presetDisplayName: null,
        isCurrentActivePreset: true,
        postEndState: 'activeSession',
        readyAtMs: 300,
        isLatest: false,
      },
    ])
  })

  it('keeps same-session final-ready captures in the rail when their preview is still available', () => {
    const previews = selectCurrentSessionPreviews(
      createManifest([
        createCapture({
          captureId: 'capture_final_ready',
          renderStatus: 'finalReady',
          preview: {
            assetPath: 'fixtures/final-ready-preview.jpg',
            enqueuedAtMs: 150,
            readyAtMs: 520,
          },
          final: {
            assetPath: 'fixtures/final-ready-final.jpg',
            readyAtMs: 560,
          },
        }),
        createCapture({
          captureId: 'capture_preview_ready',
          renderStatus: 'previewReady',
          preview: {
            assetPath: 'fixtures/preview-ready.jpg',
            enqueuedAtMs: 120,
            readyAtMs: 430,
          },
        }),
      ]),
    )

    expect(previews).toEqual([
      {
        captureId: 'capture_final_ready',
        assetPath: 'fixtures/final-ready-preview.jpg',
        activePresetId: 'preset_soft-glow',
        activePresetVersion: '2026.03.20',
        presetDisplayName: null,
        isCurrentActivePreset: true,
        postEndState: 'activeSession',
        readyAtMs: 520,
        isLatest: true,
      },
      {
        captureId: 'capture_preview_ready',
        assetPath: 'fixtures/preview-ready.jpg',
        activePresetId: 'preset_soft-glow',
        activePresetVersion: '2026.03.20',
        presetDisplayName: null,
        isCurrentActivePreset: true,
        postEndState: 'activeSession',
        readyAtMs: 430,
        isLatest: false,
      },
    ])
  })

  it('returns an empty list when there is no active manifest', () => {
    expect(selectCurrentSessionPreviews(null)).toEqual([])
  })

  it('breaks same-millisecond ties with enqueue time and persisted time so latest stays deterministic', () => {
    const previews = selectCurrentSessionPreviews(
      createManifest([
        createCapture({
          captureId: 'capture_first',
          raw: {
            assetPath: 'fixtures/first-raw.jpg',
            persistedAtMs: 210,
          },
          preview: {
            assetPath: 'fixtures/first.jpg',
            enqueuedAtMs: 250,
            readyAtMs: 500,
          },
        }),
        createCapture({
          captureId: 'capture_second',
          raw: {
            assetPath: 'fixtures/second-raw.jpg',
            persistedAtMs: 220,
          },
          preview: {
            assetPath: 'fixtures/second.jpg',
            enqueuedAtMs: 260,
            readyAtMs: 500,
          },
        }),
      ]),
    )

    expect(previews[0]).toMatchObject({
      captureId: 'capture_second',
      isLatest: true,
    })
    expect(previews[1]).toMatchObject({
      captureId: 'capture_first',
      isLatest: false,
    })
  })

  it('ignores preview-ready captures whose asset path points at another session directory', () => {
    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'

    const previews = selectCurrentSessionPreviews(
      createManifest(
        [
          createCapture({
            captureId: 'capture_visible',
            preview: {
              assetPath: 'fixtures/current-session.jpg',
              enqueuedAtMs: 130,
              readyAtMs: 220,
            },
          }),
          createCapture({
            captureId: 'capture_foreign_asset',
            preview: {
              assetPath:
                'C:\\Users\\Example\\Pictures\\dabi_shoot\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1n\\renders\\previews\\other-session.jpg',
              enqueuedAtMs: 140,
              readyAtMs: 320,
            },
          }),
        ],
        sessionId,
      ),
    )

    expect(previews).toEqual([
      {
        captureId: 'capture_visible',
        assetPath: 'fixtures/current-session.jpg',
        activePresetId: 'preset_soft-glow',
        activePresetVersion: '2026.03.20',
        presetDisplayName: null,
        isCurrentActivePreset: true,
        postEndState: 'activeSession',
        readyAtMs: 220,
        isLatest: true,
      },
    ])
  })

  it('rejects relative and UNC-style asset paths that point outside the active session scope', () => {
    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'

    const previews = selectCurrentSessionPreviews(
      createManifest(
        [
          createCapture({
            captureId: 'capture_visible',
            preview: {
              assetPath: 'fixtures/current-session.jpg',
              enqueuedAtMs: 130,
              readyAtMs: 220,
            },
          }),
          createCapture({
            captureId: 'capture_relative_foreign_asset',
            preview: {
              assetPath:
                'sessions/session_01hs6n1r8b8zc5v4ey2x7b9g1n/renders/previews/other-session.jpg',
              enqueuedAtMs: 150,
              readyAtMs: 330,
            },
          }),
          createCapture({
            captureId: 'capture_unc_foreign_asset',
            preview: {
              assetPath:
                '\\\\server\\share\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1n\\renders\\previews\\other-session.jpg',
              enqueuedAtMs: 160,
              readyAtMs: 340,
            },
          }),
          createCapture({
            captureId: 'capture_unknown_relative_asset',
            preview: {
              assetPath: 'renders/foreign-preview.jpg',
              enqueuedAtMs: 170,
              readyAtMs: 350,
            },
          }),
        ],
        sessionId,
      ),
    )

    expect(previews).toEqual([
      {
        captureId: 'capture_visible',
        assetPath: 'fixtures/current-session.jpg',
        activePresetId: 'preset_soft-glow',
        activePresetVersion: '2026.03.20',
        presetDisplayName: null,
        isCurrentActivePreset: true,
        postEndState: 'activeSession',
        readyAtMs: 220,
        isLatest: true,
      },
    ])
  })

  it('rejects runtime relative session paths even when they point at the active session id', () => {
    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'

    const previews = selectCurrentSessionPreviews(
      createManifest(
        [
          createCapture({
            captureId: 'capture_visible',
            preview: {
              assetPath: 'fixtures/current-session.jpg',
              enqueuedAtMs: 130,
              readyAtMs: 220,
            },
          }),
          createCapture({
            captureId: 'capture_relative_current_session_asset',
            preview: {
              assetPath:
                'sessions/session_01hs6n1r8b8zc5v4ey2x7b9g1m/renders/previews/current-session.jpg',
              enqueuedAtMs: 150,
              readyAtMs: 330,
            },
          }),
        ],
        sessionId,
      ),
    )

    expect(previews).toEqual([
      {
        captureId: 'capture_visible',
        assetPath: 'fixtures/current-session.jpg',
        activePresetId: 'preset_soft-glow',
        activePresetVersion: '2026.03.20',
        presetDisplayName: null,
        isCurrentActivePreset: true,
        postEndState: 'activeSession',
        readyAtMs: 220,
        isLatest: true,
      },
    ])
  })

  it('rejects absolute paths outside the booth runtime root even when they contain the active session id', () => {
    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'

    const previews = selectCurrentSessionPreviews(
      createManifest(
        [
          createCapture({
            captureId: 'capture_visible',
            preview: {
              assetPath:
                'C:\\Users\\Example\\Pictures\\dabi_shoot\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1m\\renders\\previews\\current-session.jpg',
              enqueuedAtMs: 130,
              readyAtMs: 220,
            },
          }),
          createCapture({
            captureId: 'capture_wrong_root',
            preview: {
              assetPath:
                'C:\\Users\\Kim\\AppData\\Local\\Boothy\\foreign-runtime\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1m\\renders\\previews\\foreign-root.jpg',
              enqueuedAtMs: 150,
              readyAtMs: 330,
            },
          }),
        ],
        sessionId,
      ),
    )

    expect(previews).toEqual([
      {
        captureId: 'capture_visible',
        assetPath:
          'C:\\Users\\Example\\Pictures\\dabi_shoot\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1m\\renders\\previews\\current-session.jpg',
        activePresetId: 'preset_soft-glow',
        activePresetVersion: '2026.03.20',
        presetDisplayName: null,
        isCurrentActivePreset: true,
        postEndState: 'activeSession',
        readyAtMs: 220,
        isLatest: true,
      },
    ])
  })

  it('keeps valid in-session filenames that contain double dots', () => {
    const sessionId = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'

    const previews = selectCurrentSessionPreviews(
      createManifest(
        [
          createCapture({
            captureId: 'capture_versioned_filename',
            preview: {
              assetPath:
                'C:\\Users\\Example\\Pictures\\dabi_shoot\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1m\\renders\\previews\\portrait..v2.jpg',
              enqueuedAtMs: 130,
              readyAtMs: 220,
            },
          }),
        ],
        sessionId,
      ),
    )

    expect(previews).toEqual([
      {
        captureId: 'capture_versioned_filename',
        assetPath:
          'C:\\Users\\Example\\Pictures\\dabi_shoot\\sessions\\session_01hs6n1r8b8zc5v4ey2x7b9g1m\\renders\\previews\\portrait..v2.jpg',
        activePresetId: 'preset_soft-glow',
        activePresetVersion: '2026.03.20',
        presetDisplayName: null,
        isCurrentActivePreset: true,
        postEndState: 'activeSession',
        readyAtMs: 220,
        isLatest: true,
      },
    ])
  })

  it('keeps past capture bindings distinct from the current active preset', () => {
    const previews = selectCurrentSessionPreviews(
      createManifest(
        [
          createCapture({
            captureId: 'capture_previous_look',
            activePresetId: 'preset_mono-pop',
            activePresetVersion: '2026.03.19',
            activePresetDisplayName: 'Mono Pop',
            preview: {
              assetPath: 'fixtures/previous-look.jpg',
              enqueuedAtMs: 110,
              readyAtMs: 410,
            },
          }),
        ],
      ),
    )

    expect(previews).toEqual([
      {
        captureId: 'capture_previous_look',
        assetPath: 'fixtures/previous-look.jpg',
        activePresetId: 'preset_mono-pop',
        activePresetVersion: '2026.03.19',
        presetDisplayName: 'Mono Pop',
        isCurrentActivePreset: false,
        postEndState: 'activeSession',
        readyAtMs: 410,
        isLatest: true,
      },
    ])
  })

  it('keeps legacy captures visible even when activePresetId is missing', () => {
    const previews = selectCurrentSessionPreviews(
      createManifest([
        createCapture({
          captureId: 'capture_legacy',
          activePresetId: undefined,
          activePresetDisplayName: null,
          preview: {
            assetPath: 'fixtures/legacy.jpg',
            enqueuedAtMs: 110,
            readyAtMs: 410,
          },
        }),
      ]),
    )

    expect(previews).toEqual([
      {
        captureId: 'capture_legacy',
        assetPath: 'fixtures/legacy.jpg',
        activePresetId: null,
        activePresetVersion: '2026.03.20',
        presetDisplayName: null,
        isCurrentActivePreset: false,
        postEndState: 'activeSession',
        readyAtMs: 410,
        isLatest: true,
      },
    ])
  })
})
