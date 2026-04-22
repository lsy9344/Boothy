using System.Collections.Concurrent;
using System.Runtime.InteropServices;
using System.Threading.Tasks;
using CanonHelper.Protocol;
using EDSDKLib;

namespace CanonHelper.Runtime;

internal sealed class CanonSdkCamera : IDisposable
{
    private readonly record struct CaptureShutterPlan(
        EDSDK.EdsShutterButton ReleaseCommand,
        bool PrimeWithHalfway,
        TimeSpan DelayBeforeRelease
    );

    private static readonly TimeSpan MinimumSdkRecycleInterval = TimeSpan.FromSeconds(2);
    private static readonly TimeSpan InternalTriggerReconnectReadyWarmup = TimeSpan.FromSeconds(5);
    private static readonly TimeSpan InternalTriggerRetryHalfPressLead = TimeSpan.FromMilliseconds(150);
    private static readonly TimeSpan KeepAliveInterval = TimeSpan.FromMilliseconds(1500);
    private const uint DefaultPreviewJpegQuality = 8;
    // Live camera-thumbnail extraction can stall the RAW handoff on EOS 700D
    // hardware. Prefer the post-download raw fallback so capture correctness
    // wins over speculative first-visible speed.
    private const bool EnableImmediateCameraThumbnailFastPreview = false;
    private const string ConnectionAttemptTimeoutEnvVar = "BOOTHY_HELPER_CONNECT_TIMEOUT_MS";
    private static readonly string[] DisplayablePreviewExtensions =
    [
        ".jpg",
        ".jpeg",
        ".png",
        ".webp",
        ".gif",
        ".bmp",
    ];
    private const string CaptureCompletionTimeoutOverrideFileName =
        ".camera-helper-capture-timeout-ms";
    // Real follow-up captures on EOS 700D hardware can occasionally cross the
    // 30 second mark before the transfer boundary closes. Keep enough headroom
    // to avoid treating slow but valid RAW handoffs as fatal helper failures.
    private static readonly TimeSpan DefaultCaptureCompletionTimeout = TimeSpan.FromMilliseconds(
        45000
    );
    private static readonly TimeSpan DefaultConnectionAttemptTimeout = TimeSpan.FromSeconds(15);

    private readonly object _sync = new();
    private readonly BlockingCollection<Action> _sdkThreadQueue = new();
    private readonly GCHandle _selfHandle;
    private readonly EDSDK.EdsObjectEventHandler _objectHandler;
    private readonly EDSDK.EdsPropertyEventHandler _propertyHandler;
    private readonly EDSDK.EdsStateEventHandler _stateHandler;
    private readonly Thread _sdkThread;

    private IntPtr _camera = IntPtr.Zero;
    private bool _sdkInitialized;
    private bool _sessionOpen;
    private CameraSnapshot _snapshot =
        new("connecting", "starting", "helper-starting", null, null);
    private CurrentCaptureContext? _currentCapture;
    private Task? _connectTask;
    private DateTimeOffset _connectAttemptStartedAt = DateTimeOffset.MinValue;
    private Action<CurrentCaptureContext, IntPtr>? _downloadCaptureOverride;
    private Func<bool>? _connectAttemptOverride;
    private Func<uint>? _pumpEventsOverride;
    private Func<IntPtr, uint, int, uint>? _sendCommandOverride;
    private readonly Queue<PendingFastPreviewDownload> _pendingFastPreviewDownloads = new();
    private DateTimeOffset _lastKeepAlive = DateTimeOffset.MinValue;
    private DateTimeOffset _lastSdkRecycleAt = DateTimeOffset.MinValue;
    private DateTimeOffset _delayedReadyNotBeforeAt = DateTimeOffset.MinValue;
    private DateTimeOffset _internalTriggerRetryGuardNotBeforeAt = DateTimeOffset.MinValue;
    private bool _useProtectedRetryShutterPlanOnNextCapture;
    private int _sdkThreadId;

    public CanonSdkCamera()
    {
        _selfHandle = GCHandle.Alloc(this);
        _objectHandler = HandleObjectEvent;
        _propertyHandler = HandlePropertyEvent;
        _stateHandler = HandleStateEvent;
        _sdkThread = new Thread(SdkThreadLoop)
        {
            IsBackground = true,
            Name = "canon-helper-sdk-sta",
        };
        _sdkThread.SetApartmentState(ApartmentState.STA);
        _sdkThread.Start();
    }

    public CameraSnapshot Snapshot
    {
        get
        {
            lock (_sync)
            {
                RefreshDelayedReadyLocked(DateTimeOffset.UtcNow);
                return _snapshot;
            }
        }
    }

    public bool IsReady
    {
        get
        {
            lock (_sync)
            {
                RefreshDelayedReadyLocked(DateTimeOffset.UtcNow);
                return _sessionOpen && _snapshot.CameraState == "ready";
            }
        }
    }

    public bool ForceCaptureTimeoutIfStuck(string runtimeRoot, DateTimeOffset now)
    {
        CurrentCaptureContext? activeCapture;
        lock (_sync)
        {
            activeCapture = _currentCapture;
        }

        if (
            activeCapture is null
            || activeCapture.Completion.Task.IsCompleted
            || now - activeCapture.StartedAt < ResolveCaptureCompletionTimeout(runtimeRoot)
        )
        {
            return false;
        }

        FailActiveCapture(
            new CanonCaptureException(
                "capture-download-timeout",
                "RAW handoff를 기다리다 시간이 초과되었어요.",
                recoveryRequired: true
            )
        );
        return true;
    }

    internal void SetDownloadCaptureOverrideForTests(
        Action<CurrentCaptureContext, IntPtr>? downloadCaptureOverride
    )
    {
        _downloadCaptureOverride = downloadCaptureOverride;
    }

    internal void SetConnectAttemptOverrideForTests(Func<bool>? connectAttemptOverride)
    {
        _connectAttemptOverride = connectAttemptOverride;
    }

    internal void SetPumpEventsOverrideForTests(Func<uint>? pumpEventsOverride)
    {
        _pumpEventsOverride = pumpEventsOverride;
    }

    internal void SetSendCommandOverrideForTests(Func<IntPtr, uint, int, uint>? sendCommandOverride)
    {
        _sendCommandOverride = sendCommandOverride;
    }

    public void PumpEvents()
    {
        bool sdkInitialized;
        bool sessionOpen;
        lock (_sync)
        {
            sdkInitialized = _sdkInitialized;
            sessionOpen = _sessionOpen;
        }

        // Event pumping is only valid after the camera session is fully open.
        // Calling into EDSDK while the connect/open path is still running can
        // race the session-open work and stall startup on booth hardware.
        if (!sdkInitialized || !sessionOpen)
        {
            return;
        }

        RunOnSdkStaThreadAsync(() =>
            {
                uint result;
                try
                {
                    result = _pumpEventsOverride?.Invoke() ?? CanonSdkNative.EdsGetEvent();
                }
                catch (DllNotFoundException)
                {
                    UpdateFailure("error", "error", "sdk-payload-missing");
                    return 0;
                }
                catch (Exception)
                {
                    HandleConnectionLost("event-pump-failed", "recovering");
                    return 0;
                }

                if (result == EDSDK.EDS_ERR_OK)
                {
                    return 0;
                }

                switch (result)
                {
                    case EDSDK.EDS_ERR_COMM_DISCONNECTED:
                    case EDSDK.EDS_ERR_DEVICE_NOT_FOUND:
                    case EDSDK.EDS_ERR_DEVICE_INVALID:
                    case EDSDK.EDS_ERR_SESSION_NOT_OPEN:
                        HandleConnectionLost("usb-disconnected", "recovering");
                        return 0;
                    default:
                        HandleConnectionLost("event-pump-failed", "recovering");
                        return 0;
                }
            })
            .GetAwaiter()
            .GetResult();
    }

