using System.Runtime.InteropServices;
using EDSDKLib;

namespace CanonHelper.Runtime;

internal static class CanonSdkNative
{
    [DllImport("EDSDK.dll")]
    public static extern uint EdsGetEvent();

    [DllImport("EDSDK.dll")]
    public static extern uint EdsSaveImage(
        IntPtr inImageRef,
        EDSDK.EdsTargetImageType inImageType,
        EDSDK.EdsSaveImageSetting inSaveSetting,
        IntPtr outStreamRef
    );
}
