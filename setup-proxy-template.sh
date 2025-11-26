#!/usr/bin/env bash
#
# setup-proxy-template.sh (v2 + proxy.mount ordering fix)
#
# Turn a fresh minimal Debian VM into a hardened proxy TEMPLATE VM:
#  - Minimal packages only (network + proxy client tools)
#  - IP forwarding enabled
#  - Journald mostly in RAM (limited persistent logs)
#  - machine-id & DHCP state scrubbed for clean clones
#  - /proxy mount prepared (9p from host) + fstab entry
#  - proxy-boot.sh + systemd unit to run /proxy/apply-proxy.sh on boot
#  - root autologin on tty1 (console) for this template
#
# Run ONCE, as root, INSIDE the VM.
#
set -u -o pipefail   # we handle -e manually to get proper reporting

# -------------------- helpers --------------------

declare -A STEP_STATUS
declare -A STEP_MSG
CRITICAL_FAIL=0

banner() {
  echo
  echo "============================================================"
  echo "$*"
  echo "============================================================"
  echo
}

log()   { echo "[*] $*"; }
warn()  { echo "[WARN] $*" >&2; }
fatal() { echo "[FATAL] $*" >&2; exit 1; }

step_run() {
  local id="$1" desc="$2" severity="$3" fn="$4"
  banner "$desc"
  log "Running step: $id ($severity)"

  if "$fn"; then
    STEP_STATUS["$id"]="OK"
  else
    if [[ "$severity" == "critical" ]]; then
      STEP_STATUS["$id"]="FAIL"
      CRITICAL_FAIL=1
      STEP_MSG["$id"]="Critical step failed in function: $fn"
    else
      STEP_STATUS["$id"]="WARN"
      STEP_MSG["$id"]="Non-critical step had issues in function: $fn"
    fi
  fi
}

print_summary_and_exit() {
  echo
  echo "=================== SETUP SUMMARY ==================="
  for id in "${!STEP_STATUS[@]}"; do
    printf "  %-15s : %s\n" "$id" "${STEP_STATUS[$id]}"
    if [[ -n "${STEP_MSG[$id]:-}" ]]; then
      echo "    -> ${STEP_MSG[$id]}"
    fi
  done
  echo "============================================================"
  echo

  if (( CRITICAL_FAIL )); then
    fatal "One or more CRITICAL steps failed. Template is NOT safe to use."
  else
    echo "[OK] All critical steps succeeded. Template is ready to be frozen."
    exit 0
  fi
}

# -------------------- step functions --------------------

step_safety() {
  if [[ "${EUID:-$(id -u)}" -ne 0 ]]; then
    echo "Script must be run as root."
    return 1
  fi

  if ! grep -qi debian /etc/os-release 2>/dev/null; then
    echo "This script is intended for Debian only."
    return 1
  fi

  if ! command -v systemctl >/dev/null 2>&1; then
    echo "systemd/systemctl not found – this template assumes systemd."
    return 1
  fi

  return 0
}

step_dns() {
  # Ensure we have at least one nameserver; if not, set Quad9.
  if ! grep -Eq '^\s*nameserver\s+' /etc/resolv.conf 2>/dev/null; then
    echo "nameserver 9.9.9.9" > /etc/resolv.conf || return 1
    log "resolv.conf was empty – set nameserver 9.9.9.9 (Quad9)."
  else
    log "resolv.conf already has a nameserver – leaving it as-is."
  fi
  return 0
}

step_packages() {
  local pkgs=(
    iproute2
    iputils-ping
    curl
    wget
    ca-certificates
    netcat-openbsd
    proxychains
  )

  log "Updating APT index..."
  if ! apt update -y; then
    warn "apt update failed."
    return 1
  fi

  log "Installing packages: ${pkgs[*]}"
  if ! apt install -y "${pkgs[@]}"; then
    warn "apt install failed."
    return 1
  fi

  log "Cleaning APT cache..."
  apt autoremove -y || warn "apt autoremove had issues."
  apt clean || warn "apt clean had issues."

  return 0
}

step_ip_forward() {
  if ! grep -q '^net.ipv4.ip_forward=1' /etc/sysctl.conf 2>/dev/null; then
    echo 'net.ipv4.ip_forward=1' >> /etc/sysctl.conf || return 1
    log "Added net.ipv4.ip_forward=1 to /etc/sysctl.conf"
  else
    log "net.ipv4.ip_forward already set."
  fi

  sysctl -p || warn "sysctl -p reported an error (continuing)."
  return 0
}

