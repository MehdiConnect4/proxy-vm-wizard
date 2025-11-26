# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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


