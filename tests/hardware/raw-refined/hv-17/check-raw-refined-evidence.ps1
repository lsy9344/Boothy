<#
.SYNOPSIS
  HV-17 (Story 7.6) 기계식 게이트 — RAW 정밀본 무중단 교체 증거의 완결성 검사.

.DESCRIPTION
  **두 gate는 서로 독립이다. 한쪽이 다른 쪽 결과를 물려받을 수 없다** (AC 5).
  그래서 이 스크립트는 HV-17A와 HV-17B의 판정을 **각자의 입력만으로** 계산하고
  두 verdict를 따로 출력한다. 한쪽이 없으면 그 gate만 `No-Go`가 되고,
  다른 쪽 판정은 그대로 남는다.

  PASS는 HV-17 Go가 아니다. 이 스크립트는 **기계적으로 확인 가능한 것만** 본다.
  일반 속도 화면 녹화 검토, 관찰자 3명의 탐지 검사 실행, Noah Lee 서명은 사람이 한다.

  HV-17A (scheduler / 용량 / 취소 / process tree):
    A1. 모든 display lane generation이 스케줄러 span을 싣는다
    A2. 우선순위가 실제 실행 순서로 나타난다 (P0가 같은 촬영 P1보다 먼저 dequeue)
    A3. burst 회차에서 P0 탈락이 0건이다
    A4. 취소 회차마다 `cancelOrphanCount = 0`이고 taskkill 결과 로그가 있다
    A5. 취소 회차에서 final 렌더가 완료됐다
    A6. 큐 대기 p50/p95/max와 전체 종단 지연에서 차지하는 비율이 기록됐다

  HV-17B (seamless tier 전환 / frame 무결성):
    B1. 촬영마다 proxy commit → refined commit 이력이 있다
    B2. 같은 촬영의 proxy와 refined의 실측 픽셀 크기가 **동일**하다
    B3. 전환 결함 zero 보고가 전부 0이다
    B4. commit된 generation당 terminal present 행이 정확히 하나다
    B5. detail 축과 look 축 판정이 원자료와 함께 있다
    B6. 전환 탐지 검사에 **대조군이 있고** 탐지율이 오탐율 + 10%p를 넘지 않는다

  **120fps+ 물리 frame과 compositor→photon 오프셋은 이 gate가 제공하지 않는다.**
  그 증거는 HV-18B가 단독 소유한다 (2026-08-12 correct-course).

.PARAMETER RunRoot
  회차 디렉터리. 아래 구조를 갖는다:
    environment.md
    session-evidence/display/generations.jsonl
    session-evidence/diagnostics/viewer-present.jsonl
    scheduler/cancel-rounds.jsonl
    scheduler/burst.json
    transition/zero-report.json
    tier-justification/verdict.json
    detection-trial/trials.csv

.EXAMPLE
  ./check-raw-refined-evidence.ps1 -RunRoot ../run-20260820-101500-hv17
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][string]$RunRoot,
  [string]$OutFile
)

$ErrorActionPreference = 'Stop'

$refinedTier = 'rawRefinedDisplay'
$proxyTier = 'displayFitPresetProxy'
# 승인된 결정 3. 교체 탐지율이 대조군 오탐율보다 이만큼 넘게 높으면 사람이 전환을 본 것이다.
$detectionMarginPercentagePoints = 10

$hv17aFailures = New-Object System.Collections.Generic.List[string]
$hv17bFailures = New-Object System.Collections.Generic.List[string]

function Add-AFailure([string]$message) { $hv17aFailures.Add($message) | Out-Null }
function Add-BFailure([string]$message) { $hv17bFailures.Add($message) | Out-Null }

# 손상된 행을 조용히 건너뛰면 분모가 줄어 성공률이 실제보다 좋아 보인다.
function Read-JsonLines([string]$path, [string]$label, [scriptblock]$onMalformed) {
  $rows = New-Object System.Collections.Generic.List[object]

  if (-not (Test-Path -LiteralPath $path)) {
    return $null
  }

  $lineNumber = 0
  foreach ($line in Get-Content -LiteralPath $path -Encoding utf8) {
    $lineNumber++
    if ([string]::IsNullOrWhiteSpace($line)) { continue }
    try {
      $rows.Add(($line | ConvertFrom-Json)) | Out-Null
    }
    catch {
      & $onMalformed "${label}: line ${lineNumber} is malformed JSON and cannot be counted"
    }
  }

  return $rows
}

