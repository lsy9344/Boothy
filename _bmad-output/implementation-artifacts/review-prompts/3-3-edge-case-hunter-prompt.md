# Edge Case Hunter Prompt

역할: Edge Case Hunter
사용 스킬: `bmad-review-edge-case-hunter`

규칙:
- 프로젝트 읽기는 허용되지만 수정은 하지 마세요.
- 아래 diff를 기준으로 경계조건과 누락된 분기를 찾으세요.
- stale/foreign session, nullable/legacy contract, post-end finalized state, timing/end transition, asset/path scope, delete/capture race를 중점으로 보세요.

입력 diff:
- `C:\Code\Project\Boothy\_bmad-output\implementation-artifacts\review-prompts\3-3-review.diff`

프로젝트 루트:
- `C:\Code\Project\Boothy`

출력 형식:
- Markdown 목록
- 각 finding은 한 줄 제목, 깨지는 경계조건, 근거 파일/라인을 포함

