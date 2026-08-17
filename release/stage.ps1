<#
.SYNOPSIS
  Story 7.7 — 오프라인 페이로드를 staging 하고 인벤토리를 생성한다.

.DESCRIPTION
  순서:
    1. 카메라 helper 를 self-contained 로 publish 한다
    2. publish 결과가 **실제로 실행되는지** 확인한다 (--version, --self-check)
    3. 동봉할 darktable 트리가 정말 5.4.1 인지 확인한다
    4. release/dist/inventory.json 을 생성한다

  **판정하지 않는다.** 결손은 인벤토리에 그대로 기록되고, 릴리스를 막는 것은
  `release/verify-inventory.ps1` 의 몫이다. staging 과 판정을 한 곳에 두면
  staging 이 자기 산출물을 통과시키는 게이트가 된다.

  빌드는 됐는데 실행이 안 되는 산출물은 인벤토리에 들어가지 않는다. 그런 경우
  `PAYLOAD-NOT-STAGED.md` 표시를 남겨 생성기가 `missing` 으로 기록하게 한다.

.PARAMETER SkipHelperPublish
  이미 publish 된 트리를 그대로 쓴다. 반복 실행 시 시간을 아끼기 위한 것이고,
  검증 단계는 건너뛰지 않는다.

.EXAMPLE
  ./stage.ps1
  ./stage.ps1 -SkipHelperPublish
#>
[CmdletBinding()]
param(
  [switch]$SkipHelperPublish
)

$ErrorActionPreference = 'Stop'

# 파이프로 나가는 출력이 콘솔 코드페이지를 타면 운영자가 사유를 읽을 수 없다.
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$repoRoot = Split-Path -Parent $PSScriptRoot
$helperProject = Join-Path $repoRoot 'sidecar/canon-helper/src/CanonHelper/CanonHelper.csproj'
$helperOutput = Join-Path $repoRoot 'release/dist/canon-helper'
$darktableRoot = Join-Path $repoRoot 'release/vendor/darktable-5.4.1'
$generator = Join-Path $PSScriptRoot 'build-inventory.ts'
$pinnedDarktableVersion = '5.4.1'
$notStagedMarker = 'PAYLOAD-NOT-STAGED.md'

function Write-Marker {
  param([string]$Directory, [string]$Component, [string]$Reason)

  New-Item -ItemType Directory -Path $Directory -Force | Out-Null
  $body = @"
# 이 자리는 아직 채워지지 않았습니다

- 구성요소: ``$Component``
- 사유: $Reason

이 파일이 설치본 안에서 발견되면 그 설치본은 release candidate가 아닙니다.
``release:verify`` 가 ``inventory-component-missing`` 으로 막고,
``boothy.exe --self-check`` 가 종료 코드 1로 실패합니다.

채우는 방법은 ``release/README.md`` 에 있습니다.
"@
  [System.IO.File]::WriteAllText(
    (Join-Path $Directory $notStagedMarker),
    $body,
    [System.Text.UTF8Encoding]::new($false))
}

function Remove-Marker {
  param([string]$Directory)

  $marker = Join-Path $Directory $notStagedMarker
  if (Test-Path -LiteralPath $marker) {
    Remove-Item -LiteralPath $marker -Force
  }
}

Write-Host '=== Story 7.7 release staging ==='
Write-Host ''

# --- 1. 카메라 helper --------------------------------------------------------

Write-Host '[1/4] 카메라 helper self-contained publish'

$helperFailure = $null

if ($SkipHelperPublish) {
  Write-Host '  publish 를 건너뜁니다 (-SkipHelperPublish).'
}
else {
  Remove-Marker -Directory $helperOutput

  & dotnet publish $helperProject -c Release -r win-x64 --self-contained true -o $helperOutput

  if ($LASTEXITCODE -ne 0) {
    $helperFailure = "dotnet publish 가 종료 코드 $LASTEXITCODE 로 실패했습니다. EDSDK 페이로드가 있는지 release/README.md 를 확인하세요."
  }
}

