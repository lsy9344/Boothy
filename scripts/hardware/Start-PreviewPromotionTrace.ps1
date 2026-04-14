[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$SessionId,
    [Parameter(Mandatory = $true)]
    [string]$PresetId,
    [Parameter(Mandatory = $true)]
    [string]$PublishedVersion,
    [string]$CaptureId = 'capture-pending',
    [string]$RepoRoot,
    [string]$TraceRoot,
    [switch]$DryRun,
    [switch]$Execute,
    [switch]$EmitJson
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Resolve-RepoRoot {
    param([string]$ConfiguredRoot)

    if ($ConfiguredRoot) {
        return (Resolve-Path -LiteralPath $ConfiguredRoot).Path
    }

    return (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
}

function Get-ToolPath {
    param([string]$CommandName)

    $command = Get-Command -Name $CommandName -ErrorAction SilentlyContinue
    if ($null -eq $command) {
        return $null
    }

    return $command.Source
}

$resolvedRepoRoot = Resolve-RepoRoot -ConfiguredRoot $RepoRoot
$generatedAt = (Get-Date).ToString('o')

if (-not $TraceRoot) {
    $TraceRoot = Join-Path $resolvedRepoRoot (
        'artifacts\hardware-validation\{0}\{1}\{2}\{3}\trace-{4}' -f
        $SessionId,
        $PresetId,
        $PublishedVersion,
        $CaptureId,
        'default'
    )
}

$wprTracePath = Join-Path $TraceRoot 'preview-promotion.etl'
$tracePlanPath = Join-Path $TraceRoot 'preview-promotion-trace-plan.json'
$wpaTableExportPath = Join-Path $TraceRoot 'preview-promotion-wpa-table.csv'
$pixTimingCapturePath = Join-Path $TraceRoot 'preview-promotion-timing.wpix'
$pixEventCsvPath = Join-Path $TraceRoot 'preview-promotion-pix-events.csv'
$pixScreenshotPath = Join-Path $TraceRoot 'preview-promotion-pix-screenshot.png'
$wprPath = Get-ToolPath -CommandName 'wpr.exe'
$wpaPath = Get-ToolPath -CommandName 'wpa.exe'
$pixPath = Get-ToolPath -CommandName 'pix.exe'
$pixtoolPath = Get-ToolPath -CommandName 'pixtool.exe'

$plan = [ordered]@{
    schemaVersion = 'preview-promotion-trace-plan/v1'
    mode = if ($Execute -and -not $DryRun) { 'executed' } elseif ($DryRun) { 'dry-run' } else { 'planned' }
    generatedAt = $generatedAt
    sessionId = $SessionId
    presetId = $PresetId
    publishedVersion = $PublishedVersion
    captureId = $CaptureId
    traceRoot = $TraceRoot
    tools = [ordered]@{
        wpr = $wprPath
        wpa = $wpaPath
        pix = $pixPath
        pixtool = $pixtoolPath
    }
    traces = [ordered]@{
        tracePlanPath = $tracePlanPath
        wprTracePath = $wprTracePath
        wpaTableExportPath = $wpaTableExportPath
        pixTimingCapturePath = $pixTimingCapturePath
        pixEventCsvPath = $pixEventCsvPath
        pixScreenshotPath = $pixScreenshotPath
    }
    commands = [ordered]@{
        startWpr = if ($wprPath) { ('"{0}" -start GeneralProfile -filemode' -f $wprPath) } else { $null }
        stopWpr = if ($wprPath) { ('"{0}" -stop "{1}"' -f $wprPath, $wprTracePath) } else { $null }
        wpaExportHint = if ($wpaPath) { ('Open "{0}" in WPA and export the target table view to "{1}".' -f $wprTracePath, $wpaTableExportPath) } else { $null }
        pixCaptureHint = if ($pixPath) { ('Start a PIX timing capture for Boothy and save it to "{0}".' -f $pixTimingCapturePath) } else { $null }
        pixExportHint = if ($pixtoolPath) { ('Use "{0}" to export CSV/PNG artifacts into "{1}".' -f $pixtoolPath, $TraceRoot) } else { $null }
    }
}

if ($Execute -and -not $DryRun) {
    New-Item -ItemType Directory -Force -Path $TraceRoot | Out-Null
    if ($wprPath) {
        & $wprPath -start GeneralProfile -filemode | Out-Null
    }
    $plan | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $tracePlanPath -Encoding utf8
}

if ($EmitJson) {
    $plan | ConvertTo-Json -Depth 8
    return
}

Write-Host ('Preview promotion trace plan ready: {0}' -f $TraceRoot)
