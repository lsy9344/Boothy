<img src="https://r2cdn.perplexity.ai/pplx-full-logo-primary-dark%402x.png" style="height:64px;margin-right:32px"/>

# EDSDK 재접속 시 ‘좀비 Teardown 스레드’로 인한 교착 상태 해결 방안

**핵심 결론:**
현재 구조(EDSDK Teardown을 `spawn_blocking`으로 실행 + 30초 타임아웃 후 재접속 시도)에서는, 한 번 Teardown이 EDSDK 내부에서 영구 블로킹되면 같은 프로세스 안에서는 “정상 복구”가 사실상 불가능하다. `spawn_blocking` 작업은 시작된 이후에는 중단/abort할 수 없고, 내부에서 EDSDK가 락을 쥔 채 멈춰 있으면 이후 `sdk.initialize` 등 새 연결 시도는 모두 그 락에 막혀 영구 대기 상태로 빠진다.[^1][^2][^3]

따라서 **“타임아웃 후 재접속”을 허용하는 대신, 해당 상황을 ‘치명적 장애’로 간주하고, EDSDK를 프로세스 단위로 격리하거나 최소한 전용 스레드/상태 머신으로 관리하여** 아래와 같이 설계해야 한다.

***

## 1. 문제 요약 및 근본 원인

1. **Teardown 지연 및 타임아웃**
    - EDSDK 종료/세션 해제 코드가 `spawn_blocking` 안에서 실행되며, 어느 시점부터 `Teardown timed out after 30s` 로그와 함께 30초 동안 응답이 없는 현상이 발생.
    - 이는 EDSDK 내부가 카메라/USB/드라이버 상태 문제 등으로 인해 특정 API 호출(예: `EdsCloseSession`, `EdsTerminateSDK`, 라이브뷰 종료 등)에서 리턴하지 않고 **무기한 블로킹**된 상태로 추정.
2. **`spawn_blocking`의 특성**
    - Tokio의 `spawn_blocking` 작업은 **시작된 이후에는 abort 할 수 없다**. `JoinHandle::abort()`를 호출해도, 이미 실행 중인 blocking 클로저는 계속 돈다.[^2][^3][^1]
    - 런타임 종료 시에도, 이미 돌고 있는 `spawn_blocking` 작업은 끝나지 않으면 런타임는 그 작업이 끝날 때까지 기다리거나, `shutdown_timeout` 이후 “더 이상 기다리지 않고 리턴만 할 뿐, 작업 자체는 백그라운드에서 계속 돈다”.[^3][^1]
3. **좀비 Teardown 스레드 + 재접속 시도**
    - 30초 타임아웃 이후 컨트롤러는 “실패로 간주”하고 다음 단계(재연결)를 진행하지만, **실제 EDSDK teardown 스레드는 살아 있고, 라이브러리 내부 락/리소스를 쥔 채 백그라운드에서 블로킹** 중이다.
    - 이 상태에서 `Connect called` → `sdk.initialize()` (또는 `EdsInitializeSDK`, `EdsOpenSession` 등)을 호출하면,
        - 이전 스레드가 쥐고 있는 락이나 내부 상태 때문에 **새로운 초기화가 영구 대기** 상태로 빠짐.
        - 그 결과, **성공/실패 콜백도 못 쏘고 UI가 멈춘 것처럼 보이는 현상**이 발생.
4. **요약하면**
    - **원인 1:** EDSDK 호출이 특정 상황에서 영구 블로킹 될 수 있다.
    - **원인 2:** `spawn_blocking`으로 감싼 blocking 호출은 한 번 시작되면 외부에서 강제 종료할 수 없다.
    - **원인 3:** 그 상태에서 같은 프로세스 내에서 EDSDK를 다시 초기화/재접속하려 하면 라이브러리 내부 락 때문에 교착/영구 대기 상태에 빠진다.

***

## 2. 해결 설계의 방향성

해결 전략은 크게 세 축으로 나눌 수 있다.

