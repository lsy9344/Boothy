using System.Diagnostics;

namespace CanonHelper.Runtime;

internal static class ParentProcessMonitor
{
    private static readonly object Sync = new();
    private static readonly Dictionary<int, DateTimeOffset> ParentStartTimes = new();

    public static bool IsAlive(int? parentPid)
    {
        return IsAlive(parentPid, ParentStartTimes, ReadSnapshot);
    }

    internal static bool IsAlive(
        int? parentPid,
        IDictionary<int, DateTimeOffset> parentStartTimes,
        Func<int, ParentProcessSnapshot?> readSnapshot
    )
    {
        if (parentPid is null)
        {
            return true;
        }

        var snapshot = readSnapshot(parentPid.Value);
        if (snapshot is null || snapshot.HasExited)
        {
            return false;
        }

        lock (Sync)
        {
            if (!parentStartTimes.TryGetValue(parentPid.Value, out var expectedStartTime))
            {
                parentStartTimes[parentPid.Value] = snapshot.StartTimeUtc;
                return true;
            }

            return expectedStartTime == snapshot.StartTimeUtc;
        }
    }

    private static ParentProcessSnapshot? ReadSnapshot(int parentPid)
    {
        try
        {
            using var process = Process.GetProcessById(parentPid);
            return new ParentProcessSnapshot(!process.HasExited, process.StartTime.ToUniversalTime());
        }
        catch
        {
            return null;
        }
    }
}

internal sealed record ParentProcessSnapshot(bool IsAlive, DateTimeOffset StartTimeUtc)
{
    public bool HasExited => !IsAlive;
}
