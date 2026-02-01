using System;
using System.Collections.Generic;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Boothy.CameraSidecar.IPC
{
    /// <summary>
    /// IPC Protocol Version - Must match Rust side (apps/boothy/src-tauri/src/camera/ipc_models.rs)
    /// </summary>
    public static class IpcProtocol
    {
        public const string Version = "1.0.0";
    }

    internal sealed class IpcMessageTypeJsonConverter : JsonStringEnumConverter
    {
        public IpcMessageTypeJsonConverter()
            : base(JsonNamingPolicy.CamelCase, allowIntegerValues: false)
        {
        }
    }

    internal sealed class IpcErrorCodeJsonConverter : JsonConverter<IpcErrorCode>
    {
        public override IpcErrorCode Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
        {
            if (reader.TokenType != JsonTokenType.String)
            {
                throw new JsonException($"Expected string for {nameof(IpcErrorCode)}");
            }

            var raw = reader.GetString();
            if (string.IsNullOrWhiteSpace(raw))
            {
                throw new JsonException($"Empty {nameof(IpcErrorCode)}");
            }

            // Accept both legacy ("CameraNotConnected") and canonical ("CAMERA_NOT_CONNECTED") forms.
            if (Enum.TryParse(raw, ignoreCase: true, out IpcErrorCode parsed))
            {
                return parsed;
            }

            var normalized = raw.Replace("-", "_").Replace(" ", "_").Trim();
            foreach (var value in Enum.GetValues<IpcErrorCode>())
            {
                if (string.Equals(ToScreamingSnakeCase(value.ToString()), normalized, StringComparison.OrdinalIgnoreCase))
                {
                    return value;
                }
            }

            throw new JsonException($"Unknown {nameof(IpcErrorCode)}: {raw}");
        }

        public override void Write(Utf8JsonWriter writer, IpcErrorCode value, JsonSerializerOptions options)
        {
            writer.WriteStringValue(ToScreamingSnakeCase(value.ToString()));
        }

        private static string ToScreamingSnakeCase(string name)
        {
            if (string.IsNullOrEmpty(name))
            {
                return name;
            }

            var chars = new List<char>(name.Length + 8);
            for (int i = 0; i < name.Length; i++)
            {
                var c = name[i];
                if (char.IsUpper(c) && i > 0)
                {
                    var prev = name[i - 1];
                    if (char.IsLower(prev) || char.IsDigit(prev))
                    {
                        chars.Add('_');
                    }
                }
                chars.Add(char.ToUpperInvariant(c));
            }
            return new string(chars.ToArray());
        }
    }

    [JsonConverter(typeof(IpcMessageTypeJsonConverter))]
    public enum IpcMessageType
    {
        Request,
        Response,
        Event,
        Error
    }

    /// <summary>
    /// Base IPC Message Envelope matching Rust IpcMessage structure
    /// </summary>
    public class IpcMessage
    {
        [JsonPropertyName("protocolVersion")]
        public string ProtocolVersion { get; set; } = IpcProtocol.Version;

        [JsonPropertyName("messageType")]
        public IpcMessageType MessageType { get; set; }

        [JsonPropertyName("requestId")]
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        public string? RequestId { get; set; }

        [JsonPropertyName("correlationId")]
        public string CorrelationId { get; set; } = "";

        [JsonPropertyName("timestamp")]
        public DateTime Timestamp { get; set; } = DateTime.UtcNow;

        [JsonPropertyName("method")]
        public string Method { get; set; } = "";

        [JsonPropertyName("payload")]
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        public JsonElement? Payload { get; set; }

        [JsonPropertyName("error")]
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        public IpcError? Error { get; set; }

        /// <summary>
        /// Create a new event message
        /// </summary>
        public static IpcMessage NewEvent(string method, string correlationId, object payload)
        {
            return new IpcMessage
            {
                ProtocolVersion = IpcProtocol.Version,
                MessageType = IpcMessageType.Event,
                CorrelationId = correlationId,
                Method = method,
                Payload = JsonSerializer.SerializeToElement(payload),
                Timestamp = DateTime.UtcNow
            };
        }

        /// <summary>
        /// Create a new response message
        /// </summary>
        public static IpcMessage NewResponse(string method, string correlationId, string requestId, object payload)
        {
            return new IpcMessage
            {
                ProtocolVersion = IpcProtocol.Version,
                MessageType = IpcMessageType.Response,
                RequestId = requestId,
                CorrelationId = correlationId,
                Method = method,
                Payload = JsonSerializer.SerializeToElement(payload),
                Timestamp = DateTime.UtcNow
            };
        }

        /// <summary>
        /// Create a new error message
        /// </summary>
        public static IpcMessage NewError(string method, string correlationId, string? requestId, IpcError error)
        {
            return new IpcMessage
            {
                ProtocolVersion = IpcProtocol.Version,
                MessageType = IpcMessageType.Error,
                RequestId = requestId,
                CorrelationId = correlationId,
                Method = method,
                Error = error,
                Timestamp = DateTime.UtcNow
            };
        }
    }

    [JsonConverter(typeof(IpcErrorCodeJsonConverter))]
    public enum IpcErrorCode
    {
        VersionMismatch,

        Timeout,

        Disconnect,

        CameraNotConnected,

        CaptureFailed,

        FileTransferFailed,

        InvalidPayload,

        SessionDestinationNotSet,

        FileSystemError,

        Unknown
    }

    public class IpcError
    {
        [JsonPropertyName("code")]
        public IpcErrorCode Code { get; set; }

        [JsonPropertyName("message")]
        public string Message { get; set; } = "";

        [JsonPropertyName("context")]
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        public Dictionary<string, string>? Context { get; set; }
    }

    // ============================================================================
    // Event Payloads
    // ============================================================================

    public class PhotoTransferredPayload
    {
        [JsonPropertyName("path")]
        public string Path { get; set; } = "";

        [JsonPropertyName("transferredAt")]
        public DateTime TransferredAt { get; set; }

        [JsonPropertyName("originalFilename")]
        public string OriginalFilename { get; set; } = "";

        [JsonPropertyName("fileSize")]
        public long FileSize { get; set; }
    }

    public class CaptureStartedPayload
    {
        [JsonPropertyName("startedAt")]
        public DateTime StartedAt { get; set; }
    }

    public class CameraErrorPayload
    {
        [JsonPropertyName("error")]
        public IpcError Error { get; set; } = new();
    }

    // ============================================================================
    // Request/Response Payloads
    // ============================================================================

    public class SetSessionDestinationRequest
    {
        [JsonPropertyName("destinationPath")]
        public string DestinationPath { get; set; } = "";

        [JsonPropertyName("sessionName")]
        public string SessionName { get; set; } = "";
    }

    public class SetSessionDestinationResponse
    {
        [JsonPropertyName("success")]
        public bool Success { get; set; }

        [JsonPropertyName("destinationPath")]
        public string DestinationPath { get; set; } = "";
    }

    public class CameraStatusResponse
    {
        [JsonPropertyName("connected")]
        public bool Connected { get; set; }

        [JsonPropertyName("cameraDetected")]
        public bool CameraDetected { get; set; }

        [JsonPropertyName("sessionDestination")]
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        public string? SessionDestination { get; set; }

        [JsonPropertyName("cameraModel")]
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        public string? CameraModel { get; set; }
    }

    // ============================================================================
    // Helper Functions
    // ============================================================================

    public static class IpcHelpers
    {
        public static string GenerateCorrelationId()
        {
            long timestamp = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
            Guid guid = Guid.NewGuid();
            return $"corr-{timestamp}-{guid}";
        }

        public static string GenerateRequestId()
        {
            return $"req-{Guid.NewGuid()}";
        }
    }
}
