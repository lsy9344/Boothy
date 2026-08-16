[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

$gateScript = Join-Path $PSScriptRoot 'check-source-completeness.ps1'
$powerShellExe = (Get-Process -Id $PID).Path
$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$fixtureRoot = Join-Path $tempRoot ("boothy-hv14-gate-{0}" -f [guid]::NewGuid().ToString('N'))

function New-ValidSamples {
  $samples = @()
  for ($blockIndex = 0; $blockIndex -lt 35; $blockIndex++) {
    $blockOrder = if (($blockIndex % 2) -eq 0) { 'AB' } else { 'BA' }
    $routes = if ($blockOrder -eq 'AB') {
      @('embedded-jpeg', 'camera-paired-jpeg', 'windows-shell-thumbnail')
    }
    else {
      @('camera-paired-jpeg', 'embedded-jpeg', 'windows-shell-thumbnail')
    }

    foreach ($route in $routes) {
      $objectIndex = if ($route -eq 'camera-paired-jpeg') { 1 } else { 0 }
      $sample = [ordered]@{
        schemaVersion = 'source-comparison/v1'
        candidate = [ordered]@{
          captureId = ('capture-{0:d2}' -f $blockIndex)
          requestId = ('request-{0:d2}' -f $blockIndex)
          sessionId = 'session-synthetic-hv14'
          route = $route
          assetPath = ('C:/synthetic/{0}-{1}.jpg' -f $blockIndex, $route)
          widthPx = 5184
          heightPx = 3456
          byteSize = 1024
          exifOrientation = 1
          decodeValid = $true
          sourceHash = ('hash-{0:d2}-{1}' -f $blockIndex, $route)
          objectIndex = $objectIndex
          groupId = 42
          readyAtHostMicros = 1000000 + $blockIndex
          extractionCostMicros = 12000
        }
        accepted = $true
        rejectReason = $null
        objectRole = 'jpeg'
        usedFallbackCorrelation = $false
        blockOrder = $blockOrder
        blockIndex = $blockIndex
        isWarmUp = $blockIndex -lt 5
        randomizationSeed = 42
        isPresetApplied = $false
        recordedAtHostMicros = 2000000 + $blockIndex
      }
      $samples += [pscustomobject]$sample
    }
  }
  return $samples
}

function Write-Fixture {
  param(
    [string]$Name,
    [object[]]$Samples
  )

  $sessionEvidenceDir = Join-Path (Join-Path $fixtureRoot $Name) 'session-evidence'
  $diagnosticsDir = Join-Path $sessionEvidenceDir 'diagnostics'
  New-Item -ItemType Directory -Path $diagnosticsDir -Force | Out-Null
  $jsonLines = @($Samples | ForEach-Object { $_ | ConvertTo-Json -Depth 8 -Compress })
  Set-Content -LiteralPath (Join-Path $diagnosticsDir 'source-comparison.jsonl') -Value $jsonLines -Encoding utf8
  return $sessionEvidenceDir
}

function Invoke-GateCase {
  param(
    [string]$Name,
    [object[]]$Samples,
    [int]$ExpectedExitCode,
    [string]$ExpectedText,
    [switch]$FinalPackageGate,
    [switch]$CompleteFinalPackage,
    [ValidateSet('', 'environment', 'capability', 'correlation', 'quality', 'aggregate', 'decision')]
    [string]$InvalidFinalPackagePart = '',
    [string]$AppendRawLine
  )

  $sessionEvidenceDir = Write-Fixture -Name $Name -Samples $Samples
  if ($AppendRawLine) {
    Add-Content -LiteralPath (Join-Path $sessionEvidenceDir 'diagnostics/source-comparison.jsonl') -Value $AppendRawLine -Encoding utf8
  }
  if ($CompleteFinalPackage.IsPresent) {
    $runDir = Split-Path -Parent $sessionEvidenceDir
    $environmentValues = [ordered]@{
      'recordedAt' = '2026-08-13T12:00:00+09:00'
      'validator' = 'HV-14 operator'
      'booth PC' = 'NOAH_WIN'
      'app commit' = '0123456789abcdef'
      'OS' = 'Windows 11 Pro build 26200'
      'CPU / RAM' = 'Intel Core i9-9900KF / 63.9 GB'
      'GPU / driver' = 'NVIDIA GTX 1080 / 560.94'
      'camera' = 'Canon EOS 700D'
      'camera firmware' = '1.1.5'
      'lens' = 'EF-S 18-55mm'
      'memory card' = 'SanDisk 64GB U3'
      'power' = 'AC adapter'
      'USB port / cable / hub' = 'USB 3.0 / certified cable / no hub'
      'Canon EDSDK' = '13.19.0'
      'helper / protocol' = '0.1.0 / camera-helper-sidecar-v2'
      'source comparison mode' = 'ab'
      'display sample mode' = 'off'
      'Image Quality before' = 'RAW+JPEG Large Fine'
      'Image Quality after' = 'RAW+JPEG Large Fine'
    }
    if ($InvalidFinalPackagePart -eq 'environment') {
      $environmentValues['camera firmware'] = '<confirm>'
    }
    $environmentLines = @('# HV-14 Environment', '') + @($environmentValues.GetEnumerator() | ForEach-Object { "- $($_.Key): ``$($_.Value)``" })
    Set-Content -LiteralPath (Join-Path $runDir 'environment.md') -Value $environmentLines -Encoding utf8

    $capabilityDir = Join-Path $runDir 'capability'
    New-Item -ItemType Directory -Path $capabilityDir -Force | Out-Null
    $capability = [ordered]@{
      schemaVersion = 'hv-14-capability/v1'
      currentBefore = '0x00640013'
      currentAfter = '0x00640013'
      rawPlusJpegSupported = $true
      descriptorValues = @('0x00640013', '0x00640010')
      combinations = @(
        [ordered]@{ value = '0x00640013'; status = 'attempted'; reason = 'descriptor-supported' },
        [ordered]@{ value = '0x00130000'; status = 'skipped'; reason = 'descriptor-absent' }
      )
    }
    if ($InvalidFinalPackagePart -eq 'capability') { $capability.descriptorValues = @() }
    $capability | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $capabilityDir 'summary.json') -Encoding utf8

    $correlationDir = Join-Path $runDir 'correlation'
    New-Item -ItemType Directory -Path $correlationDir -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $correlationDir 'paired-events.jsonl') -Value '{"type":"source-object-arrived"}' -Encoding utf8
    $correlation = [ordered]@{
      schemaVersion = 'hv-14-correlation/v1'
      pairedRequestCount = 35
      rejectedObjectCount = 0
      fallbackCorrelationCount = 0
      originalsJpegCount = 0
      rawTruthPreserved = $true
      evidenceFiles = @('paired-events.jsonl')
    }
    if ($InvalidFinalPackagePart -eq 'correlation') { $correlation.originalsJpegCount = 1 }
    $correlation | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $correlationDir 'summary.json') -Encoding utf8

    $qualityDir = Join-Path $runDir 'quality'
    New-Item -ItemType Directory -Path $qualityDir -Force | Out-Null
    $qualityScenes = @()
    foreach ($sceneKind in @('bright', 'dark', 'portrait', 'high-contrast')) {
      $assets = @()
      foreach ($route in @('embedded-jpeg', 'camera-paired-jpeg', 'windows-shell-thumbnail')) {
        $fileName = "$sceneKind-$route.jpg"
        [IO.File]::WriteAllBytes((Join-Path $qualityDir $fileName), [byte[]](0xFF, 0xD8, 0xFF, 0xD9))
        $assets += [ordered]@{ route = $route; file = $fileName }
      }
      $qualityScenes += [ordered]@{ kind = $sceneKind; assets = $assets }
    }
    $quality = [ordered]@{
      schemaVersion = 'hv-14-quality/v1'
      humanApproved = $true
      reviewer = 'HV-14 operator'
      scenes = $qualityScenes
    }
    if ($InvalidFinalPackagePart -eq 'quality') { $quality.humanApproved = $false }
    $quality | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $qualityDir 'manifest.json') -Encoding utf8

    $aggregateDir = Join-Path $runDir 'aggregate'
    New-Item -ItemType Directory -Path $aggregateDir -Force | Out-Null
    $routeAggregates = @(
      [ordered]@{ route = 'embedded-jpeg'; attemptCount = 35; successCount = 35; p50Micros = 12000; p95Micros = 18000; maxMicros = 22000; failureModes = [pscustomobject]@{} },
      [ordered]@{ route = 'camera-paired-jpeg'; attemptCount = 35; successCount = 34; p50Micros = 14000; p95Micros = 24000; maxMicros = 32000; failureModes = [pscustomobject]@{ timeout = 1 }; arrivalOrderDistribution = [pscustomobject]@{ jpegFirst = 12; rawFirst = 23 } },
      [ordered]@{ route = 'windows-shell-thumbnail'; attemptCount = 35; successCount = 35; p50Micros = 20000; p95Micros = 30000; maxMicros = 41000; failureModes = [pscustomobject]@{} }
    )
    if ($InvalidFinalPackagePart -eq 'aggregate') { $routeAggregates[1].arrivalOrderDistribution = $null }
    [ordered]@{
      schemaVersion = 'hv-14-aggregate/v1'
      randomizationSeed = 42
      blockCount = 35
      routes = $routeAggregates
    } | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $aggregateDir 'summary.json') -Encoding utf8

    $decisionLines = @(
      '# HV-14 Decision',
      '',
      '- Decision: primary',
      '- Selected route: embedded-jpeg',
      '- Baseline improvement: p95 12ms faster than incumbent',
      '- Rejected routes: camera-paired-jpeg due to one timeout',
      '- Story 7.4 constraint: keep preset rendering after source admission'
    )
    if ($InvalidFinalPackagePart -eq 'decision') { $decisionLines = @('# Decision', '- Decision: primary') }
    Set-Content -LiteralPath (Join-Path $runDir 'decision.md') -Value $decisionLines -Encoding utf8
  }

  $arguments = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $gateScript, '-SessionEvidenceDir', $sessionEvidenceDir)
  if ($FinalPackageGate.IsPresent) {
    $arguments += '-FinalPackageGate'
  }
  $output = & $powerShellExe @arguments 2>&1
  $exitCode = $LASTEXITCODE
  $rendered = $output -join [Environment]::NewLine

  if ($exitCode -ne $ExpectedExitCode) {
    throw "[$Name] expected exit code $ExpectedExitCode, got $exitCode.`n$rendered"
  }
  if ($rendered -notmatch $ExpectedText) {
    throw "[$Name] output did not contain '$ExpectedText'.`n$rendered"
  }
  Write-Output "PASS: $Name"
}

