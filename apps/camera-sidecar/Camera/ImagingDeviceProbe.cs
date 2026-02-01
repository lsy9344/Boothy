using System;
using System.Runtime.InteropServices;

namespace Boothy.CameraSidecar.Camera
{
    internal static class ImagingDeviceProbe
    {
        // GUID_DEVINTERFACE_IMAGE: imaging devices (cameras, scanners)
        private static readonly Guid GuidDevInterfaceImage = new Guid("6BDD1FC6-810F-11D0-BEC7-08002BE2092F");

        private const uint DIGCF_PRESENT = 0x00000002;
        private const uint DIGCF_DEVICEINTERFACE = 0x00000010;
        private const uint DIGCF_ALLCLASSES = 0x00000004;
        private const int ERROR_NO_MORE_ITEMS = 259;
        private const int ERROR_INSUFFICIENT_BUFFER = 122;

        private const uint SPDRP_DEVICEDESC = 0x00000000;
        private const uint SPDRP_HARDWAREID = 0x00000001;
        private const uint SPDRP_COMPATIBLEIDS = 0x00000002;
        private const uint SPDRP_MFG = 0x0000000B;

        private static readonly nint InvalidHandleValue = new nint(-1);

        public static bool IsCanonImagingDevicePresent()
        {
            // Canon cameras are often exposed as "Portable Devices" (WPD) rather than the "Imaging devices"
            // interface class, so only checking GUID_DEVINTERFACE_IMAGE can produce false negatives.
            // Prefer an "all present devices" scan and only return false when we're confident.
            return IsCanonUsbDevicePresent();
        }

        public static bool IsAnyImagingDevicePresent()
        {
            nint deviceInfoSet = nint.Zero;
            try
            {
                var guid = GuidDevInterfaceImage;
                deviceInfoSet = SetupDiGetClassDevsForInterface(
                    ref guid,
                    nint.Zero,
                    nint.Zero,
                    DIGCF_PRESENT | DIGCF_DEVICEINTERFACE
                );

                if (deviceInfoSet == nint.Zero || deviceInfoSet == InvalidHandleValue)
                {
                    return true;
                }

                var data = new SP_DEVICE_INTERFACE_DATA
                {
                    cbSize = (uint)Marshal.SizeOf<SP_DEVICE_INTERFACE_DATA>(),
                };

                return SetupDiEnumDeviceInterfaces(
                    deviceInfoSet,
                    nint.Zero,
                    ref guid,
                    0,
                    ref data
                );
            }
            catch
            {
                // Fail open: if we can't determine presence, fall back to EDSDK probing.
                return true;
            }
            finally
            {
                try
                {
                    if (deviceInfoSet != nint.Zero && deviceInfoSet != InvalidHandleValue)
                    {
                        _ = SetupDiDestroyDeviceInfoList(deviceInfoSet);
                    }
                }
                catch
                {
                    // ignore
                }
            }
        }

        private static bool IsCanonUsbDevicePresent()
        {
            // Fail open on any errors: we only want to short-circuit EDSDK probing when we're confident
            // the camera is not present. A false negative would prevent detection entirely.
            nint deviceInfoSet = nint.Zero;
            try
            {
                deviceInfoSet = SetupDiGetClassDevsAllClasses(
                    nint.Zero,
                    nint.Zero,
                    nint.Zero,
                    DIGCF_PRESENT | DIGCF_ALLCLASSES
                );

                if (deviceInfoSet == nint.Zero || deviceInfoSet == InvalidHandleValue)
                {
                    return true;
                }

                const string vid = "VID_04A9";
                uint index = 0;
                while (true)
                {
                    var devInfoData = new SP_DEVINFO_DATA
                    {
                        cbSize = (uint)Marshal.SizeOf<SP_DEVINFO_DATA>(),
                    };

                    if (!SetupDiEnumDeviceInfo(deviceInfoSet, index, ref devInfoData))
                    {
                        int err = Marshal.GetLastWin32Error();
                        if (err == ERROR_NO_MORE_ITEMS)
                        {
                            return false;
                        }
                        return true;
                    }

                    if (DeviceHasVid(deviceInfoSet, ref devInfoData, vid))
                    {
                        return true;
                    }

                    index += 1;
                }
            }
            catch
            {
                return true;
            }
            finally
            {
                try
                {
                    if (deviceInfoSet != nint.Zero && deviceInfoSet != InvalidHandleValue)
                    {
                        _ = SetupDiDestroyDeviceInfoList(deviceInfoSet);
                    }
                }
                catch
                {
                    // ignore
                }
            }
        }

