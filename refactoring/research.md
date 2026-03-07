# Boothy Refactoring Research (Superseded)

작성일: 2026-03-07
작성자: Codex
상태: Historical alternatives only
현재 기준 문서: `refactoring/research-codex.md`

이 문서는 더 이상 현재 권고안을 정의하는 주 문서가 아니다.
이 문서는 이전에 검토했던 대안들과, 왜 그 대안들이 현재 주 경로에서 내려왔는지를 짧게 정리하는 용도로만 남긴다.

## 1. What This Older Thread Still Contributes

이전 연구에서 여전히 유효한 통찰은 있다.

- 현재 React/Rust/C# 분리 구조는 통신 병목과 책임 분산 문제를 만들었다.
- `capture accepted`와 실제 파일 도착을 같은 성공으로 취급하면 현장 품질이 무너진다.
- session destination 손실, sidecar restart, three-brain 상태 추론은 다시 만들면 안 되는 실패 패턴이다.

즉, 이 문서는 "현재 구조를 왜 중심 계획으로 두면 안 되는가"를 설명하는 postmortem 자료로는 여전히 가치가 있다.

## 2. Historical Options That Are Now Demoted

| 이전 대안 | 왜 현재 주 경로가 아닌가 |
| --- | --- |
| Photino.NET single-process 방향 | 셸 재작성 비용을 먼저 지불하면서도 RapidRAW Host/UI 자산 활용이 늦어진다 |
| HTTP sidecar modernization | 옛 sidecar 구조를 계속 중심에 두게 만들어 새 boundary 설계를 미룬다 |
| current-stack stabilization first | 기존 실패 구조를 더 오래 유지하게 만든다 |
| full native desktop rewrite | 웹 UI 강점과 기존 Host 흐름 자산을 불필요하게 버린다 |

이 대안들은 완전히 무의미한 것은 아니지만, 현재 문맥에서는 **historical alternatives**일 뿐이다.

## 3. Current Recommendation That Replaces This Document

현재 기준 권고안은 아래 한 줄이다.

> **`RapidRAW Host/UI selective reuse + new Canon-focused Camera Engine Boundary`**

이 권고안은 다음 판단을 포함한다.

- RapidRAW는 Host/UI selective reuse 대상으로 본다.
- digiCamControl은 Canon 흐름 추출 reference로 본다.
- 전체 digiCamControl 솔루션, 전체 `CameraDeviceManager`, 옛 sidecar 안정화는 주 경로가 아니다.

상세 설계와 근거는 모두 `refactoring/research-codex.md`를 따른다.

## 4. When to Read This File

이 파일은 아래 경우에만 읽는다.

- 왜 Photino-first, HTTP-sidecar-first, current-stack stabilization이 내려갔는지 확인할 때
- 현재 구조의 실패 패턴을 짧게 되짚을 때

이 파일을 읽고 바로 아키텍처를 결정하면 안 된다.
현재 설계 결정은 반드시 `refactoring/research-codex.md`를 기준으로 내려야 한다.
