# Flathub Submission Checklist

This document tracks Flathub submission requirements for Proxy VM Wizard.

## ✅ Application Requirements

### App ID
- ✅ Valid reverse-DNS format: `io.github.proxyvmwizard.ProxyVmWizard`
- ✅ Matches repository owner (proxyvmwizard organization or MehdiConnect4 user)

### Metadata
- ✅ Valid metainfo.xml (AppStream format)
- ✅ License specified: MIT
- ✅ Screenshots included (dashboard.png)
- ✅ Release history documented (0.1.0, 0.2.0, 0.2.7)
- ✅ OARS content rating included
- ✅ Categories appropriate: System, Utility, Network

### Desktop File
- ✅ Valid desktop entry
- ✅ Icon properly referenced
- ✅ Categories match metainfo
- ✅ Keywords included

### Icon
- ✅ SVG format (scalable)
- ✅ Follows naming convention
- ✅ Proper installation path

## ✅ Build System

### Manifest
- ✅ Uses org.freedesktop.Platform runtime
- ✅ Rust SDK extension used
- ✅ Offline build (cargo-sources.json)
- ✅ Proper build commands

### Build Process
- ✅ Builds from source
- ✅ No prebuilt binaries
- ✅ All dependencies from crates.io
- ✅ Reproducible build

## ✅ Permissions (finish-args)

### Justified Permissions

**--socket=system-bus**
- **Why**: Required to communicate with libvirtd system service
- **Usage**: Create/manage VMs via virsh commands
- **Alternative**: None - libvirt requires system bus access

**--filesystem=/var/lib/libvirt/images:rw**
- **Why**: Required to create VM disk images (qcow2 overlays)
- **Usage**: virt-install and qemu-img write disks here
- **Alternative**: None - libvirt standard path for VM disks

**--filesystem=/etc/libvirt:ro**
- **Why**: Read libvirt network definitions
- **Usage**: Check if networks exist before creating VMs
- **Alternative**: Could fail gracefully, but UX would suffer

**--talk-name=org.freedesktop.PolicyKit1**
- **Why**: Prompt for password when writing to system directories
- **Usage**: pkexec for elevated operations (disk creation)
- **Alternative**: None - required for security

**--share=network**
- **Why**: User-initiated proxy connection testing
- **Usage**: Only when user clicks "Test Connection" button
- **Alternative**: Could disable feature, but critical for troubleshooting

### Restrictive Approach
- ❌ NOT using --filesystem=home (too broad)
- ✅ Using --filesystem=xdg-documents/VMS:create (specific)
- ✅ Using --persist=.config/proxy-vm-wizard (config only)

## ✅ Security

### No Security Red Flags
- ✅ No shell invocation (direct Command execution only)
- ✅ No hardcoded secrets or tokens
- ✅ No telemetry or external connections
- ✅ Input validation on all user data
- ✅ Secure password handling (Argon2id + AES-256-GCM)
- ✅ Restrictive file permissions (0600)

### Security Documentation
- ✅ SECURITY.md provided
- ✅ Threat model documented
- ✅ Cryptography choices justified
- ✅ No unsafe code blocks

## ✅ Quality Standards

### Code Quality
- ✅ All tests pass (21/21)
- ✅ Zero compiler warnings
- ✅ Zero clippy warnings
- ✅ Formatted with rustfmt
- ✅ No panics in production code
- ✅ Comprehensive error handling

### Documentation
- ✅ README with clear instructions
- ✅ User guide
- ✅ Architecture documentation
- ✅ Contributing guidelines
- ✅ Security documentation
- ✅ Changelog maintained

## ⚠️ Flathub Review Notes

### Permissions Justification

**This app requires elevated system access because:**
1. It manages virtual machines via libvirtd (system service)
2. VM disk images must be in /var/lib/libvirt/images (libvirt standard)
3. Network definitions are in /etc/libvirt (libvirt standard)

**This is similar to:**
- virt-manager (also requires system-bus + /var/lib/libvirt access)
- GNOME Boxes (similar virtualization requirements)
- Cockpit (system management tool)

**Security measures:**
- No direct filesystem writes outside sanctioned paths
- PolicyKit authentication for privileged operations
- All operations logged
- No background processes or daemons
- User-initiated actions only

### Expected Review Questions

**Q: Why --socket=system-bus?**
A: Required for libvirt communication. Libvirtd runs as system service. Alternative would be session libvirt, but that doesn't support KVM.

**Q: Why --filesystem=/var/lib/libvirt:rw?**
A: VM disk images must be accessible by qemu (runs as libvirt-qemu user). This is the standard libvirt images directory.

**Q: Why --share=network?**
A: Only for user-initiated proxy connection testing. User clicks "Test Connection" button to verify proxy servers are reachable. No background network activity.

**Q: Why --talk-name=org.freedesktop.PolicyKit1?**
A: For graphical sudo prompts (pkexec) when writing to system directories. Better UX than terminal sudo.

## 📋 Pre-Submission Checklist

- ✅ Version bumped to 0.2.7 everywhere
- ✅ Metainfo.xml updated with latest release
- ✅ Repository URLs corrected
- ✅ Flatpak manifest permissions justified
- ✅ All tests passing
- ✅ AppStream metadata valid
- ✅ Desktop file valid
- ✅ Icon in proper format (SVG)
- ✅ Build commands work offline
- ✅ No bundled precompiled binaries
- ✅ cargo-sources.json generated
- ✅ Screenshots accessible
- ✅ Documentation complete

## 🚀 Submission Process

1. Fork flathub/flathub repository
2. Create new repository: flathub/io.github.proxyvmwizard.ProxyVmWizard
3. Copy manifest files:
   - io.github.proxyvmwizard.ProxyVmWizard.yml
   - flathub.json
   - cargo-sources.json (generate first)
4. Create pull request
5. Address review feedback
6. Wait for approval

## 📝 Generating cargo-sources.json

```bash
# Install flatpak-cargo-generator
pip3 install aiohttp toml

# Generate sources
python3 flatpak-cargo-generator.py Cargo.lock -o flatpak/cargo-sources.json
```

## 🔗 Useful Links

- Flathub submission: https://github.com/flathub/flathub/wiki/App-Submission
- App requirements: https://docs.flathub.org/docs/for-app-authors/requirements
- Flatpak permissions: https://docs.flatpak.org/en/latest/sandbox-permissions.html
- AppStream spec: https://www.freedesktop.org/software/appstream/docs/

## ⚠️ Known Limitations

**Flatpak Sandbox Limitations:**
- Requires system-bus access (elevated permission)
- Requires /var/lib/libvirt access (system path)
- These are necessary for virtualization management
- Similar to virt-manager's requirements

**This is acceptable because:**
- App explicitly manages system virtual machines
- Disclosed to users in description
- No more permissions than necessary
- Follows libvirt standards