1. **상태 머신/정책 변경:**
“Teardown 타임아웃 → 재접속”이라는 현재 정책을 버리고, 한번 EDSDK Teardown이 타임아웃되면 **해당 프로세스에서 더 이상 재접속을 시도하지 않고, UI에 명시적으로 ‘앱 재시작 필요’ 또는 ‘카메라 서비스 재시작 필요’를 표시**하도록 한다.
2. **EDSDK 호출 경계 재설계 (전용 스레드/메시지 루프):**
EDSDK는 COM/메시지 루프 기반의 특이한 동작을 요구하며, 한 스레드에서 일관되게 호출하는 것이 권장된다.[^4][^5][^6]
    - 모든 EDSDK 호출을 **단일 전용 스레드(또는 STA 스레드)** 에서만 실행하고, 나머지는 메시지 큐/채널로 명령을 전달하는 구조로 바꾼다.
    - 이 전용 스레드가 멈추더라도, **Tokio 런타임/메인 스레드/UI는 멈추지 않고 에러 상태를 표현할 수 있다.**
3. **최종적으로는 프로세스 격리:**
가장 강력하고 안정적인 해법은 EDSDK를 **별도의 헬퍼 프로세스**에서만 사용하고, 메인 앱은 IPC를 통해만 카메라를 제어하는 것이다.
    - 이 경우 EDSDK가 영구 블로킹되더라도 **헬퍼 프로세스를 kill \& restart** 하면 깨끗한 상태로 재시작할 수 있고, 메인 프로세스의 UI나 다른 기능은 그대로 유지된다.
    - Rust 커뮤니티에서도 “차단/중단할 수 없는 블로킹 작업은 쓰레드가 아니라 아예 서브프로세스로 분리하라”는 패턴이 자주 언급된다.[^7][^8]

***

## 3. 1단계: 정책(상태 머신) 차원의 최소 변경 해법

먼저, 현재 구조를 크게 바꾸지 않고도 적용할 수 있는 **정책 레벨의 해결책**을 정리한다.

### 3.1. 카메라/EDSDK 상태 머신 정의

카메라/SDK 상태를 명시적인 상태 머신으로 관리한다.

- `Disconnected`
- `Connecting`
- `Connected`
- `Disconnecting` (Teardown 진행중)
- `Failed(TeardownHang)` (치명적 오류 상태)

규칙:

1. **어떤 상태에서도 Teardown(Disconnecting)이 시작되면, 그 작업이 완료/실패로 귀결되기 전까지 새로운 Connect는 절대 허용하지 않는다.**
2. Teardown이 **30초 이상 걸리면**,
    - 내부 플래그 `state = Failed(TeardownHang)`로 전환한다.
    - 이 상태에서 들어오는 모든 카메라 관련 요청(Connect/TakePicture/LiveView 등)에 대해 **즉시 실패(에러 코드 반환)** 하며,
UI에는 “카메라 드라이버가 응답하지 않습니다. 앱(혹은 카메라 서비스)을 재시작해 주세요.” 를 표시한다.
3. `Failed(TeardownHang)` 상태에서는,
    - **동일 프로세스 내에서 EDSDK 재초기화 시도 자체를 막는다.**
    - 필요하다면 상위 레벨에서 “프로세스 재시작” 또는 “카메라 서비스 재시작”을 유도하는 UX를 제공한다.

이렇게 하면, **현재처럼 “실패로 간주하고 재접속 진입 → 영구 블로킹”이라는 최악의 패턴**은 제거된다.
대신 “한 번 망가지면 이 프로세스에서는 더 이상 카메라를 못 쓰고, 사용자에게 재시작을 요구한다”는 명확한 정책으로 바뀐다.

### 3.2. 구현 포인트

- Teardown 호출부:

```rust
async fn teardown_with_timeout(...) -> Result<(), TeardownError> {
    // 상태: Disconnecting 으로 전환
    state.store(State::Disconnecting, Ordering::SeqCst);

    let res = tokio::time::timeout(
        Duration::from_secs(30),
        tokio::task::spawn_blocking(move || {
            // 여기서 EDSDK::CloseSession, Terminate 등 호출
            run_edsdk_teardown_blocking()
        }),
    )
    .await;

    match res {
        Ok(join_res) => {
            // spawn_blocking 작업은 (성공하든 실패하든) 완료됨
            let inner = join_res.map_err(|_| TeardownError::JoinError)?;
            // inner: EDSDK 에러 코드 등을 담음
            state.store(State::Disconnected, Ordering::SeqCst);
            Ok(inner?)
        }
        Err(_elapsed) => {
            // timeout: spawn_blocking 작업은 여전히 백그라운드에서 돌고 있음
            state.store(State::FailedTeardownHang, Ordering::SeqCst);
            Err(TeardownError::TimeoutButStillRunning)
        }
    }
}
```

