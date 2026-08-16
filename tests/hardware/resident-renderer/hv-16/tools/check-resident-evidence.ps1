<#
.SYNOPSIS
  HV-16 (Story 7.5) 기계식 게이트 — 상주 renderer evidence 패키지의 완결성 검사.

.DESCRIPTION
  이 스크립트는 **기계적으로 확인 가능한 것만** 본다.
  PASS는 HV-16 `Go`가 아니다. 이 게이트가 확인하는 것은
  "판정을 내릴 만큼 증거가 갖춰졌는가"이지 "채택해도 되는가"가 아니다.

  검사 항목:
    1. 필수 문서와 원자료 파일이 전부 존재한다
    2. baseline 원자료의 행 수가 선언한 표본 수와 일치한다 (느린 표본을 뺀 흔적이 없다)
    3. parity 결과의 측정 쌍 수가 선언 쌍 수와 같다 (읽지 못한 표본을 조용히 빼지 않았다)
    4. parity 결과가 판정 필드를 들고 있지 않다 (측정과 판정을 섞지 않았다)
    5. `decision.md`가 명시적 판정 문자열을 담고 있다
    6. **fixture-only 결과에 productionEligible=true가 붙은 행이 하나도 없다**
    7. 승인된 direct decoder 목록이 비어 있다면 어떤 행도 production eligible이 아니다

.PARAMETER RunRoot
  HV-16 회차 루트. 이 저장소에서는 `tests/hardware/resident-renderer/hv-16`.

.PARAMETER GenerationsJsonl
  선택. 상주 후보가 게시한 generation journal. 있으면 6·7번 항목을 실제 행으로 검사한다.
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][string]$RunRoot,
  [Parameter(Mandatory = $false)][string]$GenerationsJsonl
)

$ErrorActionPreference = 'Stop'

$failures = New-Object System.Collections.Generic.List[string]
function Add-Failure([string]$message) { $failures.Add($message) | Out-Null }

# --- 1. 필수 파일 --------------------------------------------------------
$required = @(
  'decision.md',
  't0-input-premise.md',
  'environment.md',
  'README.md',
  'baseline/darktable-oneshot-latency.csv',
  'baseline/wic-cr2-decode.csv',
  'baseline/summary.json',
  'capability/engine-contract.json',
  'parity/wic-vs-darktable-neutral.json',
  'parity/wic-vs-darktable-preset.json'
)

foreach ($relative in $required) {
  $path = Join-Path $RunRoot $relative
  if (-not (Test-Path -LiteralPath $path)) {
    Add-Failure "missing required evidence file: $relative"
  }
}

if ($failures.Count -gt 0) {
  foreach ($failure in $failures) { Write-Host "FAIL: $failure" }
  Write-Host "HV-16 evidence gate: FAIL ($($failures.Count))"
  exit 1
}

# --- 2. baseline 표본 완결성 ---------------------------------------------
$summary = Get-Content -LiteralPath (Join-Path $RunRoot 'baseline/summary.json') -Raw | ConvertFrom-Json

$latencyRows = @(Import-Csv -LiteralPath (Join-Path $RunRoot 'baseline/darktable-oneshot-latency.csv'))
$declaredLatency = [int]$summary.darktableOneShot.all.n

if ($latencyRows.Count -ne $declaredLatency) {
  Add-Failure "darktable baseline row count $($latencyRows.Count) does not match the declared sample count $declaredLatency"
}

$blankLatency = @($latencyRows | Where-Object { [string]::IsNullOrWhiteSpace($_.wallMs) })
if ($blankLatency.Count -gt 0) {
  Add-Failure "darktable baseline has $($blankLatency.Count) rows without a measured latency"
}

$wicRows = @(Import-Csv -LiteralPath (Join-Path $RunRoot 'baseline/wic-cr2-decode.csv'))
$declaredWic = 0
foreach ($mode in $summary.windowsWicDirectDecode.perMode.PSObject.Properties) {
  $declaredWic += [int]$mode.Value.n
}

