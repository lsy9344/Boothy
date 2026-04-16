import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'vitest'

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..')

const readRepoFile = (...segments: string[]) =>
  readFileSync(resolve(repoRoot, ...segments), 'utf8')

const storyFiles = [
  {
    file: '_bmad-output/implementation-artifacts/1-4-준비-상태-안내와-유효-상태에서만-촬영-허용.md',
    status: 'done',
    gate: 'Go',
    gateHeading: '### Hardware Gate Reference',
    automationMarker: 'automated pass',
  },
  {
    file: '_bmad-output/implementation-artifacts/1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백.md',
    status: 'done',
    gate: 'Go',
    gateHeading: '### Hardware Gate Reference',
    automationMarker: 'automated pass',
  },
  {
    file: '_bmad-output/implementation-artifacts/1-6-실카메라-helper-readiness-truth-연결과-false-ready-차단.md',
    status: 'done',
    gate: 'Go',
    gateHeading: '### Hardware Gate Reference',
    automationMarker: 'automated pass',
  },
  {
    file: '_bmad-output/implementation-artifacts/3-2-export-waiting과-truthful-completion-안내.md',
    status: 'done',
    gate: 'Go',
    gateHeading: '### Hardware Gate Reference',
    automationMarker: 'automated pass',
  },
  {
    file: '_bmad-output/implementation-artifacts/1-13-guarded-cutover와-original-visible-to-preset-applied-visible-hardware-validation-gate.md',
    status: 'backlog',
    gate: 'No-Go',
    gateHeading: '### Validation Gate Reference',
    automationMarker: 'automated regression/build proof',
  },
  {
    file: '_bmad-output/implementation-artifacts/4-2-부스-호환성-검증과-승인-준비-상태-전환.md',
    status: 'done',
    gate: 'Go',
    gateHeading: '### Hardware Gate Reference',
    automationMarker: 'automated pass',
  },
  {
    file: '_bmad-output/implementation-artifacts/4-3-승인과-불변-게시-아티팩트-생성.md',
    status: 'review',
    gate: 'No-Go',
    gateHeading: '### Hardware Gate Reference',
    automationMarker: 'automated pass',
  },
] as const

const canonicalSprintStoryKeys = [
  '1-2-이름과-뒤4자리-기반-세션-시작',
  '1-3-승인된-프리셋-카탈로그와-활성-프리셋-선택',
  '1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백',
  '1-6-실카메라-helper-readiness-truth-연결과-false-ready-차단',
  '1-21-metric-reset과-full-screen-2500ms-acceptance-정렬',
  '1-22-capture-full-screen-visible-evidence-chain-trace-reset',
  '1-23-local-full-screen-lane-prototype과-truthful-artifact-generation',
  '2-1-현재-세션-전용-사진-검토-화면',
  '2-2-현재-세션-삭제-정책-기반-사진-삭제',
  '2-3-세션-중-활성-프리셋-변경',
  '2-4-조정된-종료-시각-상시-노출',
  '3-2-export-waiting과-truthful-completion-안내',
  '4-1-내부-프리셋-작성-작업공간',
  '4-2-부스-호환성-검증과-승인-준비-상태-전환',
  '4-4-미래-세션-대상-롤백',
  '5-1-운영자용-현재-세션-문맥과-장애-진단',
  '5-3-라이프사이클과-개입-감사-로그',
  '5-4-카메라-연결-상태-전용-진단-항목',
  '6-1-지점별-단계적-배포와-단일-액션-롤백',
] as const

const canonicalEpicHeadings = [
  '### Story 1.5: 현재 세션 촬영 저장과 truthful preview waiting 피드백',
  '### Story 1.6: 실카메라 helper readiness truth 연결과 false-ready 차단',
  '### Story 3.2: Export Waiting과 truthful completion 안내',
  '### Story 4.2: 부스 호환성 검증과 승인 준비 상태 전환',
] as const

