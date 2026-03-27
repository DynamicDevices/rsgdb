#!/usr/bin/env bash
# Local CI parity check (Linux/macOS/Git Bash). Run from repository root.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "==> cargo fmt --check"
cargo fmt --all -- --check

echo "==> cargo test --all-features"
cargo test --all-features

echo "==> cargo clippy --all-targets --all-features -- -D warnings"
cargo clippy --all-targets --all-features -- -D warnings

echo "==> cargo doc (deny warnings)"
export RUSTDOCFLAGS='-D warnings'
cargo doc --no-deps --all-features

if [[ "${RUN_E2E_GDB:-}" == "1" ]]; then
  echo "==> e2e gdb smoke (gdbserver -> rsgdb -> gdb batch)"
  cargo build --release
  ./scripts/e2e_gdb_smoke.sh
fi

echo "==> OK — all local checks passed."