        private static bool DeviceHasVid(nint deviceInfoSet, ref SP_DEVINFO_DATA devInfoData, string vidMarker)
        {
            try
            {
                var upperVid = vidMarker.ToUpperInvariant();

                if (TryGetDeviceInstanceId(deviceInfoSet, ref devInfoData, out var instanceId) &&
                    ContainsIgnoreCase(instanceId, upperVid))
                {
                    return true;
                }

                if (TryGetDeviceRegMultiSz(deviceInfoSet, ref devInfoData, SPDRP_HARDWAREID, out var hardwareIds) &&
                    MultiSzContains(hardwareIds, upperVid))
                {
                    return true;
                }

                if (TryGetDeviceRegMultiSz(deviceInfoSet, ref devInfoData, SPDRP_COMPATIBLEIDS, out var compatibleIds) &&
                    MultiSzContains(compatibleIds, upperVid))
                {
                    return true;
                }

                // Fallback: some devices only expose text fields.
                if (TryGetDeviceRegString(deviceInfoSet, ref devInfoData, SPDRP_DEVICEDESC, out var desc) &&
                    ContainsIgnoreCase(desc, upperVid))
                {
                    return true;
                }
                if (TryGetDeviceRegString(deviceInfoSet, ref devInfoData, SPDRP_MFG, out var mfg) &&
                    ContainsIgnoreCase(mfg, "CANON"))
                {
                    return true;
                }

                return false;
            }
            catch
            {
                // Unknown: fail open at caller.
                return false;
            }
        }

        private static bool TryGetDeviceInstanceId(
            nint deviceInfoSet,
            ref SP_DEVINFO_DATA devInfoData,
            out string instanceId
        )
        {
            instanceId = "";

            uint required = 0;
            _ = SetupDiGetDeviceInstanceId(deviceInfoSet, ref devInfoData, null, 0, out required);
            if (required == 0)
            {
                return false;
            }

            var buffer = new char[required];
            if (!SetupDiGetDeviceInstanceId(deviceInfoSet, ref devInfoData, buffer, required, out _))
            {
                return false;
            }

            instanceId = new string(buffer).TrimEnd('\0');
            return !string.IsNullOrWhiteSpace(instanceId);
        }

        private static bool TryGetDeviceRegMultiSz(
            nint deviceInfoSet,
            ref SP_DEVINFO_DATA devInfoData,
            uint property,
            out string[] values
        )
        {
            values = Array.Empty<string>();

            byte[] buffer = new byte[512];
            if (!SetupDiGetDeviceRegistryProperty(
                deviceInfoSet,
                ref devInfoData,
                property,
                out _,
                buffer,
                (uint)buffer.Length,
                out var requiredSize
            ))
            {
                int err = Marshal.GetLastWin32Error();
                if (err == ERROR_INSUFFICIENT_BUFFER && requiredSize > 0)
                {
                    buffer = new byte[requiredSize];
                    if (!SetupDiGetDeviceRegistryProperty(
                        deviceInfoSet,
                        ref devInfoData,
                        property,
                        out _,
                        buffer,
                        (uint)buffer.Length,
                        out requiredSize
                    ))
                    {
                        return false;
                    }
                }
                else
                {
                    return false;
                }
            }

            var str = DecodeRegistryString(buffer, requiredSize);
            if (string.IsNullOrWhiteSpace(str))
            {
                return false;
            }

            values = str.Split('\0', StringSplitOptions.RemoveEmptyEntries);
            return values.Length > 0;
        }

