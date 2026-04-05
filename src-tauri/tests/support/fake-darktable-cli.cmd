@echo off
setlocal EnableExtensions EnableDelayedExpansion

set "raw=%~1"
set "xmp=%~2"
set "output=%~3"
set "actualOutput=%output%"

if /I "%~1"=="--version" (
  echo darktable-cli 5.4.1
  exit /b 0
)

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

powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "$bytes=[Convert]::FromBase64String('/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8UHRofHh0aHBwgJC4nICIsIxwcKDcpLDAxNDQ0Hyc5PTgyPC4zNDL/2wBDAQkJCQwLDBgNDRgyIRwhMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjL/wAARCAABAAEDASIAAhEBAxEB/8QAFQABAQAAAAAAAAAAAAAAAAAAAAb/xAAgEAACAQQCAwAAAAAAAAAAAAABAgMABAURITESQVFh/8QAFQEBAQAAAAAAAAAAAAAAAAAAAAT/xAAZEQADAQEBAAAAAAAAAAAAAAAAARECEiH/2gAMAwEAAhEDEQA/AJ9b0qS2K4wqY5lW9L0L4b2E6b9K1+JrZk3QmY2Dg5Nf/2Q==');[IO.File]::WriteAllBytes('!actualOutput!',$bytes)"

exit /b 0
