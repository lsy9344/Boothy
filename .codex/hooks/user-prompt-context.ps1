$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'camera-history-hook-common.ps1')

$payload = Read-HookPayload
$repoRoot = Get-GitRoot -FallbackCwd $payload.cwd
$historyPath = Get-HistoryDocPath -RepoRoot $repoRoot

if ($payload.session_id -and $payload.turn_id) {
  Save-TurnState -RepoRoot $repoRoot -SessionId ([string]$payload.session_id) -TurnId ([string]$payload.turn_id) -Prompt ([string]$payload.prompt)
}

$context = @"
Continue using $historyPath as the canonical troubleshooting log for the camera capture issue.
Before you finish this turn, make sure you run relevant verification commands, share the results, and append the outcome to that history document.
"@.Trim()

Write-HookOutputJson -Payload @{
  hookSpecificOutput = @{
    hookEventName = 'UserPromptSubmit'
    additionalContext = $context
  }
}
