$ErrorActionPreference = 'Stop'

function Read-HookPayload {
  $raw = [Console]::In.ReadToEnd()
  if ([string]::IsNullOrWhiteSpace($raw)) {
    return @{}
  }

  return $raw | ConvertFrom-Json
}

function Get-GitRoot {
  param([string]$FallbackCwd)

  try {
    $root = (git rev-parse --show-toplevel 2>$null | Select-Object -First 1)
    if (-not [string]::IsNullOrWhiteSpace($root)) {
      return $root.Trim()
    }
  } catch {
  }

  if (-not [string]::IsNullOrWhiteSpace($FallbackCwd)) {
    return $FallbackCwd
  }

  return (Get-Location).Path
}

function Ensure-Directory {
  param([string]$Path)

  if (-not (Test-Path $Path -PathType Container)) {
    New-Item -ItemType Directory -Path $Path -Force | Out-Null
  }
}

function Get-HistoryDocPath {
  param([string]$RepoRoot)

  return Join-Path $RepoRoot 'history/camera-capture-validation-history.md'
}

function Get-StateRoot {
  param([string]$RepoRoot)

  $path = Join-Path $RepoRoot '.codex/hook-state'
  Ensure-Directory -Path $path
  return $path
}

function Get-TurnStatePath {
  param(
    [string]$RepoRoot,
    [string]$SessionId,
    [string]$TurnId
  )

  $safeSessionId = ($SessionId -replace '[^A-Za-z0-9._-]', '_')
  $safeTurnId = ($TurnId -replace '[^A-Za-z0-9._-]', '_')
  $stateDir = Join-Path (Get-StateRoot -RepoRoot $RepoRoot) 'turns'
  Ensure-Directory -Path $stateDir
  return Join-Path $stateDir ("{0}__{1}.json" -f $safeSessionId, $safeTurnId)
}

function Save-TurnState {
  param(
    [string]$RepoRoot,
    [string]$SessionId,
    [string]$TurnId,
    [string]$Prompt
  )

  $path = Get-TurnStatePath -RepoRoot $RepoRoot -SessionId $SessionId -TurnId $TurnId
  $payload = @{
    session_id = $SessionId
    turn_id = $TurnId
    prompt = $Prompt
    recorded_at = [DateTimeOffset]::Now.ToString('o')
  }

  Set-Content -Path $path -Value ($payload | ConvertTo-Json -Depth 6) -Encoding utf8
}

function Load-TurnState {
  param(
    [string]$RepoRoot,
    [string]$SessionId,
    [string]$TurnId
  )

  $path = Get-TurnStatePath -RepoRoot $RepoRoot -SessionId $SessionId -TurnId $TurnId
  if (-not (Test-Path $path -PathType Leaf)) {
    return $null
  }

  return (Get-Content -Path $path -Raw) | ConvertFrom-Json
}

function Get-TranscriptObjects {
  param([string]$TranscriptPath)

  $objects = New-Object System.Collections.Generic.List[object]
  if ([string]::IsNullOrWhiteSpace($TranscriptPath) -or -not (Test-Path $TranscriptPath -PathType Leaf)) {
    return $objects
  }

  foreach ($line in Get-Content -Path $TranscriptPath) {
    if ([string]::IsNullOrWhiteSpace($line)) {
      continue
    }

    try {
      $objects.Add(($line | ConvertFrom-Json))
    } catch {
    }
  }

  return $objects
}

function Get-TextFromMessageContent {
  param($ContentItems)

  if ($null -eq $ContentItems) {
    return ''
  }

  $segments = New-Object System.Collections.Generic.List[string]
  foreach ($item in $ContentItems) {
    if ($item.PSObject.Properties.Name -contains 'text' -and -not [string]::IsNullOrWhiteSpace($item.text)) {
      $segments.Add($item.text)
    }
  }

  return ($segments -join "`n").Trim()
}

function Parse-JsonString {
  param([string]$Raw)

  if ([string]::IsNullOrWhiteSpace($Raw)) {
    return $null
  }

  try {
    return $Raw | ConvertFrom-Json
  } catch {
    return $null
  }
}

function Get-ExitCodeFromToolOutput {
  param([string]$Output)

  if ([string]::IsNullOrWhiteSpace($Output)) {
    return $null
  }

  $match = [regex]::Match($Output, 'Exit code:\s*(-?\d+)')
  if ($match.Success) {
    return [int]$match.Groups[1].Value
  }

  return $null
}

function Get-TurnShellCommandResults {
  param(
    [string]$TranscriptPath,
    [string]$TurnId
  )

  $turnIdsByCallId = @{}
  $shellCommandsByCallId = @{}
  $outputsByCallId = @{}

  foreach ($entry in (Get-TranscriptObjects -TranscriptPath $TranscriptPath)) {
    if ($entry.type -eq 'event_msg' -and $entry.payload.type -eq 'exec_command_end') {
      if ($entry.payload.call_id -and $entry.payload.turn_id) {
        $turnIdsByCallId[$entry.payload.call_id] = $entry.payload.turn_id
      }

      continue
    }

    if ($entry.type -ne 'response_item') {
      continue
    }

    if ($entry.payload.type -eq 'function_call' -and $entry.payload.name -eq 'shell_command') {
      $argumentsObject = Parse-JsonString -Raw $entry.payload.arguments
      $shellCommandsByCallId[$entry.payload.call_id] = @{
        command = if ($argumentsObject -and $argumentsObject.command) { [string]$argumentsObject.command } else { '' }
        workdir = if ($argumentsObject -and $argumentsObject.workdir) { [string]$argumentsObject.workdir } else { '' }
      }

      continue
    }

    if ($entry.payload.type -eq 'function_call_output' -and $entry.payload.call_id) {
      $outputsByCallId[$entry.payload.call_id] = [string]$entry.payload.output
    }
  }

  $results = New-Object System.Collections.Generic.List[object]
  foreach ($callId in $turnIdsByCallId.Keys) {
    if ($turnIdsByCallId[$callId] -ne $TurnId) {
      continue
    }

    if (-not $shellCommandsByCallId.ContainsKey($callId)) {
      continue
    }

    $commandInfo = $shellCommandsByCallId[$callId]
    $output = if ($outputsByCallId.ContainsKey($callId)) { $outputsByCallId[$callId] } else { '' }
    $results.Add([PSCustomObject]@{
      call_id = $callId
      command = $commandInfo.command
      workdir = $commandInfo.workdir
      output = $output
      exit_code = Get-ExitCodeFromToolOutput -Output $output
    })
  }

  return $results
}

