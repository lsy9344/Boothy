<#
.SYNOPSIS
  HV-13B 계측 완결성 게이트. commit된 generation 하나당 terminal present 행이 정확히 하나인지 검사한다.

.DESCRIPTION
  2026-08-12 offscreen 회차에서 10개 중 1개의 terminal 행이 사라진 것을 사람이 눈으로 찾아냈다.
  분모가 조용히 줄면 성공률이 실제보다 좋아 보이므로, 이 검사는 회차마다 기계적으로 돌린다.

  통과 조건:
    - generations.jsonl의 committed generation id 집합 == viewer-present.jsonl의 terminal 행 id 집합
    - 중복 terminal 행 없음
    - present-unreported 행 없음 (있으면 그 표본은 실패로 집계한다)

  `confidence: unreported` 행은 KPI 집계에서 빠지지만 성공률 분모에는 남는다.

  Story 7.6부터 **한 촬영이 generation 2개를 만든다** (displayFitPresetProxy → rawRefinedDisplay).
  완결성 계약은 그대로다 — commit된 generation **하나당** terminal 행 하나다.
  달라지는 것은 KPI 분모뿐이다: 정밀본 present는 두 번째 terminal 행이지 KPI 종료점이 아니므로,
  qualifying 판정은 **첫 자격 frame tier에 대해서만** 한다. 정밀본 행까지 분모에 넣으면
  같은 촬영이 두 번 세어지고, 정밀본을 게시하지 않은 회차와 비교가 불가능해진다.

  `tier` 필드가 없는 v1/v2 행은 정밀본이 존재하지 않던 시절의 행이므로 전부 첫 프레임으로 센다.

.PARAMETER SessionEvidenceDir
  회차 디렉터리의 session-evidence 경로.
  예: tests/hardware/viewer-present/run-<timestamp>/session-evidence

.EXAMPLE
  ./check-telemetry-completeness.ps1 -SessionEvidenceDir ../run-20260812-172000-hv13b-offscreen-direct/session-evidence
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [string]$SessionEvidenceDir,

  [string]$OutFile
)

$ErrorActionPreference = 'Stop'

function Read-JsonLines {
  param([string]$Path)

  if (-not (Test-Path -LiteralPath $Path)) {
    throw "필요한 증거 파일이 없습니다: $Path"
  }

  Get-Content -LiteralPath $Path -Encoding utf8 |
    Where-Object { $_.Trim().Length -gt 0 } |
    ForEach-Object { $_ | ConvertFrom-Json }
}

$generationsPath = Join-Path $SessionEvidenceDir 'display/generations.jsonl'
$presentPath = Join-Path $SessionEvidenceDir 'diagnostics/viewer-present.jsonl'

$committed = @(Read-JsonLines -Path $generationsPath | Where-Object { $_.outcome -eq 'committed' })
$present = @(Read-JsonLines -Path $presentPath)

$committedIds = @($committed | ForEach-Object { $_.generation.generationId })
$presentIds = @($present | ForEach-Object { $_.generationId })

# Story 7.6. 정밀본 generation은 승급이지 첫 자격 frame이 아니다.
$refinedTier = 'rawRefinedDisplay'
$firstFrameCommittedIds = @(
  $committed |
    Where-Object { $_.generation.tier -ne $refinedTier } |
    ForEach-Object { $_.generation.generationId }
)
$refinedCommittedIds = @(
  $committed |
    Where-Object { $_.generation.tier -eq $refinedTier } |
    ForEach-Object { $_.generation.generationId }
)

$missing = @($committedIds | Where-Object { $presentIds -notcontains $_ })
$unknown = @($presentIds | Where-Object { $committedIds -notcontains $_ })
$duplicated = @(
  $presentIds |
    Group-Object |
    Where-Object { $_.Count -gt 1 } |
    ForEach-Object { $_.Name }
)
$unreported = @($present | Where-Object { $_.rejectReason -eq 'present-unreported' })

# 성공 표본: presented + trusted input + 입력 이후 창 이벤트 0 + **첫 자격 frame tier**.
# 정밀본 행은 `qualifyingLatencyMicros`를 갖지 않지만, tier로도 명시적으로 걸러 둔다 —
# 두 판정이 어긋나면 그 자체가 계측 결함이므로 아래에서 따로 확인한다.
$qualifying = @(
  $present | Where-Object {
    $_.outcome -eq 'presented' -and
    $_.isTrustedInput -eq $true -and
    $_.viewerWindowEventsAfterInput -eq 0 -and
    $null -ne $_.qualifyingLatencyMicros -and
    $_.tier -ne $refinedTier
  }
)

# 정밀본 행이 KPI 종료점을 주장하면 그 회차의 NFR-003 판정은 거짓이 된다.
$refinedRowsClaimingKpi = @(
  $present | Where-Object {
    $_.tier -eq $refinedTier -and $null -ne $_.qualifyingLatencyMicros
  }
)

$isComplete = ($missing.Count -eq 0) -and ($unknown.Count -eq 0) -and ($duplicated.Count -eq 0)

$result = [ordered]@{
  schemaVersion              = 'hv-13b-telemetry-completeness/v2'
  sessionEvidenceDir         = (Resolve-Path -LiteralPath $SessionEvidenceDir).Path
  committedGenerations       = $committedIds.Count
  # Story 7.6. 한 촬영이 generation 2개를 만들 수 있다. 두 값을 분리해 남긴다.
  firstFrameGenerations      = $firstFrameCommittedIds.Count
  refinedGenerations         = $refinedCommittedIds.Count
  terminalPresentRows        = $presentIds.Count
  # KPI 분모는 **첫 자격 frame generation 수**다. 기록된 행 수도, 전체 generation 수도 아니다.
  qualifyingSamples          = $qualifying.Count
  presentUnreportedRows      = $unreported.Count
  refinedRowsClaimingKpi     = $refinedRowsClaimingKpi.Count
  missingTerminalGenerations = $missing
  unknownTerminalGenerations = $unknown
  duplicatedTerminalRows     = $duplicated
  completenessGate           = 'fail'
  verdict                    = 'No-Go'
}

if ($isComplete) {
  $result.completenessGate = 'pass'
}

# 완결성이 통과해도 present-unreported 표본이 있으면 그 회차는 qualifying이 아니다.
if ($isComplete -and
    $unreported.Count -eq 0 -and
    $refinedRowsClaimingKpi.Count -eq 0 -and
    $qualifying.Count -eq $firstFrameCommittedIds.Count) {
  $result.verdict = 'telemetry-complete'
}

$json = $result | ConvertTo-Json -Depth 5

if ($OutFile) {
  $json | Out-File -LiteralPath $OutFile -Encoding utf8
}

Write-Output $json

if (-not $isComplete) {
  Write-Error "계측 완결성 실패: committed $($committedIds.Count) / terminal $($presentIds.Count). 남은 표본만으로 집계하지 마세요."
  exit 1
}

if ($unreported.Count -gt 0) {
  Write-Error "present-unreported 표본 $($unreported.Count)건. 그 표본은 실패로 집계하고 원인을 result.md에 기록하세요."
  exit 1
}

if ($refinedRowsClaimingKpi.Count -gt 0) {
  Write-Error "정밀본 행 $($refinedRowsClaimingKpi.Count)건이 qualifyingLatencyMicros를 들고 있습니다. 정밀본 present는 KPI 종료점이 아닙니다 — 이 회차의 NFR-003 수치를 신뢰하지 마세요."
  exit 1
}

exit 0
