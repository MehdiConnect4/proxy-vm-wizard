# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.2] - 2025-11-26

### Fixed
- **Critical performance bug**: Template edit dialog was calling virsh commands 600+ times per second, freezing the UI
- Now fetches VM data once on dialog open, then uses cached data for instant smooth scrolling

## [0.2.0] - 2025-01-15

### Added
- **Password-based encryption** for all configuration and template data
- **AES-256-GCM** encryption for sensitive data at rest
- **Argon2id** key derivation for secure password hashing
- Password setup screen on first launch
- Login screen for subsequent launches
- Automatic migration from plain text to encrypted storage
- Encrypted file detection and handling

### Security
- All configuration files now encrypted by default
- Secure key derivation using Argon2id with random salts
- AES-256-GCM for authenticated encryption
- Password minimum length enforcement (8 characters)
- Secure memory handling for encryption keys
- No plaintext storage of sensitive configuration data

## [0.1.0] - 2024-01-01

### Added
- Initial release
- Gateway VM creation with proxy chains (1-8 SOCKS5/HTTP hops)
- WireGuard VPN gateway support
- OpenVPN gateway support
- App VM creation and management
- Disposable (ephemeral) VM support
- Template management for qcow2 base images
- Dashboard with role overview and VM controls
- Configuration editor for gateway settings
- Automatic cleanup on wizard failure/cancellation
- VPN config file parsing (shows server details)
- Protocol exclusivity with confirmation dialogs

### Security
- No network calls (except user-initiated proxy tests)
- No telemetry or analytics
- Direct command execution (no shell invocation)
- Input validation on all user data
- PolicyKit integration for privileged operations


