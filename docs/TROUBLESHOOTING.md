# Troubleshooting Guide

## Common Issues

### Forgot Password

**Cause**: Password is lost and there's no recovery mechanism.

**Solution**: 
You'll need to delete the encrypted configuration and start fresh:
```bash
# Backup if needed (though you can't decrypt it)
cp -r ~/.config/proxy-vm-wizard ~/.config/proxy-vm-wizard.backup

# Remove config
rm -rf ~/.config/proxy-vm-wizard/

# Next launch will prompt for new password setup
```

**Note**: You'll need to re-register all templates and recreate roles. The VMs themselves are not encrypted and can be managed via virt-manager.

### "Incorrect password" on login

**Cause**: Wrong password entered.

**Solution**: 
1. Try again carefully - passwords are case-sensitive
2. If you've truly forgotten it, see "Forgot Password" above

### "Prerequisite Error" on startup

**Cause**: Missing libvirt commands or permissions.

**Solution**:
```bash
# Install required packages
sudo apt install libvirt-clients virtinst qemu-utils

# Add user to libvirt group
sudo usermod -aG libvirt $USER

# Log out and back in
```

### "LAN network does not exist"

**Cause**: The `lan-net` network hasn't been created in libvirt.

**Solution**: Create a bridge network in virt-manager or with virsh:
```bash
# Create a simple NAT network for testing
cat > /tmp/lan-net.xml << EOF
<network>
  <name>lan-net</name>
  <forward mode='nat'/>
  <bridge name='virbr1' stp='on' delay='0'/>
  <ip address='192.168.100.1' netmask='255.255.255.0'>
    <dhcp>
      <range start='192.168.100.100' end='192.168.100.200'/>
    </dhcp>
  </ip>
</network>
EOF

virsh net-define /tmp/lan-net.xml
virsh net-start lan-net
virsh net-autostart lan-net
```

### "Permission denied" when creating overlay

**Cause**: The app tried to write to `/var/lib/libvirt/images/` without elevated privileges.

**Solution**: The app should prompt for password via pkexec. If it doesn't:
```bash
# Ensure pkexec is installed
sudo apt install policykit-1

# Or manually create with sudo
sudo qemu-img create -f qcow2 -F qcow2 -b /path/to/template.qcow2 /var/lib/libvirt/images/overlay.qcow2
```

### "Template file not accessible by libvirt"

**Cause**: Template is in a location libvirt/QEMU can't access.

**Solution**: 
1. The app will automatically copy templates to `/var/lib/libvirt/images/`
2. If that fails, manually copy:
```bash
sudo cp /path/to/template.qcow2 /var/lib/libvirt/images/
sudo chown libvirt-qemu:kvm /var/lib/libvirt/images/template.qcow2
```

### VM starts but network doesn't work

**Cause**: Gateway VM isn't properly configured or not running.

**Solution**:
1. Ensure gateway VM is running (check Dashboard)
2. Check gateway VM console for errors
3. Verify `/proxy/proxy.conf` exists in the gateway
4. Run `/proxy/apply-proxy.sh` manually in the gateway

### "Domain is not running" when stopping VM

**Cause**: VM was already stopped (e.g., from virt-manager).

**Solution**: This is just a warning - click Refresh to update the state.

### Wizard fails partway through

**Cause**: Various (network issue, disk space, permissions).

**Solution**:
1. Note the error message
2. Click **Clean Up & Cancel** to remove partial resources
3. Fix the underlying issue
4. Try again

### Proxy connection test fails

**Cause**: Proxy server unreachable or wrong credentials.

**Solution**:
1. Verify proxy server is running
2. Check host/port are correct
3. Test from command line:
```bash
curl --socks5 proxy.example.com:1080 https://ifconfig.me
```

## Logs and Debugging

### Application logs

Run from terminal to see logs:
```bash
RUST_LOG=debug proxy-vm-wizard
```

### Libvirt logs

```bash
journalctl -u libvirtd -f
```

### QEMU logs

```bash
# Find the log file
ls /var/log/libvirt/qemu/

# View specific VM log
sudo tail -f /var/log/libvirt/qemu/work-gw.log
```

### VM console

Access the VM console via virt-manager or:
```bash
virsh console work-gw
```

## Reset Everything

If you want to start fresh:

```bash
# Remove all VMs for a role
virsh destroy work-gw
virsh undefine work-gw
virsh destroy work-app-1
virsh undefine work-app-1

# Remove network
virsh net-destroy work-inet
virsh net-undefine work-inet

# Remove overlay disks
sudo rm /var/lib/libvirt/images/work-*.qcow2

# Remove config
rm -rf ~/VMS/VM-Proxy-configs/work/

# Remove app config
rm -rf ~/.config/proxy-vm-wizard/
```

## Getting Help

1. Check the [FAQ](FAQ.md)
2. Search [existing issues](https://github.com/proxyvmwizard/proxy-vm-wizard/issues)
3. Open a new issue with:
   - OS version
   - libvirt version (`virsh --version`)
   - Steps to reproduce
   - Error messages
   - Relevant logs


