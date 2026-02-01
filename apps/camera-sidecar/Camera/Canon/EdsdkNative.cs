using System;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace Boothy.CameraSidecar.Camera.Canon
{
    /// <summary>
    /// Minimal Canon EDSDK interop used for camera presence/model probing.
    /// Derived from the public digiCamControl EDSDK interop patterns.
    /// </summary>
    internal static class EdsdkNative
    {
        public const uint EDS_ERR_OK = 0x00000000;
        private const int EDS_MAX_NAME = 256;
        private const ushort MachineX86 = 0x014c;
        private const ushort MachineX64 = 0x8664;

        private const uint LOAD_WITH_ALTERED_SEARCH_PATH = 0x00000008;
        private const uint LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR = 0x00000100;
        private const uint LOAD_LIBRARY_SEARCH_DEFAULT_DIRS = 0x00001000;

        public delegate uint EdsObjectEventHandler(uint inEvent, IntPtr inRef, IntPtr inContext);
        public delegate uint EdsCameraAddedHandler(IntPtr inContext);
        public delegate uint EdsStateEventHandler(uint inEvent, uint inParameter, IntPtr inContext);

        public const uint ObjectEvent_All = 0x00000200;
        public const uint ObjectEvent_DirItemRequestTransfer = 0x00000208;

        public const uint StateEvent_All = 0x00000300;
        public const uint StateEvent_Shutdown = 0x00000301;
        public const uint StateEvent_InternalError = 0x00000306;

        public const uint CameraCommand_TakePicture = 0x00000000;

        public const uint PropID_SaveTo = 0x0000000b;

        static EdsdkNative()
        {
            try
            {
                NativeLibrary.SetDllImportResolver(typeof(EdsdkNative).Assembly, ResolveEdsdk);
            }
            catch (InvalidOperationException)
            {
                // Resolver already set for this assembly.
            }
        }

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern IntPtr LoadLibraryEx(string lpFileName, IntPtr hFile, uint dwFlags);

        public static string? FindEdsdkDllPath(out string? diagnostic)
        {
            diagnostic = null;

            var envDll = Environment.GetEnvironmentVariable("BOOTHY_EDSDK_DLL");
            if (!string.IsNullOrWhiteSpace(envDll))
            {
                var dllPath = envDll.Trim();
                if (File.Exists(dllPath))
                {
                    return ValidateArchitecture(dllPath, out diagnostic);
                }
                diagnostic = $"BOOTHY_EDSDK_DLL is set but file does not exist: {dllPath}";
                return null;
            }

            var envDir = Environment.GetEnvironmentVariable("BOOTHY_EDSDK_DIR");
            if (!string.IsNullOrWhiteSpace(envDir))
            {
                var dllPath = Path.Combine(envDir.Trim(), "EDSDK.dll");
                if (File.Exists(dllPath))
                {
                    return ValidateArchitecture(dllPath, out diagnostic);
                }
                diagnostic = $"BOOTHY_EDSDK_DIR is set but EDSDK.dll not found: {dllPath}";
                return null;
            }

            var baseDir = AppContext.BaseDirectory;
            var bundledPath = Path.Combine(baseDir, "edsdk", "EDSDK.dll");
            if (File.Exists(bundledPath))
            {
                return ValidateArchitecture(bundledPath, out diagnostic);
            }

            diagnostic =
                $"EDSDK.dll not found. Tried: {bundledPath}. " +
                "Set BOOTHY_EDSDK_DLL (full path) or BOOTHY_EDSDK_DIR (directory containing EDSDK.dll).";
            return null;
        }

        private static string? ValidateArchitecture(string dllPath, out string? diagnostic)
        {
            diagnostic = null;

            try
            {
                var machine = ReadPeMachine(dllPath);
                if (machine == null)
                {
                    return dllPath;
                }

                var expected = Environment.Is64BitProcess ? MachineX64 : MachineX86;
                if (machine.Value != expected)
                {
                    var actualLabel = machine.Value == MachineX86 ? "x86" : machine.Value == MachineX64 ? "x64" : $"0x{machine.Value:X}";
                    var expectedLabel = expected == MachineX86 ? "x86" : "x64";
                    diagnostic =
                        $"EDSDK.dll architecture mismatch. Sidecar is {expectedLabel}, but EDSDK.dll is {actualLabel}: {dllPath}. " +
                        "Provide a matching EDSDK build (x64 for x64 sidecar) via BOOTHY_EDSDK_DLL/BOOTHY_EDSDK_DIR.";
                    return null;
                }

                return dllPath;
            }
            catch (Exception ex)
            {
                diagnostic = $"Failed to validate EDSDK.dll architecture: {ex.Message}";
                return null;
            }
        }

        private static ushort? ReadPeMachine(string path)
        {
            using var stream = File.OpenRead(path);
            using var reader = new BinaryReader(stream);

            stream.Seek(0x3C, SeekOrigin.Begin);
            var peOffset = reader.ReadInt32();
            stream.Seek(peOffset, SeekOrigin.Begin);

            var signature = reader.ReadUInt32();
            if (signature != 0x00004550)
            {
                return null;
            }

            return reader.ReadUInt16();
        }

        private static IntPtr ResolveEdsdk(string libraryName, Assembly assembly, DllImportSearchPath? searchPath)
        {
            if (!libraryName.Equals("EDSDK.dll", StringComparison.OrdinalIgnoreCase))
            {
                return IntPtr.Zero;
            }

            var edsdkPath = FindEdsdkDllPath(out _);
            if (string.IsNullOrWhiteSpace(edsdkPath))
            {
                return IntPtr.Zero;
            }

            try
            {
                // Canon EDSDK is typically shipped with companion DLLs in the same directory.
                // LoadLibraryEx with SEARCH_DLL_LOAD_DIR makes dependency resolution work reliably
                // even when BOOTHY_EDSDK_DLL points to an absolute path outside the app dir.
                var handle = LoadLibraryEx(
                    edsdkPath,
                    IntPtr.Zero,
                    LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS
                );
                if (handle != IntPtr.Zero)
                {
                    return handle;
                }

                // Fallback for older Windows configurations without LOAD_LIBRARY_SEARCH_* support.
                handle = LoadLibraryEx(edsdkPath, IntPtr.Zero, LOAD_WITH_ALTERED_SEARCH_PATH);
                if (handle != IntPtr.Zero)
                {
                    return handle;
                }

                return NativeLibrary.Load(edsdkPath);
            }
            catch
            {
                return IntPtr.Zero;
            }
        }

        [DllImport("EDSDK.dll")]
        public static extern uint EdsInitializeSDK();

        [DllImport("EDSDK.dll")]
        public static extern uint EdsTerminateSDK();

        // NOTE: Canon EDSDK requires periodic event polling to dispatch callbacks
        // registered via EdsSet*EventHandler / EdsSetCameraAddedHandler.
        // digiCamControl relies on this behavior to observe camera shutdown/hotplug.
        [DllImport("EDSDK.dll")]
        public static extern uint EdsGetEvent();

        [DllImport("EDSDK.dll")]
        public static extern uint EdsSetCameraAddedHandler(EdsCameraAddedHandler inCameraAddedHandler, IntPtr inContext);

        [DllImport("EDSDK.dll")]
        public static extern uint EdsGetCameraList(out IntPtr outCameraListRef);

        [DllImport("EDSDK.dll")]
        public static extern uint EdsGetChildAtIndex(IntPtr inRef, int inIndex, out IntPtr outRef);

        [DllImport("EDSDK.dll")]
        public static extern uint EdsGetChildCount(IntPtr inRef, out int outCount);

        [DllImport("EDSDK.dll")]
        public static extern uint EdsGetDeviceInfo(IntPtr inCameraRef, out EdsDeviceInfo outDeviceInfo);

        [DllImport("EDSDK.dll")]
        public static extern uint EdsOpenSession(IntPtr inCameraRef);

        [DllImport("EDSDK.dll")]
        public static extern uint EdsCloseSession(IntPtr inCameraRef);

        [DllImport("EDSDK.dll")]
        public static extern uint EdsSendCommand(IntPtr inCameraRef, uint inCommand, int inParam);

        [DllImport("EDSDK.dll")]
        public static extern uint EdsSetObjectEventHandler(
            IntPtr inCameraRef,
            uint inEvent,
            EdsObjectEventHandler inObjectEventHandler,
            IntPtr inContext
        );

        [DllImport("EDSDK.dll")]
        public static extern uint EdsSetCameraStateEventHandler(
            IntPtr inCameraRef,
            uint inEvent,
            EdsStateEventHandler inStateEventHandler,
            IntPtr inContext
        );

        [DllImport("EDSDK.dll")]
        public static extern uint EdsSetPropertyData(
            IntPtr inRef,
            uint inPropertyId,
            int inParam,
            int inPropertySize,
            [MarshalAs(UnmanagedType.AsAny), In] object inPropertyData
        );

        [DllImport("EDSDK.dll")]
        public static extern uint EdsSetCapacity(IntPtr inCameraRef, EdsCapacity inCapacity);

        [DllImport("EDSDK.dll")]
        public static extern uint EdsGetDirectoryItemInfo(IntPtr inDirItemRef, out EdsDirectoryItemInfo outDirItemInfo);

        [DllImport("EDSDK.dll")]
        public static extern uint EdsCreateFileStream(
            string inFileName,
            EdsFileCreateDisposition inCreateDisposition,
            EdsAccess inDesiredAccess,
            out IntPtr outStream
        );

        [DllImport("EDSDK.dll")]
        public static extern uint EdsDownload(IntPtr inDirItemRef, uint inReadSize, IntPtr outStream);

        [DllImport("EDSDK.dll")]
        public static extern uint EdsDownloadComplete(IntPtr inDirItemRef);

        [DllImport("EDSDK.dll")]
        public static extern uint EdsRelease(IntPtr inRef);

        public enum EdsAccess : uint
        {
            Read = 0,
            Write = 1,
            ReadWrite = 2,
            Error = 0xFFFFFFFF,
        }

        public enum EdsFileCreateDisposition : uint
        {
            CreateNew = 0,
            CreateAlways = 1,
            OpenExisting = 2,
            OpenAlways = 3,
            TruncateExsisting = 4,
        }

        public enum EdsSaveTo : uint
        {
            Camera = 1,
            Host = 2,
            Both = 3,
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct EdsDeviceInfo
        {
            [MarshalAs(UnmanagedType.ByValTStr, SizeConst = EDS_MAX_NAME)]
            public string szPortName;

            [MarshalAs(UnmanagedType.ByValTStr, SizeConst = EDS_MAX_NAME)]
            public string szDeviceDescription;

            public uint DeviceSubType;
            public uint reserved;
        }

        [StructLayout(LayoutKind.Sequential, Pack = 2)]
        public struct EdsCapacity
        {
            public int NumberOfFreeClusters;
            public int BytesPerSector;
            public int Reset;
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct EdsDirectoryItemInfo
        {
            public uint Size;
            public int isFolder;
            public uint GroupID;
            public uint Option;

            [MarshalAs(UnmanagedType.ByValTStr, SizeConst = EDS_MAX_NAME)]
            public string szFileName;

            public uint format;
        }
    }
}
