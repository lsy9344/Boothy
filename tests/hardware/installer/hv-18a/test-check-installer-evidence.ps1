<#
.SYNOPSIS
  HV-18A 게이트 자체의 테스트.

.DESCRIPTION
  **통과만 하는 게이트가 되지 않게 한다.** 완전한 회차 fixture 를 만들어 놓고,
  각 조건을 하나씩 깨뜨려 게이트가 실제로 실패하는지 확인한다.

  깨뜨리는 조건:
    C1  self-check 보고서 부재 / overall != pass
    C2  darktable 해석 출처가 bundled-resource 가 아님
    C3  설치본 해시 불일치
    C4  lifecycle 단계 누락 / 미실행
    C5  제거 후 세션 루트 사라짐 / 고객 사진 수 감소
    C6  업그레이드 후 세션 읽히지 않음
    C7  미서명
    C8  네트워크 차단 증거 없음 / 실제로 연결됨
    C9  사전 미설치 증거 없음
    C10 WebView2 버전 미기록
    C11 실제 카메라 촬영 / capture-bound 산출물 증거 누락 또는 불일치

.EXAMPLE
  ./test-check-installer-evidence.ps1
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

# 파이프로 나가는 출력이 콘솔 코드페이지를 타면 운영자가 사유를 읽을 수 없다.
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$gate = Join-Path $PSScriptRoot 'check-installer-evidence.ps1'

$passed = 0
$failed = New-Object System.Collections.Generic.List[string]

function Write-Json {
  param([string]$Path, $Value)

  $directory = Split-Path -Parent $Path
  if (-not (Test-Path -LiteralPath $directory)) {
    New-Item -ItemType Directory -Path $directory -Force | Out-Null
  }

  [System.IO.File]::WriteAllText(
    $Path,
    ($Value | ConvertTo-Json -Depth 10),
    [System.Text.UTF8Encoding]::new($false))
}

