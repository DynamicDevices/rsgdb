#!/usr/bin/env bash
# Build Zephyr hello_world for native_sim (runs as a normal Linux process), then:
#   gdbserver -> rsgdb -> GDB (batch).
#
# Use this to exercise rsgdb against a real embedded-style ELF without hardware.
#
# Prerequisites (host):
#   - Zephyr west workspace with SDK / toolchain (Getting Started guide).
#   - gcc, gdb, gdbserver on PATH (same as scripts/e2e_gdb_smoke.sh).
#
# Usage:
#   export ZEPHYR_WORKSPACE=/path/to/zephyrproject   # contains .west/ and zephyr/
#   ./scripts/e2e_zephyr_native_sim.sh
#
# Optional:
#   ZEPHYR_APP=zephyr/samples/hello_world   (path relative to workspace root)
#   ZEPHYR_BOARD=native_sim/native/64       (default: 64-bit LP64 — works on typical x86_64 Linux without gcc-multilib)
#   RSGDB, GDB_PORT, PROXY_PORT — same as e2e_gdb_smoke.sh
#
# CI: not run by default (heavy); set RUN_E2E_ZEPHYR_NATIVE=1 in validate_local.sh locally.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RSGDB="${RSGDB:-$ROOT/target/release/rsgdb}"

if [[ ! -x "$RSGDB" ]]; then
  echo "error: rsgdb binary not found or not executable: $RSGDB" >&2
  echo "  build with: cargo build --release" >&2
  exit 1
fi

if [[ -z "${ZEPHYR_WORKSPACE:-}" ]]; then
  echo "error: set ZEPHYR_WORKSPACE to your west workspace root (directory with .west/ and zephyr/)." >&2
  echo "  See: https://docs.zephyrproject.org/latest/develop/getting_started/index.html" >&2
  exit 1
fi

if [[ ! -f "$ZEPHYR_WORKSPACE/.west/config" ]]; then
  echo "error: $ZEPHYR_WORKSPACE/.west/config not found (not a west workspace root?)." >&2
  exit 1
fi

for cmd in gdb gdbserver west; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "error: required command not found: $cmd" >&2
    exit 1
  fi
done

# native_sim links with host gcc; ensure a C compiler exists for the Zephyr build.
if ! command -v gcc >/dev/null 2>&1 && ! command -v clang >/dev/null 2>&1; then
  echo "error: need a host C compiler (gcc or clang) for native_sim." >&2
  exit 1
fi

ZEPHYR_APP="${ZEPHYR_APP:-zephyr/samples/hello_world}"
# Default board is 64-bit native_sim so linking does not require 32-bit multilib (-m32 + libgcc).
ZEPHYR_BOARD="${ZEPHYR_BOARD:-native_sim/native/64}"
WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"; kill ${RSGDB_PID:-0} ${GDBSERVER_PID:-0} 2>/dev/null || true' EXIT

GDB_PORT="${GDB_PORT:-13335}"
PROXY_PORT="${PROXY_PORT:-13336}"
BUILD_DIR="$WORKDIR/native_sim_build"

echo "==> west build -b $ZEPHYR_BOARD (first run can take several minutes)"
(
  cd "$ZEPHYR_WORKSPACE"
  west build -b "$ZEPHYR_BOARD" -p auto -d "$BUILD_DIR" "$ZEPHYR_APP" -- \
    -DCONFIG_NO_OPTIMIZATIONS=y
)

ZEPHYR_EXE="$BUILD_DIR/zephyr/zephyr.exe"
if [[ ! -x "$ZEPHYR_EXE" ]]; then
  echo "error: expected executable not found: $ZEPHYR_EXE" >&2
  exit 1
fi

wait_listen() {
  local port="$1"
  local i=0
  while [[ "$i" -lt 120 ]]; do
    if command -v ss >/dev/null 2>&1; then
      if ss -tln 2>/dev/null | grep -qE ":${port}\\s"; then
        return 0
      fi
    else
      sleep 0.4
      return 0
    fi
    sleep 0.05
    i=$((i + 1))
  done
  echo "error: timeout waiting for LISTEN on 127.0.0.1:$port" >&2
  return 1
}

echo "==> gdbserver 127.0.0.1:$GDB_PORT (Zephyr native_sim zephyr.exe)"
gdbserver "127.0.0.1:$GDB_PORT" "$ZEPHYR_EXE" &
GDBSERVER_PID=$!
wait_listen "$GDB_PORT"

echo "==> rsgdb :$PROXY_PORT -> 127.0.0.1:$GDB_PORT"
"$RSGDB" --port "$PROXY_PORT" --target-host 127.0.0.1 --target-port "$GDB_PORT" &
RSGDB_PID=$!
wait_listen "$PROXY_PORT"

# hello_world sample: printf is on line 11 of src/main.c (see Zephyr tree).
HELLO_SRC="$ZEPHYR_WORKSPACE/zephyr/samples/hello_world/src/main.c"
if [[ ! -f "$HELLO_SRC" ]]; then
  echo "error: expected $HELLO_SRC (hello_world sample)" >&2
  exit 1
fi

echo "==> gdb batch (host gdb — breakpoint on hello_world printf line)"
OUT=$(gdb -nx --batch \
  -ex "set pagination off" \
  -ex "target extended-remote 127.0.0.1:$PROXY_PORT" \
  -ex "break \"$HELLO_SRC\":11" \
  -ex "continue" \
  -ex "list" \
  -ex "quit" \
  "$ZEPHYR_EXE" 2>&1) || true

echo "$OUT"

if ! echo "$OUT" | grep -qE 'Breakpoint|Temporary breakpoint'; then
  echo "error: expected GDB to set a breakpoint" >&2
  exit 1
fi

if ! echo "$OUT" | grep -qE 'hello_world/src/main\.c:11|main\.c:11'; then
  echo "error: expected GDB to stop at hello_world/src/main.c line 11 (printf)" >&2
  exit 1
fi

echo "==> OK — Zephyr native_sim debug session through rsgdb succeeded."
