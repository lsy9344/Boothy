<#
.SYNOPSIS
  Story 7.7 — `verify-inventory.ps1` 게이트 자체의 테스트.

.DESCRIPTION
  **통과만 하는 게이트가 되지 않게 한다.** 각 결손을 하나씩 인공적으로 만들어
  게이트가 정말 그 사유, 그 종료 코드로 실패하는지 확인한다.

  인벤토리는 손으로 짓지 않고 `release/build-inventory.ts` 가 실제로 생성한다.
  그래야 TS 쪽 해시 계산과 PowerShell 쪽 재계산이 **같은 값**을 내는지도 함께 증명된다.
  둘이 갈라지면 릴리스 경로가 항상 digest-mismatch 로 막힌다.

.EXAMPLE
  ./test-verify-inventory.ps1
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

# 파이프로 나가는 출력이 콘솔 코드페이지를 타면 운영자가 사유를 읽을 수 없다.
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$repoRoot = Split-Path -Parent $PSScriptRoot
$verifyScript = Join-Path $PSScriptRoot 'verify-inventory.ps1'
$generator = Join-Path $PSScriptRoot 'build-inventory.ts'

$passed = 0
$failed = New-Object System.Collections.Generic.List[string]

function Write-Utf8NoBom {
  param([string]$Path, [string]$Content)

  # Windows PowerShell 5.1 의 `Set-Content -Encoding utf8` 은 BOM 을 붙인다.
  # 계약 문서에 BOM 이 섞이면 다른 도구가 파싱에 실패한다.
  [System.IO.File]::WriteAllText($Path, $Content, [System.Text.UTF8Encoding]::new($false))
}

function New-Workspace {
  $workspace = Join-Path ([System.IO.Path]::GetTempPath()) ("boothy-verify-" + [System.Guid]::NewGuid().ToString('n'))
  New-Item -ItemType Directory -Path $workspace -Force | Out-Null
  New-Item -ItemType Directory -Path (Join-Path $workspace 'release/dist') -Force | Out-Null
  New-Item -ItemType Directory -Path (Join-Path $workspace 'payload/bin') -Force | Out-Null
  New-Item -ItemType Directory -Path (Join-Path $workspace 'payload/lib') -Force | Out-Null

  Set-Content -LiteralPath (Join-Path $workspace 'payload/bin/tool.exe') -Value 'tool' -Encoding utf8 -NoNewline
  Set-Content -LiteralPath (Join-Path $workspace 'payload/lib/plugin.dll') -Value 'plugin' -Encoding utf8 -NoNewline

  # **culture-aware 정렬과 ordinal 정렬이 갈라지는 이름들.**
  # 실제 .NET publish 트리가 이런 이름으로 가득하고, 311개짜리 실제 트리에서
  # `Sort-Object` 기본 비교자가 TS/Rust 와 다른 순서를 냈다. 작은 fixture 로는 안 잡혔다.
  # 이 이름들이 있으면 정렬이 다시 culture-aware 로 돌아가는 순간 digest 가 갈라진다.
  foreach ($name in @(
      'System.Private.CoreLib.dll',
      'System-Private.dll',
      'SystemaPrivate.dll',
      'a-b.dll',
      'a.b.dll',
      'ab.dll',
      '_leading.dll',
      'Z.dll',
      'z.dll'
    )) {
    Set-Content -LiteralPath (Join-Path $workspace "payload/lib/$name") -Value $name -Encoding utf8 -NoNewline
  }

  return $workspace
}

