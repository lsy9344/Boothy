using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Threading;
using System.Threading.Tasks;
using Boothy.CameraSidecar.Camera.Canon;
using Boothy.CameraSidecar.IPC;
using Boothy.CameraSidecar.Logging;

namespace Boothy.CameraSidecar.Camera
{
    /// <summary>
    /// Real camera controller skeleton.
    /// Performs prerequisite checks and degrades gracefully when unavailable.
    /// </summary>
    public class RealCameraController : ICameraController, IDisposable
    {
        private readonly bool isOperational;
        private readonly string? diagnostic;
        private string? sessionDestination;
        private readonly string? edsdkResolvedPath;
        private readonly string platformLabel = Environment.Is64BitProcess ? "x64" : "x86";

        private readonly int deviceHintDebounceMs;
        private readonly int probeTimeoutMs;
        private readonly int statusChangedMinIntervalMs;

        private readonly string edsdkPumpMode;
        private readonly int pumpBoostWindowMs;
        private readonly int pumpLowIntervalMs;
        private readonly int pumpHighIntervalMs;
        private DateTime pumpBoostUntilUtc = DateTime.MinValue;

        private readonly object sdkLock = new object();
        private bool sdkInitialized;
        private string? sdkDiagnostic;
        private IntPtr cameraRef;
        private bool sessionOpen;
        private EdsdkNative.EdsObjectEventHandler? objectEventHandler;
        private EdsdkNative.EdsStateEventHandler? stateEventHandler;
        private EdsdkNative.EdsCameraAddedHandler? cameraAddedHandler;
        private CancellationTokenSource? eventPumpCts;
        private Task? eventPumpTask;
        private string? pendingTransferCorrelationId;
        private int consecutiveNoCameraCount;
        private int consecutiveUnusableEnumerationCount;
        private DateTime lastSdkResetAtUtc = DateTime.MinValue;
        private int cameraAddedSignalCount;
        private bool cameraShutdownSignal;
        private bool hotplugWatchActive;
        private bool sdkResetRequested;
        private int lastCameraCount = 0;
        private DateTime lastCameraCountChangeAt = DateTime.MinValue;
        private DateTime lastEventPumpErrorAtUtc = DateTime.MinValue;

        private readonly object probeLock = new object();
        private CancellationTokenSource? probeDebounceCts;
        private bool probeInFlight;
        private bool probeRequested;
        private string? pendingProbeReason;
        private string? pendingProbeCorrelationId;
        private long statusSeq;
        private string? lastStatusFingerprint;
        private DateTime lastStatusEmitAtUtc = DateTime.MinValue;
        private DeviceChangeWatcher? deviceChangeWatcher;

        public event EventHandler<IpcMessage>? OnPhotoTransferred;
        public event EventHandler<IpcMessage>? OnCaptureStarted;
        public event EventHandler<IpcMessage>? OnError;
        public event EventHandler<IpcMessage>? OnStatusHint;
        public event EventHandler<IpcMessage>? OnStatusChanged;

        public RealCameraController()
        {
            var (ok, details) = CheckPrerequisites();
            isOperational = ok;
            diagnostic = details;
            edsdkResolvedPath = EdsdkNative.FindEdsdkDllPath(out _);

            deviceHintDebounceMs = ReadEnvInt("BOOTHY_CAMERA_DEVICE_HINT_DEBOUNCE_MS", 500, 0, 60_000);
            probeTimeoutMs = ReadEnvInt("BOOTHY_CAMERA_PROBE_TIMEOUT_MS", 2000, 250, 60_000);
            statusChangedMinIntervalMs = ReadEnvInt("BOOTHY_CAMERA_STATUS_CHANGED_MIN_INTERVAL_MS", 250, 0, 60_000);

            edsdkPumpMode = ReadEnvString("BOOTHY_EDSDK_PUMP_MODE", "both");
            pumpBoostWindowMs = ReadEnvInt("BOOTHY_EDSDK_PUMP_BOOST_WINDOW_MS", 10_000, 0, 120_000);
            pumpLowIntervalMs = ReadEnvInt("BOOTHY_EDSDK_PUMP_LOW_INTERVAL_MS", 500, 10, 10_000);
            pumpHighIntervalMs = ReadEnvInt("BOOTHY_EDSDK_PUMP_HIGH_INTERVAL_MS", 150, 10, 10_000);

            if (!isOperational)
            {
                Logger.Warning("system", $"Real mode prerequisites missing: {diagnostic}");
                return;
            }

            if (ShouldRunMessageLoopPump())
            {
                try
                {
                    string correlationId = IpcHelpers.GenerateCorrelationId();
                    deviceChangeWatcher = new DeviceChangeWatcher(correlationId, hintReason =>
                    {
                        try
                        {
                            var corr = IpcHelpers.GenerateCorrelationId();
                            TriggerStatusProbe(corr, hintReason);
                        }
                        catch
                        {
                            // ignore
                        }
                    });
                }
                catch (Exception ex)
                {
                    Logger.Warning("system", $"Failed to start device change watcher: {ex.Message}");
                }
            }
        }

        public void SetSessionDestination(string destinationPath)
        {
            sessionDestination = destinationPath;
            Logger.Info("system", $"Session destination set to: {destinationPath}");

            if (!Directory.Exists(destinationPath))
            {
                Directory.CreateDirectory(destinationPath);
                Logger.Info("system", $"Created session destination directory: {destinationPath}");
            }

            if (!isOperational)
            {
                return;
            }

            var correlationId = IpcHelpers.GenerateCorrelationId();
            lock (sdkLock)
            {
                _ = EnsureCameraSessionOpen(correlationId);
            }
        }

        public async Task<bool> CaptureAsync(string correlationId, CancellationToken cancellationToken = default)
        {
            if (!isOperational)
            {
                EmitError(correlationId, IpcErrorCode.CameraNotConnected, diagnostic);
                return false;
            }

            if (string.IsNullOrWhiteSpace(sessionDestination))
            {
                EmitError(correlationId, IpcErrorCode.SessionDestinationNotSet, "Session destination not set.");
                return false;
            }

            IntPtr cameraHandle;
            lock (sdkLock)
            {
                if (!EnsureCameraSessionOpen(correlationId))
                {
                    EmitError(correlationId, IpcErrorCode.CameraNotConnected, sdkDiagnostic ?? diagnostic);
                    return false;
                }

                pendingTransferCorrelationId = correlationId;
                cameraHandle = cameraRef;

                DateTime startedAt = DateTime.UtcNow;
                OnCaptureStarted?.Invoke(this, IpcMessage.NewEvent(
                    "event.camera.captureStarted",
                    correlationId,
                    new CaptureStartedPayload { StartedAt = startedAt }
                ));
            }

            var err = EdsdkNative.EdsSendCommand(cameraHandle, EdsdkNative.CameraCommand_TakePicture, 0);
            if (err != EdsdkNative.EDS_ERR_OK)
            {
                lock (sdkLock)
                {
                    pendingTransferCorrelationId = null;
                    sdkDiagnostic = $"EdsSendCommand(TakePicture) failed (0x{err:X8})";
                }
                Logger.Warning(correlationId, sdkDiagnostic!);
                EmitError(correlationId, IpcErrorCode.CaptureFailed, sdkDiagnostic);
                return false;
            }

            await Task.CompletedTask;
            return true;
        }

