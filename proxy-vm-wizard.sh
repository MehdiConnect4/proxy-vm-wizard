#!/usr/bin/env bash
# proxy-vm-wizard.sh (overlay + per-role config)
#
# Host-side script to:
#   - ask for a role name (work, bank, etc.)
#   - build a per-role config dir under CFG_ROOT
#   - generate proxy config into proxy.conf (supports multi-proxy chains)
#   - generate /proxy/apply-proxy.sh stub that applies that config on boot
#   - create an isolated libvirt network <role>-inet (if missing)
#   - create a qcow2 overlay disk on top of TEMPLATE_DISK
#   - define a new VM <role>-gw using virt-install:
#         NIC1 on LAN_NET (pfSense LAN)
#         NIC2 on <role>-inet
#         9p share "proxy" -> per-role config dir
#
# Assumptions:
#   - host user: kurajiko
#   - libvirt/QEMU environment
#   - TEMPLATE_DISK points to your read-only proxy template qcow2

set -euo pipefail

CFG_ROOT="/home/kurajiko/VMS/VM-Proxy-configs"
IMAGES_DIR="/var/lib/libvirt/images"
TEMPLATE_DISK="${IMAGES_DIR}/debian13-1.qcow2"
LAN_NET="lan-net"
RAM_MB=512
OS_VARIANT="debian12"

# global state for cleanup
TMP=""
ROLE=""
ROLE_NET=""
NEW_VM=""
OVERLAY=""
CREATED_ROLE_NET=0
SUCCESS=0

require_cmd() {
  for cmd in "$@"; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
      echo "ERROR: required command '$cmd' not found in PATH." >&2
      echo "       Install it (e.g. 'sudo apt install libvirt-clients virtinst qemu-utils')." >&2
      exit 1
    fi
  done
}

test_tcp_connect() {
  local HOST="$1"
  local PORT="$2"
  printf "  - Testing TCP connectivity to %s:%s ... " "$HOST" "$PORT"
  if timeout 5 bash -c ">/dev/tcp/$HOST/$PORT" 2>/dev/null; then
    echo "OK"
    return 0
  else
    echo "FAILED"
    return 1
  fi
}

