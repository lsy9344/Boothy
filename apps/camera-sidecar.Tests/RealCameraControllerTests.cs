using System.Reflection;
using Boothy.CameraSidecar.Camera;
using Xunit;

namespace Boothy.CameraSidecar.Tests;

public class RealCameraControllerTests
{
    private const uint DeviceBusyError = 0x00000081u;
    private const uint Ok = 0x00000000u;

    public static IEnumerable<object[]> DownloadableObjectEvents()
    {
        yield return new object[] { 0x00000208u, true };
        yield return new object[] { 0x00000204u, true };
        yield return new object[] { 0x00000209u, true };
        yield return new object[] { 0x0000020Au, false };
        yield return new object[] { 0x00000201u, false };
    }

    [Theory]
    [MemberData(nameof(DownloadableObjectEvents))]
    public void CanDownloadFromObjectEvent_RecognizesTransferCapableEvents(uint objectEvent, bool expected)
    {
        var method = typeof(RealCameraController).GetMethod(
            "CanDownloadFromObjectEvent",
            BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic
        );

        Assert.NotNull(method);

        var result = method!.Invoke(null, new object[] { objectEvent });

        Assert.IsType<bool>(result);
        Assert.Equal(expected, (bool)result!);
    }

    [Fact]
    public void ExecuteWithDeviceBusyRetry_RetriesBusySdkCallsUntilTheySucceed()
    {
        var method = typeof(RealCameraController).GetMethod(
            "ExecuteWithDeviceBusyRetry",
            BindingFlags.Static | BindingFlags.NonPublic
        );

        Assert.NotNull(method);

        var attempts = 0;
        Func<uint> operation = () =>
        {
            attempts += 1;
            return attempts < 3 ? DeviceBusyError : Ok;
        };

        var result = method!.Invoke(
            null,
            new object[] { operation, "corr-test", "capture", new[] { 0, 0 } }
        );

        Assert.IsType<uint>(result);
        Assert.Equal(Ok, (uint)result!);
        Assert.Equal(3, attempts);
    }

    [Fact]
    public void ExecuteWithDeviceBusyRetry_DoesNotRetryNonBusyErrors()
    {
        var method = typeof(RealCameraController).GetMethod(
            "ExecuteWithDeviceBusyRetry",
            BindingFlags.Static | BindingFlags.NonPublic
        );

        Assert.NotNull(method);

        const uint someOtherError = 0x00000020u;
        var attempts = 0;
        Func<uint> operation = () =>
        {
            attempts += 1;
            return someOtherError;
        };

        var result = method!.Invoke(
            null,
            new object[] { operation, "corr-test", "capture", new[] { 0, 0 } }
        );

        Assert.IsType<uint>(result);
        Assert.Equal(someOtherError, (uint)result!);
        Assert.Equal(1, attempts);
    }

    [Fact]
    public void GetHostCaptureConfigurationRetryDelays_SkipsRetriesForStatusProbe()
    {
        var method = typeof(RealCameraController).GetMethod(
            "GetHostCaptureConfigurationRetryDelays",
            BindingFlags.Static | BindingFlags.NonPublic
        );

        Assert.NotNull(method);

        var result = method!.Invoke(null, new object[] { false });

        Assert.IsType<int[]>(result);
        Assert.Empty((int[])result!);
    }

    [Fact]
    public void GetHostCaptureConfigurationRetryDelays_UsesStandardRetriesForCaptureFlow()
    {
        var method = typeof(RealCameraController).GetMethod(
            "GetHostCaptureConfigurationRetryDelays",
            BindingFlags.Static | BindingFlags.NonPublic
        );

        Assert.NotNull(method);

        var result = method!.Invoke(null, new object[] { true });

        Assert.IsType<int[]>(result);
        Assert.Equal(new[] { 150, 300, 500, 750 }, (int[])result!);
    }

    [Fact]
    public void ShouldReopenSessionForCapturePreparation_ReopensProbeOpenedSession()
    {
        var method = typeof(RealCameraController).GetMethod(
            "ShouldReopenSessionForCapturePreparation",
            BindingFlags.Static | BindingFlags.NonPublic
        );

        Assert.NotNull(method);

        var result = method!.Invoke(null, new object[] { true, true });

        Assert.IsType<bool>(result);
        Assert.True((bool)result!);
    }

    [Fact]
    public void ShouldReopenSessionForCapturePreparation_DoesNotReopenNormalCaptureSession()
    {
        var method = typeof(RealCameraController).GetMethod(
            "ShouldReopenSessionForCapturePreparation",
            BindingFlags.Static | BindingFlags.NonPublic
        );

        Assert.NotNull(method);

        var result = method!.Invoke(null, new object[] { false, true });

        Assert.IsType<bool>(result);
        Assert.False((bool)result!);
    }

    [Theory]
    [InlineData(true, false, true)]
    [InlineData(true, true, false)]
    [InlineData(false, false, false)]
    public void ShouldConfigureHostCaptureTarget_OnlyWhenCapturePrepIsNeeded(
        bool prepareHostCaptureTarget,
        bool hostCaptureTargetPrepared,
        bool expected
    )
    {
        var method = typeof(RealCameraController).GetMethod(
            "ShouldConfigureHostCaptureTarget",
            BindingFlags.Static | BindingFlags.NonPublic
        );

        Assert.NotNull(method);

        var result = method!.Invoke(
            null,
            new object[] { prepareHostCaptureTarget, hostCaptureTargetPrepared }
        );

        Assert.IsType<bool>(result);
        Assert.Equal(expected, (bool)result!);
    }
}