step_identity_scrub() {
  log "Resetting /etc/machine-id..."
  truncate -s 0 /etc/machine-id || return 1
  ln -sf /etc/machine-id /var/lib/dbus/machine-id || return 1

  log "Clearing DHCP client state..."
  rm -f /var/lib/dhcp/* 2>/dev/null || true

  return 0
}

step_journald() {
  log "Configuring journald for mostly in-RAM logging..."
  mkdir -p /etc/systemd/journald.conf.d || return 1

  cat > /etc/systemd/journald.conf.d/10-proxy-volatile.conf << 'EOF'
[Journal]
Storage=volatile
Compress=yes
SystemMaxUse=50M
RuntimeMaxUse=50M
EOF

  systemctl restart systemd-journald || {
    warn "systemd-journald restart failed (journald config still written)."
    return 1
  }
  return 0
}

step_proxy_mount() {
  log "Ensuring /proxy mount point exists..."
  mkdir -p /proxy || return 1

  local line="proxy  /proxy  9p  trans=virtio,version=9p2000.L,msize=262144  0  0"

  if ! grep -q '^proxy[[:space:]]\+/proxy[[:space:]]\+9p' /etc/fstab 2>/dev/null; then
    echo "$line" >> /etc/fstab || return 1
    log "Added 9p mount to /etc/fstab: $line"
  else
    log "9p mount for /proxy already present in /etc/fstab."
  fi

  # This may fail if host 9p isn't wired yet -> WARN, not FAIL.
  log "Testing mount -a for /proxy (may WARN if host 9p not ready)..."
  if ! mount -a; then
    warn "mount -a reported an error (likely 9p not configured on host yet)."
    return 1
  fi

  return 0
}

step_boot_hook() {
  log "Creating /usr/local/sbin/proxy-boot.sh..."
  cat > /usr/local/sbin/proxy-boot.sh << 'EOF'
#!/usr/bin/env bash
set -euo pipefail

# This runs at boot in every Proxy VM created from this template.
# Real per-VM logic lives in /proxy/apply-proxy.sh, provided by the host.

if [ -x /proxy/apply-proxy.sh ]; then
    /proxy/apply-proxy.sh
fi
EOF

  chmod +x /usr/local/sbin/proxy-boot.sh || return 1

  log "Creating /etc/systemd/system/proxy-boot.service..."
  cat > /etc/systemd/system/proxy-boot.service << 'EOF'
[Unit]
Description=Proxy Boot Initialization
After=network-online.target proxy.mount
Wants=network-online.target
Requires=proxy.mount

[Service]
Type=oneshot
ExecStart=/usr/local/sbin/proxy-boot.sh
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
EOF

  systemctl daemon-reload || return 1
  systemctl enable proxy-boot.service || return 1

  # Best-effort test run, not fatal if it fails.
  systemctl start proxy-boot.service || warn "proxy-boot.service start failed (probably no /proxy/apply-proxy.sh yet)."

  return 0
}

step_root_autologin() {
  log "Configuring root autologin on tty1..."
  mkdir -p /etc/systemd/system/getty@tty1.service.d || return 1

  local agetty_path
  agetty_path="$(command -v agetty || echo /sbin/agetty)"

  cat > /etc/systemd/system/getty@tty1.service.d/override.conf <<EOF
[Service]
ExecStart=
ExecStart=-${agetty_path} --autologin root --noclear %I
EOF

  systemctl daemon-reload || return 1

  if ! systemctl restart getty@tty1.service; then
    warn "Failed to restart getty@tty1.service; autologin will apply next boot."
  fi

  return 0
}

# -------------------- main --------------------

banner "Proxy Template Setup – Debian Tiny → Hardened Proxy Template"

step_run "safety"        "Safety checks (root + Debian)"           "critical" step_safety
step_run "dns"           "DNS sanity (Quad9 fallback if empty)"   "critical" step_dns
step_run "packages"      "Install minimal proxy toolset"          "critical" step_packages
step_run "ip_forward"    "Enable IPv4 forwarding"                 "critical" step_ip_forward
step_run "identity"      "Scrub machine-id & DHCP state"          "critical" step_identity_scrub
step_run "journald"      "Configure journald (volatile logs)"     "warning"  step_journald
step_run "proxy_mount"   "Prepare /proxy mount & fstab 9p entry"  "warning"  step_proxy_mount
step_run "boot_hook"     "Install proxy boot hook + systemd unit" "critical" step_boot_hook
step_run "root_autologin" "Enable root autologin on tty1 (console)" "critical" step_root_autologin

print_summary_and_exit

