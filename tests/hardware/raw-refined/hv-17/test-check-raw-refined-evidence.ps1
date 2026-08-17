<#
.SYNOPSIS
  HV-17 게이트 자체를 합성 fixture로 검증한다.

.DESCRIPTION
  게이트가 "언제나 PASS"면 아무것도 지키지 못한다. 통과해야 할 회차 하나와
  반드시 실패해야 할 결함 회차들을 만들어 경계를 고정한다.

  **AC 5의 독립성도 여기서 검증한다.** HV-17A 쪽 결함은 HV-17B verdict를 건드리지 않고,
  그 반대도 마찬가지여야 한다. 한쪽 gate가 다른 쪽 결과를 물려받으면 그것은
  두 gate가 아니라 하나다.

.EXAMPLE
  ./test-check-raw-refined-evidence.ps1
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$gate = Join-Path $PSScriptRoot 'check-raw-refined-evidence.ps1'
$root = Join-Path ([System.IO.Path]::GetTempPath()) ("hv17-gate-" + [guid]::NewGuid().ToString('N'))
$failures = 0

function Write-JsonLines([string]$path, [object[]]$rows) {
  New-Item -ItemType Directory -Path (Split-Path $path -Parent) -Force | Out-Null
  $lines = $rows | ForEach-Object { $_ | ConvertTo-Json -Depth 8 -Compress }
  Set-Content -LiteralPath $path -Value $lines -Encoding utf8
}

function Write-JsonFile([string]$path, [object]$value) {
  New-Item -ItemType Directory -Path (Split-Path $path -Parent) -Force | Out-Null
  Set-Content -LiteralPath $path -Value ($value | ConvertTo-Json -Depth 8) -Encoding utf8
}

function New-Generation([string]$captureId, [int]$seq, [string]$tier, [int]$width, [int]$height) {
  $quality = if ($tier -eq 'rawRefinedDisplay') { 'high' } else { 'fast' }

  return @{
    outcome    = 'committed'
    generation = @{
      generationId    = "req-$captureId-$('{0:d6}' -f $seq)"
      generationSeq   = $seq
      sessionId       = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
      requestId       = "req-$captureId"
      captureId       = $captureId
      viewerEpoch     = 3
      tier            = $tier
      sourceWidthPx   = $width
      sourceHeightPx  = $height
      proxyProvenance = @{
        presetId      = 'preset_soft-glow'
        presetVersion = '2026.08.01'
        renderQuality = $quality
      }
    }
  }
}

function New-PresentRow([string]$captureId, [int]$seq, [string]$tier) {
  $isRefined = $tier -eq 'rawRefinedDisplay'

  return @{
    generationId            = "req-$captureId-$('{0:d6}' -f $seq)"
    tier                    = $tier
    outcome                 = 'presented'
    isTrustedInput          = $true
    viewerWindowEventsAfterInput = 0
    qualifyingLatencyMicros = if ($isRefined) { $null } else { 3200000 }
    schedulerPriority       = if ($isRefined) { 'P1' } else { 'P0' }
    schedulerDeadlineMicros = 5000000
    rejectReason            = $null
  }
}

