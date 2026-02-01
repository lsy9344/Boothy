using System;
using System.Threading;
using System.Threading.Tasks;
using Boothy.CameraSidecar.IPC;

namespace Boothy.CameraSidecar.Camera
{
    public interface ICameraController
    {
        event EventHandler<IpcMessage>? OnPhotoTransferred;
        event EventHandler<IpcMessage>? OnCaptureStarted;
        event EventHandler<IpcMessage>? OnError;
        event EventHandler<IpcMessage>? OnStatusHint;
        event EventHandler<IpcMessage>? OnStatusChanged;

        void SetSessionDestination(string destinationPath);
        Task<bool> CaptureAsync(string correlationId, CancellationToken cancellationToken = default);
        CameraStatusResponse GetStatus(string correlationId);

        /// <summary>
        /// Trigger a status probe that will emit event.camera.statusChanged.
        /// Intended for startup and OS/device hints (power-cycle, hot-plug).
        /// </summary>
        void TriggerStatusProbe(string correlationId, string reason);
    }
}
