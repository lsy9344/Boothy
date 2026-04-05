# Local Renderer Sidecar Boundary

이 디렉터리는 truthful preview close canary용 local dedicated renderer 자산 전용 경계다.

- `canon-helper`와 책임을 섞지 않는다.
- host는 이 경계의 실행 산출물을 candidate로만 취급한다.
- canonical preview path 승격과 `previewReady` truth는 계속 host가 소유한다.
- 기본 booth route는 darktable이며, local renderer는 opt-in canary에서만 선택된다.
- repo에는 `local-renderer-sidecar.cmd` / `local-renderer-sidecar.ps1` bootstrap entrypoint를 포함한다.
- 기본 bootstrap은 현재 approved darktable binary를 sidecar boundary 뒤에서 호출해 candidate contract를 유지한다.
- host는 repo 경계, 실행 파일 인접 경계, 번들 resource 경계 순서로 sidecar 자산을 탐색해 active booth runtime에서도 canary를 찾을 수 있어야 한다.
- 전용 renderer executable이 있으면 host의 `BOOTHY_LOCAL_RENDERER_BIN` env로 이 bootstrap 대신 명시적으로 교체한다.
