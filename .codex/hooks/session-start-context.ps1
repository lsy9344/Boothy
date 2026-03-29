$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'camera-history-hook-common.ps1')

$payload = Read-HookPayload
$repoRoot = Get-GitRoot -FallbackCwd $payload.cwd
$historyPath = Get-HistoryDocPath -RepoRoot $repoRoot

$context = @"
Use $historyPath as the running history for the camera capture troubleshooting work.
For each user turn in this repository:
1. review the latest notes in that history before deciding;
2. after implementation or investigation, run relevant verification commands and report the results;
3. record cause analysis, attempts, implementation, verification, and test results in the same history document;
4. if a verification command does not apply, explicitly state why in both the response and the history.
Follow the same workflow even when hooks do not fire. The official Codex hooks page notes that native Windows Codex CLI currently disables hooks.
"@.Trim()

Write-HookOutputJson -Payload @{
  hookSpecificOutput = @{
    hookEventName = 'SessionStart'
    additionalContext = $context
  }
}
