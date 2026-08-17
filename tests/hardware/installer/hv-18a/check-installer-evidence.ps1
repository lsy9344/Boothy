<#
.SYNOPSIS
  HV-18A (Story 7.7) 기계식 게이트 — 승인된 Windows PC 설치 증거의 완결성 검사.

.DESCRIPTION
  **PASS는 HV-18A `Go`가 아니다.** 이 스크립트는 기계적으로 확인 가능한 것만 본다.
  fixture가 실제로 화면에 제대로 보였는지, 설치 절차가 정직하게 수행됐는지, 승인자 서명은
  사람이 판단한다.

  모든 검사가 통과해야만 `Go` 후보다:

    C1. self-check 보고서가 존재하고 `overall: "pass"`
    C2. `darktableResolution.source == "bundled-resource"`  (**핀이 깨지지 않았다는 증거**)
    C3. 설치본 sha256 이 봉인된 인벤토리와 일치
    C4. 8단계 lifecycle 결과가 모두 기록되어 있고 미실행 항목이 없다
    C5. 제거 후 세션 루트가 **살아 있다** (고객 사진)
    C6. 업그레이드 후 이전 세션이 **읽힌다**
    C7. `signingStatus` 가 `signed` (책임자 면제 가능)
    C8. 네트워크가 실제로 차단된 상태였다는 증거가 있다 (책임자 면제 가능)
    C9. Node / Rust / .NET SDK / darktable / WebView2 가 사전에 없었다는 증거가 있다 (책임자 면제 가능)
    C10. WebView2 런타임 실제 버전이 기록되어 있다 (설치본으로 고정할 수 없으므로)
    C11. 실제 카메라 촬영 1건의 raw-original / proxy / RAW 정밀본 / final 산출물이 있다

  C7/C8/C9를 면제할 때는 회차 루트의 `waivers.json`에 승인자, 승인 시각, 사유를 기록해야 한다.
  면제는 검증 통과가 아니며 결과는 `Go-candidate-with-waivers`로 구분한다.

.PARAMETER RunRoot
  회차 디렉터리. 구조는 이 디렉터리의 README.md 를 따른다.

.PARAMETER OutFile
  판정 JSON 을 남길 경로 (선택).

.EXAMPLE
  ./check-installer-evidence.ps1 -RunRoot ../run-20260820-101500-hv18a
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][string]$RunRoot,
  [string]$OutFile
)

$ErrorActionPreference = 'Stop'

# 파이프로 나가는 출력이 콘솔 코드페이지를 타면 운영자가 사유를 읽을 수 없다.
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$requiredLifecycleSteps = @(
  'install',
  'launch',
  'self-check',
  'fixture-display',
  'upgrade',
  'rollback',
  'uninstall',
  'data-preservation'
)

$failures = New-Object System.Collections.Generic.List[string]
$observations = [ordered]@{}

function Add-Failure {
  param([string]$Check, [string]$Message)
  $failures.Add(("{0}: {1}" -f $Check, $Message)) | Out-Null
}

function Read-JsonOrNull {
  param([string]$Path)

  if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
    return $null
  }

  try {
    return Get-Content -LiteralPath $Path -Raw -Encoding UTF8 | ConvertFrom-Json
  }
  catch {
    return $null
  }
}

function Test-Sha256Text {
  param([string]$Value)
  return $Value -match '^[0-9a-fA-F]{64}$'
}

