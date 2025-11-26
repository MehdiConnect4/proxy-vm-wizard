# Template Setup Guide

Templates are the foundation of Proxy VM Wizard. This guide explains how to create and configure them.

## What is a Template?

A template is a qcow2 disk image that serves as the base for all VMs. When you create a VM, an overlay disk is created on top of the template, so the original template is never modified.

## Recommended Templates

### Gateway Template (Debian)

For proxy/gateway VMs, we recommend **Debian stable** because it's:
- Minimal and lightweight
- Stable and long-term supported
- Well-tested with QEMU/KVM

**Creating a Gateway Template:**

1. Download a Debian cloud image:
```bash
wget https://cloud.debian.org/images/cloud/bookworm/latest/debian-12-generic-amd64.qcow2
sudo mv debian-12-generic-amd64.qcow2 /var/lib/libvirt/images/
```

2. Or create from scratch using the included setup script:
```bash
# Boot a minimal Debian VM, then run:
curl -O https://raw.githubusercontent.com/proxyvmwizard/proxy-vm-wizard/main/setup-proxy-template.sh
sudo bash setup-proxy-template.sh
# Shut down and use the disk as template
```

The setup script:
- Installs minimal network tools
- Configures IP forwarding
- Sets up the 9p mount for `/proxy`
- Installs the boot service that runs `/proxy/apply-proxy.sh`
- Cleans up for cloning

### App Template (Debian or Fedora)

For app VMs, you have more flexibility:

**Debian** - Stable, lightweight
```bash
wget https://cloud.debian.org/images/cloud/bookworm/latest/debian-12-generic-amd64.qcow2
```

**Fedora** - Newer packages, good hardware support
```bash
wget https://download.fedoraproject.org/pub/fedora/linux/releases/39/Cloud/x86_64/images/Fedora-Cloud-Base-39-1.5.x86_64.qcow2
```

## Registering Templates

1. Open Proxy VM Wizard
2. Go to **📁 Templates**
3. Click **➕ Add Template**
4. Fill in:
   - **Label**: Human-readable name (e.g., "Debian 12 Gateway")
   - **Path**: Select your qcow2 file
   - **OS Variant**: e.g., `debian12`, `fedora39`
   - **Role Kind**: Gateway, App, or Generic
   - **Default RAM**: 1024 MB for gateways, 2048+ for apps

## OS Variants

The OS variant tells virt-install how to optimize the VM. Common values:

| OS | Variant |
|----|---------|
| Debian 12 | `debian12` |
| Debian 13 | `debian13` |
| Fedora 39 | `fedora39` |
| Fedora 40 | `fedora40` |
| Ubuntu 22.04 | `ubuntu22.04` |
| Ubuntu 24.04 | `ubuntu24.04` |

To see all available variants:
```bash
osinfo-query os
```

## Storage Location

Templates should be stored in `/var/lib/libvirt/images/` for best compatibility. When you add a template from another location, the app will automatically copy it there.

## Template Requirements

### Gateway Templates Must Have:

1. **9p support** in the kernel
2. **Fstab entry** for mounting `/proxy`:
   ```
   proxy  /proxy  9p  trans=virtio,version=9p2000.L,msize=262144  0  0
   ```
3. **Boot service** to run `/proxy/apply-proxy.sh`
4. **proxychains** installed (for proxy chain mode)

### App Templates Should Have:

1. Basic desktop environment (if GUI needed)
2. Configured to get network via DHCP
3. Any applications you want pre-installed

## Updating Templates

If you need to update a template:

1. Create a new VM from the template
2. Make your changes
3. Clean up:
   ```bash
   # Clear bash history
   history -c
   # Clear machine-id
   sudo truncate -s 0 /etc/machine-id
   # Clear DHCP leases
   sudo rm -f /var/lib/dhcp/*
   ```
4. Shut down the VM
5. The overlay disk becomes your new template
6. Register it in the app

## Multiple Templates

You can have multiple templates for different purposes:

- `debian12-gateway` - Minimal Debian for gateways
- `debian12-desktop` - Debian with GNOME for general use
- `fedora40-dev` - Fedora with development tools
- `ubuntu24-browser` - Ubuntu with hardened browser

Register each with the appropriate Role Kind and use them when creating roles.