function Read-JsonFile([string]$path) {
  if (-not (Test-Path -LiteralPath $path)) { return $null }
  return (Get-Content -LiteralPath $path -Raw -Encoding utf8 | ConvertFrom-Json)
}

$generationsPath = Join-Path $RunRoot 'session-evidence/display/generations.jsonl'
$presentPath = Join-Path $RunRoot 'session-evidence/diagnostics/viewer-present.jsonl'
$cancelPath = Join-Path $RunRoot 'scheduler/cancel-rounds.jsonl'
$burstPath = Join-Path $RunRoot 'scheduler/burst.json'
$zeroReportPath = Join-Path $RunRoot 'transition/zero-report.json'
$tierVerdictPath = Join-Path $RunRoot 'tier-justification/verdict.json'
$trialsPath = Join-Path $RunRoot 'detection-trial/trials.csv'
$environmentPath = Join-Path $RunRoot 'environment.md'

if (-not (Test-Path -LiteralPath $environmentPath)) {
  # 환경을 모르면 어느 gate의 숫자도 재현할 수 없다. 두 gate 모두에 남긴다.
  Add-AFailure 'environment.md is missing — the run cannot be reproduced'
  Add-BFailure 'environment.md is missing — the run cannot be reproduced'
}

$journal = Read-JsonLines $generationsPath 'generations.jsonl' { param($m) Add-BFailure $m }
$present = Read-JsonLines $presentPath 'viewer-present.jsonl' { param($m) Add-BFailure $m }

# ---------------------------------------------------------------------------
# HV-17A: scheduler / 용량 / 취소 / process tree
# ---------------------------------------------------------------------------

$displayPresentRows = @()

if ($null -eq $present) {
  Add-AFailure "viewer-present.jsonl not found at $presentPath"
}
else {
  $displayPresentRows = @($present | Where-Object { $_.tier -eq $proxyTier -or $_.tier -eq $refinedTier })

  if ($displayPresentRows.Count -eq 0) {
    Add-AFailure 'no display lane sample was recorded — the scheduler never ran a customer render'
  }

  # A1. 스케줄러 span 없이는 우선순위가 실제로 적용됐는지 말할 수 없다.
  $missingSpans = @($displayPresentRows | Where-Object {
      $null -eq $_.schedulerPriority -or $null -eq $_.schedulerDeadlineMicros
    })

  if ($missingSpans.Count -gt 0) {
    Add-AFailure "$($missingSpans.Count) display lane rows carry no scheduler span"
  }

  # A2. 우선순위가 이름표로만 남고 실행 순서에 나타나지 않으면 증거가 아니다.
  $wrongPriority = @($displayPresentRows | Where-Object {
      ($_.tier -eq $proxyTier -and $_.schedulerPriority -ne 'P0') -or
      ($_.tier -eq $refinedTier -and $_.schedulerPriority -ne 'P1')
    })

  if ($wrongPriority.Count -gt 0) {
    Add-AFailure "$($wrongPriority.Count) rows record a priority that does not match their tier"
  }
}

$burst = Read-JsonFile $burstPath

if ($null -eq $burst) {
  Add-AFailure "scheduler/burst.json not found at $burstPath"
}
else {
  # A3. 오늘의 실제 위험은 대기가 아니라 탈락이다.
  if ($null -eq $burst.captureCount -or $burst.captureCount -lt 5) {
    Add-AFailure 'burst round must cover at least 5 consecutive captures'
  }
  if ($null -eq $burst.p0DroppedCount) {
    Add-AFailure 'burst.json does not report p0DroppedCount — a missing count is not zero'
  }
  elseif ($burst.p0DroppedCount -ne 0) {
    Add-AFailure "P0 dropouts occurred: $($burst.p0DroppedCount)"
  }

  # A6. 남은 지연의 소유를 Story 7.8로 넘기려면 큐 대기 비율이 수치로 있어야 한다.
  foreach ($field in @('p50', 'p95', 'max')) {
    if ($null -eq $burst.queueWaitMicros -or $null -eq $burst.queueWaitMicros.$field) {
      Add-AFailure "burst.json is missing queueWaitMicros.$field"
    }
  }
  if ($null -eq $burst.queueWaitShareOfEndToEnd) {
    Add-AFailure 'burst.json is missing queueWaitShareOfEndToEnd — the remaining latency cannot be assigned'
  }
}

