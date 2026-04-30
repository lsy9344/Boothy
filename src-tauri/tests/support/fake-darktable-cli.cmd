@echo off
setlocal EnableExtensions EnableDelayedExpansion

set "raw=%~1"
set "xmp="
set "output=%~2"
set "third=%~3"

if not "%third%"=="" if /I not "%third:~0,2%"=="--" (
  set "xmp=%~2"
  set "output=%~3"
)

set "actualOutput=%output%"

if "%output%"=="" exit /b 2

if exist "%output%" (
  for %%I in ("%output%") do (
    set "actualOutput=%%~dpnI_01%%~xI"
  )
)

for %%I in ("%output%") do (
  if not exist "%%~dpI" mkdir "%%~dpI" >nul 2>&1
)

echo(%raw% | findstr /c:"force-process-fail" >nul 2>&1
if not errorlevel 1 exit /b 17

findstr /c:"force-process-fail" "%raw%" >nul 2>&1
if not errorlevel 1 exit /b 17

echo(%raw% | findstr /c:"force-missing-output" >nul 2>&1
if not errorlevel 1 exit /b 0

findstr /c:"force-missing-output" "%raw%" >nul 2>&1
if not errorlevel 1 exit /b 0

echo(%raw% | findstr /c:"force-invalid-output" >nul 2>&1
if not errorlevel 1 (
  >"!actualOutput!" echo not-a-jpeg
  exit /b 0
)

findstr /c:"force-invalid-output" "%raw%" >nul 2>&1
if not errorlevel 1 (
  >"!actualOutput!" echo not-a-jpeg
  exit /b 0
)

set "variant=baseline"
if not "%xmp%"=="" if exist "%xmp%" (
  findstr /c:"force-no-visual-delta" "%xmp%" >nul 2>&1
  if errorlevel 1 set "variant=xmp-delta"
)

set "sleepFile=%CD%\.boothy-darktable\fake-darktable-sleep-ms.txt"
if exist "%sleepFile%" (
  set /p fakeSleepMs=<"%sleepFile%"
  powershell -NoProfile -ExecutionPolicy Bypass -Command "Start-Sleep -Milliseconds ([int]'!fakeSleepMs!')"
)

powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "$bytes = if ('!variant!' -eq 'xmp-delta') { [Convert]::FromBase64String('/9j/4AAQSkZJRgABAQEAYABgAAD/2wBDAAMCAgMCAgMDAwMEAwMEBQgFBQQEBQoHBwYIDAoMDAsKCwsNDhIQDQ4RDgsLEBYQERMUFRUVDA8XGBYUGBIUFRT/2wBDAQMEBAUEBQkFBQkUDQsNFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBT/wAARCAABAAEDASIAAhEBAxEB/8QAHwAAAQUBAQEBAQEAAAAAAAAAAAECAwQFBgcICQoL/8QAtRAAAgEDAwIEAwUFBAQAAAF9AQIDAAQRBRIhMUEGE1FhByJxFDKBkaEII0KxwRVS0fAkM2JyggkKFhcYGRolJicoKSo0NTY3ODk6Q0RFRkdISUpTVFVWV1hZWmNkZWZnaGlqc3R1dnd4eXqDhIWGh4iJipKTlJWWl5iZmqKjpKWmp6ipqrKztLW2t7i5usLDxMXGx8jJytLT1NXW19jZ2uHi4+Tl5ufo6erx8vP09fb3+Pn6/8QAHwEAAwEBAQEBAQEBAQAAAAAAAAECAwQFBgcICQoL/8QAtREAAgECBAQDBAcFBAQAAQJ3AAECAxEEBSExBhJBUQdhcRMiMoEIFEKRobHBCSMzUvAVYnLRChYkNOEl8RcYGRomJygpKjU2Nzg5OkNERUZHSElKU1RVVldYWVpjZGVmZ2hpanN0dXZ3eHl6goOEhYaHiImKkpOUlZaXmJmaoqOkpaanqKmqsrO0tba3uLm6wsPExcbHyMnK0tPU1dbX2Nna4uPk5ebn6Onq8vP09fb3+Pn6/9oADAMBAAIRAxEAPwD8qqKKKAP/2Q==') } else { [Convert]::FromBase64String('/9j/4AAQSkZJRgABAQEAYABgAAD/2wBDAAMCAgMCAgMDAwMEAwMEBQgFBQQEBQoHBwYIDAoMDAsKCwsNDhIQDQ4RDgsLEBYQERMUFRUVDA8XGBYUGBIUFRT/2wBDAQMEBAUEBQkFBQkUDQsNFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBT/wAARCAABAAEDASIAAhEBAxEB/8QAHwAAAQUBAQEBAQEAAAAAAAAAAAECAwQFBgcICQoL/8QAtRAAAgEDAwIEAwUFBAQAAAF9AQIDAAQRBRIhMUEGE1FhByJxFDKBkaEII0KxwRVS0fAkM2JyggkKFhcYGRolJicoKSo0NTY3ODk6Q0RFRkdISUpTVFVWV1hZWmNkZWZnaGlqc3R1dnd4eXqDhIWGh4iJipKTlJWWl5iZmqKjpKWmp6ipqrKztLW2t7i5usLDxMXGx8jJytLT1NXW19jZ2uHi4+Tl5ufo6erx8vP09fb3+Pn6/8QAHwEAAwEBAQEBAQEBAQAAAAAAAAECAwQFBgcICQoL/8QAtREAAgECBAQDBAcFBAQAAQJ3AAECAxEEBSExBhJBUQdhcRMiMoEIFEKRobHBCSMzUvAVYnLRChYkNOEl8RcYGRomJygpKjU2Nzg5OkNERUZHSElKU1RVVldYWVpjZGVmZ2hpanN0dXZ3eHl6goOEhYaHiImKkpOUlZaXmJmaoqOkpaanqKmqsrO0tba3uLm6wsPExcbHyMnK0tPU1dbX2Nna4uPk5ebn6Onq8vP09fb3+Pn6/9oADAMBAAIRAxEAPwD9U6KKKAP/2Q==') }; [IO.File]::WriteAllBytes('!actualOutput!', $bytes)"

exit /b 0
