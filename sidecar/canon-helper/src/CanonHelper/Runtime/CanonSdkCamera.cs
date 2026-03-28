using System.Runtime.InteropServices;
using CanonHelper.Protocol;
using EDSDKLib;

namespace CanonHelper.Runtime;

internal sealed class CanonSdkCamera : IDisposable
{
    private static readonly TimeSpan MinimumSdkRecycleInterval = TimeSpan.FromSeconds(2);
    private static readonly TimeSpan KeepAliveInterval = TimeSpan.FromSeconds(5);

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

            captureContext = new CurrentCaptureContext(paths, request);
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
            ClearCaptureContext(
                captureContext,
                err == EDSDK.EDS_ERR_DEVICE_BUSY ? "camera-busy" : "capture-trigger-failed",
                err == EDSDK.EDS_ERR_DEVICE_BUSY ? "ready" : "recovering",
                err != EDSDK.EDS_ERR_DEVICE_BUSY
            );

            throw new CanonCaptureException(
                err == EDSDK.EDS_ERR_DEVICE_BUSY ? "camera-busy" : "capture-trigger-failed",
                $"셔터 명령을 보낼 수 없었어요: 0x{err:x8}",
                recoveryRequired: err != EDSDK.EDS_ERR_DEVICE_BUSY
            );
        }

        var result = await captureContext.Completion.Task.WaitAsync(cancellationToken);
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
        if (_camera == IntPtr.Zero || DateTimeOffset.UtcNow - _lastKeepAlive < KeepAliveInterval)
        {
            return;
        }

        var result = EDSDK.EdsSendCommand(_camera, EDSDK.CameraCommand_ExtendShutDownTimer, 0);
        if (result == EDSDK.EDS_ERR_OK)
        {
            _lastKeepAlive = DateTimeOffset.UtcNow;
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

        Task.Run(() => DownloadCapture(captureContext, inRef));
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

            context.Completion.TrySetResult(
                new CaptureDownloadResult(
                    context.Request.RequestId,
                    captureId,
                    finalPath,
                    DateTimeOffset.UtcNow
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
    DateTimeOffset ArrivedAt
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
    public CurrentCaptureContext(SessionPaths paths, CaptureRequestMessage request)
    {
        Paths = paths;
        Request = request;
        Completion = new TaskCompletionSource<CaptureDownloadResult>(
            TaskCreationOptions.RunContinuationsAsynchronously
        );
    }

    public SessionPaths Paths { get; }
    public CaptureRequestMessage Request { get; }
    public TaskCompletionSource<CaptureDownloadResult> Completion { get; }
    public int DownloadStarted;
}
