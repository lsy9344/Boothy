using System.Diagnostics;

namespace CanonHelper.Runtime;

internal static class ParentProcessMonitor
{
    public static bool IsAlive(int? parentPid)
    {
        if (parentPid is null)
        {
            return true;
        }

        try
        {
            using var process = Process.GetProcessById(parentPid.Value);
            return !process.HasExited;
        }
        catch
        {
            return false;
        }
    }
}
