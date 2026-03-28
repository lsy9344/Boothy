using System.Text.Json;
using CanonHelper.Protocol;
using CanonHelper.Runtime;

namespace CanonHelper;

internal static class Program
{
    private static readonly JsonSerializerOptions JsonOptions = new(JsonSerializerDefaults.Web)
    {
        WriteIndented = false,
    };

    [STAThread]
    private static async Task<int> Main(string[] args)
    {
        try
        {
            var options = CanonHelperOptions.Parse(args);

            if (options.VersionOnly)
            {
                Console.WriteLine(
                    JsonSerializer.Serialize(
                        new
                        {
                            helperVersion = HelperVersion.Current,
                            protocolVersion = HelperVersion.ProtocolVersion,
                            sdkFamily = HelperVersion.SdkFamily,
                            sdkVersion = HelperVersion.SdkVersion,
                        },
                        JsonOptions
                    )
                );
                return 0;
            }

            if (options.SelfCheckOnly)
            {
                var result = CanonSdkCamera.RunSelfCheck(options.SdkRoot);
                var report = new SelfCheckReport(
                    HelperVersion.Current,
                    HelperVersion.ProtocolVersion,
                    Environment.OSVersion.VersionString,
                    HelperVersion.SdkFamily,
                    HelperVersion.SdkVersion,
                    result.IsWindows,
                    result.RuntimeDllPresent,
                    result.SdkSourcePresent,
                    result.SdkInitialized,
                    result.CameraCount,
                    result.DetailCode,
                    result.Message
                );
                Console.WriteLine(JsonSerializer.Serialize(report, JsonOptions));
                return result.RuntimeDllPresent && result.IsWindows ? 0 : 1;
            }

            options.ValidateForRuntime();

            using var shutdown = new CancellationTokenSource();
            Console.CancelKeyPress += (_, eventArgs) =>
            {
                eventArgs.Cancel = true;
                shutdown.Cancel();
            };

            using var service = new CanonHelperService(options);
            await service.RunAsync(shutdown.Token);
            return 0;
        }
        catch (OperationCanceledException)
        {
            return 0;
        }
        catch (Exception error)
        {
            var envelope = new HelperErrorMessage(
                CanonHelperSchemas.HelperError,
                "helper-error",
                null,
                DateTimeOffset.UtcNow.ToString("O"),
                "helper-startup-failed",
                error.Message
            );
            Console.Error.WriteLine(JsonSerializer.Serialize(envelope, JsonOptions));
            return 1;
        }
    }
}
