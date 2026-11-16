#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

if [ ! -d node_modules ]; then
  pnpm install
fi

pnpm lint
pnpm typecheck
pnpm test:unit
pnpm format:check
pnpm build

if command -v cargo >/dev/null 2>&1; then
  cargo fmt --all --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
else
  echo "Rust toolchain missing: skipped fmt/clippy/test. GitHub Actions runs them on Windows."
fi
