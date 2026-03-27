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

echo "==> OK — all local checks passed."
