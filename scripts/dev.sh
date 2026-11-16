#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

if ! command -v cargo >/dev/null 2>&1; then
  echo "Rust toolchain not found. Use scripts/dev-web.sh for the browser demo, or install rustup."
  exit 1
fi

if [ ! -d node_modules ]; then
  pnpm install
fi

pnpm tauri dev