        public CameraStatusResponse GetStatus(string correlationId)
        {
            if (!isOperational)
            {
                Logger.Info(correlationId, $"GetStatus: real prerequisites missing (dest={sessionDestination ?? "null"}) diag={diagnostic ?? "null"}");
                EmitStatusChangedSnapshot(correlationId, "probe", connected: false, cameraDetected: false, cameraReady: false, cameraCount: 0, cameraModel: null, sdkInitializedOverride: false, sdkDiagnosticOverride: diagnostic);
                return new CameraStatusResponse
                {
                    Connected = false,
                    CameraDetected = false,
                    SessionDestination = sessionDestination,
                    CameraModel = string.IsNullOrWhiteSpace(diagnostic) ? null : diagnostic,
                };
            }

            Logger.Debug(
                correlationId,
                $"GetStatus: begin (dest={sessionDestination ?? "null"}) sdkInitialized={sdkInitialized} sessionOpen={sessionOpen} cameraRef={(cameraRef == IntPtr.Zero ? "0" : "nonzero")}"
            );

            bool connected = EnsureSdkInitialized(correlationId);
            if (!connected)
            {
                Logger.Warning(
                    correlationId,
                    $"GetStatus: SDK not initialized. sdkDiagnostic={sdkDiagnostic ?? "null"}"
                );
                EmitStatusChangedSnapshot(correlationId, "probe", connected: false, cameraDetected: false, cameraReady: false, cameraCount: 0, cameraModel: null, sdkInitializedOverride: false, sdkDiagnosticOverride: sdkDiagnostic);
                return new CameraStatusResponse
                {
                    Connected = false,
                    CameraDetected = false,
                    SessionDestination = sessionDestination,
                    CameraModel = string.IsNullOrWhiteSpace(sdkDiagnostic) ? null : sdkDiagnostic,
                };
            }

            CanonProbeResult probeResult;
            lock (sdkLock)
            {
                try
                {
                    if (sdkResetRequested && CanResetSdkUnsafe())
                    {
                        Logger.Warning(correlationId, "GetStatus: performing requested EDSDK reset");
                        sdkResetRequested = false;
                        ResetSdkUnsafe(correlationId);
                        _ = EnsureSdkInitialized(correlationId);
                    }

                    if (cameraShutdownSignal)
                    {
                        cameraShutdownSignal = false;
                        Logger.Warning(correlationId, "GetStatus: received camera shutdown signal; closing session");
                        CloseCameraSession();

                        // Many Canon bodies/driver stacks require a full SDK reset after power-off/hot-unplug
                        // to resume enumeration reliably.
                        sdkResetRequested = true;
                    }

                    // If we already have an open session, validate it first.
                    if (sessionOpen && cameraRef != IntPtr.Zero)
                    {
                        var err = EdsdkNative.EdsGetDeviceInfo(cameraRef, out var deviceInfo);
                        if (err == EdsdkNative.EDS_ERR_OK)
                        {
                            var model = string.IsNullOrWhiteSpace(deviceInfo.szDeviceDescription)
                                ? null
                                : deviceInfo.szDeviceDescription.Trim();

                            sdkDiagnostic = null;
                            Logger.Info(
                                correlationId,
                                $"GetStatus: session already open -> detected model={(model ?? "unknown")}"
                            );
                            consecutiveNoCameraCount = 0;
                            hotplugWatchActive = false;
                            EmitStatusChangedSnapshot(correlationId, "probe", connected: true, cameraDetected: true, cameraReady: true, cameraCount: Math.Max(1, lastCameraCount), cameraModel: model);
                            return new CameraStatusResponse
                            {
                                Connected = true,
                                CameraDetected = true,
                                SessionDestination = sessionDestination,
                                CameraModel = model,
                            };
                        }

                        sdkDiagnostic = $"EdsGetDeviceInfo failed on open session (0x{err:X8})";
                        Logger.Warning(correlationId, $"GetStatus: {sdkDiagnostic}. Closing session and probing again.");
                        CloseCameraSession();
                    }

                    // Stronger check: if any camera is present, attempt to open a session (detects "USB connected but camera off/busy" cases).
                    // This also helps recovery after power-cycle/hot-plug by forcing a fresh session open when possible.
                    if (EnsureCameraSessionOpen(correlationId))
                    {
                        var err = EdsdkNative.EdsGetDeviceInfo(cameraRef, out var deviceInfo);
                        if (err == EdsdkNative.EDS_ERR_OK)
                        {
                            var model = string.IsNullOrWhiteSpace(deviceInfo.szDeviceDescription)
                                ? null
                                : deviceInfo.szDeviceDescription.Trim();

                            sdkDiagnostic = null;
                            Logger.Info(correlationId, $"GetStatus: session open succeeded -> detected model={(model ?? "unknown")}");
                            consecutiveNoCameraCount = 0;
                            hotplugWatchActive = false;
                            EmitStatusChangedSnapshot(correlationId, "probe", connected: true, cameraDetected: true, cameraReady: true, cameraCount: Math.Max(1, lastCameraCount), cameraModel: model);
                            return new CameraStatusResponse
                            {
                                Connected = true,
                                CameraDetected = true,
                                SessionDestination = sessionDestination,
                                CameraModel = model,
                            };
                        }

                        sdkDiagnostic = $"EdsGetDeviceInfo failed after session open (0x{err:X8})";
                        Logger.Warning(correlationId, $"GetStatus: {sdkDiagnostic}. Closing session.");
                        CloseCameraSession();
                    }

                    probeResult = CanonEdsdkProbe.ProbeFirstCamera();
                    if (!string.IsNullOrWhiteSpace(probeResult.Diagnostic))
                    {
                        sdkDiagnostic = probeResult.Diagnostic;
                        Logger.Warning(correlationId, $"GetStatus: Canon EDSDK probe diagnostic: {sdkDiagnostic}");
                    }

                    Logger.Info(
                        correlationId,
                        $"GetStatus: probe result detected={probeResult.CameraDetected} count={probeResult.CameraCount} model={(probeResult.CameraModel ?? "null")}"
                    );

                    // Detect camera reconnection (camera count changed from 0 to 1+)
                    // This helps when EDSDK event pump misses cameraAdded events
                    var currentCameraCount = probeResult.CameraCount;
                    if (lastCameraCount == 0 && currentCameraCount > 0)
                    {
                        var now = DateTime.UtcNow;
                        if ((now - lastCameraCountChangeAt).TotalSeconds > 1)
                        {
                            // Debounce: prevent duplicate emits within 1 second
                            lastCameraCountChangeAt = now;
                            Logger.Info(correlationId, $"Camera reconnection detected: count {lastCameraCount} -> {currentCameraCount}");
                            EmitStatusHint(correlationId, "cameraReconnected");
                        }
                    }
                    else if (lastCameraCount > 0 && currentCameraCount == 0)
                    {
                        var now = DateTime.UtcNow;
                        if ((now - lastCameraCountChangeAt).TotalSeconds > 1)
                        {
                            // Debounce: prevent duplicate emits within 1 second
                            lastCameraCountChangeAt = now;
                            Logger.Info(correlationId, $"Camera disconnect detected: count {lastCameraCount} -> {currentCameraCount}");
                            EmitStatusHint(correlationId, "cameraDisconnected");
                        }
                    }
                    lastCameraCount = currentCameraCount;

                    if (probeResult.CameraCount <= 0)
                    {
                        consecutiveNoCameraCount += 1;
                        Logger.Info(
                            correlationId,
                            $"GetStatus: no camera reported (count=0). consecutiveNoCameraCount={consecutiveNoCameraCount}"
                        );

                        // If the camera is powered on after app start, EDSDK can occasionally lag in enumeration.
                        // Force a safe SDK reset after a few consecutive empty enumerations to recover.
                        if (ShouldResetSdkAfterNoCamera())
                        {
                            Logger.Warning(correlationId, "GetStatus: forcing EDSDK reset due to repeated no-camera results");
                            ResetSdkUnsafe(correlationId);

                            // Re-initialize and re-probe once immediately.
                            if (EnsureSdkInitialized(correlationId))
                            {
                                var retry = CanonEdsdkProbe.ProbeFirstCamera();
                                Logger.Info(
                                    correlationId,
                                    $"GetStatus: after reset probe detected={retry.CameraDetected} count={retry.CameraCount} model={(retry.CameraModel ?? "null")} diag={(retry.Diagnostic ?? "null")}"
                                );
                                probeResult = retry;
                            }
                        }
                    }
                    else
                    {
                        consecutiveNoCameraCount = 0;
                    }

                    if (!probeResult.CameraDetected && string.IsNullOrWhiteSpace(sdkDiagnostic))
                    {
                        sdkDiagnostic =
                            "No camera detected via Canon EDSDK. " +
                            "Check camera USB mode (PTP/EOS Utility), cable, and Canon driver stack.";
                    }
                }
                catch (Exception ex)
                {
                    sdkDiagnostic = $"Canon EDSDK probe exception: {ex.GetType().Name}: {ex.Message}";
                    Logger.Error(correlationId, $"GetStatus: {sdkDiagnostic}", ex);
                    probeResult = new CanonProbeResult(false, 0, null, sdkDiagnostic);
                }
            }

            // NOTE: Some Canon bodies remain enumerated (count>0) even after power-off/hot-unplug,
            // but will fail to open a session. For Boothy UX, "cameraDetected" must mean "usable".
            // If we reached here, session open failed above, so treat enumerated cameras as not ready.
            var cameraDetected = probeResult.CameraDetected;
            if (probeResult.CameraCount > 0)
            {
                cameraDetected = false;
                if (string.IsNullOrWhiteSpace(sdkDiagnostic))
                {
                    sdkDiagnostic =
                        "Camera enumerated via Canon EDSDK but could not open a session. " +
                        "Camera may be powered off, busy (EOS Utility), or in an incompatible USB mode.";
                }

                Logger.Warning(
                    correlationId,
                    $"GetStatus: camera enumerated (count={probeResult.CameraCount}, model={(probeResult.CameraModel ?? "null")}) but session open failed -> reporting cameraDetected=false. diag={(sdkDiagnostic ?? "null")}"
                );

                // Recovery: after a power-cycle/hot-plug, Canon EDSDK can get stuck in an "enumerated but unusable"
                // state until the SDK is fully reset (Terminate + Initialize). Do a throttled reset after a couple
                // consecutive occurrences to restore detection without requiring an app restart.
                bool recovered = false;
                string? recoveredModel = null;
                lock (sdkLock)
                {
                    consecutiveUnusableEnumerationCount += 1;
                    if (ShouldResetSdkAfterUnusableEnumeration())
                    {
                        Logger.Warning(
                            correlationId,
                            "GetStatus: forcing EDSDK reset due to repeated enumerated-but-unusable results"
                        );
                        ResetSdkUnsafe(correlationId);

                        if (EnsureSdkInitialized(correlationId) && EnsureCameraSessionOpen(correlationId))
                        {
                            var err = EdsdkNative.EdsGetDeviceInfo(cameraRef, out var deviceInfo);
                            if (err == EdsdkNative.EDS_ERR_OK)
                            {
                                var model = string.IsNullOrWhiteSpace(deviceInfo.szDeviceDescription)
                                    ? null
                                    : deviceInfo.szDeviceDescription.Trim();

                                recovered = true;
                                recoveredModel = model;
                                sdkDiagnostic = null;
                                consecutiveUnusableEnumerationCount = 0;
                                consecutiveNoCameraCount = 0;
                                hotplugWatchActive = false;
                            }
                            else
                            {
                                sdkDiagnostic = $"EdsGetDeviceInfo failed after reset (0x{err:X8})";
                                Logger.Warning(correlationId, sdkDiagnostic);
                                CloseCameraSession();
                            }
                        }
                    }
                }

                if (recovered)
                {
                    Logger.Info(
                        correlationId,
                        $"GetStatus: recovery succeeded after reset -> detected model={(recoveredModel ?? "unknown")}"
                    );
                    lock (sdkLock)
                    {
                        hotplugWatchActive = false;
                    }
                    EmitStatusChangedSnapshot(correlationId, "sdkReset", connected: true, cameraDetected: true, cameraReady: true, cameraCount: Math.Max(1, lastCameraCount), cameraModel: recoveredModel);
                    return new CameraStatusResponse
                    {
                        Connected = true,
                        CameraDetected = true,
                        SessionDestination = sessionDestination,
                        CameraModel = recoveredModel,
                    };
                }
            }
            else
            {
                lock (sdkLock)
                {
                    consecutiveUnusableEnumerationCount = 0;
                }
            }

            EmitStatusChangedSnapshot(
                correlationId,
                "probe",
                connected: true,
                cameraDetected: cameraDetected,
                cameraReady: cameraDetected && probeResult.CameraCount > 0,
                cameraCount: probeResult.CameraCount,
                cameraModel: probeResult.CameraModel ?? sdkDiagnostic
            );

            return new CameraStatusResponse
            {
                Connected = true,
                CameraDetected = cameraDetected,
                SessionDestination = sessionDestination,
                CameraModel = probeResult.CameraModel ?? sdkDiagnostic
            };
        }

