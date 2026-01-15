# 세션 생성 시 UI 깜빡임/클릭 불가 + `[TAURI] Couldn't find callback id ...`

## 요약
- **증상**: 프로그램 실행 후 세션을 생성/선택하면 화면이 깜빡이고 UI가 클릭 불가 상태가 되며, DevTools 콘솔에 Tauri 콜백 경고가 반복 출력됨.
- **주요 로그**:
  - ` [TAURI] Couldn't find callback id XXXXX. This might happen when the app is reloaded while Rust is running an asynchronous operation.`
  - (일부 환경) Sidecar: `Pipe server error | Exception: 모든 파이프 인스턴스가 사용 중입니다.`
- **근본 원인**: 세션 전환 시 **프론트엔드에서 동일 세션 적용/폴더 로딩이 중복 실행**되어, 동시에 여러 `invoke`가 경쟁(race)하며 상태를 서로 덮어쓰기 → UI 상태가 반복 초기화되어 깜빡임/입력 불가로 이어짐. 이 과정에서 Tauri가 이미 정리된 콜백을 다시 받는 형태로 경고가 발생.
- **해결**: 세션 적용을 **중복 방지**하고, 폴더 로딩은 **최신 요청만 유효**하도록 가드(요청 ID)하여 경쟁 상태를 제거.

## 환경/관련 컴포넌트
- **Frontend**: `apps/boothy/src/App.tsx` (React + Tauri invoke)
- **Backend**: `apps/boothy/src-tauri/src/main.rs`
- **Sidecar(Windows)**: `apps/camera-sidecar` (Named Pipe IPC)

## 재현 절차(관찰된 케이스)
1. 앱 실행
2. 세션 이름 입력 → 세션 생성/오픈
3. 세션의 Raw 폴더에 RAW 파일(예: `.cr2`)을 추가하거나(드래그/복사), 세션 전환 직후 라이브러리 갱신이 일어나는 상황을 유발
4. UI가 깜빡이거나 클릭 불가 상태가 되면서 콘솔에 Tauri 콜백 경고가 반복됨

## 증상 상세
- 라이브러리/트리/미리보기 영역이 빠르게 다시 그려지거나 로딩 상태가 반복 토글
- 클릭/단축키가 간헐적으로 먹지 않거나, 특정 순간부터 UI 입력이 사실상 불가능해짐
- DevTools 콘솔에 `Couldn't find callback id ...` 반복

## 로그/신호 해석
### 1) `[TAURI] Couldn't find callback id ...`
일반적으로 “페이지 리로드” 상황에서 자주 보이지만, 이 케이스에서는 실제 리로드가 아니라도 **동일 시점에 겹쳐 수행되는 invoke/async 작업들이 취소/중단/교체**되면서 프레임워크 레벨에서 콜백 매칭이 깨질 때도 발생할 수 있음.

### 2) `Pipe server error | 모든 파이프 인스턴스가 사용 중입니다.`
별도 축의 문제로, **sidecar가 종료되지 않고 남거나 중복 실행**되면 Named Pipe 서버 인스턴스가 점유되어 신규 서버 생성이 실패할 수 있음. 이 현상은 세션 전환 문제를 악화시키는 트리거가 될 수 있으나, 이번 “깜빡임/클릭 불가”의 핵심 원인은 프론트엔드 경쟁 상태였음.

## 근본 원인(RCA)
### 원인 A: 세션 적용의 중복 호출 (프론트엔드)
세션 생성/오픈 시 프론트엔드에서 세션 적용이 **두 경로로 동시에 발생**할 수 있었음:
- `handleStartSession()`에서 `invoke(boothy_create_or_open_session)` 결과를 받자마자 `applyBoothySession(session)` 호출
- 동시에 백엔드가 `boothy-session-changed` 이벤트를 emit → 프론트 리스너가 다시 `applyBoothySession(session)` 호출

그 결과 동일 Raw 경로에 대해 `handleSelectSubfolder(rawPath, true)`가 **중복/동시 실행**됨.

### 원인 B: 폴더 로딩(invoke) 경쟁과 stale 응답의 상태 덮어쓰기
`handleSelectSubfolder`는 내부에서 다음을 수행:
- `cancel_thumbnail_generation` 호출
- `GetFolderTree`, `ListImagesInDir/ListImagesRecursive`, `ReadExifForPaths` 등 여러 `invoke` 실행
- 로딩 UI 상태/선택 상태/썸네일 상태 등을 초기화

