[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [ValidatePattern('^session_[a-z0-9]+$')]
  [string]$SessionId
)

$ErrorActionPreference = 'Stop'

$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\..\..'))
$runtimeRoot = Join-Path $env:USERPROFILE 'Pictures\dabi_shoot'
$sessionsRoot = Join-Path $runtimeRoot 'sessions'
$sourceSession = [IO.Path]::GetFullPath((Join-Path $sessionsRoot $SessionId))
$resolvedSessionsRoot = [IO.Path]::GetFullPath($sessionsRoot)
$targetSession = Join-Path $PSScriptRoot 'session-evidence'
$stagingSession = Join-Path $PSScriptRoot ('.session-evidence.staging-{0}' -f [guid]::NewGuid().ToString('N'))
$environmentPath = Join-Path $PSScriptRoot 'environment.md'

if (-not (Test-Path -LiteralPath $environmentPath -PathType Leaf)) {
  throw 'environment.md is required before evidence collection.'
}
$environmentRecord = Get-Content -LiteralPath $environmentPath -Raw -Encoding utf8
if ($environmentRecord -match '<[^>]+>' -or $environmentRecord -match 'post-capture-pending') {
  throw 'environment.md still has an unconfirmed value, including the post-capture Image Quality check.'
}

if (-not $sourceSession.StartsWith($resolvedSessionsRoot, [StringComparison]::OrdinalIgnoreCase)) {
  throw 'Resolved session path escaped the runtime sessions directory.'
}
if (-not (Test-Path -LiteralPath $sourceSession -PathType Container)) {
  throw "Session directory was not found: $sourceSession"
}
if (Test-Path -LiteralPath $targetSession) {
  throw "Evidence target already exists and will not be overwritten: $targetSession"
}

$sourceComparison = Join-Path $sourceSession 'diagnostics\source-comparison.jsonl'
if (-not (Test-Path -LiteralPath $sourceComparison -PathType Leaf)) {
  throw "Source comparison telemetry was not found: $sourceComparison"
}

$sourceGate = Join-Path $repoRoot 'tests\hardware\capture-source\hv-14\check-source-completeness.ps1'
$displayGate = Join-Path $repoRoot 'tests\hardware\viewer-present\hv-13b\check-telemetry-completeness.ps1'
$completenessDir = Join-Path $PSScriptRoot 'completeness'
$sourceReport = Join-Path $completenessDir 'source-completeness.md'
$displayReport = Join-Path $completenessDir 'display-completeness.md'
$pendingSourceReport = "$sourceReport.pending"
$pendingDisplayReport = "$displayReport.pending"
$powerShellExe = (Get-Process -Id $PID).Path

function Invoke-EvidenceGate {
  param(
    [string]$ScriptPath,
    [string[]]$Arguments,
    [string]$Name
  )

  & $powerShellExe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "$Name failed with exit code $LASTEXITCODE."
  }
}

try {
  Copy-Item -LiteralPath $sourceSession -Destination $stagingSession -Recurse
  New-Item -ItemType Directory -Path $completenessDir -Force | Out-Null

  Invoke-EvidenceGate -ScriptPath $sourceGate -Name 'Source completeness gate' -Arguments @(
    '-SessionEvidenceDir', $stagingSession,
    '-ExpectedPerRoute', '35',
    '-OutFile', $pendingSourceReport
  )
  Invoke-EvidenceGate -ScriptPath $displayGate -Name 'Display completeness gate' -Arguments @(
    '-SessionEvidenceDir', $stagingSession,
    '-OutFile', $pendingDisplayReport
  )

  Move-Item -LiteralPath $stagingSession -Destination $targetSession
  Move-Item -LiteralPath $pendingSourceReport -Destination $sourceReport -Force
  Move-Item -LiteralPath $pendingDisplayReport -Destination $displayReport -Force
}
finally {
  foreach ($path in @($stagingSession, $pendingSourceReport, $pendingDisplayReport)) {
    if (Test-Path -LiteralPath $path) {
      Remove-Item -LiteralPath $path -Recurse -Force
    }
  }
}

Write-Host "Evidence copied and completeness gates passed for $SessionId." -ForegroundColor Green
Write-Host 'Telemetry PASS is not HV-14 Go. Complete capability, correlation, quality, aggregate, and decision evidence next.'
