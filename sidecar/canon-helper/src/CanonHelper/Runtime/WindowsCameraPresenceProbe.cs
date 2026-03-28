using System.Runtime.InteropServices;
using System.Text;

namespace CanonHelper.Runtime;

internal sealed record WindowsCameraPresence(
    bool IsPresent,
    string? FriendlyName,
    string? Status,
    string? InstanceId
);

internal static class WindowsCameraPresenceProbe
{
    private const string CanonUsbVendorId = "VID_04A9";
    private const uint DigcfPresent = 0x00000002;
    private const uint DigcfAllClasses = 0x00000004;
    private const uint SpdrpDevDesc = 0x00000000;
    private const uint SpdrpHardwareId = 0x00000001;
    private const uint SpdrpFriendlyName = 0x0000000C;
    private const int ErrorNoMoreItems = 259;

    public static WindowsCameraPresence DetectCanonCamera()
    {
        if (!OperatingSystem.IsWindows())
        {
            return new WindowsCameraPresence(false, null, null, null);
        }

        var deviceInfoSet = SetupDiGetClassDevs(
            IntPtr.Zero,
            null,
            IntPtr.Zero,
            DigcfPresent | DigcfAllClasses
        );

        if (deviceInfoSet == IntPtr.Zero || deviceInfoSet == new IntPtr(-1))
        {
            return new WindowsCameraPresence(false, null, null, null);
        }

        try
        {
            var index = 0u;
            while (true)
            {
                var deviceInfoData = new SpDevinfoData
                {
                    CbSize = (uint)Marshal.SizeOf<SpDevinfoData>(),
                };

                if (!SetupDiEnumDeviceInfo(deviceInfoSet, index, ref deviceInfoData))
                {
                    if (Marshal.GetLastWin32Error() == ErrorNoMoreItems)
                    {
                        break;
                    }

                    index += 1;
                    continue;
                }

                var hardwareIds = ReadMultiSzProperty(
                    deviceInfoSet,
                    ref deviceInfoData,
                    SpdrpHardwareId
                );

                if (
                    !hardwareIds.Any(hardwareId =>
                        hardwareId.IndexOf(CanonUsbVendorId, StringComparison.OrdinalIgnoreCase) >= 0
                    )
                )
                {
                    index += 1;
                    continue;
                }

                var friendlyName =
                    ReadStringProperty(deviceInfoSet, ref deviceInfoData, SpdrpFriendlyName)
                    ?? ReadStringProperty(deviceInfoSet, ref deviceInfoData, SpdrpDevDesc);
                var instanceId = hardwareIds.FirstOrDefault();

                return new WindowsCameraPresence(
                    true,
                    friendlyName,
                    "Present",
                    instanceId
                );
            }
        }
        finally
        {
            SetupDiDestroyDeviceInfoList(deviceInfoSet);
        }

        return new WindowsCameraPresence(false, null, null, null);
    }

    private static string? ReadStringProperty(
        IntPtr deviceInfoSet,
        ref SpDevinfoData deviceInfoData,
        uint property
    )
    {
        var buffer = new byte[1024];
        if (
            !SetupDiGetDeviceRegistryProperty(
                deviceInfoSet,
                ref deviceInfoData,
                property,
                out _,
                buffer,
                (uint)buffer.Length,
                out _
            )
        )
        {
            return null;
        }

        var raw = Encoding.Unicode.GetString(buffer);
        var terminatorIndex = raw.IndexOf('\0');
        return terminatorIndex >= 0 ? raw[..terminatorIndex] : raw;
    }

    private static IReadOnlyList<string> ReadMultiSzProperty(
        IntPtr deviceInfoSet,
        ref SpDevinfoData deviceInfoData,
        uint property
    )
    {
        var buffer = new byte[4096];
        if (
            !SetupDiGetDeviceRegistryProperty(
                deviceInfoSet,
                ref deviceInfoData,
                property,
                out _,
                buffer,
                (uint)buffer.Length,
                out _
            )
        )
        {
            return Array.Empty<string>();
        }

        return Encoding.Unicode
            .GetString(buffer)
            .Split('\0', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct SpDevinfoData
    {
        public uint CbSize;
        public Guid ClassGuid;
        public uint DevInst;
        public IntPtr Reserved;
    }

    [DllImport("setupapi.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr SetupDiGetClassDevs(
        IntPtr classGuid,
        string? enumerator,
        IntPtr hwndParent,
        uint flags
    );

    [DllImport("setupapi.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetupDiEnumDeviceInfo(
        IntPtr deviceInfoSet,
        uint memberIndex,
        ref SpDevinfoData deviceInfoData
    );

    [DllImport("setupapi.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetupDiGetDeviceRegistryProperty(
        IntPtr deviceInfoSet,
        ref SpDevinfoData deviceInfoData,
        uint property,
        out uint propertyRegDataType,
        byte[] propertyBuffer,
        uint propertyBufferSize,
        out uint requiredSize
    );

    [DllImport("setupapi.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetupDiDestroyDeviceInfoList(IntPtr deviceInfoSet);
}