중복 호출된 여러 async 작업이 **서로 다른 시점에 완료**되면서,
- 늦게 끝난 “이전 요청”이 “새 요청”의 상태를 다시 덮어쓰는 stale update가 발생
- 그 과정에서 로딩/초기화가 반복되어 UI가 계속 리셋되는 형태(깜빡임)로 보였음
- Tauri 콜백 경고는 이런 경쟁 상황에서 콜백/핸들러 수명과 매칭이 꼬이는 신호로 관찰됨

## 해결 내용(요약)
### 1) 세션 적용 중복 방지
- 동일 세션(세션 폴더명/Raw 경로)을 이미 적용한 경우, `applyBoothySession`가 재진입해도 `handleSelectSubfolder`를 호출하지 않도록 차단.

### 2) 폴더 로딩 “최신 요청만 유효” 가드
- `handleSelectSubfolder` 시작 시 요청 ID를 증가시키고,
- 각 await 이후/then 핸들러에서 “내 요청이 최신인지” 확인하여 **stale 응답이 UI 상태를 덮어쓰지 못하게** 함.

> 적용 위치: `apps/boothy/src/App.tsx`

### 3) (부가) sidecar/로그 개선
동일 현상이 재발할 때 원인 분리를 쉽게 하기 위해 다음도 함께 정리됨:
- sidecar 중복 실행/종료 누락을 줄이기 위한 lifecycle 정리(기존 pipe 연결 시 재사용, 종료 시 shutdown 신호 등)
- sidecar 로그 파일 기록 및 tauri stdout 로그에 target 포함

## 검증 체크리스트(재발 방지용)
1. 세션 생성/전환을 연속으로 빠르게 수행해도 UI가 깜빡이지 않음
2. 세션 생성 직후 Raw 폴더에 `.cr2` 추가 시 라이브러리 갱신은 되지만 UI가 입력 불가 상태로 빠지지 않음
3. DevTools 콘솔에서 `Couldn't find callback id ...`가 반복되지 않거나, 최소화됨
4. (Windows) sidecar가 하나만 실행 중이며, `모든 파이프 인스턴스가 사용 중입니다.`가 재발하지 않음

## 다음에 같은 문제가 발생하면 (트러블슈팅 플레이북)
### 1단계: 증상 분류
- **UI만** 깜빡이고 세션 전환/폴더 로딩 시점에 발생 → 프론트 invoke 경쟁 의심
- **sidecar pipe 에러** 동반(특히 “모든 파이프 인스턴스…”) → sidecar 중복/잔존 프로세스 의심

### 2단계: 확인할 로그 위치
- Tauri 로그: `%APPDATA%\\Boothy\\logs\\boothy-YYYYMMDD.log`
- Sidecar 로그: `%APPDATA%\\Boothy\\logs\\boothy-sidecar-YYYYMMDD.log`
- DevTools 콘솔: callback id 경고/세션 이벤트 처리 타이밍 확인

### 3단계: 빠른 조치
- (Windows) 작업 관리자에서 `Boothy.CameraSidecar.exe`가 다중 실행인지 확인 후 정리
- 세션 생성 직후 이벤트/호출 흐름 확인:
  - `boothy-session-changed` 이벤트가 중복으로 들어오는지
  - `handleStartSession()`에서 `applyBoothySession()`를 별도로 호출하는지
  - `handleSelectSubfolder()`가 동시에 여러 번 호출되는지(로그로 확인 권장)

### 4단계: 재발 시 추가로 넣을 관측 포인트(권장)
- 프론트에서 `boothy_log_frontend`를 사용해 아래를 로그로 남기면 재현 시점 원인 추적이 빨라짐:
  - 세션 키(session_folder_name/raw_path), selectSubfolder requestId, invoke 시작/종료
  - `boothy-session-changed` 수신 횟수/순서

## 관련 변경 파일(참고)
- `apps/boothy/src/App.tsx` (중복 세션 적용 방지 + 최신 요청 가드)
- `apps/boothy/src-tauri/src/camera/ipc_client.rs` (sidecar lifecycle 강화)
- `apps/boothy/src-tauri/src/main.rs` (앱 종료 시 sidecar 정리)
- `apps/boothy/src-tauri/src/logging.rs` (stdout 로그에 target 포함)
- `apps/camera-sidecar/Program.cs`, `apps/camera-sidecar/Logging/Logger.cs` (sidecar shutdown/log 파일)

