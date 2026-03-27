#!/usr/bin/env bash
# Build a tiny Zephyr app (scripts/zephyr_multi_printf_app) for native_sim, then:
#   gdbserver -> rsgdb -> GDB (batch): break first printf, next, next; check RSGDB_E2E log lines.
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
#   ZEPHYR_APP_SOURCE_DIR=/abs/path/to/app   (west -s; default: rsgdb/scripts/zephyr_multi_printf_app)
#   ZEPHYR_BOARD=native_sim/native/64
#   RSGDB, GDB_PORT, PROXY_PORT
#
# CI: not run by default; set RUN_E2E_ZEPHYR_NATIVE=1 in validate_local.sh locally.
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

if ! command -v gcc >/dev/null 2>&1 && ! command -v clang >/dev/null 2>&1; then
  echo "error: need a host C compiler (gcc or clang) for native_sim." >&2
  exit 1
fi

ZEPHYR_APP_SOURCE_DIR="${ZEPHYR_APP_SOURCE_DIR:-$ROOT/scripts/zephyr_multi_printf_app}"
ZEPHYR_BOARD="${ZEPHYR_BOARD:-native_sim/native/64}"
# First printf() in zephyr_multi_printf_app/src/main.c (keep in sync with that file).
FIRST_PRINTF_LINE=9

APP_MAIN_SRC="$ZEPHYR_APP_SOURCE_DIR/src/main.c"
if [[ ! -f "$APP_MAIN_SRC" ]]; then
  echo "error: missing $APP_MAIN_SRC" >&2
  exit 1
fi

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"; kill ${RSGDB_PID:-0} ${GDBSERVER_PID:-0} 2>/dev/null || true' EXIT

GDB_PORT="${GDB_PORT:-13335}"
PROXY_PORT="${PROXY_PORT:-13336}"
BUILD_DIR="$WORKDIR/native_sim_build"
GDBSERVER_LOG="$WORKDIR/gdbserver.log"

echo "==> west build -b $ZEPHYR_BOARD -s $ZEPHYR_APP_SOURCE_DIR (first run can take several minutes)"
(
  cd "$ZEPHYR_WORKSPACE"
  west build -b "$ZEPHYR_BOARD" -p auto -d "$BUILD_DIR" -s "$ZEPHYR_APP_SOURCE_DIR" -- \
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

echo "==> gdbserver 127.0.0.1:$GDB_PORT (log: $GDBSERVER_LOG)"
gdbserver "127.0.0.1:$GDB_PORT" "$ZEPHYR_EXE" >"$GDBSERVER_LOG" 2>&1 &
GDBSERVER_PID=$!
wait_listen "$GDB_PORT"

echo "==> rsgdb :$PROXY_PORT -> 127.0.0.1:$GDB_PORT"
"$RSGDB" --port "$PROXY_PORT" --target-host 127.0.0.1 --target-port "$GDB_PORT" &
RSGDB_PID=$!
wait_listen "$PROXY_PORT"

echo "==> gdb batch: break first printf (line $FIRST_PRINTF_LINE), continue, next, next"
OUT=$(gdb -nx --batch \
  -ex "set pagination off" \
  -ex "set debuginfod enabled off" \
  -ex "target extended-remote 127.0.0.1:$PROXY_PORT" \
  -ex "break \"$APP_MAIN_SRC\":$FIRST_PRINTF_LINE" \
  -ex "continue" \
  -ex "next" \
  -ex "next" \
  -ex "quit" \
  "$ZEPHYR_EXE" 2>&1) || true

echo "--- gdb output ---"
echo "$OUT"
echo "--- gdbserver / inferior log ($GDBSERVER_LOG) ---"
cat "$GDBSERVER_LOG"
echo "--- end logs ---"

if ! echo "$OUT" | grep -qE 'Breakpoint|Temporary breakpoint'; then
  echo "error: expected GDB to set a breakpoint" >&2
  exit 1
fi

if ! echo "$OUT" | grep -qF "main.c:$FIRST_PRINTF_LINE"; then
  echo "error: expected GDB to reference main.c:$FIRST_PRINTF_LINE" >&2
  exit 1
fi

if ! grep -qF 'RSGDB_E2E line 1' "$GDBSERVER_LOG"; then
  echo "error: expected inferior log to contain RSGDB_E2E line 1 (after first next)" >&2
  exit 1
fi

if ! grep -qF 'RSGDB_E2E line 2' "$GDBSERVER_LOG"; then
  echo "error: expected inferior log to contain RSGDB_E2E line 2 (after second next)" >&2
  exit 1
fi

echo "==> OK — Zephyr native_sim stepped printfs through rsgdb; log markers matched."
