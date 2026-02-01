using System;
using System.IO;
using System.Text;

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

        private static void RepairLogFileIfNeeded(string logFile)
        {
            try
            {
                var info = new FileInfo(logFile);
                if (!info.Exists || info.Length == 0)
                {
                    return;
                }

                using (var readStream = new FileStream(logFile, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
                {
                    int headerLen = (int)Math.Min(4096, readStream.Length);
                    var header = new byte[headerLen];
                    int read = readStream.Read(header, 0, headerLen);
                    if (read <= 0 || header[0] != 0)
                    {
                        return;
                    }
                }

                long firstNonZeroOffset = -1;
                using (var scanStream = new FileStream(logFile, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
                {
                    var buffer = new byte[64 * 1024];
                    long offset = 0;
                    while (true)
                    {
                        int read = scanStream.Read(buffer, 0, buffer.Length);
                        if (read <= 0)
                        {
                            break;
                        }

                        for (int i = 0; i < read; i++)
                        {
                            if (buffer[i] != 0)
                            {
                                firstNonZeroOffset = offset + i;
                                break;
                            }
                        }

                        if (firstNonZeroOffset >= 0)
                        {
                            break;
                        }

                        offset += read;
                    }
                }

                string tempPath = logFile + ".repair.tmp";

                if (firstNonZeroOffset < 0)
                {
                    using (var truncateStream = new FileStream(logFile, FileMode.Truncate, FileAccess.Write, FileShare.ReadWrite))
                    {
                    }
                    return;
                }

                using (var input = new FileStream(logFile, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
                using (var output = new FileStream(tempPath, FileMode.Create, FileAccess.Write, FileShare.ReadWrite))
                {
                    input.Position = firstNonZeroOffset;
                    input.CopyTo(output);
                    output.Flush(true);
                }

                try
                {
                    File.Move(tempPath, logFile, true);
                }
                catch
                {
                    try
                    {
                        File.Replace(tempPath, logFile, null, true);
                    }
                    catch
                    {
                    }
                }
            }
            catch
            {
            }
        }

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

                RepairLogFileIfNeeded(logFile);

                fileWriter = new StreamWriter(
                    new FileStream(logFile, FileMode.Append, FileAccess.Write, FileShare.ReadWrite),
                    new UTF8Encoding(false)
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
