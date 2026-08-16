using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;

namespace CanonHelper.Runtime;

/// <summary>
/// 요청 크기와 실제 산출 크기가 다를 수 있는 Shell 썸네일 경로.
/// </summary>
/// <remarks>
/// Story 7.3 Route C(incumbent 기준선)의 계측 결과다. <c>BiggerSizeOk</c>는 요청보다 큰
/// 캐시본을 허용하므로 <see cref="RequestedEdgePx"/>와 <see cref="WidthPx"/>가 일치한다는
/// 보장이 없다. display-fit 판정(Story 7.4)의 입력은 요청값이 아니라 실측값이어야 한다.
/// </remarks>
internal sealed record ShellThumbnailMeasurement(
    bool Succeeded,
    int RequestedEdgePx,
    int WidthPx,
    int HeightPx,
    long ByteSize,
    long ExtractionCostMicros,
    string? FailureDetailCode
);

internal static class WindowsShellThumbnail
{
    /// <summary>제품 코드가 오랫동안 써 온 요청 크기. 이 Story는 값을 바꾸지 않는다.</summary>
    internal const int RequestedEdgePx = 1600;

    public static bool TrySavePreviewJpeg(string rawPath, string previewPath)
    {
        return Measure(rawPath, previewPath).Succeeded;
    }

    /// <summary>
    /// 썸네일을 저장하면서 <b>실제 산출 크기와 추출 비용</b>을 함께 돌려준다.
    /// </summary>
    /// <remarks>
    /// 실패해도 예외를 던지지 않는다. incumbent는 best-effort 경로이며 RAW truth와 무관하다.
    /// 실패 표본도 고유 detail code와 함께 남는다 — 행이 없는 시도를 만들면 성공률 분모가
    /// 조용히 줄어 기준선이 실제보다 좋아 보인다.
    /// </remarks>
    public static ShellThumbnailMeasurement Measure(string rawPath, string previewPath)
    {
        var startedAt = System.Diagnostics.Stopwatch.GetTimestamp();

        long ElapsedMicros() =>
            (System.Diagnostics.Stopwatch.GetTimestamp() - startedAt)
            * 1_000_000L
            / System.Diagnostics.Stopwatch.Frequency;

        ShellThumbnailMeasurement Failure(string detailCode) =>
            new(false, RequestedEdgePx, 0, 0, 0, ElapsedMicros(), detailCode);

        if (!OperatingSystem.IsWindows())
        {
            return Failure("shell-thumbnail-unsupported-os");
        }

        if (!File.Exists(rawPath))
        {
            return Failure("shell-thumbnail-source-missing");
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
                new NativeSize { Width = RequestedEdgePx, Height = RequestedEdgePx },
                ShellItemImageFlags.ResizeToFit
                    | ShellItemImageFlags.BiggerSizeOk
                    | ShellItemImageFlags.ThumbnailOnly,
                out bitmapHandle
            );

            if (bitmapHandle == IntPtr.Zero)
            {
                return Failure("shell-thumbnail-empty-handle");
            }

            Directory.CreateDirectory(Path.GetDirectoryName(previewPath)!);
            using var bitmap = Image.FromHbitmap(bitmapHandle);
            bitmap.Save(previewPath, ImageFormat.Jpeg);

            var previewInfo = new FileInfo(previewPath);
            if (!previewInfo.Exists || previewInfo.Length == 0)
            {
                return Failure("shell-thumbnail-empty-file");
            }

            return new ShellThumbnailMeasurement(
                true,
                RequestedEdgePx,
                bitmap.Width,
                bitmap.Height,
                previewInfo.Length,
                ElapsedMicros(),
                null
            );
        }
        catch
        {
            return Failure("shell-thumbnail-failed");
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
