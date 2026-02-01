using System;
using System.IO;
using System.Threading.Tasks;
using System.Threading;
using Boothy.CameraSidecar.IPC;
using Boothy.CameraSidecar.Logging;

namespace Boothy.CameraSidecar.Camera
{
    /// <summary>
    /// Mock camera controller for hardware-less development and integration testing
    /// Simulates camera capture and file transfer without actual camera hardware
    /// </summary>
    public class MockCameraController : ICameraController
    {
        private string? sessionDestination;
        private readonly Random random = new Random();
        private int captureCounter = 0;
        private long statusSeq = 0;
        private string? lastStatusFingerprint;
        private DateTime lastStatusEmitAtUtc = DateTime.MinValue;

        public event EventHandler<IpcMessage>? OnPhotoTransferred;
        public event EventHandler<IpcMessage>? OnCaptureStarted;
        public event EventHandler<IpcMessage>? OnError;
        public event EventHandler<IpcMessage>? OnStatusHint;
        public event EventHandler<IpcMessage>? OnStatusChanged;

        public bool IsCameraConnected => true; // Mock always reports connected
        public string CameraModel => "Mock Canon EOS R5";

        public void SetSessionDestination(string destinationPath)
        {
            sessionDestination = destinationPath;
            string correlationId = IpcHelpers.GenerateCorrelationId();
            Logger.Info(correlationId, $"Session destination set to: {destinationPath}");

            // Ensure destination directory exists
            if (!Directory.Exists(destinationPath))
            {
                Directory.CreateDirectory(destinationPath);
                Logger.Info(correlationId, $"Created session destination directory: {destinationPath}");
            }

            TriggerStatusProbe(correlationId, "startup");
        }

        /// <summary>
        /// Simulate a camera capture operation
        /// Creates a mock RAW file in the session destination
        /// </summary>
        public async Task<bool> CaptureAsync(string correlationId, CancellationToken cancellationToken = default)
        {
            if (string.IsNullOrEmpty(sessionDestination))
            {
                Logger.Error(correlationId, "Cannot capture: session destination not set");
                OnError?.Invoke(this, IpcMessage.NewError(
                    "event.camera.error",
                    correlationId,
                    null,
                    new IpcError
                    {
                        Code = IpcErrorCode.SessionDestinationNotSet,
                        Message = "Session destination must be set before capturing"
                    }
                ));
                return false;
            }

            try
            {
                // Emit capture started event
                DateTime startedAt = DateTime.UtcNow;
                Logger.Info(correlationId, "Mock capture started");
                OnCaptureStarted?.Invoke(this, IpcMessage.NewEvent(
                    "event.camera.captureStarted",
                    correlationId,
                    new CaptureStartedPayload { StartedAt = startedAt }
                ));

                // Simulate capture delay (realistic camera timing)
                await Task.Delay(random.Next(200, 500), cancellationToken);

                // Generate mock RAW file
                captureCounter++;
                string filename = $"MOCK_{DateTime.Now:yyyyMMdd_HHmmss}_{captureCounter:D4}.CR3";
                string filePath = Path.Combine(sessionDestination, filename);

                // Create a mock file with some content (simulating RAW data)
                byte[] mockRawData = GenerateMockRawData();
                await File.WriteAllBytesAsync(filePath, mockRawData, cancellationToken);

                Logger.Info(correlationId, $"Mock file created: {filePath} ({mockRawData.Length} bytes)");

                // Simulate file transfer/write time
                await Task.Delay(random.Next(100, 300), cancellationToken);

                // Emit photo transferred event
                DateTime transferredAt = DateTime.UtcNow;
                OnPhotoTransferred?.Invoke(this, IpcMessage.NewEvent(
                    "event.camera.photoTransferred",
                    correlationId,
                    new PhotoTransferredPayload
                    {
                        Path = filePath,
                        TransferredAt = transferredAt,
                        OriginalFilename = filename,
                        FileSize = mockRawData.Length
                    }
                ));

                Logger.Info(correlationId, $"Photo transferred: {filename}");
                return true;
            }
            catch (Exception ex)
            {
                Logger.Error(correlationId, "Mock capture failed", ex);
                OnError?.Invoke(this, IpcMessage.NewError(
                    "event.camera.error",
                    correlationId,
                    null,
                    new IpcError
                    {
                        Code = IpcErrorCode.CaptureFailed,
                        Message = $"Capture failed: {ex.Message}"
                    }
                ));
                return false;
            }
        }

        /// <summary>
        /// Generate mock RAW file data (minimal valid structure)
        /// In production, this would be actual camera data
        /// </summary>
        private byte[] GenerateMockRawData()
        {
            // Generate a small mock file (1-5 MB simulating RAW)
            int size = random.Next(1024 * 1024, 5 * 1024 * 1024);
            byte[] data = new byte[size];

            // Add some header-like data to make it identifiable as mock
            byte[] header = System.Text.Encoding.UTF8.GetBytes("MOCK_RAW_FILE_v1.0");
            Array.Copy(header, data, Math.Min(header.Length, data.Length));

            // Fill rest with pseudo-random data
            random.NextBytes(data);

            return data;
        }

        /// <summary>
        /// Get current camera status
        /// </summary>
        public CameraStatusResponse GetStatus(string correlationId)
        {
            Logger.Debug(correlationId, $"Mock GetStatus (dest={sessionDestination ?? "null"})");
            TriggerStatusProbe(correlationId, "probe");
            return new CameraStatusResponse
            {
                Connected = true,
                CameraDetected = IsCameraConnected,
                SessionDestination = sessionDestination,
                CameraModel = CameraModel
            };
        }

        public void TriggerStatusProbe(string correlationId, string reason)
        {
            try
            {
                var now = DateTime.UtcNow;
                var fingerprint =
                    $"mode=mock|connected=true|cameraDetected=true|cameraReady=true|cameraCount=1|cameraModel={CameraModel}|dest={sessionDestination ?? "null"}";

                if (fingerprint == lastStatusFingerprint && (now - lastStatusEmitAtUtc).TotalMilliseconds < 250)
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
                            mode = "mock",
                            sdk = new
                            {
                                initialized = false,
                                diagnostic = "Mock mode (EDSDK not in use)",
                                resolvedPath = (string?)null,
                                platform = Environment.Is64BitProcess ? "x64" : "x86",
                            },
                            state = "ready",
                            connected = true,
                            cameraDetected = true,
                            cameraReady = true,
                            cameraCount = 1,
                            cameraModel = CameraModel,
                        }
                    )
                );
            }
            catch
            {
                // ignore
            }
        }
    }
}
