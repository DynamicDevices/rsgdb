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
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library root
│   ├── proxy/               # Core proxy logic
│   │   ├── mod.rs
│   │   ├── server.rs        # GDB server implementation
│   │   └── client.rs        # Backend client
│   ├── protocol/            # RSP protocol handling
│   │   ├── mod.rs
│   │   ├── parser.rs        # Packet parsing
│   │   └── commands.rs      # Command handling
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

## Areas for Contribution

We welcome contributions in these areas:

### High Priority
- [ ] GDB RSP protocol implementation
- [ ] Backend integrations (probe-rs, OpenOCD)
- [ ] Breakpoint management system
- [ ] State tracking and inspection

### Medium Priority
- [ ] Session recording/replay
- [ ] TUI interface
- [ ] SVD peripheral decoding
- [ ] Performance optimizations

### Always Welcome
- Documentation improvements
- Test coverage expansion
- Bug fixes
- Example code
- Performance improvements

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