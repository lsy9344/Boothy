using System.Runtime.InteropServices;
using CanonHelper.Protocol;
using EDSDKLib;

namespace CanonHelper.Runtime;

internal sealed class CanonSdkCamera : IDisposable
{
    private static readonly TimeSpan MinimumSdkRecycleInterval = TimeSpan.FromSeconds(2);
    private static readonly TimeSpan KeepAliveInterval = TimeSpan.FromMilliseconds(1500);
    private const uint DefaultPreviewJpegQuality = 8;
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
    // Real follow-up captures on EOS 700D hardware can take well beyond 15 seconds
    // before the transfer boundary closes. Keep enough headroom to avoid treating
    // slow but valid RAW handoffs as fatal helper failures.
    private static readonly TimeSpan DefaultCaptureCompletionTimeout = TimeSpan.FromMilliseconds(
        30000
    );
    private static readonly TimeSpan PairedObjectCompletionTimeout = TimeSpan.FromSeconds(5);

    // RAW+JPEG(paired) 비교 lane에서는 transfer object가 둘이라 RAW handoff가 늦게 닫힐 수
    // 있다. HV-14 첫 회차에서 20회 요청 중 2회가 30초 예산을 넘겨 중단됐다. host의
    // PAIRED_SOURCE_CAPTURE_TIMEOUT_ALLOWANCE_MS와 같은 값이어야 host가 먼저 끊지 않는다.
    internal static readonly TimeSpan PairedCaptureCompletionAllowance = TimeSpan.FromSeconds(15);

    // DEVICE_BUSY는 셔터가 눌리지 않았다는 뜻이므로 재시도해도 이중 촬영이 되지 않는다.
    // HV-14에서 10~20초 간격의 정상 회차 중에도 busy 1건이 run을 중단시켰다.
    private static readonly TimeSpan ShutterBusyRetryInterval = TimeSpan.FromMilliseconds(250);
    internal const int ShutterBusyRetryLimit = 8;

    private readonly object _sync = new();
    private readonly GCHandle _selfHandle;
    private readonly EDSDK.EdsObjectEventHandler _objectHandler;
    private readonly EDSDK.EdsPropertyEventHandler _propertyHandler;
    private readonly EDSDK.EdsStateEventHandler _stateHandler;

    private IntPtr _camera = IntPtr.Zero;
    private bool _sdkInitialized;
    private bool _sessionOpen;
    private CameraSnapshot _snapshot =
        new("connecting", "starting", "helper-starting", null, null);
    private CurrentCaptureContext? _currentCapture;
    // paired 비교 lane이 ImageQuality를 바꿨을 때의 원래 값. 촬영마다 되돌리지 않고 유지한다
    // — per-shot 설정 왕복이 다음 셔터의 DEVICE_BUSY를 유발했다 (HV-14). 복원 시점은
    // 요청 실패, 비-paired 촬영 진입, 카메라 세션 종료다.
    private int? _heldOriginalImageQuality;
    private readonly Queue<PendingFastPreviewDownload> _pendingFastPreviewDownloads = new();
    private DateTimeOffset _lastKeepAlive = DateTimeOffset.MinValue;
    private DateTimeOffset _lastSdkRecycleAt = DateTimeOffset.MinValue;

    public CanonSdkCamera()
    {
        _selfHandle = GCHandle.Alloc(this);
        _objectHandler = HandleObjectEvent;
        _propertyHandler = HandlePropertyEvent;
        _stateHandler = HandleStateEvent;
    }

    public CameraSnapshot Snapshot
    {
        get
        {
            lock (_sync)
            {
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
                return _sessionOpen && _snapshot.CameraState == "ready";
            }
        }
    }

    public void PumpEvents()
    {
        bool sdkInitialized;
        lock (_sync)
        {
            sdkInitialized = _sdkInitialized;
        }

        if (!sdkInitialized)
        {
            return;
        }

        uint result;
        try
        {
            result = CanonSdkNative.EdsGetEvent();
        }
        catch (DllNotFoundException)
        {
            UpdateFailure("error", "error", "sdk-payload-missing");
            return;
        }
        catch (Exception)
        {
            HandleConnectionLost("event-pump-failed", "recovering");
            return;
        }

        if (result == EDSDK.EDS_ERR_OK)
        {
            return;
        }

        switch (result)
        {
            case EDSDK.EDS_ERR_COMM_DISCONNECTED:
            case EDSDK.EDS_ERR_DEVICE_NOT_FOUND:
            case EDSDK.EDS_ERR_DEVICE_INVALID:
            case EDSDK.EDS_ERR_SESSION_NOT_OPEN:
                HandleConnectionLost("usb-disconnected", "recovering");
                return;
            default:
                HandleConnectionLost("event-pump-failed", "recovering");
                return;
        }
    }

