<#
.SYNOPSIS
  HV-14 source-comparison telemetry 완결성 게이트.

.DESCRIPTION
  AB 회차의 35개 촬영 각각에 세 route 행이 정확히 하나씩 있는지 검증한다.
  기본 PASS는 telemetry completeness만 뜻하며 HV-14 Go가 아니다.
  -FinalPackageGate를 추가하면 capability/correlation/quality/aggregate/decision 증거도 확인한다.

.PARAMETER SessionEvidenceDir
  회차 디렉터리 아래 session-evidence 경로.

.PARAMETER ExpectedPerRoute
  route당 기대 촬영 수. 기본 35 = warm-up 5 + measured 30.

.PARAMETER WarmUpPerRoute
  route당 기대 warm-up 수. 기본 5.

.PARAMETER ExpectedRoutes
  이번 회차가 반드시 포함해야 하는 route. 기본은 AB의 세 canonical route다.

.PARAMETER FinalPackageGate
  session-evidence의 상위 run 디렉터리에서 최종 HV-14 증거 패키지까지 검사한다.
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [string]$SessionEvidenceDir,

  [int]$ExpectedPerRoute = 35,

  [int]$WarmUpPerRoute = 5,

  [string[]]$ExpectedRoutes = @(
    'embedded-jpeg',
    'camera-paired-jpeg',
    'windows-shell-thumbnail'
  ),

  [switch]$FinalPackageGate,

  [string]$OutFile
)

$ErrorActionPreference = 'Stop'

$canonicalRoutes = @(
  'embedded-jpeg',
  'camera-paired-jpeg',
  'windows-shell-thumbnail'
)
$failures = New-Object System.Collections.Generic.List[string]
$lines = New-Object System.Collections.Generic.List[string]

function Has-Text {
  param($Value)
  return $null -ne $Value -and -not [string]::IsNullOrWhiteSpace([string]$Value)
}

function Read-JsonLines {
  param(
    [string]$Path,
    [System.Collections.Generic.List[string]]$Failures
  )

  if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
    $Failures.Add("필요한 증거 파일이 없습니다: $Path")
    return
  }

  $lineNumber = 0
  foreach ($line in (Get-Content -LiteralPath $Path -Encoding utf8)) {
    $lineNumber += 1
    if ([string]::IsNullOrWhiteSpace($line)) {
      continue
    }

    try {
      $line | ConvertFrom-Json -ErrorAction Stop
    }
    catch {
      $Failures.Add("JSONL $lineNumber 행을 파싱할 수 없습니다: $($_.Exception.Message)")
    }
  }
}

function Test-NonEmptyFile {
  param([string]$Path)
  return (Test-Path -LiteralPath $Path -PathType Leaf) -and (Get-Item -LiteralPath $Path).Length -gt 0
}

function Read-JsonEvidence {
  param(
    [string]$Path,
    [string]$Label
  )

  if (-not (Test-NonEmptyFile $Path)) {
    $failures.Add("$Label JSON 증거가 없거나 비었습니다: $Path")
    return $null
  }

  try {
    return Get-Content -LiteralPath $Path -Raw -Encoding utf8 | ConvertFrom-Json -ErrorAction Stop
  }
  catch {
    $failures.Add("$Label JSON 증거를 파싱할 수 없습니다: $($_.Exception.Message)")
    return $null
  }
}

function Get-MarkdownField {
  param(
    [string]$Content,
    [string]$Label
  )

  $pattern = '(?im)^\s*-?\s*' + [regex]::Escape($Label) + '\s*:\s*(.+?)\s*$'
  $match = [regex]::Match($Content, $pattern)
  if (-not $match.Success) {
    return $null
  }
  return $match.Groups[1].Value.Trim().Trim('`')
}

function Test-ConfirmedText {
  param($Value)
  if (-not (Has-Text $Value)) {
    return $false
  }
  $text = [string]$Value
  return $text -notmatch '<[^>]+>|(?i)pending|(?i)todo|(?i)synthetic|(?i)example'
}