describe('hardware validation governance baseline', () => {
  it('locks the canonical gate map and sprint review board in one sprint-owned artifact', () => {
    const ledger = readRepoFile(
      '_bmad-output',
      'implementation-artifacts',
      'hardware-validation-ledger.md',
    )

    expect(ledger).toContain('Story 1.4')
    expect(ledger).toContain('HV-02, HV-03, HV-10')
    expect(ledger).toContain('Story 1.5')
    expect(ledger).toContain('HV-04, HV-05')
    expect(ledger).toContain('Story 1.6')
    expect(ledger).toContain('Story 1.8')
    expect(ledger).toContain('HV-05/HV-07/HV-08/HV-11/HV-12')
    expect(ledger).toContain('Story 1.13')
    expect(ledger).toContain('HV-00, HV-04, HV-05, HV-07, HV-08, HV-10, HV-11, HV-12')
    expect(ledger).toContain('Story 3.2')
    expect(ledger).toContain('HV-08, HV-11')
    expect(ledger).toContain('Story 4.2')
    expect(ledger).toContain('HV-01, HV-09')
    expect(ledger).toContain('Story 4.3')
    expect(ledger).toContain('HV-01, HV-07, HV-12')
    expect(ledger).toContain('Automated Pass')
    expect(ledger).toContain('Hardware Pass')
    expect(ledger).toContain('Go / No-Go')
    expect(ledger).toContain('Latency')
    expect(ledger).toContain('Parity')
    expect(ledger).toContain('Fallback Ratio')
    expect(ledger).toContain('Route Policy State')
    expect(ledger).toContain('Rollback Evidence')
    expect(ledger).toContain('Blocker')
    expect(ledger).toContain('Owner')
    expect(ledger).toContain('Evidence Path')
    expect(ledger).toContain('story key')
    expect(ledger).toContain('HV checklist ID')
    expect(ledger).toContain('evidence package path')
    expect(ledger).toContain('executedAt')
    expect(ledger).toContain('validator')
    expect(ledger).toContain('booth PC')
    expect(ledger).toContain('camera model')
    expect(ledger).toContain('darktable pin')
    expect(ledger).toContain('helper identifier')
    expect(ledger).toContain('sameCaptureFullScreenVisibleMs')
    expect(ledger).toContain('Story 1.22 resets the selected-capture evidence chain')
    expect(ledger).toContain('visibleOwnerTransitionAtMs')
    expect(ledger).toContain('replacementMs')
    expect(ledger).toContain('parity')
    expect(ledger).toContain('fallback ratio')
    expect(ledger).toContain('route policy state')
    expect(ledger).toContain('rollback evidence')
    expect(ledger).toContain('release blocker')
    expect(ledger).toContain('follow-up owner')
    expect(ledger).toContain('core evidence paths')
  })

  it('keeps the runbook and release baseline aligned to the canonical gate policy', () => {
    const runbook = readRepoFile('docs', 'runbooks', 'booth-hardware-validation-checklist.md')
    const evidencePackage = readRepoFile(
      'docs',
      'runbooks',
      'preview-promotion-evidence-package.md',
    )
    const releaseBaseline = readRepoFile('docs', 'release-baseline.md')
    const rootReleaseBaseline = readRepoFile('release-baseline.md')

    expect(runbook).toContain('canonical release-gated stories')
    expect(runbook).toContain('Story 1.4')
    expect(runbook).toContain('Story 1.13')
    expect(runbook).not.toContain('- Story 1.3: 승인된 프리셋 카탈로그 표시와 활성 프리셋 선택')
    expect(runbook).toContain('Story 2.3')
    expect(runbook).toContain('Story 1.19')
    expect(runbook).toContain('hardware-validation-ledger.md')
    expect(runbook).toContain('HV-00, HV-01, HV-02, HV-03, HV-04, HV-05, HV-07, HV-09, HV-10, HV-11, HV-12')
    expect(runbook).toContain('Story 1.5를 `review`로 되돌린다')
    expect(runbook).toContain('Story 2.3 / 4.3 경계를 우선 재점검한다')
    expect(runbook).toContain('HV-09 실패: Story 4.2 경계를 우선 재점검한다')
    expect(runbook).toContain('preview-promotion-evidence.jsonl')
    expect(runbook).toContain('Start-PreviewPromotionTrace.ps1')
    expect(runbook).toContain('New-PreviewPromotionEvidenceBundle.ps1')

    expect(evidencePackage).toContain(
      'same-capture preset-applied full-screen visible <= 2500ms',
    )
    expect(evidencePackage).toContain('same `2500ms` threshold')
    expect(evidencePackage).toContain('legacy comparison only')
    expect(evidencePackage).toContain('new-track release field')
    expect(evidencePackage).toContain('selected capture chain only')
    expect(evidencePackage).toContain('visibleOwner')
    expect(evidencePackage).toContain('visibleOwnerTransitionAtMs')
    expect(evidencePackage).toContain('Story 1.22 owns the trace/evidence reset only')

    expect(releaseBaseline).toContain('automated proof')
    expect(releaseBaseline).toContain('hardware proof')
    expect(releaseBaseline).toContain('hardware-validation-ledger.md')
    expect(releaseBaseline).toContain('Go')
    expect(releaseBaseline).toContain('No-Go')
    expect(releaseBaseline).toContain('release hold')
    expect(releaseBaseline).toContain('Story 1.13')
    expect(releaseBaseline).toContain('preview-renderer-policy.json')
    expect(releaseBaseline).toContain('latency, parity, fallback ratio, route policy state, and rollback evidence')
    expect(releaseBaseline).toContain(
      'same-capture preset-applied full-screen visible <= 2500ms',
    )
    expect(releaseBaseline).toContain('same `2500ms` threshold')
    expect(releaseBaseline).toContain('legacy comparison only')
    expect(releaseBaseline).toContain('selected-capture evidence reset owner')
    expect(releaseBaseline).toContain('selected-capture timing events only')

    expect(rootReleaseBaseline).toContain(
      'same-capture preset-applied full-screen visible <= 2500ms',
    )
    expect(rootReleaseBaseline).toContain('same `2500ms` threshold')
    expect(rootReleaseBaseline).toContain('legacy comparison only')
    expect(rootReleaseBaseline).toContain('selected-capture evidence reset owner')
  })

  it('keeps the local-lane prototype ownership boundary explicit across contract docs', () => {
    const localDedicatedRendererContract = readRepoFile(
      'docs',
      'contracts',
      'local-dedicated-renderer.md',
    )
    const renderWorkerContract = readRepoFile('docs', 'contracts', 'render-worker.md')
    const sessionManifestContract = readRepoFile('docs', 'contracts', 'session-manifest.md')
    const releaseBaseline = readRepoFile('docs', 'release-baseline.md')

    expect(localDedicatedRendererContract).toContain('Story 1.23')
    expect(localDedicatedRendererContract).toContain('prototype owner')
    expect(localDedicatedRendererContract).toContain('Story 1.24')
    expect(localDedicatedRendererContract).toContain('Story 1.25')
    expect(localDedicatedRendererContract).toContain('Story 1.13')
    expect(localDedicatedRendererContract).toContain('darktable-compatible')
    expect(localDedicatedRendererContract).toContain('parity/fallback/final reference')
    expect(localDedicatedRendererContract).toContain('truthfulArtifactReadyAtMs')
    expect(localDedicatedRendererContract).toContain('sameCaptureFullScreenVisibleMs')
    expect(localDedicatedRendererContract).toContain('visibleOwner')
    expect(localDedicatedRendererContract).toContain('visibleOwnerTransitionAtMs')
    expect(localDedicatedRendererContract).toContain('captured-preview-renderer-policy')
    expect(localDedicatedRendererContract).toContain('captured-catalog-state')

    expect(renderWorkerContract).toContain('Story 1.23')
    expect(renderWorkerContract).toContain('display-sized preset-applied truthful artifact')
    expect(renderWorkerContract).toContain('darktable-compatible path remains the parity/fallback/final reference')
    expect(renderWorkerContract).toContain('Story 1.24 canary')
    expect(renderWorkerContract).toContain('Story 1.25 default/rollback')
    expect(renderWorkerContract).toContain('Story 1.13 final release close')

    expect(sessionManifestContract).toContain('capture-time route snapshot')
    expect(sessionManifestContract).toContain('later policy change')
    expect(sessionManifestContract).toContain('reinterpreted')
    expect(sessionManifestContract).toContain('Story 1.23 prototype owner')

    expect(releaseBaseline).toContain('Story 1.23 local lane prototype')
    expect(releaseBaseline).toContain('Story 1.24 canary proof')
    expect(releaseBaseline).toContain('Story 1.25 default/rollback proof')
    expect(releaseBaseline).toContain('Story 1.13 remains the final guarded cutover / release-close owner')
  })

  it('keeps sprint status aligned with the ledger-recorded close state', () => {
    const sprintStatus = readRepoFile(
      '_bmad-output',
      'implementation-artifacts',
      'sprint-status.yaml',
    )

    expect(sprintStatus).toContain('1-13-guarded-cutover와-hardware-validation-gate: backlog')
    expect(sprintStatus).toContain('1-4-준비-상태-안내와-유효-상태에서만-촬영-허용: done')
    expect(sprintStatus).toContain('1-5-현재-세션-촬영-저장과-truthful-preview-waiting-피드백: done')
    expect(sprintStatus).toContain(
      '1-6-실카메라-helper-readiness-truth-연결과-false-ready-차단: done',
    )
    expect(sprintStatus).toContain('1-21-metric-reset과-full-screen-2500ms-acceptance-정렬: done')
    expect(sprintStatus).toContain('3-2-export-waiting과-truthful-completion-안내: done')
    expect(sprintStatus).toContain('4-2-부스-호환성-검증과-승인-준비-상태-전환: done')
    expect(sprintStatus).toContain('4-3-승인과-불변-게시-아티팩트-생성: review')
    expect(sprintStatus).toContain('1-18-retired-dedicated-close-후보-activation-baseline-evidence-보존: backlog')
    expect(sprintStatus).toContain('1-19-legacy-validated-track-parity와-instrumentation-ledger-정리: backlog')
    expect(sprintStatus).toContain('1-20-legacy-route-activation-validation-track-유지: backlog')
  })

  it('keeps existing story ids stable across sprint tracking and planning artifacts', () => {
    const sprintStatus = readRepoFile(
      '_bmad-output',
      'implementation-artifacts',
      'sprint-status.yaml',
    )
    const epics = readRepoFile('_bmad-output', 'planning-artifacts', 'epics.md')

    for (const storyKey of canonicalSprintStoryKeys) {
      expect(sprintStatus).toContain(`${storyKey}:`)
    }

    for (const heading of canonicalEpicHeadings) {
      expect(epics).toContain(heading)
    }
  })

  it('normalizes hardware gate references across every impacted story document', () => {
    for (const storyFile of storyFiles) {
      const story = readRepoFile(...storyFile.file.split('/'))

      expect(story).toContain(`Status: ${storyFile.status}`)
      expect(story).toContain(storyFile.gateHeading)
      expect(story).toContain('hardware-validation-ledger.md')
      expect(story).toContain(`Current hardware gate: \`${storyFile.gate}\``)
      expect(story).toContain(storyFile.automationMarker)
    }
  })

  it('closes Story 1.8 once canonical booth evidence is recorded in the ledger', () => {
    const story18 = readRepoFile(
      '_bmad-output',
      'implementation-artifacts',
      '1-8-게시된-프리셋-xmp-적용-preview-final-render-worker-연결.md',
    )
    const ledger = readRepoFile(
      '_bmad-output',
      'implementation-artifacts',
      'hardware-validation-ledger.md',
    )

    expect(story18).toContain('Status: done')
    expect(story18).toContain('### Validation Gate Reference')
    expect(story18).toContain('Current hardware gate: `Go`')
    expect(story18).toContain('Close policy: automated pass만으로 닫지 않는다.')
    expect(ledger).toContain('### Story 1.8')
    expect(ledger).toContain('Go / No-Go result: `Go`')
    expect(ledger).toContain('session_000000000018a4df863488433c')
    expect(ledger).toContain('session_000000000018a4e49821e18790')
    expect(ledger).toContain('xmp/test-look.xmp')
    expect(ledger).toContain('xmp/template.xmp')
  })

  it('removes stale done-era notes from stories that are still under a No-Go hardware gate', () => {
    const story32 = readRepoFile(
      '_bmad-output',
      'implementation-artifacts',
      '3-2-export-waiting과-truthful-completion-안내.md',
    )

    expect(story32).not.toContain('상태를 `done`으로 반영했다')
  })

  it('keeps governance context snapshots aligned with the current sprint state', () => {
    const story43 = readRepoFile(
      '_bmad-output',
      'implementation-artifacts',
      '4-3-승인과-불변-게시-아티팩트-생성.md',
    )
    const story62 = readRepoFile(
      '_bmad-output',
      'implementation-artifacts',
      '6-2-실장비-hardware-validation-gate와-evidence-기반-done-정책.md',
    )

    expect(story43).not.toContain('현재 sprint 상태상 4.2는 `ready-for-dev`이며 4.1은 아직 backlog다.')
    expect(story62).not.toContain('Story 5.4는 계속 `ready-for-dev`')
    expect(story62).toContain('Story 1.4, 1.5, 1.6, 1.8, 3.2는 `done`')
    expect(story62).toContain('Story 4.2, 4.3은 `review`로 유지')
    expect(story62).toContain('Story 5.4는 `review`')
    expect(story62).toContain('Story 6.2는 governance story로 `done`')
  })

  it('keeps Story 1.7 as supporting evidence only for the canonical 1.5 close owner', () => {
    const story17 = readRepoFile(
      '_bmad-output',
      'implementation-artifacts',
      '1-7-실카메라-capture-round-trip과-raw-handoff-correlation.md',
    )

    expect(story17).toContain('Story 1.5 close review에 공급하는 Story 1.7 primary supporting correlation evidence')
    expect(story17).toContain('canonical close owner는 Story 1.5 ledger row')
    expect(story17).not.toContain('HV-04: Story 1.7 primary closure evidence')
  })
})
