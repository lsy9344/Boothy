using CanonHelper.Runtime;
using EDSDKLib;
using Xunit;

namespace CanonHelper.Tests;

/// <summary>
/// HV-14 실장비 회차의 중단 원인 2종(<c>camera-busy</c>, <c>capture-download-timeout</c>)에
/// 대한 helper 쪽 방어선.
/// </summary>
/// <remarks>
/// DEVICE_BUSY는 셔터가 눌리지 않았다는 뜻이므로 재시도가 이중 촬영을 만들지 않는다.
/// paired(RAW+JPEG) 촬영은 transfer object가 둘이라 RAW handoff 예산에 allowance를 더한다.
/// </remarks>
public sealed class CaptureRetryPolicyTests
{
    [Fact]
    public void Device_busy_is_retried_within_the_bounded_limit()
    {
        for (var attempt = 0; attempt < CanonSdkCamera.ShutterBusyRetryLimit; attempt++)
        {
            Assert.True(CanonSdkCamera.ShouldRetryShutterBusy(EDSDK.EDS_ERR_DEVICE_BUSY, attempt));
        }
    }

    [Fact]
    public void Device_busy_stops_retrying_after_the_limit()
    {
        Assert.False(
            CanonSdkCamera.ShouldRetryShutterBusy(
                EDSDK.EDS_ERR_DEVICE_BUSY,
                CanonSdkCamera.ShutterBusyRetryLimit
            )
        );
    }

    [Fact]
    public void Success_and_other_errors_are_never_retried()
    {
        Assert.False(CanonSdkCamera.ShouldRetryShutterBusy(EDSDK.EDS_ERR_OK, 0));
        Assert.False(CanonSdkCamera.ShouldRetryShutterBusy(EDSDK.EDS_ERR_TAKE_PICTURE_AF_NG, 0));
    }

    [Fact]
    public void Paired_capture_extends_the_raw_handoff_budget()
    {
        var baseTimeout = TimeSpan.FromSeconds(30);

        Assert.Equal(
            baseTimeout + CanonSdkCamera.PairedCaptureCompletionAllowance,
            CanonSdkCamera.ApplyPairedCompletionAllowance(baseTimeout, expectedObjectCount: 2)
        );
    }

    /// <summary>단일 object(제품 기본 경로)의 예산은 그대로다. lane off 무영향 원칙.</summary>
    [Fact]
    public void Single_object_capture_keeps_the_existing_budget()
    {
        var baseTimeout = TimeSpan.FromSeconds(30);

        Assert.Equal(
            baseTimeout,
            CanonSdkCamera.ApplyPairedCompletionAllowance(baseTimeout, expectedObjectCount: 1)
        );
    }
}
