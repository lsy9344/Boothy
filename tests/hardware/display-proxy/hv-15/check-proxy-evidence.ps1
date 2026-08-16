<#
.SYNOPSIS
  HV-15 (Story 7.4) 기계식 게이트 — display-fit preset proxy 증거의 완결성 검사.

.DESCRIPTION
  이 스크립트는 **기계적으로 확인 가능한 것만** 본다.
  PASS는 HV-15 Go가 아니다. display profile 확정, bundle 승인 근거, 품질 corpus,
  zero-wrong-frame 사람 검토, HV-14 route 결정이 모두 갖춰져야 Go다.

  검사 항목:
    1. 모든 proxy generation이 display fit을 만족한다 (upscale 0)
    2. 모든 proxy generation이 provenance를 싣고, preset이 기대값과 일치한다
    3. 목표 크기가 회차의 실측 requiredSource와 일치한다
    4. commit된 generation 하나당 present 표본 행이 정확히 하나다
    5. fixture 표본과 proxy 표본이 한 회차에 섞이지 않았다
    6. 거부된 generation은 전부 고유 사유를 남겼다

.PARAMETER SessionRoot
  세션 루트. `renders/display/`와 `diagnostics/`를 포함해야 한다.
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][string]$SessionRoot,
  [Parameter(Mandatory = $true)][string]$ExpectedPresetId,
  [Parameter(Mandatory = $true)][string]$ExpectedPresetVersion,
  [Parameter(Mandatory = $true)][int]$RequiredSourceWidthPx,
  [Parameter(Mandatory = $true)][int]$RequiredSourceHeightPx
)

$ErrorActionPreference = 'Stop'

$failures = New-Object System.Collections.Generic.List[string]
function Add-Failure([string]$message) { $failures.Add($message) | Out-Null }

$journalPath = Join-Path $SessionRoot 'renders/display/generations.jsonl'
$presentPath = Join-Path $SessionRoot 'diagnostics/viewer-present.jsonl'

if (-not (Test-Path $journalPath)) {
  Write-Host "FAIL: generations.jsonl not found at $journalPath"
  exit 1
}
if (-not (Test-Path $presentPath)) {
  Write-Host "FAIL: viewer-present.jsonl not found at $presentPath"
  exit 1
}

# 손상된 행을 조용히 건너뛰면 분모가 줄어 성공률이 실제보다 좋아 보인다.
function Read-JsonLines([string]$path, [string]$label) {
  $rows = New-Object System.Collections.Generic.List[object]
  $lineNumber = 0
  foreach ($line in Get-Content -LiteralPath $path) {
    $lineNumber++
    if ([string]::IsNullOrWhiteSpace($line)) { continue }
    try {
      $rows.Add(($line | ConvertFrom-Json)) | Out-Null
    }
    catch {
      Add-Failure "${label}: line ${lineNumber} is malformed JSON and cannot be counted"
    }
  }
  return $rows
}

$journal = Read-JsonLines $journalPath 'generations.jsonl'
$present = Read-JsonLines $presentPath 'viewer-present.jsonl'

$committed = @($journal | Where-Object { $_.outcome -eq 'committed' -and $null -ne $_.generation })
$rejected = @($journal | Where-Object { $_.outcome -eq 'rejected' })
$proxyGenerations = @($committed | Where-Object { $_.generation.tier -eq 'displayFitPresetProxy' })
$sampleGenerations = @($committed | Where-Object { $_.generation.tier -eq 'sample' })

if ($proxyGenerations.Count -eq 0) {
  Add-Failure 'no displayFitPresetProxy generation was committed — the lane never ran'
}

# 5. fixture와 실제 결과가 한 회차에 섞이면 표본이 오염된다.
if ($sampleGenerations.Count -gt 0) {
  Add-Failure "measurement fixtures ($($sampleGenerations.Count)) were committed in the same run as preset proxies — set BOOTHY_DISPLAY_SAMPLE_MODE to off"
}

