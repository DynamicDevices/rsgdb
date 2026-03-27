#!/usr/bin/env bash
# Install your SSH public key on the target (one-time) so rsgdb/scp use key auth.
# Defaults match examples/board_test_app/rsgdb.remote.toml (user fio, board 192.168.2.139).
#
# From repository root:
#   ./examples/board_test_app/install_ssh_key.sh
#
# Non-interactive (password once; requires sshpass: sudo apt install sshpass):
#   export RSGDB_SSH_PASSWORD=yourpassword
#   ./examples/board_test_app/install_ssh_key.sh
#
# Override:
#   SSH_HOST=192.168.1.22 SSH_USER=fio SSH_PORT=22 ./examples/board_test_app/install_ssh_key.sh
#   ./examples/board_test_app/install_ssh_key.sh 192.168.1.22 fio
#
# Force a specific public key file:
#   SSH_PUBKEY=~/.ssh/id_ed25519.pub ./examples/board_test_app/install_ssh_key.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

SSH_HOST="${1:-${SSH_HOST:-192.168.2.139}}"
SSH_USER="${2:-${SSH_USER:-fio}}"
SSH_PORT="${SSH_PORT:-22}"

PASS="${RSGDB_SSH_PASSWORD:-${SSH_PASSWORD:-}}"

resolve_pubkey() {
	if [[ -n "${SSH_PUBKEY:-}" ]]; then
		local p="${SSH_PUBKEY/#\~/$HOME}"
		if [[ -f "$p" ]]; then
			echo "$p"
			return 0
		fi
		echo "SSH_PUBKEY is set but not a file: $p" >&2
		exit 1
	fi
	local c
	for c in "$HOME/.ssh/id_ed25519.pub" "$HOME/.ssh/id_rsa.pub" "$HOME/.ssh/id_ecdsa.pub"; do
		if [[ -f "$c" ]]; then
			echo "$c"
			return 0
		fi
	done
	echo "No default public key found (~/.ssh/id_ed25519.pub, id_rsa.pub, id_ecdsa.pub)." >&2
	echo "Generate one: ssh-keygen -t ed25519 -C \"your@email\"" >&2
	echo "Or set SSH_PUBKEY=/path/to/key.pub" >&2
	exit 1
}

PUBKEY="$(resolve_pubkey)"
TARGET="${SSH_USER}@${SSH_HOST}"

ssh_base_args=( -p "$SSH_PORT" -o "StrictHostKeyChecking=accept-new" )

run_copy_id() {
	if command -v ssh-copy-id >/dev/null 2>&1; then
		if [[ -n "$PASS" ]]; then
			command -v sshpass >/dev/null 2>&1 || {
				echo "Set RSGDB_SSH_PASSWORD but sshpass is not installed (e.g. sudo apt install sshpass)." >&2
				exit 1
			}
			SSHPASS="$PASS" sshpass -e ssh-copy-id -i "$PUBKEY" "${ssh_base_args[@]}" "$TARGET"
		else
			ssh-copy-id -i "$PUBKEY" "${ssh_base_args[@]}" "$TARGET"
		fi
		return 0
	fi
	echo "ssh-copy-id not found; appending key via ssh." >&2
	if [[ -n "$PASS" ]]; then
		command -v sshpass >/dev/null 2>&1 || {
			echo "Install ssh-copy-id (openssh-client) or sshpass for password-based install." >&2
			exit 1
		}
		cat "$PUBKEY" | SSHPASS="$PASS" sshpass -e ssh "${ssh_base_args[@]}" "$TARGET" \
			"umask 077; mkdir -p .ssh && chmod 700 .ssh && cat >> .ssh/authorized_keys && chmod 600 .ssh/authorized_keys"
	else
		cat "$PUBKEY" | ssh "${ssh_base_args[@]}" "$TARGET" \
			"umask 077; mkdir -p .ssh && chmod 700 .ssh && cat >> .ssh/authorized_keys && chmod 600 .ssh/authorized_keys"
	fi
}

echo "Installing $(basename "$PUBKEY") -> ${TARGET} (port ${SSH_PORT})" >&2
echo "Repo root (for reference): ${ROOT}" >&2
run_copy_id

# Fail if we still cannot log in without a password (matches non-interactive scp/ssh expectations).
verify_ssh_key_access() {
	if ! ssh "${ssh_base_args[@]}" -o BatchMode=yes -o ConnectTimeout=10 "$TARGET" "echo ok" >/dev/null 2>&1; then
		echo "SSH key check failed: cannot log in to ${TARGET} without a password (BatchMode)." >&2
		echo "Fix: RSGDB_SSH_PASSWORD=... $0  # one-time install, or fix authorized_keys on the target" >&2
		exit 1
	fi
	echo "SSH key access OK (${TARGET})." >&2
}

verify_ssh_key_access
