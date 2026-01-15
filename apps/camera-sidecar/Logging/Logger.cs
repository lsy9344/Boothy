using System;

namespace Boothy.CameraSidecar.Logging
{
    public enum LogLevel
    {
        Trace,
        Debug,
        Info,
        Warning,
        Error,
        Fatal
    }

    /// <summary>
    /// Structured logger with correlation ID support for end-to-end tracing
    /// Follows the pattern: capture → transfer → ingest correlation
    /// </summary>
    public static class Logger
    {
        private static readonly object lockObj = new object();
        private static LogLevel minLevel = LogLevel.Info;

        public static void SetMinLevel(LogLevel level)
        {
            minLevel = level;
        }

        public static void Log(LogLevel level, string correlationId, string message, Exception? ex = null)
        {
            if (level < minLevel) return;

            lock (lockObj)
            {
                string timestamp = DateTime.UtcNow.ToString("yyyy-MM-ddTHH:mm:ss.fffZ");
                string levelStr = level.ToString().ToUpper();
                string exceptionInfo = ex != null ? $" | Exception: {ex.Message}\n{ex.StackTrace}" : "";

                Console.WriteLine($"[{timestamp}] [{levelStr}] [{correlationId}] {message}{exceptionInfo}");
            }
        }

        public static void Trace(string correlationId, string message) => Log(LogLevel.Trace, correlationId, message);
        public static void Debug(string correlationId, string message) => Log(LogLevel.Debug, correlationId, message);
        public static void Info(string correlationId, string message) => Log(LogLevel.Info, correlationId, message);
        public static void Warning(string correlationId, string message) => Log(LogLevel.Warning, correlationId, message);
        public static void Error(string correlationId, string message, Exception? ex = null) => Log(LogLevel.Error, correlationId, message, ex);
        public static void Fatal(string correlationId, string message, Exception? ex = null) => Log(LogLevel.Fatal, correlationId, message, ex);
    }
}
