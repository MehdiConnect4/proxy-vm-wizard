# Proxy VM Wizard - User Guide

## Overview

Proxy VM Wizard helps you create isolated virtual machines that route all their traffic through a proxy gateway. This is useful for:

- Separating work and personal browsing
- Banking with extra isolation
- Testing through different network paths
- Privacy-focused workflows

## Concepts

### Roles

A **role** is a named grouping of VMs and network resources. For example, you might have:
- `work` - For work-related browsing
- `banking` - For financial sites
- `personal` - For personal use

Each role has:
- A **gateway VM** that handles proxying/VPN
- One or more **app VMs** that use the gateway
- An isolated **network** connecting them

### Templates

**Templates** are base qcow2 disk images used to create VMs. You should have at least:
- A **gateway template** (Debian recommended - minimal and stable)
- An **app template** (Debian or Fedora)

Templates are never modified - VMs use overlay disks on top of them.

### Gateway Modes

Each gateway can use one of three modes:

1. **Proxy Chain** - Route through SOCKS5/HTTP proxies
2. **WireGuard** - Use a WireGuard VPN
3. **OpenVPN** - Use an OpenVPN connection

## Getting Started

### Step 1: Prepare Your System

Make sure libvirt is running:
```bash
sudo systemctl enable --now libvirtd
```

Verify your user has access:
```bash
virsh list --all
```

If you get a permission error, add yourself to the libvirt group and log out/in.

### Step 2: Create a Base Image

You need a qcow2 base image. Options:

**Option A: Download a cloud image**
```bash
# Debian
wget https://cloud.debian.org/images/cloud/bookworm/latest/debian-12-generic-amd64.qcow2

# Fedora
wget https://download.fedoraproject.org/pub/fedora/linux/releases/39/Cloud/x86_64/images/Fedora-Cloud-Base-39-1.5.x86_64.qcow2
```

**Option B: Create from an ISO** (using virt-manager or virt-install)

### Step 3: Set Up Your LAN Network

The gateway VM needs to connect to an upstream network. This is typically:
- A bridge to your physical network
- A connection to a pfSense/OPNsense VM

Create a network called `lan-net` in virt-manager or with virsh.

### Step 4: Register Templates

1. Open Proxy VM Wizard
2. Go to **📁 Templates**
3. Click **➕ Add Template**
4. Browse to your qcow2 file
5. Set the OS variant (e.g., `debian12`)
6. Set the role kind:
   - **Proxy/Gateway** for gateway images
   - **App** for application VM images

### Step 5: Create Your First Role

1. Go to **🧙 Wizard**
2. Enter a role name (e.g., `work`)
3. Select your gateway template
4. Select your app template
5. Choose a gateway mode and configure it
6. Click **Create Role**

### Step 6: Use Your VMs

From the **📊 Dashboard**:
- **Start** the gateway VM
- **Create** app VMs as needed
- **Launch** disposable VMs for one-time use

## Gateway Configuration

### Proxy Chain

Add 1-8 proxy hops:
1. Select type (SOCKS5 or HTTP)
2. Enter host and port
3. Add credentials if needed
4. Use **Test Connection** to verify

### WireGuard

1. Click **Browse** to select your .conf file
2. The file will be copied to the role directory
3. Set the interface name (default: wg0)
4. Choose whether to route all traffic

### OpenVPN

1. Click **Browse** to select your .ovpn file
2. Optionally add an auth file for credentials
3. Choose whether to route all traffic

## Tips

### Performance

- Gateway VMs need minimal resources (1GB RAM is fine)
- App VMs should have more resources for actual work
- Use SSD storage for better performance

### Security

- Use different roles for different trust levels
- Don't share templates between high and low security roles
- Regularly update your base images

### Troubleshooting

- If VMs won't start, check libvirt logs: `journalctl -u libvirtd`
- If networking fails, verify the gateway VM is running
- Check the gateway's `/proxy/proxy.conf` for configuration


