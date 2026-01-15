using System;
using System.IO;

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
        private static StreamWriter? fileWriter;
        private static bool initialized;

        public static void SetMinLevel(LogLevel level)
        {
            minLevel = level;
        }

        public static void Initialize()
        {
            if (initialized)
            {
                return;
            }

            initialized = true;

            try
            {
                string appData = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
                if (string.IsNullOrWhiteSpace(appData))
                {
                    return;
                }

                string logDir = Path.Combine(appData, "Boothy", "logs");
                Directory.CreateDirectory(logDir);

                string logFile = Path.Combine(
                    logDir,
                    $"boothy-sidecar-{DateTime.UtcNow:yyyyMMdd}.log"
                );

                fileWriter = new StreamWriter(
                    new FileStream(logFile, FileMode.Append, FileAccess.Write, FileShare.ReadWrite)
                )
                {
                    AutoFlush = true
                };

                Log(LogLevel.Info, "system", $"Sidecar log file: {logFile}");
            }
            catch (Exception ex)
            {
                Console.WriteLine(
                    $"[{DateTime.UtcNow:yyyy-MM-ddTHH:mm:ss.fffZ}] [WARN] [system] " +
                    $"Failed to initialize file logging: {ex.Message}"
                );
            }
        }

        public static void Log(LogLevel level, string correlationId, string message, Exception? ex = null)
        {
            if (level < minLevel) return;

            lock (lockObj)
            {
                string timestamp = DateTime.UtcNow.ToString("yyyy-MM-ddTHH:mm:ss.fffZ");
                string levelStr = level.ToString().ToUpper();
                string exceptionInfo = ex != null ? $" | Exception: {ex.Message}\n{ex.StackTrace}" : "";
                string logLine = $"[{timestamp}] [{levelStr}] [{correlationId}] {message}{exceptionInfo}";

                Console.WriteLine(logLine);
                if (fileWriter != null)
                {
                    try
                    {
                        fileWriter.WriteLine(logLine);
                    }
                    catch
                    {
                        // Ignore logging failures to avoid crashing the sidecar.
                    }
                }
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