if (-not $helperFailure) {
  $helperExe = Join-Path $helperOutput 'canon-helper.exe'

  if (-not (Test-Path -LiteralPath $helperExe)) {
    $helperFailure = "publish 결과에 canon-helper.exe 가 없습니다: $helperOutput"
  }
  else {
    foreach ($required in @('EDSDK.dll', 'EdsImage.dll', 'hostfxr.dll', 'hostpolicy.dll', 'System.Private.CoreLib.dll')) {
      if (-not (Test-Path -LiteralPath (Join-Path $helperOutput $required))) {
        $helperFailure = "publish 결과에 $required 가 없습니다. self-contained publish 인지 확인하세요."
        break
      }
    }
  }
}

if (-not $helperFailure) {
  # **빌드는 됐는데 실행이 안 되는 산출물을 담지 않는다.**
  & (Join-Path $helperOutput 'canon-helper.exe') --version | Write-Host

  if ($LASTEXITCODE -ne 0) {
    $helperFailure = "canon-helper.exe --version 이 종료 코드 $LASTEXITCODE 로 실패했습니다."
  }
  else {
    & (Join-Path $helperOutput 'canon-helper.exe') --self-check --sdk-root $helperOutput | Write-Host

    if ($LASTEXITCODE -ne 0) {
      $helperFailure = "canon-helper.exe --self-check 가 종료 코드 $LASTEXITCODE 로 실패했습니다."
    }
  }
}

if ($helperFailure) {
  Write-Host "  결손: $helperFailure"
  Write-Marker -Directory $helperOutput -Component 'camera-helper / edsdk-runtime' -Reason $helperFailure
}
else {
  # 이전 실패가 남긴 marker는 실제 payload 검증이 성공한 시점에만 제거한다.
  # `-SkipHelperPublish` 복구 경로도 동일해야 한다.
  Remove-Marker -Directory $helperOutput
  Write-Host '  helper 트리가 실행 검증까지 통과했습니다.'
}

Write-Host ''

# --- 2. darktable ------------------------------------------------------------

Write-Host '[2/4] 동봉할 darktable 트리 확인'

$darktableFailure = $null
$darktableCli = Join-Path $darktableRoot 'bin/darktable-cli.exe'

foreach ($required in @('bin/darktable-cli.exe', 'lib', 'share/darktable')) {
  if (-not (Test-Path -LiteralPath (Join-Path $darktableRoot $required))) {
    $darktableFailure = "darktable 트리에 $required 가 없습니다. bin/ lib/ share/darktable/ 이 형제로 있어야 합니다."
    break
  }
}

if (-not $darktableFailure) {
  $versionOutput = (& $darktableCli --version 2>&1 | Out-String)
  Write-Host ("  " + ($versionOutput -split "`n")[0].Trim())

  if ($versionOutput -notmatch [regex]::Escape($pinnedDarktableVersion)) {
    $darktableFailure = "darktable 이 $pinnedDarktableVersion 이 아닙니다. HV-15 가 검증한 렌더 인자는 이 버전에서만 성립합니다."
  }
}

if ($darktableFailure) {
  Write-Host "  결손: $darktableFailure"
  Write-Marker -Directory $darktableRoot -Component 'raw-renderer (darktable 5.4.1)' -Reason $darktableFailure
}
else {
  Remove-Marker -Directory $darktableRoot
  Write-Host "  darktable $pinnedDarktableVersion 트리가 확인되었습니다."
}

Write-Host ''

# --- 3. 인벤토리 생성 ---------------------------------------------------------

Write-Host '[3/4] 인벤토리 생성'
& node $generator

if ($LASTEXITCODE -ne 0) {
  Write-Host "인벤토리 생성이 종료 코드 $LASTEXITCODE 로 실패했습니다."
  exit 1
}

Write-Host ''

# --- 4. 다음 단계 -------------------------------------------------------------

Write-Host '[4/4] staging 완료'
Write-Host '  다음: pnpm release:verify'

if ($helperFailure -or $darktableFailure) {
  Write-Host ''
  Write-Host '  결손이 인벤토리에 기록되었습니다. release:verify 가 이 빌드를 막습니다.'
}

exit 0
