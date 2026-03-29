namespace CanonHelper;

internal sealed record CanonHelperOptions(
    bool VersionOnly = false,
    bool SelfCheckOnly = false,
    bool EchoJsonToStdout = false,
    string? RuntimeRoot = null,
    string? SessionId = null,
    string? SdkRoot = null,
    int? ParentPid = null,
    int PollIntervalMs = 250,
    int StatusIntervalMs = 1_000
)
{
    public static CanonHelperOptions Parse(string[] args)
    {
        var options = new CanonHelperOptions();

        for (var index = 0; index < args.Length; index++)
        {
            var argument = args[index];

            switch (argument)
            {
                case "--version":
                    options = options with { VersionOnly = true };
                    break;
                case "--self-check":
                    options = options with { SelfCheckOnly = true };
                    break;
                case "--stdio":
                    options = options with { EchoJsonToStdout = true };
                    break;
                case "--runtime-root":
                    options = options with { RuntimeRoot = ReadValue(args, ref index, argument) };
                    break;
                case "--session-id":
                    options = options with { SessionId = ReadValue(args, ref index, argument) };
                    break;
                case "--sdk-root":
                    options = options with { SdkRoot = ReadValue(args, ref index, argument) };
                    break;
                case "--parent-pid":
                    options = options with
                    {
                        ParentPid = ParsePositiveInt(
                            ReadValue(args, ref index, argument),
                            argument
                        ),
                    };
                    break;
                case "--poll-interval-ms":
                    options = options with
                    {
                        PollIntervalMs = ParsePositiveInt(
                            ReadValue(args, ref index, argument),
                            argument
                        ),
                    };
                    break;
                case "--status-interval-ms":
                    options = options with
                    {
                        StatusIntervalMs = ParsePositiveInt(
                            ReadValue(args, ref index, argument),
                            argument
                        ),
                    };
                    break;
                default:
                    throw new ArgumentException($"알 수 없는 인자예요: {argument}");
            }
        }

        return options;
    }

    public void ValidateForRuntime()
    {
        if (VersionOnly || SelfCheckOnly)
        {
            return;
        }

        if (string.IsNullOrWhiteSpace(RuntimeRoot))
        {
            throw new ArgumentException("--runtime-root 값이 필요해요.");
        }

        if (string.IsNullOrWhiteSpace(SessionId))
        {
            throw new ArgumentException("--session-id 값이 필요해요.");
        }
    }

    private static string ReadValue(string[] args, ref int index, string argument)
    {
        if (index + 1 >= args.Length)
        {
            throw new ArgumentException($"{argument} 뒤에 값이 필요해요.");
        }

        index += 1;
        return args[index];
    }

    private static int ParsePositiveInt(string value, string argument)
    {
        if (!int.TryParse(value, out var parsed) || parsed <= 0)
        {
            throw new ArgumentException($"{argument} 값은 1 이상의 정수여야 해요.");
        }

        return parsed;
    }
}
