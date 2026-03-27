# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial project structure and repository setup
- Basic GDB Remote Serial Protocol (RSP) packet parser
- Module structure for proxy, protocol, breakpoints, state, logger, backends, recorder, and UI
- CLI interface with clap
- Comprehensive documentation (README, CONTRIBUTING)
- CI/CD pipeline with GitHub Actions
- Dual MIT/Apache-2.0 licensing
- Unit tests for protocol parsing
- **Priority 1 Enhancements:**
  - Complete GDB command parsing (query, set, memory, breakpoints, execution control)
  - Tokio codec for packet streaming with ACK/NACK support
  - Comprehensive configuration system with TOML support
  - Enhanced error handling with specific error types
  - Configuration validation and environment variable support
  - Example configuration file (rsgdb.toml.example)
  - 22 passing unit tests

### Changed
- N/A

### Deprecated
- N/A

### Removed
- N/A

### Fixed
- N/A

### Security
- N/A

## [0.1.0] - 2026-03-27

### Added
- Initial release
- Project foundation and structure