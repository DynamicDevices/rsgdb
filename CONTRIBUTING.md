# Contributing to rsgdb

Thank you for your interest in contributing to rsgdb! This document provides guidelines and instructions for contributing to the project.

## Code of Conduct

We are committed to providing a welcoming and inclusive environment. Please be respectful and constructive in all interactions.

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Git
- A debug probe (optional, for hardware testing)
- Familiarity with GDB and embedded debugging concepts

### Setting Up Your Development Environment

1. Fork the repository on GitHub
2. Clone your fork:
   ```bash
   git clone git@github.com:YOUR_USERNAME/rsgdb.git
   cd rsgdb
   ```
3. Add the upstream repository:
   ```bash
   git remote add upstream git@github.com:DynamicDevices/rsgdb.git
   ```
4. Build the project:
   ```bash
   cargo build
   ```
5. Run tests:
   ```bash
   cargo test
   ```

### Local validation (before you open a PR)

From the repo root, run the same checks CI uses (fmt, tests with `--all-features`, clippy with `-D warnings`, docs with warnings denied):

```bash
./scripts/validate_local.sh
```

On Windows, use **Git Bash** or **WSL** so the script runs, or run the `cargo` commands from that script by hand.

### Optional — dependency hygiene (before releases)

```bash
./scripts/deps_check.sh
```

