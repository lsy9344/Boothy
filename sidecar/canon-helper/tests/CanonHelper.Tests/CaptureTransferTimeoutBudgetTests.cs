using CanonHelper.Runtime;
using Xunit;

namespace CanonHelper.Tests;

public sealed class CaptureTransferTimeoutBudgetTests
{
    [Fact]
    public void MissingTransferStartTimesOutOnceTheBudgetExpires()
    {
        var acceptedAt = DateTimeOffset.Parse("2026-04-05T12:00:00Z");
        var now = acceptedAt.AddMilliseconds(8001);

        var timedOut = CaptureTransferTimeoutBudget.HasExceeded(
            acceptedAt,
            downloadStarted: false,
            timeout: TimeSpan.FromMilliseconds(8000),
            now
        );

        Assert.True(timedOut);
    }

    [Fact]
    public void StartedTransferDoesNotTripTheMissingTransferBudget()
    {
        var acceptedAt = DateTimeOffset.Parse("2026-04-05T12:00:00Z");
        var now = acceptedAt.AddMilliseconds(12000);

        var timedOut = CaptureTransferTimeoutBudget.HasExceeded(
            acceptedAt,
            downloadStarted: true,
            timeout: TimeSpan.FromMilliseconds(8000),
            now
        );

        Assert.False(timedOut);
    }
}