        private static bool TryGetDeviceRegString(
            nint deviceInfoSet,
            ref SP_DEVINFO_DATA devInfoData,
            uint property,
            out string value
        )
        {
            value = "";

            byte[] buffer = new byte[512];
            if (!SetupDiGetDeviceRegistryProperty(
                deviceInfoSet,
                ref devInfoData,
                property,
                out _,
                buffer,
                (uint)buffer.Length,
                out var requiredSize
            ))
            {
                int err = Marshal.GetLastWin32Error();
                if (err == ERROR_INSUFFICIENT_BUFFER && requiredSize > 0)
                {
                    buffer = new byte[requiredSize];
                    if (!SetupDiGetDeviceRegistryProperty(
                        deviceInfoSet,
                        ref devInfoData,
                        property,
                        out _,
                        buffer,
                        (uint)buffer.Length,
                        out requiredSize
                    ))
                    {
                        return false;
                    }
                }
                else
                {
                    return false;
                }
            }

            var str = DecodeRegistryString(buffer, requiredSize);
            if (string.IsNullOrWhiteSpace(str))
            {
                return false;
            }

            value = str;
            return true;
        }

        private static string DecodeRegistryString(byte[] buffer, uint size)
        {
            int len = (int)Math.Min(size, (uint)buffer.Length);
            if (len <= 0)
            {
                return "";
            }

            // SetupAPI returns UTF-16 strings for these properties on Windows.
            var text = System.Text.Encoding.Unicode.GetString(buffer, 0, len);
            return text.TrimEnd('\0');
        }

        private static bool MultiSzContains(string[] values, string upperNeedle)
        {
            for (int i = 0; i < values.Length; i++)
            {
                if (ContainsIgnoreCase(values[i], upperNeedle))
                {
                    return true;
                }
            }
            return false;
        }

        private static bool ContainsIgnoreCase(string? haystack, string upperNeedle)
        {
            if (string.IsNullOrWhiteSpace(haystack) || string.IsNullOrWhiteSpace(upperNeedle))
            {
                return false;
            }
            return haystack.ToUpperInvariant().Contains(upperNeedle);
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct SP_DEVICE_INTERFACE_DATA
        {
            public uint cbSize;
            public Guid InterfaceClassGuid;
            public uint Flags;
            public nint Reserved;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct SP_DEVINFO_DATA
        {
            public uint cbSize;
            public Guid ClassGuid;
            public uint DevInst;
            public nint Reserved;
        }

        [DllImport("setupapi.dll", SetLastError = true, EntryPoint = "SetupDiGetClassDevsW")]
        private static extern nint SetupDiGetClassDevsAllClasses(
            nint ClassGuid,
            nint Enumerator,
            nint hwndParent,
            uint Flags
        );

        [DllImport("setupapi.dll", SetLastError = true)]
        private static extern nint SetupDiGetClassDevsForInterface(
            [In] ref Guid ClassGuid,
            nint Enumerator,
            nint hwndParent,
            uint Flags
        );

        [DllImport("setupapi.dll", SetLastError = true)]
        private static extern bool SetupDiEnumDeviceInfo(
            nint DeviceInfoSet,
            uint MemberIndex,
            ref SP_DEVINFO_DATA DeviceInfoData
        );

        [DllImport("setupapi.dll", SetLastError = true)]
        private static extern bool SetupDiGetDeviceRegistryProperty(
            nint DeviceInfoSet,
            ref SP_DEVINFO_DATA DeviceInfoData,
            uint Property,
            out uint PropertyRegDataType,
            [Out] byte[] PropertyBuffer,
            uint PropertyBufferSize,
            out uint RequiredSize
        );

        [DllImport("setupapi.dll", SetLastError = true, CharSet = CharSet.Auto)]
        private static extern bool SetupDiGetDeviceInstanceId(
            nint DeviceInfoSet,
            ref SP_DEVINFO_DATA DeviceInfoData,
            [Out] char[]? DeviceInstanceId,
            uint DeviceInstanceIdSize,
            out uint RequiredSize
        );

        // Keep the imaging-interface based P/Invokes because DeviceChangeWatcher uses the same class GUID
        // and this is useful in other parts of the system.
        [DllImport("setupapi.dll", SetLastError = true)]
        private static extern bool SetupDiEnumDeviceInterfaces(
            nint DeviceInfoSet,
            nint DeviceInfoData,
            [In] ref Guid InterfaceClassGuid,
            uint MemberIndex,
            ref SP_DEVICE_INTERFACE_DATA DeviceInterfaceData
        );

        [DllImport("setupapi.dll", SetLastError = true)]
        private static extern bool SetupDiDestroyDeviceInfoList(nint DeviceInfoSet);
    }
}