function New-HealthyRun([string]$name) {
  $runRoot = Join-Path $root $name
  New-Item -ItemType Directory -Path $runRoot -Force | Out-Null
  Set-Content -LiteralPath (Join-Path $runRoot 'environment.md') -Value '# environment' -Encoding utf8

  $generations = @()
  $presentRows = @()

  # 촬영 5회, 각 촬영이 proxy → refined 두 generation을 만든다.
  for ($index = 1; $index -le 5; $index++) {
    $captureId = "cap-$index"
    $proxySeq = $index * 2 - 1
    $refinedSeq = $index * 2
    $generations += New-Generation $captureId $proxySeq 'displayFitPresetProxy' 1620 1080
    $generations += New-Generation $captureId $refinedSeq 'rawRefinedDisplay' 1620 1080
    $presentRows += New-PresentRow $captureId $proxySeq 'displayFitPresetProxy'
    $presentRows += New-PresentRow $captureId $refinedSeq 'rawRefinedDisplay'
  }

  Write-JsonLines (Join-Path $runRoot 'session-evidence/display/generations.jsonl') $generations
  Write-JsonLines (Join-Path $runRoot 'session-evidence/diagnostics/viewer-present.jsonl') $presentRows

  $logDir = Join-Path $runRoot 'logs'
  New-Item -ItemType Directory -Path $logDir -Force | Out-Null
  $cancelRounds = @()

  foreach ($round in @('delete', 'session-replaced', 'viewer-epoch-changed', 'newer-capture')) {
    $logRelative = "logs/taskkill-$round.log"
    Set-Content -LiteralPath (Join-Path $runRoot $logRelative) -Value 'SUCCESS: ...' -Encoding utf8
    $cancelRounds += @{
      round                   = $round
      cancelRequestedAtMicros = 1000
      cancelCompletedAtMicros = 42000
      killOutcome             = 'exit=0'
      directKillFallback      = $false
      cancelOrphanCount       = 0
      taskkillLogPath         = $logRelative
      finalRenderCompleted    = $true
    }
  }

  Write-JsonLines (Join-Path $runRoot 'scheduler/cancel-rounds.jsonl') $cancelRounds
  Write-JsonFile (Join-Path $runRoot 'scheduler/burst.json') @{
    captureCount            = 5
    p0DroppedCount          = 0
    queueWaitMicros         = @{ p50 = 120000; p95 = 480000; max = 900000 }
    queueWaitShareOfEndToEnd = 0.06
  }

  New-Item -ItemType Directory -Path (Join-Path $runRoot 'transition') -Force | Out-Null
  Set-Content -LiteralPath (Join-Path $runRoot 'transition/swap.mp4') -Value 'recording' -Encoding utf8
  Write-JsonFile (Join-Path $runRoot 'transition/zero-report.json') @{
    blank                  = 0
    spinner                = 0
    previousOrOtherCapture = 0
    otherPreset            = 0
    cropJump               = 0
    scaleJump              = 0
    tierDowngrade          = 0
    staleOverwrite         = 0
    screenRecordingPath    = 'transition/swap.mp4'
  }

  $perSample = @()
  foreach ($chart in @('chart-1', 'chart-2', 'chart-3')) {
    foreach ($preset in @('preset_daylight', 'preset_mono-pop', 'preset_soft-glow')) {
      $perSample += @{
        sampleId     = "$chart/$preset"
        presetId     = $preset
        proxyMtf50   = 0.20
        refinedMtf50 = 0.25
        ratio        = 1.25
      }
    }
  }

  Write-JsonFile (Join-Path $runRoot 'tier-justification/verdict.json') @{
    detail = @{ verdict = 'justified'; ratio = 1.25; regressions = @(); perSample = $perSample }
    look   = @{ verdict = 'same-look'; medianDeltaE = 1.1; p95DeltaE = 3.4 }
  }

  $trials = @()
  foreach ($observer in @('observer-1', 'observer-2', 'observer-3')) {
    for ($index = 1; $index -le 10; $index++) {
      $trials += [pscustomobject]@{ observer = $observer; trial = $index; condition = 'swap'; reportedChange = 'false' }
    }
    for ($index = 11; $index -le 20; $index++) {
      $trials += [pscustomobject]@{ observer = $observer; trial = $index; condition = 'control'; reportedChange = 'false' }
    }
  }

  New-Item -ItemType Directory -Path (Join-Path $runRoot 'detection-trial') -Force | Out-Null
  $trials | Export-Csv -LiteralPath (Join-Path $runRoot 'detection-trial/trials.csv') -NoTypeInformation -Encoding utf8

  return $runRoot
}

function Invoke-Gate([string]$runRoot) {
  $outFile = Join-Path $runRoot 'gate.json'
  & $gate -RunRoot $runRoot -OutFile $outFile *> $null
  $exitCode = $LASTEXITCODE

  return @{
    exitCode = $exitCode
    result   = (Get-Content -LiteralPath $outFile -Raw -Encoding utf8 | ConvertFrom-Json)
  }
}