function Test-ReferencedEvidenceFile {
  param(
    [string]$Directory,
    [string]$RelativePath
  )

  if (-not (Test-ConfirmedText $RelativePath)) {
    return $false
  }
  if ([IO.Path]::IsPathRooted($RelativePath)) {
    return $false
  }

  $resolvedDirectory = [IO.Path]::GetFullPath($Directory)
  $resolvedFile = [IO.Path]::GetFullPath((Join-Path $resolvedDirectory $RelativePath))
  $directoryPrefix = $resolvedDirectory.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
  return $resolvedFile.StartsWith($directoryPrefix, [StringComparison]::OrdinalIgnoreCase) -and (Test-NonEmptyFile $resolvedFile)
}

function Test-ImageEvidenceFile {
  param(
    [string]$Directory,
    [string]$RelativePath
  )

  if (-not (Test-ReferencedEvidenceFile -Directory $Directory -RelativePath $RelativePath)) {
    return $false
  }
  $bytes = [IO.File]::ReadAllBytes([IO.Path]::GetFullPath((Join-Path $Directory $RelativePath)))
  $isJpeg = $bytes.Length -ge 3 -and $bytes[0] -eq 0xFF -and $bytes[1] -eq 0xD8 -and $bytes[2] -eq 0xFF
  $isPng = $bytes.Length -ge 8 -and
    $bytes[0] -eq 0x89 -and $bytes[1] -eq 0x50 -and $bytes[2] -eq 0x4E -and $bytes[3] -eq 0x47 -and
    $bytes[4] -eq 0x0D -and $bytes[5] -eq 0x0A -and $bytes[6] -eq 0x1A -and $bytes[7] -eq 0x0A
  return $isJpeg -or $isPng
}

if ($ExpectedPerRoute -le 0) {
  $failures.Add('ExpectedPerRoute는 1 이상이어야 합니다.')
}
if ($WarmUpPerRoute -lt 0 -or $WarmUpPerRoute -ge $ExpectedPerRoute) {
  $failures.Add('WarmUpPerRoute는 0 이상이며 ExpectedPerRoute보다 작아야 합니다.')
}

$expectedRoutesNormalized = @($ExpectedRoutes | Where-Object { Has-Text $_ } | Sort-Object -Unique)
if ($expectedRoutesNormalized.Count -eq 0) {
  $failures.Add('ExpectedRoutes가 비어 있습니다.')
}
foreach ($route in $expectedRoutesNormalized) {
  if ($route -notin $canonicalRoutes) {
    $failures.Add("ExpectedRoutes에 알 수 없는 route '$route'가 있습니다.")
  }
}

$comparisonPath = Join-Path $SessionEvidenceDir 'diagnostics/source-comparison.jsonl'
$samples = @(Read-JsonLines -Path $comparisonPath -Failures $failures)

$lines.Add('# HV-14 source-comparison telemetry 완결성 결과')
$lines.Add('')
$lines.Add("- 증거 경로: $comparisonPath")
$lines.Add("- 전체 표본 행: $($samples.Count)")
$lines.Add("- 기대 route: $($expectedRoutesNormalized -join ', ')")
$lines.Add("- route당 기대: warm-up $WarmUpPerRoute + measured $($ExpectedPerRoute - $WarmUpPerRoute) = $ExpectedPerRoute")
$lines.Add("- 최종 패키지 게이트: $($FinalPackageGate.IsPresent)")
$lines.Add('')

if ($samples.Count -eq 0) {
  $failures.Add('표본 행이 하나도 없습니다. 측정 lane이 꺼져 있었을 수 있습니다.')
}

