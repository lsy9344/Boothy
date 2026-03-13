import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { BranchConfigContext } from '../../branch-config/BranchConfigContext.js';
import { approvedBoothPresetCatalog } from '../../preset-catalog/services/presetCatalogService.js';
import { CustomerFlowContent } from './CustomerFlowScreen.js';
import { SessionFlowProvider } from '../../session-domain/state/SessionFlowProvider.js';

function createReadinessAdapter() {
  return {
    getReadinessSnapshot: vi.fn(async ({ sessionId }: { sessionId: string }) => ({
      sessionId,
      connectionState: 'preparing' as const,
      captureEnabled: false,
      lastStableCustomerState: null,
      error: null,
      emittedAt: '2026-03-13T09:00:02.000Z',
    })),
    watchReadiness: vi.fn(async () => () => undefined),
    getCaptureConfidenceSnapshot: vi.fn(async () => {
      throw new Error('capture confidence not used in this test');
    }),
    watchCaptureConfidence: vi.fn(async () => () => undefined),
  };
}

function createSessionTimingService() {
  return {
    initializeSessionTiming: vi.fn(),
    getSessionTiming: vi.fn(async () => ({
      ok: true as const,
      value: {
        sessionId: 'session-42',
        manifestPath: 'C:/sessions/session-42/session.json',
        timing: {
          reservationStartAt: '2026-03-13T09:00:00.000Z',
          actualShootEndAt: '2099-03-13T09:50:00.000Z',
          sessionType: 'standard' as const,
          operatorExtensionCount: 0,
          lastTimingUpdateAt: '2026-03-13T09:00:00.000Z',
        },
      },
    })),
    extendSessionTiming: vi.fn(),
  };
}

function renderCustomerFlow(options: {
  startSession: () => Promise<
    | {
        ok: true;
        value: {
          sessionId: string;
          sessionName: string;
          sessionFolder: string;
          manifestPath: string;
          createdAt: string;
          preparationState: 'preparing';
        };
      }
    | {
        ok: false;
        errorCode: 'session.validation_failed' | 'session.provisioning_failed' | 'session_name.required';
        message: string;
      }
  >;
}) {
  return render(
    <BranchConfigContext.Provider
      value={{
        status: 'ready',
        config: {
          branchId: 'gangnam-main',
          branchPhoneNumber: '010-1234-5678',
          operationalToggles: {
            enablePhoneEscalation: true,
          },
        },
      }}
    >
      <SessionFlowProvider
        cameraAdapter={createReadinessAdapter()}
        lifecycleLogger={{ recordReadinessReached: vi.fn(async () => undefined) }}
        lifecycleService={{
          startSession: vi.fn(options.startSession),
        }}
        presetCatalogService={{
          loadApprovedPresetCatalog: vi.fn(async () => ({
            status: 'ready' as const,
            presets: approvedBoothPresetCatalog,
            source: 'approved' as const,
          })),
        }}
        sessionTimingService={createSessionTimingService()}
      >
        <CustomerFlowContent />
      </SessionFlowProvider>
    </BranchConfigContext.Provider>,
  );
}

describe('CustomerFlowContent', () => {
  it('uses the production start-session path from customer entry to preparation', async () => {
    const user = userEvent.setup();

    renderCustomerFlow({
      startSession: async () => ({
        ok: true,
        value: {
          sessionId: 'session-42',
          sessionName: '김보라 오후 세션',
          sessionFolder: 'C:/sessions/session-42',
          manifestPath: 'C:/sessions/session-42/session.json',
          createdAt: '2026-03-13T09:00:00.000Z',
          preparationState: 'preparing',
        },
      }),
    });

    await user.type(await screen.findByRole('textbox', { name: '세션 이름' }), '김보라 오후 세션');
    await user.click(screen.getByRole('button', { name: '시작하기' }));

    expect(await screen.findByText('세션 이름을 확인했어요.')).toBeInTheDocument();
    expect(screen.getByText('김보라 오후 세션')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: '계속하기' }));

    await waitFor(() => {
      expect(screen.getByText('촬영 준비 중입니다. 잠시만 기다려 주세요.')).toBeInTheDocument();
    });
    expect(screen.getByLabelText('세션 이름')).toHaveTextContent('김보라 오후 세션');
  });

  it('returns to the customer start surface with safe copy when host validation fails after handoff', async () => {
    const user = userEvent.setup();

    renderCustomerFlow({
      startSession: async () => ({
        ok: false,
        errorCode: 'session.validation_failed',
        message: 'Session details are invalid.',
      }),
    });

    await user.type(await screen.findByRole('textbox', { name: '세션 이름' }), '김보라 오후 세션');
    await user.click(screen.getByRole('button', { name: '시작하기' }));
    await user.click(await screen.findByRole('button', { name: '계속하기' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('세션 정보를 확인한 뒤 다시 시도해 주세요.');
    expect(screen.getByRole('textbox', { name: '세션 이름' })).toHaveValue('김보라 오후 세션');
    expect(screen.queryByText('Session details are invalid.')).not.toBeInTheDocument();
    expect(screen.queryByText('세션 이름을 확인했어요.')).not.toBeInTheDocument();
  });
});
