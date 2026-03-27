# rsgdb - Enhanced GDB Server/Proxy

[![CI](https://github.com/DynamicDevices/rsgdb/workflows/CI/badge.svg)](https://github.com/DynamicDevices/rsgdb/actions)
[![Zephyr E2E](https://github.com/DynamicDevices/rsgdb/workflows/Zephyr%20E2E/badge.svg)](https://github.com/DynamicDevices/rsgdb/actions/workflows/zephyr-e2e.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/rsgdb.svg)](https://crates.io/crates/rsgdb)

A Rust GDB **RSP proxy** with structured logging, optional CMSIS-SVD memory labels, JSONL session recording/replay, RTOS packet decode logs, and external flash orchestration — with room to grow into richer breakpoints and UIs.

## Design principles

rsgdb exists to make **remote debugging practical for embedded developers**: boards on the bench, in the lab, or over the network, without unnecessary ceremony. **Automation** and **reliability** are first-class goals, not afterthoughts.

| Principle | What it means in practice |
|-----------|---------------------------|
| **Ease** | Few steps from “target is reachable” to GDB inspecting your ELF; sensible defaults; documented flows (config + helper scripts) that match how teams actually work. |
| **Automation** | One place to describe the target (address, transport, optional binary upload); **`remote_ssh`** with optional **`scp`** so you are not hand-copying builds; preflight checks (e.g. SSH key access) before a long attach fails halfway. |
| **Reliability** | Fail **early** with **actionable** errors (auth, ports, firewall, stub not listening); predictable behavior and exit codes; logging you can use in the field; CI and local E2E smoke where we can prove the path. |

These principles align with **zero-touch remote debugging** below and inform roadmap work (fewer manual steps, clearer credential UX, stronger health checks over time).

## 🎯 Project Goals

**rsgdb** aims to bridge the gap between traditional GDB debugging and modern embedded development needs by providing:

- **Enhanced Visibility**: Comprehensive logging of all GDB protocol traffic with structured output
- **Advanced Breakpoint Management** (roadmap): Named breakpoints, conditional expressions, and hardware/software optimization — config and parsing exist; the proxy today **forwards** breakpoint RSP unchanged
- **State Inspection** (partial today): Peripheral/register **labels** for memory traffic via CMSIS-SVD in logs; snapshots / deep state are not implemented yet
- **Session Management**: JSONL **recording** and **`rsgdb replay`** mock-backend playback (see below)
- **Backend Flexibility** (today): **`transport = tcp`** connects to an existing stub on **`proxy.target_host`:`proxy.target_port`**. **`transport = native`** spawns a configured command (`[backend.spawn] program` with `{port}`), waits for TCP on `bind_host`, then uses the same RSP path; the child is killed when the GDB session ends ([#9](https://github.com/DynamicDevices/rsgdb/issues/9)). **`transport = remote_ssh`** runs **`gdbserver` on the target** over SSH; optional **`upload_local` / `upload_remote`** runs **`scp`** first so you are not forced to copy binaries by hand. `[backend].backend_type` / `--backend` is a **label** for logs only.
- **Zero-touch remote debugging** (direction / aim): Reduce manual steps for **Linux targets** (e.g. Yocto boards): one config with **target address**, **login** (SSH user), and **credential** (SSH key or `RSGDB_SSH_PASSWORD` + `sshpass`) should drive **upload → gdbserver → proxy → GDB** where possible; more automation and polish are expected over time — see **Design principles** above and roadmap below.
- **Modern Architecture**: Built with Rust for safety, performance, and reliability

## ✨ Key Features

### Current (**v0.2.0-dev.1** — development release)

See [`CHANGELOG.md`](CHANGELOG.md) and [`RELEASING.md`](RELEASING.md). Pre-release versions may change; pin carefully in automation.
- ✅ GDB Remote Serial Protocol (RSP) codec + command parse (see `tests/rsp_codec_matrix.rs`)
- ✅ Transparent TCP proxy (integration tests; forwards RSP bytes)
- ✅ Structured logging (`tracing`, config-driven)
- ✅ TOML + env configuration
- 💾 **Session recording (rsgdb-record v1)** — ordered RSP trace as JSON Lines (`.jsonl`)
- ▶️ **`rsgdb replay`** — load a recording and serve a **mock TCP backend** for one client (order-preserving playback / tests)
- 📝 **SVD annotation (read-only)** — CMSIS-SVD → log labels for memory RSP (`m` / `M`): `Peripheral.REGISTER`, overlapping **fields**, and enumerated **variant names** where present (`target: rsgdb::svd`, debug)
- ⚡ **`rsgdb flash`** — run a configured external flash tool (`[flash].program` with `{image}` substitution; OpenOCD/probe-rs/etc.)
- 🔌 **`transport = native`** — spawn a GDB stub **on this machine** per session (`[backend.spawn]` + `{port}`); teardown on disconnect
- 🖧 **`transport = remote_ssh`** — run **`gdbserver` on the target** via **`ssh user@host …`** (`[backend.remote_ssh]` + `{port}`); optional **`upload_local`/`upload_remote`** → **`scp`** first; TCP to `proxy.target_host`:`proxy.target_port`; optional `RSGDB_SSH_PASSWORD` + `sshpass`
- 🧵 **RTOS RSP decode / log (Zephyr-first)** — thread-extension packets are decoded and logged at `target: rsgdb::rtos` (debug). Thread *data* comes from your stub (e.g. OpenOCD **Zephyr** RTOS awareness); other RTOSes use the same GDB RSP when the stub implements them (see below).
- 🧪 **CI + local E2E smoke** — `gdbserver` → `rsgdb` → `gdb` (batch), `scripts/e2e_gdb_smoke.sh` (Ubuntu job in **CI** workflow). **Zephyr `native_sim`** E2E (`scripts/e2e_zephyr_native_sim.sh`) runs in the **Zephyr E2E** workflow when those scripts/app change, on `main`/`develop`, weekly, or manually. See [CONTRIBUTING.md](CONTRIBUTING.md).
- ✅ **Phase A (trust path)** — RSP codec matrix tests (`tests/rsp_codec_matrix.rs`, `scripts/e2e_rsp_regression.sh`), proxy TCP integration tests (`tests/proxy_integration.rs`).
- 📎 **Phase B (GDB productivity)** — [`scripts/gdbinit.rsgdb.example`](scripts/gdbinit.rsgdb.example), `qSupported`-style proxy test, backend thread-reply summaries in `rsgdb::rtos` (decode/log only).

### Planned
- 📊 Enhanced logging with filtering and export (JSON, CSV)
- 🎯 Advanced breakpoint management wired into the proxy (today: config + RSP parse; not a full manager on the wire)
- 🔍 State tracking and visualization (beyond SVD-annotated memory logs)
- 🔌 Deeper probe integration (beyond managed TCP spawn; CLI `backend_type` remains a label)
- 🖥️ Terminal UI (TUI) for interactive debugging
- 📝 SVD: decode register **values** to enum names on the wire, and **correlation** with session recordings ([#11](https://github.com/DynamicDevices/rsgdb/issues/11) follow-ups)

## Continuous integration

| Workflow | What runs |
|----------|-----------|
| **CI** (`.github/workflows/ci.yml`) | `cargo fmt`, multi-OS `cargo test`, clippy, **E2E GDB smoke** (Ubuntu: `scripts/e2e_gdb_smoke.sh`), `cargo doc`, **tarpaulin** coverage (Codecov), release `cargo build` |
| **Zephyr E2E** (`.github/workflows/zephyr-e2e.yml`) | West workspace + `native_sim` build, then same chain as local `RUN_E2E_ZEPHYR_NATIVE=1` (`scripts/e2e_zephyr_native_sim.sh`). Triggered by path filters, `main`/`develop` pushes, a weekly schedule, or **workflow_dispatch** |

Both workflows support **workflow_dispatch** (run manually from the Actions tab).

## 🚀 Quick Start

### Installation

```bash
# From source (recommended during development)
git clone https://github.com/DynamicDevices/rsgdb.git
cd rsgdb
cargo build --release

# The binary will be at target/release/rsgdb
```

### Basic Usage

```bash
# Start rsgdb in proxy mode (forwards GDB on 3333 to OpenOCD on 3334)
rsgdb --backend openocd --port 3333 --target-host localhost --target-port 3334

# Connect with GDB
arm-none-eabi-gdb
(gdb) target extended-remote localhost:3333
```

### Wiring: stub → rsgdb → GDB (Phase A)

rsgdb is a **transparent TCP proxy**: GDB speaks RSP to rsgdb; rsgdb forwards the same bytes to your **debug stub** (OpenOCD, `gdbserver`, probe-rs GDB port, etc.). Nothing is rewritten on the wire unless you add higher layers later.

| Role | Typical bind | GDB connects to |
|------|----------------|-----------------|
| **Stub** (OpenOCD, gdbserver, …) | `host:3334` (example) | — |
| **rsgdb** | `0.0.0.0:3333` → `target_host:target_port` | `target extended-remote host:3333` |
| **GDB** | — | rsgdb listen port |

**Choosing `tcp` vs `native` vs `remote_ssh`**

| Use | When |
|-----|------|
| **`transport = tcp`** (default) | The stub is **already running** (you started OpenOCD, probe-rs gdb, gdbserver, …). rsgdb **dials** `proxy.target_host`:`proxy.target_port`. |
| **`transport = native`** | You want rsgdb to **spawn** the stub **on this machine** per GDB session with `[backend.spawn] program` and `{port}`, then connect to `bind_host` at that port. Kills the stub when GDB disconnects. |
| **`transport = remote_ssh`** | The stub runs **on a remote host** (e.g. Yocto board). Optional **`upload_local`** + **`upload_remote`** run **`scp`** first (same auth as SSH). Then rsgdb runs **`ssh user@host …`** with `[backend.remote_ssh] program` (must include `{port}` → `proxy.target_port`), then connects TCP to **`proxy.target_host`:`proxy.target_port`**. Kills the **local** `ssh` process when GDB disconnects (typically ends remote `gdbserver`). Requires **OpenSSH** `ssh`/`scp` on PATH; optional **`RSGDB_SSH_PASSWORD`** + **`sshpass`** for non-interactive password auth. |

#### Setting up a Linux target for `remote_ssh` debugging

Do this **once per host/user** so `ssh`, `scp`, and rsgdb agree on the same auth (no password prompts in normal use).

1. **Install your SSH public key on the target (recommended)** — from the repository root, run [`examples/board_test_app/install_ssh_key.sh`](examples/board_test_app/install_ssh_key.sh). Defaults match the example [`examples/board_test_app/rsgdb.remote.toml`](examples/board_test_app/rsgdb.remote.toml) (`fio` @ `192.168.2.139`). Override with `SSH_HOST`, `SSH_USER`, `SSH_PORT`, or positional `host` / `user`:
   ```bash
   ./examples/board_test_app/install_ssh_key.sh
   ```
   If you must pass the account password non-interactively (e.g. first-time automation), set `RSGDB_SSH_PASSWORD` and install **`sshpass`**; the script uses the same variable as rsgdb.
2. **Or use password auth** — omit keys and rely on interactive prompts, or set **`RSGDB_SSH_PASSWORD`** + **`sshpass`** for rsgdb/`scp` (see table above).
3. **Verify** — `ssh -p <port> user@host` should succeed without a password after step 1. Then use your `[backend.remote_ssh]` + `[proxy]` config, or follow [`examples/board_test_app/README.md`](examples/board_test_app/README.md) for a full smoke (`debug_remote.sh`).

**Config:** `[proxy] listen_port`, `target_host`, `target_port`. **`timeout_secs`** applies only to **establishing** the TCP connection to the backend, not to idle GDB sessions (no read timeout on open connections).

**Common issues**

| Symptom | Things to check |
|---------|-------------------|
| Connection refused to rsgdb | rsgdb not running or wrong `--port` / `RSGDB_PORT`. |
| Connection refused through rsgdb | Stub not listening on `target_host:target_port`; firewall. |
| GDB hangs | Backend disconnected; use logs / `RUST_LOG=debug`. |
| Windows `connect` to `0.0.0.0` | Use `127.0.0.1` or the actual listener address from `ss` / `netstat`. |

**Fast RSP regression (no gdb binary):** `./scripts/e2e_rsp_regression.sh` runs codec + proxy integration tests only.

### GDB productivity (Phase B)

- **Example GDB init:** [`scripts/gdbinit.rsgdb.example`](scripts/gdbinit.rsgdb.example) — `pagination off`, optional `set debug remote 1`, and a commented `target extended-remote` line. Copy or `gdb -x scripts/gdbinit.rsgdb.example` after adjusting the port.
- **Transparency:** integration tests include a **`qSupported:…`-style** packet round-trip so negotiation-shaped payloads are not mangled by the proxy.
- **Thread replies (read-only logs):** when the stub sends thread-list / `QC` / hex name replies, **`rsgdb::rtos`** logs a short summary for **backend → client** packets (same wire bytes; no RSP injection).

Enable with e.g. `RUST_LOG=rsgdb::rtos=debug,rsgdb=info`.

### Configuration

Create a `rsgdb.toml` configuration file:

```toml
[proxy]
listen_port = 3333
target_host = "localhost"
target_port = 3334

[backend]
# Label for logs (openocd, probe-rs, gdbserver, …)
backend_type = "openocd"
# tcp = existing stub on target_host:target_port; native = spawn [backend.spawn] with {port}
transport = "tcp"

[logging]
level = "debug"
format = "json"
output = "rsgdb.log"

[breakpoints]
auto_optimize = true
max_hardware = 6

[recording]
enabled = false
output_dir = "./recordings"
max_size_mb = 100

[svd]
# Optional: path to CMSIS-SVD XML for memory access labels in logs
path = "device.svd"
```

**Managed stub (`transport = native`):** set `transport = "native"` and define `[backend.spawn]` with a `program` array that includes the literal `{port}`. rsgdb picks an ephemeral port, substitutes it into argv, spawns the process, and connects to `bind_host` (default `127.0.0.1`) at that port. **`proxy.target_host` / `target_port` are not used** for this path. When GDB disconnects, the stub process is killed. If the stub exits before TCP is ready, or a connect times out, error messages point at argv / `bind_host` / `ready_timeout_secs`, and **include a tail of stub stderr** when the process produced any. Use `RUST_LOG=debug` for resolved spawn argv (`spawning native GDB stub subprocess`) and **`RUST_LOG=rsgdb::stub_stderr=debug`** (or `debug` globally) for **line-by-line stub stderr** while the process runs. See `rsgdb.toml.example` for a commented template.

### SVD labels (read-only)

If `[svd] path` points to a valid CMSIS-SVD file (or use `--svd FILE` / env `RSGDB_SVD`), rsgdb builds a register map and emits **debug** logs for **client** memory packets (`m` read, `M` write) with a human-readable range when the access overlaps known registers — for example `GPIOA.MODER (4 bytes)` or, when the SVD lists fields, `GPIOA.MODER (4 bytes); fields: GPIOA.MODER.MODE0 [Input, Output], …`. Enumerated value **names** from the SVD are shown alongside fields; decoding actual register **values** against those enums is not implemented yet. This is **display only**; RSP bytes are unchanged.

Enable the log target with e.g. `RUST_LOG=rsgdb::svd=debug,rsgdb=info` (or `-d` / verbose per your logging setup).

### Session recording (rsgdb-record v1)

When enabled, each GDB↔backend connection writes one **JSON Lines** file under `recording.output_dir`. Line 1 is a header (`format`, `version`, `session_id`, `started_at`). Later lines are RSP events: `direction` (`client_to_backend` / `backend_to_client`), `kind` (`packet` / `ack` / `nack`), and for packets `payload_hex` / `payload_len` (hex is the raw packet payload bytes, not the `$…#xx` framing).

**Enable:** `rsgdb --record`, or set `[recording] enabled = true` in config, or `RSGDB_RECORD=1`. Optional directory override: `--record-dir DIR` or `RSGDB_RECORD_DIR`.

**Replay:** `rsgdb replay <FILE.jsonl> [--listen ADDR]` (default `127.0.0.1:3334`) loads an `rsgdb-record` v1 session and listens for TCP; the **first** GDB client connection is served by a mock backend that replays `backend_to_client` / expects `client_to_backend` events in order (for regression tests and inspection). You can still inspect raw `.jsonl` with `jq` or other tools. Tracked as [#10](https://github.com/DynamicDevices/rsgdb/issues/10).

### Flash orchestration (`rsgdb flash`)

rsgdb does not embed flash algorithms; it **runs a configured external command** (OpenOCD `program`, `probe-rs download`, a wrapper script, etc.). Put an argv template in `[flash].program`; at least one element must contain the placeholder `{image}`, which is replaced by the **absolute** path to the firmware file you pass on the CLI.

```toml
[flash]
program = [
  "openocd",
  "-f", "board.cfg",
  "-c", "init; reset halt; program {image} verify; reset run; shutdown",
]
```

```bash
rsgdb flash --config rsgdb.toml build/zephyr/zephyr.signed.bin
```

Stdin is closed; stdout/stderr are inherited so tool output appears in your terminal. Non-zero exit status is surfaced as an error.

### RTOS awareness — which RTOS / how Zephyr fits in

rsgdb does **not** ship RTOS kernels or flash algorithms. **Which threads exist and their names** are produced by the **GDB remote stub** (OpenOCD with an RTOS plugin, probe-rs, etc.) using GDB’s standard **thread-extension RSP**. The proxy stays transparent: it **forwards** packets and optionally **decodes/logs** them.

| Area | What we claim |
|------|----------------|
| **Zephyr** | **Primary / reference workflow.** Use a stub that exposes Zephyr threads to GDB (e.g. OpenOCD’s Zephyr RTOS support). rsgdb logs `qC`, `qfThreadInfo` / `qsThreadInfo`, `Hg` / `Hc`, `qThreadExtraInfo`, `qXfer:threads`, and `T…thread:…` stop replies at **`rsgdb::rtos`**. |
| **Other RTOSes** (FreeRTOS, ThreadX, …) | **Same RSP shapes** when your stub implements GDB thread extensions; rsgdb does not special-case them. If the stub does not expose threads, there is nothing for these packets to carry. |

Enable thread-oriented logs with e.g. `RUST_LOG=rsgdb::rtos=debug,rsgdb=info` (or `-d` / config as for other targets). Session **recordings** (JSONL) already capture raw RSP for later correlation; richer “thread timeline” analysis is future work.

## 📖 Documentation

This README, [CONTRIBUTING.md](CONTRIBUTING.md), [CHANGELOG.md](CHANGELOG.md), and [RELEASING.md](RELEASING.md) (maintainer release checklist). Design notes: [docs/ADR-001-breakpoints-semihosting.md](docs/ADR-001-breakpoints-semihosting.md) (breakpoint policy + semihosting spike).

**Visual debugging (VS Code / Cursor):** open the **repo root** as the workspace. Shared [`.vscode/launch.json`](.vscode/launch.json) and [`.vscode/tasks.json`](.vscode/tasks.json) drive **`rsgdb`** then attach **GDB** (`gdb-multiarch`) to **`127.0.0.1:<listen_port>`** — same RSP path as the CLI. See [`examples/board_test_app/README.md`](examples/board_test_app/README.md) § *Visual debug*.

## Releases

- **Development**: tagged pre-releases (e.g. **`v0.2.0-dev.1`**) are documented in [`CHANGELOG.md`](CHANGELOG.md); see [`RELEASING.md`](RELEASING.md) to cut the next tag.
- **crates.io**: the badge above reflects the latest **published** crate; git tags may be ahead until `cargo publish` is run.

## 🏗️ Architecture

```
┌─────────────┐
│  GDB Client │
└──────┬──────┘
       │ RSP Protocol
       ▼
┌─────────────────────────────────────┐
│         rsgdb Proxy Core            │
│  ┌──────────┐  ┌─────────────────┐ │
│  │ Protocol │  │  Breakpoints    │ │
│  │  Parser  │  │  (roadmap)      │ │
│  └──────────┘  └─────────────────┘ │
│  ┌──────────┐  ┌─────────────────┐ │
│  │  Logger  │  │ SVD / RTOS log  │ │
│  │          │  │ (decode only)   │ │
│  └──────────┘  └─────────────────┘ │
└──────────┬──────────────────────────┘
           │ TCP (RSP bytes forwarded)
           ▼
    ┌──────────────┐
    │Debug Backend │
    │(probe-rs/OCD)│
    └──────┬───────┘
           │ JTAG/SWD
           ▼
    ┌──────────────┐
    │Target Device │
    └──────────────┘
```

## 🛠️ Development

### Prerequisites

- Rust 1.70 or later
- A debug probe (for testing with real hardware)
- Optional: OpenOCD, probe-rs, or pyOCD

### Building

```bash
# Debug build
cargo build

# Release build with optimizations
cargo build --release

# Run tests
cargo test

# Match CI before pushing (fmt, clippy, tests --all-features, doc)
./scripts/validate_local.sh

# Optional: real GDB session through the proxy (needs gcc, gdb, gdbserver)
# cargo build --release && ./scripts/e2e_gdb_smoke.sh
# Or: RUN_E2E_GDB=1 ./scripts/validate_local.sh
#
# Optional: Zephyr native_sim (Linux ELF) — needs ZEPHYR_WORKSPACE; default board is native_sim/native/64
# ./scripts/e2e_zephyr_native_sim.sh
# Or: RUN_E2E_ZEPHYR_NATIVE=1 ./scripts/validate_local.sh

# Run with logging (also respects [logging] in rsgdb.toml after init)
RUST_LOG=debug cargo run
```

### Project Structure

```
rsgdb/
├── src/                     # Library + CLI
├── tests/                   # Integration tests
├── .vscode/                 # shared VS Code / Cursor: launch + tasks (board_test_app remote debug)
├── examples/board_test_app/ # remote Linux target smoke (Makefile, rsgdb.remote.toml, helper scripts)
├── CHANGELOG.md
├── RELEASING.md
├── scripts/validate_local.sh
├── scripts/deps_check.sh         # optional: duplicate deps, cargo audit, cargo outdated
├── scripts/e2e_gdb_smoke.sh      # gdbserver → rsgdb → gdb (batch); CI E2E job
├── scripts/e2e_zephyr_native_sim.sh  # optional: west build native_sim + multi-printf stepping test
├── scripts/e2e_rsp_regression.sh     # fast: codec + proxy integration only (no gdb)
├── scripts/gdbinit.rsgdb.example     # optional GDB init snippet for use with rsgdb
├── scripts/zephyr_multi_printf_app/  # tiny Zephyr app for that script (west -s)
├── rsgdb.toml.example
└── .github/workflows/
```

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. From the repo root, run **`./scripts/validate_local.sh`** (matches CI: fmt, `cargo test --all-features`, clippy `-D warnings`, `cargo doc` with warnings denied). Optional: `RUN_E2E_GDB=1` or `RUN_E2E_ZEPHYR_NATIVE=1` if you have those tools installed (see [CONTRIBUTING.md](CONTRIBUTING.md)).
5. Commit your changes (`git commit -m 'feat: add amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

## 📋 Roadmap

Source of truth for ordering and scope: **[GitHub Issues](https://github.com/DynamicDevices/rsgdb/issues)** (labels `roadmap`, `enhancement`).

| Milestone (docs) | What it means | Issue |
|------------------|---------------|-------|
| **Foundation + proxy** | RSP codec, TCP proxy, config, logging, CI (incl. GDB + Zephyr E2E), session record (JSONL), SVD labels, flash orchestration, RTOS decode/log | Closed: [#1–#8](https://github.com/DynamicDevices/rsgdb/issues?q=is%3Aissue+is%3Aclosed) |
| **Native spawn backend** | `BackendTransport::Native` + `[backend.spawn]` (`{port}`), managed `Child` lifecycle, stderr capture | [#9](https://github.com/DynamicDevices/rsgdb/issues/9) (implemented) |
| **Replay** | `rsgdb replay` + mock TCP backend from `.jsonl` | [#10](https://github.com/DynamicDevices/rsgdb/issues/10) (closed) |
| **Richer SVD** | Overlapping fields + enum variant names in annotations; value decode / recording correlation follow-ups | [#11](https://github.com/DynamicDevices/rsgdb/issues/11) (baseline closed; follow-ups optional) |
| **Zero-touch remote debug** | Fewer manual steps: remote IP + SSH identity + optional `scp` upload + `remote_ssh` gdbserver orchestration; expand credential UX and workflows over time | Aim (see **Project Goals**) |

Older versioned bullets (v0.2–v0.4) below are **aspirational**; issue titles supersede them.

### Aspirational (not scheduled per-issue yet)
- Enhanced logging export (JSON/CSV), advanced breakpoints, TUI, performance work — see **Planned** under Key Features and open an issue when starting.
- Deeper **zero-touch remote debugging** (beyond current `scp` + `remote_ssh`): e.g. integrated workflows, fewer external tools, clearer security story for secrets — track as project aim above.

## 📄 License

This project is dual-licensed under:

- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

You may choose either license for your use.

## 🙏 Acknowledgments

- The Rust embedded community
- GDB and OpenOCD projects
- probe-rs project for inspiration

## 📞 Contact

- GitHub Issues: [https://github.com/DynamicDevices/rsgdb/issues](https://github.com/DynamicDevices/rsgdb/issues)
- Repository: [https://github.com/DynamicDevices/rsgdb](https://github.com/DynamicDevices/rsgdb)

---

**Status**: 🚧 **Development release [`v0.2.0-dev.1`](CHANGELOG.md)** — CI green on `main` (multi-OS tests, GDB smoke, Zephyr `native_sim` E2E, native-spawn integration test with Python). **`remote_ssh`** + optional **`scp`** are in-tree for Linux-on-target debugging; deeper probe integration and stable **0.2.0** crates.io publish remain roadmap items; see issues and [`RELEASING.md`](RELEASING.md).