# 필수 필드와 행 단위 불변식
for ($index = 0; $index -lt $samples.Count; $index++) {
  $sample = $samples[$index]
  $row = $index + 1
  $candidate = $sample.candidate

  if ($sample.schemaVersion -ne 'source-comparison/v1') {
    $failures.Add("$row 행의 schemaVersion이 source-comparison/v1이 아닙니다.")
  }
  if ($null -eq $candidate) {
    $failures.Add("$row 행에 candidate가 없습니다.")
    continue
  }
  foreach ($field in @('sessionId', 'requestId', 'captureId', 'route')) {
    if (-not (Has-Text $candidate.$field)) {
      $failures.Add("$row 행의 candidate.$field 값이 비어 있습니다.")
    }
  }
  if ($sample.accepted -isnot [bool]) {
    $failures.Add("$row 행의 accepted가 boolean이 아닙니다.")
  }
  if ($sample.isWarmUp -isnot [bool]) {
    $failures.Add("$row 행의 isWarmUp이 boolean이 아닙니다.")
  }
  if ($sample.usedFallbackCorrelation -isnot [bool]) {
    $failures.Add("$row 행의 usedFallbackCorrelation이 boolean이 아닙니다.")
  }
  if ($sample.isPresetApplied -ne $false) {
    $failures.Add("$row 행의 isPresetApplied가 false가 아닙니다.")
  }
  if ($sample.blockOrder -notin @('AB', 'BA')) {
    $failures.Add("$row 행의 blockOrder가 AB/BA가 아닙니다.")
  }

  $parsedBlockIndex = 0
  if (-not [int]::TryParse([string]$sample.blockIndex, [ref]$parsedBlockIndex) -or $parsedBlockIndex -lt 0) {
    $failures.Add("$row 행의 blockIndex가 0 이상의 정수가 아닙니다.")
  }
  if (-not (Has-Text $sample.randomizationSeed)) {
    $failures.Add("$row 행의 randomizationSeed가 비어 있습니다.")
  }

  if ($sample.accepted -eq $false -and -not (Has-Text $sample.rejectReason)) {
    $failures.Add("$row 행은 거부됐지만 rejectReason이 비어 있습니다.")
  }
  if ($sample.accepted -eq $true -and (Has-Text $sample.rejectReason)) {
    $failures.Add("$row 행은 승격됐지만 rejectReason이 있습니다.")
  }

  if ($candidate.route -eq 'camera-paired-jpeg' -and $sample.accepted -eq $true) {
    if ($null -eq $candidate.objectIndex) {
      $failures.Add("$row 행의 paired JPEG에 objectIndex가 없습니다.")
    }
    if ($null -eq $candidate.groupId -and $sample.usedFallbackCorrelation -ne $true) {
      $failures.Add("$row 행의 paired JPEG에 groupId도 명시적 fallback correlation도 없습니다.")
    }
  }
}

# route 집합은 기대 집합과 정확히 같아야 한다.
$observedRoutes = @(
  $samples |
    ForEach-Object { $_.candidate.route } |
    Where-Object { Has-Text $_ } |
    Sort-Object -Unique
)
foreach ($route in $observedRoutes) {
  if ($route -notin $canonicalRoutes) {
    $failures.Add("알 수 없는 route '$route'가 표본에 있습니다.")
  }
  if ($route -notin $expectedRoutesNormalized) {
    $failures.Add("이번 회차에서 기대하지 않은 route '$route'가 표본에 있습니다.")
  }
}
foreach ($route in $expectedRoutesNormalized) {
  if ($route -notin $observedRoutes) {
    $failures.Add("필수 route '$route'의 표본이 하나도 없습니다.")
  }
}

$lines.Add('## Route별 집계')
$lines.Add('')
$lines.Add('| route | 표본 행 | 승격 | 거부 | 성공률 | warm-up | measured |')
$lines.Add('| --- | --- | --- | --- | --- | --- | --- |')

foreach ($route in $expectedRoutesNormalized) {
  $routeSamples = @($samples | Where-Object { $_.candidate.route -eq $route })
  $total = $routeSamples.Count
  $accepted = @($routeSamples | Where-Object { $_.accepted -eq $true }).Count
  $rejected = @($routeSamples | Where-Object { $_.accepted -eq $false }).Count
  $warmUp = @($routeSamples | Where-Object { $_.isWarmUp -eq $true }).Count
  $measured = @($routeSamples | Where-Object { $_.isWarmUp -eq $false }).Count
  $rate = if ($total -gt 0) { [math]::Round(100.0 * $accepted / $total, 1) } else { 0 }

  $lines.Add("| $route | $total | $accepted | $rejected | $rate% | $warmUp | $measured |")

  if ($total -ne $ExpectedPerRoute) {
    $failures.Add("route '$route'의 표본 행이 $total건입니다. 기대값은 $ExpectedPerRoute건입니다.")
  }
  if ($warmUp -ne $WarmUpPerRoute) {
    $failures.Add("route '$route'의 warm-up이 $warmUp건입니다. 기대값은 $WarmUpPerRoute건입니다.")
  }
  if ($measured -ne ($ExpectedPerRoute - $WarmUpPerRoute)) {
    $failures.Add("route '$route'의 measured 표본이 $measured건입니다. 기대값은 $($ExpectedPerRoute - $WarmUpPerRoute)건입니다.")
  }
}
$lines.Add('')