function Write-Spec {
  param(
    [string]$Workspace,
    [hashtable]$Overrides = @{}
  )

  $component = [ordered]@{
    name                       = 'fixture payload'
    role                       = 'raw-renderer'
    expectedStatus             = 'present'
    version                    = '5.4.1'
    origin                     = 'fixture'
    license                    = 'GPL-3.0-or-later'
    licenseEvidencePath        = 'release/licenses/darktable-5.4.1.md'
    installRelativePath        = 'darktable/'
    signingStatus              = 'not-applicable'
    rationale                  = $null
    stage                      = [ordered]@{
      root            = 'payload'
      requiredEntries = @('bin/tool.exe')
    }
    requiresPin                = $true
    pinnedSourceArchiveSha256  = $null
    pinnedStagedTreeDigest     = $null
  }

  foreach ($key in $Overrides.Keys) {
    $component[$key] = $Overrides[$key]
  }

  $spec = [ordered]@{
    schemaVersion = 'release-inventory-spec/v1'
    components    = @($component)
  }

  $specPath = Join-Path $Workspace 'release/inventory-spec.json'
  Set-Content -LiteralPath $specPath -Value ($spec | ConvertTo-Json -Depth 8) -Encoding utf8
  return $specPath
}

function Invoke-Generator {
  param([string]$Workspace, [string]$SpecPath)

  $inventoryPath = Join-Path $Workspace 'release/dist/inventory.json'
  & node $generator --repo-root $Workspace --spec $SpecPath --out $inventoryPath | Out-Null

  if ($LASTEXITCODE -ne 0) {
    throw "generator failed with exit code $LASTEXITCODE"
  }

  return $inventoryPath
}

function Invoke-Verify {
  param([string]$Workspace, [string]$SpecPath, [string]$InventoryPath, [switch]$RequirePins)

  $arguments = @{
    RepoRoot      = $Workspace
    SpecPath      = $SpecPath
    InventoryPath = $InventoryPath
  }

  if ($RequirePins) {
    & $verifyScript @arguments -RequirePins *> $null
  }
  else {
    & $verifyScript @arguments *> $null
  }

  return $LASTEXITCODE
}

function Assert-ExitCode {
  param([string]$Case, [int]$Expected, [int]$Actual)

  if ($Expected -eq $Actual) {
    $script:passed++
    Write-Host ("  PASS  {0} -> {1}" -f $Case, $Actual)
  }
  else {
    $script:failed.Add(("{0}: 기대 {1}, 실제 {2}" -f $Case, $Expected, $Actual)) | Out-Null
    Write-Host ("  FAIL  {0} -> 기대 {1}, 실제 {2}" -f $Case, $Expected, $Actual)
  }
}

Write-Host 'verify-inventory.ps1 게이트 테스트'
Write-Host ''

# --- 0. 정상 회차는 통과한다 --------------------------------------------------

$workspace = New-Workspace
try {
  $specPath = Write-Spec -Workspace $workspace
  $inventoryPath = Invoke-Generator -Workspace $workspace -SpecPath $specPath
  Assert-ExitCode 'staged 트리와 인벤토리가 일치하면 통과' 0 (Invoke-Verify -Workspace $workspace -SpecPath $specPath -InventoryPath $inventoryPath)
}
finally {
  Remove-Item -LiteralPath $workspace -Recurse -Force -ErrorAction SilentlyContinue
}

# --- 2. 명세에 있는데 없다 ----------------------------------------------------

$workspace = New-Workspace
try {
  $specPath = Write-Spec -Workspace $workspace
  $inventoryPath = Invoke-Generator -Workspace $workspace -SpecPath $specPath
  Remove-Item -LiteralPath (Join-Path $workspace 'payload') -Recurse -Force
  Assert-ExitCode 'staged 트리가 통째로 사라지면 inventory-component-missing' 2 (Invoke-Verify -Workspace $workspace -SpecPath $specPath -InventoryPath $inventoryPath)
}
finally {
  Remove-Item -LiteralPath $workspace -Recurse -Force -ErrorAction SilentlyContinue
}

$workspace = New-Workspace
try {
  $specPath = Write-Spec -Workspace $workspace
  $inventoryPath = Invoke-Generator -Workspace $workspace -SpecPath $specPath
  Set-Content -LiteralPath (Join-Path $workspace 'payload/PAYLOAD-NOT-STAGED.md') -Value '# 미조달' -Encoding utf8
  Assert-ExitCode '결손 표시 파일이 있으면 inventory-component-missing' 2 (Invoke-Verify -Workspace $workspace -SpecPath $specPath -InventoryPath $inventoryPath)
}
finally {
  Remove-Item -LiteralPath $workspace -Recurse -Force -ErrorAction SilentlyContinue
}

