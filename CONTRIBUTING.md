# Contributing to Proxy VM Wizard

Thank you for your interest in contributing! This document provides guidelines and information for contributors.

## Code of Conduct

Please be respectful and constructive in all interactions. We're all here to build something useful together.

## How to Contribute

### Reporting Bugs

1. Check existing [issues](https://github.com/proxyvmwizard/proxy-vm-wizard/issues) first
2. Use the bug report template
3. Include:
   - OS and version
   - Steps to reproduce
   - Expected vs actual behavior
   - Relevant logs (check `~/.config/proxy-vm-wizard/`)

### Suggesting Features

1. Check existing issues/discussions first
2. Use the feature request template
3. Explain the use case and why it would be valuable

### Pull Requests

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make your changes
4. Run tests: `cargo test`
5. Run clippy: `cargo clippy`
6. Format code: `cargo fmt`
7. Commit with clear messages
8. Push and create a PR

## Development Setup

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install system dependencies (Debian/Ubuntu)
sudo apt install libvirt-daemon-system libvirt-clients virtinst qemu-kvm \
  libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libxkbcommon-dev libssl-dev pkg-config
```

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run
```

### Project Structure

```
proxy-vm-wizard/
├── core/               # Core library (no GUI dependencies)
│   └── src/
│       ├── auth.rs     # Authentication and encryption
│       ├── config.rs   # Configuration management
│       ├── libvirt.rs  # Libvirt/QEMU integration
│       ├── model.rs    # Domain models
│       └── ...
├── ui/                 # GUI application
│   └── src/
│       ├── app.rs      # Application state
│       ├── views/      # UI views
│       └── ...
├── assets/             # Icons, desktop files
├── scripts/            # Installation scripts
└── flatpak/            # Flatpak packaging
```

### Code Style

- Follow Rust conventions
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Write doc comments for public APIs
- Add tests for new functionality

### Testing

- Unit tests go in the same file as the code
- Integration tests go in `tests/`
- Test with actual libvirt if possible (but mock for CI)

## Architecture Guidelines

### Core Library (`core/`)

- No GUI dependencies
- Pure logic and data structures
- All libvirt/QEMU interaction
- Configuration management
- Should be usable as a library

### UI (`ui/`)

- GUI only (egui/eframe)
- Thin wrapper around core
- State management
- User interaction

### Security Considerations

- Never invoke shell (use direct Command execution)
- Validate all user input
- All configuration data must be encrypted (use `EncryptionManager`)
- Handle encryption keys securely in memory
- No network calls except user-initiated tests
- Use Argon2id for password hashing and key derivation
- Never store passwords in plain text

## Release Process

1. Update version in `Cargo.toml` files
2. Update `CHANGELOG.md`
3. Create and push a version tag: `git tag v0.1.0 && git push --tags`
4. GitHub Actions will build and create a release

## Questions?

Open a [discussion](https://github.com/proxyvmwizard/proxy-vm-wizard/discussions) or reach out to the maintainers.


