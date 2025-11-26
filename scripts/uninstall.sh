#!/bin/bash
# Proxy VM Wizard Uninstallation Script
set -e

INSTALL_DIR="${INSTALL_DIR:-/usr/local}"

echo "=== Proxy VM Wizard Uninstaller ==="
echo

if [ "$EUID" -ne 0 ]; then
    echo "This script needs root privileges."
    echo "Please run with sudo: sudo ./uninstall.sh"
    exit 1
fi

echo "Removing files..."

rm -f "${INSTALL_DIR}/bin/proxy-vm-wizard"
rm -f "${INSTALL_DIR}/share/applications/io.github.proxyvmwizard.ProxyVmWizard.desktop"
rm -f "${INSTALL_DIR}/share/icons/hicolor/scalable/apps/io.github.proxyvmwizard.ProxyVmWizard.svg"
rm -f "${INSTALL_DIR}/share/metainfo/io.github.proxyvmwizard.ProxyVmWizard.metainfo.xml"

# Update caches
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database "${INSTALL_DIR}/share/applications" 2>/dev/null || true
fi

if command -v gtk-update-icon-cache &> /dev/null; then
    gtk-update-icon-cache -f -t "${INSTALL_DIR}/share/icons/hicolor" 2>/dev/null || true
fi

echo
echo "=== Uninstallation Complete ==="
echo
echo "Note: User configuration files in ~/.config/proxy-vm-wizard/ were not removed."
echo "Delete them manually if you want a complete removal."
echo


