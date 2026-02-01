using System;
using System.Runtime.InteropServices;
using System.Threading;
using Boothy.CameraSidecar.Logging;

namespace Boothy.CameraSidecar.Camera
{
    internal sealed class DeviceChangeWatcher : IDisposable
    {
        private const int WM_DEVICECHANGE = 0x0219;
        private const int WM_QUIT = 0x0012;

        private const int DBT_DEVICEARRIVAL = 0x8000;
        private const int DBT_DEVICEREMOVECOMPLETE = 0x8004;

        private const int DBT_DEVTYP_DEVICEINTERFACE = 0x00000005;
        private const int DEVICE_NOTIFY_WINDOW_HANDLE = 0x00000000;

        // GUID_DEVINTERFACE_IMAGE: imaging devices (cameras, scanners)
        private static readonly Guid GuidDevInterfaceImage = new Guid("6BDD1FC6-810F-11D0-BEC7-08002BE2092F");

        private readonly Action<string> onHint;
        private readonly string correlationId;
        private Thread? thread;
        private uint threadId;
        private nint hwnd;
        private nint notificationHandle;
        private WndProc? wndProc;
        private bool disposed;

        public DeviceChangeWatcher(string correlationId, Action<string> onHint)
        {
            this.correlationId = correlationId;
            this.onHint = onHint;

            thread = new Thread(ThreadMain)
            {
                IsBackground = true,
                Name = "Boothy.DeviceChangeWatcher",
            };
            thread.SetApartmentState(ApartmentState.STA);
            thread.Start();
        }

        public void Dispose()
        {
            if (disposed)
            {
                return;
            }
            disposed = true;

            try
            {
                if (threadId != 0)
                {
                    _ = PostThreadMessage(threadId, WM_QUIT, 0, 0);
                }
            }
            catch
            {
                // ignore
            }

            try
            {
                thread?.Join(TimeSpan.FromSeconds(2));
            }
            catch
            {
                // ignore
            }
            finally
            {
                thread = null;
            }
        }

        private void ThreadMain()
        {
            try
            {
                threadId = GetCurrentThreadId();

                wndProc = WindowProc;
                var className = $"BoothyDeviceChangeWatcher-{Guid.NewGuid():N}";

                var wc = new WNDCLASSEX
                {
                    cbSize = (uint)Marshal.SizeOf<WNDCLASSEX>(),
                    lpfnWndProc = Marshal.GetFunctionPointerForDelegate(wndProc),
                    lpszClassName = className,
                };

                ushort atom = RegisterClassEx(ref wc);
                if (atom == 0)
                {
                    Logger.Warning(correlationId, $"DeviceChangeWatcher: RegisterClassEx failed (err={Marshal.GetLastWin32Error()})");
                    return;
                }

                hwnd = CreateWindowEx(
                    0,
                    className,
                    "BoothyDeviceChangeWatcher",
                    0,
                    0,
                    0,
                    0,
                    0,
                    nint.Zero,
                    nint.Zero,
                    nint.Zero,
                    nint.Zero
                );

                if (hwnd == nint.Zero)
                {
                    Logger.Warning(correlationId, $"DeviceChangeWatcher: CreateWindowEx failed (err={Marshal.GetLastWin32Error()})");
                    return;
                }

                var filter = new DEV_BROADCAST_DEVICEINTERFACE
                {
                    dbcc_size = (uint)Marshal.SizeOf<DEV_BROADCAST_DEVICEINTERFACE>(),
                    dbcc_devicetype = DBT_DEVTYP_DEVICEINTERFACE,
                    dbcc_reserved = 0,
                    dbcc_classguid = GuidDevInterfaceImage,
                    dbcc_name = 0,
                };

                notificationHandle = RegisterDeviceNotification(hwnd, ref filter, DEVICE_NOTIFY_WINDOW_HANDLE);
                if (notificationHandle == nint.Zero)
                {
                    Logger.Warning(correlationId, $"DeviceChangeWatcher: RegisterDeviceNotification failed (err={Marshal.GetLastWin32Error()})");
                }
                else
                {
                    Logger.Info(correlationId, "DeviceChangeWatcher: listening for imaging device hot-plug events");
                }

                while (GetMessage(out var msg, nint.Zero, 0, 0) > 0)
                {
                    TranslateMessage(ref msg);
                    DispatchMessage(ref msg);
                }
            }
            catch (Exception ex)
            {
                Logger.Warning(correlationId, $"DeviceChangeWatcher: loop error: {ex.Message}");
            }
            finally
            {
                try
                {
                    if (notificationHandle != nint.Zero)
                    {
                        _ = UnregisterDeviceNotification(notificationHandle);
                        notificationHandle = nint.Zero;
                    }
                }
                catch
                {
                    // ignore
                }

                try
                {
                    if (hwnd != nint.Zero)
                    {
                        _ = DestroyWindow(hwnd);
                        hwnd = nint.Zero;
                    }
                }
                catch
                {
                    // ignore
                }
            }
        }

