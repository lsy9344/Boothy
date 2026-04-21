param(
    [Parameter(Mandatory = $true)]
    [string]$Prompt,

    [string]$Preset = "look2",

    [int]$CaptureCount = 5,

    [string]$PhoneLastFour = "",

    [string]$BaseDir = "",

    [string]$OutputDir = "",

    [switch]$SkipAppLaunch
)

$ErrorActionPreference = "Stop"

function Get-DescendantProcessIds {
    param(
        [Parameter(Mandatory = $true)]
        [int]$ParentProcessId
    )

    $descendantIds = New-Object System.Collections.Generic.List[int]
    $pendingIds = New-Object System.Collections.Generic.Queue[int]
    $pendingIds.Enqueue($ParentProcessId)

    while ($pendingIds.Count -gt 0) {
        $currentParentId = $pendingIds.Dequeue()
        $childProcesses = @(Get-CimInstance Win32_Process -Filter "ParentProcessId = $currentParentId" -ErrorAction SilentlyContinue)

        foreach ($childProcess in $childProcesses) {
            if ($descendantIds.Contains($childProcess.ProcessId)) {
                continue
            }

            $descendantIds.Add($childProcess.ProcessId)
            $pendingIds.Enqueue($childProcess.ProcessId)
        }
    }

    return $descendantIds.ToArray()
}

function Stop-ProcessTree {
    param(
        [Parameter(Mandatory = $true)]
        [System.Diagnostics.Process]$Process
    )

    if ($null -eq $Process) {
        return
    }

    try {
        if ($Process.HasExited) {
            return
        }
    }
    catch {
        return
    }

    $descendantIds = @(Get-DescendantProcessIds -ParentProcessId $Process.Id)
    [array]::Reverse($descendantIds)

    foreach ($descendantId in $descendantIds) {
        try {
            Stop-Process -Id $descendantId -Force -ErrorAction Stop
        }
        catch {
        }
    }

    try {
        Stop-Process -Id $Process.Id -Force -ErrorAction Stop
    }
    catch {
    }
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$tauriRoot = Join-Path $repoRoot "src-tauri"

Push-Location $tauriRoot

$launchedApp = $null
$launchedDevServer = $null
$previousRuntimeProfile = $env:BOOTHY_RUNTIME_PROFILE
$previousAdminAuthenticated = $env:BOOTHY_ADMIN_AUTHENTICATED

try {
    cargo build --bin hardware-validation-runner

    $runnerExe = Join-Path $tauriRoot "target\debug\hardware-validation-runner.exe"

    if (-not (Test-Path $runnerExe)) {
        throw "hardware-validation-runner.exe not found at $runnerExe"
    }

    if (-not $SkipAppLaunch) {
        $env:BOOTHY_RUNTIME_PROFILE = "operator-enabled"
        $env:BOOTHY_ADMIN_AUTHENTICATED = "true"
        $devCommand = "pnpm tauri dev --no-watch"
        $launchedDevServer = Start-Process -FilePath "powershell" -ArgumentList @(
            "-NoLogo",
            "-Command",
            $devCommand
        ) -WorkingDirectory $repoRoot -PassThru -WindowStyle Hidden
        if ($null -eq $previousRuntimeProfile) {
            Remove-Item Env:BOOTHY_RUNTIME_PROFILE -ErrorAction SilentlyContinue
        }
        else {
            $env:BOOTHY_RUNTIME_PROFILE = $previousRuntimeProfile
        }
        if ($null -eq $previousAdminAuthenticated) {
            Remove-Item Env:BOOTHY_ADMIN_AUTHENTICATED -ErrorAction SilentlyContinue
        }
        else {
            $env:BOOTHY_ADMIN_AUTHENTICATED = $previousAdminAuthenticated
        }
        Start-Sleep -Seconds 8
    }

    $arguments = @(
        "--prompt", $Prompt,
        "--preset", $Preset,
        "--capture-count", $CaptureCount.ToString(),
        "--skip-app-launch"
    )

    if ($PhoneLastFour -ne "") {
        $arguments += @("--phone-last-four", $PhoneLastFour)
    }

    if ($BaseDir -ne "") {
        $arguments += @("--base-dir", $BaseDir)
    }

    if ($OutputDir -ne "") {
        $arguments += @("--output-dir", $OutputDir)
    }

    & $runnerExe @arguments
    $exitCode = $LASTEXITCODE
    exit $exitCode
}
finally {
    if ($null -eq $previousRuntimeProfile) {
        Remove-Item Env:BOOTHY_RUNTIME_PROFILE -ErrorAction SilentlyContinue
    }
    else {
        $env:BOOTHY_RUNTIME_PROFILE = $previousRuntimeProfile
    }

    if ($null -eq $previousAdminAuthenticated) {
        Remove-Item Env:BOOTHY_ADMIN_AUTHENTICATED -ErrorAction SilentlyContinue
    }
    else {
        $env:BOOTHY_ADMIN_AUTHENTICATED = $previousAdminAuthenticated
    }

    if ($launchedApp) {
        Stop-ProcessTree -Process $launchedApp
    }

    if ($launchedDevServer) {
        Stop-ProcessTree -Process $launchedDevServer
    }

    Pop-Location
}
