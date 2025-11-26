#!/bin/bash
#
# Quick Install Script for Proxy VM Wizard
# Downloads and installs the latest release
#
# Usage: curl -sSL https://raw.githubusercontent.com/proxyvmwizard/proxy-vm-wizard/main/scripts/quick-install.sh | bash
#
set -e

REPO="proxyvmwizard/proxy-vm-wizard"
INSTALL_DIR="${HOME}/.local/bin"
RELEASE_URL="https://api.github.com/repos/${REPO}/releases/latest"

echo "=== Proxy VM Wizard Quick Installer ==="
echo

# Detect architecture
ARCH=$(uname -m)
case $ARCH in
    x86_64)
        ARCH_NAME="x86_64"
        ;;
    aarch64)
        ARCH_NAME="aarch64"
        ;;
    *)
        echo "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

echo "[1/4] Detecting latest release..."
if command -v curl &> /dev/null; then
    DOWNLOAD_URL=$(curl -s "$RELEASE_URL" | grep "browser_download_url.*${ARCH_NAME}.*AppImage" | head -1 | cut -d'"' -f4)
elif command -v wget &> /dev/null; then
    DOWNLOAD_URL=$(wget -qO- "$RELEASE_URL" | grep "browser_download_url.*${ARCH_NAME}.*AppImage" | head -1 | cut -d'"' -f4)
else
    echo "Error: curl or wget required"
    exit 1
fi

if [ -z "$DOWNLOAD_URL" ]; then
    echo "Error: Could not find release for ${ARCH_NAME}"
    echo "Please download manually from: https://github.com/${REPO}/releases"
    exit 1
fi

echo "[2/4] Downloading AppImage..."
mkdir -p "$INSTALL_DIR"
APPIMAGE_PATH="${INSTALL_DIR}/proxy-vm-wizard"

if command -v curl &> /dev/null; then
    curl -L "$DOWNLOAD_URL" -o "$APPIMAGE_PATH"
else
    wget "$DOWNLOAD_URL" -O "$APPIMAGE_PATH"
fi

echo "[3/4] Making executable..."
chmod +x "$APPIMAGE_PATH"

echo "[4/4] Checking dependencies..."
MISSING_DEPS=""
command -v virsh &> /dev/null || MISSING_DEPS="$MISSING_DEPS libvirt-clients"
command -v virt-install &> /dev/null || MISSING_DEPS="$MISSING_DEPS virtinst"
command -v qemu-img &> /dev/null || MISSING_DEPS="$MISSING_DEPS qemu-utils"

if [ -n "$MISSING_DEPS" ]; then
    echo
    echo "⚠ Missing dependencies:$MISSING_DEPS"
    echo
    echo "Install them with:"
    echo "  sudo apt install$MISSING_DEPS"
    echo
fi

# Check PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo
    echo "Note: Add $INSTALL_DIR to your PATH:"
    echo "  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc"
    echo "  source ~/.bashrc"
    echo
fi

echo
echo "=== Installation Complete ==="
echo
echo "Run with: proxy-vm-wizard"
echo
echo "If your user is not in the libvirt group, add it:"
echo "  sudo usermod -aG libvirt \$USER"
echo "  (log out and back in)"
echo


