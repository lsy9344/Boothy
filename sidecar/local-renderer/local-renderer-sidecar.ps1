param(
  [Parameter(Mandatory = $true)]
  [string]$requestPath,
  [Parameter(Mandatory = $true)]
  [string]$responsePath
)

$ErrorActionPreference = "Stop"

function Get-UnixTimeMilliseconds {
  return [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
}

function Resolve-ExecutablePath {
  param(
    [Parameter(Mandatory = $true)]
    [string]$commandName
  )

  if ([System.IO.Path]::IsPathRooted($commandName) -and (Test-Path -LiteralPath $commandName)) {
    return (Resolve-Path -LiteralPath $commandName).Path
  }

  $command = Get-Command $commandName -ErrorAction SilentlyContinue
  if ($command -and $command.Source -and $command.Source.Trim().Length -gt 0) {
    return $command.Source
  }

  return $null
}

function Resolve-DarktableBinary {
  if ($env:BOOTHY_DARKTABLE_CLI_BIN -and $env:BOOTHY_DARKTABLE_CLI_BIN.Trim().Length -gt 0) {
    return $env:BOOTHY_DARKTABLE_CLI_BIN.Trim()
  }

  return "darktable-cli"
}

function Read-DarktableVersionCache {
  param(
    [Parameter(Mandatory = $true)]
    [string]$cachePath,
    [Parameter(Mandatory = $true)]
    [string]$resolvedBinaryPath
  )

  if (-not (Test-Path -LiteralPath $cachePath)) {
    return $null
  }

  try {
    $cache = Get-Content -Path $cachePath -Raw | ConvertFrom-Json
  }
  catch {
    return $null
  }

  if (-not $cache) {
    return $null
  }

  $binaryInfo = Get-Item -LiteralPath $resolvedBinaryPath -ErrorAction SilentlyContinue
  if (-not $binaryInfo) {
    return $null
  }

  if ($cache.binaryPath -ne $resolvedBinaryPath) {
    return $null
  }

  if ($cache.lastWriteTimeUtc -ne $binaryInfo.LastWriteTimeUtc.Ticks) {
    return $null
  }

  if ($cache.length -ne $binaryInfo.Length) {
    return $null
  }

  if (-not $cache.version -or $cache.version.Trim().Length -eq 0) {
    return $null
  }

  return [string]$cache.version
}

function Write-DarktableVersionCache {
  param(
    [Parameter(Mandatory = $true)]
    [string]$cachePath,
    [Parameter(Mandatory = $true)]
    [string]$resolvedBinaryPath,
    [Parameter(Mandatory = $true)]
    [string]$version
  )

  $binaryInfo = Get-Item -LiteralPath $resolvedBinaryPath -ErrorAction SilentlyContinue
  if (-not $binaryInfo) {
    return
  }

  $cachePayload = @{
    binaryPath = $resolvedBinaryPath
    lastWriteTimeUtc = $binaryInfo.LastWriteTimeUtc.Ticks
    length = $binaryInfo.Length
    version = $version
  }

  [System.IO.File]::WriteAllText(
    $cachePath,
    ($cachePayload | ConvertTo-Json -Depth 5)
  )
}

function Resolve-DarktableVersion {
  param(
    [Parameter(Mandatory = $true)]
    [string]$binaryPath,
    [Parameter(Mandatory = $true)]
    [string]$cachePath
  )

  $resolvedBinaryPath = Resolve-ExecutablePath -commandName $binaryPath
  if ($resolvedBinaryPath) {
    $cachedVersion = Read-DarktableVersionCache -cachePath $cachePath -resolvedBinaryPath $resolvedBinaryPath
    if ($cachedVersion) {
      return $cachedVersion
    }
  }

  $versionOutput = & $binaryPath --version 2>$null
  if ($LASTEXITCODE -ne $null -and [int]$LASTEXITCODE -ne 0) {
    throw "darktable version probe exit code: $LASTEXITCODE"
  }

  $versionText = ($versionOutput | Out-String).Trim()
  if ($versionText -match '(\d+\.\d+\.\d+)') {
    $resolvedVersion = $Matches[1]
    if ($resolvedBinaryPath) {
      Write-DarktableVersionCache -cachePath $cachePath -resolvedBinaryPath $resolvedBinaryPath -version $resolvedVersion
    }
    return $resolvedVersion
  }

  throw "darktable version probe did not return a semantic version"
}

function ConvertTo-DarktableVersionParts {
  param(
    [Parameter(Mandatory = $true)]
    [string]$version
  )

  if ($version -notmatch '^(\d+)\.(\d+)\.(\d+)$') {
    return $null
  }

  return @{
    major = [int]$Matches[1]
    minor = [int]$Matches[2]
    patch = [int]$Matches[3]
  }
}

function Test-DarktableVersionCompatibility {
  param(
    [Parameter(Mandatory = $true)]
    [string]$requestedVersion,
    [Parameter(Mandatory = $true)]
    [string]$resolvedVersion
  )

  if ($requestedVersion -eq $resolvedVersion) {
    return $true
  }

  $requestedParts = ConvertTo-DarktableVersionParts -version $requestedVersion
  $resolvedParts = ConvertTo-DarktableVersionParts -version $resolvedVersion
  if (-not $requestedParts -or -not $resolvedParts) {
    return $false
  }

  return (
    $requestedParts.major -eq $resolvedParts.major -and
    $requestedParts.minor -eq $resolvedParts.minor
  )
}

function Write-ErrorResponse {
  param(
    [string]$schemaVersion,
    [string]$message
  )

  $payload = @{
    schemaVersion = $schemaVersion
    error = @{
      message = $message
    }
  }

  [System.IO.File]::WriteAllText(
    $responsePath,
    ($payload | ConvertTo-Json -Depth 5)
  )
}

$request = Get-Content -Path $requestPath -Raw | ConvertFrom-Json
$candidatePath = [string]$request.candidateOutputPath
$candidateDir = Split-Path -Path $candidatePath -Parent
$previewDir = Split-Path -Path $candidateDir -Parent
$sessionRoot = Split-Path -Path $previewDir -Parent
$sessionsRoot = Split-Path -Path $sessionRoot -Parent
$runtimeRoot = Split-Path -Path $sessionsRoot -Parent
if (-not $runtimeRoot -or $runtimeRoot.Trim().Length -eq 0) {
  $runtimeRoot = $sessionRoot
}
$workerRoot = Join-Path $runtimeRoot ".boothy-local-renderer\preview"
$configDir = Join-Path $workerRoot "config"
$libraryPath = Join-Path $workerRoot "library.db"
$cacheDir = Join-Path $workerRoot "cache"
$versionCachePath = Join-Path $workerRoot "darktable-version-cache.json"

[System.IO.Directory]::CreateDirectory($candidateDir) | Out-Null
[System.IO.Directory]::CreateDirectory($configDir) | Out-Null
[System.IO.Directory]::CreateDirectory($cacheDir) | Out-Null

$darktableBinary = Resolve-DarktableBinary
$startedAt = Get-UnixTimeMilliseconds
$arguments = @(
  [string]$request.sourceAssetPath,
  [string]$request.xmpTemplatePath,
  $candidatePath,
  "--hq",
  "false",
  "--apply-custom-presets",
  "false",
  "--disable-opencl",
  "--width",
  [string]$request.previewWidthCap,
  "--height",
  [string]$request.previewHeightCap,
  "--core",
  "--configdir",
  $configDir,
  "--cachedir",
  $cacheDir,
  "--library",
  $libraryPath
)

try {
  $resolvedDarktableVersion = Resolve-DarktableVersion -binaryPath $darktableBinary -cachePath $versionCachePath
  $requestedDarktableVersion = [string]$request.darktableVersion
  if ($requestedDarktableVersion.Trim().Length -eq 0) {
    throw "darktable version pin is missing from the request"
  }

  if (-not (Test-DarktableVersionCompatibility -requestedVersion $requestedDarktableVersion -resolvedVersion $resolvedDarktableVersion)) {
    throw "darktable version mismatch: requested=$requestedDarktableVersion actual=$resolvedDarktableVersion"
  }

  & $darktableBinary @arguments
  $exitCode = if ($LASTEXITCODE -ne $null) { [int]$LASTEXITCODE } else { 0 }
  if ($exitCode -ne 0) {
    throw "darktable bridge exit code: $exitCode"
  }
  if (-not (Test-Path -LiteralPath $candidatePath)) {
    throw "candidate output missing after darktable bridge"
  }

  $elapsedMs = [Math]::Max(0, (Get-UnixTimeMilliseconds) - $startedAt)
  $payload = @{
    schemaVersion = "local-renderer-response/v1"
    route = "local-renderer-sidecar"
    sessionId = [string]$request.sessionId
    captureId = [string]$request.captureId
    requestId = [string]$request.requestId
    presetId = [string]$request.presetId
    presetVersion = [string]$request.presetVersion
    candidatePath = $candidatePath
    candidateWrittenAtMs = Get-UnixTimeMilliseconds
    elapsedMs = $elapsedMs
    fidelity = @{
      verdict = "baseline-bridge"
      detail = "engine=darktable-cli-bridge;comparison=not-run"
    }
    attempt = @{
      retryOrdinal = 0
      completionOrdinal = 1
    }
  }

  [System.IO.File]::WriteAllText(
    $responsePath,
    ($payload | ConvertTo-Json -Depth 5)
  )
}
catch {
  Write-ErrorResponse -schemaVersion "local-renderer-response/v1" -message $_.Exception.Message
  throw
}
