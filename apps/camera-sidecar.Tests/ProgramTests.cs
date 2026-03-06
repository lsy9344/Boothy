using System.Reflection;
using System.Threading.Tasks;
using Xunit;

namespace Boothy.CameraSidecar.Tests;

public class ProgramTests
{
    [Theory]
    [InlineData(true, false)]
    [InlineData(false, true)]
    public void ShouldPrepareCameraForSessionDestinationUpdate_MatchesRequestContext(
        bool isCaptureRequest,
        bool expected
    )
    {
        var sidecarAssembly = Assembly.Load("Boothy.CameraSidecar");
        var method = sidecarAssembly
            .GetType("Boothy.CameraSidecar.Program")
            ?.GetMethod(
                "ShouldPrepareCameraForSessionDestinationUpdate",
                BindingFlags.Static | BindingFlags.NonPublic
            );

        Assert.NotNull(method);

        var result = method!.Invoke(null, new object[] { isCaptureRequest });

        Assert.IsType<bool>(result);
        Assert.Equal(expected, (bool)result!);
    }

    [Fact]
    public async Task AwaitStartupProbeBeforeCaptureAsync_ReturnsImmediatelyWhenProbeAlreadyCompleted()
    {
        var result = await InvokeAwaitStartupProbeBeforeCaptureAsync(Task.CompletedTask, 50);

        Assert.True(result);
    }

    [Fact]
    public async Task AwaitStartupProbeBeforeCaptureAsync_TimesOutWhenProbeNeverCompletes()
    {
        var result = await InvokeAwaitStartupProbeBeforeCaptureAsync(
            new TaskCompletionSource().Task,
            10
        );

        Assert.False(result);
    }

    private static async Task<bool> InvokeAwaitStartupProbeBeforeCaptureAsync(
        Task startupProbeTask,
        int timeoutMs
    )
    {
        var sidecarAssembly = Assembly.Load("Boothy.CameraSidecar");
        var method = sidecarAssembly
            .GetType("Boothy.CameraSidecar.Program")
            ?.GetMethod(
                "AwaitStartupProbeBeforeCaptureAsync",
                BindingFlags.Static | BindingFlags.NonPublic
            );

        Assert.NotNull(method);

        var task = method!.Invoke(null, new object[] { startupProbeTask, timeoutMs });

        var typedTask = Assert.IsAssignableFrom<Task<bool>>(task);
        return await typedTask;
    }
}
