# board_test_app — GDB smoke test on a Linux target

Tiny C program: infinite loop, `volatile` globals/locals you can watch in GDB (`print g_counter`, breakpoints on `printf`, etc.).

## First-time target setup (SSH)

Before debugging with **`transport = remote_ssh`** (so `scp` + `ssh` work without typing a password every time):

1. **Install your public key on the board** (one-time). From the **repository root**:
   ```bash
   ./examples/board_test_app/install_ssh_key.sh
   ```
   Defaults match `rsgdb.remote.toml` in this directory (`fio` @ `192.168.2.139`). Use `SSH_HOST`, `SSH_USER`, `SSH_PORT`, or `./examples/board_test_app/install_ssh_key.sh <host> <user>` to match your board. For a non-interactive password on that first run: `export RSGDB_SSH_PASSWORD=…` (requires **`sshpass`**).
2. Confirm **`ssh`** to the target works without a password.
3. Continue with **Build** and **Debug** below. See the main README **Setting up a Linux target for `remote_ssh` debugging** for the full project-wide checklist.

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
