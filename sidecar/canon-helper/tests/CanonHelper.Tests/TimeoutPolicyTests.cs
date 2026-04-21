using System.Diagnostics;
using System.Reflection;
using System.Threading;
using CanonHelper.Protocol;
using CanonHelper.Runtime;
using EDSDKLib;
using Xunit;

namespace CanonHelper.Tests;

public sealed class TimeoutPolicyTests
{
    [Fact]
    public void ResolveCaptureCompletionTimeout_uses_the_new_default_headroom_without_overrides()
    {
        var runtimeRoot = Path.Combine(
            Path.GetTempPath(),
            $"boothy-helper-timeout-default-{Guid.NewGuid():N}"
        );
        Directory.CreateDirectory(runtimeRoot);
        Environment.SetEnvironmentVariable("BOOTHY_CAPTURE_TIMEOUT_MS", null);

        var timeout = ResolveCaptureCompletionTimeout(runtimeRoot);

        Assert.Equal(TimeSpan.FromMilliseconds(45_000), timeout);
    }

    [Fact]
    public void ResolveConnectionAttemptTimeout_uses_the_new_default_headroom_without_overrides()
    {
        var previousTimeout = Environment.GetEnvironmentVariable("BOOTHY_HELPER_CONNECT_TIMEOUT_MS");
        Environment.SetEnvironmentVariable("BOOTHY_HELPER_CONNECT_TIMEOUT_MS", null);

        try
        {
            var timeout = ResolveConnectionAttemptTimeout();

            Assert.Equal(TimeSpan.FromMilliseconds(15_000), timeout);
        }
        finally
        {
            Environment.SetEnvironmentVariable("BOOTHY_HELPER_CONNECT_TIMEOUT_MS", previousTimeout);
        }
    }

    [Fact]
    public async Task ForceCaptureTimeoutIfStuck_fails_an_orphaned_active_capture()
    {
        var runtimeRoot = Path.Combine(
            Path.GetTempPath(),
            $"boothy-helper-watchdog-{Guid.NewGuid():N}"
        );
        var sessionId = $"session_{Guid.NewGuid():N}";
        Directory.CreateDirectory(runtimeRoot);

        var camera = new CanonSdkCamera();
        var paths = new SessionPaths(runtimeRoot, sessionId);
        var request = new CaptureRequestMessage(
            CanonHelperSchemas.CaptureRequest,
            "request-capture",
            sessionId,
            "request_watchdog",
            DateTimeOffset.UtcNow.ToString("O"),
            "preset_soft-glow",
            "2026.03.20"
        );
        var context = new CurrentCaptureContext(paths, request, null, null, null);

        SetField(camera, "_currentCapture", context);
        SetField(
            camera,
            "_snapshot",
            new CameraSnapshot("capturing", "healthy", "capture-in-flight", null, request.RequestId)
        );

        var forced = camera.ForceCaptureTimeoutIfStuck(
            runtimeRoot,
            context.StartedAt.AddMilliseconds(45_001)
        );

        Assert.True(forced);
        var error = await Assert.ThrowsAsync<CanonCaptureException>(() => context.Completion.Task);
        Assert.Equal("capture-download-timeout", error.DetailCode);
        Assert.True(error.RecoveryRequired);

        var snapshot = camera.Snapshot;
        Assert.Equal("recovering", snapshot.CameraState);
        Assert.Equal("recovering", snapshot.HelperState);
        Assert.Equal("capture-download-timeout", snapshot.DetailCode);
        Assert.Null(snapshot.RequestId);
    }