    public async Task EnsureConnectedAsync(CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();

        Task? connectTask;
        DateTimeOffset connectAttemptStartedAt;
        bool sessionOpen;

        lock (_sync)
        {
            sessionOpen = _sessionOpen;

            if (sessionOpen)
            {
                _connectTask = null;
                _connectAttemptStartedAt = DateTimeOffset.MinValue;
                return;
            }

            if (_connectTask is null)
            {
                _connectAttemptStartedAt = DateTimeOffset.UtcNow;
                _connectTask = StartConnectTask();
            }

            connectTask = _connectTask;
            connectAttemptStartedAt = _connectAttemptStartedAt;
        }

        if (sessionOpen)
        {
            KeepCameraAwakeIfNeeded();
            return;
        }

        if (connectTask is null)
        {
            return;
        }

        if (connectTask.IsCompleted)
        {
            await ObserveCompletedConnectAttemptAsync(connectTask);
            return;
        }

        if (DateTimeOffset.UtcNow - connectAttemptStartedAt >= ResolveConnectionAttemptTimeout())
        {
            FailConnectAttemptAsTimedOut();
        }
    }

    public async Task<CaptureDownloadResult> CaptureAsync(
        SessionPaths paths,
        CaptureRequestMessage request,
        Action<CaptureFastPreviewAttemptedResult>? onFastPreviewAttempted,
        Action<CaptureFastPreviewReadyResult>? onFastPreviewReady,
        Action<CaptureFastPreviewFailedResult>? onFastPreviewFailed,
        CancellationToken cancellationToken
    )
    {
        CurrentCaptureContext captureContext;
        CaptureShutterPlan shutterPlan;

        lock (_sync)
        {
            if (!_sessionOpen || _camera == IntPtr.Zero)
            {
                throw new CanonCaptureException(
                    "camera-not-ready",
                    "카메라 세션이 열려 있지 않아요.",
                    recoveryRequired: true
                );
            }

            if (_currentCapture is not null)
            {
                throw new CanonCaptureException(
                    "capture-in-flight",
                    "이미 진행 중인 촬영이 있어요.",
                    recoveryRequired: false
                );
            }

            captureContext = new CurrentCaptureContext(
                paths,
                request,
                onFastPreviewAttempted,
                onFastPreviewReady,
                onFastPreviewFailed
            );
            _currentCapture = captureContext;
            _snapshot = _snapshot with
            {
                CameraState = "capturing",
                HelperState = "healthy",
                DetailCode = "capture-in-flight",
                RequestId = request.RequestId,
            };
            shutterPlan = ResolveShutterPlanForNextCaptureLocked();
        }

        var err = await RunOnSdkStaThreadAsync(
            () =>
            {
                if (shutterPlan.DelayBeforeRelease > TimeSpan.Zero)
                {
                    Thread.Sleep(shutterPlan.DelayBeforeRelease);
                }

                return ExecuteCaptureShutterPlan(
                    _camera,
                    shutterPlan.ReleaseCommand,
                    shutterPlan.PrimeWithHalfway,
                    allowInternalErrorFallback: true
                );
            }
        );

        if (err != EDSDK.EDS_ERR_OK)
        {
            var captureTriggerException = BuildCaptureTriggerException(err);
            ClearCaptureContext(
                captureContext,
                captureTriggerException.DetailCode,
                captureTriggerException.RecoveryRequired ? "recovering" : "ready",
                captureTriggerException.RecoveryRequired,
                captureTriggerException.SessionResetRequired
            );

            throw captureTriggerException;
        }

        CaptureDownloadResult result;
        try
        {
            var captureCompletionTimeout = ResolveCaptureCompletionTimeout(paths.RuntimeRoot);
            result = await captureContext.Completion.Task.WaitAsync(
                captureCompletionTimeout,
                cancellationToken
            );
        }
        catch (TimeoutException)
        {
            var timeoutException = new CanonCaptureException(
                "capture-download-timeout",
                "RAW handoff를 기다리다 시간이 초과되었어요.",
                recoveryRequired: true
            );
            captureContext.Completion.TrySetException(timeoutException);
            ClearCaptureContext(
                captureContext,
                timeoutException.DetailCode,
                "recovering",
                timeoutException.RecoveryRequired
            );
            throw timeoutException;
        }

        lock (_sync)
        {
            if (_currentCapture == captureContext)
            {
                _currentCapture = null;
                _snapshot = _snapshot with
                {
                    CameraState = "ready",
                    HelperState = "healthy",
                    DetailCode = "camera-ready",
                    RequestId = null,
                };
            }
        }

        return result;
    }

    public void TryBackfillPreviewAssets(SessionPaths paths)
    {
        bool sdkInitialized;
        bool captureInFlight;
        lock (_sync)
        {
            sdkInitialized = _sdkInitialized;
            captureInFlight = _currentCapture is not null;
        }

        // Missing previews are best-effort. Don't compete with an active capture
        // for SDK time while the live RAW transfer boundary is still open.
        if (!sdkInitialized || captureInFlight || !Directory.Exists(paths.CapturesOriginalsDir))
        {
            return;
        }

        foreach (var rawPath in Directory.EnumerateFiles(paths.CapturesOriginalsDir))
        {
            var captureId = Path.GetFileNameWithoutExtension(rawPath);
            if (string.IsNullOrWhiteSpace(captureId))
            {
                continue;
            }

            if (HasRasterPreviewAsset(paths, captureId))
            {
                continue;
            }

            if (TryExtractPreviewWithWindowsShell(paths, rawPath, captureId))
            {
                continue;
            }

            TryRenderPreviewFromRaw(paths, rawPath, captureId);
        }
    }

    public void TryCompletePendingFastPreviewDownload()
    {
        PendingFastPreviewDownload? pendingDownload;
        lock (_sync)
        {
            if (_currentCapture is not null || _pendingFastPreviewDownloads.Count == 0)
            {
                return;
            }

            if (!_sessionOpen)
            {
                ReleasePendingFastPreviewDownloadsLocked();
                return;
            }

            pendingDownload = _pendingFastPreviewDownloads.Dequeue();
        }

        if (pendingDownload is null)
        {
            return;
        }

        try
        {
            var fastPreviewDownload = new CaptureFastPreviewDownloadResult(
                null,
                "raw-fallback-preview",
                "fast-preview-pending-missing-raw"
            );
            if (!string.IsNullOrWhiteSpace(pendingDownload.RawPath))
            {
                EmitFastPreviewAttempted(
                    pendingDownload.Context,
                    pendingDownload.CaptureId,
                    "raw-fallback-preview"
                );
                fastPreviewDownload = TryGenerateFastPreviewFromRaw(
                    pendingDownload.Context.Paths,
                    pendingDownload.RawPath,
                    pendingDownload.CaptureId,
                    null
                );
            }
            if (!string.IsNullOrWhiteSpace(fastPreviewDownload.FastPreviewPath))
            {
                try
                {
                    pendingDownload.Context.OnFastPreviewReady?.Invoke(
                        new CaptureFastPreviewReadyResult(
                            pendingDownload.Context.Request.RequestId,
                            pendingDownload.CaptureId,
                            fastPreviewDownload.FastPreviewPath,
                            fastPreviewDownload.FastPreviewKind,
                            DateTimeOffset.UtcNow
                        )
                    );
                }
                catch
                {
                    // Fast-preview notifications are best-effort. The RAW handoff
                    // remains the only correctness boundary for capture success.
                }
            }
            else if (!string.IsNullOrWhiteSpace(fastPreviewDownload.FailureDetailCode))
            {
                try
                {
                    pendingDownload.Context.OnFastPreviewFailed?.Invoke(
                        new CaptureFastPreviewFailedResult(
                            pendingDownload.Context.Request.RequestId,
                            pendingDownload.CaptureId,
                            fastPreviewDownload.FastPreviewKind,
                            fastPreviewDownload.FailureDetailCode,
                            DateTimeOffset.UtcNow
                        )
                    );
                }
                catch
                {
                    // Failure telemetry is best-effort and must not block RAW persistence.
                }
            }
        }
        finally
        {
            if (pendingDownload.DirectoryItem != IntPtr.Zero)
            {
                EDSDK.EdsRelease(pendingDownload.DirectoryItem);
            }
        }
    }

