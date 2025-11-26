#!/bin/bash
# Proxy VM Wizard Installation Script
set -e

INSTALL_DIR="${INSTALL_DIR:-/usr/local}"
BIN_DIR="${INSTALL_DIR}/bin"
SHARE_DIR="${INSTALL_DIR}/share"
APP_DIR="${SHARE_DIR}/applications"
ICON_DIR="${SHARE_DIR}/icons/hicolor/scalable/apps"
METAINFO_DIR="${SHARE_DIR}/metainfo"

echo "=== Proxy VM Wizard Installer ==="
echo

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "This script needs root privileges to install system-wide."
    echo "Please run with sudo: sudo ./install.sh"
    exit 1
fi

# Check for required system dependencies
echo "Checking dependencies..."
MISSING_DEPS=""

if ! command -v virsh &> /dev/null; then
    MISSING_DEPS="$MISSING_DEPS libvirt-clients"
fi

if ! command -v virt-install &> /dev/null; then
    MISSING_DEPS="$MISSING_DEPS virtinst"
fi

if ! command -v qemu-img &> /dev/null; then
    MISSING_DEPS="$MISSING_DEPS qemu-utils"
fi

if ! command -v pkexec &> /dev/null; then
    MISSING_DEPS="$MISSING_DEPS policykit-1"
fi

if [ -n "$MISSING_DEPS" ]; then
    echo
    echo "WARNING: Missing dependencies detected!"
    echo "Please install:$MISSING_DEPS"
    echo
    echo "On Debian/Ubuntu:"
    echo "  sudo apt install$MISSING_DEPS"
    echo
    echo "On Fedora:"
    echo "  sudo dnf install libvirt virt-install qemu-img polkit"
    echo
    read -p "Continue installation anyway? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Create directories
echo "Creating directories..."
mkdir -p "$BIN_DIR"
mkdir -p "$APP_DIR"
mkdir -p "$ICON_DIR"
mkdir -p "$METAINFO_DIR"

# Install binary
echo "Installing binary..."
if [ -f "proxy-vm-wizard" ]; then
    install -m 755 proxy-vm-wizard "$BIN_DIR/proxy-vm-wizard"
elif [ -f "target/release/proxy-vm-wizard" ]; then
    install -m 755 target/release/proxy-vm-wizard "$BIN_DIR/proxy-vm-wizard"
else
    echo "ERROR: Binary not found. Please build first with: cargo build --release"
    exit 1
fi

# Install desktop file
echo "Installing desktop entry..."
if [ -f "assets/io.github.proxyvmwizard.ProxyVmWizard.desktop" ]; then
    install -m 644 assets/io.github.proxyvmwizard.ProxyVmWizard.desktop "$APP_DIR/"
fi

# Install icon
echo "Installing icon..."
if [ -f "assets/io.github.proxyvmwizard.ProxyVmWizard.svg" ]; then
    install -m 644 assets/io.github.proxyvmwizard.ProxyVmWizard.svg "$ICON_DIR/"
fi

# Install metainfo
echo "Installing metainfo..."
if [ -f "assets/io.github.proxyvmwizard.ProxyVmWizard.metainfo.xml" ]; then
    install -m 644 assets/io.github.proxyvmwizard.ProxyVmWizard.metainfo.xml "$METAINFO_DIR/"
fi

# Update desktop database
echo "Updating desktop database..."
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database "$APP_DIR" 2>/dev/null || true
fi

# Update icon cache
echo "Updating icon cache..."
if command -v gtk-update-icon-cache &> /dev/null; then
    gtk-update-icon-cache -f -t "${SHARE_DIR}/icons/hicolor" 2>/dev/null || true
fi

echo
echo "=== Installation Complete ==="
echo
echo "You can now run: proxy-vm-wizard"
echo
echo "Make sure your user is in the libvirt group:"
echo "  sudo usermod -aG libvirt \$USER"
echo "  (Log out and back in for changes to take effect)"
echo


