# rsgdb - Enhanced GDB Server/Proxy

[![CI](https://github.com/DynamicDevices/rsgdb/workflows/CI/badge.svg)](https://github.com/DynamicDevices/rsgdb/actions)
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
- 🧪 **CI + local E2E smoke** — `gdbserver` → `rsgdb` → `gdb` (batch), script `scripts/e2e_gdb_smoke.sh`; GitHub Actions job **E2E GDB smoke** (Ubuntu). See [CONTRIBUTING.md](CONTRIBUTING.md).

### Planned
- 📊 Enhanced logging with filtering and export (JSON, CSV)
- 🎯 Advanced breakpoint management (named, conditional, grouped)
- 🔍 State tracking and visualization
- 💾 Session **replay** tooling (mock backend / automated playback)
- 🔌 Multiple backend support (probe-rs, OpenOCD)
- 🖥️ Terminal UI (TUI) for interactive debugging
- 📝 Richer SVD decoding (fields, enums) and correlation with recordings

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

# Run with logging (also respects [logging] in rsgdb.toml after init)
RUST_LOG=debug cargo run
```

### Project Structure

```
rsgdb/
├── src/                     # Library + CLI
├── tests/                   # Integration tests
├── scripts/validate_local.sh
├── scripts/e2e_gdb_smoke.sh # gdbserver → rsgdb → gdb (batch); CI E2E job
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