try {
  New-Item -ItemType Directory -Path $fixtureRoot -Force | Out-Null

  $valid = @(New-ValidSamples)
  Invoke-GateCase -Name 'valid' -Samples $valid -ExpectedExitCode 0 -ExpectedText 'PASS \(telemetry completeness only\)'
  Invoke-GateCase -Name 'missing-final-package' -Samples $valid -ExpectedExitCode 1 -ExpectedText 'environment.md' -FinalPackageGate
  Invoke-GateCase -Name 'complete-final-package' -Samples $valid -ExpectedExitCode 0 -ExpectedText 'PASS \(final package gate\)' -FinalPackageGate -CompleteFinalPackage
  Invoke-GateCase -Name 'unconfirmed-environment' -Samples $valid -ExpectedExitCode 1 -ExpectedText 'camera firmware' -FinalPackageGate -CompleteFinalPackage -InvalidFinalPackagePart environment
  Invoke-GateCase -Name 'empty-capability-descriptor' -Samples $valid -ExpectedExitCode 1 -ExpectedText 'descriptorValues' -FinalPackageGate -CompleteFinalPackage -InvalidFinalPackagePart capability
  Invoke-GateCase -Name 'jpeg-in-originals' -Samples $valid -ExpectedExitCode 1 -ExpectedText 'originals' -FinalPackageGate -CompleteFinalPackage -InvalidFinalPackagePart correlation
  Invoke-GateCase -Name 'quality-not-approved' -Samples $valid -ExpectedExitCode 1 -ExpectedText 'quality corpus' -FinalPackageGate -CompleteFinalPackage -InvalidFinalPackagePart quality
  Invoke-GateCase -Name 'missing-arrival-distribution' -Samples $valid -ExpectedExitCode 1 -ExpectedText 'camera-paired-jpeg' -FinalPackageGate -CompleteFinalPackage -InvalidFinalPackagePart aggregate
  Invoke-GateCase -Name 'vague-decision' -Samples $valid -ExpectedExitCode 1 -ExpectedText 'Selected route' -FinalPackageGate -CompleteFinalPackage -InvalidFinalPackagePart decision

  $missingRoute = @($valid | Where-Object {
    -not ($_.candidate.requestId -eq 'request-34' -and $_.candidate.route -eq 'windows-shell-thumbnail')
  })
  Invoke-GateCase -Name 'missing-route' -Samples $missingRoute -ExpectedExitCode 1 -ExpectedText 'windows-shell-thumbnail'

  $brokenMatrix = @(New-ValidSamples)
  $movedRow = $brokenMatrix | Where-Object {
    $_.candidate.requestId -eq 'request-34' -and $_.candidate.route -eq 'windows-shell-thumbnail'
  } | Select-Object -First 1
  $movedRow.candidate.requestId = 'request-extra'
  $movedRow.candidate.captureId = 'capture-extra'
  Invoke-GateCase -Name 'broken-request-matrix' -Samples $brokenMatrix -ExpectedExitCode 1 -ExpectedText 'request'

  $wrongWarmUp = @(New-ValidSamples)
  $wrongWarmUpRow = $wrongWarmUp | Where-Object {
    $_.candidate.requestId -eq 'request-00' -and $_.candidate.route -eq 'embedded-jpeg'
  } | Select-Object -First 1
  $wrongWarmUpRow.isWarmUp = $false
  Invoke-GateCase -Name 'wrong-warm-up' -Samples $wrongWarmUp -ExpectedExitCode 1 -ExpectedText 'warm-up'

  $missingRequiredField = @(New-ValidSamples)
  $missingRequiredField[0].candidate.captureId = ''
  Invoke-GateCase -Name 'missing-required-field' -Samples $missingRequiredField -ExpectedExitCode 1 -ExpectedText 'candidate.captureId'

  $mixedSessions = @(New-ValidSamples)
  $mixedSessions[0].candidate.sessionId = 'session-other'
  Invoke-GateCase -Name 'mixed-sessions' -Samples $mixedSessions -ExpectedExitCode 1 -ExpectedText 'sessionId'

  Invoke-GateCase -Name 'malformed-jsonl' -Samples $valid -ExpectedExitCode 1 -ExpectedText 'JSONL' -AppendRawLine '{"broken":'

  Write-Output 'HV-14 completeness gate synthetic tests passed: structured final-package boundaries plus telemetry defect fixtures.'
}
finally {
  $resolvedFixtureRoot = [IO.Path]::GetFullPath($fixtureRoot)
  $resolvedTempRoot = [IO.Path]::GetFullPath($tempRoot)
  if ($resolvedFixtureRoot -ne $resolvedTempRoot -and $resolvedFixtureRoot.StartsWith($resolvedTempRoot, [StringComparison]::OrdinalIgnoreCase)) {
    if (Test-Path -LiteralPath $resolvedFixtureRoot) {
      Remove-Item -LiteralPath $resolvedFixtureRoot -Recurse -Force
    }
  }
  else {
    Write-Warning "Refused to remove unexpected fixture path: $resolvedFixtureRoot"
  }
}
