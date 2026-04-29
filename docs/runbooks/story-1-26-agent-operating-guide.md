---
documentType: agent-operating-guide
status: active
date: 2026-04-29
scope: story-1-26-preview-route
---

# Story 1.26 Agent Operating Guide

이 문서는 Story `1.26`을 다룰 때 AI agent가 긴 이력을 매번 전부 읽지 않게 하기 위한 작업 규칙이다.

## 현재 관리 원칙

Story `1.26` 파일은 과거 로그 저장소가 아니라 현재 작업 지시서로 유지한다.

- Story 파일에는 최신 상태, 합격 조건, 남은 blocker, 최신 검증 요약만 둔다.
- 공식 `Go / No-Go` 판정은 `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`가 소유한다.
- 긴 원인 분석과 과거 run 해석은 `docs/runbooks/story-1-26-review-root-cause-and-improvement-direction-20260427.md`에 둔다.
- raw log는 문서에 붙여 넣지 않고 session/run 경로만 기록한다.
- 과거 반복 실패 로그는 최신 결론을 바꾸는 경우에만 다시 읽는다.

## Agent Reading Budget

작업 시작 시 전체 파일을 통째로 읽지 않는다.

1. Story 파일 상단 80줄만 읽는다.
2. Story 파일 마지막 120줄만 읽는다.
3. ledger는 `Last Updated`, `Current Preview Track Interpretation`, `1.26` row만 읽는다.
4. 최신 hardware run은 `run-summary.json`과 `run-steps.jsonl`의 실패 step만 읽는다.
5. 앱 로그는 최신 session id, `file-arrived`, `fast-preview-ready`, `preview-render-ready`, `capture_preview_ready` 라인만 뽑아 읽는다.
6. `git status`는 먼저 `git diff --stat` 또는 제한된 path로 확인한다.
7. 큰 `git diff`, 전체 story, 전체 ledger, 전체 runbook은 사용자가 명시하거나 최신 결론이 충돌할 때만 읽는다.

## 최신 제품 판단

Story `1.26`은 현재 latest approved-hardware package 기준 fresh `Go` evidence가 있다. Story close 자체는 human/product review policy를 따른다.

최신 blocker는 카메라 저장 실패, `Preview Waiting` cleanup 실패, native RAW over-white false Go, full-preset truthful artifact 부재가 아니다.

Latest approved-hardware evidence:

- hardware-validation-run `1777434275752` passed `5/5`.
- session `session_000000000018aab70e79e5baa8` produced same-capture resident full-preset route evidence.
- route evidence includes `binary=fast-preview-handoff`, `source=fast-preview-handoff`, `engineMode=resident-full-preset`, `engineAdapter=darktable-compatible`, `inputSourceAsset=raw-original`, `sourceAsset=preset-applied-preview`, `truthOwner=display-sized-preset-applied`, `truthProfile=original-full-preset`.
- official `originalVisibleToPresetAppliedVisibleMs` band was `2316ms ~ 2338ms`.
- the prior 11:38 native approximation pass remains retracted as false Go.

Current answer to track:

- The correct Story `1.26` product path is option 2: a resident/long-lived darktable-compatible full-preset owner.
- The accepted preview is a same-capture artifact generated from the original RAW input, not a thumbnail, fast raster, or native approximation.
- The accepted preview must keep these route fields together: `inputSourceAsset=raw-original`, `sourceAsset=preset-applied-preview`, `truthOwner=display-sized-preset-applied`, `truthProfile=original-full-preset`, `engineMode=resident-full-preset`, `engineAdapter=darktable-compatible`.
- The latest pass proves the route can meet the official customer-visible window on approved hardware. It does not make partial native RAW approximation an official product path.
- The next product work is hardening this resident owner and preserving route evidence, not tuning per-capture fallback.

Current direction:

- Do not continue partial native approximation as the product path.
- Option 2 is now the implemented active route: resident/long-lived darktable-compatible engine ownership that produces the actual full preset result.
- This is not per-capture darktable fallback tail tuning. Product follow-up should harden the resident engine owner and keep route evidence honest.
- Native RAW output remains comparison-only unless it can honestly prove `truthProfile=original-full-preset`.

## 검증 데이터 기록 규칙

하드웨어 검증 뒤에는 네 곳만 갱신한다.

- Story 파일: 최신 run 요약 1개 섹션
- Ledger: `Last Updated`, preview route snapshot, `1.26` row, evidence path
- Sprint status: 최신 requested validation 한 줄
- Root-cause runbook: 새로 배운 원인이나 개선 방향이 있을 때만 추가

반복 실패가 같은 원인이면 긴 설명을 다시 쓰지 않는다.

추천 문장:

> 최신 run은 resident darktable-compatible full-preset route로 `5/5` 통과했다. Official timing은 `2316ms ~ 2338ms`였고 route evidence는 `truthProfile=original-full-preset`, `engineMode=resident-full-preset`, `inputSourceAsset=raw-original`, `sourceAsset=preset-applied-preview`를 포함한다.

## 사용자 기본 프롬프트

아래 프롬프트는 Story `1.26` 작업에 맞춘 효율 버전이다.

```text
최근 앱 실행 로그파일을 검토하여 문제를 개선하세요.

토큰 절약 규칙:
- Story 1.26 전체를 통째로 읽지 말고, 상단 80줄과 마지막 120줄만 먼저 읽으세요.
- 먼저 docs/runbooks/story-1-26-agent-operating-guide.md를 읽고 그 규칙을 따르세요.
- hardware-validation-ledger.md는 Last Updated, Current Preview Track Interpretation, 1.26 row만 확인하세요.
- 앱 로그는 최신 session의 file-arrived, fast-preview-ready, preview-render-ready, capture_preview_ready 라인만 먼저 확인하세요.
- git 상태는 git diff --stat 또는 관련 path로만 먼저 확인하고, 큰 diff는 필요한 파일만 보세요.
- raw log와 긴 과거 이력은 문서에 붙여 넣지 말고 경로와 핵심 수치만 기록하세요.

작업 목표:
- 최신 앱 로그에서 Story 1.26의 현재 blocker가 바뀌었는지 확인하세요.
- 문제가 코드/검증기/문서 중 어디에 있는지 최소 범위로 개선하세요.
- 관련 문서 및 스토리 문서에는 최신 검증 데이터만 간결히 기록하세요.
- 반복 원인이 같으면 긴 설명을 다시 쓰지 말고 "동일 원인 반복"으로 요약하세요.

하드웨어 검증 자동화 스크립트를 실행하고 결과 로그를 확인한 뒤 문서에 기록하세요.

powershell -ExecutionPolicy Bypass -File C:\Code\Project\Boothy_lrc_first_visible\scripts\hardware-validation-runner.ps1 -Prompt "Kim4821"
```

## Agent Stop Rule

다음 조건이면 긴 탐색을 멈추고 최신 상태를 바로 요약한다.

- latest requested run이 `passed / 5/5`다.
- route evidence가 `truthProfile=original-full-preset`, `engineMode=resident-full-preset`, `inputSourceAsset=raw-original`, `sourceAsset=preset-applied-preview`를 포함한다.
- official timing band is inside `3000ms`.

이 경우 결론은 새 추측이 아니라 option 2 latest approved-hardware `Go` evidence다.

다음 실제 개선은 partial native tuning이 아니라 resident/long-lived actual preset engine hardening이다.