cleanup() {
  local code=$?
  # always remove temp config file if present
  if [[ -n "${TMP:-}" && -f "$TMP" ]]; then
    rm -f "$TMP"
  fi

  # if SUCCESS==1, normal exit -> do not touch libvirt resources
  if [[ "${SUCCESS:-0}" -eq 1 ]]; then
    return
  fi

  # best-effort cleanup on abort / error
  if [[ -n "${NEW_VM:-}" ]]; then
    virsh destroy "$NEW_VM" >/dev/null 2>&1 || true
    virsh undefine "$NEW_VM" >/dev/null 2>&1 || true
  fi

  if [[ -n "${OVERLAY:-}" && -f "$OVERLAY" ]]; then
    rm -f "$OVERLAY" >/dev/null 2>&1 || true
  fi

  if [[ "${CREATED_ROLE_NET:-0}" -eq 1 && -n "${ROLE_NET:-}" ]]; then
    virsh net-destroy "$ROLE_NET" >/dev/null 2>&1 || true
    virsh net-undefine "$ROLE_NET" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT INT TERM

cfgdir_for_role() {
  local role="$1"
  echo "${CFG_ROOT}/${role}"
}

ensure_template_disk() {
  if [[ ! -f "$TEMPLATE_DISK" ]]; then
    echo "ERROR: template disk '$TEMPLATE_DISK' does not exist." >&2
    echo "       Adjust TEMPLATE_DISK in this script or move your qcow2 there." >&2
    exit 1
  fi
}

ensure_lan_net_exists() {
  if ! virsh net-info "$LAN_NET" >/dev/null 2>&1; then
    echo "ERROR: LAN network '$LAN_NET' does not exist in libvirt." >&2
    echo "       Create/verify it via virt-manager (this is your pfSense LAN)." >&2
    exit 1
  fi
}

ensure_role_network() {
  local R="$1"
  ROLE_NET="${R}-inet"

  if virsh net-info "$ROLE_NET" >/dev/null 2>&1; then
    echo "Network '$ROLE_NET' already exists – reusing."
    CREATED_ROLE_NET=0
    return
  fi

  echo "Creating isolated libvirt network '$ROLE_NET'..."
  local NX
  NX="$(mktemp)"
  cat > "$NX" <<EOF
<network>
  <name>${ROLE_NET}</name>
  <bridge stp='on' delay='0'/>
</network>
EOF

  virsh net-define "$NX"
  virsh net-autostart "$ROLE_NET"
  virsh net-start "$ROLE_NET"
  rm -f "$NX"
  CREATED_ROLE_NET=1
}

set_kv_in_tmp() {
  local KEY="$1"
  local VAL="$2"
  local VAL_ESCAPED
  VAL_ESCAPED=$(printf '%s\n' "$VAL" | sed 's/[&/]/\\&/g')

  if grep -q "^$KEY=" "$TMP" 2>/dev/null; then
    sed -i "s|^$KEY=.*|$KEY=$VAL_ESCAPED|" "$TMP"
  else
    printf '%s=%s\n' "$KEY" "$VAL_ESCAPED" >> "$TMP"
  fi
}

configure_proxy_chain() {
  local ROLE="$1"
  local ROLE_DIR
  ROLE_DIR="$(cfgdir_for_role "$ROLE")"

  echo
  echo "How many proxies in the chain for role '$ROLE'? [1]"
  read -rp "Number of proxies (1-8): " COUNT
  if [[ -z "$COUNT" ]]; then
    COUNT=1
  fi
  if ! [[ "$COUNT" =~ ^[0-9]+$ ]] || [[ "$COUNT" -lt 1 || "$COUNT" -gt 8 ]]; then
    echo "Invalid number of proxies. Must be between 1 and 8." >&2
    exit 1
  fi

  set_kv_in_tmp "GATEWAY_MODE" "PROXY_CHAIN"
  set_kv_in_tmp "CHAIN_STRATEGY" "strict_chain"
  set_kv_in_tmp "PROXY_COUNT" "$COUNT"

  local idx
  for ((idx=1; idx<=COUNT; idx++)); do
    echo
    echo "=== Proxy $idx of $COUNT ==="
    echo "  1) SOCKS5"
    echo "  2) HTTP"
    local PNUM
    read -rp "Proxy type [1]: " PNUM
    local PTYPE
    case "$PNUM" in
      2) PTYPE="HTTP" ;;
      ""|1) PTYPE="SOCKS5" ;;
      *)
        echo "Invalid choice." >&2
        exit 1
        ;;
    esac

    local H PORT U P LABEL

    while true; do
      read -rp "Host (IP or domain): " H
      read -rp "Port: " PORT
      read -rp "Username (optional): " U
      read -rp "Password (optional): " P
      read -rp "Label/notes (optional, type/provider/etc.): " LABEL

      if [[ -z "$H" || -z "$PORT" ]]; then
        echo "Host and port are required." >&2
        continue
      fi
      if ! [[ "$PORT" =~ ^[0-9]+$ ]] || [[ "$PORT" -lt 1 || "$PORT" -gt 65535 ]]; then
        echo "Invalid port number." >&2
        continue
      fi

      if test_tcp_connect "$H" "$PORT"; then
        break
      else
        echo "Connection test FAILED. Re-enter this proxy or Ctrl+C to abort."
      fi
    done

    set_kv_in_tmp "PROXY_${idx}_TYPE" "$PTYPE"
    set_kv_in_tmp "PROXY_${idx}_HOST" "$H"
    set_kv_in_tmp "PROXY_${idx}_PORT" "$PORT"
    set_kv_in_tmp "PROXY_${idx}_USER" "$U"
    set_kv_in_tmp "PROXY_${idx}_PASS" "$P"
    set_kv_in_tmp "PROXY_${idx}_LABEL" "$LABEL"

    # For backwards compatibility populate first proxy fields
    if [[ "$idx" -eq 1 ]]; then
      set_kv_in_tmp "ACTIVE_PROTOCOL" "$PTYPE"
      if [[ "$PTYPE" = "SOCKS5" ]]; then
        set_kv_in_tmp "SOCKS5_HOST" "$H"
        set_kv_in_tmp "SOCKS5_PORT" "$PORT"
        set_kv_in_tmp "SOCKS5_USER" "$U"
        set_kv_in_tmp "SOCKS5_PASS" "$P"
      elif [[ "$PTYPE" = "HTTP" ]]; then
        set_kv_in_tmp "HTTP_HOST" "$H"
        set_kv_in_tmp "HTTP_PORT" "$PORT"
        set_kv_in_tmp "HTTP_USER" "$U"
        set_kv_in_tmp "HTTP_PASS" "$P"
      fi
    fi
  done
}

