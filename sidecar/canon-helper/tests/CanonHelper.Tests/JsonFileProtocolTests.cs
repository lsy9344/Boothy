using System.Text;
using System.Text.Json;
using CanonHelper.Protocol;
using CanonHelper.Runtime;
using Xunit;

namespace CanonHelper.Tests;

public sealed class JsonFileProtocolTests : IDisposable
{
    private static readonly JsonSerializerOptions JsonOptions = new(JsonSerializerDefaults.Web)
    {
        WriteIndented = false,
    };

    private readonly string _runtimeRoot = Path.Combine(
        Path.GetTempPath(),
        $"boothy-canon-helper-tests-{Guid.NewGuid():N}"
    );

    [Fact]
    public void ReadRequests_skips_persisted_request_ids_across_protocol_restarts_and_only_reads_new_appends()
    {
        var paths = CreateSessionPaths();
        AppendRequest(paths, CreateRequest("request_existing_1"));
        AppendRequest(paths, CreateRequest("request_existing_2"));

        var persisted = new JsonFileProtocol(paths, echoJsonToStdout: false);
        persisted.AppendProcessedRequestId("request_existing_1");
        persisted.AppendProcessedRequestId("request_existing_2");

        var protocol = new JsonFileProtocol(paths, echoJsonToStdout: false);
        var processedRequestIds = protocol.ReadProcessedRequestIds();

        Assert.Empty(protocol.ReadRequests(processedRequestIds));

        var latestRequest = CreateRequest("request_new_1");
        AppendRequest(paths, latestRequest);

        var requests = protocol.ReadRequests(processedRequestIds);

        var captured = Assert.Single(requests);
        Assert.Equal(latestRequest.RequestId, captured.RequestId);
        Assert.Empty(protocol.ReadRequests(processedRequestIds));
    }

    [Fact]
    public void ReadProcessedRequestIds_backfills_already_handled_requests_from_existing_event_log()
    {
        var paths = CreateSessionPaths();
        var priorRequest = CreateRequest("request_existing_event_1");
        AppendRequest(paths, priorRequest);
        AppendEvent(
            paths.EventsLogPath,
            new
            {
                schemaVersion = CanonHelperSchemas.CaptureAccepted,
                type = "capture-accepted",
                sessionId = priorRequest.SessionId,
                requestId = priorRequest.RequestId,
                detailCode = "capture-in-flight",
            }
        );

        var protocol = new JsonFileProtocol(paths, echoJsonToStdout: false);
        var processedRequestIds = protocol.ReadProcessedRequestIds();

        Assert.Contains(priorRequest.RequestId, processedRequestIds);

        var latestRequest = CreateRequest("request_new_event_1");
        AppendRequest(paths, latestRequest);

        var requests = protocol.ReadRequests(processedRequestIds);

        var captured = Assert.Single(requests);
        Assert.Equal(latestRequest.RequestId, captured.RequestId);
    }

    [Fact]
    public void ReadRequests_buffers_incomplete_trailing_lines_until_the_json_line_is_finished()
    {
        var paths = CreateSessionPaths();
        var protocol = new JsonFileProtocol(paths, echoJsonToStdout: false);

        var request = CreateRequest("request_partial_1");
        var line = JsonSerializer.Serialize(request, JsonOptions);
        var splitIndex = line.Length / 2;

        AppendRaw(paths.RequestLogPath, line[..splitIndex]);

        Assert.Empty(protocol.ReadRequests());

        AppendRaw(paths.RequestLogPath, line[splitIndex..] + Environment.NewLine);

        var requests = protocol.ReadRequests();

        var captured = Assert.Single(requests);
        Assert.Equal(request.RequestId, captured.RequestId);
        Assert.Empty(protocol.ReadRequests());
    }

    public void Dispose()
    {
        if (Directory.Exists(_runtimeRoot))
        {
            Directory.Delete(_runtimeRoot, recursive: true);
        }
    }

    private SessionPaths CreateSessionPaths()
    {
        var paths = new SessionPaths(_runtimeRoot, "session_duplicate_shutter_fix");
        paths.EnsureExists();
        return paths;
    }

    private static CaptureRequestMessage CreateRequest(string requestId)
    {
        return new CaptureRequestMessage(
            CanonHelperSchemas.CaptureRequest,
            "request-capture",
            "session_duplicate_shutter_fix",
            requestId,
            "2026-03-29T00:00:00.0000000+00:00",
            "preset_soft-glow",
            "2026.03.29"
        );
    }

    private static void AppendRequest(SessionPaths paths, CaptureRequestMessage request)
    {
        var line = JsonSerializer.Serialize(request, JsonOptions) + Environment.NewLine;
        AppendRaw(paths.RequestLogPath, line);
    }

    private static void AppendEvent(string path, object message)
    {
        var line = JsonSerializer.Serialize(message, JsonOptions) + Environment.NewLine;
        AppendRaw(path, line);
    }

    private static void AppendRaw(string path, string contents)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        File.AppendAllText(path, contents, new UTF8Encoding(false));
    }
}
