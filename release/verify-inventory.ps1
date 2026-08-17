<#
.SYNOPSIS
  Story 7.7 기계식 게이트 — 인벤토리 명세와 실제 staged 페이로드를 대조한다.

.DESCRIPTION
  **생성기가 자기 산출물을 통과시키는 게이트가 되면 안 된다.** 그래서 이 스크립트는
  `release/build-inventory.ts`와 독립적으로 디스크를 다시 읽고 해시를 다시 계산한다.

  세 가지 결손을 **서로 다른 종료 코드**로 낸다. 한 덩어리 "검증 실패"로 뭉치면
  운영자가 무엇을 해야 할지 알 수 없다 (AC 2의 actionable operator result).

    0  pass
    2  inventory-component-missing        명세에 있는데 staged 트리에 없다
    3  inventory-digest-mismatch          있는데 해시가 다르다
    4  inventory-unexpected-component     명세에 없는데 들어왔다
    5  inventory-not-staged               release:stage 가 돌지 않았다
    6  inventory-manifest-unreadable      인벤토리 파일이 없거나 읽히지 않는다
    7  inventory-pin-missing              -RequirePins 인데 핀이 비어 있다

  여러 결손이 동시에 있으면 위 순서의 **역순 우선순위**로 하나를 고른다:
  6 > 5 > 2 > 3 > 4 > 7. 모든 결손은 항상 전부 출력한다 — 종료 코드만 하나다.

.PARAMETER RepoRoot
  저장소 루트. 기본값은 이 스크립트의 부모 디렉터리.

.PARAMETER InventoryPath
  검사할 인벤토리. 기본값은 `<RepoRoot>/release/dist/inventory.json`.

.PARAMETER SpecPath
  명세. 기본값은 `<RepoRoot>/release/inventory-spec.json`.

.PARAMETER RequirePins
  릴리스 경로용. `requiresPin: true`인 구성요소의 `pinnedStagedTreeDigest`가 비어 있으면 실패시킨다.
  **조달된 페이로드만 핀을 요구한다.** 이 저장소가 빌드하는 산출물은 매 빌드 바이트가
  달라지므로 핀을 걸면 재빌드가 영원히 통과하지 못한다.

.EXAMPLE
  ./verify-inventory.ps1
  ./verify-inventory.ps1 -RequirePins
#>
[CmdletBinding()]
param(
  [string]$RepoRoot,
  [string]$InventoryPath,
  [string]$SpecPath,
  [switch]$RequirePins
)

$ErrorActionPreference = 'Stop'

# 파이프로 나가는 출력이 콘솔 코드페이지를 타면 운영자가 사유를 읽을 수 없다.
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

if (-not $RepoRoot) {
  $RepoRoot = Split-Path -Parent $PSScriptRoot
}
if (-not $InventoryPath) {
  $InventoryPath = Join-Path $RepoRoot 'release/dist/inventory.json'
}
if (-not $SpecPath) {
  $SpecPath = Join-Path $RepoRoot 'release/inventory-spec.json'
}

$exitComponentMissing = 2
$exitDigestMismatch = 3
$exitUnexpectedComponent = 4
$exitNotStaged = 5
$exitManifestUnreadable = 6
$exitPinMissing = 7

$notStagedMarker = 'PAYLOAD-NOT-STAGED.md'

$failures = New-Object System.Collections.Generic.List[string]
$observedCodes = New-Object System.Collections.Generic.HashSet[int]

function Add-Failure {
  param([int]$Code, [string]$ReasonCode, [string]$Message)

  $failures.Add(("[{0}] {1}" -f $ReasonCode, $Message)) | Out-Null
  $observedCodes.Add($Code) | Out-Null
}

