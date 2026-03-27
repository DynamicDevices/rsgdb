# board_test_app — GDB smoke test on a Linux target

Tiny C program: infinite loop, `volatile` globals/locals you can watch in GDB (`print g_counter`, breakpoints on `printf`, etc.).

Validated with rsgdb **v0.2.0-dev.1** using **`transport = remote_ssh`** and **`gdbserver`** on the target; see root [`CHANGELOG.md`](../../CHANGELOG.md).

## First-time target setup (SSH)

Before debugging with **`transport = remote_ssh`** (so `scp` + `ssh` work without typing a password every time):

1. **Install your public key on the board** (one-time). From the **repository root**:
   ```bash
   ./examples/board_test_app/install_ssh_key.sh
   ```
   Defaults match `rsgdb.remote.toml` in this directory (`fio` @ `192.168.2.139`). Use `SSH_HOST`, `SSH_USER`, `SSH_PORT`, or `./examples/board_test_app/install_ssh_key.sh <host> <user>` to match your board. For a non-interactive password on that first run: `export RSGDB_SSH_PASSWORD=…` (requires **`sshpass`**).
2. Confirm **`ssh`** to the target works without a password.
3. Continue with **Build** and **Debug** below. See the main README **Setting up a Linux target for `remote_ssh` debugging** for the full project-wide checklist.

## Configure target IP, deploy, and debug

**Remote deploy + debug** is driven by [`rsgdb.remote.toml`](rsgdb.remote.toml): **`rsgdb`** **`scp`**s your built ELF to the board, **`ssh`** starts **`gdbserver`** there, then **GDB** (CLI or Cursor) talks to **`127.0.0.1:<listen_port>`** on your PC while the proxy forwards to the board.

### 1. Point at your board (IP, user, ports)

Edit **`examples/board_test_app/rsgdb.remote.toml`** (paths are relative to the **repo root** when you run **`rsgdb`** from there):

| Setting | Meaning |
|--------|--------|
| **`[proxy] target_host`** | Board **IP or hostname** (SSH and TCP target when **`[backend.remote_ssh] host`** is unset). |
| **`[proxy] target_port`** | Port **`gdbserver`** listens on **on the board** (must match **`{port}`** in **`program`**). |
| **`[proxy] listen_port`** | Port **on your PC** where **GDB** attaches (**`127.0.0.1:this`**). Default **3333** — if you change it, update [`.vscode/launch.json`](../../.vscode/launch.json) **`miDebuggerServerAddress`**. |
| **`[backend.remote_ssh] user`** | SSH login on the target. |
| **`upload_local` / `upload_remote`** | Host path → path on the board for **`scp`** before **`gdbserver`**. |
| **`program`** | Remote command; must include **`{port}`** (same value as **`target_port`**). |

**Override IP/port without committing edits to the TOML** (applied after the file is loaded): create **`examples/board_test_app/rsgdb.env`** from [`rsgdb.env.example`](rsgdb.env.example) (that file is **gitignored**). Set **`RSGDB_TARGET_HOST`**, optional **`RSGDB_TARGET_PORT`**, **`RSGDB_PORT`**, or **`RSGDB_SSH_PASSWORD`**. Cursor’s preLaunch task runs [`run_rsgdb_proxy.sh`](run_rsgdb_proxy.sh), which **`source`**s **`rsgdb.env`** if present, then starts **`rsgdb`**.

```bash
cp examples/board_test_app/rsgdb.env.example examples/board_test_app/rsgdb.env
# edit IP / password in rsgdb.env
```

### 2. Start debugging from Cursor

1. **`make`** / build **`board_test_app`** (the preLaunch task does this).
2. Run **`rsgdb: board_test_app (build, start proxy, debug)`** — it starts **`rsgdb`** with this TOML, which **deploys** the binary and **starts gdbserver** on the target, then attaches **gdb-multiarch** to **localhost:3333** (or whatever **`listen_port`** / **`RSGDB_PORT`** you use).

If **`rsgdb`** is already running in a terminal with the right env/config, use **`rsgdb: board_test_app (proxy already running)`** instead.

### 3. Same flow on the CLI

```bash
cd /path/to/rsgdb
./examples/board_test_app/run_rsgdb_proxy.sh
# or: export RSGDB_TARGET_HOST=… && ./target/release/rsgdb --config examples/board_test_app/rsgdb.remote.toml
```

