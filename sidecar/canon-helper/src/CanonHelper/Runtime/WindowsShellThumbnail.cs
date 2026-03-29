using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;

namespace CanonHelper.Runtime;

internal static class WindowsShellThumbnail
{
    public static bool TrySavePreviewJpeg(string rawPath, string previewPath)
    {
        if (!OperatingSystem.IsWindows() || !File.Exists(rawPath))
        {
            return false;
        }

        IntPtr bitmapHandle = IntPtr.Zero;

        try
        {
            SHCreateItemFromParsingName(
                rawPath,
                IntPtr.Zero,
                typeof(IShellItemImageFactory).GUID,
                out IShellItemImageFactory imageFactory
            );
            imageFactory.GetImage(
                new NativeSize { Width = 1600, Height = 1600 },
                ShellItemImageFlags.ResizeToFit
                    | ShellItemImageFlags.BiggerSizeOk
                    | ShellItemImageFlags.ThumbnailOnly,
                out bitmapHandle
            );

            if (bitmapHandle == IntPtr.Zero)
            {
                return false;
            }

            Directory.CreateDirectory(Path.GetDirectoryName(previewPath)!);
            using var bitmap = Image.FromHbitmap(bitmapHandle);
            bitmap.Save(previewPath, ImageFormat.Jpeg);

            var previewInfo = new FileInfo(previewPath);
            return previewInfo.Exists && previewInfo.Length > 0;
        }
        catch
        {
            return false;
        }
        finally
        {
            if (bitmapHandle != IntPtr.Zero)
            {
                DeleteObject(bitmapHandle);
            }
        }
    }

    [ComImport]
    [Guid("bcc18b79-ba16-442f-80c4-8a59c30c463b")]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IShellItemImageFactory
    {
        void GetImage(
            NativeSize size,
            ShellItemImageFlags flags,
            out IntPtr bitmapHandle
        );
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct NativeSize
    {
        public int Width;
        public int Height;
    }

    [Flags]
    private enum ShellItemImageFlags
    {
        ResizeToFit = 0x0,
        BiggerSizeOk = 0x1,
        MemoryOnly = 0x2,
        IconOnly = 0x4,
        ThumbnailOnly = 0x8,
        InCacheOnly = 0x10,
        CropToSquare = 0x20,
        WideThumbnail = 0x40,
        IconBackground = 0x80,
        ScaleUp = 0x100,
    }

    [DllImport("shell32.dll", CharSet = CharSet.Unicode, PreserveSig = false)]
    private static extern void SHCreateItemFromParsingName(
        string path,
        IntPtr bindContext,
        [MarshalAs(UnmanagedType.LPStruct)] Guid interfaceId,
        [MarshalAs(UnmanagedType.Interface)] out IShellItemImageFactory shellItemImageFactory
    );

    [DllImport("gdi32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool DeleteObject(IntPtr objectHandle);
}