- Connect 호출부:

```rust
async fn connect(...) -> Result<(), ConnectError> {
    match state.load(Ordering::SeqCst) {
        State::FailedTeardownHang => {
            // 같은 프로세스에서는 더 이상 SDK를 만지지 않는다.
            return Err(ConnectError::FatalDriverStuck);
        }
        State::Disconnecting => {
            // Teardown 중: 재접속 불허
            return Err(ConnectError::TeardownInProgress);
        }
        _ => { /* 정상 플로우 진행 */ }
    }

    // 실제 Connect 로직 수행 (필요 시 Mutex 등으로 serialize)
}
```

이 단계만 적용해도, **UI가 ‘멈춘 것처럼’ 보이는 현상은 사라지고, 대신 명시적인 에러 상태/가이드가 제공**된다.

***

## 4. 2단계: EDSDK 호출을 전용 스레드/메시지 루프로 격리

정책 변화만으로는 “한 번 망가지면 프로세스 재시작”이 필요하다는 제약은 그대로다. 다만 **사용자 경험은 향상**된다.

다음 단계는 **EDSDK 호출을 전용 스레드에서만 실행**하도록 구조를 바꾸는 것이다. 이는 Canon EDSDK를 실제로 사용하는 라이브러리들(C\#, WinForms, .NET 래퍼 등)의 일반적인 패턴이기도 하다.[^6][^4]

### 4.1. 구조 개요

1. 앱 시작 시 **EDSDK 전용 스레드**를 하나 만든다. (`std::thread::spawn`)
2. 이 스레드는 내부에서:
    - `EdsInitializeSDK`
    - 카메라 디텍션, `EdsOpenSession`, 이벤트 핸들러 등록
    - 촬영, 다운로드, 라이브뷰 등
    - `EdsCloseSession`, `EdsTerminateSDK`
를 **모두 이 스레드에서만** 수행한다.
3. 메인(또는 Tokio) 스레드는 이 스레드와 **채널(mpsc) 기반 커맨드/이벤트 통신**만 한다.

예시 구조:

```rust
enum CameraCommand {
    Initialize,
    OpenSession,
    CloseSession,
    TakePicture { /* … */ },
    Shutdown,
}

enum CameraEvent {
    Initialized(Result<(), EDSDKError>),
    SessionOpened(Result<(), EDSDKError>),
    SessionClosed(Result<(), EDSDKError>),
    PictureTaken(Result<ImageData, EDSDKError>),
    FatalHangDetected,
}

fn edsdk_worker(cmd_rx: Receiver<CameraCommand>, evt_tx: Sender<CameraEvent>) {
    // 이 안에서만 EDSDK 사용
    loop {
        match cmd_rx.recv() {
            Ok(CameraCommand::Initialize) => { /* EDSDK initialize */ }
            Ok(CameraCommand::OpenSession) => { /* open session */ }
            Ok(CameraCommand::CloseSession) => {
                // 여기서 CloseSession/Terminate 호출
                // 만약 이 함수가 블로킹에 빠지면, 이 스레드만 죽는 것 (앱 전체는 살음)
            }
            Ok(CameraCommand::Shutdown) | Err(_) => break,
            // ...
        }
    }
}
```

Tokio 쪽에서는:

```rust
let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();
let (evt_tx, evt_rx) = std::sync::mpsc::channel();

// 전용 스레드 시작
std::thread::spawn(move || edsdk_worker(cmd_rx, evt_tx));

// async 코드에서는 cmd_tx로 명령을 보내고, evt_rx를 별도 task에서 풀링해서
// UI/상태 머신에 반영
```


### 4.2. 장점

- EDSDK 내부가 영구 블로킹되더라도, **해당 전용 스레드만 멈출 뿐**,
    - 메인 이벤트 루프, UI, 다른 비동기 작업은 살아 있다.
    - “카메라 기능은 더 이상 동작하지 않음”을 UI에 알려줄 수 있고, 로그를 남기고, 사용자에게 “카메라 서비스 재시작 버튼” 등을 제공할 수 있다.
- Canon EDSDK가 요구하는 COM/메시지 루프/STA 스레드 요건을 지키기 쉬워지고, 콜백/이벤트도 한 스레드에서만 처리하게 되어 **데드락 리스크 자체가 줄어든다**.[^5][^4][^6]


### 4.3. 한계

- 여전히 **같은 프로세스 안에서 EDSDK 라이브러리가 한 번 망가지면 완전한 “정상 복구”는 보장하기 어렵다.**
- 최악의 경우, 전용 스레드를 재시작하는 것조차 안 될 수 있다(라이브러리가 전역 상태/전역 락을 쥠).

따라서 **궁극적인 안정성을 원한다면 프로세스 격리를 고려해야 한다.**

***

## 5. 3단계: EDSDK를 별도 프로세스로 완전히 격리 (권장 최종 해법)

가장 견고한 아키텍처는 EDSDK를 아예 **별도 OS 프로세스**에서만 사용하도록 분리하는 것이다. 이 패턴은 Rust 커뮤니티에서도 “차단/중단 불가능한 블로킹 I/O, 버그가 있을 수 있는 C 라이브러리, 드라이버 의존적 코드”를 다룰 때 많이 추천된다.[^8][^7]

### 5.1. 구조 개요

1. **카메라 서비스(헬퍼) 프로세스**
    - 역할:
        - EDSDK 초기화 / 세션 관리
        - 촬영, 라이브뷰, 다운로드
        - 카메라 연결 이벤트 처리
    - 모든 EDSDK 함수는 이 프로세스 안에서만 호출.
    - 앞에서 설명한 “전용 스레드 + 상태 머신” 구조를 이 프로세스 내부에서 사용.
2. **메인 애플리케이션 프로세스**
    - UI, 비즈니스 로직, 기타 기능 담당.
    - 카메라 서비스와는 **IPC(예: TCP/Unix 소켓, named pipe, gRPC, protobuf, JSON-RPC 등)** 로 통신.
    - 카메라 관련 요청은 카메라 서비스에 RPC로 보내고, 응답/이벤트를 받아서 UI에 반영.
3. **헬퍼 프로세스 모니터링 및 자동 재시작**
    - 메인 프로세스는 헬퍼 프로세스의 “하트비트/헬스 체크”를 수행.
    - 일정 시간 응답이 없거나 프로세스가 죽으면:
        - 기존 헬퍼 프로세스를 kill (또는 이미 죽어있을 것)
        - 새 헬퍼 프로세스를 clean하게 띄운다.
    - 사용자에게는 “카메라 서비스 재시작 중…” 정도의 안내만 보여준다.

### 5.2. 장점

- **EDSDK가 영구 블로킹/데드락/메모리 오염을 일으켜도, kill 가능한 유일한 단위가 “프로세스”이므로 확실하게 회복 가능**하다.[^7][^8]
- 메인 앱은 헬퍼가 죽었다/재시작 중이라도, UI와 다른 기능은 계속 동작.
- 카메라 서비스만 별도로 크래시 리포트/코어 덤프/로그를 모을 수 있어, 장애 분석이 더욱 쉬워진다.


### 5.3. 구현 시 고려사항

- IPC 프로토콜 설계:
    - 간단히는 “명령/이벤트 구조체를 bincode/JSON으로 직렬화해서 송수신”.
    - 명령:
        - `Connect`, `Disconnect`, `TakePicture`, `StartLiveView`, `StopLiveView`, `Shutdown`, …
    - 이벤트:
        - `Connected`, `Disconnected`, `CaptureCompleted`, `ErrorOccurred`, `FatalHangDetected`, …
- 에러 모델 정의:
    - `EDSDK_ERROR` 이외에 `ProcessCrashed`, `Timeout`, `FatalHang` 등의 추상 에러 타입을 정의.
- 보안/권한:
    - 프로세스 간 통신 시 권한/보안 이슈 (로컬 전용 소켓, 인증 토큰 등)를 고려.

***

## 6. Rust FFI/리소스 관리 레벨에서의 보조적인 개선점

위의 큰 구조를 잡은 뒤, 개별 EDSDK 호출/자원 관리를 다음과 같이 정리하면 안정성이 더 올라간다.

### 6.1. 안전한 FFI 래퍼 + Drop 활용

- Canon EDSDK 카메라/세션/이미지 핸들을 **Rust 구조체로 감싸고**, `Drop` 트레이트에서 적절한 `EdsRelease` 호출 등 정리 로직을 넣어두면, 스코프에서 벗어날 때 자동으로 정리가 된다.[^9][^10][^11]
- 단, 이번 이슈처럼 **해제 함수(예: `EdsCloseSession`) 자체가 영구 블로킹될 수 있는 경우**,
    - 해당 부분은 `Drop`이 아닌 **명시적인 “종료 메서드 + 타임아웃 + 상태 머신”으로 관리해야 한다.**
    - 예: `fn close_session(&mut self, timeout: Duration) -> Result<(), Error>` 같은 안전한 고수준 API.


### 6.2. EDSDK 호출 직전/직후의 방어 코드

- 모든 EDSDK 호출 wrapper에서:
    - 입력 포인터/핸들이 null인지 검사.
    - EDSDK가 반환하는 에러 코드를 Rust `Result`로 변환.
    - 오류 시 적절한 로그/메트릭 기록.
- 촬영/다운로드 관련해서는 Canon 포럼/Stack Overflow에서 자주 언급되는 주의 사항들(예: `EdsDownloadComplete` / `EdsDownloadCancel` 반드시 호출, 다운로드 중 병렬 작업 금지, 콜백에서 락 조심 등)을 지키면 “SDK 내부가 애매한 상태로 남는” 빈도를 줄일 수 있다.[^12][^6]

***

## 7. 테스트 및 운영 전략

1. **강제 장애 주입 테스트**
    - 카메라를 촬영 중에 USB 케이블 뽑기 / 전원 끄기 / 저장 매체 제거 등의 상황을 재현.
    - Teardown이 타임아웃되는 상황을 인위적으로 많이 만들어,
        - 상태 머신이 `Failed(TeardownHang)`으로 전환되는지,
        - UI가 제대로 에러를 노출하는지,
        - 프로세스/서비스 재시작 경로가 정상 동작하는지를 검증.
2. **모니터링 지표**
    - EDSDK 호출별 latency, 타임아웃 횟수.
    - 카메라 서비스 프로세스의 크래시/재시작 횟수.
    - “카메라 사용 불가” 상태의 누적 시간/비율.
3. **배포 전략**
    - 1차: 기존 구조 유지 + 상태 머신/정책 변경(재접속 금지, 명확한 에러 처리).
    - 2차: EDSDK 전용 스레드 및 메시지 루프 도입.
    - 3차: 별도 헬퍼 프로세스 분리 및 자동 재시작 구현.

***

## 8. 정리

- **문제의 본질**은 “EDSDK의 Teardown가 영구 블로킹될 수 있고, Tokio `spawn_blocking` 작업은 외부에서 중단할 수 없다”는 점이다.[^1][^2][^3]
- 이 상태에서 “타임아웃 후 재접속”을 허용하면, 백그라운드에 남아있는 좀비 Teardown 스레드가 EDSDK 내부 리소스/락을 점유한 채, 새로운 연결 시도까지 모두 영구 대기에 빠뜨리게 된다.
- **실질적인 해결책**은:

1. **상태 머신을 도입하여, Teardown 타임아웃을 치명적 장애로 간주하고 동일 프로세스 내 재접속을 금지**하는 것.
2. **EDSDK 호출을 전용 스레드/메시지 루프로 격리**하여, SDK 블로킹이 앱 전체를 멈추지 않도록 하는 것.[^4][^6]
3. 궁극적으로는 **EDSDK를 별도 프로세스로 완전히 격리하고, 헬퍼 프로세스를 kill \& restart하는 구조**로 가는 것.[^8][^7]

이 세 가지를 순차적으로 도입하면, 현재 겪고 있는 “재접속 시 상태 포착 실패 + UI 멈춤” 문제를 구조적으로 제거하고, 실제 운영 환경에서 훨씬 견고한 카메라 제어 시스템을 만들 수 있다.
<span style="display:none">[^13][^14][^15][^16][^17][^18][^19][^20][^21][^22][^23][^24][^25][^26][^27][^28][^29][^30][^31][^32][^33][^34][^35][^36][^37][^38][^39][^40]</span>

<div align="center">⁂</div>

[^1]: https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html

[^2]: https://stackoverflow.com/questions/73528236/how-to-terminate-a-blocking-tokio-task

[^3]: https://cseweb.ucsd.edu/classes/sp22/cse223B-a/tribbler/actix_web/rt/task/fn.spawn_blocking.html

[^4]: https://stackoverflow.com/questions/35492576/running-edsdk-in-a-parallel-task-in-c-sharp

[^5]: https://stackoverflow.com/questions/18360402/no-callback-from-event-handler-canon-sdk-2-12

[^6]: https://github.com/thrixton/EDSDK.NET/blob/master/EDSDK.NET/SDKHandler.cs

[^7]: https://users.rust-lang.org/t/how-do-i-force-an-end-to-a-thread-that-is-blocking/65973

[^8]: https://stackoverflow.com/questions/2760652/how-to-kill-or-avoid-zombie-processes-with-subprocess-module

[^9]: https://akhil.sh/tutorials/rust/rust/integrating_rust_ffi_safe_abstractions/

[^10]: https://doc.rust-lang.org/std/ops/trait.Drop.html

[^11]: https://doc.rust-lang.org/book/ch15-03-drop.html

[^12]: https://stackoverflow.com/questions/47956621/edsdk-camera-seems-locked-with-message-recording-remaining-images

[^13]: https://www.papercut.com/kb/Main/EmbeddedSoftwareTimeouts/

[^14]: https://stackoverflow.com/questions/38431488/how-to-avoid-zombie-processes-when-running-a-command

[^15]: https://v8.dev/features/explicit-resource-management

[^16]: https://stackoverflow.com/questions/29035775/edsdk-error-if-open-camera-session-wait-some-seconds-and-take-a-picture

[^17]: https://users.rust-lang.org/t/how-do-i-force-an-end-to-a-thread-that-is-blocking/65973/3

[^18]: https://samuel-sorial.hashnode.dev/deadlock-prevention-and-necessary-conditions-to-occur

[^19]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Resource_management

[^20]: https://github.com/nunit/nunit/issues/2397

[^21]: https://github.com/Motion-Project/motion/issues/1769

[^22]: https://sourceforge.net/projects/canon-eos-digital-info/files/Portable/CanonEosDigitalInfo_v1.4_SDK_v2.14.zip/download

[^23]: https://docshield.tungstenautomation.com/atalasoftDotImage/en_US/11.5.0-8wax4k031j/print/AtalasoftDotImageDevelopersguide_EN.pdf

[^24]: https://ws-pace.tistory.com/30

[^25]: https://downloads.triprism.com/public/[ Steve ]/forTyler/canon 3.4sdk/EDSDK_API%203.4.pdf

[^26]: https://www.rustfinity.com/practice/rust/challenges/the-drop-trait?tab=solutions

[^27]: https://cybrancee.com/learn/knowledge-base/complete-guide-to-all-rust-debug-camera-commands/

[^28]: https://stackoverflow.com/questions/64646556/canon-edsdk-13-11-10-not-saving-to-host-pc

[^29]: https://www.youtube.com/watch?v=LJKpr09k5jE

[^30]: https://docs.rs/camera_controllers

[^31]: https://community.usa.canon.com/t5/Camera-Software/Canon-EDSDK-Close-session-command-hangs/td-p/273465

[^32]: https://github.com/esskar/Canon.Eos.Framework/blob/master/Canon.Eos.Framework/Internal/SDK/EDSDK.cs

[^33]: https://www.youtube.com/watch?v=2ut3Y-71Bog

[^34]: https://velog.io/@dreamcomestrue/OS-Deadlock-Handling-1-Deadlock-Prevention

[^35]: https://users.rust-lang.org/t/how-does-rust-release-resources-in-drop/109423

[^36]: https://lib.rs/rust-patterns

[^37]: https://www.geeksforgeeks.org/operating-systems/zombie-processes-prevention/

[^38]: https://stackoverflow.com/questions/71001701/tokio-spawn-blocking-when-passing-reference-requires-a-static-lifetime

[^39]: https://notes.suhaib.in/docs/tech/utilities/killing-zombie-processes-and-preventing-them/

[^40]: https://www.reddit.com/r/rust/comments/1p879tk/how_to_manage_async_shared_access_to_a_blocking/

