#!/usr/bin/env bash
# One-shot prep for Cursor/VS Code "Run and Debug": build ELF + rsgdb, wait until the proxy port listens.
# More reliable than a background task + problemMatcher (often hangs in Cursor).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

LISTEN_PORT="${RSGDB_PORT:-3333}"
LOG="${TMPDIR:-/tmp}/rsgdb-cursor-${LISTEN_PORT}.log"

echo "==> rsgdb board_test_app: prepare for debug (port ${LISTEN_PORT})" >&2

if [[ ! -x "${ROOT}/target/release/rsgdb" ]]; then
	echo "==> Building rsgdb (release)…" >&2
	( cd "$ROOT" && cargo build --release )
fi

echo "==> Building board_test_app…" >&2
make -C "${ROOT}/examples/board_test_app"

echo "==> Freeing TCP ${LISTEN_PORT} if in use…" >&2
fuser -k "${LISTEN_PORT}/tcp" 2>/dev/null || true
sleep 0.3

echo "==> Starting rsgdb (log: ${LOG})…" >&2
: >"$LOG"
nohup "${ROOT}/examples/board_test_app/run_rsgdb_proxy.sh" >>"$LOG" 2>&1 &
RPID=$!

wait_for_port() {
	local i
	for ((i = 0; i < 450; i++)); do
		if ! kill -0 "$RPID" 2>/dev/null; then
			echo "rsgdb exited before listening. Log:" >&2
			cat "$LOG" >&2
			return 1
		fi
		if ss -lnt 2>/dev/null | grep -qE ":${LISTEN_PORT}\\s"; then
			return 0
		fi
		if command -v nc >/dev/null 2>&1 && nc -z 127.0.0.1 "${LISTEN_PORT}" 2>/dev/null; then
			return 0
		fi
		sleep 0.2
	done
	echo "Timeout waiting for 127.0.0.1:${LISTEN_PORT}. Log:" >&2
	cat "$LOG" >&2
	return 1
}

if wait_for_port; then
	echo "==> rsgdb is listening on 127.0.0.1:${LISTEN_PORT} (pid ${RPID}). You can start GDB." >&2
	exit 0
fi
exit 1