function Get-FileSha256 {
  param([string]$Path)
  return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

if (-not (Test-Path -LiteralPath $RunRoot -PathType Container)) {
  Write-Host "회차 디렉터리를 찾지 못했습니다: $RunRoot"
  exit 1
}

$selfCheckPath = Join-Path $RunRoot 'self-check/install-self-check.json'
$afterUpgradePath = Join-Path $RunRoot 'self-check/after-upgrade.json'
$afterRollbackPath = Join-Path $RunRoot 'self-check/after-rollback.json'
$sealedInventoryPath = Join-Path $RunRoot 'installer/inventory.release.json'
$installerHashPath = Join-Path $RunRoot 'installer/installer.sha256.txt'
$lifecyclePath = Join-Path $RunRoot 'lifecycle.json'
$preservationPath = Join-Path $RunRoot 'data-preservation.json'
$networkPath = Join-Path $RunRoot 'network/connectivity.json'
$prerequisitesPath = Join-Path $RunRoot 'prerequisites.json'
$pipelinePath = Join-Path $RunRoot 'pipeline/capture-evidence.json'
$waiverPath = Join-Path $RunRoot 'waivers.json'

# --- 책임자 승인 면제 --------------------------------------------------------

$waivers = Read-JsonOrNull $waiverPath
$waivedChecks = @()

if ($null -ne $waivers) {
  if ($waivers.schemaVersion -ne 'hv-18a-waivers/v1') {
    Add-Failure 'W1' "면제 스키마가 올바르지 않습니다: $($waivers.schemaVersion)"
  }
  elseif ([string]::IsNullOrWhiteSpace([string]$waivers.approvedBy) -or
          [string]::IsNullOrWhiteSpace([string]$waivers.approvedAt) -or
          [string]::IsNullOrWhiteSpace([string]$waivers.reason)) {
    Add-Failure 'W1' '면제에는 approvedBy, approvedAt, reason이 모두 필요합니다.'
  }
  else {
    $waivedChecks = @($waivers.waivedChecks | Where-Object { $_ -in @('C7', 'C8', 'C9') } | Select-Object -Unique)
    $observations['waivedChecks'] = $waivedChecks
    $observations['waiverApprovedBy'] = $waivers.approvedBy
    $observations['waiverApprovedAt'] = $waivers.approvedAt
    $observations['waivedEvidence'] = @($waivers.waivedEvidence)
  }
}

function Test-Waived {
  param([string]$Check)
  return $waivedChecks -contains $Check
}

# --- C1. self-check 보고서 ----------------------------------------------------

$selfCheck = Read-JsonOrNull $selfCheckPath

if ($null -eq $selfCheck) {
  Add-Failure 'C1' "self-check 보고서를 읽지 못했습니다: $selfCheckPath"
}
else {
  if ($selfCheck.schemaVersion -ne 'install-self-check/v1') {
    Add-Failure 'C1' "self-check 보고서 스키마가 install-self-check/v1 이 아닙니다: $($selfCheck.schemaVersion)"
  }

  if ($selfCheck.overall -ne 'pass') {
    $failed = @($selfCheck.components | Where-Object { $_.status -eq 'fail' } | ForEach-Object { "$($_.name)=$($_.reasonCode)" })
    Add-Failure 'C1' ("self-check 전체 결과가 pass 가 아닙니다: $($selfCheck.overall). 실패 항목: " + ($failed -join ', '))
  }

  $observations['selfCheckOverall'] = $selfCheck.overall
  $observations['appVersion'] = $selfCheck.appVersion
  $observations['identifier'] = $selfCheck.identifier

  # --- C2. 핀이 깨지지 않았다는 증거 -----------------------------------------

  $source = $selfCheck.darktableResolution.source

  if ($source -ne 'bundled-resource') {
    Add-Failure 'C2' "darktable 이 번들 트리에서 해석되지 않았습니다: '$source'. 이 회차는 핀이 깨진 회차입니다."
  }

  $observations['darktableResolutionSource'] = $source
  $observations['darktableBinary'] = $selfCheck.darktableResolution.binary

  # --- C10. WebView2 실제 버전 -----------------------------------------------

  if ([string]::IsNullOrWhiteSpace([string]$selfCheck.webview2Version)) {
    Add-Failure 'C10' 'WebView2 런타임 버전이 기록되지 않았습니다. 설치본으로 고정할 수 없으므로 회차마다 실측이 필요합니다.'
  }

  $observations['webview2Version'] = $selfCheck.webview2Version
  $observations['helperVersion'] = $selfCheck.helperVersion
}

# --- C11. 실제 카메라 촬영과 capture-bound 산출물 ----------------------------

$pipeline = Read-JsonOrNull $pipelinePath

if ($null -eq $pipeline) {
  Add-Failure 'C11' "실제 촬영 pipeline 증거를 읽지 못했습니다: $pipelinePath"
}
else {
  if ($pipeline.schemaVersion -ne 'hv-18a-capture-evidence/v1') {
    Add-Failure 'C11' "pipeline 증거 스키마가 올바르지 않습니다: $($pipeline.schemaVersion)"
  }

  if ([string]::IsNullOrWhiteSpace([string]$pipeline.captureId)) {
    Add-Failure 'C11' '실제 촬영 captureId가 없습니다.'
  }

  if ($pipeline.cameraCapture.status -ne 'pass' -or $pipeline.cameraCapture.source -ne 'canon-camera') {
    Add-Failure 'C11' 'Canon 카메라의 실제 촬영 성공 증거가 없습니다.'
  }

  $artifacts = @($pipeline.artifacts)
  $runRootFull = [System.IO.Path]::GetFullPath($RunRoot).TrimEnd('\')
  $runRootPrefix = $runRootFull + '\'
  $requiredArtifactRoles = @('raw-original', 'display-proxy', 'raw-refined', 'final')
  $hasRefined = @($artifacts | Where-Object { $_.role -eq 'raw-refined' }).Count -eq 1

  if (-not $hasRefined) {
    $disposition = $pipeline.rawRefinedDisposition
    $hv17RelativePath = [string]$disposition.hv17GatePath
    $partialAccepted = $disposition.status -eq 'not-produced' -and
      $disposition.reason -eq 'tier-not-justified' -and
      -not [string]::IsNullOrWhiteSpace($hv17RelativePath)

    if ($partialAccepted) {
      $hv17Path = [System.IO.Path]::GetFullPath((Join-Path $RunRoot $hv17RelativePath))
      if (-not $hv17Path.StartsWith($runRootPrefix, [System.StringComparison]::OrdinalIgnoreCase) -or
          -not (Test-Path -LiteralPath $hv17Path -PathType Leaf)) {
        $partialAccepted = $false
      }
      else {
        $hv17 = Read-JsonOrNull $hv17Path
        $partialAccepted = $null -ne $hv17 -and
          $hv17.productDecision.verdict -eq 'Partial' -and
          $hv17.productDecision.laneDefaultEnabled -eq $false -and
          $hv17.productDecision.tierDetailVerdict -eq 'tier-not-justified'
      }
    }

    if ($partialAccepted) {
      $requiredArtifactRoles = @('raw-original', 'display-proxy', 'final')
      $observations['rawRefinedDisposition'] = 'not-produced:tier-not-justified'
    }
    else {
      Add-Failure 'C11' 'RAW 정밀본이 없고, 이를 허용하는 HV-17 Partial 결정 증거도 올바르지 않습니다.'
    }
  }

  foreach ($role in $requiredArtifactRoles) {
    $entry = @($artifacts | Where-Object { $_.role -eq $role })

    if ($entry.Count -ne 1) {
      Add-Failure 'C11' "capture '$($pipeline.captureId)'의 '$role' 산출물이 정확히 하나 있어야 합니다 (발견: $($entry.Count))."
      continue
    }

    $relativePath = [string]$entry[0].relativePath
    $recordedHash = ([string]$entry[0].sha256).Trim().ToLowerInvariant()

    if ([string]::IsNullOrWhiteSpace($relativePath)) {
      Add-Failure 'C11' "'$role' 산출물 경로가 없습니다."
      continue
    }

    $artifactPath = [System.IO.Path]::GetFullPath((Join-Path $RunRoot $relativePath))
    if (-not $artifactPath.StartsWith($runRootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
      Add-Failure 'C11' "'$role' 산출물 경로가 회차 디렉터리 밖을 가리킵니다: $relativePath"
      continue
    }

    if (-not (Test-Path -LiteralPath $artifactPath -PathType Leaf)) {
      Add-Failure 'C11' "'$role' 산출물 파일이 없습니다: $relativePath"
      continue
    }

    if ((Get-Item -LiteralPath $artifactPath).Length -le 0) {
      Add-Failure 'C11' "'$role' 산출물 파일이 비어 있습니다: $relativePath"
      continue
    }

    if (-not (Test-Sha256Text $recordedHash)) {
      Add-Failure 'C11' "'$role' 산출물 sha256 형식이 올바르지 않습니다."
      continue
    }

    $actualHash = Get-FileSha256 $artifactPath
    if ($actualHash -ne $recordedHash) {
      Add-Failure 'C11' "'$role' 산출물 해시가 증거와 다릅니다. 기록 $recordedHash, 실측 $actualHash"
    }
  }

  $observations['captureId'] = $pipeline.captureId
  $observations['captureArtifactRoles'] = $artifacts.role
}

# --- C3 / C7. 설치본 동일성과 서명 --------------------------------------------

$sealed = Read-JsonOrNull $sealedInventoryPath

if ($null -eq $sealed) {
  Add-Failure 'C3' "봉인된 인벤토리를 읽지 못했습니다: $sealedInventoryPath"
}
else {
  if ($null -eq $sealed.installer) {
    Add-Failure 'C3' '봉인된 인벤토리에 설치본 항목이 없습니다. release:seal 을 실행했는지 확인하세요.'
  }
  elseif (-not (Test-Path -LiteralPath $installerHashPath -PathType Leaf)) {
    Add-Failure 'C3' "VM 에서 다시 계산한 설치본 해시가 없습니다: $installerHashPath"
  }
  else {
    $measured = (Get-Content -LiteralPath $installerHashPath -Raw).Trim().ToLowerInvariant()
    $recorded = ([string]$sealed.installer.sha256).Trim().ToLowerInvariant()

    if (-not (Test-Sha256Text $measured) -or -not (Test-Sha256Text $recorded)) {
      Add-Failure 'C3' '설치본 sha256은 64자리 16진수여야 합니다.'
    }
    elseif ($measured -ne $recorded) {
      Add-Failure 'C3' "설치한 파일이 인벤토리가 가리키는 파일이 아닙니다. 인벤토리 $recorded, 실측 $measured"
    }

    $observations['installerFileName'] = $sealed.installer.fileName
    $observations['installerSha256'] = $recorded
    $observations['installerSizeBytes'] = $sealed.installer.sizeBytes
  }

  if ($sealed.signingStatus -ne 'signed' -and -not (Test-Waived 'C7')) {
    Add-Failure 'C7' "설치본이 서명되지 않았습니다: '$($sealed.signingStatus)'. 미서명 설치본은 release candidate 가 아닙니다."
  }

  $observations['signingStatus'] = $sealed.signingStatus
}

# --- C4. 8단계 lifecycle ------------------------------------------------------

$lifecycle = Read-JsonOrNull $lifecyclePath

if ($null -eq $lifecycle) {
  Add-Failure 'C4' "lifecycle 결과를 읽지 못했습니다: $lifecyclePath"
}
else {
  $recordedSteps = @($lifecycle.steps | ForEach-Object { $_.id })

  foreach ($step in $requiredLifecycleSteps) {
    $entry = @($lifecycle.steps | Where-Object { $_.id -eq $step })

    if ($entry.Count -ne 1) {
      Add-Failure 'C4' "lifecycle 단계 '$step' 기록이 정확히 하나 있어야 합니다 (발견: $($entry.Count))."
      continue
    }

    if ($entry[0].status -ne 'pass') {
      Add-Failure 'C4' "lifecycle 단계 '$step' 이 통과하지 않았습니다: '$($entry[0].status)'"
    }
  }

  foreach ($step in $recordedSteps) {
    if ($requiredLifecycleSteps -notcontains $step) {
      Add-Failure 'C4' "lifecycle 에 정의되지 않은 단계가 있습니다: '$step'"
    }
  }

  $observations['lifecycleSteps'] = $recordedSteps
}

# --- C5 / C6. 데이터 보존 -----------------------------------------------------

$preservation = Read-JsonOrNull $preservationPath

if ($null -eq $preservation) {
  Add-Failure 'C5' "데이터 보존 기록을 읽지 못했습니다: $preservationPath"
}
else {
  # **제거가 고객 사진을 지우면 그 회차는 즉시 No-Go 다.** 되돌릴 수 없는 결함이다.
  if ($preservation.sessionRootSurvivesUninstall -ne $true) {
    Add-Failure 'C5' '제거 후 세션 루트가 남아 있지 않습니다. 고객 사진이 지워졌습니다.'
  }

  if ($preservation.customerPhotoCountAfterUninstall -ne $preservation.customerPhotoCountBeforeUninstall) {
    Add-Failure 'C5' "제거 전후 고객 사진 수가 다릅니다: $($preservation.customerPhotoCountBeforeUninstall) -> $($preservation.customerPhotoCountAfterUninstall)"
  }

  if ($preservation.sessionReadableAfterUpgrade -ne $true) {
    Add-Failure 'C6' '업그레이드 후 이전 세션이 읽히지 않았습니다.'
  }

  if ($preservation.sessionReadableAfterRollback -ne $true) {
    Add-Failure 'C6' '롤백 후 세션이 읽히지 않았습니다. v_b 가 만든 세션이 v_a 에서 읽혀야 합니다.'
  }

  foreach ($removal in @('programFilesRemoved', 'bundledDarktableRemoved', 'helperTreeRemoved', 'startMenuEntryRemoved')) {
    if ($preservation.$removal -ne $true) {
      Add-Failure 'C5' "제거 후에도 남아 있는 것이 있습니다: $removal"
    }
  }

  $observations['sessionRootPath'] = $preservation.sessionRootPath
  $observations['customerPhotoCount'] = $preservation.customerPhotoCountAfterUninstall
}

# --- C6 보강. 업그레이드 후 self-check --------------------------------------

$afterUpgrade = Read-JsonOrNull $afterUpgradePath

if ($null -eq $afterUpgrade) {
  Add-Failure 'C6' "업그레이드 후 self-check 보고서를 읽지 못했습니다: $afterUpgradePath"
}
elseif ($afterUpgrade.overall -ne 'pass') {
  Add-Failure 'C6' "업그레이드 후 self-check 가 통과하지 않았습니다: $($afterUpgrade.overall)"
}
else {
  $sessionChecks = @($afterUpgrade.components | Where-Object { $_.name -eq 'session-compatibility' })
  if ($sessionChecks.Count -ne 1 -or $sessionChecks[0].status -ne 'pass') {
    Add-Failure 'C6' '업그레이드 후 설치된 앱이 이전 세션 manifest를 직접 읽어 통과한 증거가 없습니다.'
  }
}

$afterRollback = Read-JsonOrNull $afterRollbackPath

if ($null -eq $afterRollback) {
  Add-Failure 'C6' "롤백 후 self-check 보고서를 읽지 못했습니다: $afterRollbackPath"
}
elseif ($afterRollback.overall -ne 'pass') {
  Add-Failure 'C6' "롤백 후 self-check 가 통과하지 않았습니다: $($afterRollback.overall)"
}
else {
  $sessionChecks = @($afterRollback.components | Where-Object { $_.name -eq 'session-compatibility' })
  if ($sessionChecks.Count -ne 1 -or $sessionChecks[0].status -ne 'pass') {
    Add-Failure 'C6' '롤백 후 설치된 앱이 이전 세션 manifest를 직접 읽어 통과한 증거가 없습니다.'
  }
}

# --- C8. 네트워크 차단 증거 ---------------------------------------------------

$network = Read-JsonOrNull $networkPath

if (Test-Waived 'C8') {
  $observations['networkValidation'] = 'waived-by-owner'
}
elseif ($null -eq $network) {
  Add-Failure 'C8' "네트워크 차단 증거를 읽지 못했습니다: $networkPath"
}
else {
  if ($network.adaptersDisabled -ne $true) {
    Add-Failure 'C8' '네트워크 어댑터 비활성 기록이 없습니다.'
  }

  $probes = @($network.probes)

  if ($probes.Count -lt 1) {
    Add-Failure 'C8' '연결 시도 기록이 없습니다. 차단은 주장이 아니라 측정이어야 합니다.'
  }

  foreach ($probe in $probes) {
    if ($probe.reachable -ne $false) {
      Add-Failure 'C8' "네트워크가 실제로 차단되지 않았습니다: $($probe.target)"
    }
  }

  $observations['networkProbeCount'] = $probes.Count
}

# --- C9. 사전 미설치 증거 -----------------------------------------------------

$prerequisites = Read-JsonOrNull $prerequisitesPath

if (Test-Waived 'C9') {
  $observations['cleanEnvironmentValidation'] = 'waived-by-owner'
}
elseif ($null -eq $prerequisites) {
  Add-Failure 'C9' "사전 미설치 확인 기록을 읽지 못했습니다: $prerequisitesPath"
}
else {
  foreach ($tool in @('node', 'rust', 'dotnetSdk', 'darktable', 'webview2')) {
    if ($prerequisites.absent.$tool -ne $true) {
      Add-Failure 'C9' "clean VM 이 아닙니다. 설치 전에 이미 있었습니다: $tool"
    }
  }
}

# --- 판정 ---------------------------------------------------------------------

$verdict = if ($failures.Count -gt 0) {
  'No-Go'
}
elseif ($waivedChecks.Count -gt 0) {
  'Go-candidate-with-waivers'
}
else {
  'Go-candidate'
}

Write-Host "HV-18A 기계식 게이트: $verdict"
Write-Host ''

foreach ($key in $observations.Keys) {
  Write-Host ("  {0,-26} {1}" -f $key, ($observations[$key] -join ', '))
}

if ($failures.Count -gt 0) {
  Write-Host ''
  Write-Host "결손 $($failures.Count)건"
  foreach ($failure in $failures) {
    Write-Host "  $failure"
  }
}
else {
  Write-Host ''
  Write-Host '  PASS 는 Go 가 아닙니다. fixture 표시 검토와 승인자 서명은 사람이 합니다.'
}

if ($OutFile) {
  $result = [ordered]@{
    schemaVersion = 'hv-18a-gate/v1'
    verdict       = $verdict
    failures      = @($failures)
    waivers       = if ($null -eq $waivers) { $null } else { $waivers }
    observations  = $observations
  }

  $directory = Split-Path -Parent $OutFile
  if ($directory -and -not (Test-Path -LiteralPath $directory)) {
    New-Item -ItemType Directory -Path $directory -Force | Out-Null
  }

  [System.IO.File]::WriteAllText(
    $OutFile,
    ($result | ConvertTo-Json -Depth 8),
    [System.Text.UTF8Encoding]::new($false))
}

if ($failures.Count -gt 0) {
  exit 1
}

exit 0