if ($wicRows.Count -ne $declaredWic) {
  Add-Failure "WIC decode row count $($wicRows.Count) does not match the declared sample count $declaredWic"
}

foreach ($row in $wicRows) {
  foreach ($field in @('openMs', 'decodePixelsMs', 'totalMs')) {
    $parsed = 0.0
    if (
      [string]::IsNullOrWhiteSpace($row.$field) -or
      -not [double]::TryParse(
        [string]$row.$field,
        [System.Globalization.NumberStyles]::Float,
        [System.Globalization.CultureInfo]::InvariantCulture,
        [ref]$parsed
      ) -or
      $parsed -lt 0
    ) {
      Add-Failure "WIC decode row '$($row.capture)' has an invalid $field value"
    }
  }
}

# --- 3~4. parity 결과 -----------------------------------------------------
foreach ($parityFile in @('parity/wic-vs-darktable-preset.json', 'parity/wic-vs-darktable-neutral.json')) {
  $path = Join-Path $RunRoot $parityFile
  if (-not (Test-Path -LiteralPath $path)) { continue }

  $parity = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json

  if ([int]$parity.measuredPairCount -ne [int]$parity.declaredPairCount) {
    Add-Failure "${parityFile}: measured $($parity.measuredPairCount) of $($parity.declaredPairCount) declared pairs"
  }

  if ($parity.schemaVersion -ne 'hv-16-parity/v2') {
    Add-Failure "${parityFile}: expected schemaVersion hv-16-parity/v2"
  }

  if (
    [string]::IsNullOrWhiteSpace($parity.ssimDefinition) -or
    [string]::IsNullOrWhiteSpace($parity.deltaEDefinition) -or
    [string]::IsNullOrWhiteSpace($parity.clippingDefinition)
  ) {
    Add-Failure "${parityFile}: the metric definitions must travel with the numbers"
  }

  foreach ($row in $parity.results) {
    if ($null -eq $row.measurement) {
      Add-Failure "${parityFile}: pair $($row.captureId)/$($row.preset) has no measurement"
      continue
    }
    # 측정 결과가 스스로 합격을 주장하면 지표를 "통과하도록" 손보고 싶은 유혹이 생긴다.
    foreach ($forbidden in @('passed', 'verdict', 'gate')) {
      if ($null -ne $row.measurement.PSObject.Properties[$forbidden]) {
        Add-Failure "${parityFile}: measurement carries a verdict field '$forbidden'; judging belongs to the decision, not the metric"
      }
    }
  }
}

# --- 5. 판정 문자열 -------------------------------------------------------
$decision = Get-Content -LiteralPath (Join-Path $RunRoot 'decision.md') -Raw
$declared = @([regex]::Matches(
  $decision,
  '(?m)^Verdict:\s*(Go|Technology No-Go|Blocked)\s*$'
))

if ($declared.Count -ne 1) {
  Add-Failure 'decision.md must declare exactly one structured Verdict: Go, Technology No-Go, or Blocked'
}

# --- 6~7. 채택 자격 위조 검사 --------------------------------------------
$contract = Get-Content -LiteralPath (Join-Path $RunRoot 'capability/engine-contract.json') -Raw | ConvertFrom-Json
$approvedDecoders = @($contract.adoptionGate.approvedDirectDecoders)