function Get-FileSha256 {
  param([string]$Path)

  return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Get-Utf8SortKey {
  param([string]$Value)

  # UTF-8 바이트 순서로 정렬하기 위한 키.
  # 각 바이트를 같은 코드 포인트의 문자로 옮기면 ordinal 비교가 곧 바이트 비교가 된다.
  # TS(Buffer.compare)와 Rust(String Ord)가 쓰는 순서와 정확히 같아야 digest가 일치한다.
  # `[Encoding]::Latin1` 은 .NET 5 부터라 Windows PowerShell 5.1 에는 없다. 코드 페이지로 부른다.
  return [System.Text.Encoding]::GetEncoding(28591).GetString([System.Text.Encoding]::UTF8.GetBytes($Value))
}

function Get-TreeDigest {
  param([string[]]$Lines)

  # **`Sort-Object`를 쓰지 않는다.** 기본 비교자가 culture-aware라서 `.`과 `-`이 섞인 실제
  # 어셈블리 이름들을 TS/Rust와 다른 순서로 늘어놓는다. 작은 fixture에서는 두 순서가 같아
  # 보이지만, 311개짜리 실제 helper 트리에서 digest가 갈라졌다.
  # `[Array]::Sort` + `StringComparer.Ordinal`은 Latin1 키에서 곧 UTF-8 바이트 순서다.
  $keys = [string[]]@($Lines | ForEach-Object { Get-Utf8SortKey $_ })
  $sorted = [string[]]@($Lines)
  [Array]::Sort($keys, $sorted, [System.StringComparer]::Ordinal)

  $joined = [string]::Join("`n", $sorted)
  $bytes = [System.Text.Encoding]::UTF8.GetBytes($joined)
  $sha256 = [System.Security.Cryptography.SHA256]::Create()

  try {
    return -join ($sha256.ComputeHash($bytes) | ForEach-Object { $_.ToString('x2') })
  }
  finally {
    $sha256.Dispose()
  }
}

function Get-StagedRelativeFiles {
  param([string]$Root, $Stage)

  if (-not (Test-Path -LiteralPath $Root -PathType Container)) {
    return @()
  }

  $excluded = @()
  if ($Stage.PSObject.Properties.Name -contains 'exclude' -and $Stage.exclude) {
    $excluded = @($Stage.exclude)
  }

  if ($Stage.PSObject.Properties.Name -contains 'include' -and $Stage.include) {
    $relative = @($Stage.include | Where-Object { Test-Path -LiteralPath (Join-Path $Root $_) -PathType Leaf })
  }
  else {
    $prefixLength = (Resolve-Path -LiteralPath $Root).Path.Length + 1
    $relative = @(
      Get-ChildItem -LiteralPath $Root -Recurse -File -Force |
      ForEach-Object { $_.FullName.Substring($prefixLength).Replace('\', '/') }
    )
  }

  return @($relative | Where-Object { $excluded -notcontains $_ })
}

# --- 1. 인벤토리 매니페스트를 읽는다 -----------------------------------------

if (-not (Test-Path -LiteralPath $SpecPath -PathType Leaf)) {
  Write-Host "[inventory-manifest-unreadable] 명세를 찾지 못했습니다: $SpecPath"
  exit $exitManifestUnreadable
}

if (-not (Test-Path -LiteralPath $InventoryPath -PathType Leaf)) {
  Write-Host "[inventory-manifest-unreadable] 인벤토리를 찾지 못했습니다: $InventoryPath"
  Write-Host "  먼저 'pnpm release:stage' 를 실행하세요."
  exit $exitManifestUnreadable
}

try {
  $spec = Get-Content -LiteralPath $SpecPath -Raw -Encoding UTF8 | ConvertFrom-Json
  $inventory = Get-Content -LiteralPath $InventoryPath -Raw -Encoding UTF8 | ConvertFrom-Json
}
catch {
  Write-Host "[inventory-manifest-unreadable] 문서를 파싱하지 못했습니다: $($_.Exception.Message)"
  exit $exitManifestUnreadable
}

if ($inventory.schemaVersion -ne 'release-inventory/v1') {
  Write-Host "[inventory-manifest-unreadable] 알 수 없는 스키마 버전: $($inventory.schemaVersion)"
  exit $exitManifestUnreadable
}

if ($inventory.staging -ne 'staged') {
  Write-Host "[inventory-not-staged] $($inventory.reason)"
  Write-Host '  이 빌드는 release candidate가 아닙니다.'
  exit $exitNotStaged
}

# --- 2. 명세의 구성요소를 하나씩 대조한다 -------------------------------------

$specRoles = @($spec.components | ForEach-Object { $_.role })
$inventoryRoles = @($inventory.components | ForEach-Object { $_.role })

foreach ($specComponent in $spec.components) {
  $role = $specComponent.role
  $recorded = @($inventory.components | Where-Object { $_.role -eq $role })

  if ($recorded.Count -ne 1) {
    Add-Failure $exitComponentMissing 'inventory-component-missing' `
      "인벤토리에 '$role' 항목이 정확히 하나 있어야 합니다 (발견: $($recorded.Count))."
    continue
  }

  $recorded = $recorded[0]

  if ($specComponent.expectedStatus -ne $recorded.status) {
    if ($recorded.status -eq 'missing') {
      Add-Failure $exitComponentMissing 'inventory-component-missing' `
        "'$role' 이 staged 되지 않았습니다: $($recorded.rationale)"
    }
    else {
      Add-Failure $exitComponentMissing 'inventory-component-missing' `
        "'$role' 상태가 명세와 다릅니다. 기대 '$($specComponent.expectedStatus)', 기록 '$($recorded.status)'."
    }
    continue
  }

  if ($specComponent.pinnedSourceArchiveSha256 -and
      $recorded.sourceArchiveSha256 -ne $specComponent.pinnedSourceArchiveSha256) {
    Add-Failure $exitDigestMismatch 'inventory-digest-mismatch' `
      "'$role' 원본 아카이브 해시가 핀과 다릅니다. 핀 $($specComponent.pinnedSourceArchiveSha256), 기록 $($recorded.sourceArchiveSha256)."
  }

  if ($recorded.status -ne 'present') {
    continue
  }

  # --- 3. 디스크를 다시 읽어 해시를 다시 계산한다 -----------------------------

  $stage = $specComponent.stage
  $root = Join-Path $RepoRoot ($stage.root -replace '/', [System.IO.Path]::DirectorySeparatorChar)

  if (Test-Path -LiteralPath (Join-Path $root $notStagedMarker)) {
    Add-Failure $exitComponentMissing 'inventory-component-missing' `
      "'$role' 트리에 $notStagedMarker 표시가 있습니다. 페이로드가 조달되지 않았습니다."
    continue
  }

  $requiredEntries = @()
  if ($stage.PSObject.Properties.Name -contains 'requiredEntries' -and $stage.requiredEntries) {
    $requiredEntries = @($stage.requiredEntries)
  }

  foreach ($entry in $requiredEntries) {
    $entryPath = Join-Path $root ($entry -replace '/', [System.IO.Path]::DirectorySeparatorChar)
    if (-not (Test-Path -LiteralPath $entryPath)) {
      Add-Failure $exitComponentMissing 'inventory-component-missing' `
        "'$role' 트리에 필수 항목이 없습니다: $entry"
    }
  }

  $relativeFiles = Get-StagedRelativeFiles -Root $root -Stage $stage

  if ($relativeFiles.Count -eq 0) {
    Add-Failure $exitComponentMissing 'inventory-component-missing' `
      "'$role' 트리에 파일이 없습니다: $($stage.root)"
    continue
  }

  $lines = @(
    $relativeFiles | ForEach-Object {
      '{0}:{1}' -f $_, (Get-FileSha256 (Join-Path $root ($_ -replace '/', [System.IO.Path]::DirectorySeparatorChar)))
    }
  )
  $actualDigest = Get-TreeDigest -Lines $lines

  if ($actualDigest -ne $recorded.stagedTreeDigest) {
    Add-Failure $exitDigestMismatch 'inventory-digest-mismatch' `
      "'$role' 트리 해시가 인벤토리와 다릅니다. 인벤토리 $($recorded.stagedTreeDigest), 실측 $actualDigest."
  }

  if ($relativeFiles.Count -ne $recorded.fileCount) {
    Add-Failure $exitDigestMismatch 'inventory-digest-mismatch' `
      "'$role' 파일 수가 인벤토리와 다릅니다. 인벤토리 $($recorded.fileCount), 실측 $($relativeFiles.Count)."
  }

  if ($specComponent.pinnedStagedTreeDigest) {
    if ($actualDigest -ne $specComponent.pinnedStagedTreeDigest) {
      Add-Failure $exitDigestMismatch 'inventory-digest-mismatch' `
        "'$role' 트리 해시가 명세의 핀과 다릅니다. 핀 $($specComponent.pinnedStagedTreeDigest), 실측 $actualDigest."
    }
  }
  elseif ($RequirePins -and $specComponent.requiresPin -eq $true) {
    # **핀을 요구하는 것은 조달된 페이로드뿐이다.** 이 저장소가 빌드하는 산출물은
    # 매 빌드 바이트가 달라지므로(PE 타임스탬프 등) 핀을 걸면 재빌드가 통과할 수 없다.
    Add-Failure $exitPinMissing 'inventory-pin-missing' `
      "'$role' 의 pinnedStagedTreeDigest 가 비어 있습니다. 실측값 $actualDigest 을 release/inventory-spec.json 에 기록하세요."
  }
}

# --- 4. 명세에 없는데 들어온 것을 찾는다 --------------------------------------

foreach ($role in $inventoryRoles) {
  if ($specRoles -notcontains $role) {
    Add-Failure $exitUnexpectedComponent 'inventory-unexpected-component' `
      "인벤토리에 명세가 모르는 구성요소가 있습니다: '$role'"
  }
}

# staged 트리에 있으면서 어떤 구성요소도 자기 것이라고 하지 않은 파일.
# **설치본에 들어갔는데 인벤토리에 없으면 실패다.** 누락과 같은 심각도다.
$stageRoots = @(
  $spec.components |
  Where-Object { $_.stage } |
  ForEach-Object { $_.stage.root } |
  Sort-Object -Unique
)

foreach ($stageRoot in $stageRoots) {
  $root = Join-Path $RepoRoot ($stageRoot -replace '/', [System.IO.Path]::DirectorySeparatorChar)

  if (-not (Test-Path -LiteralPath $root -PathType Container)) {
    continue
  }

  $prefixLength = (Resolve-Path -LiteralPath $root).Path.Length + 1
  $onDisk = @(
    Get-ChildItem -LiteralPath $root -Recurse -File |
    ForEach-Object { $_.FullName.Substring($prefixLength).Replace('\', '/') }
  )

  $claimed = New-Object System.Collections.Generic.HashSet[string]
  foreach ($specComponent in $spec.components) {
    if (-not $specComponent.stage -or $specComponent.stage.root -ne $stageRoot) {
      continue
    }

    foreach ($relative in (Get-StagedRelativeFiles -Root $root -Stage $specComponent.stage)) {
      $claimed.Add($relative) | Out-Null
    }
  }

  foreach ($relative in $onDisk) {
    if (-not $claimed.Contains($relative)) {
      Add-Failure $exitUnexpectedComponent 'inventory-unexpected-component' `
        "'$stageRoot' 에 어떤 구성요소도 자기 것이라고 하지 않은 파일이 있습니다: $relative"
    }
  }
}

# --- 5. 판정 ------------------------------------------------------------------

if ($failures.Count -eq 0) {
  Write-Host 'PASS: 인벤토리 명세와 staged 페이로드가 일치합니다.'
  Write-Host "  구성요소 $($inventory.components.Count)개, 서명 상태 $($inventory.signingStatus)"

  if ($inventory.signingStatus -ne 'signed') {
    Write-Host '  주의: 미서명 설치본은 HV-18A Go 의 근거가 아닙니다.'
  }

  exit 0
}

Write-Host "FAIL: 결손 $($failures.Count)건"
foreach ($failure in $failures) {
  Write-Host "  $failure"
}

foreach ($code in @($exitManifestUnreadable, $exitNotStaged, $exitComponentMissing, $exitDigestMismatch, $exitUnexpectedComponent, $exitPinMissing)) {
  if ($observedCodes.Contains($code)) {
    Write-Host ''
    Write-Host "종료 코드 $code"
    exit $code
  }
}

exit 1