        private nint WindowProc(nint hWnd, uint msg, nint wParam, nint lParam)
        {
            if (msg == WM_DEVICECHANGE)
            {
                int eventType = unchecked((int)(long)wParam);
                if (eventType == DBT_DEVICEARRIVAL)
                {
                    onHint("pnpAdded");
                }
                else if (eventType == DBT_DEVICEREMOVECOMPLETE)
                {
                    onHint("pnpRemoved");
                }
            }

            return DefWindowProc(hWnd, msg, wParam, lParam);
        }

        private delegate nint WndProc(nint hWnd, uint msg, nint wParam, nint lParam);

        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
        private struct WNDCLASSEX
        {
            public uint cbSize;
            public uint style;
            public nint lpfnWndProc;
            public int cbClsExtra;
            public int cbWndExtra;
            public nint hInstance;
            public nint hIcon;
            public nint hCursor;
            public nint hbrBackground;
            public string? lpszMenuName;
            public string lpszClassName;
            public nint hIconSm;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct MSG
        {
            public nint hwnd;
            public uint message;
            public nint wParam;
            public nint lParam;
            public uint time;
            public POINT pt;
            public uint lPrivate;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct POINT
        {
            public int x;
            public int y;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct DEV_BROADCAST_DEVICEINTERFACE
        {
            public uint dbcc_size;
            public uint dbcc_devicetype;
            public uint dbcc_reserved;
            public Guid dbcc_classguid;
            public short dbcc_name;
        }

        [DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
        private static extern ushort RegisterClassEx([In] ref WNDCLASSEX lpwcx);

        [DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
        private static extern nint CreateWindowEx(
            int dwExStyle,
            string lpClassName,
            string lpWindowName,
            int dwStyle,
            int x,
            int y,
            int nWidth,
            int nHeight,
            nint hWndParent,
            nint hMenu,
            nint hInstance,
            nint lpParam
        );

        [DllImport("user32.dll")]
        private static extern nint DefWindowProc(nint hWnd, uint msg, nint wParam, nint lParam);

        [DllImport("user32.dll", SetLastError = true)]
        private static extern int GetMessage(out MSG lpMsg, nint hWnd, uint wMsgFilterMin, uint wMsgFilterMax);

        [DllImport("user32.dll")]
        private static extern bool TranslateMessage([In] ref MSG lpMsg);

        [DllImport("user32.dll")]
        private static extern nint DispatchMessage([In] ref MSG lpMsg);

        [DllImport("user32.dll", SetLastError = true)]
        private static extern bool DestroyWindow(nint hWnd);

        [DllImport("user32.dll", SetLastError = true)]
        private static extern nint RegisterDeviceNotification(nint hRecipient, [In] ref DEV_BROADCAST_DEVICEINTERFACE NotificationFilter, uint Flags);

        [DllImport("user32.dll", SetLastError = true)]
        private static extern bool UnregisterDeviceNotification(nint Handle);

        [DllImport("user32.dll", SetLastError = true)]
        private static extern bool PostThreadMessage(uint idThread, uint Msg, nint wParam, nint lParam);

        [DllImport("kernel32.dll")]
        private static extern uint GetCurrentThreadId();
    }
}

