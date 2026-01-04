# Intro Project Analysis and Context

## Existing Project Overview

- Boothy(working title: RapidTetherRAW)는 **RapidRAW(업스트림)** 를 기반으로 하는 **Windows 전용** 데스크톱 앱을 목표로 한다.
- RapidRAW 업스트림은 **Tauri 2** 기반 데스크톱 앱이며, 대략적인 구성은 다음과 같다.
  - Frontend: React + TypeScript + Vite + Tailwind
  - Backend: Rust (Tauri commands, RAW develop, GPU processing(wgpu/WGSL), AI features 등)
  - 비파괴 워크플로우: 이미지 옆에 sidecar(`*.rrdata`)로 보정값 저장
- 현재 작업공간에는 업스트림 코드가 `upstream/RapidRAW` 에 포함되어 있고, 상세한 구조/엔트리포인트/연동 포인트는 `docs/brownfield-architecture.md` 에 정리되어 있다.

## Available Documentation Analysis

- `docs/brownfield-architecture.md`: 업스트림(RapidRAW) 기반 **현재 코드 구조/연동 포인트/기술 부채/리스크** 정리(문서-프로젝트 분석 결과)
- `project-analysis.md`: Boothy(포크 컨텍스트) 관점에서의 핵심 핫스팟 요약
- `prd-rapidraw-customer-mode-v1.1.1.md`: Customer/Admin Mode + 세션/ExportLock/리셋 + EDSDK 테더링 요구사항(한국어 PRD)

## Understanding to Validate (Assumptions)

아래 항목은 현재 저장소의 문서/분석 결과를 기반으로 정리한 “현 상태 이해”이다. Requirements 작성 전에 사용자 확인이 필요하다.

1. (플랫폼) **Windows-only** 로 MVP/1.0을 개발한다.
2. (카메라) 지원 기종은 **Canon EOS 700D 단일 기종**이다.
3. (테더링) 테더링은 **Canon EDSDK를 앱에 동봉/재배포하지 않고**, 사용자가 로컬 설치 후 **경로 설정**으로 연결한다.
4. (캡처 저장) 촬영 결과물은 **active session folder에 직접 저장**되며, 앱은 이를 즉시 반영(리스트/썸네일 갱신)한다.
5. (Customer Mode 네비게이션) Customer Mode에서 파일/폴더 탐색은 **세션 폴더로 제한**된다.
6. (ExportLock) Export 중에는 촬영이 잠기며, Export 완료 전에는 **다음 세션 시작이 불가**하다.
7. (프라이버시 리셋) 리셋은 **앱 상태/캐시/백그라운드 작업 정리**에 한정되며, 세션 폴더의 이미지/사이드카 파일은 **삭제하지 않는다**.
8. (Export 파일명) Export 파일명 패턴은 `휴대폰뒤4자리-{hh}시-{sequence}` 이며, `{hh}`/`{sequence}`는 업스트림의 템플릿 로직을 활용하고 `휴대폰번호 뒤4자리`는 런타임에 주입한다.

Confirmed by user: 2026-01-02

## Enhancement Scope Definition

본 PRD는 “기존 코드(RapidRAW) 위에 상당한 규모의 동작/UX/백엔드 커맨드/새 모듈(테더링)을 추가”하는 **브라운필드 대형 개선**을 대상으로 한다. 현재 저장소 컨텍스트 기준으로 포함 범위는 다음과 같다.

- **Customer Mode(키오스크 플로우)**: 예약 확인 → 필터 선택 → 촬영 → 자동 Export/전송 → 자동 리셋의 단일 상태머신(Full-screen + 최소 조작)
- **Admin Mode(PIN)**: 운영/관리 기능(프리셋, Export 규칙, 장애 진단 등)
- **세션 타이머 + ExportLock 게이트**: 시간 종료 시 촬영 잠금 및 Export 강제 전환, Export 완료 전 다음 세션 시작 불가
- **프라이버시 리셋**: 앱 상태/캐시 초기화, 백그라운드 작업 정리(세션 폴더 파일/사이드카 삭제 금지)
- **Canon EOS 700D 테더링(EDSDK)**: Windows-only, 로컬 설치된 EDSDK를 경로 설정으로 사용(재배포/동봉 없음), 캡처 결과는 세션 폴더로 직접 저장

## Goals and Background Context

- 무인 셀프 스튜디오에서 고객이 “컴퓨터 조작”이 아니라 **단순 촬영/선택 경험**만 하도록 UX를 최소화한다.
- 운영자는 Admin Mode에서 카메라/프리셋/Export 정책을 관리하고, 고객 모드에서는 실수 가능성을 구조적으로 제거한다.
- 기존 RapidRAW의 비파괴 편집/프리셋/Export 파이프라인 장점을 유지하면서, 테더 촬영 및 세션 기반 운영을 추가한다.

## Change Log

| Date       | Version | Description | Author |
| ---------- | ------- | ----------- | ------ |
| 2026-01-02 | v0.1    | Initial draft from existing PRD + brownfield analysis | PM (John) |

---
