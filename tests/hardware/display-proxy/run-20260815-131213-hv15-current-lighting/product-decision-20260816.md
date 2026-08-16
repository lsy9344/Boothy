# 현재 밝기 제품 예외 결정

- 결정 일시: 2026-08-16 01:05 +09:00
- 결정자: Noah Lee
- 선택: 현재 조명과 카메라 노출 상태를 그대로 허용
- 적용 범위: Story 7.4 / HV-15의 2026-08-15 실장비 촬영 결과

## 보존되는 측정 사실

- Daylight 평균 밝기: 12.27/255
- 최종 Mono Pop 평균 밝기: 7.39/255
- 최종 Mono Pop에서 밝기 16 미만 픽셀: 85.4%

이 결정은 위 결과가 일반 품질 기준을 통과했다는 뜻이 아니다. 현재의 매우 어두운 결과를 제품 책임자가 제한적으로 허용한 예외다.

## 포함되지 않는 결정

- 확인되지 않은 카메라 펌웨어·렌즈·카드·케이블, ICC, HDR 정보 면제
- 측정하지 않은 화질 수치나 사람 비교 검토를 수행한 것으로 간주
- Story 7.4 또는 HV-15의 자동 `Go`

## 2026-08-16 추가 결정

이후 Noah Lee는 확인 가능한 장비 정보를 기존 증거와 Windows 조회로 보완하고, 읽을 수 없는 세부값 및 별도 metrics/blind review를 Story 7.4 범위의 제한 예외로 기록한 뒤 다음 Story로 진행하도록 승인했다. 세부 내용은 `environment-completion-20260816.md`에 있다. Story 7.5의 상주 렌더러 화질 비교는 면제되지 않는다.

근거 문서: `_bmad-output/planning-artifacts/sprint-change-proposal-20260816-010559-current-brightness-exception.md`
