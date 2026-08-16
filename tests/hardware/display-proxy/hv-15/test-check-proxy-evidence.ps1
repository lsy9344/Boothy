<#
.SYNOPSIS
  HV-15 게이트 자체를 합성 fixture로 검증한다.

.DESCRIPTION
  게이트가 "언제나 PASS"면 아무것도 지키지 못한다. 통과해야 할 회차 하나와
  반드시 실패해야 할 결함 회차들을 만들어 경계를 고정한다.
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$gate = Join-Path $PSScriptRoot 'check-proxy-evidence.ps1'
$root = Join-Path ([System.IO.Path]::GetTempPath()) ("hv15-gate-" + [guid]::NewGuid().ToString('N'))
$failures = 0

function New-Run([string]$name) {
  $sessionRoot = Join-Path $root $name
  New-Item -ItemType Directory -Path (Join-Path $sessionRoot 'renders/display') -Force | Out-Null
  New-Item -ItemType Directory -Path (Join-Path $sessionRoot 'diagnostics') -Force | Out-Null
  return $sessionRoot
}

function New-ProxyGeneration([string]$sessionRoot, [hashtable]$overrides) {
  $assetPath = Join-Path $sessionRoot 'renders/display/req-1/000001-proxy.jpg'
  New-Item -ItemType Directory -Path (Split-Path $assetPath -Parent) -Force | Out-Null
  Set-Content -LiteralPath $assetPath -Value 'jpeg' -Encoding utf8

  $generation = @{
    generationId    = 'req-1-000001'
    generationSeq   = 1
    sessionId       = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
    requestId       = 'req-1'
    captureId       = 'capture-1'
    viewerEpoch     = 3
    tier            = 'displayFitPresetProxy'
    assetPath       = $assetPath
    sourceWidthPx   = 1620
    sourceHeightPx  = 1080
    byteSize        = 400000
    sourceHash      = 'fnv1a64:0123456789abcdef'
    sampleVariant   = $null
    proxyProvenance = @{
      presetId                 = 'preset_soft-glow'
      presetVersion            = '2026.08.01'
      approvalBasis            = 'exact-reference-renderer'
      proxyRecipeVersion       = '1'
      referenceRenderer        = 'darktable'
      referenceRendererVersion = '5.4.1'
      renderProfileId          = 'preset_soft-glow-preview'
      outputColorSpace         = 'sRGB'
      jpegQuality              = 92
      sourceRoute              = 'raw-original'
      sourceAssetHash          = 'fnv1a64:00000000000000aa'
      targetWidthPx            = 1620
      targetHeightPx           = 1080
      displayProfileId         = 'approved-1080p'
      devicePixelRatio         = 1
    }
    committedAtHostMicros = 1500000
  }

  foreach ($key in $overrides.Keys) {
    if ($key -like 'provenance.*') {
      $generation.proxyProvenance[$key.Substring('provenance.'.Length)] = $overrides[$key]
    }
    else {
      $generation[$key] = $overrides[$key]
    }
  }

  return $generation
}

function Write-Evidence([string]$sessionRoot, $generations, $presentRows) {
  $journal = Join-Path $sessionRoot 'renders/display/generations.jsonl'
  $lines = foreach ($generation in $generations) {
    @{
      schemaVersion        = 'viewer-display/v2'
      outcome              = 'committed'
      sessionId            = $generation.sessionId
      requestId            = $generation.requestId
      generationSeq        = $generation.generationSeq
      generation           = $generation
      recordedAtHostMicros = 1500000
    } | ConvertTo-Json -Depth 10 -Compress
  }
  Set-Content -LiteralPath $journal -Value $lines -Encoding utf8

  $presentPath = Join-Path $sessionRoot 'diagnostics/viewer-present.jsonl'
  $presentLines = foreach ($row in $presentRows) { $row | ConvertTo-Json -Depth 10 -Compress }
  Set-Content -LiteralPath $presentPath -Value $presentLines -Encoding utf8
}

function New-PresentRow([string]$generationId, [string]$tier) {
  return @{
    schemaVersion = 'viewer-present/v1'
    sessionId     = 'session_01hs6n1r8b8zc5v4ey2x7b9g1m'
    requestId     = 'req-1'
    generationId  = $generationId
    tier          = $tier
    outcome       = 'presented'
  }
}

function Invoke-Gate([string]$sessionRoot) {
  & $gate -SessionRoot $sessionRoot `
    -ExpectedPresetId 'preset_soft-glow' `
    -ExpectedPresetVersion '2026.08.01' `
    -RequiredSourceWidthPx 1620 `
    -RequiredSourceHeightPx 1080 *> $null
  return $LASTEXITCODE
}

function Assert-Gate([string]$name, [string]$sessionRoot, [int]$expected) {
  $actual = Invoke-Gate $sessionRoot
  if ($actual -ne $expected) {
    Write-Host "  FAIL $name (expected exit $expected, got $actual)"
    $script:failures++
  }
  else {
    Write-Host "  ok   $name"
  }
}