foreach ($row in $proxyGenerations) {
  $generation = $row.generation
  $id = $generation.generationId

  # 1. contain display fit. 한 축이 경계에 닿아야 브라우저 확대 없이 표시할 수 있다.
  if ($generation.sourceWidthPx -lt $RequiredSourceWidthPx -and $generation.sourceHeightPx -lt $RequiredSourceHeightPx) {
    Add-Failure "${id}: committed at $($generation.sourceWidthPx)x$($generation.sourceHeightPx); both axes are below the contain boundary $RequiredSourceWidthPx x $RequiredSourceHeightPx"
  }

  # 2. provenance.
  $provenance = $generation.proxyProvenance
  if ($null -eq $provenance) {
    Add-Failure "${id}: proxy generation carries no provenance"
    continue
  }

  foreach ($field in @('presetId', 'presetVersion', 'approvalBasis', 'proxyRecipeVersion', 'referenceRenderer', 'referenceRendererVersion', 'sourceRoute', 'sourceAssetHash', 'displayProfileId')) {
    if ([string]::IsNullOrWhiteSpace([string]$provenance.$field)) {
      Add-Failure "${id}: provenance field '$field' is empty"
    }
  }

  if ($provenance.presetId -ne $ExpectedPresetId -or $provenance.presetVersion -ne $ExpectedPresetVersion) {
    Add-Failure "${id}: preset is $($provenance.presetId)@$($provenance.presetVersion), expected $ExpectedPresetId@$ExpectedPresetVersion"
  }

  # 정확 경로 프레임과 승인된 근사 프레임은 evidence에서 구분되어야 한다.
  if ($provenance.approvalBasis -notin @('exact-reference-renderer', 'visual-approval')) {
    Add-Failure "${id}: approvalBasis is '$($provenance.approvalBasis)', expected exact-reference-renderer or visual-approval"
  }
  # 정확 경로라고 주장하려면 source가 RAW 원본이어야 한다.
  if ($provenance.approvalBasis -eq 'exact-reference-renderer' -and $provenance.sourceRoute -ne 'raw-original') {
    Add-Failure "${id}: claims the exact reference-renderer path but rendered from '$($provenance.sourceRoute)'"
  }

  # 3. 목표 크기가 회차의 실측 photoRect에서 왔는가.
  if ($provenance.targetWidthPx -ne $RequiredSourceWidthPx -or $provenance.targetHeightPx -ne $RequiredSourceHeightPx) {
    Add-Failure "${id}: rendered for $($provenance.targetWidthPx)x$($provenance.targetHeightPx), not the measured $RequiredSourceWidthPx x $RequiredSourceHeightPx"
  }

  # 확정 자산이 실제로 존재해야 한다.
  if (-not (Test-Path -LiteralPath $generation.assetPath)) {
    Add-Failure "${id}: committed asset is missing at $($generation.assetPath)"
  }
}

# 4. commit된 generation 하나당 terminal 행 정확히 하나.
#    Story 7.2가 이 결함으로 한 회차를 잃었다.
foreach ($row in $proxyGenerations) {
  $id = $row.generation.generationId
  $rows = @($present | Where-Object { $_.generationId -eq $id })

  if ($rows.Count -eq 0) {
    Add-Failure "${id}: committed but has no terminal present row — 'not displayed' and 'report lost' are indistinguishable"
  }
  elseif ($rows.Count -gt 1) {
    Add-Failure "${id}: has $($rows.Count) terminal present rows, expected exactly 1"
  }
  elseif ($rows[0].tier -ne 'displayFitPresetProxy') {
    Add-Failure "${id}: present row records tier '$($rows[0].tier)', expected displayFitPresetProxy"
  }
}

# 6. 거부는 조용히 사라지지 않는다.
foreach ($row in $rejected) {
  if ([string]::IsNullOrWhiteSpace([string]$row.rejectReason)) {
    Add-Failure "a rejected generation (seq $($row.generationSeq)) recorded no reason"
  }
}

Write-Host '--- HV-15 mechanical gate ---'
Write-Host "committed proxy generations : $($proxyGenerations.Count)"
Write-Host "committed fixtures          : $($sampleGenerations.Count) (must be 0)"
Write-Host "rejected generations        : $($rejected.Count)"
Write-Host "present rows                : $($present.Count)"

if ($failures.Count -gt 0) {
  Write-Host ''
  Write-Host "FAIL: $($failures.Count) problem(s)"
  foreach ($failure in $failures) { Write-Host "  - $failure" }
  exit 1
}

Write-Host ''
Write-Host 'PASS: telemetry and provenance are complete for this run.'
Write-Host 'NOTE: this is NOT HV-15 Go. Display profile confirmation, bundle approval evidence,'
Write-Host '      quality corpus review, the zero-wrong-frame human review, and the HV-14 route'
Write-Host '      decision are all still required.'
exit 0
