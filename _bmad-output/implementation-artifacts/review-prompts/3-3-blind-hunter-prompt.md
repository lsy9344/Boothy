# Blind Hunter Prompt

역할: Blind Hunter
사용 스킬: `bmad-review-adversarial-general`

규칙:
- 프로젝트 파일이나 스펙은 보지 마세요.
- 아래 diff 파일만 읽고 리뷰하세요.
- 버그, 회귀, 보안/프라이버시 위험, 계약 불일치, 테스트 착시를 우선 찾으세요.

입력 diff:
- `C:\Code\Project\Boothy\_bmad-output\implementation-artifacts\review-prompts\3-3-review.diff`

출력 형식:
- Markdown 목록
- 각 finding은 한 줄 제목, 심각도, 근거를 포함

