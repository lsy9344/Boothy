using System.Reflection;
using CanonHelper.Runtime;
using Xunit;

namespace CanonHelper.Tests;

public sealed class TimeoutPolicyTests
{
    [Fact]
    public void ResolveCaptureCompletionTimeout_uses_the_new_default_headroom_without_overrides()
    {
        var runtimeRoot = Path.Combine(
            Path.GetTempPath(),
            $"boothy-helper-timeout-default-{Guid.NewGuid():N}"
        );
        Directory.CreateDirectory(runtimeRoot);
        Environment.SetEnvironmentVariable("BOOTHY_CAPTURE_TIMEOUT_MS", null);

        var timeout = ResolveCaptureCompletionTimeout(runtimeRoot);

        Assert.Equal(TimeSpan.FromMilliseconds(45_000), timeout);
    }

    private static TimeSpan ResolveCaptureCompletionTimeout(string runtimeRoot)
    {
        var method = typeof(CanonSdkCamera).GetMethod(
            "ResolveCaptureCompletionTimeout",
            BindingFlags.NonPublic | BindingFlags.Static
        );

        Assert.NotNull(method);

        return Assert.IsType<TimeSpan>(method!.Invoke(null, [runtimeRoot]));
    }
}
