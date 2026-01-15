using System;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using Boothy.CameraSidecar.Camera;
using Boothy.CameraSidecar.IPC;
using Boothy.CameraSidecar.Logging;

namespace Boothy.CameraSidecar
{
    class Program
    {
        private static NamedPipeServer? pipeServer;
        private static MockCameraController? camera;
        private static bool isRunning = true;

        static async Task Main(string[] args)
        {
            // Set up logging
            Logger.SetMinLevel(LogLevel.Debug);
            string startupCorrelationId = IpcHelpers.GenerateCorrelationId();
            Logger.Info(startupCorrelationId, "========================================");
            Logger.Info(startupCorrelationId, "Boothy Camera Sidecar Starting...");
            Logger.Info(startupCorrelationId, $"Protocol Version: {IpcProtocol.Version}");
            Logger.Info(startupCorrelationId, "Mode: Mock (no camera hardware required)");
            Logger.Info(startupCorrelationId, "========================================");

            // Initialize components
            camera = new MockCameraController();
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

            // Handle incoming IPC messages
            pipeServer.OnMessageReceived += async (sender, message) =>
            {
                await HandleIpcMessageAsync(message);
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

            camera!.SetSessionDestination(request.DestinationPath);

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

        private static async Task HandleGetStatusAsync(IpcMessage message)
        {
            Logger.Debug(message.CorrelationId, "Getting camera status");

            var status = camera!.GetStatus();

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

            bool success = await camera!.CaptureAsync();

            var response = IpcMessage.NewResponse(
                message.Method,
                message.CorrelationId,
                message.RequestId ?? "",
                new { success }
            );

            await pipeServer!.SendMessageAsync(response);
        }
    }
}
