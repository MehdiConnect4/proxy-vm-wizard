# Security Overview

This document outlines the security measures implemented in Proxy VM Wizard.

## Cryptographic Implementation

### Encryption at Rest
- **Algorithm**: AES-256-GCM (Galois/Counter Mode)
- **Key Size**: 256 bits
- **Nonce**: 96 bits (12 bytes), randomly generated per encryption
- **Authentication**: Built-in authenticated encryption (AEAD)

### Password Security
- **Hashing**: Argon2id (memory-hard, GPU-resistant)
- **Salt**: Random salt generated per password using `OsRng`
- **Key Derivation**: Argon2id with separate salt for encryption keys
- **Minimum Length**: 8 characters enforced
- **Storage**: Only cryptographic hash stored, never plaintext

### File Permissions
All sensitive files are created with restrictive permissions (0600 - owner read/write only):
- `~/.config/proxy-vm-wizard/auth.json` - Authentication state
- `~/.config/proxy-vm-wizard/config.toml` - Encrypted configuration
- `~/.config/proxy-vm-wizard/templates.toml` - Encrypted template registry
- `~/VMS/VM-Proxy-configs/<role>/proxy.conf` - Proxy configuration (may contain credentials)

### Memory Security
- Passwords cleared from memory immediately after use (`password.clear()`)
- Encryption keys handled securely (not logged or serialized)
- No password caching or persistence beyond encryption manager lifetime

## Code Security

### No Unsafe Code
- ✅ Zero `unsafe` blocks in the codebase
- ✅ All Rust safety guarantees maintained
- ✅ Memory-safe by design

### Command Execution
- ✅ Direct process execution only (`std::process::Command`)
- ✅ **No shell invocation** (no `sh -c`, no `bash -c`)
- ✅ All arguments passed explicitly (prevents injection)
- ✅ No user input directly concatenated into commands

Example:
```rust
// Safe - direct execution with explicit args
Command::new("virsh")
    .args(["start", vm_name])
    .output()

// NEVER DONE - shell invocation (injection risk)
Command::new("sh")
    .args(["-c", format!("virsh start {}", vm_name)])
    .output()
```

### Input Validation
- **Role names**: Restricted to `^[a-z0-9_-]+$` regex
- **Paths**: Validated for existence and accessibility
- **Ports**: Parsed and validated as u16
- **File extensions**: Checked for expected types

### Privilege Escalation
- **pkexec** used for operations requiring root
- Graphical password prompt (user-friendly)
- Minimal privilege scope (specific commands only)
- No password storage for privileged operations

## Network Security

### No Telemetry
- ✅ No analytics or tracking
- ✅ No automatic update checks
- ✅ No phone-home functionality

### User-Initiated Only
- ✅ Proxy connection tests only when user clicks "Test"
- ✅ No background network operations
- ✅ All external connections explicit and visible

## Data Storage

### Local Only
- ✅ All data stored on local filesystem
- ✅ No cloud sync or external storage
- ✅ User has full control of all data

### Encrypted Files
Format: `PVMW_ENC_V1` header + nonce + ciphertext
- `config.toml` - Global settings
- `templates.toml` - Template registry
- Both encrypted with AES-256-GCM using user's password-derived key

### Plaintext Files (By Design)
- `auth.json` - Contains only password hash and salts (no sensitive data)
- `role-meta.toml` - Role metadata (no credentials)
- `proxy.conf` - May contain proxy passwords (protected by file permissions 0600)
- VM disks - QEMU/libvirt manage these

## Threat Model

### What We Protect Against

✅ **Unauthorized access to configuration**
- Encrypted storage with strong password

✅ **Credential theft from filesystem**
- Restrictive file permissions (0600)
- Encryption for primary config files

✅ **Command injection attacks**
- Direct command execution only
- No shell invocation
- Input validation

✅ **Memory inspection**
- Passwords cleared after use
- Keys derived on-demand, not persisted

✅ **Unauthorized privilege escalation**
- pkexec with explicit command arguments
- No sudo credential caching abuse

### What We Don't Protect Against

❌ **Physical access with root**
- Root user can access any file
- This is inherent to the Linux privilege model

❌ **Memory dumps while app is running**
- Encryption keys are in memory while app is open
- This is unavoidable for encrypted storage

❌ **Keyloggers or screen capture**
- Password entered via keyboard
- Use hardware-based authentication for this threat level

❌ **Malicious libvirt/QEMU**
- We trust the virtualization stack
- Run VMs with appropriate isolation

## Audit Recommendations

### For Production Deployment

1. **Password Strength**
   - Enforce strong passwords (consider 12+ characters)
   - Add password complexity requirements if needed

2. **File Permissions**
   - Audit `~/VMS/VM-Proxy-configs/` directory permissions
   - Ensure role directories are not world-readable

3. **System Hardening**
   - Use SELinux/AppArmor for additional isolation
   - Limit libvirt user group membership
   - Enable firewalld/iptables for network segmentation

4. **Regular Updates**
   - Keep Rust dependencies updated
   - Monitor security advisories for: argon2, aes-gcm, base64
   - Update base VM templates regularly

5. **Backup Security**
   - Backups of encrypted config should remain encrypted
   - Store password securely (password manager recommended)
   - No password recovery mechanism (by design)

## Compliance

### GDPR/Privacy
- ✅ No personal data collection
- ✅ No third-party data sharing
- ✅ User has full control of data
- ✅ Easy data deletion (remove config directory)

### Security Standards
- ✅ OWASP password hashing recommendations (Argon2id)
- ✅ NIST encryption standards (AES-256-GCM)
- ✅ Secure random number generation (OsRng)
- ✅ Input validation throughout

## Reporting Security Issues

Please report security vulnerabilities privately to the maintainers via:
- GitHub Security Advisories (preferred)
- Direct email to maintainers

Do NOT open public issues for security vulnerabilities.

## Last Updated

This security document was last updated: 2025-11-26

