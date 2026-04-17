[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$SessionId,
    [Parameter(Mandatory = $true)]
    [string]$CaptureId,
    [Parameter(Mandatory = $true)]
    [string]$PresetId,
    [Parameter(Mandatory = $true)]
    [string]$PublishedVersion,
    [string]$RepoRoot,
    [string]$OutputRoot,
    [string]$BaselineImagePath,
    [string]$BaselineMetadataPath,
    [string]$FallbackImagePath,
    [string]$FallbackMetadataPath,
    [string[]]$BoothVisualEvidencePaths = @(),
    [string[]]$OperatorVisualEvidencePaths = @(),
    [string[]]$RollbackEvidencePaths = @(),
    [double]$ParityThreshold = 6,
    [switch]$DryRun,
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

function Read-JsonFile {
    param([string]$Path)

    return Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Get-SelectedCaptureTimingEvents {
    param(
        [string]$Path,
        [string]$TargetSessionId,
        [string]$TargetCaptureId,
        [string]$TargetRequestId
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        throw ('Timing events not found for selected capture chain: {0}' -f $Path)
    }

    $lines = Get-Content -LiteralPath $Path | Where-Object { $_.Trim() }
    $selectedLines = @(
        $lines | Where-Object {
            $_ -like "*session=$TargetSessionId*" -and
            $_ -like "*request=$TargetRequestId*" -and
            (
                $_ -like '*event=request-capture*' -or
                $_ -like '*event=capture-accepted*' -or
                $_ -like "*capture=$TargetCaptureId*"
            )
        }
    )

    if ($selectedLines.Count -eq 0) {
        throw (
            'wrong-capture attribution risk: no timing events matched the selected capture correlation {0}/{1}/{2}' -f
            $TargetSessionId,
            $TargetCaptureId,
            $TargetRequestId
        )
    }

    foreach ($requiredEvent in @(
        'event=request-capture',
        'event=capture-accepted',
        'event=file-arrived',
        'event=capture_preview_ready',
        'event=recent-session-visible',
        'event=capture_preview_transition_summary'
    )) {
        if (-not @($selectedLines | Where-Object { $_ -like "*$requiredEvent*" }).Count) {
            throw (
                'selected capture chain is incomplete: missing {0} for {1}/{2}/{3}' -f
                $requiredEvent,
                $TargetSessionId,
                $TargetCaptureId,
                $TargetRequestId
            )
        }
    }

    $hasFastPreviewSeam = @(
        $selectedLines | Where-Object {
            $_ -like '*event=fast-preview-ready*' -or
            $_ -like '*event=current-session-preview-pending-visible*' -or
            $_ -like '*event=recent-session-pending-visible*' -or
            $_ -like '*event=fast-thumbnail-attempted*' -or
            $_ -like '*event=fast-thumbnail-failed*'
        }
    ).Count -gt 0
    if (-not $hasFastPreviewSeam) {
        throw (
            'selected capture chain is incomplete: missing fast-preview seam for {0}/{1}/{2}' -f
            $TargetSessionId,
            $TargetCaptureId,
            $TargetRequestId
        )
    }

    return $selectedLines
}

function Assert-CaptureTimeSnapshotPath {
    param(
        [string]$SnapshotPath,
        [string]$SessionRoot,
        [string]$Label,
        [string]$CaptureId
    )

    if (-not $SnapshotPath) {
        throw ('stale-preview attribution risk: capture-time {0} snapshot path is missing.' -f $Label)
    }

    if (-not (Test-Path -LiteralPath $SnapshotPath)) {
        throw ('stale-preview attribution risk: capture-time {0} snapshot not found: {1}' -f $Label, $SnapshotPath)
    }

    $resolvedSnapshotPath = (Resolve-Path -LiteralPath $SnapshotPath).Path
    $diagnosticsRoot = (Resolve-Path -LiteralPath (Join-Path $SessionRoot 'diagnostics')).Path
    if (-not $resolvedSnapshotPath.StartsWith($diagnosticsRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw (
            'stale-preview attribution risk: capture-time {0} snapshot must stay under session diagnostics, not {1}' -f
            $Label,
            $resolvedSnapshotPath
        )
    }

    if ($CaptureId) {
        $snapshotFileName = [System.IO.Path]::GetFileName($resolvedSnapshotPath)
        if ($snapshotFileName -notmatch [regex]::Escape($CaptureId)) {
            throw (
                'stale-preview attribution risk: capture-time {0} snapshot must match the selected capture {1}, not {2}' -f
                $Label,
                $CaptureId,
                $snapshotFileName
            )
        }
    }
}

function Get-OptionalRecordValue {
    param(
        $Record,
        [string]$PropertyName
    )

    if (
        $null -eq $Record -or
        -not $Record.PSObject.Properties[$PropertyName]
    ) {
        return $null
    }

    return $Record.$PropertyName
}

function Find-EvidenceRecord {
    param(
        [string]$Path,
        [string]$TargetCaptureId,
        [string]$TargetSessionId,
        [string]$TargetPresetId,
        [string]$TargetPublishedVersion
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        return $null
    }

    $records = Get-Content -LiteralPath $Path |
        Where-Object { $_.Trim() } |
        ForEach-Object { $_ | ConvertFrom-Json }

    return $records |
        Where-Object {
            $_.captureId -eq $TargetCaptureId -and
            $_.sessionId -eq $TargetSessionId -and
            $_.presetId -eq $TargetPresetId -and
            $_.publishedVersion -eq $TargetPublishedVersion
        } |
        Select-Object -Last 1
}

function Find-EvidenceFamily {
    param(
        [string]$Path,
        [string]$TargetSessionId,
        [string]$TargetPresetId,
        [string]$TargetPublishedVersion,
        [string]$TargetRouteStage
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        return @()
    }

    $records = Get-Content -LiteralPath $Path |
        Where-Object { $_.Trim() } |
        ForEach-Object { $_ | ConvertFrom-Json }

    return @(
        $records | Where-Object {
            $_.sessionId -eq $TargetSessionId -and
            $_.presetId -eq $TargetPresetId -and
            $_.publishedVersion -eq $TargetPublishedVersion -and
            (
                -not $TargetRouteStage -or
                $_.routeStage -eq $TargetRouteStage
            )
        }
    )
}

function Get-FallbackRatio {
    param([object[]]$EvidenceRecords)

    $records = @(
        $EvidenceRecords | Where-Object {
            (Get-OptionalRecordValue -Record $_ -PropertyName 'implementationTrack') -eq 'actual-primary-lane'
        }
    )
    if ($records.Count -eq 0) {
        return 1.0
    }

    $fallbackCount = @(
        $records | Where-Object {
            (Get-OptionalRecordValue -Record $_ -PropertyName 'implementationTrack') -ne 'actual-primary-lane' -or
            $_.laneOwner -ne 'local-fullscreen-lane' -or (
                $null -ne $_.fallbackReasonCode -and
                $_.fallbackReasonCode -ne '' -and
                $_.fallbackReasonCode -ne 'none'
            )
        }
    ).Count

    return [math]::Round(($fallbackCount / $records.Count), 4)
}

function Read-OracleMetadata {
    param(
        [string]$MetadataPath,
        [string]$ExpectedSessionId,
        [string]$ExpectedCaptureId,
        [string]$ExpectedPresetId,
        [string]$ExpectedPublishedVersion
    )

    if (-not $MetadataPath) {
        throw 'Parity oracle metadata is required when a reference image is provided.'
    }

    if (-not (Test-Path -LiteralPath $MetadataPath)) {
        throw ('Parity oracle metadata not found: {0}' -f $MetadataPath)
    }

    $metadata = Read-JsonFile -Path $MetadataPath
    if (
        $metadata.sessionId -ne $ExpectedSessionId -or
        $metadata.captureId -ne $ExpectedCaptureId -or
        $metadata.presetId -ne $ExpectedPresetId -or
        $metadata.publishedVersion -ne $ExpectedPublishedVersion
    ) {
        throw (
            'Parity oracle must match the same-capture correlation. expected={0}/{1}/{2}/{3} actual={4}/{5}/{6}/{7}' -f
            $ExpectedSessionId,
            $ExpectedCaptureId,
            $ExpectedPresetId,
            $ExpectedPublishedVersion,
            $metadata.sessionId,
            $metadata.captureId,
            $metadata.presetId,
            $metadata.publishedVersion
        )
    }

    return $metadata
}

function Get-ParityMeasurement {
    param(
        [string]$CandidatePath,
        [string]$ReferencePath,
        [string]$ReferenceMetadataPath,
        [string]$ExpectedSessionId,
        [string]$ExpectedCaptureId,
        [string]$ExpectedPresetId,
        [string]$ExpectedPublishedVersion,
        [double]$Threshold
    )

    if (-not $ReferencePath) {
        return [ordered]@{
            status = 'not-run'
            result = 'not-run'
            referencePath = $null
            referenceMetadataPath = $null
            threshold = $Threshold
            numericScore = $null
            maxChannelDelta = $null
            reason = 'reference-not-provided'
        }
    }

    if (-not (Test-Path -LiteralPath $CandidatePath)) {
        return [ordered]@{
            status = 'invalid-input'
            result = 'fail'
            referencePath = $ReferencePath
            referenceMetadataPath = $ReferenceMetadataPath
            threshold = $Threshold
            numericScore = $null
            maxChannelDelta = $null
            reason = 'candidate-missing'
        }
    }

    if (-not (Test-Path -LiteralPath $ReferencePath)) {
        return [ordered]@{
            status = 'invalid-input'
            result = 'fail'
            referencePath = $ReferencePath
            referenceMetadataPath = $ReferenceMetadataPath
            threshold = $Threshold
            numericScore = $null
            maxChannelDelta = $null
            reason = 'reference-missing'
        }
    }

    $null = Read-OracleMetadata `
        -MetadataPath $ReferenceMetadataPath `
        -ExpectedSessionId $ExpectedSessionId `
        -ExpectedCaptureId $ExpectedCaptureId `
        -ExpectedPresetId $ExpectedPresetId `
        -ExpectedPublishedVersion $ExpectedPublishedVersion

    Add-Type -AssemblyName System.Drawing
    try {
        $candidate = New-Object System.Drawing.Bitmap $CandidatePath
        $reference = New-Object System.Drawing.Bitmap $ReferencePath
    }
    catch {
        return [ordered]@{
            status = 'invalid-input'
            result = 'fail'
            referencePath = $ReferencePath
            referenceMetadataPath = $ReferenceMetadataPath
            threshold = $Threshold
            numericScore = $null
            maxChannelDelta = $null
            reason = 'image-decode-failed'
        }
    }

    try {
        if ($candidate.Width -ne $reference.Width -or $candidate.Height -ne $reference.Height) {
            return [ordered]@{
                status = 'invalid-input'
                result = 'fail'
                referencePath = $ReferencePath
                referenceMetadataPath = $ReferenceMetadataPath
                threshold = $Threshold
                numericScore = $null
                maxChannelDelta = $null
                reason = 'dimension-mismatch'
            }
        }

        $totalDelta = 0.0
        $sampleCount = 0
        $maxChannelDelta = 0

        for ($y = 0; $y -lt $candidate.Height; $y++) {
            for ($x = 0; $x -lt $candidate.Width; $x++) {
                $candidatePixel = $candidate.GetPixel($x, $y)
                $referencePixel = $reference.GetPixel($x, $y)
                foreach ($delta in @(
                    [math]::Abs($candidatePixel.R - $referencePixel.R),
                    [math]::Abs($candidatePixel.G - $referencePixel.G),
                    [math]::Abs($candidatePixel.B - $referencePixel.B)
                )) {
                    $totalDelta += $delta
                    $sampleCount += 1
                    if ($delta -gt $maxChannelDelta) {
                        $maxChannelDelta = $delta
                    }
                }
            }
        }

        $numericScore = if ($sampleCount -gt 0) {
            [math]::Round($totalDelta / $sampleCount, 4)
        } else {
            0
        }

        return [ordered]@{
            status = 'measured'
            result = if ($numericScore -le $Threshold) { 'pass' } else { 'fail' }
            referencePath = $ReferencePath
            referenceMetadataPath = $ReferenceMetadataPath
            threshold = $Threshold
            numericScore = $numericScore
            maxChannelDelta = $maxChannelDelta
            reason = if ($numericScore -le $Threshold) { 'within-threshold' } else { 'threshold-exceeded' }
        }
    }
    finally {
        $candidate.Dispose()
        $reference.Dispose()
    }
}

function Copy-Artifacts {
    param(
        [hashtable]$Artifacts,
        [switch]$DryRun
    )

    $missing = New-Object System.Collections.Generic.List[string]

    foreach ($key in $Artifacts.Keys) {
        $artifact = $Artifacts[$key]
        $hasInlineContent = $artifact.ContainsKey('content')
        if (-not $hasInlineContent -and (-not $artifact.source -or -not (Test-Path -LiteralPath $artifact.source))) {
            if ($artifact.required) {
                $missing.Add($artifact.source)
            }
            continue
        }

        if ($DryRun) {
            continue
        }

        $destinationDirectory = Split-Path -Path $artifact.destination -Parent
        New-Item -ItemType Directory -Force -Path $destinationDirectory | Out-Null
        if ($hasInlineContent) {
            $artifact.content | Set-Content -LiteralPath $artifact.destination -Encoding utf8
        }
        else {
            Copy-Item -LiteralPath $artifact.source -Destination $artifact.destination -Force
        }
    }

    return $missing
}

$resolvedRepoRoot = Resolve-RepoRoot -ConfiguredRoot $RepoRoot
$sessionRoot = Join-Path $resolvedRepoRoot ('sessions\{0}' -f $SessionId)
$sessionManifestPath = Join-Path $sessionRoot 'session.json'
$timingEventsPath = Join-Path $sessionRoot 'diagnostics\timing-events.log'
$evidenceLogPath = Join-Path $sessionRoot 'diagnostics\dedicated-renderer\preview-promotion-evidence.jsonl'
$operatorAuditLogPath = Join-Path $resolvedRepoRoot 'diagnostics\operator-audit-log.json'
$generatedAt = (Get-Date).ToString('o')

if (-not (Test-Path -LiteralPath $sessionManifestPath)) {
    throw ('Session manifest not found: {0}' -f $sessionManifestPath)
}

$manifest = Read-JsonFile -Path $sessionManifestPath
$capture = $manifest.captures | Where-Object { $_.captureId -eq $CaptureId } | Select-Object -First 1
if ($null -eq $capture) {
    throw ('Capture not found in session manifest: {0}' -f $CaptureId)
}

$evidenceRecord = Find-EvidenceRecord `
    -Path $evidenceLogPath `
    -TargetCaptureId $CaptureId `
    -TargetSessionId $SessionId `
    -TargetPresetId $PresetId `
    -TargetPublishedVersion $PublishedVersion

if ($null -eq $evidenceRecord) {
    throw (
        'preview promotion evidence record not found for same-capture correlation: {0}/{1}/{2}/{3}' -f
        $SessionId,
        $CaptureId,
        $PresetId,
        $PublishedVersion
    )
}

if ($capture.requestId -ne $evidenceRecord.requestId) {
    throw (
        'wrong-capture attribution risk: selected capture request id does not match evidence record. expected={0} actual={1}' -f
        $capture.requestId,
        $evidenceRecord.requestId
    )
}

if ($capture.sessionId -ne $SessionId) {
    throw (
        'cross-session leak risk: selected capture belongs to a different session. expected={0} actual={1}' -f
        $SessionId,
        $capture.sessionId
    )
}

if (-not $evidenceRecord.PSObject.Properties['visibleOwner'] -or -not $evidenceRecord.visibleOwner) {
    throw 'visible owner transition evidence is missing: visibleOwner must be recorded for the selected capture chain.'
}

if (
    -not $evidenceRecord.PSObject.Properties['visibleOwnerTransitionAtMs'] -or
    $null -eq $evidenceRecord.visibleOwnerTransitionAtMs
) {
    throw 'visible owner transition evidence is missing: visibleOwnerTransitionAtMs must be recorded for the selected capture chain.'
}

foreach ($fieldName in @('captureRequestedAtMs', 'rawPersistedAtMs', 'truthfulArtifactReadyAtMs')) {
    if (
        -not $evidenceRecord.PSObject.Properties[$fieldName] -or
        $null -eq $evidenceRecord.$fieldName
    ) {
        throw ('selected capture chain is incomplete: {0} is required for canonical evidence.' -f $fieldName)
    }
}

if (@($BoothVisualEvidencePaths).Count -eq 0) {
    throw 'At least one booth visual evidence path is required for the canonical evidence bundle.'
}

if (@($OperatorVisualEvidencePaths).Count -eq 0) {
    throw 'At least one operator visual evidence path is required for the canonical evidence bundle.'
}

if (@($RollbackEvidencePaths).Count -eq 0) {
    throw 'At least one rollback evidence path is required for the canonical evidence bundle.'
}

$routePolicySnapshotPath = $evidenceRecord.routePolicySnapshotPath
$publishedBundlePath = if ($evidenceRecord.publishedBundlePath) {
    $evidenceRecord.publishedBundlePath
} else {
    Join-Path $resolvedRepoRoot ('preset-catalog\published\{0}\{1}\bundle.json' -f $PresetId, $PublishedVersion)
}
$catalogStatePath = $evidenceRecord.catalogStatePath
Assert-CaptureTimeSnapshotPath -SnapshotPath $routePolicySnapshotPath -SessionRoot $sessionRoot -Label 'route policy' -CaptureId $CaptureId
Assert-CaptureTimeSnapshotPath -SnapshotPath $catalogStatePath -SessionRoot $sessionRoot -Label 'catalog state' -CaptureId $CaptureId

$evidenceFamily = Find-EvidenceFamily `
    -Path $evidenceLogPath `
    -TargetSessionId $SessionId `
    -TargetPresetId $PresetId `
    -TargetPublishedVersion $PublishedVersion `
    -TargetRouteStage $evidenceRecord.routeStage
$fallbackRatio = Get-FallbackRatio -EvidenceRecords $evidenceFamily
$selectedTimingEvents = Get-SelectedCaptureTimingEvents `
    -Path $timingEventsPath `
    -TargetSessionId $SessionId `
    -TargetCaptureId $CaptureId `
    -TargetRequestId $evidenceRecord.requestId

$candidatePreviewPath = if ($evidenceRecord -and $evidenceRecord.previewAssetPath) {
    $evidenceRecord.previewAssetPath
} elseif ($capture.preview.assetPath) {
    $capture.preview.assetPath
} else {
    $null
}

if (-not $OutputRoot) {
    $OutputRoot = Join-Path $resolvedRepoRoot (
        'artifacts\hardware-validation\{0}\{1}\{2}\{3}' -f
        $SessionId,
        $PresetId,
        $PublishedVersion,
        $CaptureId
    )
}

$bundleManifestPath = Join-Path $OutputRoot 'preview-promotion-evidence-bundle.json'

$artifacts = [ordered]@{
    sessionManifest = @{
        source = $sessionManifestPath
        destination = Join-Path $OutputRoot 'session.json'
        required = $true
    }
    timingEvents = @{
        source = $timingEventsPath
        destination = Join-Path $OutputRoot 'timing-events.log'
        content = (($selectedTimingEvents -join [Environment]::NewLine) + [Environment]::NewLine)
        required = $true
    }
    routePolicySnapshot = @{
        source = $routePolicySnapshotPath
        destination = Join-Path $OutputRoot 'preview-renderer-policy.json'
        required = $true
    }
    publishedBundle = @{
        source = $publishedBundlePath
        destination = Join-Path $OutputRoot 'bundle.json'
        required = $true
    }
    catalogState = @{
        source = $catalogStatePath
        destination = Join-Path $OutputRoot 'catalog-state.json'
        required = $true
    }
    operatorAuditLog = @{
        source = $operatorAuditLogPath
        destination = Join-Path $OutputRoot 'operator-audit-log.json'
        required = $false
    }
    previewPromotionEvidence = @{
        source = $evidenceLogPath
        destination = Join-Path $OutputRoot 'preview-promotion-evidence.jsonl'
        content = (ConvertTo-Json $evidenceRecord -Depth 12 -Compress)
        required = $true
    }
    candidatePreview = @{
        source = $candidatePreviewPath
        destination = Join-Path $OutputRoot 'candidate-preview.jpg'
        required = $true
    }
}

$visualArtifactIndex = 0
foreach ($path in $BoothVisualEvidencePaths) {
    $visualArtifactIndex += 1
    $artifacts["boothVisualEvidence$visualArtifactIndex"] = @{
        source = $path
        destination = Join-Path $OutputRoot ("visual/booth/{0}{1}" -f $visualArtifactIndex, [System.IO.Path]::GetExtension($path))
        required = $true
    }
}

$visualArtifactIndex = 0
foreach ($path in $OperatorVisualEvidencePaths) {
    $visualArtifactIndex += 1
    $artifacts["operatorVisualEvidence$visualArtifactIndex"] = @{
        source = $path
        destination = Join-Path $OutputRoot ("visual/operator/{0}{1}" -f $visualArtifactIndex, [System.IO.Path]::GetExtension($path))
        required = $true
    }
}

$rollbackArtifactIndex = 0
foreach ($path in $RollbackEvidencePaths) {
    $rollbackArtifactIndex += 1
    $artifacts["rollbackEvidence$rollbackArtifactIndex"] = @{
        source = $path
        destination = Join-Path $OutputRoot ("rollback/{0}{1}" -f $rollbackArtifactIndex, [System.IO.Path]::GetExtension($path))
        required = $true
    }
}

$missingArtifacts = Copy-Artifacts -Artifacts $artifacts -DryRun:$DryRun
if (@($missingArtifacts).Count -gt 0) {
    throw ('Required evidence artifacts are missing: {0}' -f (($missingArtifacts | Sort-Object -Unique) -join ', '))
}

$baselineParity = Get-ParityMeasurement `
    -CandidatePath $candidatePreviewPath `
    -ReferencePath $BaselineImagePath `
    -ReferenceMetadataPath $BaselineMetadataPath `
    -ExpectedSessionId $SessionId `
    -ExpectedCaptureId $CaptureId `
    -ExpectedPresetId $PresetId `
    -ExpectedPublishedVersion $PublishedVersion `
    -Threshold $ParityThreshold
$fallbackParity = Get-ParityMeasurement `
    -CandidatePath $candidatePreviewPath `
    -ReferencePath $FallbackImagePath `
    -ReferenceMetadataPath $FallbackMetadataPath `
    -ExpectedSessionId $SessionId `
    -ExpectedCaptureId $CaptureId `
    -ExpectedPresetId $PresetId `
    -ExpectedPublishedVersion $PublishedVersion `
    -Threshold $ParityThreshold

$parityResult = if ($baselineParity.result -eq 'pass') {
    'pass'
} elseif ($fallbackParity.result -eq 'pass') {
    'conditional'
} elseif ($baselineParity.status -eq 'not-run' -and $fallbackParity.status -eq 'not-run') {
    'not-run'
} else {
    'fail'
}

$parityReason = switch ($parityResult) {
    'pass' { 'baseline-within-threshold' }
    'conditional' { 'fallback-within-threshold' }
    'not-run' { 'oracle-not-provided' }
    default { 'threshold-or-input-failure' }
}
$sameCaptureFullScreenVisibleMs = Get-OptionalRecordValue -Record $evidenceRecord -PropertyName 'sameCaptureFullScreenVisibleMs'
$replacementMs = Get-OptionalRecordValue -Record $evidenceRecord -PropertyName 'replacementMs'

$bundle = [ordered]@{
    schemaVersion = 'preview-promotion-evidence-bundle/v1'
    generatedAt = $generatedAt
    sessionId = $SessionId
    captureId = $CaptureId
    requestId = if ($evidenceRecord -and $evidenceRecord.requestId) { $evidenceRecord.requestId } else { $capture.requestId }
    presetId = $PresetId
    publishedVersion = $PublishedVersion
    laneOwner = if ($evidenceRecord) { $evidenceRecord.laneOwner } else { 'unknown' }
    fallbackReasonCode = if ($evidenceRecord) { $evidenceRecord.fallbackReasonCode } else { $null }
    routeStage = if ($evidenceRecord) { $evidenceRecord.routeStage } else { 'shadow' }
    implementationTrack = Get-OptionalRecordValue -Record $evidenceRecord -PropertyName 'implementationTrack'
    warmState = if ($evidenceRecord) { $evidenceRecord.warmState } else { $null }
    captureRequestedAtMs = if ($evidenceRecord) { $evidenceRecord.captureRequestedAtMs } else { $null }
    rawPersistedAtMs = if ($evidenceRecord) { $evidenceRecord.rawPersistedAtMs } else { $null }
    truthfulArtifactReadyAtMs = if ($evidenceRecord) { $evidenceRecord.truthfulArtifactReadyAtMs } else { $null }
    visibleOwner = if ($evidenceRecord) { $evidenceRecord.visibleOwner } else { $null }
    visibleOwnerTransitionAtMs = if ($evidenceRecord) { $evidenceRecord.visibleOwnerTransitionAtMs } else { $null }
    firstVisibleMs = if ($evidenceRecord) { $evidenceRecord.firstVisibleMs } else { $null }
    sameCaptureFullScreenVisibleMs = $sameCaptureFullScreenVisibleMs
    replacementMs = $replacementMs
    originalVisibleToPresetAppliedVisibleMs = if ($evidenceRecord) { $evidenceRecord.originalVisibleToPresetAppliedVisibleMs } else { $null }
    fallbackRatio = $fallbackRatio
    outputRoot = $OutputRoot
    bundleManifestPath = $bundleManifestPath
    artifacts = $artifacts
    missingArtifacts = @($missingArtifacts)
    visualEvidence = [ordered]@{
        booth = @($BoothVisualEvidencePaths)
        operator = @($OperatorVisualEvidencePaths)
    }
    rollbackEvidence = @(
        $artifacts.Keys |
            Where-Object { $_ -like 'rollbackEvidence*' } |
            Sort-Object |
            ForEach-Object { $artifacts[$_].destination }
    )
    parity = [ordered]@{
        result = $parityResult
        reason = $parityReason
        threshold = $ParityThreshold
        baseline = $baselineParity
        fallback = $fallbackParity
    }
}

if (-not $DryRun) {
    New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null
    $bundle | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $bundleManifestPath -Encoding utf8
}

if ($EmitJson) {
    $bundle | ConvertTo-Json -Depth 12
    return
}

Write-Host ('Preview promotion evidence bundle ready: {0}' -f $OutputRoot)