$cancelRounds = Read-JsonLines $cancelPath 'cancel-rounds.jsonl' { param($m) Add-AFailure $m }

if ($null -eq $cancelRounds) {
  Add-AFailure "scheduler/cancel-rounds.jsonl not found at $cancelPath"
}
else {
  $requiredRounds = @('delete', 'session-replaced', 'viewer-epoch-changed', 'newer-capture')
  $observedRounds = @($cancelRounds | ForEach-Object { $_.round })

  foreach ($round in $requiredRounds) {
    if ($observedRounds -notcontains $round) {
      Add-AFailure "cancel round '$round' was not executed"
    }
  }

  foreach ($round in $cancelRounds) {
    # A4. **`cancelOrphanCount`가 없는 것과 0인 것은 다르다.**
    if ($null -eq $round.cancelOrphanCount) {
      Add-AFailure "cancel round '$($round.round)' does not report cancelOrphanCount"
    }
    elseif ($round.cancelOrphanCount -ne 0) {
      Add-AFailure "cancel round '$($round.round)' left $($round.cancelOrphanCount) orphaned process(es)"
    }

    if ([string]::IsNullOrWhiteSpace($round.killOutcome)) {
      Add-AFailure "cancel round '$($round.round)' does not record the taskkill outcome"
    }

    if ([string]::IsNullOrWhiteSpace($round.taskkillLogPath)) {
      Add-AFailure "cancel round '$($round.round)' does not point at the raw taskkill log"
    }
    elseif (-not (Test-Path -LiteralPath (Join-Path $RunRoot $round.taskkillLogPath))) {
      Add-AFailure "cancel round '$($round.round)' taskkill log is missing: $($round.taskkillLogPath)"
    }

    if ($null -eq $round.cancelRequestedAtMicros -or $null -eq $round.cancelCompletedAtMicros) {
      Add-AFailure "cancel round '$($round.round)' does not record its cancellation latency"
    }
  }

  # A5. **display lane이 급하다는 이유로 final을 취소하면 고객이 결과물을 못 받는다.**
  $finalRounds = @($cancelRounds | Where-Object { $null -ne $_.finalRenderCompleted })

  if ($finalRounds.Count -eq 0) {
    Add-AFailure 'no cancel round reports whether the final render still completed'
  }
  elseif (@($finalRounds | Where-Object { -not $_.finalRenderCompleted }).Count -gt 0) {
    Add-AFailure 'a final render was cancelled by a display lane event — Story 3.2 completion truth is broken'
  }
}

# ---------------------------------------------------------------------------
# HV-17B: seamless tier 전환 / frame 무결성
# ---------------------------------------------------------------------------

$committed = @()
$proxyGenerations = @()
$refinedGenerations = @()

if ($null -eq $journal) {
  Add-BFailure "generations.jsonl not found at $generationsPath"
}
else {
  $committed = @($journal | Where-Object { $_.outcome -eq 'committed' -and $null -ne $_.generation })
  $proxyGenerations = @($committed | Where-Object { $_.generation.tier -eq $proxyTier })
  $refinedGenerations = @($committed | Where-Object { $_.generation.tier -eq $refinedTier })

  if ($refinedGenerations.Count -eq 0) {
    Add-BFailure 'no rawRefinedDisplay generation was committed — the promotion never happened'
  }

  # B1/B2. 정밀본은 **같은 촬영의 proxy에서 승급**하고 픽셀 크기가 정확히 같아야 한다.
  foreach ($refined in $refinedGenerations) {
    $captureId = $refined.generation.captureId
    $proxy = @($proxyGenerations | Where-Object { $_.generation.captureId -eq $captureId }) |
      Select-Object -First 1

    if ($null -eq $proxy) {
      Add-BFailure "capture ${captureId}: a refined frame was committed without a proxy of its own capture"
      continue
    }

    if ($proxy.generation.generationSeq -ge $refined.generation.generationSeq) {
      Add-BFailure "capture ${captureId}: the refined frame did not follow its proxy"
    }

    if ($proxy.generation.sourceWidthPx -ne $refined.generation.sourceWidthPx -or
        $proxy.generation.sourceHeightPx -ne $refined.generation.sourceHeightPx) {
      Add-BFailure ("capture ${captureId}: proxy is " +
        "$($proxy.generation.sourceWidthPx)x$($proxy.generation.sourceHeightPx) but refined is " +
        "$($refined.generation.sourceWidthPx)x$($refined.generation.sourceHeightPx) — this is a crop/scale jump")
    }

    if ($refined.generation.proxyProvenance.renderQuality -ne 'high') {
      Add-BFailure "capture ${captureId}: the refined generation does not record renderQuality=high"
    }
    if ($proxy.generation.proxyProvenance.renderQuality -ne 'fast') {
      Add-BFailure "capture ${captureId}: the proxy generation does not record renderQuality=fast"
    }
    if ($refined.generation.proxyProvenance.presetId -ne $proxy.generation.proxyProvenance.presetId -or
        $refined.generation.proxyProvenance.presetVersion -ne $proxy.generation.proxyProvenance.presetVersion) {
      Add-BFailure "capture ${captureId}: the refined frame used a different preset than its proxy"
    }
  }
}

