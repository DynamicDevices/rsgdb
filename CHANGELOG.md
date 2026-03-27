# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Nothing yet.

## [0.2.0-dev.1] - 2026-03-27

First **development** release: the proxy is usable end-to-end for TCP, managed local stubs, and **remote Linux targets** over SSH. Pre-release semver (`-dev.N`) signals API and CLI may still evolve before **0.2.0**.

### Added

- **`transport = remote_ssh`**: run `gdbserver` on a remote host via OpenSSH; optional **`upload_local` / `upload_remote`** to **`scp`** the ELF before connect; optional **`RSGDB_SSH_PASSWORD`** + **`sshpass`** for non-interactive password auth.
- **Example** [`examples/board_test_app/`](examples/board_test_app/README.md): tiny aarch64 Linux smoke binary, [`rsgdb.remote.toml`](examples/board_test_app/rsgdb.remote.toml), [`install_ssh_key.sh`](examples/board_test_app/install_ssh_key.sh), [`debug_remote.sh`](examples/board_test_app/debug_remote.sh) (SSH key preflight + GDB batch smoke).
- **Design principles** (README): ease, automation, reliability for embedded remote debugging.
- **Documentation**: Linux target SSH setup, CONTRIBUTING cross-links, config examples for `remote_ssh`.

### Changed

- **Configuration** (`[backend.remote_ssh]`, validation): user, optional host override, SSH port, identity, upload paths, gdbserver argv with `{port}` placeholder.
- **Proxy**: backend connect path supports remote SSH session lifecycle alongside TCP and native spawn.

### Notes

- **Stability**: Suitable for daily development and field trials; treat semver pre-releases as **unstable** for automation that pins exact versions.
- **Validation**: `./scripts/validate_local.sh` matches CI (fmt, `cargo test --all-features`, clippy `-D warnings`, `cargo doc`).

[Unreleased]: https://github.com/DynamicDevices/rsgdb/compare/v0.2.0-dev.1...HEAD
[0.2.0-dev.1]: https://github.com/DynamicDevices/rsgdb/releases/tag/v0.2.0-dev.1