    public async Task EnsureConnectedAsync(CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();

        if (_sessionOpen)
        {
            KeepCameraAwakeIfNeeded();
            return;
        }

        await Task.Run(TryOpenCamera, cancellationToken);
    }

    public async Task<CaptureDownloadResult> CaptureAsync(
        SessionPaths paths,
        CaptureRequestMessage request,
        Action<CaptureFastPreviewAttemptedResult>? onFastPreviewAttempted,
        Action<CaptureFastPreviewReadyResult>? onFastPreviewReady,
        Action<CaptureFastPreviewFailedResult>? onFastPreviewFailed,
        CancellationToken cancellationToken,
        bool enablePairedJpeg = false,
        Action<ImageQualityCapability>? onImageQualityCapability = null,
        Action<CapturePairedJpegResult>? onPairedJpeg = null,
        Action<CaptureObjectRejectedResult>? onObjectRejected = null,
        Action<CaptureCameraSettingWarningResult>? onCameraSettingWarning = null
    )
    {
        CurrentCaptureContext captureContext;
        ImageQualityCapability? imageQualityCapability = null;
        string? imageQualityActivationFailure = null;

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

            var pairedJpegActive = false;
            if (enablePairedJpeg)
            {
                imageQualityCapability = ProbeImageQualityCapability(
                    _camera,
                    DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() * 1_000
                );
                var originalImageQuality = imageQualityCapability.CurrentValue;
                var pairedValue = ImageQualityValue.SelectRawPlusJpegCandidate(
                    imageQualityCapability.SupportedValues
                );

                if (pairedValue is null)
                {
                    imageQualityActivationFailure = "unsupported-combination";
                }
                else if (originalImageQuality is null)
                {
                    imageQualityActivationFailure = "image-quality-current-unavailable";
                }
                else if (pairedValue.Value == originalImageQuality.Value)
                {
                    // 이미 paired 값이다 — 직전 촬영부터 유지 중이거나 사용자의 원래 설정이다.
                    // 어느 쪽이든 property 왕복 없이 그대로 촬영한다.
                    pairedJpegActive = true;
                }
                else if (TrySetImageQuality(_camera, pairedValue.Value))
                {
                    pairedJpegActive = true;
                    // 최초 전환 시점의 원래 값만 기억한다. 이후 paired 촬영은 위의
                    // 같은-값 분기로 들어와 property를 건드리지 않는다.
                    _heldOriginalImageQuality ??= originalImageQuality;
                }
                else
                {
                    imageQualityActivationFailure = "image-quality-set-failed";
                }
            }
            else
            {
                // 비교 lane이 꺼진 촬영이 들어오면 유지 중이던 측정 설정을 먼저 되돌린다.
                RestoreHeldImageQuality(null);
            }

            captureContext = new CurrentCaptureContext(
                paths,
                request,
                onFastPreviewAttempted,
                onFastPreviewReady,
                onFastPreviewFailed,
                BuildCaptureId(),
                pairedJpegActive ? 2 : 1,
                onPairedJpeg,
                onObjectRejected,
                onCameraSettingWarning
            );
            _currentCapture = captureContext;
            _snapshot = _snapshot with
            {
                CameraState = "capturing",
                HelperState = "healthy",
                DetailCode = "capture-in-flight",
                RequestId = request.RequestId,
            };
        }

        if (imageQualityCapability is not null)
        {
            try
            {
                onImageQualityCapability?.Invoke(imageQualityCapability);
            }
            catch
            {
                // Capability telemetry is advisory and must not block RAW capture.
            }
        }

        if (imageQualityActivationFailure is not null)
        {
            EmitObjectRejected(
                captureContext,
                imageQualityActivationFailure,
                "jpeg",
                0,
                null
            );
        }