        public void TriggerStatusProbe(string correlationId, string reason)
        {
            if (!isOperational)
            {
                EmitStatusChangedSnapshot(correlationId, reason, connected: false, cameraDetected: false, cameraReady: false, cameraCount: 0, cameraModel: null, sdkInitializedOverride: false, sdkDiagnosticOverride: diagnostic);
                return;
            }

            BoostPump(reason);
            ScheduleProbe(correlationId, reason);
        }

        private bool ShouldResetSdkAfterNoCamera()
        {
            if (consecutiveNoCameraCount < 2)
            {
                return false;
            }

            // If Windows reports no imaging devices, an EDSDK reset is unlikely to help and can
            // prolong the disconnected/reconnect loop. Wait for a device add signal instead.
            if (!ImagingDeviceProbe.IsCanonImagingDevicePresent())
            {
                return false;
            }

            // Avoid resets while an active session exists or a transfer is pending.
            if (sessionOpen || cameraRef != IntPtr.Zero || !string.IsNullOrWhiteSpace(pendingTransferCorrelationId))
            {
                return false;
            }

            // Throttle resets (Canon SDK resets are expensive but sometimes required for recovery).
            return (DateTime.UtcNow - lastSdkResetAtUtc).TotalSeconds > 3;
        }

        private bool ShouldResetSdkAfterUnusableEnumeration()
        {
            if (consecutiveUnusableEnumerationCount < 2)
            {
                return false;
            }

            // Avoid resets while an active session exists or a transfer is pending.
            if (sessionOpen || cameraRef != IntPtr.Zero || !string.IsNullOrWhiteSpace(pendingTransferCorrelationId))
            {
                return false;
            }

            // Throttle resets (share the same reset clock).
            return (DateTime.UtcNow - lastSdkResetAtUtc).TotalSeconds > 3;
        }

