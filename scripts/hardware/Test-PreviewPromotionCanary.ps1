[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$BundlePath,
    [string]$OutputPath,
    [int]$PrimaryThresholdMs = 2500,
    [double]$MaxFallbackRatio = 0,
    [int]$MaxEvidenceAgeMinutes = 7200,
    [switch]$EmitJson
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Read-JsonFile {
    param([string]$Path)

    return Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Get-ObjectPropertyValue {
    param(
        $Object,
        [string]$PropertyName
    )

    if (
        $null -eq $Object -or
        -not $Object.PSObject.Properties[$PropertyName]
    ) {
        return $null
    }

    return $Object.$PropertyName
}

function New-CheckResult {
    param(
        [string]$Status,
        [string]$Reason,
        [hashtable]$Additional = @{}
    )

    $result = [ordered]@{
        status = $Status
        reason = $Reason
    }

    foreach ($key in $Additional.Keys) {
        $result[$key] = $Additional[$key]
    }

    return $result
}

function Convert-ToTrimmedString {
    param(
        $Value,
        [string]$FieldName,
        [switch]$AllowNull
    )

    if ($null -eq $Value) {
        if ($AllowNull) {
            return $null
        }

        throw ("malformed bundle: {0} is missing." -f $FieldName)
    }

    if (
        $Value -is [System.Collections.IDictionary] -or
        ($Value -is [System.Collections.IEnumerable] -and -not ($Value -is [string]))
    ) {
        throw ("malformed bundle: {0} must be a string value." -f $FieldName)
    }

    $text = ([string]$Value).Trim()
    if ([string]::IsNullOrWhiteSpace($text)) {
        if ($AllowNull) {
            return $null
        }

        throw ("malformed bundle: {0} is empty." -f $FieldName)
    }

    return $text
}

function Convert-ToAllowedString {
    param(
        $Value,
        [string]$FieldName,
        [string[]]$AllowedValues,
        [switch]$AllowNull
    )

    $text = Convert-ToTrimmedString -Value $Value -FieldName $FieldName -AllowNull:$AllowNull
    if ($null -eq $text) {
        return $null
    }

    if ($AllowedValues -notcontains $text) {
        throw ("malformed bundle: {0} must be one of [{1}], not {2}." -f $FieldName, ($AllowedValues -join ', '), $text)
    }

    return $text
}

function Convert-ToNullableInt {
    param(
        $Value,
        [string]$FieldName
    )

    if ($null -eq $Value) {
        return $null
    }

    if ($Value -is [System.Collections.IDictionary] -or $Value -is [System.Collections.IEnumerable] -and -not ($Value -is [string])) {
        throw ("malformed bundle: {0} must be an integer." -f $FieldName)
    }

    try {
        return [int]$Value
    }
    catch {
        throw ("malformed bundle: {0} must be an integer." -f $FieldName)
    }
}

function Convert-ToRatio {
    param(
        $Value,
        [string]$FieldName,
        [double]$DefaultValue
    )

    if ($null -eq $Value) {
        return $DefaultValue
    }

    if ($Value -is [System.Collections.IDictionary] -or $Value -is [System.Collections.IEnumerable] -and -not ($Value -is [string])) {
        throw ("malformed bundle: {0} must be a numeric ratio." -f $FieldName)
    }

    try {
        $ratio = [double]$Value
    }
    catch {
        throw ("malformed bundle: {0} must be a numeric ratio." -f $FieldName)
    }

    if ($ratio -lt 0 -or $ratio -gt 1) {
        throw ("malformed bundle: {0} must stay between 0 and 1." -f $FieldName)
    }

    return $ratio
}

function Convert-ToDateTimeOffset {
    param(
        $Value,
        [string]$FieldName
    )

    $text = Convert-ToTrimmedString -Value $Value -FieldName $FieldName
    try {
        return [DateTimeOffset]::Parse($text, [System.Globalization.CultureInfo]::InvariantCulture)
    }
    catch {
        throw ("malformed bundle: {0} must be a valid datetime with offset." -f $FieldName)
    }
}

function Test-PathWithinRoot {
    param(
        [string]$Path,
        [string]$Root
    )

    if (
        [string]::IsNullOrWhiteSpace($Path) -or
        [string]::IsNullOrWhiteSpace($Root) -or
        -not (Test-Path -LiteralPath $Path) -or
        -not (Test-Path -LiteralPath $Root)
    ) {
        return $false
    }

    $candidatePath = [System.IO.Path]::GetFullPath((Resolve-Path -LiteralPath $Path).Path).TrimEnd('\', '/')
    $rootPath = [System.IO.Path]::GetFullPath((Resolve-Path -LiteralPath $Root).Path).TrimEnd('\', '/')
    if ($candidatePath.Equals($rootPath, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $true
    }

    return $candidatePath.StartsWith(
        $rootPath + [System.IO.Path]::DirectorySeparatorChar,
        [System.StringComparison]::OrdinalIgnoreCase
    )
}

function Get-EventDetailValue {
    param(
        [string]$Line,
        [string]$Key
    )

    $match = [regex]::Match($Line, '(?:^|[;\t]|detail=)' + [regex]::Escape($Key) + '=([^;\t]+)')
    if (-not $match.Success) {
        return $null
    }

    return $match.Groups[1].Value
}

function Test-HasFallbackReason {
    param($FallbackReasonCode)

    if ($null -eq $FallbackReasonCode) {
        return $false
    }

    $reason = Convert-ToTrimmedString -Value $FallbackReasonCode -FieldName 'fallbackReasonCode' -AllowNull
    if ($null -eq $reason) {
        return $false
    }

    return $reason -ne 'none'
}

function Get-BundledPromotionEvidenceRecord {
    param(
        [string]$Path,
        [string]$BundleRoot
    )

    if (-not (Test-PathWithinRoot -Path $Path -Root $BundleRoot)) {
        return $null
    }

    $lines = @(Get-Content -LiteralPath $Path | Where-Object { $_.Trim() })
    if ($lines.Count -eq 0) {
        throw 'malformed bundle: bundled preview-promotion evidence copy is empty.'
    }

    try {
        return $lines[-1] | ConvertFrom-Json
    }
    catch {
        throw 'malformed bundle: bundled preview-promotion evidence copy is not valid JSON.'
    }
}

function Get-EvidenceFreshnessCheck {
    param(
        [string]$Path,
        [string]$BundleRoot,
        [DateTimeOffset]$GeneratedAt,
        [int]$MaxAgeMinutes
    )

    if (-not (Test-PathWithinRoot -Path $Path -Root $BundleRoot)) {
        return New-CheckResult -Status 'fail' -Reason 'bundled preview-promotion evidence copy is missing or outside the assembled bundle.'
    }

    $record = Get-BundledPromotionEvidenceRecord -Path $Path -BundleRoot $BundleRoot
    $observedAt = Convert-ToDateTimeOffset -Value (Get-ObjectPropertyValue -Object $record -PropertyName 'observedAt') -FieldName 'previewPromotionEvidence.observedAt'
    $ageMinutes = ($GeneratedAt - $observedAt).TotalMinutes

    if ($ageMinutes -lt 0) {
        return New-CheckResult -Status 'fail' -Reason 'bundled preview-promotion evidence is newer than the bundle manifest timestamp.'
    }

    if ($ageMinutes -gt $MaxAgeMinutes) {
        return New-CheckResult -Status 'fail' -Reason ('bundled preview-promotion evidence is stale ({0:N1} minutes old).' -f $ageMinutes)
    }

    return New-CheckResult -Status 'pass' -Reason 'bundled preview-promotion evidence is fresh'
}

function Get-FollowUpCaptureHealthCheck {
    param(
        [string]$Path,
        [string]$BundleRoot,
        [string]$SessionId,
        [DateTimeOffset]$SelectedObservedAt
    )

    if (-not (Test-PathWithinRoot -Path $Path -Root $BundleRoot)) {
        return New-CheckResult -Status 'fail' -Reason 'follow-up capture audit is missing from the assembled bundle.'
    }

    $auditStore = Read-JsonFile -Path $Path
    $entries = @(Get-ObjectPropertyValue -Object $auditStore -PropertyName 'entries')
    if ($null -eq $entries) {
        throw 'malformed bundle: operator audit store is missing entries.'
    }

    $blockingReasonCodes = @(
        'capture-timeout',
        'capture-rejected',
        'capture-recovery-required',
        'capture-session-mismatch',
        'capture-file-missing',
        'capture-file-empty',
        'capture-file-unscoped',
        'capture-protocol-violation'
    )

    $followUpFailure = @(
        $entries | Where-Object {
            (Get-ObjectPropertyValue -Object $_ -PropertyName 'sessionId') -eq $SessionId -and
            (Get-ObjectPropertyValue -Object $_ -PropertyName 'eventCategory') -eq 'critical-failure' -and
            (Get-ObjectPropertyValue -Object $_ -PropertyName 'eventType') -eq 'capture-round-trip-failed' -and
            $blockingReasonCodes -contains (Get-ObjectPropertyValue -Object $_ -PropertyName 'reasonCode') -and
            (Convert-ToDateTimeOffset -Value (Get-ObjectPropertyValue -Object $_ -PropertyName 'occurredAt') -FieldName 'operatorAudit.entries[].occurredAt') -ge $SelectedObservedAt
        } | Select-Object -First 1
    )

    if ($followUpFailure.Count -gt 0) {
        return New-CheckResult -Status 'fail' -Reason (
            'follow-up capture health degraded after the selected capture chain: {0}.' -f
            (Get-ObjectPropertyValue -Object $followUpFailure[0] -PropertyName 'reasonCode')
        )
    }

    return New-CheckResult -Status 'pass' -Reason 'follow-up capture health stayed clear after the selected capture chain'
}

function Test-SelectedCaptureTimingChain {
    param(
        [string]$Path,
        [string]$BundleRoot,
        [string]$SessionId,
        [string]$CaptureId,
        [string]$RequestId,
        [string]$ExpectedVisibleOwner,
        [Nullable[int]]$ExpectedVisibleOwnerTransitionAtMs,
        [string]$ExpectedLaneOwner,
        [string]$ExpectedRouteStage
    )

    if (-not (Test-PathWithinRoot -Path $Path -Root $BundleRoot)) {
        return New-CheckResult -Status 'fail' -Reason 'selected timing chain is missing from the assembled bundle output.'
    }

    $lines = @(Get-Content -LiteralPath $Path | Where-Object { $_.Trim() })
    $selectedLines = @(
        $lines | Where-Object {
            $_ -like "*session=$SessionId*" -and
            $_ -like "*request=$RequestId*" -and
            (
                $_ -like '*event=request-capture*' -or
                $_ -like '*event=capture-accepted*' -or
                $_ -like "*capture=$CaptureId*"
            )
        }
    )

    if ($selectedLines.Count -eq 0) {
        return New-CheckResult -Status 'fail' -Reason 'selected capture correlation is missing from the timing chain.'
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
            return New-CheckResult -Status 'fail' -Reason ("selected capture timing chain is incomplete: missing {0}." -f $requiredEvent)
        }
    }

    if (
        -not @(
            $selectedLines | Where-Object {
                $_ -like '*event=fast-preview-ready*' -or
                $_ -like '*event=current-session-preview-pending-visible*' -or
                $_ -like '*event=recent-session-pending-visible*' -or
                $_ -like '*event=fast-thumbnail-attempted*' -or
                $_ -like '*event=fast-thumbnail-failed*'
            }
        ).Count
    ) {
        return New-CheckResult -Status 'fail' -Reason 'selected capture timing chain is incomplete: missing fast-preview seam.'
    }

    $recentSessionVisibleLine = @($selectedLines | Where-Object { $_ -like '*event=recent-session-visible*' })[-1]
    $summaryLine = @($selectedLines | Where-Object { $_ -like '*event=capture_preview_transition_summary*' })[-1]

    if (
        -not [string]::IsNullOrWhiteSpace($ExpectedVisibleOwner) -and
        (Get-EventDetailValue -Line $recentSessionVisibleLine -Key 'visibleOwner') -ne $ExpectedVisibleOwner
    ) {
        return New-CheckResult -Status 'fail' -Reason 'selected capture timing chain drifted to a different visible owner.'
    }

    if (
        $null -ne $ExpectedVisibleOwnerTransitionAtMs -and
        (Get-EventDetailValue -Line $recentSessionVisibleLine -Key 'visibleOwnerTransitionAtMs') -ne ([string]$ExpectedVisibleOwnerTransitionAtMs)
    ) {
        return New-CheckResult -Status 'fail' -Reason 'selected capture timing chain drifted to a different visible owner transition timestamp.'
    }

    if (
        -not [string]::IsNullOrWhiteSpace($ExpectedLaneOwner) -and
        (Get-EventDetailValue -Line $summaryLine -Key 'laneOwner') -ne $ExpectedLaneOwner
    ) {
        return New-CheckResult -Status 'fail' -Reason 'selected capture timing chain drifted to a different lane owner.'
    }

    if (
        -not [string]::IsNullOrWhiteSpace($ExpectedRouteStage) -and
        (Get-EventDetailValue -Line $summaryLine -Key 'routeStage') -ne $ExpectedRouteStage
    ) {
        return New-CheckResult -Status 'fail' -Reason 'selected capture timing chain drifted to a different route stage.'
    }

    return New-CheckResult -Status 'pass' -Reason 'selected-capture timing chain preserved'
}

function New-MalformedBundleAssessment {
    param(
        [string]$BundleManifestPath,
        $Bundle,
        [string]$Reason,
        [int]$ThresholdMs,
        [double]$ThresholdRatio
    )

    $sessionIdFallback = 'session_00000000000000000000000000'
    $captureIdFallback = 'capture_unknown'
    $requestIdFallback = 'request_unknown'
    $presetIdFallback = 'preset_unknown'
    $publishedVersionFallback = '1970.01.01'

    $sessionId = $sessionIdFallback
    $captureId = $captureIdFallback
    $requestId = $requestIdFallback
    $presetId = $presetIdFallback
    $publishedVersion = $publishedVersionFallback

    if ($null -ne $Bundle) {
        $sessionIdCandidate = Get-ObjectPropertyValue -Object $Bundle -PropertyName 'sessionId'
        if ($null -ne $sessionIdCandidate -and ([string]$sessionIdCandidate) -match '^session_[a-z0-9]{26}$') {
            $sessionId = [string]$sessionIdCandidate
        }

        $captureIdCandidate = Get-ObjectPropertyValue -Object $Bundle -PropertyName 'captureId'
        if ($null -ne $captureIdCandidate -and -not [string]::IsNullOrWhiteSpace([string]$captureIdCandidate)) {
            $captureId = [string]$captureIdCandidate
        }

        $requestIdCandidate = Get-ObjectPropertyValue -Object $Bundle -PropertyName 'requestId'
        if ($null -ne $requestIdCandidate -and -not [string]::IsNullOrWhiteSpace([string]$requestIdCandidate)) {
            $requestId = [string]$requestIdCandidate
        }

        $presetIdCandidate = Get-ObjectPropertyValue -Object $Bundle -PropertyName 'presetId'
        if ($null -ne $presetIdCandidate -and ([string]$presetIdCandidate) -match '^preset_[a-z0-9-]+$') {
            $presetId = [string]$presetIdCandidate
        }

        $publishedVersionCandidate = Get-ObjectPropertyValue -Object $Bundle -PropertyName 'publishedVersion'
        if ($null -ne $publishedVersionCandidate -and ([string]$publishedVersionCandidate) -match '^\d{4}\.\d{2}\.\d{2}$') {
            $publishedVersion = [string]$publishedVersionCandidate
        }
    }

    return [ordered]@{
        schemaVersion = 'preview-promotion-canary-assessment/v1'
        generatedAt = (Get-Date).ToString('o')
        bundleManifestPath = $BundleManifestPath
        sessionId = $sessionId
        captureId = $captureId
        requestId = $requestId
        presetId = $presetId
        publishedVersion = $publishedVersion
        routeStage = 'unknown'
        laneOwner = 'unknown'
        gate = 'No-Go'
        nextStageAllowed = $false
        summary = 'canary bundle remains No-Go because the assessment input is malformed.'
        blockers = @('malformed-bundle')
        checks = [ordered]@{
            kpi = New-CheckResult -Status 'fail' -Reason $Reason -Additional @{
                actualMs = $null
                thresholdMs = $ThresholdMs
            }
            fallbackStability = New-CheckResult -Status 'fail' -Reason $Reason -Additional @{
                actualRatio = 1.0
                thresholdRatio = $ThresholdRatio
            }
            wrongCapture = New-CheckResult -Status 'fail' -Reason $Reason
            fidelityDrift = New-CheckResult -Status 'fail' -Reason $Reason -Additional @{
                parityResult = 'not-run'
            }
            rollbackReadiness = New-CheckResult -Status 'fail' -Reason $Reason -Additional @{
                evidenceCount = 0
            }
            activeSessionSafety = New-CheckResult -Status 'fail' -Reason $Reason
        }
    }
}

$bundle = $null
$resolvedBundlePath = try {
    (Resolve-Path -LiteralPath $BundlePath).Path
}
catch {
    $BundlePath
}
$bundleRoot = Split-Path -Path $resolvedBundlePath -Parent

try {
    $bundle = Read-JsonFile -Path $resolvedBundlePath

    $generatedAt = Convert-ToDateTimeOffset -Value (Get-ObjectPropertyValue -Object $bundle -PropertyName 'generatedAt') -FieldName 'generatedAt'
    $sessionId = Convert-ToTrimmedString -Value (Get-ObjectPropertyValue -Object $bundle -PropertyName 'sessionId') -FieldName 'sessionId'
    $captureId = Convert-ToTrimmedString -Value (Get-ObjectPropertyValue -Object $bundle -PropertyName 'captureId') -FieldName 'captureId'
    $requestId = Convert-ToTrimmedString -Value (Get-ObjectPropertyValue -Object $bundle -PropertyName 'requestId') -FieldName 'requestId'
    $presetId = Convert-ToTrimmedString -Value (Get-ObjectPropertyValue -Object $bundle -PropertyName 'presetId') -FieldName 'presetId'
    $publishedVersion = Convert-ToTrimmedString -Value (Get-ObjectPropertyValue -Object $bundle -PropertyName 'publishedVersion') -FieldName 'publishedVersion'
    $routeStage = Convert-ToTrimmedString -Value (Get-ObjectPropertyValue -Object $bundle -PropertyName 'routeStage') -FieldName 'routeStage'
    $laneOwner = Convert-ToTrimmedString -Value (Get-ObjectPropertyValue -Object $bundle -PropertyName 'laneOwner') -FieldName 'laneOwner'
    $fallbackReasonCode = Convert-ToTrimmedString -Value (Get-ObjectPropertyValue -Object $bundle -PropertyName 'fallbackReasonCode') -FieldName 'fallbackReasonCode' -AllowNull
    $sameCaptureFullScreenVisibleMs = Convert-ToNullableInt -Value (Get-ObjectPropertyValue -Object $bundle -PropertyName 'sameCaptureFullScreenVisibleMs') -FieldName 'sameCaptureFullScreenVisibleMs'
    $fallbackRatioValue = Convert-ToRatio -Value (Get-ObjectPropertyValue -Object $bundle -PropertyName 'fallbackRatio') -FieldName 'fallbackRatio' -DefaultValue 1.0
    $visibleOwner = Convert-ToTrimmedString -Value (Get-ObjectPropertyValue -Object $bundle -PropertyName 'visibleOwner') -FieldName 'visibleOwner' -AllowNull
    $visibleOwnerTransitionAtMs = Convert-ToNullableInt -Value (Get-ObjectPropertyValue -Object $bundle -PropertyName 'visibleOwnerTransitionAtMs') -FieldName 'visibleOwnerTransitionAtMs'
    $parity = Get-ObjectPropertyValue -Object $bundle -PropertyName 'parity'
    $parityResult = Convert-ToAllowedString -Value (Get-ObjectPropertyValue -Object $parity -PropertyName 'result') -FieldName 'parity.result' -AllowedValues @('pass', 'conditional', 'not-run', 'fail')
    $rollbackEvidence = @()
    if ($bundle.PSObject.Properties['rollbackEvidence']) {
        $rollbackEvidence = @($bundle.rollbackEvidence)
    }

    $artifacts = Get-ObjectPropertyValue -Object $bundle -PropertyName 'artifacts'
    if ($null -eq $artifacts) {
        throw 'malformed bundle: artifacts section is missing.'
    }

    $timingEventsPath = Convert-ToTrimmedString -Value (Get-ObjectPropertyValue -Object (Get-ObjectPropertyValue -Object $artifacts -PropertyName 'timingEvents') -PropertyName 'destination') -FieldName 'artifacts.timingEvents.destination'
    $previewPromotionEvidencePath = Convert-ToTrimmedString -Value (Get-ObjectPropertyValue -Object (Get-ObjectPropertyValue -Object $artifacts -PropertyName 'previewPromotionEvidence') -PropertyName 'destination') -FieldName 'artifacts.previewPromotionEvidence.destination'
    $routePolicyPath = Convert-ToTrimmedString -Value (Get-ObjectPropertyValue -Object (Get-ObjectPropertyValue -Object $artifacts -PropertyName 'routePolicySnapshot') -PropertyName 'destination') -FieldName 'artifacts.routePolicySnapshot.destination' -AllowNull
    $catalogStatePath = Convert-ToTrimmedString -Value (Get-ObjectPropertyValue -Object (Get-ObjectPropertyValue -Object $artifacts -PropertyName 'catalogState') -PropertyName 'destination') -FieldName 'artifacts.catalogState.destination' -AllowNull
    $operatorAuditLogPath = Convert-ToTrimmedString -Value (Get-ObjectPropertyValue -Object (Get-ObjectPropertyValue -Object $artifacts -PropertyName 'operatorAuditLog') -PropertyName 'destination') -FieldName 'artifacts.operatorAuditLog.destination' -AllowNull

    $checks = [ordered]@{}
    $blockers = New-Object System.Collections.Generic.List[string]

    $checks.kpi = if (
        $null -ne $sameCaptureFullScreenVisibleMs -and
        $sameCaptureFullScreenVisibleMs -le $PrimaryThresholdMs
    ) {
        New-CheckResult -Status 'pass' -Reason 'within-threshold' -Additional @{
            actualMs = $sameCaptureFullScreenVisibleMs
            thresholdMs = $PrimaryThresholdMs
        }
    }
    else {
        $reason = if ($null -eq $sameCaptureFullScreenVisibleMs) {
            'sameCaptureFullScreenVisibleMs is missing from the canary bundle.'
        }
        else {
            'sameCaptureFullScreenVisibleMs exceeded the canary threshold.'
        }
        $blockers.Add('kpi-miss') | Out-Null
        New-CheckResult -Status 'fail' -Reason $reason -Additional @{
            actualMs = $sameCaptureFullScreenVisibleMs
            thresholdMs = $PrimaryThresholdMs
        }
    }

    $selectedCaptureFallback = $laneOwner -ne 'dedicated-renderer' -or (
        Test-HasFallbackReason -FallbackReasonCode $fallbackReasonCode
    )
    $checks.fallbackStability = if (
        -not $selectedCaptureFallback -and
        $fallbackRatioValue -le $MaxFallbackRatio
    ) {
        New-CheckResult -Status 'pass' -Reason 'no-fallback-observed' -Additional @{
            actualRatio = $fallbackRatioValue
            thresholdRatio = $MaxFallbackRatio
        }
    }
    else {
        $blockers.Add('fallback-instability') | Out-Null
        New-CheckResult -Status 'fail' -Reason 'fallback-heavy or fallback-owned evidence remains in the canary family.' -Additional @{
            actualRatio = $fallbackRatioValue
            thresholdRatio = $MaxFallbackRatio
        }
    }

    $checks.wrongCapture = Test-SelectedCaptureTimingChain `
        -Path $timingEventsPath `
        -BundleRoot $bundleRoot `
        -SessionId $sessionId `
        -CaptureId $captureId `
        -RequestId $requestId `
        -ExpectedVisibleOwner $visibleOwner `
        -ExpectedVisibleOwnerTransitionAtMs $visibleOwnerTransitionAtMs `
        -ExpectedLaneOwner $laneOwner `
        -ExpectedRouteStage $routeStage
    if ($checks.wrongCapture.status -ne 'pass') {
        $blockers.Add('wrong-capture') | Out-Null
    }

    $checks.fidelityDrift = if ($parityResult -eq 'pass') {
        New-CheckResult -Status 'pass' -Reason 'baseline-within-threshold' -Additional @{
            parityResult = $parityResult
        }
    }
    else {
        $blockers.Add('fidelity-drift') | Out-Null
        New-CheckResult -Status 'fail' -Reason 'parity indicates fidelity drift or insufficient oracle proof.' -Additional @{
            parityResult = $parityResult
        }
    }

    $rollbackEvidenceCount = @(
        $rollbackEvidence | ForEach-Object {
            $rollbackPath = Convert-ToTrimmedString -Value $_ -FieldName 'rollbackEvidence[]' -AllowNull
            if ($null -ne $rollbackPath -and (Test-PathWithinRoot -Path $rollbackPath -Root $bundleRoot)) {
                $rollbackPath
            }
        }
    ).Count
    $checks.rollbackReadiness = if ($rollbackEvidenceCount -gt 0) {
        New-CheckResult -Status 'pass' -Reason 'rollback proof is present for the canary package.' -Additional @{
            evidenceCount = $rollbackEvidenceCount
        }
    }
    else {
        $blockers.Add('rollback-proof-missing') | Out-Null
        New-CheckResult -Status 'fail' -Reason 'one-action rollback proof is missing from the assembled canary bundle.' -Additional @{
            evidenceCount = 0
        }
    }

    $evidenceFreshnessCheck = Get-EvidenceFreshnessCheck `
        -Path $previewPromotionEvidencePath `
        -BundleRoot $bundleRoot `
        -GeneratedAt $generatedAt `
        -MaxAgeMinutes $MaxEvidenceAgeMinutes
    $selectedObservedAt = Convert-ToDateTimeOffset `
        -Value (Get-ObjectPropertyValue -Object (Get-BundledPromotionEvidenceRecord -Path $previewPromotionEvidencePath -BundleRoot $bundleRoot) -PropertyName 'observedAt') `
        -FieldName 'previewPromotionEvidence.observedAt'
    $followUpCaptureHealthCheck = Get-FollowUpCaptureHealthCheck `
        -Path $operatorAuditLogPath `
        -BundleRoot $bundleRoot `
        -SessionId $sessionId `
        -SelectedObservedAt $selectedObservedAt

    $checks.activeSessionSafety = if (
        $evidenceFreshnessCheck.status -eq 'pass' -and
        $followUpCaptureHealthCheck.status -eq 'pass' -and
        $routeStage -eq 'canary' -and
        -not [string]::IsNullOrWhiteSpace($visibleOwner) -and
        $null -ne $visibleOwnerTransitionAtMs -and
        (Test-PathWithinRoot -Path $routePolicyPath -Root $bundleRoot) -and
        (Test-PathWithinRoot -Path $catalogStatePath -Root $bundleRoot)
    ) {
        New-CheckResult -Status 'pass' -Reason 'capture-time route snapshot and canary scope preserved'
    }
    else {
        if ($evidenceFreshnessCheck.status -ne 'pass') {
            $blockers.Add('stale-evidence') | Out-Null
            New-CheckResult -Status 'fail' -Reason $evidenceFreshnessCheck.reason
        }
        elseif ($followUpCaptureHealthCheck.status -ne 'pass') {
            $blockers.Add('follow-up-capture-health') | Out-Null
            New-CheckResult -Status 'fail' -Reason $followUpCaptureHealthCheck.reason
        }
        else {
            $blockers.Add('active-session-safety') | Out-Null
            New-CheckResult -Status 'fail' -Reason 'canary scope or capture-time safety snapshot is missing.'
        }
    }

    $uniqueBlockers = @($blockers | Select-Object -Unique)
    $gate = if ($uniqueBlockers.Count -eq 0) { 'Go' } else { 'No-Go' }
    $nextStageAllowed = $gate -eq 'Go'
    $summary = if ($gate -eq 'Go') {
        'approved canary bundle cleared KPI, fallback, wrong-capture, fidelity, rollback, and safety checks.'
    }
    else {
        'canary bundle remains No-Go until the recorded blockers are resolved.'
    }

    $assessment = [ordered]@{
        schemaVersion = 'preview-promotion-canary-assessment/v1'
        generatedAt = (Get-Date).ToString('o')
        bundleManifestPath = $resolvedBundlePath
        sessionId = $sessionId
        captureId = $captureId
        requestId = $requestId
        presetId = $presetId
        publishedVersion = $publishedVersion
        routeStage = $routeStage
        laneOwner = $laneOwner
        gate = $gate
        nextStageAllowed = $nextStageAllowed
        summary = $summary
        blockers = $uniqueBlockers
        checks = $checks
    }
}
catch {
    $assessment = New-MalformedBundleAssessment `
        -BundleManifestPath $resolvedBundlePath `
        -Bundle $bundle `
        -Reason $_.Exception.Message `
        -ThresholdMs $PrimaryThresholdMs `
        -ThresholdRatio $MaxFallbackRatio
}

if (-not $OutputPath) {
    $OutputPath = Join-Path $bundleRoot 'preview-promotion-canary-assessment.json'
}

$assessment | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $OutputPath -Encoding utf8

if ($EmitJson) {
    $assessment | ConvertTo-Json -Depth 10
    return
}

Write-Host ('Preview promotion canary assessment ready: {0}' -f $OutputPath)