In another terminal: **`gdb-multiarch`** → **`target extended-remote 127.0.0.1:3333`** with **`file`** set to the ELF (see **Automated** debug section below).

## Build (aarch64 default)

The Makefile defaults to **`aarch64-linux-gnu-gcc`**:

```bash
cd examples/board_test_app
make
```

Host-only smoke build (x86_64): `make CC=gcc`

With a **Yocto/Poky SDK** (adjust path), use the SDK compiler:

```bash
source /path/to/sdk/environment-setup-aarch64-poky-linux
cd examples/board_test_app
make CC="$CC"
```

## Deploy to the board

**Manual:** `scp` the binary and `chmod +x` on the target.

**With rsgdb** (`transport = remote_ssh`): set `upload_local` to this built `board_test_app` path and `upload_remote` to e.g. `/tmp/board_test_app` — rsgdb can **`scp`** before starting `gdbserver` (see main README).

## Debug (gdbserver on board)

### Automated (rsgdb `remote_ssh` + `scp`)

From the **repository root**, with the board reachable at the address in `rsgdb.remote.toml` (default `192.168.2.139`):

```bash
cargo build --release
cd examples/board_test_app && make && cd ../..
export RSGDB_SSH_PASSWORD=…   # if using password auth; prefer SSH keys
./examples/board_test_app/debug_remote.sh
```

Use **`gdb-multiarch`** on Ubuntu (not always `aarch64-linux-gnu-gdb`). Pass an **absolute** path to `file` if you run GDB by hand:

```bash
gdb-multiarch -ex "set debuginfod enabled off" \
  -ex "file $(pwd)/examples/board_test_app/board_test_app" \
  -ex "target extended-remote 127.0.0.1:3333"
```

### Visual debug (VS Code / Cursor)

Use the **repository root** as the workspace folder so `${workspaceFolder}` resolves correctly.

1. Install the **C/C++** extension (`ms-vscode.cpptools`) — Cursor/VS Code may prompt from [`.vscode/extensions.json`](../../.vscode/extensions.json).
2. Install **`gdb-multiarch`** on the host (e.g. `sudo apt install gdb-multiarch`).
3. **SSH**: same as above — keys or `RSGDB_SSH_PASSWORD` for non-interactive `scp`/`ssh`. For password auth, start VS Code from a shell where `RSGDB_SSH_PASSWORD` is exported, or rely on SSH keys.
4. **Run and Debug** (sidebar): choose **`rsgdb: board_test_app (build, start proxy, debug)`** in the **dropdown** (not “No Configurations”), then press **F5** or **Start Debugging**. The preLaunch task runs [`prepare_cursor_debug.sh`](prepare_cursor_debug.sh): build **`rsgdb`** if needed, **`make`** the ELF, start **`rsgdb`** via [`run_rsgdb_proxy.sh`](run_rsgdb_proxy.sh), and **wait until TCP 127.0.0.1:3333** is listening (then **cppdbg** attaches with **`useExtendedRemote`**).
5. If **`rsgdb` is already running**, use **`rsgdb: board_test_app (proxy already running)`** so the preLaunch task does not start a second proxy.

**If nothing happens or it hangs:**  
- Install **C/C++** (`ms-vscode.cpptools`) and **reload** the window; without **`cppdbg`**, debug may not start.  
- Watch the **Terminal** panel for the preLaunch task — you should see `==> rsgdb is listening...`. If you see an error, check **`/tmp/rsgdb-cursor-3333.log`**.  
- Ensure the **launch configuration name** is selected in the Run and Debug dropdown (not empty).  
- Port **3333** in use: stop the other process or change **`listen_port`** / **`RSGDB_PORT`** and [`.vscode/launch.json`](../../.vscode/launch.json) **`miDebuggerServerAddress`** to match.  
- Only **one** GDB client at a time through the proxy.

### Manual gdbserver on target

On the **target**:

```bash
/tmp/board_test_app &
/tmp/gdbserver :2345 --attach $!
# or: gdbserver :2345 /tmp/board_test_app
```

On the **host**: `transport = tcp` in rsgdb; point GDB at `127.0.0.1:<rsgdb listen port>`.

```text
(gdb) file /absolute/path/to/board_test_app
(gdb) target extended-remote 127.0.0.1:3333
```
