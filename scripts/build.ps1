$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Local Rust toolchain not found." -ForegroundColor Yellow
    Write-Host "Trigger the Windows build in GitHub Actions:" -ForegroundColor Cyan
    Write-Host "  Repository -> Actions -> 'Release Windows' -> Run workflow"
    Write-Host "Download the NSIS/MSI installers from the run artifacts."
    exit 1
}

if (-not (Test-Path "node_modules")) {
    pnpm install
}

pnpm tauri build --bundles nsis,msi
