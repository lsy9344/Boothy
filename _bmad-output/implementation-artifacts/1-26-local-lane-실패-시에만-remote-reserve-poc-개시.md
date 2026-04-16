# Story 1.26: local lane 실패 시에만 remote reserve POC 개시

Status: backlog

Ordering Note: Story 1.26은 Story 1.25가 local lane `Go` 후보와 rollback proof를 닫은 뒤에도, 승인 하드웨어에서 같은 KPI를 반복 실패할 때만 열리는 conditional reserve track이다. 이 스토리는 Story 1.13 final guarded cutover / release close를 대체하지 않으며, local forward path를 먼저 고갈한 다음에만 검토한다.

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

owner / brand operator로서,
local lane이 반복적으로 KPI를 실패할 때만 remote reserve option을 열고 싶다,
그래서 현재 release path를 불필요하게 분산시키지 않고 reserve experiment를 통제된 범위에서만 다룰 수 있다.

## Acceptance Criteria

1. local full-screen lane이 prototype, parity, canary, default decision 단계를 거친 뒤에도 승인 하드웨어에서 같은 KPI를 반복 실패할 때, remote renderer / edge appliance POC를 별도 실험 track으로 열 수 있어야 한다. 또한 local lane이 `Go` 후보를 만들 수 있는 동안에는 reserve POC를 열면 안 된다.
2. reserve POC를 시작할 때는 current release-close path와 customer default route가 local lane 기준을 유지해야 한다. 또한 remote reserve 결과는 별도 승인 없이는 Story 1.13의 `Go / No-Go` 경로를 대체하면 안 된다.

## Tasks / Subtasks

- [ ] reserve track 개시 조건을 문서와 운영 baseline에 고정한다. (AC: 1, 2)
  - [ ] `epics.md`, `sprint-plan-preview-architecture-track-20260415.md`, `docs/release-baseline.md`가 Story 1.26을 conditional reserve track으로만 읽도록 맞춘다.
  - [ ] Story 1.25와 Story 1.27 이후에만 reserve 검토가 열리고, Story 1.13 release-close ownership을 흡수하지 않는다는 문구를 유지한다.

- [ ] reserve POC가 local forward path를 대체하지 못하게 한다. (AC: 1, 2)
  - [ ] remote renderer / edge appliance를 별도 experiment track으로만 취급하고, customer default route와 current release-close path는 local lane 기준을 유지한다.
  - [ ] reserve 결과가 하나의 alternate path처럼 보이더라도 승인 없는 `Go / No-Go` 대체로 읽히지 않게 한다.

- [ ] conditional reserve gate regression을 잠근다. (AC: 1, 2)
  - [ ] reservation-open, reservation-blocked, local-Go-still-viable, approval-required negative case를 문서/테스트 baseline에서 구분한다.
  - [ ] 반복 실패가 증명되지 않으면 reserve track이 열리지 않도록 fail-closed wording을 유지한다.

## Dev Notes

### 왜 이 스토리가 필요한가

- epics는 Story 1.26을 reserve track으로 정의하고, local forward path가 같은 KPI를 반복 실패할 때만 열리도록 조건을 잠갔다. [Source: _bmad-output/planning-artifacts/epics.md#Story 1.26: local lane 실패 시에만 remote reserve POC 개시]
- architecture는 remote renderer / edge appliance를 reserve option으로 남기고, local prototype과 canary/default decision을 먼저 고갈한 뒤에만 검토하라고 정리한다. [Source: _bmad-output/planning-artifacts/architecture.md#Preview Architecture Realignment]
- Story 1.13은 여전히 final guarded cutover / release-close owner이므로, Story 1.26은 그 경로를 대체하는 문맥으로 읽히면 안 된다. [Source: _bmad-output/planning-artifacts/sprint-plan-preview-architecture-track-20260415.md]

### 스토리 목적과 범위

- 이번 스토리의 핵심은 remote reserve option을 미리 여는 일이 아니라, local path를 충분히 검증한 뒤에만 reserve experiment를 허용하는 조건을 문서적으로 고정하는 것이다.
- 이번 스토리는 아래를 소유한다.
  - conditional reserve 개시 기준
  - local path 우선 원칙
  - Story 1.13 release-close ownership 보호
- 아래 작업은 이번 스토리 범위가 아니다.
  - remote reserve 구현 자체
  - current local lane fallback 제거
  - final guarded cutover 결정

### 스토리 기반 요구사항

- PRD는 same-capture preset-applied full-screen visible 기준을 primary release sign-off로 둔다. reserve track은 이 primary KPI를 대체하지 못한다. [Source: _bmad-output/planning-artifacts/prd.md#NFR-003 Booth Responsiveness and Preview Readiness]
- sprint plan은 Story 1.26을 default implementation path 밖에 두고, local forward path가 반복 실패할 때만 여는 conditional branch로 정의한다. [Source: _bmad-output/planning-artifacts/sprint-plan-preview-architecture-track-20260415.md]
- epics도 Story 1.26이 Story 1.13의 `Go / No-Go` 경로를 대체하면 안 된다고 명시한다. [Source: _bmad-output/planning-artifacts/epics.md#Story 1.26: local lane 실패 시에만 remote reserve POC 개시]

### 구현 가드레일

- Story 1.26은 Story 1.25가 local lane `Go` 후보를 만들기 전에 열리면 안 된다.
- Story 1.26은 Story 1.27 corrective validation을 건너뛰는 지름길이 되면 안 된다.
- remote reserve 결과는 local lane의 canonical release-close 결과처럼 읽히면 안 된다.
- approval 없이 Story 1.13의 `Go / No-Go` 판단을 대체하는 문구를 쓰면 안 된다.

