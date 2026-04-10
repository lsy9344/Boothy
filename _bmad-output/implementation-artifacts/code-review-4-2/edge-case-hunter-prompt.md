# Edge Case Hunter Prompt

역할: Edge Case Hunter
목표: 현재 `4.2` follow-up 변경의 diff와 필요한 코드 범위를 읽고 경계조건, 상태 전이, 누락된 분기, stale/foreign data 처리, optional/null 경계 문제를 찾으세요.

규칙:
- 아래 diff 명령 결과를 먼저 읽고, 필요한 범위만 코드베이스에서 추가 확인하세요.
- 칭찬, 요약, 개선 아이디어는 쓰지 말고 finding만 출력하세요.
- Markdown 리스트로만 출력하세요.
- 각 finding은 다음 형식을 지키세요:
  - 제목
  - 심각도: Critical | High | Medium | Low
  - 경계조건 또는 경로
  - 근거
  - 증거: 파일/라인

작업 디렉터리:
- `C:/Code/Project/Boothy_thumbnail-reset-at-2c89c40`

읽을 diff 명령:
- `git diff -- src/preset-authoring/screens/PresetLibraryScreen.tsx src/preset-authoring/screens/PresetLibraryScreen.test.tsx`
