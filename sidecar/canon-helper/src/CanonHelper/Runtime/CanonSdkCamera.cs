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
    // Real follow-up captures on EOS 700D hardware can occasionally cross the
    // 30 second mark before the transfer boundary closes. Keep enough headroom
    // to avoid treating slow but valid RAW handoffs as fatal helper failures.
    private static readonly TimeSpan DefaultCaptureCompletionTimeout = TimeSpan.FromMilliseconds(
        45000
    );

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
        CancellationToken cancellationToken
    )
    {
        CurrentCaptureContext captureContext;

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
        }

        var err = EDSDK.EdsSendCommand(
            _camera,
            EDSDK.CameraCommand_PressShutterButton,
            (int)EDSDK.EdsShutterButton.CameraCommand_ShutterButton_Completely
        );

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

        if (captureContext is null || Interlocked.Exchange(ref captureContext.DownloadStarted, 1) == 1)
        {
            if (inRef != IntPtr.Zero)
            {
                EDSDK.EdsRelease(inRef);
            }

            return EDSDK.EDS_ERR_OK;
        }

        // Keep RAW transfer on the SDK callback path instead of hopping to an
        // arbitrary threadpool thread, which can destabilize follow-up captures.
        DownloadCapture(captureContext, inRef);
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
                    captureException.RecoveryRequired
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
        bool recoveryRequired
    )
    {
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
        Action<CaptureFastPreviewFailedResult>? onFastPreviewFailed
    )
    {
        Paths = paths;
        Request = request;
        OnFastPreviewAttempted = onFastPreviewAttempted;
        OnFastPreviewReady = onFastPreviewReady;
        OnFastPreviewFailed = onFastPreviewFailed;
        Completion = new TaskCompletionSource<CaptureDownloadResult>(
            TaskCreationOptions.RunContinuationsAsynchronously
        );
    }

    public SessionPaths Paths { get; }
    public CaptureRequestMessage Request { get; }
    public Action<CaptureFastPreviewAttemptedResult>? OnFastPreviewAttempted { get; }
    public Action<CaptureFastPreviewReadyResult>? OnFastPreviewReady { get; }
    public Action<CaptureFastPreviewFailedResult>? OnFastPreviewFailed { get; }
    public TaskCompletionSource<CaptureDownloadResult> Completion { get; }
    public int DownloadStarted;
}
