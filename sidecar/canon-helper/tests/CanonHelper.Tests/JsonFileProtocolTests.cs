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

    [Fact]
    public void Source_events_serialize_the_correlation_fields_consumed_by_the_host()
    {
        var arrived = new SourceObjectArrivedMessage(
            CanonHelperSchemas.SourceObjectArrived,
            "source-object-arrived",
            "session_1",
            "request_1",
            "capture_1",
            "2026-08-12T10:15:31Z",
            "C:\\runtime\\paired.jpg",
            1234,
            1,
            42,
            "jpeg",
            false,
            "sdkformat"
        );
        var capability = new ImageQualityCapabilityMessage(
            CanonHelperSchemas.ImageQualityCapability,
            "image-quality-capability",
            "session_1",
            "2026-08-12T10:15:30Z",
            1_723_460_130_123_456,
            true,
            0x0010ff0f,
            [0x0010ff0f, 0x00640013],
            true
        );
        var fileArrived = new FileArrivedMessage(
            CanonHelperSchemas.FileArrived,
            "file-arrived",
            "session_1",
            "request_1",
            "capture_1",
            "2026-08-12T10:15:32Z",
            "C:\\runtime\\capture_1.cr2",
            null,
            null,
            0,
            42,
            "raw",
            false
        );
        var rejected = new SourceObjectRejectedMessage(
            CanonHelperSchemas.SourceObjectRejected,
            "source-object-rejected",
            "session_1",
            "request_1",
            "capture_1",
            "2026-08-12T10:15:33Z",
            "group-mismatch",
            "jpeg",
            1,
            43,
            false,
            "sdkformat",
            "IMG_0002.JPG"
        );
        var settingWarning = new CameraSettingWarningMessage(
            CanonHelperSchemas.CameraSettingWarning,
            "camera-setting-warning",
            "session_1",
            "request_1",
            "capture_1",
            "2026-08-12T10:15:34Z",
            "image-quality-restore-failed"
        );

        using var arrivedJson = JsonDocument.Parse(JsonSerializer.Serialize(arrived, JsonOptions));
        using var capabilityJson = JsonDocument.Parse(
            JsonSerializer.Serialize(capability, JsonOptions)
        );
        using var fileArrivedJson = JsonDocument.Parse(
            JsonSerializer.Serialize(fileArrived, JsonOptions)
        );
        using var rejectedJson = JsonDocument.Parse(JsonSerializer.Serialize(rejected, JsonOptions));
        using var settingWarningJson = JsonDocument.Parse(
            JsonSerializer.Serialize(settingWarning, JsonOptions)
        );

        Assert.Equal(1, arrivedJson.RootElement.GetProperty("objectIndex").GetInt32());
        Assert.Equal(42u, arrivedJson.RootElement.GetProperty("groupId").GetUInt32());
        Assert.Equal("jpeg", arrivedJson.RootElement.GetProperty("objectRole").GetString());
        Assert.False(
            arrivedJson.RootElement.GetProperty("usedFallbackCorrelation").GetBoolean()
        );
        Assert.True(
            capabilityJson.RootElement.GetProperty("rawPlusJpegSupported").GetBoolean()
        );
        Assert.Equal(
            1_723_460_130_123_456,
            capabilityJson.RootElement.GetProperty("probedAtHostMicros").GetInt64()
        );
        Assert.Equal(0, fileArrivedJson.RootElement.GetProperty("objectIndex").GetInt32());
        Assert.Equal("raw", fileArrivedJson.RootElement.GetProperty("objectRole").GetString());
        Assert.Equal(
            "group-mismatch",
            rejectedJson.RootElement.GetProperty("rejectReason").GetString()
        );
        Assert.Equal(1, rejectedJson.RootElement.GetProperty("objectIndex").GetInt32());
        Assert.Equal(
            "image-quality-restore-failed",
            settingWarningJson.RootElement.GetProperty("detailCode").GetString()
        );
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