    public void Dispose()
    {
        ReleaseCamera();

        if (_sdkInitialized)
        {
            EDSDK.EdsTerminateSDK();
            _sdkInitialized = false;
        }

        _sdkThreadQueue.CompleteAdding();
        _sdkThread.Join(TimeSpan.FromSeconds(2));

        if (_selfHandle.IsAllocated)
        {
            _selfHandle.Free();
        }

        GC.KeepAlive(_objectHandler);
        GC.KeepAlive(_propertyHandler);
        GC.KeepAlive(_stateHandler);
    }

    private static TimeSpan ResolveCaptureCompletionTimeout(string runtimeRoot)
    {
        var overridePath = Path.Combine(runtimeRoot, CaptureCompletionTimeoutOverrideFileName);
        if (File.Exists(overridePath))
        {
            var overrideValue = File.ReadAllText(overridePath).Trim();
            if (long.TryParse(overrideValue, out var timeoutMs) && timeoutMs > 0)
            {
                return TimeSpan.FromMilliseconds(timeoutMs);
            }
        }

        var configured = Environment.GetEnvironmentVariable("BOOTHY_CAPTURE_TIMEOUT_MS");
        return long.TryParse(configured, out var configuredTimeoutMs) && configuredTimeoutMs > 0
            ? TimeSpan.FromMilliseconds(configuredTimeoutMs)
            : DefaultCaptureCompletionTimeout;
    }

    private static TimeSpan ResolveConnectionAttemptTimeout()
    {
        var configured = Environment.GetEnvironmentVariable(ConnectionAttemptTimeoutEnvVar);
        return long.TryParse(configured, out var configuredTimeoutMs) && configuredTimeoutMs > 0
            ? TimeSpan.FromMilliseconds(configuredTimeoutMs)
            : DefaultConnectionAttemptTimeout;
    }

    private uint ExecuteCaptureShutterPlan(
        IntPtr camera,
        EDSDK.EdsShutterButton releaseCommand,
        bool primeWithHalfway,
        bool allowInternalErrorFallback
    )
    {
        var err = ExecuteCaptureShutterPlanOnce(camera, releaseCommand, primeWithHalfway);

        if (
            err == EDSDK.EDS_ERR_INTERNAL_ERROR
            && allowInternalErrorFallback
            && !(primeWithHalfway && releaseCommand == EDSDK.EdsShutterButton.CameraCommand_ShutterButton_Completely_NonAF)
        )
        {
            Thread.Sleep(InternalTriggerRetryHalfPressLead);
            err = ExecuteCaptureShutterPlanOnce(
                camera,
                EDSDK.EdsShutterButton.CameraCommand_ShutterButton_Completely_NonAF,
                primeWithHalfway: true
            );
        }

        return err;
    }

    private uint ExecuteCaptureShutterPlanOnce(
        IntPtr camera,
        EDSDK.EdsShutterButton releaseCommand,
        bool primeWithHalfway
    )
    {
        var issuedHalfwayPrime = false;
        var attemptedRelease = false;
        uint err;
        if (primeWithHalfway)
        {
            err = SendCameraCommand(
                camera,
                EDSDK.CameraCommand_PressShutterButton,
                (int)EDSDK.EdsShutterButton.CameraCommand_ShutterButton_Halfway
            );
            issuedHalfwayPrime = err == EDSDK.EDS_ERR_OK;
            if (issuedHalfwayPrime)
            {
                Thread.Sleep(InternalTriggerRetryHalfPressLead);
            }
        }
        else
        {
            err = EDSDK.EDS_ERR_OK;
        }

        if (err == EDSDK.EDS_ERR_OK)
        {
            attemptedRelease = true;
            err = SendCameraCommand(
                camera,
                EDSDK.CameraCommand_PressShutterButton,
                (int)releaseCommand
            );
        }

        if (issuedHalfwayPrime || attemptedRelease)
        {
            var offErr = SendCameraCommand(
                camera,
                EDSDK.CameraCommand_PressShutterButton,
                (int)EDSDK.EdsShutterButton.CameraCommand_ShutterButton_OFF
            );
            if (err == EDSDK.EDS_ERR_OK)
            {
                err = offErr;
            }
        }

        return err;
    }

    private Task<T> RunOnSdkStaThreadAsync<T>(Func<T> work)
    {
        if (Thread.CurrentThread.ManagedThreadId == _sdkThreadId)
        {
            try
            {
                return Task.FromResult(work());
            }
            catch (Exception error)
            {
                return Task.FromException<T>(error);
            }
        }

        var completion = new TaskCompletionSource<T>(TaskCreationOptions.RunContinuationsAsynchronously);
        _sdkThreadQueue.Add(() =>
        {
            try
            {
                completion.TrySetResult(work());
            }
            catch (Exception error)
            {
                completion.TrySetException(error);
            }
        });
        return completion.Task;
    }

    private void SdkThreadLoop()
    {
        _sdkThreadId = Thread.CurrentThread.ManagedThreadId;
        foreach (var work in _sdkThreadQueue.GetConsumingEnumerable())
        {
            work();
        }
    }

    private uint SendCameraCommand(IntPtr camera, uint command, int parameter)
    {
        return _sendCommandOverride?.Invoke(camera, command, parameter)
            ?? EDSDK.EdsSendCommand(camera, command, parameter);
    }

    private void FailConnectAttemptAsTimedOut()
    {
        var detailCode = ResolveConnectTimeoutDetailCode();

        lock (_sync)
        {
            _connectTask = null;
            _connectAttemptStartedAt = DateTimeOffset.MinValue;
        }

        RecycleSdkIfNeeded();
        UpdateFailure("error", "error", detailCode);
    }

    private string ResolveConnectTimeoutDetailCode()
    {
        var snapshot = Snapshot;

        return snapshot.DetailCode switch
        {
            "sdk-initializing" or "scanning" => "sdk-init-timeout",
            "session-opening" => "session-open-timeout",
            _ => "camera-connect-timeout",
        };
    }