        private void ResetSdkUnsafe(string correlationId)
        {
            lastSdkResetAtUtc = DateTime.UtcNow;
            consecutiveNoCameraCount = 0;
            consecutiveUnusableEnumerationCount = 0;
            cameraShutdownSignal = false;
            pendingTransferCorrelationId = null;
            sdkResetRequested = false;

            CloseCameraSession();
            StopEventPumpUnsafe();

            if (!sdkInitialized)
            {
                sdkDiagnostic = null;
                return;
            }

            try
            {
                var err = EdsdkNative.EdsTerminateSDK();
                if (err != EdsdkNative.EDS_ERR_OK)
                {
                    Logger.Warning(correlationId, $"EdsTerminateSDK failed during reset (0x{err:X8})");
                }
            }
            catch (Exception ex)
            {
                Logger.Warning(correlationId, $"EdsTerminateSDK threw during reset: {ex.Message}");
            }
            finally
            {
                sdkInitialized = false;
                sdkDiagnostic = null;
            }

            try
            {
                Thread.Sleep(250);
            }
            catch
            {
                // ignore
            }
        }

        private void StartEventPumpUnsafe(string correlationId)
        {
            if (eventPumpTask != null)
            {
                return;
            }

            eventPumpCts = new CancellationTokenSource();
            var token = eventPumpCts.Token;

            Logger.Info(correlationId, "Starting Canon EDSDK event pump (EdsGetEvent loop)");

            eventPumpTask = Task.Run(async () =>
            {
                while (!token.IsCancellationRequested)
                {
                    bool shouldPoll = false;
                    int pollDelayMs = pumpLowIntervalMs;
                    lock (sdkLock)
                    {
                        if (!sdkInitialized)
                        {
                            break;
                        }

                        // Canon hotplug/power-cycle relies on EDSDK callbacks being pumped via EdsGetEvent().
                        // We poll when:
                        //  - an active session exists, or
                        //  - we're in "hotplug watch" mode after a shutdown/internal error (so we can receive cameraAdded).
                        shouldPoll = (sessionOpen && cameraRef != IntPtr.Zero) || hotplugWatchActive;
                        // Always poll fast while a session is open; otherwise use a boost window after hints.
                        if (sessionOpen && cameraRef != IntPtr.Zero)
                        {
                            pollDelayMs = pumpHighIntervalMs;
                        }
                        else
                        {
                            pollDelayMs = ShouldBoostPump() ? pumpHighIntervalMs : pumpLowIntervalMs;
                        }
                    }

                    if (shouldPoll)
                    {
                        var err = EdsdkNative.EdsGetEvent();
                        if (err != EdsdkNative.EDS_ERR_OK)
                        {
                            lock (sdkLock)
                            {
                                cameraShutdownSignal = true;
                                hotplugWatchActive = true;
                                sdkResetRequested = true;
                                pendingTransferCorrelationId = null;
                                CloseCameraSession();
                            }
                            // Throttle to avoid log spam while the camera/driver is in a transient state.
                            var now = DateTime.UtcNow;
                            if ((now - lastEventPumpErrorAtUtc).TotalSeconds >= 2)
                            {
                                lastEventPumpErrorAtUtc = now;
                                Logger.Warning("system", $"EDSDK event pump error (EdsGetEvent=0x{err:X8}); marking camera as shutdown");
                            }
                        }
                    }

                    try
                    {
                        await Task.Delay(pollDelayMs, token);
                    }
                    catch
                    {
                        break;
                    }
                }
            }, token);
        }

        private void StopEventPumpUnsafe()
        {
            try
            {
                eventPumpCts?.Cancel();
            }
            catch
            {
                // ignore
            }
            finally
            {
                eventPumpCts?.Dispose();
                eventPumpCts = null;
                eventPumpTask = null;
            }
        }