generate_config_for_role() {
  local ROLE="$1"
  local ROLE_DIR
  ROLE_DIR="$(cfgdir_for_role "$ROLE")"
  local CFG="${ROLE_DIR}/proxy.conf"
  TMP="${CFG}.tmp.$$"

  mkdir -p "$ROLE_DIR"

  # base skeleton
  cat > "$TMP" <<EOF
# Proxy config for role: ${ROLE}
GATEWAY_MODE=
CHAIN_STRATEGY=strict_chain
PROXY_COUNT=0

# First proxy (for compatibility)
ACTIVE_PROTOCOL=
SOCKS5_HOST=
SOCKS5_PORT=
SOCKS5_USER=
SOCKS5_PASS=
HTTP_HOST=
HTTP_PORT=
HTTP_USER=
HTTP_PASS=

# VPN / other modes (paths only, handled by template later)
WG_CONFIG_PATH=
WG_INTERFACE_NAME=
WG_ROUTE_ALL_TRAFFIC=
OPENVPN_CONFIG_PATH=
OPENVPN_AUTH_FILE=
OPENVPN_ROUTE_ALL_TRAFFIC=
EOF

  echo
  echo "Select gateway mode for role '$ROLE':"
  echo "  1) Proxy chain (SOCKS5/HTTP) [default]"
  echo "  2) WireGuard VPN (config path only, no magic here)"
  echo "  3) OpenVPN      (config path only, no magic here)"
  read -rp "Enter number [1-3]: " MODE_NUM

  local MODE
  case "$MODE_NUM" in
    2) MODE="WIREGUARD" ;;
    3) MODE="OPENVPN" ;;
    ""|1) MODE="PROXY_CHAIN" ;;
    *)
      echo "Invalid choice." >&2
      exit 1
      ;;
  esac

  if [[ "$MODE" = "PROXY_CHAIN" ]]; then
    configure_proxy_chain "$ROLE"
  elif [[ "$MODE" = "WIREGUARD" ]]; then
    set_kv_in_tmp "GATEWAY_MODE" "WIREGUARD"
    local WGFILE IFACE ALL
    echo
    echo "WireGuard mode selected."
    echo "Place your WireGuard config file inside: ${ROLE_DIR} (seen as /proxy/<file> in VM)."
    read -rp "WireGuard config filename (e.g. wg_${ROLE}.conf): " WGFILE
    read -rp "WireGuard interface name [wg0]: " IFACE
    read -rp "Route all traffic through WireGuard? (true/false): " ALL

    if [[ -z "$WGFILE" ]]; then
      echo "WireGuard config filename is required." >&2
      exit 1
    fi
    [[ -z "$IFACE" ]] && IFACE="wg0"

    set_kv_in_tmp "WG_CONFIG_PATH" "/proxy/$WGFILE"
    set_kv_in_tmp "WG_INTERFACE_NAME" "$IFACE"
    [[ -n "$ALL" ]] && set_kv_in_tmp "WG_ROUTE_ALL_TRAFFIC" "$ALL"
  elif [[ "$MODE" = "OPENVPN" ]]; then
    set_kv_in_tmp "GATEWAY_MODE" "OPENVPN"
    local OVCFG AUTH ALL
    echo
    echo "OpenVPN mode selected."
    echo "Place your OpenVPN files inside: ${ROLE_DIR} (seen as /proxy/<file> in VM)."
    read -rp "OpenVPN client config filename (e.g. client_${ROLE}.ovpn): " OVCFG
    read -rp "OpenVPN auth file (optional, e.g. auth_${ROLE}.txt): " AUTH
    read -rp "Route all traffic through OpenVPN? (true/false): " ALL

    if [[ -z "$OVCFG" ]]; then
      echo "OpenVPN config filename is required." >&2
      exit 1
    fi

    set_kv_in_tmp "OPENVPN_CONFIG_PATH" "/proxy/$OVCFG"
    [[ -n "$AUTH" ]] && set_kv_in_tmp "OPENVPN_AUTH_FILE" "/proxy/$AUTH"
    [[ -n "$ALL"  ]] && set_kv_in_tmp "OPENVPN_ROUTE_ALL_TRAFFIC" "$ALL"
  fi

  mv "$TMP" "$CFG"
  echo
  echo "Config written: $CFG"
  echo "Inside VM it will be readable as: /proxy/proxy.conf"
}

