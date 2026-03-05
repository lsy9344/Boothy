[CmdletBinding()]
param(
    [switch]$PersistProfile
)

$ErrorActionPreference = 'Stop'

# Apply UTF-8 to current console session.
chcp 65001 > $null
$utf8NoBom = [System.Text.UTF8Encoding]::new($false)
[Console]::InputEncoding = $utf8NoBom
[Console]::OutputEncoding = $utf8NoBom
$OutputEncoding = [Console]::OutputEncoding

if ($PersistProfile) {
    if (-not (Test-Path $PROFILE)) {
        New-Item -ItemType File -Path $PROFILE -Force | Out-Null
    }

    $profileBlock = @'
# UTF-8 console defaults
chcp 65001 > $null
[Console]::InputEncoding  = [System.Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$OutputEncoding = [Console]::OutputEncoding
'@

    $currentProfile = Get-Content -Path $PROFILE -Raw -ErrorAction SilentlyContinue
    $signature = '[System.Text.UTF8Encoding]::new($false)'
    if ($null -eq $currentProfile -or $currentProfile -notmatch [regex]::Escape($signature)) {
        Add-Content -Path $PROFILE -Value "`r`n$profileBlock`r`n" -Encoding utf8
        $profileMessage = "Profile updated: $PROFILE"
    }
    else {
        $profileMessage = "Profile already contains UTF-8 settings: $PROFILE"
    }
}

Write-Host 'UTF-8 settings applied for current session.'
Write-Host "InputEncoding : $([Console]::InputEncoding.WebName)"
Write-Host "OutputEncoding: $([Console]::OutputEncoding.WebName)"
if ($PersistProfile) {
    Write-Host $profileMessage
}
