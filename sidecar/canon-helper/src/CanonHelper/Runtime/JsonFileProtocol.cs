using System.Text;
using System.Text.Json;
using CanonHelper.Protocol;

namespace CanonHelper.Runtime;

internal sealed class JsonFileProtocol
{
    private static readonly JsonSerializerOptions JsonOptions = new(JsonSerializerDefaults.Web)
    {
        WriteIndented = false,
    };
    private static readonly UTF8Encoding Utf8WithoutBom = new(false);

    private readonly SessionPaths _paths;
    private readonly bool _echoJsonToStdout;

    public JsonFileProtocol(SessionPaths paths, bool echoJsonToStdout)
    {
        _paths = paths;
        _echoJsonToStdout = echoJsonToStdout;
    }

    public IReadOnlyList<CaptureRequestMessage> ReadRequests()
    {
        if (!File.Exists(_paths.RequestLogPath))
        {
            return [];
        }

        var requests = new List<CaptureRequestMessage>();

        foreach (var line in File.ReadLines(_paths.RequestLogPath, Encoding.UTF8))
        {
            if (string.IsNullOrWhiteSpace(line))
            {
                continue;
            }

            var request = JsonSerializer.Deserialize<CaptureRequestMessage>(line, JsonOptions);
            if (request is not null)
            {
                requests.Add(request);
            }
        }

        return requests;
    }

    public void WriteStatus(CameraStatusMessage message)
    {
        WriteAllText(_paths.StatusPath, message);
    }

    public void AppendEvent<T>(T message)
    {
        AppendJsonLine(_paths.EventsLogPath, message);
    }

    public void EmitStdout<T>(T message)
    {
        if (!_echoJsonToStdout)
        {
            return;
        }

        Console.Out.WriteLine(JsonSerializer.Serialize(message, JsonOptions));
        Console.Out.Flush();
    }

    private void AppendJsonLine<T>(string path, T message)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        var line = JsonSerializer.Serialize(message, JsonOptions);
        File.AppendAllText(path, line + Environment.NewLine, Utf8WithoutBom);
        EmitStdout(message);
    }

    private void WriteAllText<T>(string path, T message)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        var contents = JsonSerializer.Serialize(message, JsonOptions);
        File.WriteAllText(path, contents, Utf8WithoutBom);
        EmitStdout(message);
    }
}
