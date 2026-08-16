<#
.SYNOPSIS
  `check-resident-evidence.ps1`의 self-test.

.DESCRIPTION
  게이트를 실제 회차에만 돌려 보고 PASS가 나오면, 그 게이트가 **무엇도 잡지 못하는**
  경우와 구분되지 않는다. 그래서 결함을 일부러 심은 사본에 돌려서 전부 차단되는지 확인한다.

  심는 결함:
    1. 필수 문서 누락
    2. baseline 표본을 몰래 줄이기 (느린 표본 제거)
    3. parity 결과에서 읽지 못한 표본을 조용히 빼기
    4. parity 측정값에 합격 판정 필드 붙이기
    5. decision.md에서 판정 문자열 지우기
    6. fixture-only 결과에 productionEligible=true 붙이기
    7. 강등 사유 없는 상주 프레임
    8. `off` mode가 게시된 프레임에 나타나기
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory = $false)][string]$RunRoot
)

$ErrorActionPreference = 'Stop'

# `$PSScriptRoot`는 param 기본값 자리에서 비어 있을 수 있다. 본문에서 확정한다.
$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
if ([string]::IsNullOrWhiteSpace($RunRoot)) {
  $RunRoot = (Resolve-Path (Join-Path $scriptRoot '..')).Path
}

