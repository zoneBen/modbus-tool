$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

if (-not (Test-Path "node_modules")) {
    pnpm install
}

Write-Host "Starting browser demo (no Rust required)..." -ForegroundColor Cyan
pnpm dev
