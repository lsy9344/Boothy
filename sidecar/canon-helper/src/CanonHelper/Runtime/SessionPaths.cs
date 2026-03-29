namespace CanonHelper.Runtime;

internal sealed class SessionPaths
{
    public SessionPaths(string runtimeRoot, string sessionId)
    {
        RuntimeRoot = Path.GetFullPath(runtimeRoot);
        SessionId = sessionId;
        SessionRoot = Path.Combine(RuntimeRoot, "sessions", sessionId);
        CapturesOriginalsDir = Path.Combine(SessionRoot, "captures", "originals");
        DiagnosticsDir = Path.Combine(SessionRoot, "diagnostics");
        RequestLogPath = Path.Combine(DiagnosticsDir, "camera-helper-requests.jsonl");
        ProcessedRequestsPath = Path.Combine(
            DiagnosticsDir,
            "camera-helper-processed-request-ids.txt"
        );
        EventsLogPath = Path.Combine(DiagnosticsDir, "camera-helper-events.jsonl");
        StatusPath = Path.Combine(DiagnosticsDir, "camera-helper-status.json");
    }

    public string RuntimeRoot { get; }
    public string SessionId { get; }
    public string SessionRoot { get; }
    public string CapturesOriginalsDir { get; }
    public string DiagnosticsDir { get; }
    public string RequestLogPath { get; }
    public string ProcessedRequestsPath { get; }
    public string EventsLogPath { get; }
    public string StatusPath { get; }

    public void EnsureExists()
    {
        Directory.CreateDirectory(CapturesOriginalsDir);
        Directory.CreateDirectory(DiagnosticsDir);
    }
}