# 촬영 하나마다 모든 기대 route가 정확히 한 행이어야 한다.
$requestGroups = @(
  $samples |
    Where-Object { Has-Text $_.candidate.requestId } |
    Group-Object { $_.candidate.requestId }
)
if ($requestGroups.Count -ne $ExpectedPerRoute) {
  $failures.Add("고유 requestId가 $($requestGroups.Count)개입니다. 기대 촬영 수는 $ExpectedPerRoute개입니다.")
}

$allThreeRoutesExpected = $expectedRoutesNormalized.Count -eq 3 -and @(
  $canonicalRoutes | Where-Object { $_ -notin $expectedRoutesNormalized }
).Count -eq 0

foreach ($requestGroup in $requestGroups) {
  foreach ($route in $expectedRoutesNormalized) {
    $routeCount = @($requestGroup.Group | Where-Object { $_.candidate.route -eq $route }).Count
    if ($routeCount -ne 1) {
      $failures.Add("request '$($requestGroup.Name)'의 route '$route' 행이 $routeCount개입니다. 정확히 1개여야 합니다.")
    }
  }

  $captureIds = @($requestGroup.Group | ForEach-Object { $_.candidate.captureId } | Where-Object { Has-Text $_ } | Sort-Object -Unique)
  $sessionIds = @($requestGroup.Group | ForEach-Object { $_.candidate.sessionId } | Where-Object { Has-Text $_ } | Sort-Object -Unique)
  $blockIndices = @($requestGroup.Group | ForEach-Object { $_.blockIndex } | Sort-Object -Unique)
  $blockOrders = @($requestGroup.Group | ForEach-Object { $_.blockOrder } | Sort-Object -Unique)
  $warmUpValues = @($requestGroup.Group | ForEach-Object { $_.isWarmUp } | Sort-Object -Unique)
  $seedValues = @($requestGroup.Group | ForEach-Object { $_.randomizationSeed } | Where-Object { Has-Text $_ } | Sort-Object -Unique)

  if ($captureIds.Count -ne 1) { $failures.Add("request '$($requestGroup.Name)'가 하나의 captureId로 묶이지 않았습니다.") }
  if ($sessionIds.Count -ne 1) { $failures.Add("request '$($requestGroup.Name)'가 하나의 sessionId로 묶이지 않았습니다.") }
  if ($blockIndices.Count -ne 1) { $failures.Add("request '$($requestGroup.Name)'의 blockIndex가 route 간 일치하지 않습니다.") }
  if ($blockOrders.Count -ne 1) { $failures.Add("request '$($requestGroup.Name)'의 blockOrder가 route 간 일치하지 않습니다.") }
  if ($warmUpValues.Count -ne 1) { $failures.Add("request '$($requestGroup.Name)'의 isWarmUp이 route 간 일치하지 않습니다.") }
  if ($seedValues.Count -ne 1) { $failures.Add("request '$($requestGroup.Name)'의 randomizationSeed가 route 간 일치하지 않습니다.") }

  if ($blockIndices.Count -eq 1 -and $warmUpValues.Count -eq 1) {
    $shouldBeWarmUp = [int]$blockIndices[0] -lt $WarmUpPerRoute
    if ([bool]$warmUpValues[0] -ne $shouldBeWarmUp) {
      $failures.Add("request '$($requestGroup.Name)'의 isWarmUp이 blockIndex와 모순됩니다.")
    }
  }

  if ($allThreeRoutesExpected -and $blockOrders.Count -eq 1) {
    $expectedSequence = if ($blockOrders[0] -eq 'AB') {
      @('embedded-jpeg', 'camera-paired-jpeg', 'windows-shell-thumbnail')
    }
    else {
      @('camera-paired-jpeg', 'embedded-jpeg', 'windows-shell-thumbnail')
    }
    $actualSequence = @($requestGroup.Group | ForEach-Object { $_.candidate.route })
    if (($actualSequence -join '|') -ne ($expectedSequence -join '|')) {
      $failures.Add("request '$($requestGroup.Name)'의 JSONL route 순서가 blockOrder '$($blockOrders[0])'와 다릅니다.")
    }
  }
}