# --- 3. 있는데 해시가 다르다 --------------------------------------------------

$workspace = New-Workspace
try {
  $specPath = Write-Spec -Workspace $workspace
  $inventoryPath = Invoke-Generator -Workspace $workspace -SpecPath $specPath
  Set-Content -LiteralPath (Join-Path $workspace 'payload/lib/plugin.dll') -Value 'tampered' -Encoding utf8 -NoNewline
  Assert-ExitCode '파일 하나가 바뀌면 inventory-digest-mismatch' 3 (Invoke-Verify -Workspace $workspace -SpecPath $specPath -InventoryPath $inventoryPath)
}
finally {
  Remove-Item -LiteralPath $workspace -Recurse -Force -ErrorAction SilentlyContinue
}

$workspace = New-Workspace
try {
  $specPath = Write-Spec -Workspace $workspace
  $inventoryPath = Invoke-Generator -Workspace $workspace -SpecPath $specPath
  $wrongPin = 'f' * 64
  $specPath = Write-Spec -Workspace $workspace -Overrides @{ pinnedStagedTreeDigest = $wrongPin }
  Assert-ExitCode '명세의 핀과 실측이 다르면 inventory-digest-mismatch' 3 (Invoke-Verify -Workspace $workspace -SpecPath $specPath -InventoryPath $inventoryPath)
}
finally {
  Remove-Item -LiteralPath $workspace -Recurse -Force -ErrorAction SilentlyContinue
}

# --- 4. 명세에 없는데 들어왔다 ------------------------------------------------

$workspace = New-Workspace
try {
  $specPath = Write-Spec -Workspace $workspace
  $inventoryPath = Invoke-Generator -Workspace $workspace -SpecPath $specPath

  $inventory = Get-Content -LiteralPath $inventoryPath -Raw -Encoding UTF8 | ConvertFrom-Json
  $extra = $inventory.components[0].PSObject.Copy()
  $extra.role = 'display-renderer'
  $extra.status = 'not-applicable'
  $extra.stagedTreeDigest = $null
  $extra.rationale = '명세가 모르는 항목'
  $inventory.components = @($inventory.components[0], $extra)
  Set-Content -LiteralPath $inventoryPath -Value ($inventory | ConvertTo-Json -Depth 8) -Encoding utf8

  Assert-ExitCode '명세가 모르는 구성요소가 있으면 inventory-unexpected-component' 4 (Invoke-Verify -Workspace $workspace -SpecPath $specPath -InventoryPath $inventoryPath)
}
finally {
  Remove-Item -LiteralPath $workspace -Recurse -Force -ErrorAction SilentlyContinue
}

$workspace = New-Workspace
try {
  # include 로 파일을 명시한 구성요소는 나머지 파일을 자기 것이라고 하지 않는다.
  # 그 파일들은 "설치본에 들어갔는데 인벤토리에 없는" 상태다.
  $specPath = Write-Spec -Workspace $workspace -Overrides @{
    stage = [ordered]@{
      root            = 'payload'
      include         = @('bin/tool.exe')
      requiredEntries = @('bin/tool.exe')
    }
  }
  $inventoryPath = Invoke-Generator -Workspace $workspace -SpecPath $specPath
  Assert-ExitCode '아무도 자기 것이라고 하지 않은 파일이 있으면 inventory-unexpected-component' 4 (Invoke-Verify -Workspace $workspace -SpecPath $specPath -InventoryPath $inventoryPath)
}
finally {
  Remove-Item -LiteralPath $workspace -Recurse -Force -ErrorAction SilentlyContinue
}

# --- 5. staged 되지 않았다 ----------------------------------------------------

