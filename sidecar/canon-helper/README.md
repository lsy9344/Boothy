# Canon Helper Sidecar

이 폴더는 Boothy의 Windows 전용 Canon EDSDK helper sidecar 경계입니다.

- 런타임 진실 source: `src-tauri/src/capture/sidecar_client.rs`
- 계약 기준: `docs/contracts/camera-helper-sidecar-protocol.md`
- 구현 프로파일: `docs/contracts/camera-helper-edsdk-profile.md`
- helper 프로젝트: `src/CanonHelper/CanonHelper.csproj`
- SDK vendor 입력물: `vendor/canon-edsdk/`

현재 앱은 helper가 남긴 최신 status snapshot을 host에서 읽어 booth/operator readiness truth로 정규화합니다.
실제 `canon-helper.exe`는 이 경계 아래에서 빌드되며, 현재 helper는 파일 기반 session diagnostics 경계를 통해 아래 파일을 읽고 씁니다.

- request input: `sessions/<sessionId>/diagnostics/camera-helper-requests.jsonl`
- status output: `sessions/<sessionId>/diagnostics/camera-helper-status.json`
- event output: `sessions/<sessionId>/diagnostics/camera-helper-events.jsonl`

주요 명령:

- version: `dotnet run --project sidecar/canon-helper/src/CanonHelper/CanonHelper.csproj -- --version`
- self-check: `dotnet run --project sidecar/canon-helper/src/CanonHelper/CanonHelper.csproj -- --self-check --sdk-root sidecar/canon-helper/vendor/canon-edsdk`
- runtime: `canon-helper.exe --runtime-root <booth-runtime> --session-id <sessionId>`

publish 결과물:

- `sidecar/canon-helper/src/CanonHelper/bin/Release/net8.0/win-x64/publish/canon-helper.exe`

현재 helper는 `--runtime-root`, `--session-id`를 받아 지정된 세션 아래에서 status/capture correlation을 수행합니다.