    [Fact]
    public void HandleObjectEvent_queues_raw_download_without_blocking_the_helper_loop()
    {
        var runtimeRoot = Path.Combine(
            Path.GetTempPath(),
            $"boothy-helper-callback-{Guid.NewGuid():N}"
        );
        var sessionId = $"session_{Guid.NewGuid():N}";
        Directory.CreateDirectory(runtimeRoot);

        var camera = new CanonSdkCamera();
        var paths = new SessionPaths(runtimeRoot, sessionId);
        var request = new CaptureRequestMessage(
            CanonHelperSchemas.CaptureRequest,
            "request-capture",
            sessionId,
            "request_callback",
            DateTimeOffset.UtcNow.ToString("O"),
            "preset_soft-glow",
            "2026.03.20"
        );
        var context = new CurrentCaptureContext(paths, request, null, null, null);
        using var workerStarted = new ManualResetEventSlim(false);
        using var releaseWorker = new ManualResetEventSlim(false);

        SetField(camera, "_currentCapture", context);
        camera.SetDownloadCaptureOverrideForTests(
            (_, _) =>
            {
                workerStarted.Set();
                releaseWorker.Wait(TimeSpan.FromSeconds(2));
            }
        );

        var method = typeof(CanonSdkCamera).GetMethod(
            "HandleObjectEvent",
            BindingFlags.Instance | BindingFlags.NonPublic
        );
        Assert.NotNull(method);

        var stopwatch = Stopwatch.StartNew();
        var result = Assert.IsType<uint>(
            method!.Invoke(
                camera,
                [EDSDK.ObjectEvent_DirItemRequestTransfer, new IntPtr(1), IntPtr.Zero]
            )
        );
        stopwatch.Stop();

        Assert.Equal(EDSDK.EDS_ERR_OK, result);
        Assert.True(workerStarted.Wait(TimeSpan.FromSeconds(1)));
        Assert.True(
            stopwatch.Elapsed < TimeSpan.FromMilliseconds(200),
            $"callback blocked for {stopwatch.Elapsed.TotalMilliseconds}ms"
        );

        releaseWorker.Set();
    }

    [Fact]
    public async Task EnsureConnectedAsync_keeps_the_helper_loop_live_while_connect_attempt_runs()
    {
        var camera = new CanonSdkCamera();
        using var allowConnectAttemptToFinish = new ManualResetEventSlim(false);

        camera.SetConnectAttemptOverrideForTests(
            () =>
            {
                allowConnectAttemptToFinish.Wait(TimeSpan.FromSeconds(2));
                return true;
            }
        );

        var stopwatch = Stopwatch.StartNew();
        await camera.EnsureConnectedAsync(CancellationToken.None);
        stopwatch.Stop();

        var snapshot = camera.Snapshot;
        Assert.True(
            stopwatch.Elapsed < TimeSpan.FromMilliseconds(200),
            $"connect loop blocked for {stopwatch.Elapsed.TotalMilliseconds}ms"
        );
        Assert.Equal("connecting", snapshot.CameraState);

        for (var attempt = 0; attempt < 20; attempt += 1)
        {
            snapshot = camera.Snapshot;
            if (snapshot.DetailCode == "sdk-initializing")
            {
                break;
            }

            Thread.Sleep(10);
        }

        Assert.Equal("connecting", snapshot.CameraState);
        Assert.Equal("starting", snapshot.HelperState);
        Assert.Equal("sdk-initializing", snapshot.DetailCode);

        allowConnectAttemptToFinish.Set();
    }

    [Fact]
    public async Task EnsureConnectedAsync_escalates_to_an_explicit_error_after_connect_timeout()
    {
        var camera = new CanonSdkCamera();
        using var allowConnectAttemptToFinish = new ManualResetEventSlim(false);
        var previousTimeout = Environment.GetEnvironmentVariable("BOOTHY_HELPER_CONNECT_TIMEOUT_MS");
        Environment.SetEnvironmentVariable("BOOTHY_HELPER_CONNECT_TIMEOUT_MS", "25");

        try
        {
            camera.SetConnectAttemptOverrideForTests(
                () =>
                {
                    allowConnectAttemptToFinish.Wait(TimeSpan.FromSeconds(2));
                    return true;
                }
            );

            await camera.EnsureConnectedAsync(CancellationToken.None);
            Thread.Sleep(50);
            await camera.EnsureConnectedAsync(CancellationToken.None);

            var snapshot = camera.Snapshot;
            Assert.Equal("error", snapshot.CameraState);
            Assert.Equal("error", snapshot.HelperState);
            Assert.Equal("sdk-init-timeout", snapshot.DetailCode);
        }
        finally
        {
            allowConnectAttemptToFinish.Set();
            Environment.SetEnvironmentVariable("BOOTHY_HELPER_CONNECT_TIMEOUT_MS", previousTimeout);
        }
    }