        private bool EnsureCameraSessionOpen(string correlationId)
        {
            if (!EnsureSdkInitialized(correlationId))
            {
                return false;
            }

            if (sessionOpen && cameraRef != IntPtr.Zero)
            {
                return true;
            }

            CloseCameraSession();

            IntPtr cameraListRef = IntPtr.Zero;
            IntPtr newCameraRef = IntPtr.Zero;

            try
            {
                var err = EdsdkNative.EdsGetCameraList(out cameraListRef);
                if (err != EdsdkNative.EDS_ERR_OK || cameraListRef == IntPtr.Zero)
                {
                    sdkDiagnostic = $"EdsGetCameraList failed (0x{err:X8})";
                    Logger.Warning(correlationId, sdkDiagnostic);
                    return false;
                }

                err = EdsdkNative.EdsGetChildCount(cameraListRef, out var count);
                if (err != EdsdkNative.EDS_ERR_OK)
                {
                    sdkDiagnostic = $"EdsGetChildCount failed (0x{err:X8})";
                    Logger.Warning(correlationId, sdkDiagnostic);
                    return false;
                }

                if (count <= 0)
                {
                    sdkDiagnostic = "EdsGetChildCount returned 0 cameras";
                    return false;
                }

                err = EdsdkNative.EdsGetChildAtIndex(cameraListRef, 0, out newCameraRef);
                if (err != EdsdkNative.EDS_ERR_OK || newCameraRef == IntPtr.Zero)
                {
                    sdkDiagnostic = $"EdsGetChildAtIndex(0) failed (0x{err:X8})";
                    Logger.Warning(correlationId, sdkDiagnostic);
                    return false;
                }

                err = EdsdkNative.EdsOpenSession(newCameraRef);
                if (err != EdsdkNative.EDS_ERR_OK)
                {
                    sdkDiagnostic = $"EdsOpenSession failed (0x{err:X8})";
                    Logger.Warning(correlationId, sdkDiagnostic);
                    return false;
                }

                cameraRef = newCameraRef;
                newCameraRef = IntPtr.Zero;
                sessionOpen = true;

                objectEventHandler ??= HandleObjectEvent;
                err = EdsdkNative.EdsSetObjectEventHandler(
                    cameraRef,
                    EdsdkNative.ObjectEvent_All,
                    objectEventHandler,
                    IntPtr.Zero
                );
                if (err != EdsdkNative.EDS_ERR_OK)
                {
                    sdkDiagnostic = $"EdsSetObjectEventHandler failed (0x{err:X8})";
                    Logger.Warning(correlationId, sdkDiagnostic);
                }

                stateEventHandler ??= HandleStateEvent;
                err = EdsdkNative.EdsSetCameraStateEventHandler(
                    cameraRef,
                    EdsdkNative.StateEvent_All,
                    stateEventHandler,
                    IntPtr.Zero
                );
                if (err != EdsdkNative.EDS_ERR_OK)
                {
                    sdkDiagnostic = $"EdsSetCameraStateEventHandler failed (0x{err:X8})";
                    Logger.Warning(correlationId, sdkDiagnostic);
                }

                // Prefer Host storage destination so camera transfers to PC when possible.
                try
                {
                    int saveTo = (int)EdsdkNative.EdsSaveTo.Host;
                    err = EdsdkNative.EdsSetPropertyData(
                        cameraRef,
                        EdsdkNative.PropID_SaveTo,
                        0,
                        Marshal.SizeOf<int>(),
                        saveTo
                    );
                    if (err != EdsdkNative.EDS_ERR_OK)
                    {
                        Logger.Warning(correlationId, $"EdsSetPropertyData(SaveTo=Host) failed (0x{err:X8})");
                    }

                    var capacity = new EdsdkNative.EdsCapacity
                    {
                        NumberOfFreeClusters = 0x7FFFFFFF,
                        BytesPerSector = 0x1000,
                        Reset = 1
                    };
                    err = EdsdkNative.EdsSetCapacity(cameraRef, capacity);
                    if (err != EdsdkNative.EDS_ERR_OK)
                    {
                        Logger.Warning(correlationId, $"EdsSetCapacity failed (0x{err:X8})");
                    }
                }
                catch (Exception ex)
                {
                    Logger.Warning(correlationId, $"Failed to configure SaveTo/Capacity: {ex.Message}");
                }

                sdkDiagnostic = null;
                Logger.Info(correlationId, "Canon camera session opened and object event handler registered");
                return true;
            }
            catch (Exception ex)
            {
                sdkDiagnostic = $"Canon session open exception: {ex.GetType().Name}: {ex.Message}";
                Logger.Error(correlationId, sdkDiagnostic, ex);
                return false;
            }
            finally
            {
                if (newCameraRef != IntPtr.Zero)
                {
                    _ = EdsdkNative.EdsRelease(newCameraRef);
                }
                if (cameraListRef != IntPtr.Zero)
                {
                    _ = EdsdkNative.EdsRelease(cameraListRef);
                }
            }
        }

