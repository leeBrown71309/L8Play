<#
  Builds a signed L8Play release: asks for the updater key password (never stored on disk), runs the
  Tauri build, then writes latest.json next to the installer. Publishing stays a separate, explicit step.

  Usage (from the repository root, in a normal PowerShell window):
    pwsh -File scripts/release-build.ps1 -Version 0.1.4 -NotesFile release-notes.md
#>
param(
  [Parameter(Mandatory = $true)][string]$Version,
  [Parameter(Mandatory = $true)][string]$NotesFile,
  [string]$KeyPath = "$env:USERPROFILE\.tauri\l8play-updater.key"
)

$ErrorActionPreference = "Stop"
Set-Location (Split-Path -Parent $PSScriptRoot)

if (-not (Test-Path $KeyPath)) { throw "Signing key not found: $KeyPath" }
if (-not (Test-Path $NotesFile)) { throw "Release notes file not found: $NotesFile" }
if (-not (Test-Path ".env")) { Write-Warning "No .env: the build will not embed the Google client." }

$secure = Read-Host "Signing key password" -AsSecureString
$env:TAURI_SIGNING_PRIVATE_KEY = $KeyPath
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = [System.Net.NetworkCredential]::new("", $secure).Password

try {
  Get-Process l8-play -ErrorAction SilentlyContinue | Stop-Process -Force
  Start-Sleep -Seconds 3
  bun tauri build
  if ($LASTEXITCODE -ne 0) { throw "bun tauri build failed" }
  bun scripts/make-update-manifest.ts --version $Version --notes-file $NotesFile
  if ($LASTEXITCODE -ne 0) { throw "Manifest generation failed" }
}
finally {
  # The password only ever lives in this process.
  Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
  Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
}

# The raw build output is not the installed app. Removing it keeps Windows search, Start and file finders
# from offering a second "l8-play.exe" that is not the installed release.
Remove-Item "targetelease\l8-play.exe" -Force -ErrorAction SilentlyContinue

Write-Host "Done: target/release/bundle/nsis/L8Play_${Version}_x64-setup.exe(.sig) and latest.json"