    private static CanonCaptureException BuildCaptureTriggerException(uint err)
    {
        return err switch
        {
            EDSDK.EDS_ERR_DEVICE_BUSY => new CanonCaptureException(
                "camera-busy",
                "카메라가 아직 직전 촬영을 정리하고 있어요. 잠시 후 다시 시도해 주세요.",
                recoveryRequired: false
            ),
            EDSDK.EDS_ERR_TAKE_PICTURE_AF_NG => new CanonCaptureException(
                "capture-focus-not-locked",
                "카메라가 초점을 아직 잡지 못했어요. 대상을 다시 맞춘 뒤 한 번 더 시도해 주세요.",
                recoveryRequired: false
            ),
            EDSDK.EDS_ERR_INTERNAL_ERROR => new CanonCaptureException(
                "capture-trigger-failed",
                $"셔터 명령을 보낼 수 없었어요: 0x{err:x8}",
                recoveryRequired: false,
                sessionResetRequired: true
            ),
            _ => new CanonCaptureException(
                "capture-trigger-failed",
                $"셔터 명령을 보낼 수 없었어요: 0x{err:x8}",
                recoveryRequired: true
            ),
        };
    }

    public static SelfCheckResult RunSelfCheck(string? sdkRoot)
    {
        var runtimeDllPath = Path.Combine(AppContext.BaseDirectory, "EDSDK.dll");
        var report = new SelfCheckResult
        {
            IsWindows = OperatingSystem.IsWindows(),
            RuntimeDllPresent = File.Exists(runtimeDllPath),
            SdkSourcePresent = !string.IsNullOrWhiteSpace(sdkRoot) && Directory.Exists(sdkRoot),
        };

        if (!report.IsWindows || !report.RuntimeDllPresent)
        {
            report.DetailCode = !report.IsWindows ? "windows-required" : "sdk-payload-missing";
            report.Message = !report.IsWindows
                ? "Windows x64 환경에서만 실행할 수 있어요."
                : "실행 폴더에 EDSDK.dll이 없어요.";
            return report;
        }

        try
        {
            var initializeResult = EDSDK.EdsInitializeSDK();
            report.SdkInitialized = initializeResult == EDSDK.EDS_ERR_OK;

            if (!report.SdkInitialized)
            {
                report.DetailCode = "sdk-init-failed";
                report.Message = $"SDK를 초기화하지 못했어요: 0x{initializeResult:x8}";
                return report;
            }

            IntPtr cameraList = IntPtr.Zero;
            try
            {
                var listResult = EDSDK.EdsGetCameraList(out cameraList);
                if (listResult == EDSDK.EDS_ERR_OK)
                {
                    var countResult = EDSDK.EdsGetChildCount(cameraList, out var count);
                    if (countResult == EDSDK.EDS_ERR_OK)
                    {
                        report.CameraCount = count;
                        report.DetailCode = count > 0 ? "camera-ready" : "camera-not-found";
                        report.Message = count > 0
                            ? $"{count}대의 카메라를 찾았어요."
                            : "연결된 카메라를 찾지 못했어요.";
                    }
                    else
                    {
                        report.DetailCode = "sdk-camera-list-failed";
                        report.Message = $"카메라 수를 읽지 못했어요: 0x{countResult:x8}";
                    }
                }
                else
                {
                    report.DetailCode = "sdk-camera-list-failed";
                    report.Message = $"카메라 목록을 열지 못했어요: 0x{listResult:x8}";
                }
            }
            finally
            {
                if (cameraList != IntPtr.Zero)
                {
                    EDSDK.EdsRelease(cameraList);
                }

                EDSDK.EdsTerminateSDK();
            }
        }
        catch (DllNotFoundException error)
        {
            report.DetailCode = "sdk-payload-missing";
            report.Message = error.Message;
        }
        catch (Exception error)
        {
            report.DetailCode = "self-check-failed";
            report.Message = error.Message;
        }

        return report;
    }

    private void TryOpenCamera()
    {
        lock (_sync)
        {
            _snapshot = _snapshot with
            {
                CameraState = "connecting",
                HelperState = _sdkInitialized ? "connecting" : "starting",
                DetailCode = _sdkInitialized ? "session-opening" : "sdk-initializing",
                RequestId = _currentCapture?.Request.RequestId,
            };
        }

        try
        {
            if (_connectAttemptOverride?.Invoke() == true)
            {
                return;
            }

            if (!_sdkInitialized)
            {
                var initializeResult = EDSDK.EdsInitializeSDK();
                if (initializeResult != EDSDK.EDS_ERR_OK)
                {
                    UpdateFailure("error", "error", "sdk-init-failed");
                    return;
                }

                _sdkInitialized = true;
                UpdateFailure("connecting", "connecting", "scanning");
            }

            IntPtr cameraList = IntPtr.Zero;
            IntPtr camera = IntPtr.Zero;

            try
            {
                var listResult = EDSDK.EdsGetCameraList(out cameraList);
                if (listResult != EDSDK.EDS_ERR_OK)
                {
                    UpdateFailure("error", "error", "sdk-camera-list-failed");
                    return;
                }

                var countResult = EDSDK.EdsGetChildCount(cameraList, out var count);
                if (countResult != EDSDK.EDS_ERR_OK)
                {
                    RecycleSdkIfNeeded();
                    UpdateFailure("error", "error", "sdk-camera-list-failed");
                    return;
                }

                if (count <= 0)
                {
                    var windowsCamera = WindowsCameraPresenceProbe.DetectCanonCamera();
                    RecycleSdkIfNeeded();
                    UpdateFailure(
                        windowsCamera.IsPresent ? "connecting" : "disconnected",
                        "healthy",
                        windowsCamera.IsPresent
                            ? "windows-device-detected"
                            : "camera-not-found",
                        windowsCamera.FriendlyName
                    );
                    return;
                }

                var childResult = EDSDK.EdsGetChildAtIndex(cameraList, 0, out camera);
                if (childResult != EDSDK.EDS_ERR_OK || camera == IntPtr.Zero)
                {
                    RecycleSdkIfNeeded();
                    UpdateFailure("error", "error", "camera-open-failed");
                    return;
                }

                var infoResult = EDSDK.EdsGetDeviceInfo(camera, out var deviceInfo);
                if (infoResult != EDSDK.EDS_ERR_OK)
                {
                    RecycleSdkIfNeeded();
                    UpdateFailure("error", "error", "camera-open-failed");
                    return;
                }

                var context = GCHandle.ToIntPtr(_selfHandle);
                EDSDK.EdsSetPropertyEventHandler(camera, EDSDK.PropertyEvent_All, _propertyHandler, context);
                EDSDK.EdsSetObjectEventHandler(camera, EDSDK.ObjectEvent_All, _objectHandler, context);
                EDSDK.EdsSetCameraStateEventHandler(camera, EDSDK.StateEvent_All, _stateHandler, context);

                UpdateFailure(
                    "connecting",
                    "connecting",
                    "session-opening",
                    deviceInfo.szDeviceDescription
                );

                var openResult = EDSDK.EdsOpenSession(camera);
                if (openResult != EDSDK.EDS_ERR_OK)
                {
                    EDSDK.EdsRelease(camera);
                    RecycleSdkIfNeeded();
                    UpdateFailure("error", "error", "session-open-failed");
                    return;
                }

                UpdateFailure(
                    "connected-idle",
                    "healthy",
                    "session-opened",
                    deviceInfo.szDeviceDescription
                );
                ConfigureSaveToHost(camera);

                lock (_sync)
                {
                    _camera = camera;
                    _sessionOpen = true;
                    _lastKeepAlive = DateTimeOffset.UtcNow;
                    _snapshot = BuildReadySnapshot(
                        deviceInfo.szDeviceDescription,
                        _currentCapture?.Request.RequestId,
                        DateTimeOffset.UtcNow
                    );
                }
            }
            finally
            {
                if (cameraList != IntPtr.Zero)
                {
                    EDSDK.EdsRelease(cameraList);
                }
            }
        }
        catch (DllNotFoundException)
        {
            UpdateFailure("error", "error", "sdk-payload-missing");
        }
        catch (Exception)
        {
            RecycleSdkIfNeeded();
            UpdateFailure("error", "error", "camera-open-failed");
        }
    }