    [Fact]
    public async Task EnsureConnectedAsync_uses_session_open_timeout_when_camera_session_opening_stalls()
    {
        var camera = new CanonSdkCamera();
        var previousTimeout = Environment.GetEnvironmentVariable("BOOTHY_HELPER_CONNECT_TIMEOUT_MS");
        Environment.SetEnvironmentVariable("BOOTHY_HELPER_CONNECT_TIMEOUT_MS", "25");

        try
        {
            SetField(camera, "_sdkInitialized", true);
            SetField(
                camera,
                "_snapshot",
                new CameraSnapshot("connecting", "connecting", "session-opening", "Canon EOS 700D", null)
            );
            SetField(camera, "_connectAttemptStartedAt", DateTimeOffset.UtcNow.AddMilliseconds(-50));
            var stalled = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
            SetField(camera, "_connectTask", stalled.Task);

            await camera.EnsureConnectedAsync(CancellationToken.None);

            var snapshot = camera.Snapshot;
            Assert.Equal("error", snapshot.CameraState);
            Assert.Equal("error", snapshot.HelperState);
            Assert.Equal("session-open-timeout", snapshot.DetailCode);
            Assert.Null(GetField<object?>(camera, "_connectTask"));
        }
        finally
        {
            Environment.SetEnvironmentVariable("BOOTHY_HELPER_CONNECT_TIMEOUT_MS", previousTimeout);
        }
    }

    [Fact]
    public async Task EnsureConnectedAsync_runs_the_connect_attempt_on_an_sta_thread()
    {
        var camera = new CanonSdkCamera();
        using var allowConnectAttemptToFinish = new ManualResetEventSlim(false);
        ApartmentState? apartmentState = null;

        camera.SetConnectAttemptOverrideForTests(
            () =>
            {
                apartmentState = Thread.CurrentThread.GetApartmentState();
                allowConnectAttemptToFinish.Wait(TimeSpan.FromSeconds(2));
                return true;
            }
        );

        await camera.EnsureConnectedAsync(CancellationToken.None);
        allowConnectAttemptToFinish.Set();
        await Task.Delay(50);

        Assert.Equal(ApartmentState.STA, apartmentState);
    }

    [Fact]
    public void PumpEvents_does_not_touch_the_sdk_before_the_camera_session_is_open()
    {
        var camera = new CanonSdkCamera();
        var invoked = false;

        SetField(camera, "_sdkInitialized", true);
        SetField(camera, "_sessionOpen", false);
        camera.SetPumpEventsOverrideForTests(
            () =>
            {
                invoked = true;
                return EDSDK.EDS_ERR_OK;
            }
        );

        camera.PumpEvents();

        Assert.False(invoked);
    }

    [Fact]
    public void BuildCaptureTriggerException_treats_internal_error_as_retryable_without_recovery()
    {
        var method = typeof(CanonSdkCamera).GetMethod(
            "BuildCaptureTriggerException",
            BindingFlags.NonPublic | BindingFlags.Static
        );

        Assert.NotNull(method);

        var error = Assert.IsType<CanonCaptureException>(
            method!.Invoke(null, [EDSDK.EDS_ERR_INTERNAL_ERROR])
        );

        Assert.Equal("capture-trigger-failed", error.DetailCode);
        Assert.False(error.RecoveryRequired);
        Assert.True(error.SessionResetRequired);
        Assert.Contains("0x00000002", error.Message);
    }