$gate = Join-Path $scriptRoot 'check-resident-evidence.ps1'
$workRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("hv16-gate-selftest-" + [System.Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $workRoot | Out-Null

$passed = 0
$failed = 0

function New-Fixture([string]$name) {
  if ([string]::IsNullOrWhiteSpace($script:workRoot)) {
    throw "self-test bug: workRoot is not set"
  }

  $target = [System.IO.Path]::Combine($script:workRoot, $name)
  Copy-Item -LiteralPath $script:RunRoot -Destination $target -Recurse -Force

  if (-not (Test-Path -LiteralPath $target)) {
    throw "self-test bug: fixture '$name' was not created at $target"
  }

  # 함수 출력에 다른 값이 섞이면 호출자가 배열을 받는다. 경로 하나만 내보낸다.
  Write-Output -InputObject $target
}

function Invoke-Gate([string]$root, [string]$journal) {
  $arguments = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $gate, '-RunRoot', $root)
  if (-not [string]::IsNullOrWhiteSpace($journal)) {
    $arguments += @('-GenerationsJsonl', $journal)
  }
  & powershell @arguments *> $null
  return $LASTEXITCODE
}

function Assert-Blocked([string]$label, [string]$root, [string]$journal) {
  $code = Invoke-Gate $root $journal
  if ($code -eq 0) {
    Write-Host "SELFTEST FAIL: '$label' was not blocked"
    $script:failed++
  }
  else {
    Write-Host "  blocked: $label"
    $script:passed++
  }
}

function Assert-Allowed([string]$label, [string]$root, [string]$journal) {
  $code = Invoke-Gate $root $journal
  if ($code -ne 0) {
    Write-Host "SELFTEST FAIL: '$label' should have passed but was blocked"
    $script:failed++
  }
  else {
    Write-Host "  allowed: $label"
    $script:passed++
  }
}

function Write-Journal([string]$root, [object]$resident) {
  $record = @{
    generation = @{
      generationId     = 'req-1-000001'
      proxyProvenance  = @{
        presetId           = 'preset_daylight'
        residentProvenance = $resident
      }
    }
  }
  $path = Join-Path $root 'generations.jsonl'
  Set-Content -LiteralPath $path -Value ($record | ConvertTo-Json -Depth 8 -Compress) -Encoding utf8
  return $path
}

function New-Resident() {
  return [ordered]@{
    producerRenderer             = 'webgl2-resident'
    producerRendererVersion      = '0.1.0-spike'
    producerBuildId              = 'spike-build-1'
    executionMode                = 'evidence'
    recipeSchemaVersion          = 'resident-recipe/v1'
    compiledRecipeHash           = 'fnv1a64:0123456789abcdef'
    programHash                  = 'fnv1a64:bcbf55e37b6cc622'
    contextInitializedAtMicros   = 900000
    sourceReadyAtMicros          = 1200000
    hotPathProgramCompileCount   = 0
    hotPathProcessStartCount     = 0
    inputProvenance              = 'predecoded-fixture'
    inputProducer                = 'darktable-cli'
    inputProducerVersion         = '5.4.1'
    inputStartupCostMicros       = 0
    productionEligible           = $false
    adoptionBlockReason          = 'resident-source-route-unapproved'
    gpuVendor                    = 'NVIDIA'
    gpuRenderer                  = 'GeForce GTX 1080'
    fallbackReason               = $null
  }
}

Write-Host 'HV-16 gate self-test'

# 정상 회차는 통과해야 한다. 아니면 아래 차단들이 아무 의미가 없다.
$clean = New-Fixture 'clean'
Assert-Allowed 'a complete package' $clean $null

$cleanJournal = New-Fixture 'clean-journal'
$journalPath = Write-Journal $cleanJournal (New-Resident)
Assert-Allowed 'an honest fixture-only resident frame' $cleanJournal $journalPath

# 1. 필수 문서 누락
$missing = New-Fixture 'missing-decision'
Remove-Item -LiteralPath (Join-Path $missing 'decision.md') -Force
Assert-Blocked 'a package with no decision.md' $missing $null

$missingNeutral = New-Fixture 'missing-neutral-parity'
Remove-Item -LiteralPath (Join-Path $missingNeutral 'parity/wic-vs-darktable-neutral.json') -Force
Assert-Blocked 'a package with no neutral parity evidence' $missingNeutral $null

# 2. baseline 표본 몰래 줄이기
$trimmed = New-Fixture 'trimmed-baseline'
$csvPath = Join-Path $trimmed 'baseline/darktable-oneshot-latency.csv'
$rows = Import-Csv -LiteralPath $csvPath
$rows | Sort-Object { [int]$_.wallMs } | Select-Object -First ($rows.Count - 20) |
  Export-Csv -LiteralPath $csvPath -NoTypeInformation -Encoding utf8
Assert-Blocked 'a baseline with the slowest samples removed' $trimmed $null

$blankWic = New-Fixture 'blank-wic-timing'
$wicPath = Join-Path $blankWic 'baseline/wic-cr2-decode.csv'
$wicRows = @(Import-Csv -LiteralPath $wicPath)
$wicRows[0].decodePixelsMs = ''
$wicRows | Export-Csv -LiteralPath $wicPath -NoTypeInformation -Encoding utf8
Assert-Blocked 'a WIC baseline with a blank timing' $blankWic $null

# 3. parity에서 읽지 못한 표본 빼기
$dropped = New-Fixture 'dropped-pair'
$parityPath = Join-Path $dropped 'parity/wic-vs-darktable-preset.json'
$parity = Get-Content -LiteralPath $parityPath -Raw | ConvertFrom-Json
$parity.declaredPairCount = [int]$parity.declaredPairCount + 3
Set-Content -LiteralPath $parityPath -Value ($parity | ConvertTo-Json -Depth 12) -Encoding utf8
Assert-Blocked 'a parity run that silently dropped pairs' $dropped $null

$undefinedMetric = New-Fixture 'undefined-clipping-metric'
$parityPath = Join-Path $undefinedMetric 'parity/wic-vs-darktable-preset.json'
$parity = Get-Content -LiteralPath $parityPath -Raw | ConvertFrom-Json
$parity.clippingDefinition = ''
Set-Content -LiteralPath $parityPath -Value ($parity | ConvertTo-Json -Depth 12) -Encoding utf8
Assert-Blocked 'a parity run with an undefined clipping metric' $undefinedMetric $null

# 4. 측정값에 판정 필드 붙이기
$judging = New-Fixture 'judging-metric'
$parityPath = Join-Path $judging 'parity/wic-vs-darktable-preset.json'
$parity = Get-Content -LiteralPath $parityPath -Raw | ConvertFrom-Json
$parity.results[0].measurement | Add-Member -NotePropertyName 'passed' -NotePropertyValue $true
Set-Content -LiteralPath $parityPath -Value ($parity | ConvertTo-Json -Depth 12) -Encoding utf8
Assert-Blocked 'a metric that judges itself' $judging $null

# 5. 판정 문자열 삭제
$noVerdict = New-Fixture 'no-verdict'
$decisionPath = Join-Path $noVerdict 'decision.md'
Set-Content -LiteralPath $decisionPath -Value '# 결정' -Encoding utf8
Assert-Blocked 'a decision with no verdict' $noVerdict $null

# 6. fixture-only 결과에 production 자격 붙이기
$forged = New-Fixture 'forged-eligible'
$resident = New-Resident
$resident.productionEligible = $true
$resident.adoptionBlockReason = $null
Assert-Blocked 'a fixture-only frame claiming production eligibility' $forged (Write-Journal $forged $resident)

# 6b. 승인되지 않은 direct decoder
$unapproved = New-Fixture 'unapproved-decoder'
$resident = New-Resident
$resident.inputProvenance = 'real-capture-direct'
$resident.inputProducer = 'windows-wic-raw'
$resident.productionEligible = $true
$resident.adoptionBlockReason = $null
Assert-Blocked 'a production claim from an unapproved decoder' $unapproved (Write-Journal $unapproved $resident)

# 6c. hot path가 비어 있지 않은데 production 자격 주장
$hotPath = New-Fixture 'hot-path-compile'
$resident = New-Resident
$resident.inputProvenance = 'real-capture-direct'
$resident.productionEligible = $true
$resident.adoptionBlockReason = $null
$resident.hotPathProgramCompileCount = 1
Assert-Blocked 'a production claim with a non-empty hot path' $hotPath (Write-Journal $hotPath $resident)

# 7. 강등 사유 없는 상주 프레임
$silent = New-Fixture 'silent-demotion'
$resident = New-Resident
$resident.adoptionBlockReason = $null
Assert-Blocked 'a demoted frame that hides its reason' $silent (Write-Journal $silent $resident)

# 8. off mode가 게시된 프레임에 나타남
$offMode = New-Fixture 'off-mode'
$resident = New-Resident
$resident.executionMode = 'off'
Assert-Blocked 'an off-mode frame on the publication boundary' $offMode (Write-Journal $offMode $resident)

$missingProvenance = New-Fixture 'missing-provenance-field'
$resident = New-Resident
$resident.Remove('sourceReadyAtMicros')
Assert-Blocked 'a resident frame missing required provenance' $missingProvenance (Write-Journal $missingProvenance $resident)

# 9. 손상된 journal 행
$malformed = New-Fixture 'malformed-journal'
$malformedPath = Join-Path $malformed 'generations.jsonl'
Set-Content -LiteralPath $malformedPath -Value '{ this is not json' -Encoding utf8
Assert-Blocked 'a malformed journal line' $malformed $malformedPath

Remove-Item -LiteralPath $workRoot -Recurse -Force -ErrorAction SilentlyContinue

Write-Host ''
Write-Host "self-test: $passed passed, $failed failed"
if ($failed -gt 0) { exit 1 }
exit 0