function Assert-Case([string]$label, [int]$expectedExit, [string]$expectedA, [string]$expectedB, [scriptblock]$mutate) {
  $runRoot = New-HealthyRun ([guid]::NewGuid().ToString('N'))
  & $mutate $runRoot
  $outcome = Invoke-Gate $runRoot

  $ok = $true
  if ($outcome.exitCode -ne $expectedExit) {
    Write-Host "FAIL [$label]: expected exit $expectedExit, got $($outcome.exitCode)"
    $ok = $false
  }
  if ($outcome.result.hv17a.verdict -ne $expectedA) {
    Write-Host "FAIL [$label]: expected HV-17A '$expectedA', got '$($outcome.result.hv17a.verdict)'"
    $ok = $false
  }
  if ($outcome.result.hv17b.verdict -ne $expectedB) {
    Write-Host "FAIL [$label]: expected HV-17B '$expectedB', got '$($outcome.result.hv17b.verdict)'"
    $ok = $false
  }

  if ($ok) {
    Write-Host "ok   [$label]"
  }
  else {
    $script:failures++
  }
}

Assert-Case 'healthy run passes both gates' 0 'automated-pass' 'automated-pass' { param($runRoot) }

# --- 승인된 Partial 종료: 안전 gate는 통과하고 가치 없는 tier는 꺼 둔다 --------
$partialRoot = New-HealthyRun ([guid]::NewGuid().ToString('N'))
$generationPath = Join-Path $partialRoot 'session-evidence/display/generations.jsonl'
$generationRows = @(Get-Content -LiteralPath $generationPath -Encoding utf8 | ForEach-Object { $_ | ConvertFrom-Json })
Write-JsonLines $generationPath @($generationRows | Where-Object { $_.generation.tier -ne 'rawRefinedDisplay' })
$presentPath = Join-Path $partialRoot 'session-evidence/diagnostics/viewer-present.jsonl'
$presentRows = @(Get-Content -LiteralPath $presentPath -Encoding utf8 | ForEach-Object { $_ | ConvertFrom-Json })
Write-JsonLines $presentPath @($presentRows | Where-Object { $_.tier -ne 'rawRefinedDisplay' })
$tierPath = Join-Path $partialRoot 'tier-justification/verdict.json'
$tierVerdict = Get-Content -LiteralPath $tierPath -Raw -Encoding utf8 | ConvertFrom-Json
$tierVerdict.detail.verdict = 'tier-not-justified'
$tierVerdict | Add-Member -NotePropertyName corpus -NotePropertyValue ([pscustomobject]@{
    complete = $true; uniqueSamples = 3; uniquePresets = 3; uniquePairs = 9
  }) -Force
$tierVerdict.detail | Add-Member -NotePropertyName measuredPairs -NotePropertyValue 9 -Force
$tierVerdict.detail | Add-Member -NotePropertyName unmeasuredPairs -NotePropertyValue @() -Force
Write-JsonFile $tierPath $tierVerdict
Write-JsonFile (Join-Path $partialRoot 'partial-decision.json') @{
  schemaVersion          = 'hv17-partial-decision/v1'
  decision               = 'Partial'
  implementationComplete = $true
  tierEvidenceRoot       = '.'
  laneDefaultEnabled     = $false
  fr010OpenItem           = 'Story 7.10 / HV-18D'
  approvedBy             = 'Noah Lee'
  approvedAt             = '2026-08-17T00:00:00+09:00'
}
$partialOutcome = Invoke-Gate $partialRoot
if ($partialOutcome.exitCode -ne 0 -or
    $partialOutcome.result.hv17a.verdict -ne 'automated-pass' -or
    $partialOutcome.result.hv17b.verdict -ne 'No-Go' -or
    $partialOutcome.result.productDecision.verdict -ne 'Partial') {
  Write-Host "FAIL [approved Partial closes the gate]: expected exit 0 / A pass / B No-Go / Partial"
  $script:failures++
}
else {
  Write-Host 'ok   [approved Partial closes the gate]'
}

# --- HV-17A 결함: HV-17B는 그대로 통과해야 한다 (AC 5 독립성) -------------------
Assert-Case 'P0 dropout fails only HV-17A' 1 'No-Go' 'automated-pass' {
  param($runRoot)
  Write-JsonFile (Join-Path $runRoot 'scheduler/burst.json') @{
    captureCount            = 5
    p0DroppedCount          = 1
    queueWaitMicros         = @{ p50 = 1; p95 = 2; max = 3 }
    queueWaitShareOfEndToEnd = 0.01
  }
}