    [Fact]
    public void ClearCaptureContext_restores_camera_ready_after_retryable_failure()
    {
        var runtimeRoot = Path.Combine(
            Path.GetTempPath(),
            $"boothy-helper-retryable-status-{Guid.NewGuid():N}"
        );
        var sessionId = $"session_{Guid.NewGuid():N}";
        Directory.CreateDirectory(runtimeRoot);

        var camera = new CanonSdkCamera();
        var paths = new SessionPaths(runtimeRoot, sessionId);
        var request = new CaptureRequestMessage(
            CanonHelperSchemas.CaptureRequest,
            "request-capture",
            sessionId,
            "request_retryable_status",
            DateTimeOffset.UtcNow.ToString("O"),
            "preset_soft-glow",
            "2026.03.20"
        );
        var context = new CurrentCaptureContext(paths, request, null, null, null);

        SetField(camera, "_currentCapture", context);
        SetField(
            camera,
            "_snapshot",
            new CameraSnapshot("capturing", "healthy", "capture-in-flight", "Canon EOS 700D", request.RequestId)
        );

        var method = typeof(CanonSdkCamera).GetMethod(
            "ClearCaptureContext",
            BindingFlags.Instance | BindingFlags.NonPublic
        );

        Assert.NotNull(method);

        method!.Invoke(camera, [context, "capture-trigger-failed", "ready", false, false]);

        var snapshot = camera.Snapshot;
        Assert.Equal("ready", snapshot.CameraState);
        Assert.Equal("healthy", snapshot.HelperState);
        Assert.Equal("camera-ready", snapshot.DetailCode);
        Assert.Null(snapshot.RequestId);
    }

    [Fact]
    public void ClearCaptureContext_marks_internal_trigger_failure_for_reconnect_before_retry()
    {
        var runtimeRoot = Path.Combine(
            Path.GetTempPath(),
            $"boothy-helper-reconnect-status-{Guid.NewGuid():N}"
        );
        var sessionId = $"session_{Guid.NewGuid():N}";
        Directory.CreateDirectory(runtimeRoot);

        var camera = new CanonSdkCamera();
        var paths = new SessionPaths(runtimeRoot, sessionId);
        var request = new CaptureRequestMessage(
            CanonHelperSchemas.CaptureRequest,
            "request-capture",
            sessionId,
            "request_retryable_internal",
            DateTimeOffset.UtcNow.ToString("O"),
            "preset_soft-glow",
            "2026.03.20"
        );
        var context = new CurrentCaptureContext(paths, request, null, null, null);

        SetField(camera, "_currentCapture", context);
        SetField(camera, "_sessionOpen", true);
        SetField(
            camera,
            "_snapshot",
            new CameraSnapshot("capturing", "healthy", "capture-in-flight", "Canon EOS 700D", request.RequestId)
        );

        var method = typeof(CanonSdkCamera).GetMethod(
            "ClearCaptureContext",
            BindingFlags.Instance | BindingFlags.NonPublic
        );

        Assert.NotNull(method);

        method!.Invoke(camera, [context, "capture-trigger-failed", "ready", false, true]);

        var snapshot = camera.Snapshot;
        Assert.Equal("recovering", snapshot.CameraState);
        Assert.Equal("recovering", snapshot.HelperState);
        Assert.Equal("reconnect-pending", snapshot.DetailCode);
        Assert.Null(snapshot.RequestId);
        Assert.False(GetField<bool>(camera, "_sessionOpen"));
    }

    private static TimeSpan ResolveCaptureCompletionTimeout(string runtimeRoot)
    {
        var method = typeof(CanonSdkCamera).GetMethod(
            "ResolveCaptureCompletionTimeout",
            BindingFlags.NonPublic | BindingFlags.Static
        );

        Assert.NotNull(method);

        return Assert.IsType<TimeSpan>(method!.Invoke(null, [runtimeRoot]));
    }

    private static TimeSpan ResolveConnectionAttemptTimeout()
    {
        var method = typeof(CanonSdkCamera).GetMethod(
            "ResolveConnectionAttemptTimeout",
            BindingFlags.NonPublic | BindingFlags.Static
        );

        Assert.NotNull(method);

        return Assert.IsType<TimeSpan>(method!.Invoke(null, []));
    }

    private static void SetField(object target, string fieldName, object? value)
    {
        var field = target.GetType().GetField(fieldName, BindingFlags.Instance | BindingFlags.NonPublic);
        Assert.NotNull(field);
        field!.SetValue(target, value);
    }

    private static T? GetField<T>(object target, string fieldName)
    {
        var field = target.GetType().GetField(fieldName, BindingFlags.Instance | BindingFlags.NonPublic);
        Assert.NotNull(field);
        return (T?)field!.GetValue(target);
    }
}