function New-PassingRun {
  $root = Join-Path ([System.IO.Path]::GetTempPath()) ("hv18a-" + [System.Guid]::NewGuid().ToString('n'))
  New-Item -ItemType Directory -Path $root -Force | Out-Null

  $digest = 'b' * 64
  $installerHash = 'a' * 64

  $selfCheck = [ordered]@{
    schemaVersion       = 'install-self-check/v1'
    checkedAt           = '2026-08-20T10:20:00Z'
    appVersion          = '0.1.0'
    identifier          = 'com.boothy.booth'
    installRoot         = 'C:\Program Files\Boothy'
    components          = @(
      [ordered]@{ name = 'raw-renderer'; expectedDigest = $digest; actualDigest = $digest; status = 'pass'; reasonCode = $null },
      [ordered]@{ name = 'session-compatibility'; expectedDigest = $null; actualDigest = $null; status = 'pass'; reasonCode = $null }
    )
    webview2Version     = '141.0.3537.85'
    darktableResolution = [ordered]@{
      binary = 'C:\Program Files\Boothy\darktable\bin\darktable-cli.exe'
      source = 'bundled-resource'
    }
    helperVersion       = 'canon-helper 0.1.0'
    overall             = 'pass'
  }

  Write-Json (Join-Path $root 'self-check/install-self-check.json') $selfCheck
  Write-Json (Join-Path $root 'self-check/after-upgrade.json') $selfCheck
  Write-Json (Join-Path $root 'self-check/after-rollback.json') $selfCheck

  $pipelineArtifacts = @()
  foreach ($artifact in @(
    [ordered]@{ role = 'raw-original'; relativePath = 'pipeline/artifacts/capture-001.cr2'; body = 'camera-raw-original' },
    [ordered]@{ role = 'display-proxy'; relativePath = 'pipeline/artifacts/capture-001-proxy.jpg'; body = 'display-proxy' },
    [ordered]@{ role = 'raw-refined'; relativePath = 'pipeline/artifacts/capture-001-refined.jpg'; body = 'raw-refined' },
    [ordered]@{ role = 'final'; relativePath = 'pipeline/artifacts/capture-001-final.jpg'; body = 'final-render' }
  )) {
    $artifactPath = Join-Path $root $artifact.relativePath
    $artifactDirectory = Split-Path -Parent $artifactPath
    New-Item -ItemType Directory -Path $artifactDirectory -Force | Out-Null
    [System.IO.File]::WriteAllText(
      $artifactPath,
      $artifact.body,
      [System.Text.UTF8Encoding]::new($false))
    $pipelineArtifacts += [ordered]@{
      role         = $artifact.role
      relativePath = $artifact.relativePath
      sha256       = (Get-FileHash -Algorithm SHA256 -LiteralPath $artifactPath).Hash.ToLowerInvariant()
    }
  }

  Write-Json (Join-Path $root 'pipeline/capture-evidence.json') ([ordered]@{
      schemaVersion = 'hv-18a-capture-evidence/v1'
      captureId     = 'capture-001'
      requestId     = 'request-001'
      cameraCapture = [ordered]@{ status = 'pass'; source = 'canon-camera' }
      artifacts     = $pipelineArtifacts
    })

  Write-Json (Join-Path $root 'installer/inventory.release.json') ([ordered]@{
      schemaVersion     = 'release-inventory/v1'
      staging           = 'staged'
      appVersion        = '0.1.0'
      identifier        = 'com.boothy.booth'
      installerFileName = 'Boothy_0.1.0_x64-setup.exe'
      signingStatus     = 'signed'
      installer         = [ordered]@{
        fileName  = 'Boothy_0.1.0_x64-setup.exe'
        sha256    = $installerHash
        sizeBytes = 402653184
      }
      components        = @()
    })

  New-Item -ItemType Directory -Path (Join-Path $root 'installer') -Force | Out-Null
  [System.IO.File]::WriteAllText(
    (Join-Path $root 'installer/installer.sha256.txt'),
    $installerHash,
    [System.Text.UTF8Encoding]::new($false))

  Write-Json (Join-Path $root 'lifecycle.json') ([ordered]@{
      schemaVersion = 'hv-18a-lifecycle/v1'
      steps         = @(
        'install', 'launch', 'self-check', 'fixture-display',
        'upgrade', 'rollback', 'uninstall', 'data-preservation'
      ) | ForEach-Object {
        [ordered]@{ id = $_; status = 'pass'; observedAt = '2026-08-20T10:30:00Z'; notes = '' }
      }
    })

  Write-Json (Join-Path $root 'data-preservation.json') ([ordered]@{
      schemaVersion                     = 'hv-18a-data-preservation/v1'
      sessionRootPath                   = 'C:\Users\booth\Pictures\dabi_shoot'
      sessionRootSurvivesUninstall      = $true
      customerPhotoCountBeforeUninstall = 42
      customerPhotoCountAfterUninstall  = 42
      sessionReadableAfterUpgrade       = $true
      sessionReadableAfterRollback      = $true
      programFilesRemoved               = $true
      bundledDarktableRemoved           = $true
      helperTreeRemoved                 = $true
      startMenuEntryRemoved             = $true
    })

  Write-Json (Join-Path $root 'network/connectivity.json') ([ordered]@{
      schemaVersion    = 'hv-18a-network/v1'
      verifiedAt       = '2026-08-20T10:10:00Z'
      adaptersDisabled = $true
      probes           = @(
        [ordered]@{ target = '8.8.8.8'; method = 'Test-NetConnection'; reachable = $false },
        [ordered]@{ target = 'github.com'; method = 'Resolve-DnsName'; reachable = $false }
      )
    })

  Write-Json (Join-Path $root 'prerequisites.json') ([ordered]@{
      schemaVersion = 'hv-18a-prerequisites/v1'
      checkedAt     = '2026-08-20T10:05:00Z'
      absent        = [ordered]@{
        node = $true; rust = $true; dotnetSdk = $true; darktable = $true; webview2 = $true
      }
    })

  return $root
}

function Invoke-Gate {
  param([string]$RunRoot)

  & $gate -RunRoot $RunRoot *> $null
  return $LASTEXITCODE
}

function Assert-Gate {
  param([string]$Case, [int]$Expected, [scriptblock]$Break)

  $root = New-PassingRun
  try {
    if ($Break) {
      & $Break $root
    }

    $actual = Invoke-Gate -RunRoot $root

    if ($actual -eq $Expected) {
      $script:passed++
      Write-Host ("  PASS  {0} -> {1}" -f $Case, $actual)
    }
    else {
      $script:failed.Add(("{0}: 기대 {1}, 실제 {2}" -f $Case, $Expected, $actual)) | Out-Null
      Write-Host ("  FAIL  {0} -> 기대 {1}, 실제 {2}" -f $Case, $Expected, $actual)
    }
  }
  finally {
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
  }
}

function Edit-Json {
  param([string]$Path, [scriptblock]$Mutate)

  $value = Get-Content -LiteralPath $Path -Raw -Encoding UTF8 | ConvertFrom-Json
  & $Mutate $value
  Write-Json $Path $value
}

function Write-ApprovedWaiver {
  param([string]$Root)

  Write-Json (Join-Path $Root 'waivers.json') ([ordered]@{
      schemaVersion  = 'hv-18a-waivers/v1'
      approvedBy     = 'Noah Lee'
      approvedAt     = '2026-08-17T13:22:41+09:00'
      reason         = '현재 PC 기반 내부 검증으로 범위를 변경한다.'
      waivedChecks   = @('C7', 'C8', 'C9')
      waivedEvidence = @(
        'code-signing-certificate',
        'canon-edsdk-redistribution-evidence',
        'clean-offline-environment'
      )
    })
}