    private async Task ObserveCompletedConnectAttemptAsync(Task connectTask)
    {
        try
        {
            await connectTask;
        }
        catch (Exception)
        {
            RecycleSdkIfNeeded();
            UpdateFailure("error", "error", "camera-open-failed");
        }
        finally
        {
            lock (_sync)
            {
                if (ReferenceEquals(_connectTask, connectTask))
                {
                    _connectTask = null;
                    _connectAttemptStartedAt = DateTimeOffset.MinValue;
                }
            }
        }
    }

    private Task StartConnectTask()
    {
        return RunOnSdkStaThreadAsync(() =>
        {
            TryOpenCamera();
            return true;
        });
    }

    internal static bool IsStartupConnectFailureDetailCode(string? detailCode)
    {
        return detailCode is
            "camera-connect-timeout"
            or "sdk-init-timeout"
            or "session-open-timeout"
            or "camera-open-failed"
            or "session-open-failed"
            or "sdk-init-failed";
    }

    private void ConfigureSaveToHost(IntPtr camera)
    {
        var saveToResult = EDSDK.EdsSetPropertyData(
            camera,
            EDSDK.PropID_SaveTo,
            0,
            sizeof(uint),
            (uint)EDSDK.EdsSaveTo.Host
        );

        if (saveToResult != EDSDK.EDS_ERR_OK)
        {
            return;
        }

        var capacity = new EDSDK.EdsCapacity
        {
            NumberOfFreeClusters = 0x7FFFFFFF,
            BytesPerSector = 0x1000,
            Reset = 1,
        };

        EDSDK.EdsSetCapacity(camera, capacity);
    }

    private void KeepCameraAwakeIfNeeded()
    {
        IntPtr camera;

        lock (_sync)
        {
            if (
                _camera == IntPtr.Zero
                || _currentCapture is not null
                || DateTimeOffset.UtcNow - _lastKeepAlive < KeepAliveInterval
            )
            {
                return;
            }

            camera = _camera;
        }

        var result = RunOnSdkStaThreadAsync(
                () => SendCameraCommand(camera, EDSDK.CameraCommand_ExtendShutDownTimer, 0)
            )
            .GetAwaiter()
            .GetResult();
        if (result == EDSDK.EDS_ERR_OK)
        {
            lock (_sync)
            {
                if (_camera == camera)
                {
                    _lastKeepAlive = DateTimeOffset.UtcNow;
                }
            }
            return;
        }

        UpdateFailure("recovering", "recovering", "reconnect-pending");
        RecycleSdkIfNeeded();
        ReleaseCamera();
    }

    private uint HandleObjectEvent(uint inEvent, IntPtr inRef, IntPtr inContext)
    {
        if (
            inEvent != EDSDK.ObjectEvent_DirItemRequestTransfer
            && inEvent != EDSDK.ObjectEvent_DirItemRequestTransferDT
        )
        {
            if (inRef != IntPtr.Zero)
            {
                EDSDK.EdsRelease(inRef);
            }

            return EDSDK.EDS_ERR_OK;
        }

        CurrentCaptureContext? captureContext;
        lock (_sync)
        {
            captureContext = _currentCapture;
        }

        if (captureContext is null || Interlocked.Exchange(ref captureContext.DownloadStarted, 1) == 1)
        {
            if (inRef != IntPtr.Zero)
            {
                EDSDK.EdsRelease(inRef);
            }

            return EDSDK.EDS_ERR_OK;
        }

        QueueCaptureDownload(captureContext, inRef);
        return EDSDK.EDS_ERR_OK;
    }

    private uint HandlePropertyEvent(uint inEvent, uint inPropertyId, uint inParam, IntPtr inContext)
    {
        return EDSDK.EDS_ERR_OK;
    }

    private uint HandleStateEvent(uint inEvent, uint inParameter, IntPtr inContext)
    {
        if (inEvent == EDSDK.StateEvent_Shutdown)
        {
            HandleConnectionLost("usb-disconnected", "recovering");
        }

        return EDSDK.EDS_ERR_OK;
    }

    private void QueueCaptureDownload(CurrentCaptureContext context, IntPtr directoryItem)
    {
        var downloadCapture = _downloadCaptureOverride ?? DownloadCapture;
        _ = Task.Factory.StartNew(
            () => downloadCapture(context, directoryItem),
            CancellationToken.None,
            TaskCreationOptions.LongRunning,
            TaskScheduler.Default
        );
    }

    private void DownloadCapture(CurrentCaptureContext context, IntPtr directoryItem)
    {
        IntPtr stream = IntPtr.Zero;
        var downloadCompleted = false;
        var tempPath = string.Empty;

        try
        {
            Directory.CreateDirectory(context.Paths.CapturesOriginalsDir);

            var infoResult = EDSDK.EdsGetDirectoryItemInfo(directoryItem, out var info);
            if (infoResult != EDSDK.EDS_ERR_OK)
            {
                throw new CanonCaptureException(
                    "download-info-failed",
                    $"파일 정보를 읽지 못했어요: 0x{infoResult:x8}",
                    recoveryRequired: true
                );
            }

            var extension = Path.GetExtension(info.szFileName);
            if (string.IsNullOrWhiteSpace(extension))
            {
                extension = ".cr3";
            }

            var captureId = BuildCaptureId();
            tempPath = Path.Combine(
                context.Paths.CapturesOriginalsDir,
                $"{captureId}.downloading{extension}"
            );
            var finalPath = Path.Combine(context.Paths.CapturesOriginalsDir, $"{captureId}{extension}");

            // Try the same-capture camera thumbnail before the full RAW transfer.
            // If the SDK can provide it here, the host can overlap preview work
            // with the still-in-flight RAW download instead of waiting for RAW
            // persistence to finish first.
            var immediateFastPreview = EnableImmediateCameraThumbnailFastPreview
                ? TryCaptureImmediateFastPreview(context, directoryItem, captureId)
                : new CaptureFastPreviewDownloadResult(null, null, null);

            var streamResult = EDSDK.EdsCreateFileStream(
                tempPath,
                EDSDK.EdsFileCreateDisposition.CreateAlways,
                EDSDK.EdsAccess.ReadWrite,
                out stream
            );
            if (streamResult != EDSDK.EDS_ERR_OK)
            {
                throw new CanonCaptureException(
                    "download-stream-failed",
                    $"임시 파일을 만들지 못했어요: 0x{streamResult:x8}",
                    recoveryRequired: true
                );
            }

            var downloadResult = EDSDK.EdsDownload(directoryItem, info.Size, stream);
            if (downloadResult != EDSDK.EDS_ERR_OK)
            {
                EDSDK.EdsDownloadCancel(directoryItem);
                throw new CanonCaptureException(
                    "download-failed",
                    $"RAW 다운로드에 실패했어요: 0x{downloadResult:x8}",
                    recoveryRequired: true
                );
            }

            var completeResult = EDSDK.EdsDownloadComplete(directoryItem);
            if (completeResult != EDSDK.EDS_ERR_OK)
            {
                throw new CanonCaptureException(
                    "download-complete-failed",
                    $"다운로드 마무리에 실패했어요: 0x{completeResult:x8}",
                    recoveryRequired: true
                );
            }

            downloadCompleted = true;

            if (stream != IntPtr.Zero)
            {
                EDSDK.EdsRelease(stream);
                stream = IntPtr.Zero;
            }

            File.Move(tempPath, finalPath, overwrite: true);

            var fileInfo = new FileInfo(finalPath);
            if (!fileInfo.Exists || fileInfo.Length == 0)
            {
                throw new CanonCaptureException(
                    "download-empty-file",
                    "저장된 RAW 파일이 비어 있어요.",
                    recoveryRequired: true
                );
            }

            if (string.IsNullOrWhiteSpace(immediateFastPreview.FastPreviewPath))
            {
                QueuePendingFastPreviewDownload(context, directoryItem, captureId, finalPath);
                directoryItem = IntPtr.Zero;
            }

            context.Completion.TrySetResult(
                new CaptureDownloadResult(
                    context.Request.RequestId,
                    captureId,
                    finalPath,
                    DateTimeOffset.UtcNow,
                    immediateFastPreview.FastPreviewPath,
                    immediateFastPreview.FastPreviewKind
                )
            );
        }
        catch (Exception error)
        {
            if (!downloadCompleted && directoryItem != IntPtr.Zero)
            {
                EDSDK.EdsDownloadCancel(directoryItem);
            }

            if (error is CanonCaptureException captureException)
            {
                ClearCaptureContext(
                    context,
                    captureException.DetailCode,
                    captureException.RecoveryRequired ? "recovering" : "ready",
                    captureException.RecoveryRequired,
                    captureException.SessionResetRequired
                );
                context.Completion.TrySetException(captureException);
            }
            else
            {
                ClearCaptureContext(context, "download-failed", "recovering", true);
                context.Completion.TrySetException(
                    new CanonCaptureException(
                        "download-failed",
                        error.Message,
                        recoveryRequired: true
                    )
                );
            }

            if (!string.IsNullOrWhiteSpace(tempPath) && File.Exists(tempPath))
            {
                try
                {
                    File.Delete(tempPath);
                }
                catch
                {
                }
            }
        }
        finally
        {
            if (stream != IntPtr.Zero)
            {
                EDSDK.EdsRelease(stream);
            }

            if (directoryItem != IntPtr.Zero)
            {
                EDSDK.EdsRelease(directoryItem);
            }
        }
    }

