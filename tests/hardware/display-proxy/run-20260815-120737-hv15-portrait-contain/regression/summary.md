# 회귀검사 요약

- `pnpm lint`: 통과
- Story 7.4 관련 Vitest: 15 files / 270 tests 통과
- `cargo fmt --check`: 통과
- Rust lib tests: 151/151 통과
- Rust `viewer_display`: 35/35 통과
- HV-15 checker self-test: 정상 2개 통과, seeded defect 11개 모두 차단
- HV-15 final mechanical gate: 통과
- telemetry completeness gate: committed 1 / terminal 1 / qualifying 1, 통과
- 전체 `pnpm test:run`: 903 통과 / 2 실패
- `pnpm build`: 실패, 20 TypeScript 오류

전체 테스트의 두 실패는 같은 governance assertion이 본 작업 트리와 기존 `.claude/worktrees/story-7-3-hv14-fixes`에서 각각 한 번 발생한 것이다. Story 1.4 문서의 기존 hardware gate 문구 불일치이며 이번 화면 적합/삭제 변경과 무관하다.

빌드 오류는 ReadinessScreen, 기존 capture-runtime test, governance Node types, operator diagnostics, active-preset, 두 기존 TS2719 fixture, after-paint test에 분포한다. 실장비 개선 대상의 제품 실행 경로에서는 신규 오류가 확인되지 않았지만, 저장소 전체 빌드 green을 주장하지 않는다.
