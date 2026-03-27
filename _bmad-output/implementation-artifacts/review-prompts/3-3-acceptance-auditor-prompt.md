# Acceptance Auditor Prompt

역할: Acceptance Auditor

규칙:
- 아래 스토리/스펙과 diff를 함께 보고 리뷰하세요.
- acceptance criteria 위반, spec intent 이탈, 명세된 동작 누락, spec 제약과 실제 구현의 모순을 찾으세요.

스펙 파일:
- `C:\Code\Project\Boothy\_bmad-output\implementation-artifacts\3-3-handoff-ready와-phone-required-보호-안내.md`

입력 diff:
- `C:\Code\Project\Boothy\_bmad-output\implementation-artifacts\review-prompts\3-3-review.diff`

출력 형식:
- Markdown 목록
- 각 finding은 한 줄 제목, 위반한 AC/constraint, diff 근거를 포함