    private CaptureFastPreviewDownloadResult TryDownloadPreviewThumbnail(
        SessionPaths paths,
        IntPtr directoryItem,
        string captureId
    )
    {
        IntPtr thumbnailStream = IntPtr.Zero;
        var tempPreviewPath = Path.Combine(
            paths.RendersPreviewsDir,
            $"{captureId}.thumbnail.downloading.jpg"
        );

        try
        {
            Directory.CreateDirectory(paths.RendersPreviewsDir);

            if (File.Exists(tempPreviewPath))
            {
                File.Delete(tempPreviewPath);
            }

            var previewPath = Path.Combine(paths.RendersPreviewsDir, $"{captureId}.jpg");
            var createStreamResult = EDSDK.EdsCreateFileStream(
                tempPreviewPath,
                EDSDK.EdsFileCreateDisposition.CreateAlways,
                EDSDK.EdsAccess.ReadWrite,
                out thumbnailStream
            );

            if (createStreamResult != EDSDK.EDS_ERR_OK)
            {
                return new CaptureFastPreviewDownloadResult(
                    null,
                    "camera-thumbnail",
                    "fast-thumbnail-stream-create-failed"
                );
            }

            var thumbnailResult = EDSDK.EdsDownloadThumbnail(directoryItem, thumbnailStream);
            if (thumbnailResult != EDSDK.EDS_ERR_OK)
            {
                return new CaptureFastPreviewDownloadResult(
                    null,
                    "camera-thumbnail",
                    "fast-thumbnail-download-failed"
                );
            }

            EDSDK.EdsRelease(thumbnailStream);
            thumbnailStream = IntPtr.Zero;

            var previewFileInfo = new FileInfo(tempPreviewPath);
            if (!previewFileInfo.Exists || previewFileInfo.Length == 0)
            {
                return new CaptureFastPreviewDownloadResult(
                    null,
                    "camera-thumbnail",
                    "fast-thumbnail-empty-file"
                );
            }

            File.Move(tempPreviewPath, previewPath, overwrite: true);
            return new CaptureFastPreviewDownloadResult(
                previewPath,
                "camera-thumbnail",
                null
            );
        }
        catch
        {
            // Thumbnail extraction is best-effort. RAW persistence remains the source of truth.
            return new CaptureFastPreviewDownloadResult(
                null,
                "camera-thumbnail",
                "fast-thumbnail-exception"
            );
        }
        finally
        {
            if (thumbnailStream != IntPtr.Zero)
            {
                EDSDK.EdsRelease(thumbnailStream);
            }

            if (File.Exists(tempPreviewPath))
            {
                try
                {
                    File.Delete(tempPreviewPath);
                }
                catch
                {
                }
            }
        }
    }

    private CaptureFastPreviewDownloadResult TryCaptureImmediateFastPreview(
        CurrentCaptureContext context,
        IntPtr directoryItem,
        string captureId
    )
    {
        EmitFastPreviewAttempted(context, captureId, "camera-thumbnail");
        var fastPreviewDownload = TryDownloadPreviewThumbnail(context.Paths, directoryItem, captureId);
        if (string.IsNullOrWhiteSpace(fastPreviewDownload.FastPreviewPath))
        {
            return fastPreviewDownload;
        }

        EmitFastPreviewReady(context, captureId, fastPreviewDownload);
        return fastPreviewDownload;
    }

    private void EmitFastPreviewAttempted(
        CurrentCaptureContext context,
        string captureId,
        string fastPreviewKind
    )
    {
        try
        {
            context.OnFastPreviewAttempted?.Invoke(
                new CaptureFastPreviewAttemptedResult(
                    context.Request.RequestId,
                    captureId,
                    fastPreviewKind,
                    DateTimeOffset.UtcNow
                )
            );
        }
        catch
        {
            // Attempt telemetry is best-effort and must not block RAW persistence.
        }
    }

    private void EmitFastPreviewReady(
        CurrentCaptureContext context,
        string captureId,
        CaptureFastPreviewDownloadResult fastPreviewDownload
    )
    {
        try
        {
            context.OnFastPreviewReady?.Invoke(
                new CaptureFastPreviewReadyResult(
                    context.Request.RequestId,
                    captureId,
                    fastPreviewDownload.FastPreviewPath!,
                    fastPreviewDownload.FastPreviewKind,
                    DateTimeOffset.UtcNow
                )
            );
        }
        catch
        {
            // Fast-preview notifications are best-effort. The RAW handoff
            // remains the only correctness boundary for capture success.
        }
    }

    private CaptureFastPreviewDownloadResult TryGenerateFastPreviewFromRaw(
        SessionPaths paths,
        string rawPath,
        string captureId,
        string? previousFailureDetailCode
    )
    {
        if (TryExtractPreviewWithWindowsShell(paths, rawPath, captureId))
        {
            return new CaptureFastPreviewDownloadResult(
                Path.Combine(paths.RendersPreviewsDir, $"{captureId}.jpg"),
                "windows-shell-thumbnail",
                null
            );
        }

        if (TryRenderPreviewFromRaw(paths, rawPath, captureId))
        {
            return new CaptureFastPreviewDownloadResult(
                Path.Combine(paths.RendersPreviewsDir, $"{captureId}.jpg"),
                "raw-sdk-preview",
                null
            );
        }

        return new CaptureFastPreviewDownloadResult(
            null,
            "raw-sdk-preview",
            "fast-preview-fallback-failed"
        );
    }