# 한 회차는 한 session, 한 seed, 연속 blockIndex를 가져야 한다.
$sessionIds = @($samples | ForEach-Object { $_.candidate.sessionId } | Where-Object { Has-Text $_ } | Sort-Object -Unique)
if ($sessionIds.Count -ne 1) {
  $failures.Add("한 증거 패키지에 sessionId가 $($sessionIds.Count)개 있습니다. 정확히 1개여야 합니다.")
}
$seeds = @($samples | ForEach-Object { $_.randomizationSeed } | Where-Object { Has-Text $_ } | Sort-Object -Unique)
if ($seeds.Count -ne 1) {
  $failures.Add("randomizationSeed가 $($seeds.Count)개입니다. 비어 있지 않은 단일 seed여야 합니다.")
}

$blockGroups = @($samples | Group-Object { $_.blockIndex })
for ($blockIndex = 0; $blockIndex -lt $ExpectedPerRoute; $blockIndex++) {
  $matching = @($blockGroups | Where-Object { [string]$_.Name -eq [string]$blockIndex })
  if ($matching.Count -ne 1) {
    $failures.Add("blockIndex $blockIndex 묶음이 없습니다.")
    continue
  }
  $requestsInBlock = @($matching[0].Group | ForEach-Object { $_.candidate.requestId } | Sort-Object -Unique)
  if ($requestsInBlock.Count -ne 1) {
    $failures.Add("blockIndex $blockIndex가 하나의 requestId에 속하지 않습니다.")
  }
}

$abBlocks = @($requestGroups | Where-Object { $_.Group[0].blockOrder -eq 'AB' }).Count
$baBlocks = @($requestGroups | Where-Object { $_.Group[0].blockOrder -eq 'BA' }).Count
$lines.Add('## AB/BA 배치')
$lines.Add('')
$lines.Add("- randomizationSeed: $($seeds -join ', ')")
$lines.Add("- AB block: $abBlocks")
$lines.Add("- BA block: $baBlocks")
$lines.Add('')
if ($allThreeRoutesExpected -and ($abBlocks -eq 0 -or $baBlocks -eq 0)) {
  $failures.Add('AB 회차에는 AB와 BA block이 모두 최소 1개 있어야 합니다.')
}

# 실패 분포와 correlation 근거는 제외하지 않고 보고한다.
$rejections = @($samples | Where-Object { $_.accepted -eq $false })
if ($rejections.Count -gt 0) {
  $lines.Add('## 거부 사유 분포')
  $lines.Add('')
  $lines.Add('| 사유 | 건수 |')
  $lines.Add('| --- | --- |')
  foreach ($group in ($rejections | Group-Object rejectReason | Sort-Object Count -Descending)) {
    $lines.Add("| $($group.Name) | $($group.Count) |")
  }
  $lines.Add('')
}

$fallbackCorrelated = @($samples | Where-Object { $_.usedFallbackCorrelation -eq $true })
$lines.Add('## Correlation 근거')
$lines.Add('')
$lines.Add("- groupID로 묶인 표본: $($samples.Count - $fallbackCorrelated.Count)")
$lines.Add("- 보조 correlation 표본: $($fallbackCorrelated.Count)")
$lines.Add('')