        private uint HandleObjectEvent(uint objectEvent, IntPtr sender, IntPtr context)
        {
            if (sender == IntPtr.Zero)
            {
                return EdsdkNative.EDS_ERR_OK;
            }

            string? sessionDestinationSnapshot;
            string? correlationIdForTransfer = null;
            string? originalFilename = null;
            string? finalPath = null;
            string? tmpPath = null;
            long fileSize = 0;

            try
            {
                if (objectEvent != EdsdkNative.ObjectEvent_DirItemRequestTransfer)
                {
                    return EdsdkNative.EDS_ERR_OK;
                }

                lock (sdkLock)
                {
                    sessionDestinationSnapshot = sessionDestination;
                    correlationIdForTransfer = pendingTransferCorrelationId ?? IpcHelpers.GenerateCorrelationId();
                    pendingTransferCorrelationId = null;
                }

                if (string.IsNullOrWhiteSpace(sessionDestinationSnapshot))
                {
                    EmitError(correlationIdForTransfer!, IpcErrorCode.SessionDestinationNotSet, "Session destination not set.");
                    return EdsdkNative.EDS_ERR_OK;
                }

                lock (sdkLock)
                {
                    var err = EdsdkNative.EdsGetDirectoryItemInfo(sender, out var directoryItemInfo);
                    if (err != EdsdkNative.EDS_ERR_OK)
                    {
                        EmitError(
                            correlationIdForTransfer!,
                            IpcErrorCode.FileTransferFailed,
                            $"EdsGetDirectoryItemInfo failed (0x{err:X8})"
                        );
                        return EdsdkNative.EDS_ERR_OK;
                    }

                    originalFilename = directoryItemInfo.szFileName?.Trim();
                    if (string.IsNullOrWhiteSpace(originalFilename))
                    {
                        originalFilename = $"CAPTURE_{DateTime.UtcNow:yyyyMMdd_HHmmss}.CR3";
                    }

                    finalPath = MakeUniqueDestinationPath(sessionDestinationSnapshot!, originalFilename);
                    tmpPath = finalPath + ".tmp";
                    fileSize = (long)directoryItemInfo.Size;

                    IntPtr streamRef = IntPtr.Zero;
                    try
                    {
                        err = EdsdkNative.EdsCreateFileStream(
                            tmpPath,
                            EdsdkNative.EdsFileCreateDisposition.CreateAlways,
                            EdsdkNative.EdsAccess.ReadWrite,
                            out streamRef
                        );
                        if (err != EdsdkNative.EDS_ERR_OK || streamRef == IntPtr.Zero)
                        {
                            EmitError(
                                correlationIdForTransfer!,
                                IpcErrorCode.FileTransferFailed,
                                $"EdsCreateFileStream failed (0x{err:X8})"
                            );
                            return EdsdkNative.EDS_ERR_OK;
                        }

                        err = EdsdkNative.EdsDownload(sender, directoryItemInfo.Size, streamRef);
                        if (err != EdsdkNative.EDS_ERR_OK)
                        {
                            EmitError(
                                correlationIdForTransfer!,
                                IpcErrorCode.FileTransferFailed,
                                $"EdsDownload failed (0x{err:X8})"
                            );
                            return EdsdkNative.EDS_ERR_OK;
                        }

                        err = EdsdkNative.EdsDownloadComplete(sender);
                        if (err != EdsdkNative.EDS_ERR_OK)
                        {
                            EmitError(
                                correlationIdForTransfer!,
                                IpcErrorCode.FileTransferFailed,
                                $"EdsDownloadComplete failed (0x{err:X8})"
                            );
                            return EdsdkNative.EDS_ERR_OK;
                        }

                    }
                    finally
                    {
                        if (streamRef != IntPtr.Zero)
                        {
                            _ = EdsdkNative.EdsRelease(streamRef);
                        }
                    }
                }

                // At this point the SDK stream has been released, so the file is fully written.
                File.Move(tmpPath!, finalPath!, overwrite: true);

                var transferredAt = DateTime.UtcNow;
                OnPhotoTransferred?.Invoke(this, IpcMessage.NewEvent(
                    "event.camera.photoTransferred",
                    correlationIdForTransfer!,
                    new PhotoTransferredPayload
                    {
                        Path = finalPath!,
                        TransferredAt = transferredAt,
                        OriginalFilename = originalFilename!,
                        FileSize = fileSize
                    }
                ));

                Logger.Info(correlationIdForTransfer!, $"Photo transferred: {originalFilename} -> {finalPath}");
                return EdsdkNative.EDS_ERR_OK;
            }
            catch (Exception ex)
            {
                var correlationId = correlationIdForTransfer ?? IpcHelpers.GenerateCorrelationId();
                Logger.Error(correlationId, "Error handling Canon object event", ex);
                EmitError(correlationId, IpcErrorCode.FileTransferFailed, ex.Message);
                return EdsdkNative.EDS_ERR_OK;
            }
            finally
            {
                lock (sdkLock)
                {
                    _ = EdsdkNative.EdsRelease(sender);
                }
            }
        }

        private static string MakeUniqueDestinationPath(string destinationDir, string filename)
        {
            var safeName = filename;
            var baseName = Path.GetFileNameWithoutExtension(safeName);
            var ext = Path.GetExtension(safeName);

            var candidate = Path.Combine(destinationDir, safeName);
            if (!File.Exists(candidate))
            {
                return candidate;
            }

            var suffix = DateTime.UtcNow.ToString("yyyyMMdd_HHmmss_fff");
            var uniqueName = $"{baseName}_{suffix}{ext}";
            return Path.Combine(destinationDir, uniqueName);
        }

        private void CloseCameraSession()
        {
            if (cameraRef == IntPtr.Zero)
            {
                sessionOpen = false;
                return;
            }

            try
            {
                if (sessionOpen)
                {
                    _ = EdsdkNative.EdsCloseSession(cameraRef);
                }
            }
            catch
            {
                // Ignore close failures.
            }
            finally
            {
                _ = EdsdkNative.EdsRelease(cameraRef);
                cameraRef = IntPtr.Zero;
                sessionOpen = false;
            }
        }

        private uint HandleStateEvent(uint inEvent, uint inParameter, IntPtr context)
        {
            try
            {
                if (inEvent == EdsdkNative.StateEvent_Shutdown)
                {
                    var correlationId = IpcHelpers.GenerateCorrelationId();
                    lock (sdkLock)
                    {
                        cameraShutdownSignal = true;
                        hotplugWatchActive = true;
                        pendingTransferCorrelationId = null;
                        sdkResetRequested = true;
                        CloseCameraSession();
                    }
                    Logger.Warning("system", "Canon state event: Shutdown (camera powered off/disconnected)");
                    EmitStatusHint(correlationId, "shutdown");
                    TriggerStatusProbe(correlationId, "shutdown");
                }
                else if (inEvent == EdsdkNative.StateEvent_InternalError)
                {
                    var correlationId = IpcHelpers.GenerateCorrelationId();
                    lock (sdkLock)
                    {
                        cameraShutdownSignal = true;
                        hotplugWatchActive = true;
                        pendingTransferCorrelationId = null;
                        sdkResetRequested = true;
                        CloseCameraSession();
                    }
                    Logger.Warning("system", $"Canon state event: InternalError (param={inParameter})");
                    EmitStatusHint(correlationId, "internalError");
                    TriggerStatusProbe(correlationId, "sdkReset");
                }
            }
            catch
            {
                // Ignore state handler exceptions.
            }
            return EdsdkNative.EDS_ERR_OK;
        }

        private bool CanResetSdkUnsafe()
        {
            if (sessionOpen || cameraRef != IntPtr.Zero || !string.IsNullOrWhiteSpace(pendingTransferCorrelationId))
            {
                return false;
            }

            // Avoid thrashing resets.
            return (DateTime.UtcNow - lastSdkResetAtUtc).TotalSeconds > 1;
        }

        private uint HandleCameraAddedEvent(IntPtr context)
        {
            try
            {
                var correlationId = IpcHelpers.GenerateCorrelationId();
                lock (sdkLock)
                {
                    cameraAddedSignalCount += 1;
                    consecutiveNoCameraCount = 3; // trigger reset path quickly if enumeration is stale
                    hotplugWatchActive = true;
                }
                Logger.Info("system", "Canon camera added event received");
                EmitStatusHint(correlationId, "cameraAdded");
                TriggerStatusProbe(correlationId, "cameraAdded");
            }
            catch
            {
                // ignore
            }
            return EdsdkNative.EDS_ERR_OK;
        }

