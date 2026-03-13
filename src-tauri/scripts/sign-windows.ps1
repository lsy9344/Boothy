param(
    [Parameter(Mandatory = $true)]
    [string]$ArtifactPath
)

$required = @('BOOTHY_WINDOWS_CERT_PASSWORD')

$missing = $required | Where-Object {
    [string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($_))
}

if ($missing.Count -gt 0) {
    Write-Error "Signing-ready blocker: missing signing environment variables: $($missing -join ', ')"
    exit 1
}

$certPath = $env:BOOTHY_WINDOWS_CERT_PATH
if ([string]::IsNullOrWhiteSpace($certPath) -and -not [string]::IsNullOrWhiteSpace($env:BOOTHY_WINDOWS_CERT_BASE64)) {
    $certPath = Join-Path ([IO.Path]::GetTempPath()) "boothy-signing-$PID.pfx"
    [IO.File]::WriteAllBytes($certPath, [Convert]::FromBase64String($env:BOOTHY_WINDOWS_CERT_BASE64))
}

if ([string]::IsNullOrWhiteSpace($certPath)) {
    Write-Error "Signing-ready blocker: provide BOOTHY_WINDOWS_CERT_PATH or BOOTHY_WINDOWS_CERT_BASE64."
    exit 1
}

if (-not (Test-Path -LiteralPath $certPath)) {
    Write-Error "Signing-ready blocker: certificate file not found at '$certPath'."
    exit 1
}

$signTool = if ([string]::IsNullOrWhiteSpace($env:BOOTHY_WINDOWS_SIGNTOOL_PATH)) {
    'signtool.exe'
}
else {
    $env:BOOTHY_WINDOWS_SIGNTOOL_PATH
}

if (-not (Get-Command $signTool -ErrorAction SilentlyContinue)) {
    Write-Error "Signing-ready blocker: signing tool '$signTool' was not found."
    exit 1
}

$timestampUrl = if ([string]::IsNullOrWhiteSpace($env:BOOTHY_WINDOWS_TIMESTAMP_URL)) {
    'http://timestamp.digicert.com'
}
else {
    $env:BOOTHY_WINDOWS_TIMESTAMP_URL
}

& $signTool sign `
    /fd SHA256 `
    /f $certPath `
    /p $env:BOOTHY_WINDOWS_CERT_PASSWORD `
    /tr $timestampUrl `
    /td SHA256 `
    $ArtifactPath

if ($LASTEXITCODE -ne 0) {
    Write-Error "Signing-ready blocker: signtool failed with exit code $LASTEXITCODE."
    exit $LASTEXITCODE
}
