# Contracts Guide

이 문서는 `docs/contracts/` 문서의 역할을 빠르게 구분하기 위한 안내서다.

## 가장 먼저 볼 계약

1. [Render Worker Contract](./render-worker.md)
2. [Session Manifest Contract](./session-manifest.md)

## 그 다음 필요할 때 보는 계약

- [Preset Bundle Contract](./preset-bundle.md)
  - preset 적용 입력과 publication 산출물 관련 계약
- [Camera Helper Sidecar Protocol](./camera-helper-sidecar-protocol.md)
  - helper sidecar와 앱 사이 통신 계약
- [Camera Helper EDSDK Profile](./camera-helper-edsdk-profile.md)
  - EDSDK 장치 프로파일과 helper 조건
- [Branch Rollout Contract](./branch-rollout.md)
  - branch rollout 절차와 조건
- [Authoring Publication](./authoring-publication.md)
  - authoring publication 개념과 구조
- [Authoring Publication Payload](./authoring-publication-payload.md)
  - publication payload 상세 형식

## 읽기 원칙

- preview-track 문제를 이해할 때는 `render-worker`와 `session-manifest`를 먼저 본다.
- 나머지 문서는 특정 subsystem이나 publication 흐름을 볼 때만 추가로 읽는다.