write_apply_proxy_stub() {
  local ROLE="$1"
  local ROLE_DIR
  ROLE_DIR="$(cfgdir_for_role "$ROLE")"
  local SCRIPT="${ROLE_DIR}/apply-proxy.sh"

  cat > "$SCRIPT" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

ROLE="__ROLE__"
CONF="/proxy/proxy.conf"
OUT="/etc/proxychains.conf"

log() { echo "[apply-proxy][${ROLE}] $*"; }

if [[ ! -f "$CONF" ]]; then
  log "Config file $CONF not found – nothing to do."
  exit 0
fi

# shellcheck disable=SC1090
. "$CONF" || {
  log "Failed to source config from $CONF."
  exit 1
}

MODE="${GATEWAY_MODE:-}"
if [[ "$MODE" = "PROXY_CHAIN" ]]; then
  COUNT="${PROXY_COUNT:-0}"
  if ! [[ "$COUNT" =~ ^[0-9]+$ ]] || [[ "$COUNT" -lt 1 ]]; then
    log "PROXY_CHAIN mode but PROXY_COUNT is invalid ('$COUNT')."
    exit 0
  fi

  STRAT="${CHAIN_STRATEGY:-strict_chain}"
  cat > "$OUT" <<EOC
# Auto-generated by apply-proxy.sh for role ${ROLE}
${STRAT}
proxy_dns
tcp_read_time_out 15000
tcp_connect_time_out 8000

[ProxyList]
EOC

  any=0
  for ((i=1; i<=COUNT; i++)); do
    T=""
    H=""
    P=""
    U=""
    PW=""
    eval "T=\"\${PROXY_${i}_TYPE:-}\""
    eval "H=\"\${PROXY_${i}_HOST:-}\""
    eval "P=\"\${PROXY_${i}_PORT:-}\""
    eval "U=\"\${PROXY_${i}_USER:-}\""
    eval "PW=\"\${PROXY_${i}_PASS:-}\""

    if [[ -z "$T" || -z "$H" || -z "$P" ]]; then
      log "Proxy $i incomplete (type/host/port missing) – skipping."
      continue
    fi

    case "$T" in
      SOCKS5|socks5)
        if [[ -n "$U" || -n "$PW" ]]; then
          echo "socks5 $H $P $U $PW" >> "$OUT"
        else
          echo "socks5 $H $P" >> "$OUT"
        fi
        any=1
        ;;
      HTTP|http)
        if [[ -n "$U" || -n "$PW" ]]; then
          echo "http $H $P $U $PW" >> "$OUT"
        else
          echo "http $H $P" >> "$OUT"
        fi
        any=1
        ;;
      *)
        log "Proxy $i has unsupported type '$T' – skipping."
        ;;
    esac
  done

  if [[ "$any" -eq 0 ]]; then
    log "No valid proxies found in chain – leaving $OUT untouched."
    exit 0
  fi

  log "proxychains.conf updated for PROXY_CHAIN (count=$COUNT)."
  exit 0
fi

# Backward compatibility: single ACTIVE_PROTOCOL mode
case "${ACTIVE_PROTOCOL:-}" in
  SOCKS5)
    if [[ -z "${SOCKS5_HOST:-}" || -z "${SOCKS5_PORT:-}" ]]; then
      log "SOCKS5 selected but SOCKS5_HOST or SOCKS5_PORT is empty."
      exit 0
    fi
    cat > "$OUT" <<EOC
# Auto-generated by apply-proxy.sh for role ${ROLE}
strict_chain
proxy_dns
tcp_read_time_out 15000
tcp_connect_time_out 8000

[ProxyList]
EOC
    if [[ -n "${SOCKS5_USER:-}" || -n "${SOCKS5_PASS:-}" ]]; then
      echo "socks5 ${SOCKS5_HOST} ${SOCKS5_PORT} ${SOCKS5_USER:-} ${SOCKS5_PASS:-}" >> "$OUT"
    else
      echo "socks5 ${SOCKS5_HOST} ${SOCKS5_PORT}" >> "$OUT"
    fi
    log "proxychains.conf updated for single SOCKS5."
    ;;
  HTTP)
    if [[ -z "${HTTP_HOST:-}" || -z "${HTTP_PORT:-}" ]]; then
      log "HTTP selected but HTTP_HOST or HTTP_PORT is empty."
      exit 0
    fi
    cat > "$OUT" <<EOC
# Auto-generated by apply-proxy.sh for role ${ROLE}
strict_chain
proxy_dns
tcp_read_time_out 15000
tcp_connect_time_out 8000