$workspace = New-Workspace
try {
  $specPath = Write-Spec -Workspace $workspace
  $inventoryPath = Join-Path $workspace 'release/dist/inventory.json'
  Set-Content -LiteralPath $inventoryPath -Encoding utf8 -Value @'
{
  "schemaVersion": "release-inventory/v1",
  "staging": "not-staged",
  "reason": "release:stage 가 실행되지 않았습니다."
}
'@
  Assert-ExitCode 'not-staged 인벤토리는 inventory-not-staged' 5 (Invoke-Verify -Workspace $workspace -SpecPath $specPath -InventoryPath $inventoryPath)
}
finally {
  Remove-Item -LiteralPath $workspace -Recurse -Force -ErrorAction SilentlyContinue
}

# --- 6. 인벤토리를 읽을 수 없다 -----------------------------------------------

$workspace = New-Workspace
try {
  $specPath = Write-Spec -Workspace $workspace
  Assert-ExitCode '인벤토리 파일이 없으면 inventory-manifest-unreadable' 6 (Invoke-Verify -Workspace $workspace -SpecPath $specPath -InventoryPath (Join-Path $workspace 'release/dist/inventory.json'))
}
finally {
  Remove-Item -LiteralPath $workspace -Recurse -Force -ErrorAction SilentlyContinue
}

$workspace = New-Workspace
try {
  $specPath = Write-Spec -Workspace $workspace
  $inventoryPath = Join-Path $workspace 'release/dist/inventory.json'
  Set-Content -LiteralPath $inventoryPath -Value '{ this is not json' -Encoding utf8
  Assert-ExitCode '인벤토리가 깨져 있으면 inventory-manifest-unreadable' 6 (Invoke-Verify -Workspace $workspace -SpecPath $specPath -InventoryPath $inventoryPath)
}
finally {
  Remove-Item -LiteralPath $workspace -Recurse -Force -ErrorAction SilentlyContinue
}

# --- 7. 릴리스 경로인데 핀이 비어 있다 ----------------------------------------

$workspace = New-Workspace
try {
  $specPath = Write-Spec -Workspace $workspace
  $inventoryPath = Invoke-Generator -Workspace $workspace -SpecPath $specPath
  Assert-ExitCode '핀 없이 릴리스하려 하면 inventory-pin-missing' 7 (Invoke-Verify -Workspace $workspace -SpecPath $specPath -InventoryPath $inventoryPath -RequirePins)
}
finally {
  Remove-Item -LiteralPath $workspace -Recurse -Force -ErrorAction SilentlyContinue
}

# --- 7c. 저장소가 빌드하는 산출물은 핀을 요구하지 않는다 ----------------------

$workspace = New-Workspace
try {
  $specPath = Write-Spec -Workspace $workspace -Overrides @{ requiresPin = $false }
  $inventoryPath = Invoke-Generator -Workspace $workspace -SpecPath $specPath
  Assert-ExitCode 'requiresPin=false 면 핀이 없어도 릴리스 경로 통과' 0 (Invoke-Verify -Workspace $workspace -SpecPath $specPath -InventoryPath $inventoryPath -RequirePins)
}
finally {
  Remove-Item -LiteralPath $workspace -Recurse -Force -ErrorAction SilentlyContinue
}

# --- 7b. 핀이 맞으면 릴리스 경로도 통과한다 -----------------------------------

$workspace = New-Workspace
try {
  $specPath = Write-Spec -Workspace $workspace
  $inventoryPath = Invoke-Generator -Workspace $workspace -SpecPath $specPath
  $observed = (Get-Content -LiteralPath $inventoryPath -Raw -Encoding UTF8 | ConvertFrom-Json).components[0].stagedTreeDigest
  $specPath = Write-Spec -Workspace $workspace -Overrides @{ pinnedStagedTreeDigest = $observed }
  Assert-ExitCode '핀이 실측과 같으면 릴리스 경로도 통과' 0 (Invoke-Verify -Workspace $workspace -SpecPath $specPath -InventoryPath $inventoryPath -RequirePins)
}
finally {
  Remove-Item -LiteralPath $workspace -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host ''

if ($failed.Count -gt 0) {
  Write-Host "FAIL: $($failed.Count)건"
  foreach ($failure in $failed) {
    Write-Host "  $failure"
  }
  exit 1
}

Write-Host "PASS: $passed 건 모두 기대한 사유 코드로 판정했습니다."
exit 0