        try
        {
            var err = await PressShutterWithBusyRetryAsync(cancellationToken);

            if (err == EDSDK.EDS_ERR_OK)
            {
                err = EDSDK.EdsSendCommand(
                    _camera,
                    EDSDK.CameraCommand_PressShutterButton,
                    (int)EDSDK.EdsShutterButton.CameraCommand_ShutterButton_OFF
                );
            }

            if (err != EDSDK.EDS_ERR_OK)
            {
                var captureTriggerException = BuildCaptureTriggerException(err);
                ClearCaptureContext(
                    captureContext,
                    captureTriggerException.DetailCode,
                    captureTriggerException.RecoveryRequired ? "recovering" : "ready",
                    captureTriggerException.RecoveryRequired
                );

                throw captureTriggerException;
            }

            var captureCompletionTimeout = ApplyPairedCompletionAllowance(
                ResolveCaptureCompletionTimeout(paths.RuntimeRoot),
                captureContext.Correlator.ExpectedObjectCount
            );
            CaptureDownloadResult rawResult;
            try
            {
                rawResult = await captureContext.RawCompletion.Task.WaitAsync(
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
                captureContext.RawCompletion.TrySetException(timeoutException);
                captureContext.Completion.TrySetException(timeoutException);
                ClearCaptureContext(
                    captureContext,
                    timeoutException.DetailCode,
                    "recovering",
                    timeoutException.RecoveryRequired
                );
                throw timeoutException;
            }

            var result = rawResult;
            if (captureContext.Correlator.ExpectedObjectCount > 1)
            {
                try
                {
                    result = await captureContext.Completion.Task.WaitAsync(
                        PairedObjectCompletionTimeout,
                        cancellationToken
                    );
                }
                catch (TimeoutException)
                {
                    EmitObjectRejected(
                        captureContext,
                        "paired-jpeg-timeout",
                        "jpeg",
                        0,
                        null
                    );
                }
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
        catch (OperationCanceledException)
        {
            ClearCaptureContext(captureContext, "capture-cancelled", "ready", false);
            throw;
        }
        // 성공한 paired 촬영 뒤에는 ImageQuality를 되돌리지 않는다. 다음 paired 촬영이
        // 같은 값으로 다시 바꾸는 왕복이 DEVICE_BUSY를 유발했다 (HV-14). 실패 경로는
        // ClearCaptureContext가, lane 종료는 세션 종료/비-paired 촬영 진입이 복원한다.
    }

    /// <summary>
    /// DEVICE_BUSY에서 즉시 실패하지 않고 짧게 물러났다 다시 시도한다.
    /// busy는 셔터가 눌리지 않았다는 뜻이므로 재시도가 이중 촬영을 만들지 않는다.
    /// </summary>
    private async Task<uint> PressShutterWithBusyRetryAsync(CancellationToken cancellationToken)
    {
        for (var attempt = 0; ; attempt++)
        {
            var err = EDSDK.EdsSendCommand(
                _camera,
                EDSDK.CameraCommand_PressShutterButton,
                (int)EDSDK.EdsShutterButton.CameraCommand_ShutterButton_Completely
            );

            if (!ShouldRetryShutterBusy(err, attempt))
            {
                return err;
            }

            await Task.Delay(ShutterBusyRetryInterval, cancellationToken);
        }
    }

    internal static bool ShouldRetryShutterBusy(uint err, int attempt)
    {
        return err == EDSDK.EDS_ERR_DEVICE_BUSY && attempt < ShutterBusyRetryLimit;
    }

    internal static TimeSpan ApplyPairedCompletionAllowance(
        TimeSpan baseTimeout,
        int expectedObjectCount
    )
    {
        return expectedObjectCount > 1 ? baseTimeout + PairedCaptureCompletionAllowance : baseTimeout;
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
            if (!_sdkInitialized)
            {
                var initializeResult = EDSDK.EdsInitializeSDK();
                if (initializeResult != EDSDK.EDS_ERR_OK)
                {
                    UpdateFailure("error", "error", "sdk-init-failed");
                    return;
                }

                _sdkInitialized = true;
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

                var openResult = EDSDK.EdsOpenSession(camera);
                if (openResult != EDSDK.EDS_ERR_OK)
                {
                    EDSDK.EdsRelease(camera);
                    RecycleSdkIfNeeded();
                    UpdateFailure("error", "error", "session-open-failed");
                    return;
                }

                ConfigureSaveToHost(camera);

                lock (_sync)
                {
                    _camera = camera;
                    _sessionOpen = true;
                    _lastKeepAlive = DateTimeOffset.UtcNow;
                    _snapshot = new CameraSnapshot(
                        "ready",
                        "healthy",
                        "camera-ready",
                        deviceInfo.szDeviceDescription,
                        _currentCapture?.Request.RequestId
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

        var result = EDSDK.EdsSendCommand(camera, EDSDK.CameraCommand_ExtendShutDownTimer, 0);
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

        if (captureContext is null)
        {
            if (inRef != IntPtr.Zero)
            {
                TryCancelTransfer(inRef);
                EDSDK.EdsRelease(inRef);
            }

            return EDSDK.EDS_ERR_OK;
        }

        // 완료 판정은 object 단위가 아니라 request 단위다. 같은 촬영의 transfer object를
        // groupID로 묶고, 이 request의 것이 아닌 object만 사유와 함께 돌려보낸다.
        var infoResult = EDSDK.EdsGetDirectoryItemInfo(inRef, out var info);

        if (infoResult != EDSDK.EDS_ERR_OK)
        {
            // 정보를 읽지 못했다. 첫 object라면 기존과 동일하게 download 경로로 보내
            // 같은 실패(`download-info-failed`)를 내게 한다. 그 뒤 object는 버린다.
            if (captureContext.Correlator.AcceptedObjects > 0)
            {
                EmitObjectRejected(captureContext, "download-info-failed", "unknown", 0, null);

                if (inRef != IntPtr.Zero)
                {
                    TryCancelTransfer(inRef);
                    EDSDK.EdsRelease(inRef);
                }

                return EDSDK.EDS_ERR_OK;
            }

            DownloadCapture(captureContext, inRef);
            return EDSDK.EDS_ERR_OK;
        }

        var claim = captureContext.Correlator.TryClaim(
            info.GroupID,
            info.szFileName,
            info.format,
            DateTimeOffset.UtcNow
        );

        if (!claim.Accepted)
        {
            EmitObjectRejected(
                captureContext,
                claim.RejectReason ?? "object-rejected",
                claim.Role.ToString().ToLowerInvariant(),
                info.GroupID,
                info.szFileName,
                claim.ObjectIndex,
                claim.UsedFallbackCorrelation,
                claim.RoleSignal.ToString().ToLowerInvariant()
            );

            if (inRef != IntPtr.Zero)
            {
                TryCancelTransfer(inRef);
                EDSDK.EdsRelease(inRef);
            }

            return EDSDK.EDS_ERR_OK;
        }

        // JPEG object는 RAW original 경로로 절대 보내지 않는다. 확장자 기본값 fallback이
        // 정확히 그 사고를 일으킬 수 있다.
        if (claim.Role == CaptureObjectRole.Jpeg && claim.RoleSignal != CaptureObjectRoleSignal.Unknown)
        {
            DownloadPairedJpeg(captureContext, inRef, info, claim);
            return EDSDK.EDS_ERR_OK;
        }

        // Keep RAW transfer on the SDK callback path instead of hopping to an
        // arbitrary threadpool thread, which can destabilize follow-up captures.
        DownloadCapture(captureContext, inRef, info, claim);
        return EDSDK.EDS_ERR_OK;
    }

    private void EmitObjectRejected(
        CurrentCaptureContext context,
        string rejectReason,
        string role,
        uint groupId,
        string? fileName,
        int? objectIndex = null,
        bool usedFallbackCorrelation = false,
        string? roleSignal = null
    )
    {
        try
        {
            context.OnObjectRejected?.Invoke(
                new CaptureObjectRejectedResult(
                    context.Request.RequestId,
                    context.CaptureId,
                    rejectReason,
                    role,
                    objectIndex,
                    groupId,
                    usedFallbackCorrelation,
                    roleSignal,
                    fileName,
                    DateTimeOffset.UtcNow
                )
            );
        }
        catch
        {
            // 거부 알림은 best-effort다. RAW handoff가 유일한 정확성 경계로 남는다.
        }
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

    private void DownloadCapture(
        CurrentCaptureContext context,
        IntPtr directoryItem,
        EDSDK.EdsDirectoryItemInfo? knownInfo = null,
        CaptureObjectClaim? knownClaim = null
    )
    {
        IntPtr stream = IntPtr.Zero;
        var downloadCompleted = false;
        var tempPath = string.Empty;

        try
        {
            Directory.CreateDirectory(context.Paths.CapturesOriginalsDir);

            EDSDK.EdsDirectoryItemInfo info;
            if (knownInfo is not null)
            {
                info = knownInfo.Value;
            }
            else
            {
                var infoResult = EDSDK.EdsGetDirectoryItemInfo(directoryItem, out info);
                if (infoResult != EDSDK.EDS_ERR_OK)
                {
                    throw new CanonCaptureException(
                        "download-info-failed",
                        $"파일 정보를 읽지 못했어요: 0x{infoResult:x8}",
                        recoveryRequired: true
                    );
                }
            }

            var claim = knownClaim
                ?? context.Correlator.TryClaim(
                    info.GroupID,
                    info.szFileName,
                    info.format,
                    DateTimeOffset.UtcNow
                );

            if (!claim.Accepted || claim.Role != CaptureObjectRole.Raw)
            {
                EmitObjectRejected(
                    context,
                    claim.RejectReason ?? "non-raw-object-on-raw-path",
                    claim.Role.ToString().ToLowerInvariant(),
                    info.GroupID,
                    info.szFileName,
                    claim.ObjectIndex,
                    claim.UsedFallbackCorrelation,
                    claim.RoleSignal.ToString().ToLowerInvariant()
                );
                return;
            }

            // 확장자는 SDK가 준 값을 그대로 쓴다. 비어 있을 때의 기본값은 승인 하드웨어인
            // EOS 700D가 실제로 만드는 `.cr2`다. 이전 기본값 `.cr3`는 이 카메라가 만들지
            // 않는 이름이었고, object가 둘이 되면 JPEG를 RAW original로 저장할 수 있었다.
            // 기존 `.cr3` 산출물은 그대로 읽힌다 — 여기서 정하는 것은 새로 쓸 이름뿐이다.
            var extension = CaptureObjectCorrelator.ResolveRawExtension(info.szFileName);

            var captureId = context.CaptureId;
            tempPath = Path.Combine(
                context.Paths.CapturesOriginalsDir,
                $"{captureId}.downloading{extension}"
            );
            var finalPath = Path.Combine(context.Paths.CapturesOriginalsDir, $"{captureId}{extension}");

            // Try the same-capture camera thumbnail before the full RAW transfer.
            // If the SDK can provide it here, the host can overlap preview work
            // with the still-in-flight RAW download instead of waiting for RAW
            // persistence to finish first.
            var immediateFastPreview = TryCaptureImmediateFastPreview(
                context,
                directoryItem,
                captureId
            );

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

            context.RawPath = finalPath;

            if (string.IsNullOrWhiteSpace(immediateFastPreview.FastPreviewPath))
            {
                QueuePendingFastPreviewDownload(context, directoryItem, captureId, finalPath);
                directoryItem = IntPtr.Zero;
            }

            var result = new CaptureDownloadResult(
                context.Request.RequestId,
                captureId,
                finalPath,
                DateTimeOffset.UtcNow,
                immediateFastPreview.FastPreviewPath,
                immediateFastPreview.FastPreviewKind,
                claim.ObjectIndex,
                info.GroupID,
                "raw",
                claim.UsedFallbackCorrelation
            );
            context.RawResult = result;
            context.RawCompletion.TrySetResult(result);
            TryCompleteCapture(context);
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
                    captureException.RecoveryRequired
                );
                context.RawCompletion.TrySetException(captureException);
                context.Completion.TrySetException(captureException);
            }
            else
            {
                ClearCaptureContext(context, "download-failed", "recovering", true);
                var downloadException = new CanonCaptureException(
                    "download-failed",
                    error.Message,
                    recoveryRequired: true
                );
                context.RawCompletion.TrySetException(downloadException);
                context.Completion.TrySetException(downloadException);
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

    /// <summary>
    /// Route B에서 RAW와 짝지어 도착한 JPEG transfer object를 받는다.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>이 경로는 RAW 저장 경로와 완전히 분리되어 있다.</b> 여기서 무엇이 실패하거나
    /// 취소되어도 <c>context.Completion</c>을 건드리지 않으므로 이미 저장된 RAW truth의
    /// 성공 판정은 바뀌지 않는다. 예외도 밖으로 던지지 않는다 — SDK 콜백 경로다.
    /// </para>
    /// <para>
    /// 산출물은 측정 전용 디렉터리에만 쓴다. canonical preview 경로(<c>renders/previews/</c>)는
    /// 이미 booth 사진 레일에 표시되므로, 그곳에 쓰면 측정 lane이 제품 UI를 조용히 바꾼다.
    /// </para>
    /// </remarks>
    private void DownloadPairedJpeg(
        CurrentCaptureContext context,
        IntPtr directoryItem,
        EDSDK.EdsDirectoryItemInfo info,
        CaptureObjectClaim claim
    )
    {
        IntPtr stream = IntPtr.Zero;
        var tempPath = string.Empty;
        var downloadCompleted = false;

        void Reject(string reason) =>
            EmitObjectRejected(
                context,
                reason,
                "jpeg",
                info.GroupID,
                info.szFileName,
                claim.ObjectIndex,
                claim.UsedFallbackCorrelation,
                claim.RoleSignal.ToString().ToLowerInvariant()
            );

        try
        {
            // capture id는 shutter 전에 고정되어 object 도착 순서와 무관하게 공유된다.
            var captureId = context.CaptureId;
            var sourcesDir = Path.Combine(
                context.Paths.SessionRoot,
                "renders",
                "sources"
            );

            Directory.CreateDirectory(sourcesDir);

            tempPath = Path.Combine(sourcesDir, $"{captureId}-paired.downloading.jpg");
            var finalPath = Path.Combine(sourcesDir, $"{captureId}-paired.jpg");

            var streamResult = EDSDK.EdsCreateFileStream(
                tempPath,
                EDSDK.EdsFileCreateDisposition.CreateAlways,
                EDSDK.EdsAccess.ReadWrite,
                out stream
            );

            if (streamResult != EDSDK.EDS_ERR_OK)
            {
                TryCancelTransfer(directoryItem);
                Reject("paired-jpeg-stream-failed");
                return;
            }

            var downloadResult = EDSDK.EdsDownload(directoryItem, info.Size, stream);
            if (downloadResult != EDSDK.EDS_ERR_OK)
            {
                EDSDK.EdsDownloadCancel(directoryItem);
                Reject("paired-jpeg-download-failed");
                return;
            }

            var completeResult = EDSDK.EdsDownloadComplete(directoryItem);
            if (completeResult != EDSDK.EDS_ERR_OK)
            {
                TryCancelTransfer(directoryItem);
                Reject("paired-jpeg-complete-failed");
                return;
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
                Reject("paired-jpeg-empty-file");
                return;
            }

            try
            {
                context.OnPairedJpeg?.Invoke(
                    new CapturePairedJpegResult(
                        context.Request.RequestId,
                        captureId,
                        finalPath,
                        fileInfo.Length,
                        claim.ObjectIndex,
                        info.GroupID,
                        claim.UsedFallbackCorrelation,
                        claim.RoleSignal.ToString().ToLowerInvariant(),
                        DateTimeOffset.UtcNow
                    )
                );
            }
            catch
            {
                // 알림은 best-effort다.
            }
        }
        catch
        {
            if (!downloadCompleted && directoryItem != IntPtr.Zero)
            {
                EDSDK.EdsDownloadCancel(directoryItem);
            }

            Reject("paired-jpeg-exception");
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

            TryCompleteCapture(context);
        }
    }

    private static void TryCancelTransfer(IntPtr directoryItem)
    {
        if (directoryItem == IntPtr.Zero)
        {
            return;
        }

        try
        {
            EDSDK.EdsDownloadCancel(directoryItem);
        }
        catch
        {
            // SDK callback cleanup is best-effort; the reference is still released by the caller.
        }
    }

    /// <summary>
    /// 카메라의 image-quality capability descriptor를 <b>읽기만</b> 한다.
    /// </summary>
    /// <remarks>
    /// 지원 여부를 가정하지 않기 위한 probe다. 이 함수는 카메라 설정을 바꾸지 않으므로
    /// 측정 lane이 꺼져 있어도 안전하다. 실제 조합 적용과 복원은 호출자가 결정한다.
    /// </remarks>
    private ImageQualityCapability ProbeImageQualityCapability(IntPtr camera, long probedAtMicros)
    {
        if (camera == IntPtr.Zero)
        {
            return ImageQualityCapability.Unavailable(probedAtMicros);
        }

        try
        {
            int? currentValue = null;

            var currentResult = EDSDK.EdsGetPropertyData(
                camera,
                EDSDK.PropID_ImageQuality,
                0,
                out uint rawCurrentValue
            );

            if (currentResult == EDSDK.EDS_ERR_OK)
            {
                currentValue = unchecked((int)rawCurrentValue);
            }

            var descResult = EDSDK.EdsGetPropertyDesc(
                camera,
                EDSDK.PropID_ImageQuality,
                out var desc
            );

            if (descResult != EDSDK.EDS_ERR_OK || desc.NumElements <= 0)
            {
                return new ImageQualityCapability(
                    false,
                    currentValue,
                    Array.Empty<int>(),
                    false,
                    probedAtMicros
                );
            }

            var elementCount = Math.Min(desc.NumElements, desc.PropDesc?.Length ?? 0);
            var supported = new List<int>(elementCount);

            for (var index = 0; index < elementCount; index++)
            {
                supported.Add(desc.PropDesc![index]);
            }

            return new ImageQualityCapability(
                true,
                currentValue,
                supported,
                ImageQualityValue.SelectRawPlusJpegCandidate(supported) is not null,
                probedAtMicros
            );
        }
        catch
        {
            return ImageQualityCapability.Unavailable(probedAtMicros);
        }
    }

    private static bool TrySetImageQuality(IntPtr camera, int value)
    {
        if (camera == IntPtr.Zero)
        {
            return false;
        }

        try
        {
            return EDSDK.EdsSetPropertyData(
                    camera,
                    EDSDK.PropID_ImageQuality,
                    0,
                    sizeof(uint),
                    unchecked((uint)value)
                ) == EDSDK.EDS_ERR_OK;
        }
        catch
        {
            return false;
        }
    }

    /// <summary>
    /// paired 비교 lane이 유지 중이던 ImageQuality 원래 값을 카메라에 되돌린다.
    /// 복원에 실패하면 값을 계속 보유해 다음 기회(다음 촬영 진입·세션 종료)에 다시 시도한다.
    /// </summary>
    private void RestoreHeldImageQuality(CurrentCaptureContext? context)
    {
        int? held;
        lock (_sync)
        {
            held = _heldOriginalImageQuality;
            if (held is null)
            {
                return;
            }

            _heldOriginalImageQuality = null;
        }

        if (!TrySetImageQuality(_camera, held.Value))
        {
            lock (_sync)
            {
                _heldOriginalImageQuality ??= held;
            }

            try
            {
                context?.OnCameraSettingWarning?.Invoke(
                    new CaptureCameraSettingWarningResult(
                        context.Request.RequestId,
                        context.CaptureId,
                        "image-quality-restore-failed",
                        DateTimeOffset.UtcNow
                    )
                );
            }
            catch
            {
                // 설정 복원 telemetry는 advisory다. 저장된 RAW truth를 바꾸지 않는다.
            }
        }
    }

    private static void TryCompleteCapture(CurrentCaptureContext context)
    {
        if (context.RawResult is null)
        {
            return;
        }

        if (
            context.Correlator.ExpectedObjectCount == 1
            || context.Correlator.IsRequestComplete
        )
        {
            context.Completion.TrySetResult(context.RawResult);
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
        bool recoveryRequired
    )
    {
        // 요청이 실패로 끝나면 유지 중이던 측정 설정을 되돌린다. 카메라를 측정 상태로
        // 남기지 않는다는 계약은 그대로다 — 유지가 허용되는 것은 연속된 paired 촬영뿐이다.
        RestoreHeldImageQuality(context);

        lock (_sync)
        {
            if (_currentCapture == context)
            {
                _currentCapture = null;
                _snapshot = _snapshot with
                {
                    CameraState = nextCameraState,
                    HelperState = recoveryRequired ? "recovering" : "healthy",
                    DetailCode = detailCode,
                    RequestId = null,
                };
            }
        }

        if (recoveryRequired)
        {
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
        // 세션이 닫히기 전에 측정 lane이 유지하던 카메라 설정을 되돌린다. 다음 세션의
        // 제품 동작이 측정 상태의 설정에서 시작하면 안 된다.
        RestoreHeldImageQuality(null);

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
    string? FastPreviewKind,
    int ObjectIndex,
    uint GroupId,
    string ObjectRole,
    bool UsedFallbackCorrelation
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

/// <summary>
/// Route B에서 RAW와 짝지어 도착한 JPEG transfer object.
/// </summary>
/// <remarks>
/// 이 경로는 <b>RAW 저장 경로와 완전히 분리되어 있다.</b> 여기서 무엇이 실패해도
/// 이미 저장된 RAW의 성공 판정은 바뀌지 않는다.
/// 산출물은 측정 전용 디렉터리(<c>renders/sources/</c>)에만 쓰이며, booth 사진 레일에
/// 표시되는 canonical preview 경로(<c>renders/previews/</c>)를 건드리지 않는다.
/// </remarks>
internal sealed record CapturePairedJpegResult(
    string RequestId,
    string CaptureId,
    string AssetPath,
    long ByteSize,
    int ObjectIndex,
    uint GroupId,
    bool UsedFallbackCorrelation,
    string RoleSignal,
    DateTimeOffset ObservedAt
);

/// <summary>거부된 transfer object. 조용히 버리지 않기 위해 남긴다.</summary>
internal sealed record CaptureObjectRejectedResult(
    string RequestId,
    string? CaptureId,
    string RejectReason,
    string Role,
    int? ObjectIndex,
    uint GroupId,
    bool UsedFallbackCorrelation,
    string? RoleSignal,
    string? FileName,
    DateTimeOffset ObservedAt
);

internal sealed record CaptureCameraSettingWarningResult(
    string RequestId,
    string CaptureId,
    string DetailCode,
    DateTimeOffset ObservedAt
);

internal sealed record PendingFastPreviewDownload(
    CurrentCaptureContext Context,
    IntPtr DirectoryItem,
    string CaptureId,
    string RawPath
);

internal sealed class CanonCaptureException : Exception
{
    public CanonCaptureException(string detailCode, string message, bool recoveryRequired)
        : base(message)
    {
        DetailCode = detailCode;
        RecoveryRequired = recoveryRequired;
    }

    public string DetailCode { get; }
    public bool RecoveryRequired { get; }
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
        Action<CaptureFastPreviewFailedResult>? onFastPreviewFailed,
        string captureId,
        int expectedObjectCount = 1,
        Action<CapturePairedJpegResult>? onPairedJpeg = null,
        Action<CaptureObjectRejectedResult>? onObjectRejected = null,
        Action<CaptureCameraSettingWarningResult>? onCameraSettingWarning = null
    )
    {
        Paths = paths;
        Request = request;
        OnFastPreviewAttempted = onFastPreviewAttempted;
        OnFastPreviewReady = onFastPreviewReady;
        OnFastPreviewFailed = onFastPreviewFailed;
        OnPairedJpeg = onPairedJpeg;
        OnObjectRejected = onObjectRejected;
        OnCameraSettingWarning = onCameraSettingWarning;
        CaptureId = captureId;
        Correlator = new CaptureObjectCorrelator(expectedObjectCount);
        RawCompletion = new TaskCompletionSource<CaptureDownloadResult>(
            TaskCreationOptions.RunContinuationsAsynchronously
        );
        Completion = new TaskCompletionSource<CaptureDownloadResult>(
            TaskCreationOptions.RunContinuationsAsynchronously
        );
    }

    public SessionPaths Paths { get; }
    public CaptureRequestMessage Request { get; }
    public Action<CaptureFastPreviewAttemptedResult>? OnFastPreviewAttempted { get; }
    public Action<CaptureFastPreviewReadyResult>? OnFastPreviewReady { get; }
    public Action<CaptureFastPreviewFailedResult>? OnFastPreviewFailed { get; }

    /// <summary>Route B에서 짝지어 도착한 JPEG object 알림. 측정 lane에서만 채워진다.</summary>
    public Action<CapturePairedJpegResult>? OnPairedJpeg { get; }

    /// <summary>
    /// 거부된 transfer object 알림.
    /// </summary>
    /// <remarks>
    /// <b>조용히 버리지 않기 위해 존재한다.</b> 이전 구현은 두 번째 object를 아무 기록 없이
    /// <c>EdsRelease</c>했고, 그 상태로 측정하면 "카메라가 RAW+JPEG를 지원하지 않는다"는
    /// 잘못된 결론이 나온다.
    /// </remarks>
    public Action<CaptureObjectRejectedResult>? OnObjectRejected { get; }
    public Action<CaptureCameraSettingWarningResult>? OnCameraSettingWarning { get; }

    /// <summary>
    /// request 단위 완료 판정.
    /// </summary>
    /// <remarks>
    /// 이전에는 <c>DownloadStarted</c> 플래그 하나가 <i>object 단위</i>로 완료를 판정해
    /// capture당 첫 object만 받고 나머지를 버렸다. 완료 판정이 request 단위로 옮겨졌다.
    /// </remarks>
    public CaptureObjectCorrelator Correlator { get; }

    /// <summary>RAW original이 저장된 경로. JPEG object 처리에서 correlation에 쓴다.</summary>
    public string? RawPath { get; set; }

    /// <summary>이 촬영에 배정된 capture id. object 도착 순서와 무관하게 미리 고정한다.</summary>
    public string CaptureId { get; }

    public CaptureDownloadResult? RawResult { get; set; }
    public TaskCompletionSource<CaptureDownloadResult> RawCompletion { get; }
    public TaskCompletionSource<CaptureDownloadResult> Completion { get; }
}
