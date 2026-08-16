[CmdletBinding()]
param(
  [switch]$PreflightOnly,
  [switch]$OperatorReady
)

$ErrorActionPreference = 'Stop'

$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\..\..'))
$helperProject = Join-Path $repoRoot 'sidecar\canon-helper\src\CanonHelper\CanonHelper.csproj'
$vendoredSdkRoot = Join-Path $repoRoot 'sidecar\canon-helper\vendor\canon-edsdk'
$sdkRoot = if ([string]::IsNullOrWhiteSpace($env:BOOTHY_CANON_SDK_ROOT)) {
  $vendoredSdkRoot
}
else {
  [IO.Path]::GetFullPath($env:BOOTHY_CANON_SDK_ROOT)
}

if (-not (Test-Path -LiteralPath $sdkRoot -PathType Container)) {
  throw "Canon SDK root is invalid. Set BOOTHY_CANON_SDK_ROOT or restore the vendored SDK at $vendoredSdkRoot"
}

$runtimeDll = Join-Path $sdkRoot 'Windows\EDSDK_64\Dll\EDSDK.dll'
if (-not (Test-Path -LiteralPath $runtimeDll -PathType Leaf)) {
  throw "EDSDK.dll was not found at $runtimeDll"
}

$selfCheckOutput = @(
  & dotnet run --project $helperProject -- --self-check --sdk-root $sdkRoot
)
if ($LASTEXITCODE -ne 0) {
  throw "Canon helper self-check failed with exit code $LASTEXITCODE."
}
$selfCheck = $selfCheckOutput[-1] | ConvertFrom-Json
if (-not $selfCheck.sdkInitialized -or $selfCheck.cameraCount -ne 1 -or $selfCheck.detailCode -ne 'camera-ready') {
  throw "Canon helper preflight is not ready: $($selfCheckOutput[-1])"
}

if ($PreflightOnly.IsPresent) {
  Write-Host 'HV-14 launch preflight passed: SDK initialized and exactly one camera is ready.' -ForegroundColor Green
  return
}

if (-not $OperatorReady.IsPresent) {
  throw 'Operator confirmation is required. Complete operator-checklist.md, then rerun with -OperatorReady.'
}

$environmentPath = Join-Path $PSScriptRoot 'environment.md'
if (-not (Test-Path -LiteralPath $environmentPath -PathType Leaf)) {
  throw 'environment.md is required. Copy environment.template.md and fill every operator-confirmed value first.'
}
$environmentRecord = Get-Content -LiteralPath $environmentPath -Raw -Encoding utf8
if ($environmentRecord -match '<[^>]+>') {
  throw 'environment.md still contains an unconfirmed <...> placeholder.'
}

$runtimeSessions = Join-Path $env:USERPROFILE 'Pictures\dabi_shoot\sessions'
$beforeSessions = @()
if (Test-Path -LiteralPath $runtimeSessions -PathType Container) {
  $beforeSessions = @(Get-ChildItem -LiteralPath $runtimeSessions -Directory | Select-Object -ExpandProperty Name)
}

$hadSourceMode = Test-Path Env:\BOOTHY_SOURCE_COMPARE_MODE
$previousSourceMode = $env:BOOTHY_SOURCE_COMPARE_MODE
$hadDisplayMode = Test-Path Env:\BOOTHY_DISPLAY_SAMPLE_MODE
$previousDisplayMode = $env:BOOTHY_DISPLAY_SAMPLE_MODE

try {
  $env:BOOTHY_SOURCE_COMPARE_MODE = 'ab'
  Remove-Item Env:\BOOTHY_DISPLAY_SAMPLE_MODE -ErrorAction SilentlyContinue

  Write-Host 'HV-14 measurement mode is ready: source=ab, display=off.' -ForegroundColor Green
  Write-Host 'Create a NEW session and capture exactly 35 shutters (5 warm-up + 30 measured).'
  Write-Host "Follow the scene order in $(Join-Path $PSScriptRoot 'scene-plan.md')."

  Push-Location $repoRoot
  try {
    & pnpm dev:desktop
    if ($LASTEXITCODE -ne 0) {
      throw "pnpm dev:desktop exited with code $LASTEXITCODE."
    }
  }
  finally {
    Pop-Location
  }
}
finally {
  if ($hadSourceMode) {
    $env:BOOTHY_SOURCE_COMPARE_MODE = $previousSourceMode
  }
  else {
    Remove-Item Env:\BOOTHY_SOURCE_COMPARE_MODE -ErrorAction SilentlyContinue
  }

  if ($hadDisplayMode) {
    $env:BOOTHY_DISPLAY_SAMPLE_MODE = $previousDisplayMode
  }
  else {
    Remove-Item Env:\BOOTHY_DISPLAY_SAMPLE_MODE -ErrorAction SilentlyContinue
  }
}

if (Test-Path -LiteralPath $runtimeSessions -PathType Container) {
  $afterSessions = @(Get-ChildItem -LiteralPath $runtimeSessions -Directory | Select-Object -ExpandProperty Name)
  $newSessions = @($afterSessions | Where-Object { $_ -notin $beforeSessions })
  if ($newSessions.Count -eq 1) {
    Write-Host "New session: $($newSessions[0])" -ForegroundColor Green
  }
  elseif ($newSessions.Count -eq 0) {
    Write-Warning 'No new session directory was found.'
  }
  else {
    Write-Warning "Multiple new sessions were found: $($newSessions -join ', ')"
  }
}

Write-Host 'Measurement environment variables were restored.'
