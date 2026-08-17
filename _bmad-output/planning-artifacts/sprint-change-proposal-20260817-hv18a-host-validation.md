# Sprint 변경 제안 — HV-18A 현재 PC 기반 내부 검증

- 승인자: Noah Lee
- 승인 시각: 2026-08-17 13:22:41 +09:00
- 적용 범위: Story 7.7 / HV-18A
- 상태: 승인됨

## 변경 요약

Story 7.7의 MVP 내부 검증을 clean offline Windows VM이 아닌 현재 승인된 시험 PC에서 수행한다.
다음 항목은 책임자 결정으로 검증 대상에서 제외한다.

1. Boothy 코드 서명 인증서와 설치본 서명 증거
2. Canon EDSDK 재배포 권리 증거
3. 개발 도구가 없는 clean Windows 및 네트워크 차단 증거

면제 항목은 통과로 기록하지 않고 `waived-by-owner`로 기록한다. 따라서 HV-18A 결과는
`Go-candidate-with-waivers`로 구분하며, 이 결과만으로 공개 배포의 보안 신뢰, 법적 재배포 권리,
새 PC 독립 설치 가능성을 주장하지 않는다.

## 유지되는 필수 검증

- 설치본과 봉인 인벤토리의 해시 일치
- 번들 darktable 및 self-contained camera helper의 self-check
- 설치, 실행, fixture 표시, 업그레이드, 롤백, 제거
- 업그레이드·롤백 뒤 기존 세션 읽기 및 제거 뒤 고객 사진 보존
- 실제 Canon 카메라 촬영과 raw-original, display-proxy, raw-refined, final 산출물
- 누락되거나 다른 인벤토리에 대한 명시적 실패
- Story 7.6 / HV-17 Go 선행 조건

## 영향

- PRD NFR-006과 MVP release gate를 내부 시험 기준으로 수정한다.
- Epic 7 및 Story 7.7의 clean offline·서명 필수 문구를 승인 면제 모델로 수정한다.
- Architecture의 배포 검증 순서를 현재 PC lifecycle 검증으로 수정한다.
- UX 화면과 고객 문구에는 변화가 없다.
- HV-18A 게이트는 C7(서명), C8(오프라인), C9(clean 환경)를 승인 파일이 있을 때만 면제한다.
- Canon EDSDK 재배포 증거는 별도 기계식 체크가 아니므로 승인 파일과 결과 문서에 면제로 남긴다.

## 완료 조건

현재 PC에서 두 설치 버전으로 전체 lifecycle을 실행하고 증거를 HV-18A 회차에 저장한다.
면제되지 않은 조건 중 하나라도 실패하면 HV-18A는 계속 `No-Go`다.

## 승인 근거

2026-08-17 사용자 지시: “2,3번은 중요하지않아 건너뛰어. 문서에 명시해. 인증서도 필요없으니
건너뛰세요. 환경에서 전체 시험은 이 PC에서 실험하세요.”