if ($PSBoundParameters.ContainsKey('GenerationsJsonl') -and -not [string]::IsNullOrWhiteSpace($GenerationsJsonl)) {
  if (-not (Test-Path -LiteralPath $GenerationsJsonl)) {
    Add-Failure "generations journal not found: $GenerationsJsonl"
  }
  else {
    $lineNumber = 0
    foreach ($line in Get-Content -LiteralPath $GenerationsJsonl) {
      $lineNumber++
      if ([string]::IsNullOrWhiteSpace($line)) { continue }

      try { $record = $line | ConvertFrom-Json }
      catch {
        # 손상된 행을 건너뛰면 위조된 행이 그 뒤에 숨을 수 있다.
        Add-Failure "generations line ${lineNumber} is malformed JSON and cannot be checked"
        continue
      }

      $resident = $record.generation.proxyProvenance.residentProvenance
      if ($null -eq $resident) { continue }

      $requiredResidentFields = @(
        'producerRenderer', 'producerRendererVersion', 'producerBuildId',
        'executionMode', 'recipeSchemaVersion', 'compiledRecipeHash', 'programHash',
        'contextInitializedAtMicros', 'hotPathProgramCompileCount',
        'hotPathProcessStartCount', 'inputProvenance', 'inputProducer',
        'inputProducerVersion', 'sourceReadyAtMicros', 'inputStartupCostMicros',
        'productionEligible', 'gpuVendor', 'gpuRenderer'
      )
      foreach ($field in $requiredResidentFields) {
        if ($null -eq $resident.PSObject.Properties[$field]) {
          Add-Failure "generations line ${lineNumber}: resident provenance is missing '$field'"
        }
      }

      foreach ($field in @(
        'producerRenderer', 'producerRendererVersion', 'producerBuildId',
        'recipeSchemaVersion', 'compiledRecipeHash', 'programHash',
        'inputProducer', 'inputProducerVersion', 'gpuVendor', 'gpuRenderer'
      )) {
        if ([string]::IsNullOrWhiteSpace([string]$resident.$field)) {
          Add-Failure "generations line ${lineNumber}: resident provenance has a blank '$field'"
        }
      }

      if (
        [long]$resident.contextInitializedAtMicros -le 0 -or
        [long]$resident.sourceReadyAtMicros -le 0 -or
        [long]$resident.contextInitializedAtMicros -gt [long]$resident.sourceReadyAtMicros
      ) {
        Add-Failure "generations line ${lineNumber}: resident context/source timestamps are invalid"
      }

      if (
        [int]$resident.hotPathProgramCompileCount -gt 0 -or
        [int]$resident.hotPathProcessStartCount -gt 0
      ) {
        Add-Failure "generations line ${lineNumber}: resident evidence has a non-empty hot path"
      }

      if ($resident.productionEligible -eq $true) {
        if ($approvedDecoders.Count -eq 0) {
          Add-Failure "generations line ${lineNumber}: productionEligible=true while no direct decoder is approved"
        }
        elseif ($approvedDecoders -notcontains $resident.inputProducer) {
          Add-Failure "generations line ${lineNumber}: productionEligible=true from unapproved decoder '$($resident.inputProducer)'"
        }

        if ($resident.inputProvenance -ne 'real-capture-direct') {
          Add-Failure "generations line ${lineNumber}: productionEligible=true from input '$($resident.inputProvenance)'"
        }

      }
      elseif ([string]::IsNullOrWhiteSpace($resident.adoptionBlockReason)) {
        Add-Failure "generations line ${lineNumber}: demoted resident frame hides why it was demoted"
      }

      if ($resident.executionMode -ne 'evidence') {
        Add-Failure "generations line ${lineNumber}: only evidence mode can appear on a published frame"
      }
    }
  }
}

# --- 결과 ----------------------------------------------------------------
if ($failures.Count -gt 0) {
  foreach ($failure in $failures) { Write-Host "FAIL: $failure" }
  Write-Host "HV-16 evidence gate: FAIL ($($failures.Count))"
  exit 1
}

Write-Host "HV-16 evidence gate: PASS"
Write-Host "  darktable baseline rows : $($latencyRows.Count)"
Write-Host "  WIC decode rows         : $($wicRows.Count)"
Write-Host "  approved direct decoders: $($approvedDecoders.Count)"
Write-Host "NOTE: PASS means the package is complete enough to judge. It is not a Go."
exit 0