if ($FinalPackageGate.IsPresent) {
  $runDir = Split-Path -Parent ([IO.Path]::GetFullPath($SessionEvidenceDir))
  $lines.Add('## 최종 증거 패키지')
  $lines.Add('')
  $environmentPath = Join-Path $runDir 'environment.md'
  if (-not (Test-NonEmptyFile $environmentPath)) {
    $failures.Add("환경 증거가 없거나 비었습니다: $environmentPath")
  }
  else {
    $environment = Get-Content -LiteralPath $environmentPath -Raw -Encoding utf8
    $requiredEnvironmentFields = @(
      'recordedAt', 'validator', 'booth PC', 'app commit', 'OS', 'CPU / RAM',
      'GPU / driver', 'camera', 'camera firmware', 'lens', 'memory card', 'power',
      'USB port / cable / hub', 'Canon EDSDK', 'helper / protocol',
      'source comparison mode', 'display sample mode', 'Image Quality before',
      'Image Quality after'
    )
    foreach ($field in $requiredEnvironmentFields) {
      if (-not (Test-ConfirmedText (Get-MarkdownField -Content $environment -Label $field))) {
        $failures.Add("environment.md의 '$field' 값이 없거나 확정되지 않았습니다.")
      }
    }
    if ((Get-MarkdownField -Content $environment -Label 'source comparison mode') -ne 'ab') {
      $failures.Add('environment.md의 source comparison mode는 ab여야 합니다.')
    }
    if ((Get-MarkdownField -Content $environment -Label 'display sample mode') -ne 'off') {
      $failures.Add('environment.md의 display sample mode는 off여야 합니다.')
    }
    $qualityBefore = Get-MarkdownField -Content $environment -Label 'Image Quality before'
    $qualityAfter = Get-MarkdownField -Content $environment -Label 'Image Quality after'
    if ((Test-ConfirmedText $qualityBefore) -and (Test-ConfirmedText $qualityAfter) -and $qualityBefore -ne $qualityAfter) {
      $failures.Add('카메라 Image Quality의 측정 전/후 값이 같지 않아 복원을 확인할 수 없습니다.')
    }
  }

  $capabilityDir = Join-Path $runDir 'capability'
  $capability = Read-JsonEvidence -Path (Join-Path $capabilityDir 'summary.json') -Label 'capability'
  if ($null -ne $capability) {
    if ($capability.schemaVersion -ne 'hv-14-capability/v1') { $failures.Add('capability schemaVersion이 hv-14-capability/v1이 아닙니다.') }
    if (-not (Test-ConfirmedText $capability.currentBefore) -or -not (Test-ConfirmedText $capability.currentAfter)) {
      $failures.Add('capability의 측정 전/후 현재값이 확정되지 않았습니다.')
    }
    elseif ($capability.currentBefore -ne $capability.currentAfter) {
      $failures.Add('capability의 측정 전/후 현재값이 달라 카메라 설정 복원을 확인할 수 없습니다.')
    }
    $descriptorValues = @($capability.descriptorValues)
    if ($descriptorValues.Count -eq 0 -or @($descriptorValues | Where-Object { [string]$_ -notmatch '^0x[0-9a-fA-F]+$' }).Count -gt 0) {
      $failures.Add('capability descriptorValues에 유효한 16진수 원문이 없습니다.')
    }
    if ($capability.rawPlusJpegSupported -isnot [bool]) { $failures.Add('capability rawPlusJpegSupported가 boolean이 아닙니다.') }
    $combinations = @($capability.combinations)
    if ($combinations.Count -eq 0 -or @($combinations | Where-Object {
      $_.status -notin @('attempted', 'skipped') -or [string]$_.value -notmatch '^0x[0-9a-fA-F]+$' -or -not (Test-ConfirmedText $_.reason)
    }).Count -gt 0) {
      $failures.Add('capability combinations에 시도/미시도 값과 이유가 완결되게 기록되지 않았습니다.')
    }
  }

  $correlationDir = Join-Path $runDir 'correlation'
  $correlation = Read-JsonEvidence -Path (Join-Path $correlationDir 'summary.json') -Label 'correlation'
  if ($null -ne $correlation) {
    if ($correlation.schemaVersion -ne 'hv-14-correlation/v1') { $failures.Add('correlation schemaVersion이 hv-14-correlation/v1이 아닙니다.') }
    $expectedPairedCorrelationCount = @($samples | Where-Object {
      $_.candidate.route -eq 'camera-paired-jpeg' -and $null -ne $_.candidate.objectIndex
    }).Count
    if ([int]$correlation.pairedRequestCount -ne $expectedPairedCorrelationCount) {
      $failures.Add("correlation pairedRequestCount가 telemetry 근거 $expectedPairedCorrelationCount 건과 다릅니다.")
    }
    foreach ($field in @('rejectedObjectCount', 'fallbackCorrelationCount', 'originalsJpegCount')) {
      if ($null -eq $correlation.$field -or [int]$correlation.$field -lt 0) { $failures.Add("correlation $field 값이 유효하지 않습니다.") }
    }
    if ([int]$correlation.originalsJpegCount -ne 0) { $failures.Add('captures/originals에 JPEG가 저장된 증거가 있습니다.') }
    if ($correlation.rawTruthPreserved -ne $true) { $failures.Add('correlation 증거가 RAW truth 보존을 확인하지 않았습니다.') }
    $correlationFiles = @($correlation.evidenceFiles)
    if ($correlationFiles.Count -eq 0 -or @($correlationFiles | Where-Object {
      -not (Test-ReferencedEvidenceFile -Directory $correlationDir -RelativePath ([string]$_))
    }).Count -gt 0) {
      $failures.Add('correlation evidenceFiles가 없거나 실제 증거 파일을 가리키지 않습니다.')
    }
  }

  $qualityDir = Join-Path $runDir 'quality'
  $quality = Read-JsonEvidence -Path (Join-Path $qualityDir 'manifest.json') -Label 'quality'
  if ($null -ne $quality) {
    if ($quality.schemaVersion -ne 'hv-14-quality/v1') { $failures.Add('quality schemaVersion이 hv-14-quality/v1이 아닙니다.') }
    if ($quality.humanApproved -ne $true -or -not (Test-ConfirmedText $quality.reviewer)) {
      $failures.Add('quality corpus의 사람 승인과 reviewer가 확정되지 않았습니다.')
    }
    $qualityAssetPaths = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($sceneKind in @('bright', 'dark', 'portrait', 'high-contrast')) {
      $scene = @($quality.scenes | Where-Object { $_.kind -eq $sceneKind }) | Select-Object -First 1
      if ($null -eq $scene) {
        $failures.Add("quality corpus에 $sceneKind 장면이 없습니다.")
        continue
      }
      foreach ($route in $canonicalRoutes) {
        $asset = @($scene.assets | Where-Object { $_.route -eq $route }) | Select-Object -First 1
        if ($null -eq $asset -or -not (Test-ImageEvidenceFile -Directory $qualityDir -RelativePath ([string]$asset.file))) {
          $failures.Add("quality corpus의 $sceneKind / $route 비교 이미지가 없습니다.")
        }
        elseif (-not $qualityAssetPaths.Add([string]$asset.file)) {
          $failures.Add("quality corpus에서 같은 이미지 '$($asset.file)'를 여러 장면 또는 route에 재사용했습니다.")
        }
      }
    }
  }

  $aggregateDir = Join-Path $runDir 'aggregate'
  $aggregate = Read-JsonEvidence -Path (Join-Path $aggregateDir 'summary.json') -Label 'aggregate'
  if ($null -ne $aggregate) {
    if ($aggregate.schemaVersion -ne 'hv-14-aggregate/v1') { $failures.Add('aggregate schemaVersion이 hv-14-aggregate/v1이 아닙니다.') }
    if ($null -eq $aggregate.randomizationSeed -or [int]$aggregate.blockCount -lt $ExpectedPerRoute) {
      $failures.Add('aggregate의 randomizationSeed 또는 blockCount가 완결되지 않았습니다.')
    }
    foreach ($route in $canonicalRoutes) {
      $routeAggregate = @($aggregate.routes | Where-Object { $_.route -eq $route }) | Select-Object -First 1
      if ($null -eq $routeAggregate) {
        $failures.Add("aggregate에 $route 집계가 없습니다.")
        continue
      }
      $attemptCount = [int]$routeAggregate.attemptCount
      $successCount = [int]$routeAggregate.successCount
      $p50 = [long]$routeAggregate.p50Micros
      $p95 = [long]$routeAggregate.p95Micros
      $max = [long]$routeAggregate.maxMicros
      if ($attemptCount -ne $ExpectedPerRoute -or $successCount -lt 0 -or $successCount -gt $attemptCount) {
        $failures.Add("aggregate의 $route 성공률 분모/분자가 유효하지 않습니다.")
      }
      if ($p50 -lt 0 -or $p95 -lt $p50 -or $max -lt $p95) {
        $failures.Add("aggregate의 $route p50/p95/max가 유효하지 않습니다.")
      }
      if ($null -eq $routeAggregate.failureModes) {
        $failures.Add("aggregate의 $route 실패 모드 분포가 없습니다.")
      }
      else {
        $failureModeTotal = ($routeAggregate.failureModes.PSObject.Properties.Value | Measure-Object -Sum).Sum
        if ([int]$failureModeTotal -ne ($attemptCount - $successCount)) {
          $failures.Add("aggregate의 $route 실패 모드 합계가 실패 횟수와 다릅니다.")
        }
      }
      if ($route -eq 'camera-paired-jpeg' -and $null -eq $routeAggregate.arrivalOrderDistribution) {
        $failures.Add('aggregate의 camera-paired-jpeg 도착 순서 분포가 없습니다.')
      }
      elseif ($route -eq 'camera-paired-jpeg') {
        $arrivalTotal = ($routeAggregate.arrivalOrderDistribution.PSObject.Properties.Value | Measure-Object -Sum).Sum
        if ([int]$arrivalTotal -ne $attemptCount) {
          $failures.Add('aggregate의 camera-paired-jpeg 도착 순서 합계가 시도 횟수와 다릅니다.')
        }
      }
    }
  }

  $decisionPath = Join-Path $runDir 'decision.md'
  if (-not (Test-NonEmptyFile $decisionPath)) {
    $failures.Add("route 결정문이 없거나 비었습니다: $decisionPath")
  }
  else {
    $decision = Get-Content -LiteralPath $decisionPath -Raw -Encoding utf8
    $decisionKind = Get-MarkdownField -Content $decision -Label 'Decision'
    if ($decisionKind -notmatch '^(?i:primary|fallback|no-go)$') {
      $failures.Add('decision.md의 Decision은 primary / fallback / No-Go 중 하나여야 합니다.')
    }
    foreach ($field in @('Selected route', 'Baseline improvement', 'Rejected routes', 'Story 7.4 constraint')) {
      if (-not (Test-ConfirmedText (Get-MarkdownField -Content $decision -Label $field))) {
        $failures.Add("decision.md의 '$field' 근거가 없거나 확정되지 않았습니다.")
      }
    }
    $selectedRoute = Get-MarkdownField -Content $decision -Label 'Selected route'
    if ($decisionKind -match '^(?i:primary|fallback)$' -and $selectedRoute -notin $canonicalRoutes) {
      $failures.Add('primary/fallback 결정의 Selected route는 검증된 canonical route여야 합니다.')
    }
  }
  $lines.Add("- run 디렉터리: $runDir")
  $lines.Add('- 확인 항목: environment, capability, correlation, quality, aggregate, decision')
  $lines.Add('')
}

