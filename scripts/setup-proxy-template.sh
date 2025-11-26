#!/bin/bash
#
# Setup script for Proxy VM Gateway Template
# Run this inside a fresh Debian VM to prepare it as a gateway template
#
set -e

echo "=== Proxy VM Gateway Template Setup ==="
echo

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "Please run as root: sudo $0"
    exit 1
fi

# Detect OS
if [ -f /etc/debian_version ]; then
    PKG_MANAGER="apt-get"
    PKG_UPDATE="apt-get update"
    PKG_INSTALL="apt-get install -y"
elif [ -f /etc/fedora-release ]; then
    PKG_MANAGER="dnf"
    PKG_UPDATE="dnf check-update || true"
    PKG_INSTALL="dnf install -y"
else
    echo "Unsupported OS. This script supports Debian and Fedora."
    exit 1
fi

echo "[1/7] Updating package lists..."
$PKG_UPDATE

echo "[2/7] Installing required packages..."
$PKG_INSTALL \
    proxychains4 \
    iptables \
    iproute2 \
    wireguard-tools \
    openvpn \
    curl \
    wget \
    dnsutils \
    net-tools \
    procps

echo "[3/7] Enabling IP forwarding..."
cat > /etc/sysctl.d/99-ip-forward.conf << 'EOF'
net.ipv4.ip_forward = 1
net.ipv6.conf.all.forwarding = 1
EOF
sysctl -p /etc/sysctl.d/99-ip-forward.conf

echo "[4/7] Creating /proxy mount point..."
mkdir -p /proxy

# Add fstab entry for 9p mount
if ! grep -q '^proxy' /etc/fstab; then
    echo "proxy  /proxy  9p  trans=virtio,version=9p2000.L,msize=262144,nofail  0  0" >> /etc/fstab
fi

echo "[5/7] Creating boot service for apply-proxy.sh..."
cat > /etc/systemd/system/apply-proxy.service << 'EOF'
[Unit]
Description=Apply proxy/VPN configuration
After=network-online.target local-fs.target
Wants=network-online.target

[Service]
Type=oneshot
ExecStartPre=/bin/sleep 5
ExecStart=/bin/bash -c 'if [ -x /proxy/apply-proxy.sh ]; then /proxy/apply-proxy.sh; fi'
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable apply-proxy.service

echo "[6/7] Creating NAT helper script..."
cat > /usr/local/bin/setup-nat.sh << 'EOF'
#!/bin/bash
# Setup NAT from role network to upstream
# Usage: setup-nat.sh <upstream-interface> <downstream-interface>

UPSTREAM=${1:-eth0}
DOWNSTREAM=${2:-eth1}

# Enable masquerading
iptables -t nat -A POSTROUTING -o $UPSTREAM -j MASQUERADE

# Allow forwarding
iptables -A FORWARD -i $DOWNSTREAM -o $UPSTREAM -j ACCEPT
iptables -A FORWARD -i $UPSTREAM -o $DOWNSTREAM -m state --state RELATED,ESTABLISHED -j ACCEPT

echo "NAT configured: $DOWNSTREAM -> $UPSTREAM"
EOF
chmod +x /usr/local/bin/setup-nat.sh

echo "[7/7] Cleaning up for template use..."

# Clear machine-specific data
truncate -s 0 /etc/machine-id 2>/dev/null || true
rm -f /var/lib/dbus/machine-id 2>/dev/null || true

# Clear SSH host keys (will regenerate on first boot)
rm -f /etc/ssh/ssh_host_* 2>/dev/null || true

# Clear DHCP leases
rm -f /var/lib/dhcp/* 2>/dev/null || true
rm -f /var/lib/dhclient/* 2>/dev/null || true

# Clear logs
journalctl --rotate 2>/dev/null || true
journalctl --vacuum-time=1s 2>/dev/null || true
rm -rf /var/log/*.gz /var/log/*.1 /var/log/*.old 2>/dev/null || true

# Clear bash history
history -c
rm -f /root/.bash_history
rm -f /home/*/.bash_history

# Clear tmp
rm -rf /tmp/* /var/tmp/* 2>/dev/null || true

echo
echo "=== Setup Complete ==="
echo
echo "Next steps:"
echo "1. Shut down this VM: shutdown -h now"
echo "2. The disk image is now ready to use as a gateway template"
echo "3. Register it in Proxy VM Wizard as a 'Proxy/Gateway' template"
echo