Write-Host 'HV-18A 게이트 테스트'
Write-Host ''

Assert-Gate '완전한 회차는 통과' 0 $null

Assert-Gate 'C1 self-check 보고서가 없으면 실패' 1 {
  param($root)
  Remove-Item -LiteralPath (Join-Path $root 'self-check/install-self-check.json') -Force
}

Assert-Gate 'C1 self-check 가 pass 가 아니면 실패' 1 {
  param($root)
  Edit-Json (Join-Path $root 'self-check/install-self-check.json') {
    param($value)
    $value.overall = 'fail'
    $value.components[0].status = 'fail'
    $value.components[0].reasonCode = 'inventory-digest-mismatch'
  }
}

Assert-Gate 'C2 darktable 이 번들에서 해석되지 않으면 실패 (핀 깨짐)' 1 {
  param($root)
  Edit-Json (Join-Path $root 'self-check/install-self-check.json') {
    param($value)
    $value.darktableResolution.source = 'program-files-bin'
  }
}

Assert-Gate 'C3 설치본 해시가 인벤토리와 다르면 실패' 1 {
  param($root)
  [System.IO.File]::WriteAllText(
    (Join-Path $root 'installer/installer.sha256.txt'),
    ('c' * 64),
    [System.Text.UTF8Encoding]::new($false))
}

Assert-Gate 'C3 봉인된 인벤토리가 없으면 실패' 1 {
  param($root)
  Remove-Item -LiteralPath (Join-Path $root 'installer/inventory.release.json') -Force
}

Assert-Gate 'C3 같은 임의 문자열은 sha256 증거로 인정하지 않음' 1 {
  param($root)
  Edit-Json (Join-Path $root 'installer/inventory.release.json') {
    param($value)
    $value.installer.sha256 = 'not-a-sha256'
  }
  [System.IO.File]::WriteAllText(
    (Join-Path $root 'installer/installer.sha256.txt'),
    'not-a-sha256',
    [System.Text.UTF8Encoding]::new($false))
}

Assert-Gate 'C4 lifecycle 단계가 누락되면 실패' 1 {
  param($root)
  Edit-Json (Join-Path $root 'lifecycle.json') {
    param($value)
    $value.steps = @($value.steps | Where-Object { $_.id -ne 'rollback' })
  }
}

Assert-Gate 'C4 미실행 단계가 있으면 실패' 1 {
  param($root)
  Edit-Json (Join-Path $root 'lifecycle.json') {
    param($value)
    ($value.steps | Where-Object { $_.id -eq 'uninstall' }).status = 'not-run'
  }
}

Assert-Gate 'C5 제거 후 세션 루트가 사라지면 실패' 1 {
  param($root)
  Edit-Json (Join-Path $root 'data-preservation.json') {
    param($value)
    $value.sessionRootSurvivesUninstall = $false
  }
}

Assert-Gate 'C5 제거 후 고객 사진이 줄면 실패' 1 {
  param($root)
  Edit-Json (Join-Path $root 'data-preservation.json') {
    param($value)
    $value.customerPhotoCountAfterUninstall = 41
  }
}

Assert-Gate 'C5 제거 후 번들 darktable 이 남으면 실패' 1 {
  param($root)
  Edit-Json (Join-Path $root 'data-preservation.json') {
    param($value)
    $value.bundledDarktableRemoved = $false
  }
}

Assert-Gate 'C6 업그레이드 후 세션이 읽히지 않으면 실패' 1 {
  param($root)
  Edit-Json (Join-Path $root 'data-preservation.json') {
    param($value)
    $value.sessionReadableAfterUpgrade = $false
  }
}

Assert-Gate 'C6 업그레이드 후 self-check 가 없으면 실패' 1 {
  param($root)
  Remove-Item -LiteralPath (Join-Path $root 'self-check/after-upgrade.json') -Force
}

Assert-Gate 'C6 업그레이드 보고서가 이전 세션을 직접 검사하지 않으면 실패' 1 {
  param($root)
  Edit-Json (Join-Path $root 'self-check/after-upgrade.json') {
    param($value)
    $value.components = @($value.components | Where-Object { $_.name -ne 'session-compatibility' })
  }
}

Assert-Gate 'C6 롤백 후 self-check 가 없으면 실패' 1 {
  param($root)
  Remove-Item -LiteralPath (Join-Path $root 'self-check/after-rollback.json') -Force
}

Assert-Gate 'C6 롤백 후 self-check 가 fail 이면 실패' 1 {
  param($root)
  Edit-Json (Join-Path $root 'self-check/after-rollback.json') {
    param($value)
    $value.overall = 'fail'
  }
}

