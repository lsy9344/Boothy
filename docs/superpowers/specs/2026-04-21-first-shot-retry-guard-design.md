# First-Shot Retry Guard Design

## 목적

승인 하드웨어에서 첫 셔터가 `capture-trigger-failed(0x00000002)`로 실패한 뒤,
재연결 직후 첫 재시도가 너무 이르게 다시 들어가 같은 실패를 반복하는 경계를 줄인다.

이번 변경의 목표는 첫 컷 저장 성공 가능성을 높이는 것이지,
preview close latency나 startup/connect 전체 경계를 한 번에 해결하는 것이 아니다.

## 범위

포함:

- helper 내부의 `0x00000002` 이후 reconnect-first-retry 보호 모드 강화
- 그 보호 모드가 다음 한 번의 capture에만 적용되도록 경계 고정
- Rust host 회귀 테스트와 helper 단위 테스트 보강

제외:

- startup/connect stall 전반 수정
- preview truth ownership 변경
- darktable preview hot path 조정
- approved hardware rerun 자체 수행

## 현재 문제 해석

현재 구조는 이미 아래 두 보호를 가지고 있다.

- host가 internal trigger failure를 retryable로 해석하고 auto-retry를 건다
- helper가 reconnect 뒤 다음 한 번의 셔터를 `Completely_NonAF` 경로로 우회한다

하지만 현장 기록상 이것만으로는 부족했다.
문제는 reconnect가 끝난 직후 첫 셔터가 여전히 카메라 안정화 이전에 들어가
같은 `0x00000002`를 반복하는 점으로 읽는 편이 가장 일관된다.

## 설계 결정

### 1. reconnect-first-retry 보호 모드 명시화

helper는 `0x00000002`로 session reset이 필요한 경우,
단순히 "다음 한 번 NonAF"만 켜지 말고
"다음 첫 capture는 reconnect 보호 모드"라는 별도 의도를 유지한다.

이 보호 모드는 아래 둘을 함께 보장한다.

- reconnect 뒤 ready가 되더라도 즉시 일반 셔터 경로로 복귀하지 않는다
- 다음 첫 capture는 보수적 셔터 플랜으로만 실행한다

### 2. 첫 retry 셔터 플랜 강화

reconnect 보호 모드의 첫 capture는 아래 규칙을 사용한다.

- `Completely_NonAF`
- halfway prime 포함
- 기존보다 더 긴 warmup 대기 뒤 실행

즉, reconnect 직후 첫 retry는 "가장 빨리 다시 찍기"보다
"한 번에 저장까지 닫힐 가능성을 높이기"를 우선한다.

### 3. 보호 모드의 수명

보호 모드는 아래 둘 중 하나가 되면 해제한다.

- 첫 retry capture request가 실제로 실행된 직후
- reconnect가 다시 실패해 새 recovery cycle로 넘어갈 때

따라서 후속 촬영 전체를 느리게 만들지 않고,
문제가 되는 첫 재시도 한 번에만 비용을 지불한다.

### 4. host 경계

host는 이번 변경에서 retry 정책 자체를 크게 바꾸지 않는다.

- existing auto-retry count 유지
- existing ready stabilization wait 유지
- helper가 reconnect-first-retry 보호 모드를 제대로 소화하는지 검증하는 쪽에 집중

필요하면 후속 작업에서 host stabilization budget을 다시 조정한다.

## 테스트

- Rust 회귀: internal trigger failure 이후 auto-retry가 여전히 저장까지 닫히는지
- Rust 회귀: stale ready를 fresh reconnect ready로 오판하지 않는지
- helper 단위 테스트: `0x00000002` 뒤 보호 모드가 설정되는지
- helper 단위 테스트: 보호 모드가 첫 retry 한 번에만 적용되는지

## 성공 기준

- 첫 셔터 `0x00000002` 이후 다음 retry capture가 더 보수적인 경로로 실행된다
- 기존 auto-retry 회귀를 깨지 않는다
- 보호 모드가 후속 정상 촬영까지 남아 있지 않는다
- 이번 변경 이후 승인 하드웨어 rerun에서 capture 1이 저장까지 닫히는지 다시 확인할 준비가 된다