        private void EmitStatusHint(string correlationId, string reason)
        {
            try
            {
                OnStatusHint?.Invoke(
                    this,
                    IpcMessage.NewEvent(
                        "event.camera.statusHint",
                        correlationId,
                        new
                        {
                            reason,
                            occurredAt = DateTime.UtcNow,
                        }
                    )
                );
            }
            catch
            {
                // Ignore event emission failures.
            }
        }

        private bool EnsureSdkInitialized(string correlationId)
        {
            lock (sdkLock)
            {
                if (sdkInitialized)
                {
                    // State/shutdown handling may stop the event pump. If the SDK is still initialized
                    // but the pump was stopped, restart it so camera hot-plug/power-cycle events can
                    // be observed again.
                    if (eventPumpTask == null)
                    {
                        if (ShouldRunEdsGetEventPump())
                        {
                            StartEventPumpUnsafe(correlationId);
                        }
                    }
                    return true;
                }

                try
                {
                    var resolved = EdsdkNative.FindEdsdkDllPath(out var details);
                    if (!string.IsNullOrWhiteSpace(resolved))
                    {
                        try
                        {
                            var dir = Path.GetDirectoryName(resolved);
                            if (!string.IsNullOrWhiteSpace(dir))
                            {
                                var edsImage = Path.Combine(dir, "EdsImage.dll");
                                Logger.Info(
                                    correlationId,
                                    $"EDSDK companions: EdsImage.dll={(File.Exists(edsImage) ? "present" : "missing")} at {edsImage}"
                                );
                            }
                        }
                        catch
                        {
                            // Ignore companion probing errors.
                        }
                    }
                    Logger.Info(
                        correlationId,
                        $"Initializing Canon EDSDK... ResolvedPath={(string.IsNullOrWhiteSpace(resolved) ? "none" : resolved)} Details={(string.IsNullOrWhiteSpace(details) ? "null" : details)}"
                    );

                    var err = EdsdkNative.EdsInitializeSDK();
                    if (err != EdsdkNative.EDS_ERR_OK)
                    {
                        sdkDiagnostic = $"EdsInitializeSDK failed (0x{err:X8})";
                        Logger.Warning(correlationId, sdkDiagnostic);
                        return false;
                    }

                    cameraAddedHandler ??= HandleCameraAddedEvent;
                    err = EdsdkNative.EdsSetCameraAddedHandler(cameraAddedHandler, IntPtr.Zero);
                    if (err != EdsdkNative.EDS_ERR_OK)
                    {
                        Logger.Warning(correlationId, $"EdsSetCameraAddedHandler failed (0x{err:X8})");
                    }

                    sdkInitialized = true;
                    sdkDiagnostic = null;
                    Logger.Info(correlationId, "Canon EDSDK initialized");
                    if (ShouldRunEdsGetEventPump())
                    {
                        StartEventPumpUnsafe(correlationId);
                    }
                    return true;
                }
                catch (DllNotFoundException ex)
                {
                    var resolved = EdsdkNative.FindEdsdkDllPath(out var details);
                    sdkDiagnostic =
                        "EDSDK load failed. " +
                        $"ResolvedPath={(string.IsNullOrWhiteSpace(resolved) ? "none" : resolved)}. " +
                        (string.IsNullOrWhiteSpace(details) ? "" : $"Details={details}. ") +
                        $"Exception={ex.Message}. " +
                        "Make sure EDSDK.dll and its companion DLLs are present in the same directory (or bundle them under <app>/edsdk).";
                    Logger.Warning(correlationId, sdkDiagnostic);
                    return false;
                }
                catch (BadImageFormatException ex)
                {
                    sdkDiagnostic = $"EDSDK architecture mismatch: {ex.Message}";
                    Logger.Warning(correlationId, sdkDiagnostic);
                    return false;
                }
                catch (Exception ex)
                {
                    sdkDiagnostic = $"EDSDK init exception: {ex.GetType().Name}: {ex.Message}";
                    Logger.Error(correlationId, sdkDiagnostic, ex);
                    return false;
                }
            }
        }

        public void Dispose()
        {
            lock (sdkLock)
            {
                CloseCameraSession();
                StopEventPumpUnsafe();

                if (!sdkInitialized)
                {
                    return;
                }

                try
                {
                    var err = EdsdkNative.EdsTerminateSDK();
                    if (err != EdsdkNative.EDS_ERR_OK)
                    {
                        Logger.Warning("system", $"EdsTerminateSDK failed (0x{err:X8})");
                    }
                }
                catch (Exception ex)
                {
                    Logger.Warning("system", $"EdsTerminateSDK threw: {ex.Message}");
                }
                finally
                {
                    sdkInitialized = false;
                }
            }

            try
            {
                deviceChangeWatcher?.Dispose();
            }
            catch
            {
                // ignore
            }
            finally
            {
                deviceChangeWatcher = null;
            }
        }

        private void ScheduleProbe(string correlationId, string reason)
        {
            var delayMs = reason == "startup" ? 0 : deviceHintDebounceMs;
            CancellationTokenSource cts;
            lock (probeLock)
            {
                pendingProbeReason = reason;
                pendingProbeCorrelationId = correlationId;

                probeDebounceCts?.Cancel();
                probeDebounceCts?.Dispose();
                probeDebounceCts = new CancellationTokenSource();
                cts = probeDebounceCts;
            }

            _ = Task.Run(async () =>
            {
                try
                {
                    await Task.Delay(delayMs, cts.Token);
                }
                catch
                {
                    return;
                }

                string? nextReason;
                string? nextCorrelationId;
                lock (probeLock)
                {
                    nextReason = pendingProbeReason;
                    nextCorrelationId = pendingProbeCorrelationId;
                }

                if (string.IsNullOrWhiteSpace(nextReason) || string.IsNullOrWhiteSpace(nextCorrelationId))
                {
                    return;
                }

                StartProbe(nextCorrelationId, nextReason);
            });
        }

        private void StartProbe(string correlationId, string reason)
        {
            lock (probeLock)
            {
                if (probeInFlight)
                {
                    probeRequested = true;
                    pendingProbeReason = reason;
                    pendingProbeCorrelationId = correlationId;
                    return;
                }

                probeInFlight = true;
            }

            _ = Task.Run(async () =>
            {
                try
                {
                    await ProbeAndEmitStatusChangedAsync(correlationId, reason);
                }
                catch (Exception ex)
                {
                    Logger.Warning(correlationId, $"Probe failed: {ex.Message}");
                }
                finally
                {
                    string? rerunReason = null;
                    string? rerunCorrelationId = null;
                    lock (probeLock)
                    {
                        probeInFlight = false;
                        if (probeRequested)
                        {
                            probeRequested = false;
                            rerunReason = pendingProbeReason;
                            rerunCorrelationId = pendingProbeCorrelationId;
                        }
                    }

                    if (!string.IsNullOrWhiteSpace(rerunReason) && !string.IsNullOrWhiteSpace(rerunCorrelationId))
                    {
                        ScheduleProbe(rerunCorrelationId, rerunReason);
                    }
                }
            });
        }

