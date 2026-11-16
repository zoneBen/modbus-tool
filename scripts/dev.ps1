$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Rust toolchain not found. This machine is CI-first." -ForegroundColor Yellow
    Write-Host "Use scripts/dev-web.ps1 for the browser demo, or install rustup + MSVC Build Tools." -ForegroundColor Yellow
    exit 1
}

if (-not (Test-Path "node_modules")) {
    pnpm install
}

pnpm tauri dev
