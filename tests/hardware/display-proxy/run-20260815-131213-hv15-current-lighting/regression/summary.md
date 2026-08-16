# 검증 요약

- 실장비 capture/display: 5/5 성공
- Daylight 연속 촬영: 3/3 성공
- preset switch: 통과
- delete → viewer standby: 통과
- delete 후 재촬영 복구: 통과
- final HV-15 mechanical gate: 통과
- final telemetry completeness: committed 1 / terminal 1 / qualifying 1, 통과
- 카메라 timeout/busy: 0

실장비 회차 자체는 코드 변경 없이 수행했다. 2026-08-16 HV-15 `Go` 뒤 proxy 기본값을 `on`으로 전환하면서 아래 검증을 다시 실행했다.

- `cargo fmt --check`: 통과
- Rust lib: 151/151 통과
- Rust `viewer_display`: 35/35 통과
- HV-15 기계식 gate: 통과
- 계측 완결성: committed 1 / terminal 1 / qualifying 1, 통과
- `pnpm lint`: 통과
- `pnpm test:run`: 903 통과 / 2 실패. 두 실패는 기존 Story 1.4 거버넌스 문구 불일치의 root/복제 worktree 동일 실패이며 이번 변경과 무관하다.
- `pnpm build`: 기존 TypeScript 기준선 오류로 실패. proxy 기본값 변경 파일에서 새 TypeScript 오류는 없다.