        private async Task ProbeAndEmitStatusChangedAsync(string correlationId, string reason)
        {
            if (!EnsureSdkInitialized(correlationId))
            {
                string? diag;
                lock (sdkLock)
                {
                    diag = sdkDiagnostic;
                }
                EmitStatusChangedSnapshot(correlationId, reason, connected: false, cameraDetected: false, cameraReady: false, cameraCount: 0, cameraModel: null, sdkInitializedOverride: false, sdkDiagnosticOverride: diag);
                return;
            }

            var probeTask = Task.Run(() => CanonEdsdkProbe.ProbeFirstCamera());
            var completed = await Task.WhenAny(probeTask, Task.Delay(probeTimeoutMs));
            if (completed != probeTask)
            {
                Logger.Warning(correlationId, $"Probe exceeded {probeTimeoutMs}ms; skipping statusChanged emit");
                return;
            }

            var probe = await probeTask;
            EmitStatusChangedSnapshot(
                correlationId,
                reason,
                connected: true,
                cameraDetected: probe.CameraDetected,
                cameraReady: probe.CameraCount > 0,
                cameraCount: probe.CameraCount,
                cameraModel: probe.CameraModel
            );
        }

        private void EmitStatusChangedSnapshot(
            string correlationId,
            string reason,
            bool connected,
            bool cameraDetected,
            bool cameraReady,
            int cameraCount,
            string? cameraModel,
            bool? sdkInitializedOverride = null,
            string? sdkDiagnosticOverride = null
        )
        {
            try
            {
                bool sdkInit;
                string? sdkDiag;
                lock (sdkLock)
                {
                    sdkInit = sdkInitializedOverride ?? sdkInitialized;
                    sdkDiag = sdkDiagnosticOverride ?? sdkDiagnostic;
                }

                string state = connected && cameraReady ? "ready" : connected ? "noCamera" : "error";

                var fingerprint =
                    $"mode=real|sdkInit={sdkInit}|sdkDiag={sdkDiag ?? "null"}|state={state}|connected={connected}|cameraDetected={cameraDetected}|cameraReady={cameraReady}|cameraCount={cameraCount}|cameraModel={cameraModel ?? "null"}|resolved={edsdkResolvedPath ?? "null"}|platform={platformLabel}";

                var now = DateTime.UtcNow;
                if (fingerprint == lastStatusFingerprint)
                {
                    return;
                }

                var elapsed = (now - lastStatusEmitAtUtc).TotalMilliseconds;
                if (elapsed >= 0 && elapsed < statusChangedMinIntervalMs)
                {
                    return;
                }

                lastStatusFingerprint = fingerprint;
                lastStatusEmitAtUtc = now;
                var seq = Interlocked.Increment(ref statusSeq);

                OnStatusChanged?.Invoke(
                    this,
                    IpcMessage.NewEvent(
                        "event.camera.statusChanged",
                        correlationId,
                        new
                        {
                            seq,
                            observedAt = now,
                            reason,
                            mode = "real",
                            sdk = new
                            {
                                initialized = sdkInit,
                                diagnostic = sdkDiag,
                                resolvedPath = edsdkResolvedPath,
                                platform = platformLabel,
                            },
                            state,
                            connected,
                            cameraDetected,
                            cameraReady,
                            cameraCount,
                            cameraModel,
                        }
                    )
                );
            }
            catch
            {
                // ignore
            }
        }

        private void BoostPump(string reason)
        {
            // Treat any externally-visible hint as a "high-frequency" window to maximize chance of observing SDK callbacks.
            if (reason == "cameraAdded" || reason == "shutdown" || reason == "pnpAdded" || reason == "pnpRemoved" || reason == "startup" || reason == "sdkReset")
            {
                pumpBoostUntilUtc = DateTime.UtcNow.AddMilliseconds(pumpBoostWindowMs);
            }
        }

        private bool ShouldBoostPump()
        {
            return pumpBoostUntilUtc > DateTime.UtcNow;
        }

        private bool ShouldRunMessageLoopPump()
        {
            return string.Equals(edsdkPumpMode, "messageLoop", StringComparison.OrdinalIgnoreCase) ||
                   string.Equals(edsdkPumpMode, "both", StringComparison.OrdinalIgnoreCase);
        }

        private bool ShouldRunEdsGetEventPump()
        {
            return string.Equals(edsdkPumpMode, "edsGetEvent", StringComparison.OrdinalIgnoreCase) ||
                   string.Equals(edsdkPumpMode, "both", StringComparison.OrdinalIgnoreCase);
        }

        private static string ReadEnvString(string key, string fallback)
        {
            var raw = Environment.GetEnvironmentVariable(key);
            return string.IsNullOrWhiteSpace(raw) ? fallback : raw.Trim();
        }

        private static int ReadEnvInt(string key, int fallback, int min, int max)
        {
            var raw = Environment.GetEnvironmentVariable(key);
            if (string.IsNullOrWhiteSpace(raw))
            {
                return fallback;
            }

            if (!int.TryParse(raw.Trim(), out var parsed))
            {
                return fallback;
            }

            if (parsed < min)
            {
                return min;
            }

            if (parsed > max)
            {
                return max;
            }

            return parsed;
        }

        private void EmitError(string correlationId, IpcErrorCode code, string? detail)
        {
            var error = new IpcError
            {
                Code = code,
                Message = detail ?? "Camera unavailable in real mode.",
                Context = new Dictionary<string, string>
                {
                    { "mode", "real" }
                }
            };

            if (!string.IsNullOrWhiteSpace(detail))
            {
                error.Context!["diagnostic"] = detail!;
            }

            OnError?.Invoke(this, IpcMessage.NewError(
                "event.camera.error",
                correlationId,
                null,
                error
            ));
        }

        private static (bool ok, string? diagnostic) CheckPrerequisites()
        {
            var edsdkPath = EdsdkNative.FindEdsdkDllPath(out var details);
            if (string.IsNullOrWhiteSpace(edsdkPath))
            {
                return (false, details ?? "EDSDK.dll not found.");
            }

            return (true, $"EDSDK.dll resolved: {edsdkPath}");
        }
    }
}
