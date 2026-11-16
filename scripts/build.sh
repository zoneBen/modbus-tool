#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

if ! command -v cargo >/dev/null 2>&1; then
  echo "Local Rust toolchain not found."
  echo "Trigger the Windows build in GitHub Actions: Repository -> Actions -> 'Release Windows' -> Run workflow."
  exit 1
fi

if [ ! -d node_modules ]; then
  pnpm install
fi

pnpm tauri build --bundles nsis,msi