try {
  # 통과해야 하는 회차.
  $run = New-Run 'pass'
  $generation = New-ProxyGeneration $run @{}
  Write-Evidence $run @($generation) @((New-PresentRow 'req-1-000001' 'displayFitPresetProxy'))
  Assert-Gate 'a complete proxy run passes' $run 0

  # 승인된 contain 정책: 세로 산출물은 높이가 실측 영역에 닿으면 확대 없이 게시할 수 있다.
  $run = New-Run 'portrait-contain'
  $generation = New-ProxyGeneration $run @{ sourceWidthPx = 720; sourceHeightPx = 1080 }
  Write-Evidence $run @($generation) @((New-PresentRow 'req-1-000001' 'displayFitPresetProxy'))
  Assert-Gate 'a portrait proxy that reaches the contain boundary passes' $run 0

  # 결함 1: 화면보다 작은 산출물 (upscale 0 위반의 실제 증상).
  $run = New-Run 'undersized'
  $generation = New-ProxyGeneration $run @{ sourceWidthPx = 800; sourceHeightPx = 533 }
  Write-Evidence $run @($generation) @((New-PresentRow 'req-1-000001' 'displayFitPresetProxy'))
  Assert-Gate 'an undersized proxy fails' $run 1

  # 결함 2: provenance 없음.
  $run = New-Run 'no-provenance'
  $generation = New-ProxyGeneration $run @{ proxyProvenance = $null }
  Write-Evidence $run @($generation) @((New-PresentRow 'req-1-000001' 'displayFitPresetProxy'))
  Assert-Gate 'a proxy without provenance fails' $run 1

  # 결함 3: 기대와 다른 preset이 화면에 올라감.
  $run = New-Run 'wrong-preset'
  $generation = New-ProxyGeneration $run @{ 'provenance.presetVersion' = '2026.07.01' }
  Write-Evidence $run @($generation) @((New-PresentRow 'req-1-000001' 'displayFitPresetProxy'))
  Assert-Gate 'a wrong preset version fails' $run 1

  # 결함 4: 목표 크기가 실측 photoRect와 다름.
  $run = New-Run 'wrong-target'
  $generation = New-ProxyGeneration $run @{ 'provenance.targetWidthPx' = 384; 'provenance.targetHeightPx' = 384 }
  Write-Evidence $run @($generation) @((New-PresentRow 'req-1-000001' 'displayFitPresetProxy'))
  Assert-Gate 'a 384px target fails' $run 1

  # 결함 5: terminal 행 누락 — Story 7.2가 한 회차를 잃은 방식.
  $run = New-Run 'missing-terminal'
  $generation = New-ProxyGeneration $run @{}
  Write-Evidence $run @($generation) @()
  Assert-Gate 'a missing terminal row fails' $run 1

  # 결함 6: terminal 행 중복.
  $run = New-Run 'duplicate-terminal'
  $generation = New-ProxyGeneration $run @{}
  Write-Evidence $run @($generation) @(
    (New-PresentRow 'req-1-000001' 'displayFitPresetProxy'),
    (New-PresentRow 'req-1-000001' 'displayFitPresetProxy')
  )
  Assert-Gate 'a duplicated terminal row fails' $run 1

  # 결함 7: proxy generation이 하나도 없음 — lane이 돌지 않았다.
  $run = New-Run 'no-proxy'
  Write-Evidence $run @() @()
  Assert-Gate 'a run with no proxy generation fails' $run 1

  # 결함 8: fixture와 실제 결과가 섞임.
  $run = New-Run 'mixed-lanes'
  $proxy = New-ProxyGeneration $run @{}
  $fixture = New-ProxyGeneration $run @{
    generationId  = 'req-1-000002'
    generationSeq = 2
    tier          = 'sample'
    sampleVariant = 'a'
  }
  Write-Evidence $run @($proxy, $fixture) @(
    (New-PresentRow 'req-1-000001' 'displayFitPresetProxy'),
    (New-PresentRow 'req-1-000002' 'sample')
  )
  Assert-Gate 'mixed fixture and proxy lanes fail' $run 1

  # 결함 9: 확정 자산이 사라짐.
  $run = New-Run 'missing-asset'
  $generation = New-ProxyGeneration $run @{}
  Write-Evidence $run @($generation) @((New-PresentRow 'req-1-000001' 'displayFitPresetProxy'))
  Remove-Item -LiteralPath $generation.assetPath -Force
  Assert-Gate 'a missing committed asset fails' $run 1

  # 결함 10: 정확 경로라고 주장하면서 다른 source에서 렌더함.
  $run = New-Run 'basis-mismatch'
  $generation = New-ProxyGeneration $run @{ 'provenance.sourceRoute' = 'embedded-jpeg' }
  Write-Evidence $run @($generation) @((New-PresentRow 'req-1-000001' 'displayFitPresetProxy'))
  Assert-Gate 'an exact-path claim from a fast source fails' $run 1

  # 결함 11: 손상된 JSONL 행을 조용히 건너뛰지 않는다.
  $run = New-Run 'malformed'
  $generation = New-ProxyGeneration $run @{}
  Write-Evidence $run @($generation) @((New-PresentRow 'req-1-000001' 'displayFitPresetProxy'))
  Add-Content -LiteralPath (Join-Path $run 'renders/display/generations.jsonl') -Value '{ not json' -Encoding utf8
  Assert-Gate 'a malformed journal line fails' $run 1

  Write-Host ''
  if ($failures -gt 0) {
    Write-Host "FAIL: $failures gate scenario(s) behaved incorrectly"
    exit 1
  }
  Write-Host 'PASS: the HV-15 gate accepts a complete run and rejects all 11 seeded defects.'
  exit 0
}
finally {
  if (Test-Path $root) { Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue }
}
