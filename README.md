# rsgdb - Enhanced GDB Server/Proxy

[![CI](https://github.com/DynamicDevices/rsgdb/workflows/CI/badge.svg)](https://github.com/DynamicDevices/rsgdb/actions)
[![Zephyr E2E](https://github.com/DynamicDevices/rsgdb/workflows/Zephyr%20E2E/badge.svg)](https://github.com/DynamicDevices/rsgdb/actions/workflows/zephyr-e2e.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/rsgdb.svg)](https://crates.io/crates/rsgdb)

A modern, feature-rich GDB server/proxy written in Rust, designed to enhance embedded debugging workflows with advanced logging, state inspection, and breakpoint management capabilities.

## 🎯 Project Goals

**rsgdb** aims to bridge the gap between traditional GDB debugging and modern embedded development needs by providing:

- **Enhanced Visibility**: Comprehensive logging of all GDB protocol traffic with structured output
- **Advanced Breakpoint Management**: Named breakpoints, conditional expressions, and intelligent hardware/software optimization
- **State Inspection**: Memory snapshots, register tracking, and peripheral decoding using SVD files
- **Session Management**: Record, replay, and share debugging sessions
- **Backend Flexibility**: Support for multiple debug probes (probe-rs, OpenOCD, pyOCD)
- **Modern Architecture**: Built with Rust for safety, performance, and reliability

## ✨ Key Features

### Current (v0.1.0 - In Development)
- 🚧 GDB Remote Serial Protocol (RSP) parser
- 🚧 Basic proxy/pass-through mode
- 🚧 Structured logging infrastructure
- 🚧 Configuration system
- 💾 **Session recording (rsgdb-record v1)** — ordered RSP trace as JSON Lines (`.jsonl`)
- 📝 **SVD annotation (read-only)** — CMSIS-SVD file → log labels for memory RSP (`m` / `M`) as `Peripheral.REGISTER` (`target: rsgdb::svd`, debug level)
- ⚡ **`rsgdb flash`** — run a configured external flash tool (`[flash].program` with `{image}` substitution; OpenOCD/probe-rs/etc.)
- 🧵 **RTOS RSP decode / log (Zephyr-first)** — thread-extension packets are decoded and logged at `target: rsgdb::rtos` (debug). Thread *data* comes from your stub (e.g. OpenOCD **Zephyr** RTOS awareness); other RTOSes use the same GDB RSP when the stub implements them (see below).
- 🧪 **CI + local E2E smoke** — `gdbserver` → `rsgdb` → `gdb` (batch), `scripts/e2e_gdb_smoke.sh` (Ubuntu job in **CI** workflow). **Zephyr `native_sim`** E2E (`scripts/e2e_zephyr_native_sim.sh`) runs in the **Zephyr E2E** workflow when those scripts/app change, on `main`/`develop`, weekly, or manually. See [CONTRIBUTING.md](CONTRIBUTING.md).
- ✅ **Phase A (trust path)** — RSP codec matrix tests (`tests/rsp_codec_matrix.rs`, `scripts/e2e_rsp_regression.sh`), proxy TCP tests, ops matrix in README above.
- 📎 **Phase B (GDB productivity)** — [`scripts/gdbinit.rsgdb.example`](scripts/gdbinit.rsgdb.example), `qSupported`-style proxy test, backend thread-reply summaries in `rsgdb::rtos` (decode/log only).

### Planned
- 📊 Enhanced logging with filtering and export (JSON, CSV)
- 🎯 Advanced breakpoint management (named, conditional, grouped)
- 🔍 State tracking and visualization
- 💾 Session **replay** tooling (mock backend / automated playback)
- 🔌 Multiple backend support (probe-rs, OpenOCD)
- 🖥️ Terminal UI (TUI) for interactive debugging
- 📝 Richer SVD decoding (fields, enums) and correlation with recordings

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
backend = "openocd"
target_host = "localhost"
target_port = 3334

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

If `[svd] path` points to a valid CMSIS-SVD file (or use `--svd FILE` / env `RSGDB_SVD`), rsgdb builds a register map and emits **debug** logs for **client** memory packets (`m` read, `M` write) with a human-readable range when the access overlaps known registers — for example `GPIOA.MODER (4 bytes)`. This is **display only**; RSP bytes are unchanged.

Enable the log target with e.g. `RUST_LOG=rsgdb::svd=debug,rsgdb=info` (or `-d` / verbose per your logging setup).

### Session recording (rsgdb-record v1)

When enabled, each GDB↔backend connection writes one **JSON Lines** file under `recording.output_dir`. Line 1 is a header (`format`, `version`, `session_id`, `started_at`). Later lines are RSP events: `direction` (`client_to_backend` / `backend_to_client`), `kind` (`packet` / `ack` / `nack`), and for packets `payload_hex` / `payload_len` (hex is the raw packet payload bytes, not the `$…#xx` framing).

**Enable:** `rsgdb --record`, or set `[recording] enabled = true` in config, or `RSGDB_RECORD=1`. Optional directory override: `--record-dir DIR` or `RSGDB_RECORD_DIR`.

**Replay:** There is no built-in replayer yet. Inspect `.jsonl` with your usual tools or `jq`; a future release may add a mock server for automated replay.

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
│  │ Protocol │  │ Breakpoint Mgr  │ │
│  │  Parser  │  │                 │ │
│  └──────────┘  └─────────────────┘ │
│  ┌──────────┐  ┌─────────────────┐ │
│  │  Logger  │  │  State Tracker  │ │
│  │          │  │                 │ │
│  └──────────┘  └─────────────────┘ │
└──────────┬──────────────────────────┘
           │ Enhanced RSP
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

### v0.1.0 - Foundation (Current)
- [x] Project setup and structure
- [ ] Basic RSP protocol parser
- [ ] Simple pass-through proxy
- [ ] Configuration system
- [ ] Basic logging

### v0.2.0 - Core Features
- [ ] Enhanced logging with filtering
- [ ] Breakpoint management
- [ ] State tracking
- [ ] Memory inspection

### v0.3.0 - Backend Support
- [ ] probe-rs integration
- [ ] OpenOCD support
- [ ] Backend abstraction layer

### v0.4.0 - Advanced Features
- [ ] Session recording/replay
- [ ] TUI interface
- [ ] SVD peripheral decoding
- [ ] Performance optimizations

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

**Status**: 🚧 Early Development - Not yet ready for production use