using System.Text;
using CanonHelper.Protocol;

namespace CanonHelper.Runtime;

internal sealed class CanonHelperService : IDisposable
{
    private readonly CanonHelperOptions _options;
    private readonly SessionPaths _paths;
    private readonly JsonFileProtocol _protocol;
    private readonly CanonSdkCamera _camera = new();
    private readonly HashSet<string> _processedRequestIds;

    private ulong _statusSequence;
    private Task<CaptureDownloadResult>? _activeCaptureTask;
    private CaptureRequestMessage? _activeRequest;
    private CameraSnapshot? _lastWrittenSnapshot;
    private string? _lastReportedStartupFailureCode;
    private string? _lastStartupDebugSnapshotKey;

    public CanonHelperService(CanonHelperOptions options)
    {
        _options = options;
        _paths = new SessionPaths(options.RuntimeRoot!, options.SessionId!);
        _protocol = new JsonFileProtocol(_paths, options.EchoJsonToStdout);
        _processedRequestIds = new HashSet<string>(
            _protocol.ReadProcessedRequestIds(),
            StringComparer.Ordinal
        );
    }

    public async Task RunAsync(CancellationToken cancellationToken)
    {
        _paths.EnsureExists();
        _protocol.EmitStdout(
            new HelperReadyMessage(
                CanonHelperSchemas.HelperReady,
                "helper-ready",
                HelperVersion.Current,
                HelperVersion.ProtocolVersion,
                Environment.OSVersion.VersionString,
                HelperVersion.SdkFamily,
                HelperVersion.SdkVersion
            )
        );

        var nextStatusAt = DateTimeOffset.MinValue;

        while (!cancellationToken.IsCancellationRequested)
        {
            if (!ParentProcessMonitor.IsAlive(_options.ParentPid))
            {
                return;
            }

            _camera.PumpEvents();
            await _camera.EnsureConnectedAsync(cancellationToken);
            _camera.ForceCaptureTimeoutIfStuck(_paths.RuntimeRoot!, DateTimeOffset.UtcNow);
            await CompleteCaptureIfFinishedAsync();
            _camera.TryCompletePendingFastPreviewDownload();
            _camera.TryBackfillPreviewAssets(_paths);

            if (_activeCaptureTask is null)
            {
                foreach (var request in _protocol.ReadRequests(_processedRequestIds))
                {
                    if (_processedRequestIds.Contains(request.RequestId))
                    {
                        continue;
                    }

                    _protocol.AppendProcessedRequestId(request.RequestId);
                    _processedRequestIds.Add(request.RequestId);

                    if (request.SessionId != _paths.SessionId)
                    {
                        _protocol.AppendEvent(
                            new HelperErrorMessage(
                                CanonHelperSchemas.HelperError,
                                "helper-error",
                                request.SessionId,
                                UtcNow(),
                                "session-mismatch",
                                "현재 helper가 바인딩된 세션과 요청 세션이 다릅니다."
                            )
                        );
                        continue;
                    }

                    if (!_camera.IsReady)
                    {
                        var snapshot = _camera.Snapshot;
                        _protocol.AppendEvent(
                            new HelperErrorMessage(
                                CanonHelperSchemas.HelperError,
                                "helper-error",
                                request.SessionId,
                                UtcNow(),
                                snapshot.DetailCode ?? "camera-not-ready",
                                "카메라가 아직 촬영 가능한 상태가 아니에요."
                            )
                        );
                        continue;
                    }

                    _activeRequest = request;
                    _protocol.AppendEvent(
                        new CaptureAcceptedMessage(
                            CanonHelperSchemas.CaptureAccepted,
                            "capture-accepted",
                            request.SessionId,
                            request.RequestId,
                            "capture-in-flight"
                        )
                    );
                    _activeCaptureTask = _camera.CaptureAsync(
                        _paths,
                        request,
                        fastPreviewAttempted =>
                        {
                            _protocol.AppendEvent(
                                new FastThumbnailAttemptedMessage(
                                    CanonHelperSchemas.FastThumbnailAttempted,
                                    "fast-thumbnail-attempted",
                                    _paths.SessionId,
                                    fastPreviewAttempted.RequestId,
                                    fastPreviewAttempted.CaptureId,
                                    fastPreviewAttempted.ObservedAt.ToString("O"),
                                    fastPreviewAttempted.FastPreviewKind
                                )
                            );
                        },
                        fastPreview =>
                        {
                            _protocol.AppendEvent(
                                new FastPreviewReadyMessage(
                                    CanonHelperSchemas.FastPreviewReady,
                                    "fast-preview-ready",
                                    _paths.SessionId,
                                    fastPreview.RequestId,
                                    fastPreview.CaptureId,
                                    fastPreview.ObservedAt.ToString("O"),
                                    fastPreview.FastPreviewPath,
                                    fastPreview.FastPreviewKind
                                )
                            );
                        },
                        fastPreviewFailed =>
                        {
                            _protocol.AppendEvent(
                                new FastThumbnailFailedMessage(
                                    CanonHelperSchemas.FastThumbnailFailed,
                                    "fast-thumbnail-failed",
                                    _paths.SessionId,
                                    fastPreviewFailed.RequestId,
                                    fastPreviewFailed.CaptureId,
                                    fastPreviewFailed.ObservedAt.ToString("O"),
                                    fastPreviewFailed.DetailCode,
                                    fastPreviewFailed.FastPreviewKind
                                )
                            );
                        },
                        cancellationToken
                    );
                    break;
                }
            }

            if (ShouldWriteStatus(nextStatusAt))
            {
                WriteStatus();
                nextStatusAt = DateTimeOffset.UtcNow.AddMilliseconds(_options.StatusIntervalMs);
            }

            await Task.Delay(_options.PollIntervalMs, cancellationToken);
        }
    }

