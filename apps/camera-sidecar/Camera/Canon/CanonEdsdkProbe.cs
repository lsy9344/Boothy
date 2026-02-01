using System;

namespace Boothy.CameraSidecar.Camera.Canon
{
    internal readonly record struct CanonProbeResult(
        bool CameraDetected,
        int CameraCount,
        string? CameraModel,
        string? Diagnostic
    );

    internal static class CanonEdsdkProbe
    {
        public static CanonProbeResult ProbeFirstCamera()
        {
            // When the camera is physically disconnected/powered off, some Canon EDSDK calls can hang.
            // Fast path: if Windows reports no imaging devices present, skip EDSDK calls entirely.
            if (!Boothy.CameraSidecar.Camera.ImagingDeviceProbe.IsCanonImagingDevicePresent())
            {
                return new CanonProbeResult(
                    CameraDetected: false,
                    CameraCount: 0,
                    CameraModel: null,
                    Diagnostic: null
                );
            }

            IntPtr cameraListRef = IntPtr.Zero;
            IntPtr cameraRef = IntPtr.Zero;

            try
            {
                var err = EdsdkNative.EdsGetCameraList(out cameraListRef);
                if (err != EdsdkNative.EDS_ERR_OK || cameraListRef == IntPtr.Zero)
                {
                    return new CanonProbeResult(
                        CameraDetected: false,
                        CameraCount: 0,
                        CameraModel: null,
                        Diagnostic: $"EdsGetCameraList failed (0x{err:X8})"
                    );
                }

                err = EdsdkNative.EdsGetChildCount(cameraListRef, out var count);
                if (err != EdsdkNative.EDS_ERR_OK)
                {
                    return new CanonProbeResult(
                        CameraDetected: false,
                        CameraCount: 0,
                        CameraModel: null,
                        Diagnostic: $"EdsGetChildCount failed (0x{err:X8})"
                    );
                }

                if (count <= 0)
                {
                    return new CanonProbeResult(
                        CameraDetected: false,
                        CameraCount: 0,
                        CameraModel: null,
                        Diagnostic: null
                    );
                }

                err = EdsdkNative.EdsGetChildAtIndex(cameraListRef, 0, out cameraRef);
                if (err != EdsdkNative.EDS_ERR_OK || cameraRef == IntPtr.Zero)
                {
                    return new CanonProbeResult(
                        CameraDetected: false,
                        CameraCount: count,
                        CameraModel: null,
                        Diagnostic: $"EdsGetChildAtIndex(0) failed (0x{err:X8})"
                    );
                }

                err = EdsdkNative.EdsGetDeviceInfo(cameraRef, out var deviceInfo);
                if (err != EdsdkNative.EDS_ERR_OK)
                {
                    return new CanonProbeResult(
                        CameraDetected: true,
                        CameraCount: count,
                        CameraModel: null,
                        Diagnostic: $"EdsGetDeviceInfo failed (0x{err:X8})"
                    );
                }

                var model = string.IsNullOrWhiteSpace(deviceInfo.szDeviceDescription)
                    ? null
                    : deviceInfo.szDeviceDescription.Trim();

                return new CanonProbeResult(
                    CameraDetected: true,
                    CameraCount: count,
                    CameraModel: model,
                    Diagnostic: null
                );
            }
            finally
            {
                if (cameraRef != IntPtr.Zero)
                {
                    _ = EdsdkNative.EdsRelease(cameraRef);
                }

                if (cameraListRef != IntPtr.Zero)
                {
                    _ = EdsdkNative.EdsRelease(cameraListRef);
                }
            }
        }
    }
}
