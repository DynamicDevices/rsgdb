#!/usr/bin/env bash
# Start rsgdb with examples/board_test_app/rsgdb.remote.toml, optionally after
# sourcing examples/board_test_app/rsgdb.env (gitignored). Used by .vscode/tasks.json.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

ENV_FILE="${ROOT}/examples/board_test_app/rsgdb.env"
if [[ -f "$ENV_FILE" ]]; then
	# shellcheck source=/dev/null
	set -a
	source "$ENV_FILE"
	set +a
fi

exec "${ROOT}/target/release/rsgdb" --config "${ROOT}/examples/board_test_app/rsgdb.remote.toml"