    public void Dispose()
    {
        _camera.Dispose();
    }

    private async Task CompleteCaptureIfFinishedAsync()
    {
        if (_activeCaptureTask is null || !_activeCaptureTask.IsCompleted)
        {
            return;
        }

        try
        {
            var result = await _activeCaptureTask;
            _protocol.AppendEvent(
                new FileArrivedMessage(
                    CanonHelperSchemas.FileArrived,
                    "file-arrived",
                    _paths.SessionId,
                    result.RequestId,
                    result.CaptureId,
                    result.ArrivedAt.ToString("O"),
                    result.RawPath,
                    result.FastPreviewPath,
                    result.FastPreviewKind
                )
            );
        }
        catch (CanonCaptureException error)
        {
            if (error.RecoveryRequired)
            {
                _protocol.AppendEvent(
                    new RecoveryStatusMessage(
                        CanonHelperSchemas.RecoveryStatus,
                        "recovery-status",
                        _paths.SessionId,
                        "recovering",
                        UtcNow(),
                        error.DetailCode
                    )
                );
            }

            _protocol.AppendEvent(
                new HelperErrorMessage(
                    CanonHelperSchemas.HelperError,
                    "helper-error",
                    _paths.SessionId,
                    UtcNow(),
                    error.DetailCode,
                    error.Message
                )
            );
        }
        finally
        {
            _activeCaptureTask = null;
            _activeRequest = null;
            WriteStatus();
        }
    }

    private void WriteStatus()
    {
        var snapshot = _camera.Snapshot;
        _lastWrittenSnapshot = snapshot;
        _statusSequence += 1;
        _protocol.WriteStatus(
            new CameraStatusMessage(
                CanonHelperSchemas.CameraStatus,
                "camera-status",
                _paths.SessionId,
                _statusSequence,
                UtcNow(),
                snapshot.CameraState,
                snapshot.HelperState,
                snapshot.CameraModel,
                snapshot.RequestId ?? _activeRequest?.RequestId,
                snapshot.DetailCode
            )
        );
        MaybeAppendStartupFailureEvent(snapshot);
        MaybeAppendStartupDebugLine(snapshot);
    }

    private bool ShouldWriteStatus(DateTimeOffset nextStatusAt)
    {
        var snapshot = _camera.Snapshot;
        return _lastWrittenSnapshot is null
            || _lastWrittenSnapshot != snapshot
            || DateTimeOffset.UtcNow >= nextStatusAt;
    }

    private static string UtcNow()
    {
        return DateTimeOffset.UtcNow.ToString("O");
    }

    private void MaybeAppendStartupFailureEvent(CameraSnapshot snapshot)
    {
        if (
            _activeCaptureTask is not null
            || _activeRequest is not null
            || snapshot.CameraState != "error"
            || snapshot.HelperState != "error"
            || !CanonSdkCamera.IsStartupConnectFailureDetailCode(snapshot.DetailCode)
        )
        {
            _lastReportedStartupFailureCode = null;
            return;
        }

        if (_lastReportedStartupFailureCode == snapshot.DetailCode)
        {
            return;
        }

        _protocol.AppendEvent(
            new HelperErrorMessage(
                CanonHelperSchemas.HelperError,
                "helper-error",
                _paths.SessionId,
                UtcNow(),
                snapshot.DetailCode ?? "camera-connect-timeout",
                BuildStartupFailureMessage(snapshot.DetailCode)
            )
        );
        _lastReportedStartupFailureCode = snapshot.DetailCode;
    }

    private static string BuildStartupFailureMessage(string? detailCode)
    {
        return detailCode switch
        {
            "sdk-init-timeout" => "Canon SDK 초기화가 startup timeout 안에 닫히지 않았어요.",
            "session-open-timeout" => "카메라 세션 열기가 startup timeout 안에 닫히지 않았어요.",
            "camera-open-failed" => "카메라를 열지 못했어요.",
            "session-open-failed" => "카메라 세션을 열지 못했어요.",
            "sdk-init-failed" => "Canon SDK를 초기화하지 못했어요.",
            _ => "카메라 연결 시작이 timeout 안에 완료되지 않았어요.",
        };
    }

    private void MaybeAppendStartupDebugLine(CameraSnapshot snapshot)
    {
        if (_activeCaptureTask is not null || _activeRequest is not null)
        {
            _lastStartupDebugSnapshotKey = null;
            return;
        }

        var snapshotKey = string.Join(
            "|",
            snapshot.CameraState,
            snapshot.HelperState,
            snapshot.DetailCode ?? string.Empty,
            snapshot.CameraModel ?? string.Empty
        );
        if (_lastStartupDebugSnapshotKey == snapshotKey)
        {
            return;
        }

        Directory.CreateDirectory(_paths.DiagnosticsDir);
        File.AppendAllText(
            _paths.StartupLogPath,
            $"{UtcNow()}\tsequence={_statusSequence}\tcameraState={snapshot.CameraState}\thelperState={snapshot.HelperState}\tdetailCode={snapshot.DetailCode ?? ""}\tcameraModel={snapshot.CameraModel ?? ""}{Environment.NewLine}",
            Encoding.UTF8
        );
        _lastStartupDebugSnapshotKey = snapshotKey;
    }
}