$lines.Add('## 판정')
$lines.Add('')
if ($failures.Count -eq 0 -and $FinalPackageGate.IsPresent) {
  $lines.Add('**PASS (final package gate)** — telemetry와 구조화된 HV-14 증거가 모두 검증되었습니다.')
  $lines.Add('')
  $lines.Add('> 이 결과와 사람의 품질 승인·ledger 갱신을 함께 검토한 뒤에만 HV-14 Go/No-Go를 기록합니다.')
}
elseif ($failures.Count -eq 0) {
  $lines.Add('**PASS (telemetry completeness only)** — source-comparison 행의 완결성 조건을 만족합니다.')
  $lines.Add('')
  $lines.Add('> 이것은 HV-14 Go가 아닙니다. capability, correlation, quality, aggregate, decision 증거가 아직 필요합니다.')
}
else {
  $lines.Add("**FAIL** — $($failures.Count)건의 문제가 있습니다.")
  $lines.Add('')
  foreach ($failure in $failures) {
    $lines.Add("- $failure")
  }
}

$report = $lines -join [Environment]::NewLine
if ($OutFile) {
  $outParent = Split-Path -Parent ([IO.Path]::GetFullPath($OutFile))
  if (-not (Test-Path -LiteralPath $outParent)) {
    New-Item -ItemType Directory -Path $outParent -Force | Out-Null
  }
  $report | Out-File -FilePath $OutFile -Encoding utf8
}

Write-Output $report
if ($failures.Count -gt 0) {
  exit 1
}