    private bool TryRenderPreviewFromRaw(SessionPaths paths, string rawPath, string captureId)
    {
        IntPtr rawStream = IntPtr.Zero;
        IntPtr imageRef = IntPtr.Zero;
        IntPtr previewStream = IntPtr.Zero;
        var tempPreviewPath = Path.Combine(
            paths.RendersPreviewsDir,
            $"{captureId}.rendering.jpg"
        );

        try
        {
            Directory.CreateDirectory(paths.RendersPreviewsDir);

            if (!File.Exists(rawPath))
            {
                return false;
            }

            if (File.Exists(tempPreviewPath))
            {
                File.Delete(tempPreviewPath);
            }

            var previewPath = Path.Combine(paths.RendersPreviewsDir, $"{captureId}.jpg");
            var createRawStreamResult = EDSDK.EdsCreateFileStream(
                rawPath,
                EDSDK.EdsFileCreateDisposition.OpenExisting,
                EDSDK.EdsAccess.Read,
                out rawStream
            );

            if (createRawStreamResult != EDSDK.EDS_ERR_OK)
            {
                return false;
            }

            var createImageRefResult = EDSDK.EdsCreateImageRef(rawStream, out imageRef);
            if (createImageRefResult != EDSDK.EDS_ERR_OK)
            {
                return false;
            }

            var createPreviewStreamResult = EDSDK.EdsCreateFileStream(
                tempPreviewPath,
                EDSDK.EdsFileCreateDisposition.CreateAlways,
                EDSDK.EdsAccess.ReadWrite,
                out previewStream
            );

            if (createPreviewStreamResult != EDSDK.EDS_ERR_OK)
            {
                return false;
            }

            var saveResult = CanonSdkNative.EdsSaveImage(
                imageRef,
                EDSDK.EdsTargetImageType.Jpeg,
                new EDSDK.EdsSaveImageSetting
                {
                    JPEGQuality = DefaultPreviewJpegQuality,
                    reserved = 0,
                },
                previewStream
            );

            if (saveResult != EDSDK.EDS_ERR_OK)
            {
                return false;
            }

            EDSDK.EdsRelease(previewStream);
            previewStream = IntPtr.Zero;

            var previewFileInfo = new FileInfo(tempPreviewPath);
            if (!previewFileInfo.Exists || previewFileInfo.Length == 0)
            {
                return false;
            }

            File.Move(tempPreviewPath, previewPath, overwrite: true);
            return true;
        }
        catch
        {
            // RAW preview rendering is best-effort. The session keeps the RAW source of truth.
            return false;
        }
        finally
        {
            if (previewStream != IntPtr.Zero)
            {
                EDSDK.EdsRelease(previewStream);
            }

            if (imageRef != IntPtr.Zero)
            {
                EDSDK.EdsRelease(imageRef);
            }

            if (rawStream != IntPtr.Zero)
            {
                EDSDK.EdsRelease(rawStream);
            }

            if (File.Exists(tempPreviewPath))
            {
                try
                {
                    File.Delete(tempPreviewPath);
                }
                catch
                {
                }
            }
        }
    }

    private static bool HasRasterPreviewAsset(SessionPaths paths, string captureId)
    {
        return DisplayablePreviewExtensions.Any((extension) =>
            File.Exists(Path.Combine(paths.RendersPreviewsDir, $"{captureId}{extension}"))
        );
    }

    private bool TryExtractPreviewWithWindowsShell(
        SessionPaths paths,
        string rawPath,
        string captureId
    )
    {
        var tempPreviewPath = Path.Combine(
            paths.RendersPreviewsDir,
            $"{captureId}.shell-preview.jpg"
        );

        try
        {
            if (File.Exists(tempPreviewPath))
            {
                File.Delete(tempPreviewPath);
            }

            var previewPath = Path.Combine(paths.RendersPreviewsDir, $"{captureId}.jpg");
            if (!WindowsShellThumbnail.TrySavePreviewJpeg(rawPath, tempPreviewPath))
            {
                return false;
            }

            File.Move(tempPreviewPath, previewPath, overwrite: true);
            return true;
        }
        catch
        {
            // Windows shell thumbnail extraction is best-effort.
            return false;
        }
        finally
        {
            if (File.Exists(tempPreviewPath))
            {
                try
                {
                    File.Delete(tempPreviewPath);
                }
                catch
                {
                }
            }
        }
    }

    private void ClearCaptureContext(
        CurrentCaptureContext context,
        string detailCode,
        string nextCameraState,
        bool recoveryRequired,
        bool sessionResetRequired = false
    )
    {
        var shouldReconnectSession = recoveryRequired || sessionResetRequired;

        lock (_sync)
        {
            if (_currentCapture == context)
            {
                var nextDetailCode = shouldReconnectSession
                    ? "reconnect-pending"
                    : nextCameraState == "ready"
                        ? "camera-ready"
                        : detailCode;
                _currentCapture = null;
                _snapshot = _snapshot with
                {
                    CameraState = shouldReconnectSession ? "recovering" : nextCameraState,
                    HelperState = shouldReconnectSession ? "recovering" : "healthy",
                    DetailCode = nextDetailCode,
                    RequestId = null,
                };
            }
        }

        if (shouldReconnectSession)
        {
            lock (_sync)
            {
                if (sessionResetRequired)
                {
                    var protectedRetryNotBeforeAt =
                        DateTimeOffset.UtcNow + InternalTriggerReconnectReadyWarmup;
                    _delayedReadyNotBeforeAt = protectedRetryNotBeforeAt;
                    _internalTriggerRetryGuardNotBeforeAt = protectedRetryNotBeforeAt;
                    _useProtectedRetryShutterPlanOnNextCapture = true;
                }
                else
                {
                    _internalTriggerRetryGuardNotBeforeAt = DateTimeOffset.MinValue;
                    _useProtectedRetryShutterPlanOnNextCapture = false;
                }
            }
            RecycleSdkIfNeeded();
            ReleaseCamera();
        }
    }

    private void FailActiveCapture(CanonCaptureException exception)
    {
        CurrentCaptureContext? activeCapture;
        lock (_sync)
        {
            activeCapture = _currentCapture;
        }

        if (activeCapture is null)
        {
            return;
        }

        activeCapture.Completion.TrySetException(exception);
        ClearCaptureContext(activeCapture, exception.DetailCode, "recovering", exception.RecoveryRequired);
    }

    private void UpdateFailure(
        string cameraState,
        string helperState,
        string detailCode,
        string? cameraModel = null
    )
    {
        lock (_sync)
        {
            _snapshot = _snapshot with
            {
                CameraState = cameraState,
                HelperState = helperState,
                DetailCode = detailCode,
                CameraModel = cameraModel,
                RequestId = _currentCapture?.Request.RequestId,
            };
        }
    }

    private void HandleConnectionLost(string detailCode, string nextCameraState)
    {
        UpdateFailure(nextCameraState, "recovering", detailCode);
        FailActiveCapture(
            new CanonCaptureException(
                detailCode,
                "카메라 연결이 끊겼어요.",
                recoveryRequired: true
            )
        );
        RecycleSdkIfNeeded();
        ReleaseCamera();
    }

    private void ReleaseCamera()
    {
        lock (_sync)
        {
            ReleasePendingFastPreviewDownloadsLocked();
            if (_sessionOpen && _camera != IntPtr.Zero)
            {
                EDSDK.EdsCloseSession(_camera);
            }

            if (_camera != IntPtr.Zero)
            {
                EDSDK.EdsRelease(_camera);
            }

            _camera = IntPtr.Zero;
            _sessionOpen = false;
        }
    }

