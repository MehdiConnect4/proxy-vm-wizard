# Proxy VM Wizard

<p align="center">
  <img src="assets/io.github.proxyvmwizard.ProxyVmWizard.svg" width="128" height="128" alt="Proxy VM Wizard Logo">
</p>

<p align="center">
  <strong>A secure, local-only GUI for managing proxy gateway VMs with libvirt/QEMU/KVM</strong>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#installation">Installation</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#documentation">Documentation</a> •
  <a href="#building">Building</a>
</p>

---

## Features

- 🔒 **Local-only** - No network calls, telemetry, or external services
- 🔐 **Encrypted Storage** - Password-protected configuration and templates using AES-256-GCM
- 🌐 **Proxy Chains** - Route traffic through 1-8 SOCKS5/HTTP proxy hops
- 🛡️ **VPN Support** - WireGuard and OpenVPN gateway configurations
- 📦 **Template Management** - Manage qcow2 base images for different VM types
- 🖥️ **App VMs** - Create isolated VMs behind your gateway
- ⚡ **Disposable VMs** - Ephemeral VMs that auto-delete when stopped
- 🎨 **Modern GUI** - Clean, native-feeling interface built with egui

## Screenshots

<p align="center">
  <img src="screenshots/dashboard.png" width="600" alt="Dashboard">
</p>

## Installation

### Prerequisites

You need libvirt and QEMU installed on your system:

```bash
# Debian/Ubuntu
sudo apt install libvirt-daemon-system libvirt-clients virtinst qemu-kvm qemu-utils policykit-1

# Fedora
sudo dnf install @virtualization

# Arch Linux
sudo pacman -S libvirt virt-install qemu-base polkit
```

Add your user to the libvirt group:
```bash
sudo usermod -aG libvirt $USER
# Log out and back in for changes to take effect
```

### Option 1: APT Repository (Recommended for Debian/Ubuntu)

Add the repository and install:

```bash
# Add repository
echo "deb [trusted=yes] https://mehdiconnect4.github.io/proxy-vm-wizard stable main" | sudo tee /etc/apt/sources.list.d/proxy-vm-wizard.list

# Update and install
sudo apt update
sudo apt install proxy-vm-wizard
```

**Updates will be automatic** with `sudo apt update && sudo apt upgrade`

### Option 2: AppImage (Universal Linux)

Download the latest AppImage from the [Releases](https://github.com/MehdiConnect4/proxy-vm-wizard/releases) page:

```bash
chmod +x proxy-vm-wizard-x86_64.AppImage
./proxy-vm-wizard-x86_64.AppImage
```

### Option 3: Debian/Ubuntu (.deb)

```bash
# Download the .deb from Releases page
sudo dpkg -i proxy-vm-wizard_*.deb
sudo apt-get install -f  # Install any missing dependencies
```

### Option 4: Fedora/RHEL (.rpm)

```bash
# Download the .rpm from Releases page
sudo dnf install proxy-vm-wizard-*.rpm
# Or with rpm
sudo rpm -i proxy-vm-wizard-*.rpm
```

### Option 5: Binary Tarball (All Linux)

```bash
# Download and extract
tar -xzf proxy-vm-wizard-linux-x86_64.tar.gz
cd proxy-vm-wizard

# Install system-wide
sudo ./install.sh

# Or run directly
./proxy-vm-wizard
```

### Option 6: Build from Source

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/proxyvmwizard/proxy-vm-wizard.git
cd proxy-vm-wizard
cargo build --release

# Run
./target/release/proxy-vm-wizard
```

### Option 7: Flatpak

```bash
# Add Flathub if not already added
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo

# Install (once published)
flatpak install flathub io.github.proxyvmwizard.ProxyVmWizard
```

## Quick Start

### 1. Add a Template

First, you need a base qcow2 image. You can create one or download a cloud image:

```bash
# Example: Download Debian cloud image
wget https://cloud.debian.org/images/cloud/bookworm/latest/debian-12-generic-amd64.qcow2
sudo mv debian-12-generic-amd64.qcow2 /var/lib/libvirt/images/
```

Then in the app:
1. Go to **📁 Templates**
2. Click **➕ Add Template**
3. Select your qcow2 file
4. Set OS variant (e.g., `debian12`)
5. Set role kind (Proxy/Gateway or App)

### 2. Create a Role

1. Go to **🧙 Wizard** or click **➕ Create New Role**
2. Enter a role name (e.g., `work`, `banking`)
3. Select your gateway and app templates
4. Configure your gateway mode:
   - **Proxy Chain**: Add SOCKS5/HTTP proxies
   - **WireGuard**: Upload your .conf file
   - **OpenVPN**: Upload your .ovpn file
5. Click **Create Role**

### 3. Manage VMs

From the **📊 Dashboard**:
- Start/stop gateway VMs
- Create app VMs isolated behind the gateway
- Launch disposable VMs
- Edit gateway configurations

## Network Topology

```
┌─────────────────────────────────────────────────────────────┐
│                        Host Machine                         │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────────┐        ┌──────────────┐                   │
│  │   lan-net    │◄──────►│  pfSense VM  │◄──► Internet      │
│  │ (LAN bridge) │        │  (existing)  │                   │
│  └──────┬───────┘        └──────────────┘                   │
│         │                                                   │
│         ▼                                                   │
│  ┌──────────────┐                                           │
│  │  work-gw VM  │◄─── NIC1: lan-net (upstream)              │
│  │  (gateway)   │     Routes through proxy/VPN              │
│  │              │◄─── NIC2: work-inet (downstream)          │
│  └──────┬───────┘                                           │
│         │                                                   │
│         ▼                                                   │
│  ┌──────────────┐        ┌──────────────┐                   │
│  │  work-inet   │◄──────►│ work-app-1   │                   │
│  │ (isolated)   │        │   (App VM)   │                   │
│  └──────────────┘        └──────────────┘                   │
└─────────────────────────────────────────────────────────────┘
```

## Configuration Files

The app stores configuration in:
- `~/.config/proxy-vm-wizard/auth.json` - Password hash and encryption salt (no sensitive data)
- `~/.config/proxy-vm-wizard/config.toml` - Global settings (encrypted)
- `~/.config/proxy-vm-wizard/templates.toml` - Template registry (encrypted)
- `~/VMS/VM-Proxy-configs/<role>/` - Per-role configuration (customizable)

## Documentation

- [User Guide](docs/USER_GUIDE.md)
- [Template Setup](docs/TEMPLATES.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Security](SECURITY.md)

## Building

### Requirements

- Rust 1.70+ (stable)
- Linux with GUI libraries

### Development Build

```bash
cargo build
cargo run
```

### Release Build

```bash
cargo build --release
./target/release/proxy-vm-wizard
```

### Run Tests

```bash
cargo test
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Security

- **Password-protected** - All configuration and template data encrypted with AES-256-GCM
- **Argon2id** - Industry-standard password hashing for key derivation
- **No network calls** from the application (except proxy connectivity tests you initiate)
- **No telemetry** or analytics
- **Local-only** operation - all data stays on your machine
- Commands use direct execution (no shell invocation)
- Input validation on all user-provided data
- Secure memory handling for encryption keys

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [egui](https://github.com/emilk/egui) - Immediate mode GUI library
- [libvirt](https://libvirt.org/) - Virtualization API
- [QEMU](https://www.qemu.org/) - Machine emulator and virtualizer
