using System.Text.Json.Serialization;

namespace CanonHelper.Protocol;

internal static class CanonHelperSchemas
{
    public const string HelperReady = "canon-helper-ready/v1";
    public const string CameraStatus = "canon-helper-status/v1";
    public const string CaptureRequest = "canon-helper-request-capture/v1";
    public const string CaptureAccepted = "canon-helper-capture-accepted/v1";
    public const string FastPreviewReady = "canon-helper-fast-preview-ready/v1";
    public const string FastThumbnailAttempted = "canon-helper-fast-thumbnail-attempted/v1";
    public const string FastThumbnailFailed = "canon-helper-fast-thumbnail-failed/v1";
    public const string FileArrived = "canon-helper-file-arrived/v1";
    public const string RecoveryStatus = "canon-helper-recovery-status/v1";
    public const string HelperError = "canon-helper-error/v1";
}

internal sealed record HelperReadyMessage(
    [property: JsonPropertyName("schemaVersion")] string SchemaVersion,
    [property: JsonPropertyName("type")] string Type,
    string HelperVersion,
    string ProtocolVersion,
    string RuntimePlatform,
    string SdkFamily,
    string SdkVersion
);

internal sealed record CameraStatusMessage(
    [property: JsonPropertyName("schemaVersion")] string SchemaVersion,
    [property: JsonPropertyName("type")] string Type,
    string SessionId,
    ulong Sequence,
    string ObservedAt,
    string CameraState,
    string HelperState,
    string? CameraModel,
    string? RequestId,
    string? DetailCode
);

internal sealed record CaptureRequestMessage(
    [property: JsonPropertyName("schemaVersion")] string SchemaVersion,
    [property: JsonPropertyName("type")] string Type,
    string SessionId,
    string RequestId,
    string RequestedAt,
    string ActivePresetId,
    string ActivePresetVersion
);

internal sealed record CaptureAcceptedMessage(
    [property: JsonPropertyName("schemaVersion")] string SchemaVersion,
    [property: JsonPropertyName("type")] string Type,
    string SessionId,
    string RequestId,
    string? DetailCode
);

internal sealed record FastPreviewReadyMessage(
    [property: JsonPropertyName("schemaVersion")] string SchemaVersion,
    [property: JsonPropertyName("type")] string Type,
    string SessionId,
    string RequestId,
    string CaptureId,
    string ObservedAt,
    string FastPreviewPath,
    string? FastPreviewKind
);

internal sealed record FastThumbnailAttemptedMessage(
    [property: JsonPropertyName("schemaVersion")] string SchemaVersion,
    [property: JsonPropertyName("type")] string Type,
    string SessionId,
    string RequestId,
    string CaptureId,
    string ObservedAt,
    string? FastPreviewKind
);

internal sealed record FastThumbnailFailedMessage(
    [property: JsonPropertyName("schemaVersion")] string SchemaVersion,
    [property: JsonPropertyName("type")] string Type,
    string SessionId,
    string RequestId,
    string CaptureId,
    string ObservedAt,
    string DetailCode,
    string? FastPreviewKind
);

internal sealed record FileArrivedMessage(
    [property: JsonPropertyName("schemaVersion")] string SchemaVersion,
    [property: JsonPropertyName("type")] string Type,
    string SessionId,
    string RequestId,
    string CaptureId,
    string ArrivedAt,
    string RawPath,
    string? FastPreviewPath,
    string? FastPreviewKind
);

internal sealed record RecoveryStatusMessage(
    [property: JsonPropertyName("schemaVersion")] string SchemaVersion,
    [property: JsonPropertyName("type")] string Type,
    string SessionId,
    string RecoveryState,
    string ObservedAt,
    string? DetailCode
);

internal sealed record HelperErrorMessage(
    [property: JsonPropertyName("schemaVersion")] string SchemaVersion,
    [property: JsonPropertyName("type")] string Type,
    string? SessionId,
    string? ObservedAt,
    string DetailCode,
    string? Message
);

internal sealed record SelfCheckReport(
    string HelperVersion,
    string ProtocolVersion,
    string RuntimePlatform,
    string SdkFamily,
    string SdkVersion,
    bool IsWindows,
    bool RuntimeDllPresent,
    bool SdkSourcePresent,
    bool SdkInitialized,
    int CameraCount,
    string? DetailCode,
    string? Message
);
