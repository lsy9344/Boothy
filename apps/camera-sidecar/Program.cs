using System;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using Boothy.CameraSidecar.Camera;
using Boothy.CameraSidecar.Camera.Canon;
using Boothy.CameraSidecar.IPC;
using Boothy.CameraSidecar.Logging;

namespace Boothy.CameraSidecar
{
    class Program
    {
        private const int StartupProbeCaptureWaitTimeoutMs = 2500;
        private static NamedPipeServer? pipeServer;
        private static ICameraController? camera;
        private static bool isRunning = true;
        private static readonly object startupProbeLock = new();
        private static TaskCompletionSource<bool> startupProbeCompletion =
            CreateStartupProbeCompletion(completed: true);

        static async Task Main(string[] args)
        {
            // Set up logging
            Logger.SetMinLevel(LogLevel.Debug);
            Logger.Initialize();
            string startupCorrelationId = IpcHelpers.GenerateCorrelationId();
            Logger.Info(startupCorrelationId, "========================================");
            Logger.Info(startupCorrelationId, "Boothy Camera Sidecar Starting...");
            Logger.Info(startupCorrelationId, $"Protocol Version: {IpcProtocol.Version}");

            string mode = ResolveMode(args);
            Logger.Info(startupCorrelationId, $"Mode: {mode}");
            Logger.Info(startupCorrelationId, "========================================");

            // Initialize components
            camera = mode == "real"
                ? new RealCameraController()
                : new MockCameraController();
            pipeServer = new NamedPipeServer();

            // Wire up camera events to IPC
            camera.OnPhotoTransferred += async (sender, message) =>
            {
                await pipeServer.SendMessageAsync(message);
            };

            camera.OnCaptureStarted += async (sender, message) =>
            {
                await pipeServer.SendMessageAsync(message);
            };

            camera.OnError += async (sender, message) =>
            {
                await pipeServer.SendMessageAsync(message);
            };

            camera.OnStatusHint += async (sender, message) =>
            {
                await pipeServer.SendMessageAsync(message);
            };

            camera.OnStatusChanged += async (sender, message) =>
            {
                CompleteStartupProbe();
                await pipeServer.SendMessageAsync(message);
            };

            // Handle incoming IPC messages
            pipeServer.OnMessageReceived += async (sender, message) =>
            {
                await HandleIpcMessageAsync(message);
            };

            pipeServer.OnClientConnected += (sender, _) =>
            {
                try
                {
                    ResetStartupProbe();
                    string correlationId = IpcHelpers.GenerateCorrelationId();
                    Logger.Info(correlationId, "Boothy connected; emitting startup camera status snapshot");
                    camera?.TriggerStatusProbe(correlationId, "startup");
                }
                catch
                {
                    // ignore
                }
            };

            // Set up graceful shutdown
            Console.CancelKeyPress += (sender, e) =>
            {
                e.Cancel = true;
                isRunning = false;
                Logger.Info("system", "Shutdown signal received");
            };

            // Start server
            pipeServer.Start();

            // Main loop
            Logger.Info(startupCorrelationId, "Sidecar ready and waiting for commands...");
            while (isRunning)
            {
                await Task.Delay(1000);
            }

            // Shutdown
            Logger.Info("system", "Shutting down...");
            if (camera is IDisposable disposable)
            {
                disposable.Dispose();
            }
            pipeServer.Stop();
            Logger.Info("system", "Shutdown complete");
        }