Assert-Case 'a surviving orphan fails only HV-17A' 1 'No-Go' 'automated-pass' {
  param($runRoot)
  $path = Join-Path $runRoot 'scheduler/cancel-rounds.jsonl'
  $rows = @(Get-Content -LiteralPath $path -Encoding utf8 | ForEach-Object { $_ | ConvertFrom-Json })
  $rows[0].cancelOrphanCount = 2
  Write-JsonLines $path $rows
}

Assert-Case 'a cancelled final render fails only HV-17A' 1 'No-Go' 'automated-pass' {
  param($runRoot)
  $path = Join-Path $runRoot 'scheduler/cancel-rounds.jsonl'
  $rows = @(Get-Content -LiteralPath $path -Encoding utf8 | ForEach-Object { $_ | ConvertFrom-Json })
  $rows[0].finalRenderCompleted = $false
  Write-JsonLines $path $rows
}

Assert-Case 'a missing cancel round fails only HV-17A' 1 'No-Go' 'automated-pass' {
  param($runRoot)
  $path = Join-Path $runRoot 'scheduler/cancel-rounds.jsonl'
  $rows = @(Get-Content -LiteralPath $path -Encoding utf8 | ForEach-Object { $_ | ConvertFrom-Json })
  Write-JsonLines $path @($rows | Where-Object { $_.round -ne 'newer-capture' })
}

Assert-Case 'a missing queue-wait share fails only HV-17A' 1 'No-Go' 'automated-pass' {
  param($runRoot)
  Write-JsonFile (Join-Path $runRoot 'scheduler/burst.json') @{
    captureCount    = 5
    p0DroppedCount  = 0
    queueWaitMicros = @{ p50 = 1; p95 = 2; max = 3 }
  }
}

# --- HV-17B 결함: HV-17A는 그대로 통과해야 한다 (AC 5 독립성) -------------------
Assert-Case 'a size mismatch between the two tiers fails only HV-17B' 1 'automated-pass' 'No-Go' {
  param($runRoot)
  $path = Join-Path $runRoot 'session-evidence/display/generations.jsonl'
  $rows = @(Get-Content -LiteralPath $path -Encoding utf8 | ForEach-Object { $_ | ConvertFrom-Json })
  $refined = @($rows | Where-Object { $_.generation.tier -eq 'rawRefinedDisplay' })[0]
  $refined.generation.sourceWidthPx = 1619
  Write-JsonLines $path $rows
}

Assert-Case 'a refined frame without its proxy fails only HV-17B' 1 'automated-pass' 'No-Go' {
  param($runRoot)
  $path = Join-Path $runRoot 'session-evidence/display/generations.jsonl'
  $rows = @(Get-Content -LiteralPath $path -Encoding utf8 | ForEach-Object { $_ | ConvertFrom-Json })
  $kept = @($rows | Where-Object {
      -not ($_.generation.captureId -eq 'cap-1' -and $_.generation.tier -eq 'displayFitPresetProxy')
    })
  Write-JsonLines $path $kept
}

Assert-Case 'a missing terminal row fails only HV-17B' 1 'automated-pass' 'No-Go' {
  param($runRoot)
  $path = Join-Path $runRoot 'session-evidence/diagnostics/viewer-present.jsonl'
  $rows = @(Get-Content -LiteralPath $path -Encoding utf8 | ForEach-Object { $_ | ConvertFrom-Json })
  Write-JsonLines $path @($rows | Select-Object -Skip 1)
}

Assert-Case 'a refined row claiming a KPI endpoint fails only HV-17B' 1 'automated-pass' 'No-Go' {
  param($runRoot)
  $path = Join-Path $runRoot 'session-evidence/diagnostics/viewer-present.jsonl'
  $rows = @(Get-Content -LiteralPath $path -Encoding utf8 | ForEach-Object { $_ | ConvertFrom-Json })
  $refined = @($rows | Where-Object { $_.tier -eq 'rawRefinedDisplay' })[0]
  $refined.qualifyingLatencyMicros = 8284000
  Write-JsonLines $path $rows
}