Assert-Gate 'C7 미서명이면 실패' 1 {
  param($root)
  Edit-Json (Join-Path $root 'installer/inventory.release.json') {
    param($value)
    $value.signingStatus = 'unsigned'
  }
}

Assert-Gate 'C8 네트워크 차단 증거가 없으면 실패' 1 {
  param($root)
  Remove-Item -LiteralPath (Join-Path $root 'network/connectivity.json') -Force
}

Assert-Gate 'C8 실제로 연결되었으면 실패' 1 {
  param($root)
  Edit-Json (Join-Path $root 'network/connectivity.json') {
    param($value)
    $value.probes[0].reachable = $true
  }
}

Assert-Gate 'C9 사전 미설치 증거가 없으면 실패' 1 {
  param($root)
  Remove-Item -LiteralPath (Join-Path $root 'prerequisites.json') -Force
}

Assert-Gate 'C9 darktable 이 이미 깔려 있었으면 실패' 1 {
  param($root)
  Edit-Json (Join-Path $root 'prerequisites.json') {
    param($value)
    $value.absent.darktable = $false
  }
}

Assert-Gate '승인된 C7/C8/C9 면제는 미서명·현재 PC 회차를 허용' 0 {
  param($root)
  Write-ApprovedWaiver $root
  Edit-Json (Join-Path $root 'installer/inventory.release.json') {
    param($value)
    $value.signingStatus = 'unsigned'
  }
  Remove-Item -LiteralPath (Join-Path $root 'network/connectivity.json') -Force
  Remove-Item -LiteralPath (Join-Path $root 'prerequisites.json') -Force
}

Assert-Gate '승인자 없는 면제는 실패' 1 {
  param($root)
  Write-Json (Join-Path $root 'waivers.json') ([ordered]@{
      schemaVersion = 'hv-18a-waivers/v1'
      approvedBy    = ''
      approvedAt    = '2026-08-17T13:22:41+09:00'
      reason        = 'invalid'
      waivedChecks  = @('C7', 'C8', 'C9')
    })
}

Assert-Gate 'C10 WebView2 버전이 없으면 실패' 1 {
  param($root)
  Edit-Json (Join-Path $root 'self-check/install-self-check.json') {
    param($value)
    $value.webview2Version = ''
  }
}

Assert-Gate 'C11 실제 촬영 pipeline 증거가 없으면 실패' 1 {
  param($root)
  Remove-Item -LiteralPath (Join-Path $root 'pipeline/capture-evidence.json') -Force
}

Assert-Gate 'C11 실제 Canon 카메라 촬영이 아니면 실패' 1 {
  param($root)
  Edit-Json (Join-Path $root 'pipeline/capture-evidence.json') {
    param($value)
    $value.cameraCapture.source = 'fixture'
  }
}

Assert-Gate 'C11 HV-17 Partial이면 정밀본을 만들지 않고 3개 실제 산출물로 통과' 0 {
  param($root)
  Write-Json (Join-Path $root 'pipeline/hv17-gate.json') ([ordered]@{
      productDecision = [ordered]@{
        verdict = 'Partial'; laneDefaultEnabled = $false; tierDetailVerdict = 'tier-not-justified'
      }
    })
  Edit-Json (Join-Path $root 'pipeline/capture-evidence.json') {
    param($value)
    $value.artifacts = @($value.artifacts | Where-Object { $_.role -ne 'raw-refined' })
    $value | Add-Member -NotePropertyName rawRefinedDisposition -NotePropertyValue ([pscustomobject]@{
        status = 'not-produced'
        reason = 'tier-not-justified'
        hv17GatePath = 'pipeline/hv17-gate.json'
      }) -Force
  }
}

Assert-Gate 'C11 capture-bound 산출물 역할이 빠지면 실패' 1 {
  param($root)
  Edit-Json (Join-Path $root 'pipeline/capture-evidence.json') {
    param($value)
    $value.artifacts = @($value.artifacts | Where-Object { $_.role -ne 'final' })
  }
}

Assert-Gate 'C11 산출물 파일이 없으면 실패' 1 {
  param($root)
  Remove-Item -LiteralPath (Join-Path $root 'pipeline/artifacts/capture-001-refined.jpg') -Force
}

Assert-Gate 'C11 산출물 해시가 다르면 실패' 1 {
  param($root)
  [System.IO.File]::WriteAllText(
    (Join-Path $root 'pipeline/artifacts/capture-001-proxy.jpg'),
    'tampered-proxy',
    [System.Text.UTF8Encoding]::new($false))
}

Write-Host ''

if ($failed.Count -gt 0) {
  Write-Host "FAIL: $($failed.Count)건"
  foreach ($failure in $failed) {
    Write-Host "  $failure"
  }
  exit 1
}

Write-Host "PASS: $passed 건 모두 기대한 대로 판정했습니다."
exit 0
