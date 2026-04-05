@echo off
setlocal EnableExtensions
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0local-renderer-sidecar.ps1" %*
exit /b %ERRORLEVEL%