    private CameraSnapshot BuildReadySnapshot(
        string? cameraModel,
        string? requestId,
        DateTimeOffset now
    )
    {
        return now < _delayedReadyNotBeforeAt
            ? new CameraSnapshot("connected-idle", "healthy", "session-opened", cameraModel, requestId)
            : new CameraSnapshot("ready", "healthy", "camera-ready", cameraModel, requestId);
    }

    private CaptureShutterPlan ResolveShutterPlanForNextCaptureLocked()
    {
        var retryGuardDelay = TimeSpan.Zero;
        if (
            _useProtectedRetryShutterPlanOnNextCapture
            && _internalTriggerRetryGuardNotBeforeAt > DateTimeOffset.UtcNow
        )
        {
            retryGuardDelay = _internalTriggerRetryGuardNotBeforeAt - DateTimeOffset.UtcNow;
        }

        var shutterPlan = _useProtectedRetryShutterPlanOnNextCapture
            ? new CaptureShutterPlan(
                EDSDK.EdsShutterButton.CameraCommand_ShutterButton_Completely_NonAF,
                PrimeWithHalfway: true,
                DelayBeforeRelease: retryGuardDelay
            )
            : new CaptureShutterPlan(
                EDSDK.EdsShutterButton.CameraCommand_ShutterButton_Completely,
                PrimeWithHalfway: false,
                DelayBeforeRelease: TimeSpan.Zero
            );
        _useProtectedRetryShutterPlanOnNextCapture = false;
        _internalTriggerRetryGuardNotBeforeAt = DateTimeOffset.MinValue;
        return shutterPlan;
    }

    private void RefreshDelayedReadyLocked(DateTimeOffset now)
    {
        if (
            !_sessionOpen
            || _camera == IntPtr.Zero
            || _snapshot.CameraState == "ready"
            || _snapshot.DetailCode != "session-opened"
            || now < _delayedReadyNotBeforeAt
        )
        {
            return;
        }

        _delayedReadyNotBeforeAt = DateTimeOffset.MinValue;
        _snapshot = new CameraSnapshot(
            "ready",
            "healthy",
            "camera-ready",
            _snapshot.CameraModel,
            _snapshot.RequestId
        );
    }

    private void RecycleSdkIfNeeded()
    {
        lock (_sync)
        {
            if (!_sdkInitialized)
            {
                return;
            }

            if (DateTimeOffset.UtcNow - _lastSdkRecycleAt < MinimumSdkRecycleInterval)
            {
                return;
            }

            _lastSdkRecycleAt = DateTimeOffset.UtcNow;
        }

        ForceRecycleSdk();
    }

    private void ForceRecycleSdk()
    {
        ReleaseCamera();

        lock (_sync)
        {
            if (!_sdkInitialized)
            {
                return;
            }

            EDSDK.EdsTerminateSDK();
            _sdkInitialized = false;
        }
    }

    private static string BuildCaptureId()
    {
        var stamp = DateTimeOffset.UtcNow.ToString("yyyyMMddHHmmssfff");
        var suffix = Guid.NewGuid().ToString("N")[..10];
        return $"capture_{stamp}_{suffix}";
    }

    private void QueuePendingFastPreviewDownload(
        CurrentCaptureContext context,
        IntPtr directoryItem,
        string captureId,
        string rawPath
    )
    {
        lock (_sync)
        {
            if (directoryItem != IntPtr.Zero)
            {
                EDSDK.EdsRelease(directoryItem);
                directoryItem = IntPtr.Zero;
            }
            _pendingFastPreviewDownloads.Enqueue(
                new PendingFastPreviewDownload(
                context,
                directoryItem,
                captureId,
                rawPath
                )
            );
        }
    }

    private void ReleasePendingFastPreviewDownloadsLocked()
    {
        while (_pendingFastPreviewDownloads.Count > 0)
        {
            var pendingDownload = _pendingFastPreviewDownloads.Dequeue();
            if (pendingDownload.DirectoryItem != IntPtr.Zero)
            {
                EDSDK.EdsRelease(pendingDownload.DirectoryItem);
            }
        }
    }
}

internal sealed record CameraSnapshot(
    string CameraState,
    string HelperState,
    string? DetailCode,
    string? CameraModel,
    string? RequestId
);

internal sealed record CaptureDownloadResult(
    string RequestId,
    string CaptureId,
    string RawPath,
    DateTimeOffset ArrivedAt,
    string? FastPreviewPath,
    string? FastPreviewKind
);

internal sealed record CaptureFastPreviewReadyResult(
    string RequestId,
    string CaptureId,
    string FastPreviewPath,
    string? FastPreviewKind,
    DateTimeOffset ObservedAt
);

internal sealed record CaptureFastPreviewAttemptedResult(
    string RequestId,
    string CaptureId,
    string? FastPreviewKind,
    DateTimeOffset ObservedAt
);

internal sealed record CaptureFastPreviewFailedResult(
    string RequestId,
    string CaptureId,
    string? FastPreviewKind,
    string DetailCode,
    DateTimeOffset ObservedAt
);

internal sealed record CaptureFastPreviewDownloadResult(
    string? FastPreviewPath,
    string? FastPreviewKind,
    string? FailureDetailCode
);

internal sealed record PendingFastPreviewDownload(
    CurrentCaptureContext Context,
    IntPtr DirectoryItem,
    string CaptureId,
    string RawPath
);

internal sealed class CanonCaptureException : Exception
{
    public CanonCaptureException(
        string detailCode,
        string message,
        bool recoveryRequired,
        bool sessionResetRequired = false
    )
        : base(message)
    {
        DetailCode = detailCode;
        RecoveryRequired = recoveryRequired;
        SessionResetRequired = sessionResetRequired;
    }

    public string DetailCode { get; }
    public bool RecoveryRequired { get; }
    public bool SessionResetRequired { get; }
}

internal sealed class SelfCheckResult
{
    public bool IsWindows { get; set; }
    public bool RuntimeDllPresent { get; set; }
    public bool SdkSourcePresent { get; set; }
    public bool SdkInitialized { get; set; }
    public int CameraCount { get; set; }
    public string? DetailCode { get; set; }
    public string? Message { get; set; }
}

internal sealed class CurrentCaptureContext
{
    public CurrentCaptureContext(
        SessionPaths paths,
        CaptureRequestMessage request,
        Action<CaptureFastPreviewAttemptedResult>? onFastPreviewAttempted,
        Action<CaptureFastPreviewReadyResult>? onFastPreviewReady,
        Action<CaptureFastPreviewFailedResult>? onFastPreviewFailed
    )
    {
        Paths = paths;
        Request = request;
        OnFastPreviewAttempted = onFastPreviewAttempted;
        OnFastPreviewReady = onFastPreviewReady;
        OnFastPreviewFailed = onFastPreviewFailed;
        StartedAt = DateTimeOffset.UtcNow;
        Completion = new TaskCompletionSource<CaptureDownloadResult>(
            TaskCreationOptions.RunContinuationsAsynchronously
        );
    }

    public SessionPaths Paths { get; }
    public CaptureRequestMessage Request { get; }
    public Action<CaptureFastPreviewAttemptedResult>? OnFastPreviewAttempted { get; }
    public Action<CaptureFastPreviewReadyResult>? OnFastPreviewReady { get; }
    public Action<CaptureFastPreviewFailedResult>? OnFastPreviewFailed { get; }
    public DateTimeOffset StartedAt { get; }
    public TaskCompletionSource<CaptureDownloadResult> Completion { get; }
    public int DownloadStarted;
}