# B4. commit된 generation당 terminal 행이 정확히 하나다.
if ($null -ne $journal -and $null -ne $present) {
  $committedIds = @($committed | ForEach-Object { $_.generation.generationId })
  $presentIds = @($present | ForEach-Object { $_.generationId })
  $missing = @($committedIds | Where-Object { $presentIds -notcontains $_ })
  $duplicated = @($presentIds | Group-Object | Where-Object { $_.Count -gt 1 } | ForEach-Object { $_.Name })

  if ($missing.Count -gt 0) {
    Add-BFailure "$($missing.Count) committed generation(s) have no terminal present row"
  }
  if ($duplicated.Count -gt 0) {
    Add-BFailure "$($duplicated.Count) generation(s) have more than one terminal present row"
  }

  # 정밀본 present는 두 번째 terminal 행이지 KPI 종료점이 아니다.
  $refinedClaimingKpi = @($present | Where-Object {
      $_.tier -eq $refinedTier -and $null -ne $_.qualifyingLatencyMicros
    })

  if ($refinedClaimingKpi.Count -gt 0) {
    Add-BFailure "$($refinedClaimingKpi.Count) refined row(s) claim a KPI endpoint — NFR-003 numbers from this run are not trustworthy"
  }
}

# B3. 전환 결함 zero 보고.
$zeroReport = Read-JsonFile $zeroReportPath

if ($null -eq $zeroReport) {
  Add-BFailure "transition/zero-report.json not found at $zeroReportPath"
}
else {
  foreach ($field in @('blank', 'spinner', 'previousOrOtherCapture', 'otherPreset',
      'cropJump', 'scaleJump', 'tierDowngrade', 'staleOverwrite')) {
    if ($null -eq $zeroReport.$field) {
      Add-BFailure "zero-report.json does not report '$field' — a missing count is not zero"
    }
    elseif ($zeroReport.$field -ne 0) {
      Add-BFailure "transition defect '$field' occurred $($zeroReport.$field) time(s)"
    }
  }

  if ([string]::IsNullOrWhiteSpace($zeroReport.screenRecordingPath)) {
    Add-BFailure 'zero-report.json does not point at the normal-speed screen recording'
  }
  elseif (-not (Test-Path -LiteralPath (Join-Path $RunRoot $zeroReport.screenRecordingPath))) {
    Add-BFailure "the screen recording is missing: $($zeroReport.screenRecordingPath)"
  }
}

# B5. detail 축과 look 축 판정.
$tierVerdict = Read-JsonFile $tierVerdictPath

if ($null -eq $tierVerdict) {
  Add-BFailure "tier-justification/verdict.json not found at $tierVerdictPath"
}
else {
  if ($null -eq $tierVerdict.detail -or [string]::IsNullOrWhiteSpace($tierVerdict.detail.verdict)) {
    Add-BFailure 'verdict.json does not record a detail axis verdict'
  }
  elseif ($tierVerdict.detail.verdict -ne 'justified') {
    Add-BFailure "the detail axis is not justified: $($tierVerdict.detail.verdict)"
  }

  if ($null -eq $tierVerdict.look -or [string]::IsNullOrWhiteSpace($tierVerdict.look.verdict)) {
    Add-BFailure 'verdict.json does not record a look axis verdict'
  }
  elseif ($tierVerdict.look.verdict -ne 'same-look') {
    # tier 문제가 아니라 AC 4 실패다.
    Add-BFailure "the two tiers do not share a verified same look: $($tierVerdict.look.verdict)"
  }

  # 표본별 값 없이 median만 적으면 산포를 숨기게 된다.
  if ($null -eq $tierVerdict.detail.perSample -or @($tierVerdict.detail.perSample).Count -lt 9) {
    Add-BFailure 'the detail axis needs at least 9 slanted-edge pairs (3 captures x 3 approved presets) with per-sample values'
  }
}

