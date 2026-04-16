using CanonHelper.Runtime;
using Xunit;

namespace CanonHelper.Tests;

public sealed class CanonHelperServiceTests
{
    [Theory]
    [InlineData(false, 0, true)]
    [InlineData(false, 1, false)]
    [InlineData(false, 3, false)]
    [InlineData(true, 0, false)]
    [InlineData(true, 2, false)]
    public void ShouldRunPendingFastPreviewMaintenance_defers_optional_preview_work_when_capture_work_is_pending(
        bool hasActiveCaptureTask,
        int pendingRequestCount,
        bool expected
    )
    {
        Assert.Equal(
            expected,
            CanonHelperService.ShouldRunPendingFastPreviewMaintenance(
                hasActiveCaptureTask,
                pendingRequestCount
            )
        );
    }

    [Theory]
    [InlineData(false, 0, true)]
    [InlineData(false, 1, false)]
    [InlineData(true, 0, false)]
    public void ShouldRunPreviewBackfillMaintenance_rechecks_pending_capture_requests_before_expensive_backfill(
        bool hasActiveCaptureTask,
        int pendingRequestCount,
        bool expected
    )
    {
        Assert.Equal(
            expected,
            CanonHelperService.ShouldRunPreviewBackfillMaintenance(
                hasActiveCaptureTask,
                pendingRequestCount
            )
        );
    }
}