Assert-Case 'a non-zero transition defect fails only HV-17B' 1 'automated-pass' 'No-Go' {
  param($runRoot)
  Write-JsonFile (Join-Path $runRoot 'transition/zero-report.json') @{
    blank = 0; spinner = 1; previousOrOtherCapture = 0; otherPreset = 0
    cropJump = 0; scaleJump = 0; tierDowngrade = 0; staleOverwrite = 0
    screenRecordingPath = 'transition/swap.mp4'
  }
}

Assert-Case 'an unmeasured detail axis fails only HV-17B' 1 'automated-pass' 'No-Go' {
  param($runRoot)
  $path = Join-Path $runRoot 'tier-justification/verdict.json'
  $verdict = Get-Content -LiteralPath $path -Raw -Encoding utf8 | ConvertFrom-Json
  $verdict.detail.verdict = 'not-measured'
  Write-JsonFile $path $verdict
}

Assert-Case 'a measured but unjustified detail tier fails only HV-17B' 1 'automated-pass' 'No-Go' {
  param($runRoot)
  $path = Join-Path $runRoot 'tier-justification/verdict.json'
  $verdict = Get-Content -LiteralPath $path -Raw -Encoding utf8 | ConvertFrom-Json
  $verdict.detail.verdict = 'tier-not-justified'
  Write-JsonFile $path $verdict
}

Assert-Case 'a look drift fails only HV-17B' 1 'automated-pass' 'No-Go' {
  param($runRoot)
  $path = Join-Path $runRoot 'tier-justification/verdict.json'
  $verdict = Get-Content -LiteralPath $path -Raw -Encoding utf8 | ConvertFrom-Json
  $verdict.look.verdict = 'refined-look-drift'
  Write-JsonFile $path $verdict
}

Assert-Case 'a short slanted-edge corpus fails only HV-17B' 1 'automated-pass' 'No-Go' {
  param($runRoot)
  $path = Join-Path $runRoot 'tier-justification/verdict.json'
  $verdict = Get-Content -LiteralPath $path -Raw -Encoding utf8 | ConvertFrom-Json
  $verdict.detail.perSample = @($verdict.detail.perSample | Select-Object -First 8)
  Write-JsonFile $path $verdict
}

Assert-Case 'a detection test without a control group fails only HV-17B' 1 'automated-pass' 'No-Go' {
  param($runRoot)
  $path = Join-Path $runRoot 'detection-trial/trials.csv'
  $rows = @(Import-Csv -LiteralPath $path | Where-Object { $_.condition -ne 'control' })
  $rows | Export-Csv -LiteralPath $path -NoTypeInformation -Encoding utf8
}

Assert-Case 'observers who actually saw the swap fail only HV-17B' 1 'automated-pass' 'No-Go' {
  param($runRoot)
  $path = Join-Path $runRoot 'detection-trial/trials.csv'
  $rows = @(Import-Csv -LiteralPath $path)
  foreach ($row in $rows) {
    if ($row.observer -eq 'observer-2' -and $row.condition -eq 'swap') {
      $row.reportedChange = 'true'
    }
  }
  $rows | Export-Csv -LiteralPath $path -NoTypeInformation -Encoding utf8
}

Assert-Case 'only two observers fail only HV-17B' 1 'automated-pass' 'No-Go' {
  param($runRoot)
  $path = Join-Path $runRoot 'detection-trial/trials.csv'
  $rows = @(Import-Csv -LiteralPath $path | Where-Object { $_.observer -ne 'observer-3' })
  $rows | Export-Csv -LiteralPath $path -NoTypeInformation -Encoding utf8
}

# --- 환경 기록은 두 gate 모두의 재현 조건이다 ---------------------------------
Assert-Case 'a missing environment.md fails both gates' 1 'No-Go' 'No-Go' {
  param($runRoot)
  Remove-Item -LiteralPath (Join-Path $runRoot 'environment.md') -Force
}

Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue

if ($failures -gt 0) {
  Write-Error "$failures gate self-test case(s) failed."
  exit 1
}

Write-Host 'All HV-17 gate self-test cases passed.'
exit 0
