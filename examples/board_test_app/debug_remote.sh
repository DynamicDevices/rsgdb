#!/usr/bin/env bash
# End-to-end smoke: rsgdb (remote_ssh + scp) + gdb-multiarch to the board.
# Usage from repository root:
#   export RSGDB_SSH_PASSWORD=yourpassword   # if not using SSH keys
#   ./examples/board_test_app/debug_remote.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

BIN="${ROOT}/examples/board_test_app/board_test_app"
CFG="${ROOT}/examples/board_test_app/rsgdb.remote.toml"

if [[ ! -f "$BIN" ]]; then
	echo "Build the app first: cd examples/board_test_app && make" >&2
	exit 1
fi

RSGDB_BIN="${ROOT}/target/release/rsgdb"
if [[ ! -x "$RSGDB_BIN" ]]; then
	echo "Building rsgdb release…" >&2
	( cd "$ROOT" && cargo build --release )
fi

command -v gdb-multiarch >/dev/null || {
	echo "Install gdb-multiarch (e.g. sudo apt install gdb-multiarch)" >&2
	exit 1
}

# Match rsgdb.remote.toml (same defaults as install_ssh_key.sh). Skipped when using password auth.
check_ssh_key_access() {
	if [[ -n "${RSGDB_SSH_PASSWORD:-}" ]]; then
		echo "RSGDB_SSH_PASSWORD is set; skipping SSH key check." >&2
		return 0
	fi
	local ssh_host ssh_user ssh_port
	ssh_host=$(awk -F'"' '/target_host/ {print $2; exit}' "$CFG")
	ssh_user=$(awk -F'"' '/^user =/ {print $2; exit}' "$CFG")
	ssh_port="${SSH_PORT:-}"
	if [[ -z "$ssh_port" ]] && grep -qE '^ssh_port[[:space:]]*=' "$CFG" 2>/dev/null; then
		ssh_port=$(awk -F'=' '/^ssh_port/ {gsub(/[^0-9]/,"",$2); print $2; exit}' "$CFG")
	fi
	[[ -z "$ssh_port" ]] && ssh_port=22
	if [[ -z "$ssh_host" || -z "$ssh_user" ]]; then
		echo "Could not parse target_host / user from $CFG" >&2
		exit 1
	fi
	if ! ssh -p "$ssh_port" -o BatchMode=yes -o ConnectTimeout=10 -o StrictHostKeyChecking=accept-new \
		"${ssh_user}@${ssh_host}" "echo ok" >/dev/null 2>&1; then
		echo "SSH key access check failed for ${ssh_user}@${ssh_host} (port ${ssh_port})." >&2
		echo "Install your key: ./examples/board_test_app/install_ssh_key.sh" >&2
		echo "Or use password auth: export RSGDB_SSH_PASSWORD=... && $0" >&2
		exit 1
	fi
	echo "SSH key access OK (${ssh_user}@${ssh_host}:${ssh_port})" >&2
}

check_ssh_key_access

fuser -k 3333/tcp 2>/dev/null || true
sleep 1

echo "Starting rsgdb (scp + ssh gdbserver on connect)…" >&2
"$RSGDB_BIN" --config "$CFG" 2>&1 &
RPID=$!
sleep 2

cleanup() {
	kill "$RPID" 2>/dev/null || true
	wait "$RPID" 2>/dev/null || true
}
trap cleanup EXIT

echo "Connecting GDB (batch)…" >&2
gdb-multiarch -batch -nx \
	-ex "set debuginfod enabled off" \
	-ex "file $BIN" \
	-ex "target extended-remote 127.0.0.1:3333" \
	-ex "print g_counter" \
	-ex "detach" \
	-ex "quit"
echo "OK." >&2
