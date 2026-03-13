import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'

import { CaptureScreen } from './CaptureScreen.js'
import { createCaptureFlowState } from '../../session-domain/state/captureFlowState.js'

describe('CaptureScreen', () => {
  it('renders the capture-confidence trust anchors from the view model', () => {
    render(
      <CaptureScreen
        sessionName="session-24-kim"
        view={{
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
        }}
      />,
    )

    expect(screen.getByText('촬영 종료 시간')).toBeInTheDocument()
    expect(screen.getByText('오후 7:50')).toBeInTheDocument()
    expect(screen.getByText('현재 프리셋')).toBeInTheDocument()
    expect(screen.getByText('Soft Noir')).toBeInTheDocument()
    expect(screen.getByText('첫 사진을 기다리고 있어요.')).toBeInTheDocument()
  })

  it('shows the latest photo preview when a session-scoped image is ready', () => {
    render(
      <CaptureScreen
        sessionName="session-24-kim"
        view={{
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
        }}
      />,
    )

    expect(screen.getByRole('img', { name: '현재 세션의 최신 촬영 사진 미리보기' })).toHaveAttribute(
      'src',
      'asset://session-24/capture-3',
    )
  })

  it('keeps the 2.4 confidence surface focused on the latest-photo panel', () => {
    render(
      <CaptureScreen
        sessionName="session-24-kim"
        view={{
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
            assetUrl: 'asset://session-24/capture-2',
            alt: '현재 세션의 최신 촬영 사진 미리보기',
          },
          guidance: '리모컨으로 촬영을 계속해 주세요.',
          timingAlert: {
            kind: 'none',
          },
        }}
      />,
    )

    expect(screen.getByRole('img', { name: '현재 세션의 최신 촬영 사진 미리보기' })).toHaveAttribute(
      'src',
      'asset://session-24/capture-2',
    )
    expect(screen.queryByRole('img', { name: '선택된 세션 사진 미리보기' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: '첫 번째 사진 선택' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: '두 번째 사진 선택' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: '사진 크게 보기' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: '사진 삭제' })).not.toBeInTheDocument()
    expect(screen.queryByLabelText('세션 사진 썸네일')).not.toBeInTheDocument()
  })

  it('stays on the latest-photo confidence panel when the view is ready', () => {
    render(
      <CaptureScreen
        sessionName="session-24-kim"
        view={{
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
            assetUrl: 'asset://session-24/capture-2',
            alt: '현재 세션의 최신 촬영 사진 미리보기',
          },
          guidance: '리모컨으로 촬영을 계속해 주세요.',
          timingAlert: {
            kind: 'none',
          },
        }}
      />,
    )

    expect(screen.getByText('방금 저장된 사진이에요.')).toBeInTheDocument()
    expect(screen.getByRole('img', { name: '현재 세션의 최신 촬영 사진 미리보기' })).toHaveAttribute(
      'src',
      'asset://session-24/capture-2',
    )
    expect(screen.queryByText('아직 표시할 사진이 없어요.')).not.toBeInTheDocument()
  })

  it('keeps the interactive 2.4 capture surface on the latest-photo panel', () => {
    render(
      <CaptureScreen
        captureActionDisabled
        captureState={createCaptureFlowState({
          activePresetId: 'background-pink',
          sessionEndTimeLabel: '오후 7:50',
        })}
        onCapture={() => undefined}
        onClosePresetSelector={() => undefined}
        onCloseReviewDialog={() => undefined}
        onConfirmDeletePhoto={async () => undefined}
        onDismissPresetChangeFeedback={() => undefined}
        onDismissReviewFeedback={() => undefined}
        onOpenDeleteDialog={() => undefined}
        onOpenPresetSelector={() => undefined}
        onOpenReview={() => undefined}
        onSelectPreset={() => undefined}
        onSelectReviewCapture={() => undefined}
        sessionName="session-24-kim"
        view={{
          endTime: {
            label: '촬영 종료 시간',
            value: '오후 7:50',
            supporting: '이 시간까지 촬영할 수 있어요.',
          },
          preset: {
            label: '현재 프리셋',
            value: '배경지 - 핑크',
          },
          latestPhoto: {
            kind: 'ready',
            title: '방금 저장된 사진이에요.',
            supporting: '현재 세션에서 가장 최근에 저장된 사진입니다.',
            assetUrl: 'asset://session-24/capture-2',
            alt: '현재 세션의 최신 촬영 사진 미리보기',
          },
          guidance: '리모컨으로 촬영을 계속해 주세요.',
          timingAlert: {
            kind: 'none',
          },
        }}
      />,
    )

    expect(screen.getByRole('img', { name: '현재 세션의 최신 촬영 사진 미리보기' })).toBeInTheDocument()
    expect(screen.queryByRole('img', { name: '선택된 세션 사진 미리보기' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: '사진 크게 보기' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: '사진 삭제' })).not.toBeInTheDocument()
    expect(screen.queryByLabelText('세션 사진 썸네일')).not.toBeInTheDocument()
  })

  it('renders the provided verified preset catalog service inside the in-session preset selector', async () => {
    const user = userEvent.setup()

    render(
      <CaptureScreen
        captureActionDisabled
        captureState={createCaptureFlowState({
          activePresetId: 'background-pink',
          isPresetSelectorOpen: true,
          sessionEndTimeLabel: '오후 7:50',
        })}
        onCapture={() => undefined}
        onClosePresetSelector={() => undefined}
        onDismissPresetChangeFeedback={() => undefined}
        onOpenPresetSelector={() => undefined}
        onSelectPreset={() => undefined}
        presetCatalogService={{
          loadApprovedPresetCatalog: async () => ({
            status: 'ready',
            source: 'approved',
            presets: [
              {
                id: 'background-pink',
                name: '배경지 - 핑크',
                group: 'background',
                previewAssetPath: '/src/customer-flow/assets/preset-previews/background-pink.svg',
              },
            ],
          }),
        }}
        sessionName="session-24-kim"
        view={{
          endTime: {
            label: '촬영 종료 시간',
            value: '오후 7:50',
            supporting: '이 시간까지 촬영할 수 있어요.',
          },
          preset: {
            label: '현재 프리셋',
            value: '배경지 - 핑크',
          },
          latestPhoto: {
            kind: 'ready',
            title: '방금 저장된 사진이에요.',
            supporting: '현재 세션에서 가장 최근에 저장된 사진입니다.',
            assetUrl: 'asset://session-24/capture-2',
            alt: '현재 세션의 최신 촬영 사진 미리보기',
          },
          guidance: '리모컨으로 촬영을 계속해 주세요.',
          timingAlert: {
            kind: 'none',
          },
        }}
      />,
    )

    const presetButton = await screen.findByRole('button', { name: /배경지 - 핑크/i })
    expect(screen.queryByRole('button', { name: /웜톤/i })).not.toBeInTheDocument()

    await user.click(presetButton)
  })

})
