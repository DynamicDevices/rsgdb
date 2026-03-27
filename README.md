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

### Planned
- 📊 Enhanced logging with filtering and export (JSON, CSV)
- 🎯 Advanced breakpoint management (named, conditional, grouped)
- 🔍 State tracking and visualization
- 💾 Session recording and replay
- 🔌 Multiple backend support (probe-rs, OpenOCD)
- 🖥️ Terminal UI (TUI) for interactive debugging
- 📝 SVD-based peripheral register decoding

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
# Start rsgdb in proxy mode
rsgdb --backend openocd --port 3333 --target localhost:3334

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
```

## 📖 Documentation

- [Architecture Overview](docs/architecture.md)
- [GDB Protocol Reference](docs/gdb-protocol.md)
- [User Guide](docs/user-guide.md)
- [Contributing Guidelines](CONTRIBUTING.md)

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

# Run with logging
RUST_LOG=debug cargo run
```

### Project Structure

```
rsgdb/
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library root
│   ├── proxy/               # Core proxy logic
│   ├── protocol/            # RSP protocol handling
│   ├── breakpoints/         # Breakpoint management
│   ├── state/               # State tracking
│   ├── logger/              # Enhanced logging
│   ├── backends/            # Debug probe backends
│   ├── recorder/            # Session recording
│   └── ui/                  # User interfaces
├── tests/                   # Integration tests
├── examples/                # Usage examples
└── docs/                    # Documentation
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