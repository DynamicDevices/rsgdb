#!/usr/bin/env bash
# Optional dependency hygiene: duplicates, security advisories, newer crate versions.
# Install helpers once (add ~/.cargo/bin to PATH): cargo install cargo-audit cargo-outdated
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "==> cargo tree -d (duplicate crate versions in the graph)"
cargo tree -d

echo ""
echo "==> cargo audit (RustSec advisory DB against Cargo.lock)"
cargo audit

echo ""
echo "==> cargo outdated --workspace"
if cargo outdated --workspace 2>/dev/null; then
  :
else
  echo "(Skipped — install cargo-outdated and ensure it is on PATH: cargo install cargo-outdated)"
fi

echo ""
echo "==> OK — dependency checks finished."
