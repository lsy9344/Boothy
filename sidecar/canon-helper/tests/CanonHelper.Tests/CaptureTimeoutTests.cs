using System.Reflection;
using CanonHelper.Runtime;
using Xunit;

namespace CanonHelper.Tests;

public sealed class CaptureTimeoutTests
{
    [Fact]
    public void ResolveCaptureCompletionTimeout_uses_the_latest_default_budget_when_unset()
    {
        var runtimeRoot = Path.Combine(
            Path.GetTempPath(),
            $"boothy-canon-helper-timeout-{Guid.NewGuid():N}"
        );
        Directory.CreateDirectory(runtimeRoot);
        var originalTimeout = Environment.GetEnvironmentVariable("BOOTHY_CAPTURE_TIMEOUT_MS");
        Environment.SetEnvironmentVariable("BOOTHY_CAPTURE_TIMEOUT_MS", null);

        try
        {
            var method = typeof(CanonSdkCamera).GetMethod(
                "ResolveCaptureCompletionTimeout",
                BindingFlags.NonPublic | BindingFlags.Static
            );

            Assert.NotNull(method);

            var timeout = (TimeSpan)method!.Invoke(null, new object[] { runtimeRoot })!;

            Assert.Equal(TimeSpan.FromMilliseconds(45000), timeout);
        }
        finally
        {
            Environment.SetEnvironmentVariable("BOOTHY_CAPTURE_TIMEOUT_MS", originalTimeout);
            if (Directory.Exists(runtimeRoot))
            {
                Directory.Delete(runtimeRoot, recursive: true);
            }
        }
    }
}
