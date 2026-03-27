#!/usr/bin/env bash
# Phase A — fast RSP regression: codec matrix + lib codec tests + proxy integration (no gdb).
# Run from repo root: ./scripts/e2e_rsp_regression.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "==> RSP codec matrix (integration test crate)"
cargo test --all-features --test rsp_codec_matrix

echo "==> Protocol codec unit tests (lib)"
cargo test --all-features -- protocol::codec::tests::

echo "==> Proxy TCP integration tests"
cargo test --all-features --test proxy_integration

echo "==> OK — RSP regression passed."
