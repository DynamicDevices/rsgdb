# rsgdb - Enhanced GDB Server/Proxy

[![CI](https://github.com/DynamicDevices/rsgdb/workflows/CI/badge.svg)](https://github.com/DynamicDevices/rsgdb/actions)
[![Zephyr E2E](https://github.com/DynamicDevices/rsgdb/workflows/Zephyr%20E2E/badge.svg)](https://github.com/DynamicDevices/rsgdb/actions/workflows/zephyr-e2e.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/rsgdb.svg)](https://crates.io/crates/rsgdb)

A Rust GDB **RSP proxy** with structured logging, optional CMSIS-SVD memory labels, JSONL session recording/replay, RTOS packet decode logs, and external flash orchestration — with room to grow into richer breakpoints and UIs.

## 🎯 Project Goals

**rsgdb** aims to bridge the gap between traditional GDB debugging and modern embedded development needs by providing:

- **Enhanced Visibility**: Comprehensive logging of all GDB protocol traffic with structured output
- **Advanced Breakpoint Management** (roadmap): Named breakpoints, conditional expressions, and hardware/software optimization — config and parsing exist; the proxy today **forwards** breakpoint RSP unchanged
- **State Inspection** (partial today): Peripheral/register **labels** for memory traffic via CMSIS-SVD in logs; snapshots / deep state are not implemented yet
- **Session Management**: JSONL **recording** and **`rsgdb replay`** mock-backend playback (see below)
- **Backend Flexibility** (today): The proxy speaks **TCP** to whatever GDB stub you run (OpenOCD, probe-rs GDB port, `gdbserver`, …). `[backend].backend_type` / `--backend` is reserved for **future** native integration ([#9](https://github.com/DynamicDevices/rsgdb/issues/9)); it does not select a transport yet.
- **Modern Architecture**: Built with Rust for safety, performance, and reliability

## ✨ Key Features

### Current (v0.1.0 - In Development)
- ✅ GDB Remote Serial Protocol (RSP) codec + command parse (see `tests/rsp_codec_matrix.rs`)
- ✅ Transparent TCP proxy (integration tests; forwards RSP bytes)
- ✅ Structured logging (`tracing`, config-driven)
- ✅ TOML + env configuration
- 💾 **Session recording (rsgdb-record v1)** — ordered RSP trace as JSON Lines (`.jsonl`)
- ▶️ **`rsgdb replay`** — load a recording and serve a **mock TCP backend** for one client (order-preserving playback / tests)
- 📝 **SVD annotation (read-only)** — CMSIS-SVD → log labels for memory RSP (`m` / `M`): `Peripheral.REGISTER`, overlapping **fields**, and enumerated **variant names** where present (`target: rsgdb::svd`, debug)
- ⚡ **`rsgdb flash`** — run a configured external flash tool (`[flash].program` with `{image}` substitution; OpenOCD/probe-rs/etc.)
- 🧵 **RTOS RSP decode / log (Zephyr-first)** — thread-extension packets are decoded and logged at `target: rsgdb::rtos` (debug). Thread *data* comes from your stub (e.g. OpenOCD **Zephyr** RTOS awareness); other RTOSes use the same GDB RSP when the stub implements them (see below).
- 🧪 **CI + local E2E smoke** — `gdbserver` → `rsgdb` → `gdb` (batch), `scripts/e2e_gdb_smoke.sh` (Ubuntu job in **CI** workflow). **Zephyr `native_sim`** E2E (`scripts/e2e_zephyr_native_sim.sh`) runs in the **Zephyr E2E** workflow when those scripts/app change, on `main`/`develop`, weekly, or manually. See [CONTRIBUTING.md](CONTRIBUTING.md).
- ✅ **Phase A (trust path)** — RSP codec matrix tests (`tests/rsp_codec_matrix.rs`, `scripts/e2e_rsp_regression.sh`), proxy TCP integration tests (`tests/proxy_integration.rs`).
- 📎 **Phase B (GDB productivity)** — [`scripts/gdbinit.rsgdb.example`](scripts/gdbinit.rsgdb.example), `qSupported`-style proxy test, backend thread-reply summaries in `rsgdb::rtos` (decode/log only).

### Planned
- 📊 Enhanced logging with filtering and export (JSON, CSV)
- 🎯 Advanced breakpoint management wired into the proxy (today: config + RSP parse; not a full manager on the wire)
- 🔍 State tracking and visualization (beyond SVD-annotated memory logs)
- 🔌 Native / non-TCP backends and richer probe integration ([#9](https://github.com/DynamicDevices/rsgdb/issues/9); CLI `backend_type` is reserved)
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
# Used for logging / future integration; the proxy connects over TCP to target_host:target_port
backend_type = "openocd"

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

This README and [CONTRIBUTING.md](CONTRIBUTING.md). Design notes: [docs/ADR-001-breakpoints-semihosting.md](docs/ADR-001-breakpoints-semihosting.md) (breakpoint policy + semihosting spike).

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
4. Run tests (`cargo test`)
5. Run formatting (`cargo fmt`)
6. Run linting (`cargo clippy`)
7. Commit your changes (`git commit -m 'feat: add amazing feature'`)
8. Push to the branch (`git push origin feature/amazing-feature`)
9. Open a Pull Request

## 📋 Roadmap

Source of truth for ordering and scope: **[GitHub Issues](https://github.com/DynamicDevices/rsgdb/issues)** (labels `roadmap`, `enhancement`).

| Milestone (docs) | What it means | Issue |
|------------------|---------------|-------|
| **Foundation + proxy** | RSP codec, TCP proxy, config, logging, CI (incl. GDB + Zephyr E2E), session record (JSONL), SVD labels, flash orchestration, RTOS decode/log | Closed: [#1–#8](https://github.com/DynamicDevices/rsgdb/issues?q=is%3Aissue+is%3Aclosed) |
| **Next: native backend** | Probe-facing backend beyond TCP to a stub (see `backends::connect_tcp_backend`) | [#9](https://github.com/DynamicDevices/rsgdb/issues/9) (open) |
| **Replay** | `rsgdb replay` + mock TCP backend from `.jsonl` | [#10](https://github.com/DynamicDevices/rsgdb/issues/10) (closed) |
| **Richer SVD** | Overlapping fields + enum variant names in annotations; value decode / recording correlation follow-ups | [#11](https://github.com/DynamicDevices/rsgdb/issues/11) (baseline closed; follow-ups optional) |

Older versioned bullets (v0.2–v0.4) below are **aspirational**; issue titles supersede them.

### Aspirational (not scheduled per-issue yet)
- Enhanced logging export (JSON/CSV), advanced breakpoints, TUI, performance work — see **Planned** under Key Features and open an issue when starting.

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

**Status**: 🚧 Early development — CI green on `main` (multi-OS tests, GDB smoke, Zephyr `native_sim` E2E). Not a substitute for a production-qualified probe stack until native backends and release hardening land; see issues above.