function Test-IsVerificationCommand {
  param([string]$Command)

  if ([string]::IsNullOrWhiteSpace($Command)) {
    return $false
  }

  $normalized = $Command.ToLowerInvariant()
  $patterns = @(
    '\bvitest\b',
    '\bjest\b',
    '\bplaywright\b',
    '\bcypress\b',
    '\bpytest\b',
    '\beslint\b',
    '\blint\b',
    '\bcargo\s+test\b',
    '\bdotnet\s+test\b',
    '\bdotnet\s+build\b',
    '\bpnpm\s+(exec\s+)?test\b',
    '\bnpm\s+run\s+test\b',
    '\byarn\s+test\b',
    '\btsc\b',
    '\bbuild\b'
  )

  foreach ($pattern in $patterns) {
    if ($normalized -match $pattern) {
      return $true
    }
  }

  return $false
}

function Compress-Whitespace {
  param([string]$Text)

  if ([string]::IsNullOrWhiteSpace($Text)) {
    return ''
  }

  return (($Text -replace '\r', '') -replace '\n{3,}', "`n`n").Trim()
}

function Shorten-Text {
  param(
    [string]$Text,
    [int]$MaxLength = 1200
  )

  $normalized = Compress-Whitespace -Text $Text
  if ($normalized.Length -le $MaxLength) {
    return $normalized
  }

  return $normalized.Substring(0, $MaxLength).TrimEnd() + '...'
}

function Get-TurnMarker {
  param([string]$TurnId)

  return '<!-- codex-turn:{0} -->' -f $TurnId
}

function Test-HistoryHasTurnMarker {
  param(
    [string]$HistoryPath,
    [string]$TurnId
  )

  if (-not (Test-Path $HistoryPath -PathType Leaf)) {
    return $false
  }

  $raw = Get-Content -Path $HistoryPath -Raw
  return $raw.Contains((Get-TurnMarker -TurnId $TurnId))
}

function Ensure-HistoryTurnSection {
  param([string]$HistoryPath)

  if (-not (Test-Path $HistoryPath -PathType Leaf)) {
    return
  }

  $raw = Get-Content -Path $HistoryPath -Raw
  if ($raw -match '(?m)^## Codex Hook 턴 기록\s*$') {
    return
  }

  Add-Content -Path $HistoryPath -Encoding utf8 -Value "`r`n## Codex Hook 턴 기록`r`n"
}

function Append-HistoryEntry {
  param(
    [string]$HistoryPath,
    [string]$TurnId,
    [string]$Prompt,
    [string]$AssistantSummary,
    [object[]]$VerificationCommands
  )

  Ensure-HistoryTurnSection -HistoryPath $HistoryPath

  $timestamp = [DateTimeOffset]::Now.ToString('yyyy-MM-dd HH:mm:ss zzz')
  $marker = Get-TurnMarker -TurnId $TurnId
  $promptText = if ([string]::IsNullOrWhiteSpace($Prompt)) { '(prompt unavailable)' } else { Shorten-Text -Text $Prompt -MaxLength 500 }
  $assistantText = if ([string]::IsNullOrWhiteSpace($AssistantSummary)) { '(assistant summary unavailable)' } else { Shorten-Text -Text $AssistantSummary -MaxLength 1200 }

  $lines = New-Object System.Collections.Generic.List[string]
  $lines.Add('')
  $lines.Add('### {0} / turn `{1}`' -f $timestamp, $TurnId)
  $lines.Add('')
  $lines.Add($marker)
  $lines.Add('')
  $lines.Add('Prompt:')
  $lines.Add('- {0}' -f ($promptText -replace '\r?\n', ' '))
  $lines.Add('')
  $lines.Add('Implementation / response summary:')
  $lines.Add('- {0}' -f ($assistantText -replace '\r?\n', ' '))
  $lines.Add('')
  $lines.Add('Verification commands:')

  foreach ($commandResult in $VerificationCommands) {
    $exitCodeText = if ($null -eq $commandResult.exit_code) { 'exit ?' } else { 'exit {0}' -f $commandResult.exit_code }
    $commandText = Shorten-Text -Text $commandResult.command -MaxLength 220
    $lines.Add('- `{0}` -> {1}' -f $commandText, $exitCodeText)
  }

  $lines.Add('')
  $lines.Add('Hook note:')
  $lines.Add('- This entry was appended by the repo-local Codex Stop hook using the transcript payload for the completed turn.')

  Add-Content -Path $HistoryPath -Encoding utf8 -Value ($lines -join "`r`n")
}

function Write-HookOutputJson {
  param([hashtable]$Payload)

  $Payload | ConvertTo-Json -Depth 20 -Compress
}
