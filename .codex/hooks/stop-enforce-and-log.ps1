param(
  [switch]$DryRun
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'camera-history-hook-common.ps1')

$payload = Read-HookPayload
$repoRoot = Get-GitRoot -FallbackCwd $payload.cwd
$historyPath = Get-HistoryDocPath -RepoRoot $repoRoot
$turnId = [string]$payload.turn_id
$sessionId = [string]$payload.session_id
$assistantSummary = Shorten-Text -Text ([string]$payload.last_assistant_message) -MaxLength 1400
$turnState = if ($sessionId -and $turnId) { Load-TurnState -RepoRoot $repoRoot -SessionId $sessionId -TurnId $turnId } else { $null }
$promptText = if ($turnState) { [string]$turnState.prompt } else { '' }
$verificationCommands = @()

if (-not [string]::IsNullOrWhiteSpace([string]$payload.transcript_path) -and -not [string]::IsNullOrWhiteSpace($turnId)) {
  $verificationCommands = Get-TurnShellCommandResults -TranscriptPath ([string]$payload.transcript_path) -TurnId $turnId |
    Where-Object { Test-IsVerificationCommand -Command $_.command }
}

if ($verificationCommands.Count -eq 0) {
  Write-HookOutputJson -Payload @{
    decision = 'block'
    reason = 'Before ending this turn, run relevant verification commands for your implementation or investigation, report the results, and update history/camera-capture-validation-history.md. If no verification command applies, explicitly say why and record that too.'
  }
  exit 0
}

if (-not $DryRun -and -not (Test-HistoryHasTurnMarker -HistoryPath $historyPath -TurnId $turnId)) {
  Append-HistoryEntry -HistoryPath $historyPath -TurnId $turnId -Prompt $promptText -AssistantSummary $assistantSummary -VerificationCommands $verificationCommands
}

Write-HookOutputJson -Payload @{
  continue = $true
  systemMessage = if ($DryRun) { 'Stop hook dry-run completed.' } else { 'Camera capture history hook logged this turn and detected verification commands.' }
}
