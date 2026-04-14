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
$traceSummaryPath = Join-Path $TraceRoot 'preview-promotion-trace-summary.json'
$wpaTableExportPath = Join-Path $TraceRoot 'preview-promotion-wpa-table.csv'
$pixTimingCapturePath = Join-Path $TraceRoot 'preview-promotion-timing.wpix'
$pixEventCsvPath = Join-Path $TraceRoot 'preview-promotion-pix-events.csv'
$pixScreenshotPath = Join-Path $TraceRoot 'preview-promotion-pix-screenshot.png'
$wprPath = Get-ToolPath -CommandName 'wpr.exe'
$wpaPath = Get-ToolPath -CommandName 'wpa.exe'
$pixPath = Get-ToolPath -CommandName 'pix.exe'
$pixtoolPath = Get-ToolPath -CommandName 'pixtool.exe'

$summary = [ordered]@{
    schemaVersion = 'preview-promotion-trace-summary/v1'
    mode = if ($Execute -and -not $DryRun) { 'executed' } elseif ($DryRun) { 'dry-run' } else { 'planned' }
    generatedAt = $generatedAt
    sessionId = $SessionId
    presetId = $PresetId
    publishedVersion = $PublishedVersion
    captureId = $CaptureId
    traceRoot = $TraceRoot
    traces = [ordered]@{
        summaryPath = $traceSummaryPath
        wprTracePath = $wprTracePath
        wpaTableExportPath = $wpaTableExportPath
        pixTimingCapturePath = $pixTimingCapturePath
        pixEventCsvPath = $pixEventCsvPath
        pixScreenshotPath = $pixScreenshotPath
    }
    commands = [ordered]@{
        stopWpr = if ($wprPath) { ('"{0}" -stop "{1}"' -f $wprPath, $wprTracePath) } else { $null }
        wpaExportHint = if ($wpaPath) { ('Open "{0}" in WPA and export the approved analysis table to "{1}".' -f $wprTracePath, $wpaTableExportPath) } else { $null }
        pixReviewHint = if ($pixPath) { ('Review the saved PIX timing capture at "{0}" and keep the correlated export beside it.' -f $pixTimingCapturePath) } else { $null }
        pixtoolExportHint = if ($pixtoolPath) { ('Use "{0}" to export PIX CSV/PNG evidence into "{1}".' -f $pixtoolPath, $TraceRoot) } else { $null }
    }
}

if ($Execute -and -not $DryRun) {
    New-Item -ItemType Directory -Force -Path $TraceRoot | Out-Null
    if ($wprPath) {
        & $wprPath -stop $wprTracePath | Out-Null
    }
    $summary | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $traceSummaryPath -Encoding utf8
}

if ($EmitJson) {
    $summary | ConvertTo-Json -Depth 8
    return
}

Write-Host ('Preview promotion trace stop plan ready: {0}' -f $TraceRoot)
