$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

if (-not (Test-Path "node_modules")) {
    pnpm install
}

pnpm lint
pnpm typecheck
pnpm test:unit
pnpm format:check
pnpm build

if (Get-Command cargo -ErrorAction SilentlyContinue) {
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
} else {
    Write-Host "Rust toolchain missing: skipped fmt/clippy/test. GitHub Actions runs them on Windows." -ForegroundColor Yellow
}