        /// <summary>
        /// Handle incoming IPC messages (requests from Boothy)
        /// </summary>
        private static async Task HandleIpcMessageAsync(IpcMessage message)
        {
            try
            {
                switch (message.Method)
                {
                    case "camera.setSessionDestination":
                        await HandleSetSessionDestinationAsync(message);
                        break;

                    case "camera.getStatus":
                        await HandleGetStatusAsync(message);
                        break;

                    case "camera.capture":
                        await HandleCaptureAsync(message);
                        break;

                    case "system.shutdown":
                        await HandleShutdownAsync(message);
                        break;

                    default:
                        Logger.Warning(message.CorrelationId, $"Unknown method: {message.Method}");
                        var errorResponse = IpcMessage.NewError(
                            message.Method,
                            message.CorrelationId,
                            message.RequestId,
                            new IpcError
                            {
                                Code = IpcErrorCode.InvalidPayload,
                                Message = $"Unknown method: {message.Method}"
                            }
                        );
                        await pipeServer!.SendMessageAsync(errorResponse);
                        break;
                }
            }
            catch (Exception ex)
            {
                Logger.Error(message.CorrelationId, $"Error handling {message.Method}", ex);
                var errorResponse = IpcMessage.NewError(
                    message.Method,
                    message.CorrelationId,
                    message.RequestId,
                    new IpcError
                    {
                        Code = IpcErrorCode.Unknown,
                        Message = $"Internal error: {ex.Message}"
                    }
                );
                await pipeServer!.SendMessageAsync(errorResponse);
            }
        }

        private static async Task HandleSetSessionDestinationAsync(IpcMessage message)
        {
            var request = message.Payload?.Deserialize<SetSessionDestinationRequest>();
            if (request == null)
            {
                Logger.Error(message.CorrelationId, "Invalid SetSessionDestination payload");
                return;
            }

            Logger.Info(message.CorrelationId,
                $"Setting session destination: {request.SessionName} -> {request.DestinationPath}");

            camera!.SetSessionDestination(
                request.DestinationPath,
                ShouldPrepareCameraForSessionDestinationUpdate(isCaptureRequest: false)
            );

            var response = IpcMessage.NewResponse(
                message.Method,
                message.CorrelationId,
                message.RequestId ?? "",
                new SetSessionDestinationResponse
                {
                    Success = true,
                    DestinationPath = request.DestinationPath
                }
            );

            await pipeServer!.SendMessageAsync(response);
        }

        private static async Task HandleShutdownAsync(IpcMessage message)
        {
            Logger.Info(message.CorrelationId, "Shutdown requested");

            var response = IpcMessage.NewResponse(
                message.Method,
                message.CorrelationId,
                message.RequestId ?? "",
                new { success = true }
            );

            await pipeServer!.SendMessageAsync(response);
            isRunning = false;
        }

        private static async Task HandleGetStatusAsync(IpcMessage message)
        {
            Logger.Debug(message.CorrelationId, "Getting camera status");
            const int timeoutMs = 2500;
            var getStatusTask = Task.Run(() => camera!.GetStatus(message.CorrelationId));
            var completed = await Task.WhenAny(getStatusTask, Task.Delay(timeoutMs));
            if (completed != getStatusTask)
            {
                Logger.Warning(
                    message.CorrelationId,
                    $"camera.getStatus exceeded {timeoutMs}ms; returning Timeout and terminating sidecar for recovery"
                );

                var errorResponse = IpcMessage.NewError(
                    message.Method,
                    message.CorrelationId,
                    message.RequestId,
                    new IpcError
                    {
                        Code = IpcErrorCode.Timeout,
                        Message = "Camera service is not responding. Restarting camera service..."
                    }
                );
                await pipeServer!.SendMessageAsync(errorResponse);

                _ = Task.Run(() =>
                {
                    try
                    {
                        // Immediate exit is intentional: EDSDK can hang in native calls, making graceful disposal unreliable.
                        Environment.Exit(2);
                    }
                    catch
                    {
                        // ignore
                    }
                });
                return;
            }

            var status = await getStatusTask;
            var response = IpcMessage.NewResponse(
                message.Method,
                message.CorrelationId,
                message.RequestId ?? "",
                status
            );
            await pipeServer!.SendMessageAsync(response);
        }

