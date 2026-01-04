# Overview

Boothy(working title: RapidTetherRAW)는 RapidRAW(업스트림) 기반의 Windows 전용 테더 촬영 + RAW 현상 + Export 데스크톱 앱이다. 본 문서는 “Customer Mode(무인 키오스크 플로우) + Admin Mode(PIN) + EOS 700D EDSDK 테더링 + Smart Export Pipeline + ExportLock 게이트 + 프라이버시 리셋”을 기존 RapidRAW 구조에 안전하게 통합하기 위한 아키텍처 가이드를 제공한다.

프로젝트 컨텍스트:

- 업스트림 코드: `upstream/RapidRAW` (commit `a931728`) — 구현 시 실제 파일/엔트리포인트는 업스트림 기준으로 검증한다.
- 운영 환경: Windows 10/11 x64
- 개발 환경: macOS에서 개발 가능하나 EDSDK/테더 기능은 Windows 실검증이 필요하다.
- 컴플라이언스: RapidRAW 기반 **AGPL-3.0** 준수, Canon EDSDK는 **비동봉/비재배포**(사용자 로컬 설치 + 경로 설정)

---