This runs **`cargo tree -d`** (duplicate transitive versions — often benign, e.g. `thiserror` v1 via `svd-parser` and v2 via `tracing-appender`), **`cargo audit`** against [RustSec](https://github.com/RustSec/advisory-db) (install: `cargo install cargo-audit`), and **`cargo outdated --workspace`** if `cargo-outdated` is installed (`cargo install cargo-outdated`). Major upgrades (e.g. `toml` 0.8 → 1.x) need a deliberate PR, not blind `cargo update`.

### Phase A — RSP-only regression (fast, no `gdb` binary)

Codec framing + proxy TCP integration tests only (~1s):

```bash
./scripts/e2e_rsp_regression.sh
```

Use this when iterating on `src/protocol/codec.rs` or `tests/proxy_integration.rs` without running the full `cargo test` suite.

### Phase B — GDB snippet + thread reply logging

- Copy or source [`scripts/gdbinit.rsgdb.example`](scripts/gdbinit.rsgdb.example) when connecting GDB through rsgdb (adjust `target extended-remote` to your listen port).
- Backend **thread-related** RSP replies (`m…` / `l` / `QC…` / hex thread names) get short summaries at **`rsgdb::rtos`** (debug); packets are unchanged on the wire.

### Simulated GDB session (optional, Linux/macOS)

End-to-end smoke: **gdbserver → rsgdb → GDB** (batch), same shape as CI job **E2E GDB smoke**.

Requires `gcc`, `gdb`, and `gdbserver`. On Debian/Ubuntu, `gdbserver` is often a separate package: `sudo apt-get install -y gcc gdb gdbserver`.

```bash
cargo build --release
./scripts/e2e_gdb_smoke.sh
```

To run the same check as part of local validation (after installing the tools above):

```bash
RUN_E2E_GDB=1 ./scripts/validate_local.sh
```

### Zephyr as a Linux process (`native_sim`, optional)

To debug a **real Zephyr app** (still RSP/gdbserver) without QEMU or hardware, build for the **`native_sim`** board: Zephyr links a normal Linux executable (`zephyr.exe`). Flow matches CI: **gdbserver → rsgdb → GDB**.

Requires a full [Zephyr west workspace](https://docs.zephyrproject.org/latest/develop/getting_started/index.html) (`ZEPHYR_WORKSPACE` with `.west/` and `zephyr/`). See [native_sim](https://docs.zephyrproject.org/latest/boards/native/native_sim/doc/index.html). The script builds **`native_sim/native/64`** by default (LP64 host binary); the plain `native_sim` target is 32-bit and needs multilib on x86_64.

The app under **`scripts/zephyr_multi_printf_app/`** (in this repo) has three `printf` lines with markers `RSGDB_E2E line 1` … `3`. The E2E script sets a breakpoint on the **first** `printf`, runs **`next`** twice, and asserts **`RSGDB_E2E line 1`** and **`line 2`** appear in the **gdbserver** log (inferior stdout). Override the app path with **`ZEPHYR_APP_SOURCE_DIR`** if needed.

```bash
export ZEPHYR_WORKSPACE=/path/to/zephyrproject
cargo build --release
./scripts/e2e_zephyr_native_sim.sh
```

First build can take several minutes. GDB is **host** `gdb` (not `arm-none-eabi-gdb`) because the ELF matches your machine.

```bash
RUN_E2E_ZEPHYR_NATIVE=1 ./scripts/validate_local.sh
```

### GitHub Actions (optional E2E parity)

- **GDB smoke** — the **CI** workflow includes an Ubuntu job that runs `scripts/e2e_gdb_smoke.sh` after `cargo build --release` (same idea as `RUN_E2E_GDB=1` with `validate_local.sh`, without duplicating fmt/clippy/doc in that job).
- **Zephyr `native_sim`** — the **Zephyr E2E** workflow (`.github/workflows/zephyr-e2e.yml`) provisions a cached west workspace, then runs `scripts/e2e_zephyr_native_sim.sh`. It runs when `scripts/e2e_zephyr_native_sim.sh` or `scripts/zephyr_multi_printf_app/` change, on pushes to `main`/`develop` for those paths, on a weekly schedule, and via **workflow_dispatch** (Actions → Zephyr E2E → Run workflow). This mirrors `RUN_E2E_ZEPHYR_NATIVE=1` locally without checking a Zephyr tree into this repo. The workflow builds **`target/debug/rsgdb`** (not release), frees some preinstalled SDKs on the runner, and installs **`pyelftools`** in the west venv (Zephyr’s `gen_kobject_list.py` imports `elftools`).

### Issue tracking

Work is tracked in [GitHub issues](https://github.com/DynamicDevices/rsgdb/issues). **Blocked-by** dependencies define order (e.g. Part A **#1 → #3**; **#2** can run in parallel). Close an issue from a PR with `Closes #N` when it is fully done.

**Foundation (closed issues):** Part A (**#1–#3**), session recording (**#4**), SVD baseline (**#5**), breakpoint/semihosting spike (**#6**), flash (**#7**), RTOS decode/log (**#8**). **Phase A/B** in-tree: RSP matrix + proxy tests, gdbinit example, `rsgdb::rtos` decode logs — see README **Key Features**.

**Current capabilities (same as README “Current”):** RSP codec + TCP proxy, `tracing` logging, TOML/env config, JSONL **record** + **`rsgdb replay`**, SVD register/field/enum-name memory annotations, `rsgdb flash`, RTOS packet summaries, CI + optional GDB/Zephyr E2E scripts.

**Roadmap — open:** [#9](https://github.com/DynamicDevices/rsgdb/issues/9) native / non-TCP backend beyond `connect_tcp_backend`. **Optional follow-ups:** SVD value decode + recording correlation (see [#11](https://github.com/DynamicDevices/rsgdb/issues/11) history); TUI, logging export, proxy-side breakpoint management — open an issue before large changes.

**CI:** Workflow **CI** + optional **Zephyr E2E** — green on `main` (see workflow files).

**CI jobs (overview):** Workflow **CI**: `test` (matrix), `fmt`, `clippy`, `docs`, `e2e-gdb-smoke`, `coverage`, `build` (artifacts; upload may use `continue-on-error` for transient infra). Workflow **Zephyr E2E**: west + SDK + `scripts/e2e_zephyr_native_sim.sh` (path / schedule / `workflow_dispatch`).

**Design ADRs:** [docs/ADR-001-breakpoints-semihosting.md](docs/ADR-001-breakpoints-semihosting.md) — breakpoint policy + semihosting (Phase 2 spike).

### Remote board smoke (manual)

Use this when a probe and target are available (optional OpenOCD or probe-rs GDB port):

1. Start your **backend** (e.g. OpenOCD) and note its **GDB TCP port** (often `3333`).
2. Start **rsgdb** so it listens for GDB and forwards to that port, e.g.  
   `rsgdb --port 3334 --target-host 127.0.0.1 --target-port 3333`
3. From GDB: `target extended-remote 127.0.0.1:3334`
4. Confirm: **break** / **continue** / **step** / `info reg` (or `x/4xw` on a valid address).

Success means the session behaves the same **with** rsgdb in the path as **without** (direct to OpenOCD), aside from added logging. Capture ports and commands in the issue if something fails.

## Development Workflow

### Branch Naming

- `feature/description` - New features
- `fix/description` - Bug fixes
- `docs/description` - Documentation updates
- `refactor/description` - Code refactoring
- `test/description` - Test additions/improvements

### Commit Messages

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

**Examples:**
```
feat(protocol): add support for vCont packet parsing

Implements parsing for the vCont packet which allows for more
granular control over thread execution.

Closes #42
```

```
fix(breakpoints): correct hardware breakpoint limit check

The previous implementation didn't account for already-set breakpoints
when checking if a new hardware breakpoint could be added.
```

### Pull Request Process

1. **Create a feature branch** from `main`:
   ```bash
   git checkout -b feature/my-new-feature
   ```

2. **Make your changes** following the coding standards below

3. **Write or update tests** for your changes

4. **Run the test suite**:
   ```bash
   cargo test
   ```

5. **Run formatting**:
   ```bash
   cargo fmt
   ```

6. **Run linting**:
   ```bash
   cargo clippy -- -D warnings
   ```

7. **Commit your changes** with clear, descriptive commit messages

8. **Push to your fork**:
   ```bash
   git push origin feature/my-new-feature
   ```

9. **Open a Pull Request** on GitHub with:
   - Clear title and description
   - Reference to any related issues
   - Screenshots/examples if applicable
   - Test results

10. **Address review feedback** promptly and professionally

## Coding Standards

### Rust Style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for consistent formatting
- Address all `cargo clippy` warnings
- Write idiomatic Rust code

### Code Organization

- Keep modules focused and cohesive
- Use clear, descriptive names
- Document public APIs with doc comments
- Include examples in documentation where helpful

### Documentation

- All public items must have doc comments
- Use `///` for item documentation
- Use `//!` for module documentation
- Include examples in doc comments:
  ```rust
  /// Parses a GDB RSP packet from the given buffer.
  ///
  /// # Examples
  ///
  /// ```
  /// use rsgdb::protocol::parse_packet;
  ///
  /// let packet = parse_packet(b"$qSupported#37");
  /// assert!(packet.is_ok());
  /// ```
  pub fn parse_packet(buffer: &[u8]) -> Result<Packet> {
      // ...
  }
  ```

### Error Handling

- Use `Result` for fallible operations
- Use `thiserror` for custom error types
- Provide context with `anyhow` where appropriate
- Don't panic in library code (use `Result` instead)

### Testing

- Write unit tests for individual functions
- Write integration tests for end-to-end scenarios
- Use descriptive test names: `test_parse_packet_with_checksum`
- Test both success and error cases
- Aim for high code coverage

Example test structure:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_packet() {
        let result = parse_packet(b"$qSupported#37");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_invalid_checksum() {
        let result = parse_packet(b"$qSupported#00");
        assert!(result.is_err());
    }
}
```

### Logging

- Use the `tracing` crate for logging
- Choose appropriate log levels:
  - `error!`: Errors that prevent operation
  - `warn!`: Unexpected but recoverable situations
  - `info!`: Important state changes
  - `debug!`: Detailed debugging information
  - `trace!`: Very verbose debugging

Example:
```rust
use tracing::{debug, info, warn};

info!("Starting GDB proxy on port {}", port);
debug!("Received packet: {:?}", packet);
warn!("Hardware breakpoint limit reached, using software breakpoint");
```

## Project Structure

Understanding the project structure will help you navigate the codebase:

```
rsgdb/
├── src/                     # Library + binary
├── tests/                   # Integration tests (e.g. proxy RSP smoke)
├── scripts/
│   ├── validate_local.sh    # Local CI parity (run before PRs)
│   └── deps_check.sh        # Optional: tree -d, audit, outdated
├── rsgdb.toml.example
└── .github/workflows/       # CI
```

## Areas for Contribution

We welcome contributions in these areas (see **[open issues](https://github.com/DynamicDevices/rsgdb/issues?q=is%3Aopen+is%3Aissue)** for current priorities):

### High priority (roadmap)
- [ ] **Native probe / backend abstraction** — [#9](https://github.com/DynamicDevices/rsgdb/issues/9)
- [x] **Session replay from JSONL** — [#10](https://github.com/DynamicDevices/rsgdb/issues/10) (`rsgdb replay`)

### Medium priority
- [x] **Richer SVD (fields, enum names in annotations)** — [#11](https://github.com/DynamicDevices/rsgdb/issues/11) (value decode / recording correlation still optional follow-ups)
- [ ] TUI, advanced breakpoints, logging export — open an issue before large changes

### Always welcome
- Documentation and test coverage (RSP matrix, proxy integration, SVD fixtures)
- Bug fixes and small ergonomics (gdbinit, logging targets)

## Getting Help

- **Questions**: Open a [GitHub Discussion](https://github.com/DynamicDevices/rsgdb/discussions)
- **Bugs**: Open a [GitHub Issue](https://github.com/DynamicDevices/rsgdb/issues)
- **Feature Requests**: Open a [GitHub Issue](https://github.com/DynamicDevices/rsgdb/issues) with the `enhancement` label

## Review Process

All submissions require review. We use GitHub pull requests for this purpose. The review process typically includes:

1. **Automated checks**: CI must pass (tests, formatting, linting)
2. **Code review**: At least one maintainer approval required
3. **Testing**: Verify the changes work as expected
4. **Documentation**: Ensure docs are updated if needed

## License

By contributing to rsgdb, you agree that your contributions will be licensed under the same terms as the project (MIT OR Apache-2.0).

## Recognition

Contributors will be recognized in:
- The project README
- Release notes
- Git commit history

Thank you for contributing to rsgdb! 🎉