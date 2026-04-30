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
    private long _requestReadOffset;
    private string _pendingRequestLine = string.Empty;

    public JsonFileProtocol(SessionPaths paths, bool echoJsonToStdout)
    {
        _paths = paths;
        _echoJsonToStdout = echoJsonToStdout;
    }

    public IReadOnlySet<string> ReadProcessedRequestIds()
    {
        var processedRequestIds = new HashSet<string>(StringComparer.Ordinal);

        if (File.Exists(_paths.ProcessedRequestsPath))
        {
            foreach (var line in File.ReadLines(_paths.ProcessedRequestsPath, Encoding.UTF8))
            {
                var requestId = line.Trim().TrimStart('\uFEFF');
                if (!string.IsNullOrWhiteSpace(requestId))
                {
                    processedRequestIds.Add(requestId);
                }
            }
        }

        if (File.Exists(_paths.EventsLogPath))
        {
            foreach (var line in File.ReadLines(_paths.EventsLogPath, Encoding.UTF8))
            {
                TryCollectProcessedRequestIdFromEvent(line, processedRequestIds);
            }
        }

        return processedRequestIds;
    }

    public void AppendProcessedRequestId(string requestId)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(_paths.ProcessedRequestsPath)!);
        File.AppendAllText(
            _paths.ProcessedRequestsPath,
            requestId + Environment.NewLine,
            Utf8WithoutBom
        );
    }

    public IReadOnlyList<CaptureRequestMessage> ReadRequests(
        IReadOnlySet<string>? processedRequestIds = null
    )
    {
        if (!File.Exists(_paths.RequestLogPath))
        {
            _requestReadOffset = 0;
            _pendingRequestLine = string.Empty;
            return [];
        }

        using var stream = new FileStream(
            _paths.RequestLogPath,
            FileMode.Open,
            FileAccess.Read,
            FileShare.ReadWrite
        );

        if (_requestReadOffset > stream.Length)
        {
            _requestReadOffset = 0;
            _pendingRequestLine = string.Empty;
        }

        if (_requestReadOffset == stream.Length)
        {
            return [];
        }

        stream.Seek(_requestReadOffset, SeekOrigin.Begin);

        using var reader = new StreamReader(
            stream,
            Encoding.UTF8,
            detectEncodingFromByteOrderMarks: true
        );
        var requestChunk = reader.ReadToEnd();
        _requestReadOffset = stream.Position;

        if (string.IsNullOrEmpty(requestChunk) && string.IsNullOrEmpty(_pendingRequestLine))
        {
            return [];
        }

        var bufferedRequests = string.Concat(_pendingRequestLine, requestChunk);
        var requests = new List<CaptureRequestMessage>();
        var nextLineStart = 0;

        while (nextLineStart < bufferedRequests.Length)
        {
            var newlineIndex = bufferedRequests.IndexOf('\n', nextLineStart);
            if (newlineIndex < 0)
            {
                break;
            }

            var line = bufferedRequests[nextLineStart..newlineIndex]
                .TrimEnd('\r')
                .TrimStart('\uFEFF');

            if (!string.IsNullOrWhiteSpace(line))
            {
                var request = JsonSerializer.Deserialize<CaptureRequestMessage>(line, JsonOptions);
                if (
                    request is not null
                    && !(processedRequestIds?.Contains(request.RequestId) ?? false)
                )
                {
                    requests.Add(request);
                }
            }

            nextLineStart = newlineIndex + 1;
        }

        _pendingRequestLine = bufferedRequests[nextLineStart..];

        return requests;
    }

    private static void TryCollectProcessedRequestIdFromEvent(
        string line,
        HashSet<string> processedRequestIds
    )
    {
        if (string.IsNullOrWhiteSpace(line))
        {
            return;
        }

        try
        {
            using var document = JsonDocument.Parse(line.TrimStart('\uFEFF'));
            var root = document.RootElement;

            if (
                !root.TryGetProperty("type", out var typeProperty)
                || !root.TryGetProperty("requestId", out var requestIdProperty)
            )
            {
                return;
            }

            var messageType = typeProperty.GetString();
            var requestId = requestIdProperty.GetString();

            if (
                string.IsNullOrWhiteSpace(requestId)
                || (
                    messageType != "capture-accepted"
                    && messageType != "file-arrived"
                    && messageType != "helper-error"
                )
            )
            {
                return;
            }

            processedRequestIds.Add(requestId);
        }
        catch (JsonException)
        {
        }
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