        private static async Task HandleCaptureAsync(IpcMessage message)
        {
            Logger.Info(message.CorrelationId, "Capture requested");
            var startupProbeCompleted = await AwaitStartupProbeBeforeCaptureAsync(
                GetStartupProbeTask(),
                StartupProbeCaptureWaitTimeoutMs
            );
            if (startupProbeCompleted)
            {
                Logger.Debug(message.CorrelationId, "Startup camera status probe completed before capture");
            }
            else
            {
                Logger.Warning(
                    message.CorrelationId,
                    $"Startup camera status probe did not complete within {StartupProbeCaptureWaitTimeoutMs}ms; proceeding with capture"
                );
            }

            var request = message.Payload?.Deserialize<CaptureRequest>();
            if (!string.IsNullOrWhiteSpace(request?.DestinationPath))
            {
                Logger.Info(
                    message.CorrelationId,
                    $"Capture request applying session destination: {request.SessionName ?? "unknown"} -> {request.DestinationPath}"
                );
                camera!.SetSessionDestination(
                    request.DestinationPath!,
                    ShouldPrepareCameraForSessionDestinationUpdate(isCaptureRequest: true)
                );
            }

            bool success = await camera!.CaptureAsync(message.CorrelationId);

            var response = IpcMessage.NewResponse(
                message.Method,
                message.CorrelationId,
                message.RequestId ?? "",
                new { success }
            );

            await pipeServer!.SendMessageAsync(response);
        }

        private static bool ShouldPrepareCameraForSessionDestinationUpdate(bool isCaptureRequest)
        {
            return !isCaptureRequest;
        }

        private static TaskCompletionSource<bool> CreateStartupProbeCompletion(bool completed)
        {
            var completion = new TaskCompletionSource<bool>(TaskCreationOptions.RunContinuationsAsynchronously);
            if (completed)
            {
                completion.TrySetResult(true);
            }

            return completion;
        }

        private static void ResetStartupProbe()
        {
            lock (startupProbeLock)
            {
                startupProbeCompletion = CreateStartupProbeCompletion(completed: false);
            }
        }

        private static void CompleteStartupProbe()
        {
            lock (startupProbeLock)
            {
                startupProbeCompletion.TrySetResult(true);
            }
        }

        private static Task GetStartupProbeTask()
        {
            lock (startupProbeLock)
            {
                return startupProbeCompletion.Task;
            }
        }

        private static async Task<bool> AwaitStartupProbeBeforeCaptureAsync(
            Task startupProbeTask,
            int timeoutMs
        )
        {
            if (startupProbeTask.IsCompleted)
            {
                return true;
            }

            var completed = await Task.WhenAny(startupProbeTask, Task.Delay(timeoutMs));
            return completed == startupProbeTask;
        }

        private static string ResolveMode(string[] args)
        {
            string? mode = null;
            for (int i = 0; i < args.Length; i++)
            {
                var arg = args[i];
                if (arg.StartsWith("--mode=", StringComparison.OrdinalIgnoreCase))
                {
                    mode = arg.Substring("--mode=".Length);
                }
                else if (arg.Equals("--mode", StringComparison.OrdinalIgnoreCase) && i + 1 < args.Length)
                {
                    mode = args[i + 1];
                }
            }

            if (string.IsNullOrWhiteSpace(mode))
            {
                mode = Environment.GetEnvironmentVariable("BOOTHY_CAMERA_MODE");
            }

            if (string.IsNullOrWhiteSpace(mode))
            {
                // Default behavior: if EDSDK is present, prefer real mode; otherwise fall back to mock.
                // This keeps dev machines hardware-less by default while enabling field rigs automatically.
                var edsdkPath = EdsdkNative.FindEdsdkDllPath(out var diagnostic);
                if (string.IsNullOrWhiteSpace(edsdkPath))
                {
                    if (!string.IsNullOrWhiteSpace(diagnostic))
                    {
                        Logger.Warning("system", $"EDSDK not usable; defaulting camera mode to mock. {diagnostic}");
                    }
                    mode = "mock";
                }
                else
                {
                    mode = "real";
                }
            }
            else
            {
                mode = mode.Trim().ToLowerInvariant();
            }

            if (mode != "mock" && mode != "real")
            {
                Logger.Warning("system", $"Unknown mode '{mode}', defaulting to mock.");
                mode = "mock";
            }

            return mode;
        }
    }
}
