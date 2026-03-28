using System.Runtime.InteropServices;

namespace CanonHelper.Runtime;

internal static class CanonSdkNative
{
    [DllImport("EDSDK.dll")]
    public static extern uint EdsGetEvent();
}
