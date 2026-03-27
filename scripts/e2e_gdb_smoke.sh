#!/usr/bin/env bash
# Simulated end-to-end debug session: gdbserver -> rsgdb -> GDB (batch).
# Intended for Linux/macOS CI and local smoke. Requires: gcc, gdb, gdbserver, bash.
# Uses `ss` to wait for LISTEN when available (avoids bogus TCP probes to gdbserver).
#
# Ubuntu/Debian: gdbserver is often a separate package:
#   sudo apt-get install -y gcc gdb gdbserver
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RSGDB="${RSGDB:-$ROOT/target/release/rsgdb}"

if [[ ! -x "$RSGDB" ]]; then
  echo "error: rsgdb binary not found or not executable: $RSGDB" >&2
  echo "  build with: cargo build --release" >&2
  exit 1
fi

for cmd in gcc gdb gdbserver; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "error: required command not found: $cmd" >&2
    if [[ "$cmd" == "gdbserver" ]]; then
      echo "  on Debian/Ubuntu try: sudo apt-get install -y gdbserver" >&2
    fi
    exit 1
  fi
done

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"; kill ${RSGDB_PID:-0} ${GDBSERVER_PID:-0} 2>/dev/null || true' EXIT

GDB_PORT="${GDB_PORT:-13333}"
PROXY_PORT="${PROXY_PORT:-13334}"

cat >"$WORKDIR/smoke.c" <<'EOF'
volatile int g;
int main(void) {
  g = 42;
  return g - 42;
}
EOF

gcc -g -O0 -o "$WORKDIR/smoke" "$WORKDIR/smoke.c"

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

echo "==> gdbserver 127.0.0.1:$GDB_PORT (backend)"
gdbserver "127.0.0.1:$GDB_PORT" "$WORKDIR/smoke" &
GDBSERVER_PID=$!
wait_listen "$GDB_PORT"

echo "==> rsgdb :$PROXY_PORT -> 127.0.0.1:$GDB_PORT"
"$RSGDB" --port "$PROXY_PORT" --target-host 127.0.0.1 --target-port "$GDB_PORT" &
RSGDB_PID=$!
wait_listen "$PROXY_PORT"

echo "==> gdb batch (via proxy)"
OUT=$(gdb -nx --batch \
  -ex "set pagination off" \
  -ex "target extended-remote 127.0.0.1:$PROXY_PORT" \
  -ex "break main" \
  -ex "continue" \
  -ex "next" \
  -ex "print /d g" \
  -ex "quit" \
  "$WORKDIR/smoke" 2>&1) || true

echo "$OUT"

if ! echo "$OUT" | grep -qE '\$[0-9]+ = 42'; then
  echo "error: expected GDB value line like '\$1 = 42' (after next over assignment)" >&2
  exit 1
fi

echo "==> OK — simulated debugging session through rsgdb succeeded."
