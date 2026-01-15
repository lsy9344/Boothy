using System;
using System.IO;
using System.IO.Pipes;
using System.Security.AccessControl;
using System.Security.Principal;
using System.Text;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using Boothy.CameraSidecar.Logging;

namespace Boothy.CameraSidecar.IPC
{
    /// <summary>
    /// Named Pipe server for IPC communication with Boothy (Tauri backend)
    /// Uses Windows Named Pipes for secure, low-latency local IPC
    /// </summary>
    public class NamedPipeServer
    {
        private const string PipeName = "boothy_camera_sidecar";
        private NamedPipeServerStream? pipeServer;
        private CancellationTokenSource? cancellationTokenSource;
        private Task? listenerTask;

        public event EventHandler<IpcMessage>? OnMessageReceived;
        public bool IsRunning { get; private set; }

        /// <summary>
        /// Create PipeSecurity that restricts access to the current user
        /// </summary>
        private static PipeSecurity CreatePipeSecurity()
        {
            var pipeSecurity = new PipeSecurity();

            // Get the current user's identity
            var identity = WindowsIdentity.GetCurrent();
            var userSid = identity.User;

            if (userSid != null)
            {
                // Grant full control to the current user
                var userRule = new PipeAccessRule(
                    userSid,
                    PipeAccessRights.FullControl,
                    AccessControlType.Allow
                );
                pipeSecurity.AddAccessRule(userRule);
            }

            return pipeSecurity;
        }

        /// <summary>
        /// Start the Named Pipe server and begin listening for connections
        /// </summary>
        public void Start()
        {
            if (IsRunning)
            {
                Logger.Warning("system", "Named Pipe server already running");
                return;
            }

            cancellationTokenSource = new CancellationTokenSource();
            listenerTask = Task.Run(() => ListenAsync(cancellationTokenSource.Token));
            IsRunning = true;

            string correlationId = IpcHelpers.GenerateCorrelationId();
            Logger.Info(correlationId, $"Named Pipe server started on pipe: {PipeName}");
        }

        /// <summary>
        /// Stop the Named Pipe server
        /// </summary>
        public void Stop()
        {
            if (!IsRunning) return;

            string correlationId = IpcHelpers.GenerateCorrelationId();
            Logger.Info(correlationId, "Stopping Named Pipe server...");

            cancellationTokenSource?.Cancel();
            pipeServer?.Dispose();
            listenerTask?.Wait(TimeSpan.FromSeconds(5));

            IsRunning = false;
            Logger.Info(correlationId, "Named Pipe server stopped");
        }

        /// <summary>
        /// Listen for incoming connections and messages
        /// </summary>
        private async Task ListenAsync(CancellationToken cancellationToken)
        {
            while (!cancellationToken.IsCancellationRequested)
            {
                try
                {
                    // Create new pipe server for each connection with ACL hardening
                    var pipeSecurity = CreatePipeSecurity();
                    pipeServer = NamedPipeServerStreamAcl.Create(
                        pipeName: PipeName,
                        direction: PipeDirection.InOut,
                        maxNumberOfServerInstances: 1,
                        transmissionMode: PipeTransmissionMode.Byte,
                        options: PipeOptions.Asynchronous,
                        inBufferSize: 4096,
                        outBufferSize: 4096,
                        pipeSecurity: pipeSecurity
                    );

                    string correlationId = IpcHelpers.GenerateCorrelationId();
                    Logger.Info(correlationId, "Waiting for Boothy connection...");

                    // Wait for client connection
                    await pipeServer.WaitForConnectionAsync(cancellationToken);
                    Logger.Info(correlationId, "Boothy connected");

                    // Handle communication while connected
                    await HandleClientAsync(pipeServer, cancellationToken);
                }
                catch (OperationCanceledException)
                {
                    break; // Graceful shutdown
                }
                catch (Exception ex)
                {
                    string correlationId = IpcHelpers.GenerateCorrelationId();
                    Logger.Error(correlationId, "Pipe server error", ex);
                    await Task.Delay(1000, cancellationToken); // Backoff before retry
                }
                finally
                {
                    pipeServer?.Dispose();
                    pipeServer = null;
                }
            }
        }

        /// <summary>
        /// Handle communication with connected client
        /// </summary>
        private async Task HandleClientAsync(NamedPipeServerStream pipe, CancellationToken cancellationToken)
        {
            try
            {
                using var reader = new StreamReader(pipe, Encoding.UTF8, leaveOpen: true);
                using var writer = new StreamWriter(pipe, Encoding.UTF8, leaveOpen: true) { AutoFlush = true };

                while (pipe.IsConnected && !cancellationToken.IsCancellationRequested)
                {
                    string? line = await reader.ReadLineAsync();
                    if (line == null) break; // Client disconnected

                    // Parse IPC message
                    IpcMessage? message = JsonSerializer.Deserialize<IpcMessage>(line);
                    if (message == null)
                    {
                        Logger.Warning("system", $"Failed to parse IPC message: {line}");
                        continue;
                    }

                    // Validate protocol version
                    if (message.ProtocolVersion != IpcProtocol.Version)
                    {
                        Logger.Warning(message.CorrelationId,
                            $"Protocol version mismatch: expected {IpcProtocol.Version}, got {message.ProtocolVersion}");

                        var errorMsg = IpcMessage.NewError(
                            message.Method,
                            message.CorrelationId,
                            message.RequestId,
                            new IpcError
                            {
                                Code = IpcErrorCode.VersionMismatch,
                                Message = $"Protocol version mismatch: expected {IpcProtocol.Version}, got {message.ProtocolVersion}"
                            }
                        );
                        await SendMessageAsync(errorMsg, writer);
                        continue;
                    }

                    Logger.Debug(message.CorrelationId, $"Received {message.MessageType}: {message.Method}");

                    // Notify listeners
                    OnMessageReceived?.Invoke(this, message);
                }
            }
            catch (Exception ex)
            {
                string correlationId = IpcHelpers.GenerateCorrelationId();
                Logger.Error(correlationId, "Client communication error", ex);
            }
        }

        /// <summary>
        /// Send an IPC message to the connected client
        /// </summary>
        public async Task SendMessageAsync(IpcMessage message)
        {
            if (pipeServer == null || !pipeServer.IsConnected)
            {
                Logger.Warning(message.CorrelationId, "Cannot send message: pipe not connected");
                return;
            }

            try
            {
                using var writer = new StreamWriter(pipeServer, Encoding.UTF8, leaveOpen: true) { AutoFlush = true };
                await SendMessageAsync(message, writer);
            }
            catch (Exception ex)
            {
                Logger.Error(message.CorrelationId, "Failed to send message", ex);
            }
        }

        private async Task SendMessageAsync(IpcMessage message, StreamWriter writer)
        {
            string json = JsonSerializer.Serialize(message);
            await writer.WriteLineAsync(json);
            Logger.Debug(message.CorrelationId, $"Sent {message.MessageType}: {message.Method}");
        }
    }
}