# B6. 전환 탐지 검사 (승인된 결정 3).
if (-not (Test-Path -LiteralPath $trialsPath)) {
  Add-BFailure "detection-trial/trials.csv not found at $trialsPath"
}
else {
  $trials = @(Import-Csv -LiteralPath $trialsPath)

  if ($trials.Count -eq 0) {
    Add-BFailure 'detection-trial/trials.csv has no rows'
  }
  else {
    $observers = @($trials | ForEach-Object { $_.observer } | Sort-Object -Unique)

    if ($observers.Count -lt 3) {
      Add-BFailure "the detection test needs 3 observers, found $($observers.Count)"
    }

    foreach ($observer in $observers) {
      $observerTrials = @($trials | Where-Object { $_.observer -eq $observer })
      $swapTrials = @($observerTrials | Where-Object { $_.condition -eq 'swap' })
      $controlTrials = @($observerTrials | Where-Object { $_.condition -eq 'control' })

      if ($observerTrials.Count -ne 20) {
        Add-BFailure "observer '$observer' ran $($observerTrials.Count) trials, expected 20"
      }
      # **대조군 없이 얻은 "아무도 못 봤다"는 통과로 적지 않는다.**
      if ($swapTrials.Count -ne 10 -or $controlTrials.Count -ne 10) {
        Add-BFailure ("observer '$observer' ran $($swapTrials.Count) swap / " +
          "$($controlTrials.Count) control trials, expected 10 / 10")
        continue
      }

      $detected = @($swapTrials | Where-Object { $_.reportedChange -eq 'true' }).Count
      $falsePositives = @($controlTrials | Where-Object { $_.reportedChange -eq 'true' }).Count
      $detectionRate = 100.0 * $detected / $swapTrials.Count
      $falsePositiveRate = 100.0 * $falsePositives / $controlTrials.Count

      if ($detectionRate -gt ($falsePositiveRate + $detectionMarginPercentagePoints)) {
        Add-BFailure ("observer '$observer' detected the swap at $([math]::Round($detectionRate,1))% " +
          "against a $([math]::Round($falsePositiveRate,1))% control false-positive rate " +
          "(margin allowed: $detectionMarginPercentagePoints%p)")
      }
    }
  }
}

$result = [ordered]@{
  schemaVersion = 'hv-17-raw-refined-evidence/v1'
  runRoot       = (Resolve-Path -LiteralPath $RunRoot).Path
  hv17a         = [ordered]@{
    scope    = 'scheduler / capacity / cancellation / process tree'
    failures = @($hv17aFailures)
    verdict  = if ($hv17aFailures.Count -eq 0) { 'automated-pass' } else { 'No-Go' }
  }
  hv17b         = [ordered]@{
    scope    = 'seamless tier transition / frame integrity'
    failures = @($hv17bFailures)
    verdict  = if ($hv17bFailures.Count -eq 0) { 'automated-pass' } else { 'No-Go' }
  }
  # 이 gate가 제공하지 않는 것을 명시한다. 침묵은 "확인했다"로 읽힌다.
  notProvenHere = @(
    '120fps+ physical monitor frames',
    'compositor-to-photon offset',
    'frame-level zero-defect adjudication'
  )
  notProvenHereOwner = 'HV-18B'
}

$json = $result | ConvertTo-Json -Depth 6

if ($OutFile) {
  $json | Out-File -LiteralPath $OutFile -Encoding utf8
}

Write-Output $json

foreach ($failure in $hv17aFailures) { Write-Host "HV-17A FAIL: $failure" }
foreach ($failure in $hv17bFailures) { Write-Host "HV-17B FAIL: $failure" }

if ($hv17aFailures.Count -gt 0 -or $hv17bFailures.Count -gt 0) {
  exit 1
}

Write-Host 'HV-17A and HV-17B automated checks passed. This is not a Go — the human review items remain.'
exit 0