[ProxyList]
EOC
    if [[ -n "${HTTP_USER:-}" || -n "${HTTP_PASS:-}" ]]; then
      echo "http ${HTTP_HOST} ${HTTP_PORT} ${HTTP_USER:-} ${HTTP_PASS:-}" >> "$OUT"
    else
      echo "http ${HTTP_HOST} ${HTTP_PORT}" >> "$OUT"
    fi
    log "proxychains.conf updated for single HTTP."
    ;;
  *)
    log "GATEWAY_MODE='${MODE}' and ACTIVE_PROTOCOL='${ACTIVE_PROTOCOL:-}' – nothing to do in apply-proxy.sh yet."
    ;;
esac

exit 0
EOF

  chmod +x "$SCRIPT"
  sed -i "s/__ROLE__/$ROLE/g" "$SCRIPT"
  echo "apply-proxy stub written: $SCRIPT (inside VM: /proxy/apply-proxy.sh)"
}

create_proxy_vm_for_role() {
  local R="$1"
  ROLE="$R"
  NEW_VM="${ROLE}-gw"
  ROLE_NET="${ROLE}-inet"
  local ROLE_DIR
  ROLE_DIR="$(cfgdir_for_role "$ROLE")"
  OVERLAY="${IMAGES_DIR}/${NEW_VM}.qcow2"

  if virsh dominfo "$NEW_VM" >/dev/null 2>&1; then
    echo "ERROR: domain '$NEW_VM' already exists in libvirt." >&2
    exit 1
  fi

  if [[ -f "$OVERLAY" ]]; then
    echo "ERROR: overlay disk '$OVERLAY' already exists." >&2
    exit 1
  fi

  echo "Creating qcow2 overlay disk: $OVERLAY"
  qemu-img create -f qcow2 -F qcow2 -b "$TEMPLATE_DISK" "$OVERLAY"

  echo "Defining VM '$NEW_VM' with virt-install..."
  virt-install \
    --name "$NEW_VM" \
    --memory "$RAM_MB" \
    --vcpus 1 \
    --import \
    --disk "path=$OVERLAY,format=qcow2" \
    --network "network=${LAN_NET},model=virtio" \
    --network "network=${ROLE_NET},model=virtio" \
    --filesystem "source=${ROLE_DIR},target=proxy,accessmode=mapped" \
    --os-variant "$OS_VARIANT" \
    --noautoconsole

  echo "VM '$NEW_VM' created."
}

### MAIN ###

# Hard-stop if not root (sudo)
if [[ "${EUID:-$(id -u)}" -ne 0 ]]; then
  echo "This wizard must be run as root. Use: sudo $0" >&2
  exit 1
fi

require_cmd virsh virt-install qemu-img timeout

ensure_template_disk
ensure_lan_net_exists

mkdir -p "$CFG_ROOT"

echo "=== Proxy VM Wizard (overlay mode) ==="
echo

read -rp "Role name (ex: work, personal, bank, signal): " ROLE
ROLE="${ROLE// /}"          # strip spaces
ROLE="$(echo "$ROLE" | tr '[:upper:]' '[:lower:]')"

if [[ -z "$ROLE" ]]; then
  echo "ERROR: role name cannot be empty." >&2
  exit 1
fi

echo
echo "Using template disk : $TEMPLATE_DISK"
echo "Role                : $ROLE"
echo "New VM name         : ${ROLE}-gw"
echo "Internal network    : ${ROLE}-inet"
echo "Config root dir     : $(cfgdir_for_role "$ROLE")"
echo

generate_config_for_role "$ROLE"
write_apply_proxy_stub "$ROLE"
ensure_role_network "$ROLE"
create_proxy_vm_for_role "$ROLE"

SUCCESS=1

echo
echo "=== Summary ==="
echo "  Role:        $ROLE"
echo "  Template:    $TEMPLATE_DISK"
echo "  New VM:      ${ROLE}-gw"
echo "  LAN network: $LAN_NET"
echo "  Internal net: ${ROLE}-inet"
echo "  Config dir:  $(cfgdir_for_role "$ROLE")"
echo "  In-VM paths: /proxy/proxy.conf, /proxy/apply-proxy.sh"
echo
echo "Next:"
echo "  - Create a Work VM for this role attached ONLY to '${ROLE}-inet'."
echo "  - Boot '${ROLE}-gw'; on boot it will:"
echo "      * mount /proxy (9p from host role dir)"
echo "      * run /proxy/apply-proxy.sh"
echo "      * configure /etc/proxychains.conf using your proxy chain."
echo

