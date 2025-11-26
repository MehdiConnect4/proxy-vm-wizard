//! Libvirt/QEMU integration via CLI tools (virsh, virt-install, qemu-img)

use crate::{Error, NetworkInfo, NetworkState, Result, VmInfo, VmKind, VmState};
use std::collections::HashMap;
use std::fs;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

/// Helper to convert Path to &str with proper error handling
fn path_to_str(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| Error::validation(format!("Invalid path encoding: {}", path.display())))
}

/// Output from a command execution
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CommandOutput {
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Adapter for libvirt/QEMU operations via CLI
#[derive(Debug, Clone)]
pub struct LibvirtAdapter {
    /// Timeout for connectivity tests in seconds
    pub connect_timeout_secs: u64,
}

impl Default for LibvirtAdapter {
    fn default() -> Self {
        Self {
            connect_timeout_secs: 5,
        }
    }
}

impl LibvirtAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    // ==================== Command Execution ====================

    /// Run a command and capture output
    pub fn run_cmd(&self, cmd: &str, args: &[&str]) -> Result<CommandOutput> {
        let output = Command::new(cmd).args(args).output().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::CommandNotFound(cmd.to_string())
            } else {
                Error::Command {
                    cmd: format!("{} {}", cmd, args.join(" ")),
                    message: e.to_string(),
                }
            }
        })?;

        Ok(self.parse_output(output))
    }

    fn parse_output(&self, output: Output) -> CommandOutput {
        CommandOutput {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }
    }

    // ==================== Prerequisite Checks ====================

    /// Check if all required commands are available
    pub fn check_prerequisites(&self) -> Result<Vec<String>> {
        let required = ["virsh", "virt-install", "qemu-img"];
        let mut missing = Vec::new();

        for cmd in required {
            if Command::new("which")
                .arg(cmd)
                .output()
                .map(|o| !o.status.success())
                .unwrap_or(true)
            {
                missing.push(cmd.to_string());
            }
        }

        if !missing.is_empty() {
            return Err(Error::CommandNotFound(format!(
                "Required commands not found: {}. Install with: sudo apt install libvirt-clients virtinst qemu-utils",
                missing.join(", ")
            )));
        }

        Ok(missing)
    }

    /// Check if the current user has libvirt access
    pub fn check_libvirt_access(&self) -> Result<()> {
        let output = self.run_cmd("virsh", &["list", "--all"])?;
        if !output.success() {
            return Err(Error::PermissionDenied(format!(
                "Cannot access libvirt. Ensure you are in the 'libvirt' group or run with sudo. Error: {}",
                output.stderr
            )));
        }
        Ok(())
    }

    // ==================== Network Management ====================

    /// Check if a network exists
    pub fn network_exists(&self, name: &str) -> Result<bool> {
        let output = self.run_cmd("virsh", &["net-info", name])?;
        Ok(output.success())
    }

    /// Get network info
    pub fn get_network_info(&self, name: &str) -> Result<Option<NetworkInfo>> {
        let output = self.run_cmd("virsh", &["net-info", name])?;
        if !output.success() {
            return Ok(None);
        }

        let mut info = NetworkInfo {
            name: name.to_string(),
            state: NetworkState::Unknown,
            autostart: false,
        };

        for line in output.stdout.lines() {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() != 2 {
                continue;
            }
            let key = parts[0].trim().to_lowercase();
            let value = parts[1].trim();

            match key.as_str() {
                "active" => {
                    info.state = if value.eq_ignore_ascii_case("yes") {
                        NetworkState::Active
                    } else {
                        NetworkState::Inactive
                    };
                }
                "autostart" => {
                    info.autostart = value.eq_ignore_ascii_case("yes");
                }
                _ => {}
            }
        }

        Ok(Some(info))
    }

    /// Ensure the LAN network exists (does not auto-create)
    pub fn ensure_lan_net_exists(&self, lan_net: &str) -> Result<()> {
        if !self.network_exists(lan_net)? {
            return Err(Error::libvirt(format!(
                "LAN network '{}' does not exist in libvirt. Create/verify it via virt-manager (this is your pfSense LAN).",
                lan_net
            )));
        }
        Ok(())
    }

    /// Ensure the role-specific internal network exists, creating if necessary
    /// Returns true if the network was created, false if it already existed
    pub fn ensure_role_network(&self, role: &str) -> Result<bool> {
        let net_name = format!("{}-inet", role);

        if self.network_exists(&net_name)? {
            return Ok(false);
        }

        // Create temporary XML file for network definition
        let xml = format!(
            r#"<network>
  <name>{}</name>
  <bridge stp='on' delay='0'/>
</network>"#,
            net_name
        );

        let tmp_path = std::env::temp_dir().join(format!("net-{}.xml", net_name));
        fs::write(&tmp_path, &xml)?;

        // Define, autostart, and start the network
        let define_output = self.run_cmd("virsh", &["net-define", path_to_str(&tmp_path)?])?;
        if !define_output.success() {
            fs::remove_file(&tmp_path).ok();
            return Err(Error::libvirt(format!(
                "Failed to define network '{}': {}",
                net_name, define_output.stderr
            )));
        }

        let autostart_output = self.run_cmd("virsh", &["net-autostart", &net_name])?;
        if !autostart_output.success() {
            // Cleanup on failure
            self.run_cmd("virsh", &["net-undefine", &net_name]).ok();
            fs::remove_file(&tmp_path).ok();
            return Err(Error::libvirt(format!(
                "Failed to set autostart for network '{}': {}",
                net_name, autostart_output.stderr
            )));
        }

        let start_output = self.run_cmd("virsh", &["net-start", &net_name])?;
        if !start_output.success() {
            // Cleanup on failure
            self.run_cmd("virsh", &["net-destroy", &net_name]).ok();
            self.run_cmd("virsh", &["net-undefine", &net_name]).ok();
            fs::remove_file(&tmp_path).ok();
            return Err(Error::libvirt(format!(
                "Failed to start network '{}': {}",
                net_name, start_output.stderr
            )));
        }

        fs::remove_file(&tmp_path).ok();
        Ok(true)
    }

    /// Destroy and undefine a network
    pub fn destroy_network(&self, name: &str) -> Result<()> {
        self.run_cmd("virsh", &["net-destroy", name]).ok();
        let output = self.run_cmd("virsh", &["net-undefine", name])?;
        if !output.success() && !output.stderr.contains("not found") {
            return Err(Error::libvirt(format!(
                "Failed to undefine network '{}': {}",
                name, output.stderr
            )));
        }
        Ok(())
    }

    // ==================== Disk Management ====================

    /// Check if a template is in the libvirt images directory
    pub fn is_in_images_dir(&self, path: &Path, images_dir: &Path) -> bool {
        path.starts_with(images_dir)
    }

    /// Run a privileged command using pkexec (shows graphical password prompt)
    fn run_privileged(&self, cmd: &str, args: &[&str]) -> Result<CommandOutput> {
        // Build the full command as a single string for pkexec
        let mut full_args = vec![cmd];
        full_args.extend(args);

        self.run_cmd("pkexec", &full_args)
    }

    /// Copy a template to the libvirt images directory using pkexec (graphical sudo)
    /// Returns the new path, or the existing path if file already exists
    pub fn copy_template_to_images_dir(&self, source: &Path, images_dir: &Path) -> Result<PathBuf> {
        let filename = source
            .file_name()
            .ok_or_else(|| Error::template("Invalid template filename"))?;
        let dest = images_dir.join(filename);

        // If destination already exists, just return it (user can reuse same base image)
        if dest.exists() {
            return Ok(dest);
        }

        // Copy using pkexec (shows graphical password dialog)
        let source_str = source
            .to_str()
            .ok_or_else(|| Error::validation("Invalid source path encoding"))?;
        let dest_str_tmp = dest
            .to_str()
            .ok_or_else(|| Error::validation("Invalid destination path encoding"))?;
        let output = self.run_privileged("cp", &[source_str, dest_str_tmp])?;
        if !output.success() {
            return Err(Error::libvirt(format!(
                "Failed to copy template: {}",
                output.stderr
            )));
        }

        // Set proper ownership (libvirt-qemu:kvm or root:root depending on system)
        // Try libvirt-qemu first, fallback to just leaving it as root
        let dest_str = dest
            .to_str()
            .ok_or_else(|| Error::validation("Invalid path encoding"))?;
        if let Ok(chown_output) = self.run_privileged("chown", &["libvirt-qemu:kvm", dest_str]) {
            if !chown_output.success() {
                // Try alternative ownership
                self.run_privileged("chown", &["root:root", dest_str]).ok();
            }
        } else {
            // Try alternative ownership
            self.run_privileged("chown", &["root:root", dest_str]).ok();
        }

        // Ensure readable
        self.run_privileged("chmod", &["644", dest_str]).ok();

        Ok(dest)
    }

    /// Ensure the images directory exists and is writable (may need pkexec)
    pub fn ensure_images_dir(&self, images_dir: &Path) -> Result<()> {
        if !images_dir.exists() {
            let output = self.run_privileged("mkdir", &["-p", path_to_str(images_dir)?])?;
            if !output.success() {
                return Err(Error::libvirt(format!(
                    "Failed to create images directory: {}",
                    output.stderr
                )));
            }
        }
        Ok(())
    }

    /// Create a qcow2 overlay disk backed by a template
    /// Uses pkexec if the overlay path is in a system directory
    pub fn create_overlay_disk(&self, template_path: &Path, overlay_path: &Path) -> Result<()> {
        // Verify template exists
        if !template_path.exists() {
            return Err(Error::template(format!(
                "Template disk does not exist: {}",
                template_path.display()
            )));
        }

        // Check overlay doesn't already exist
        if overlay_path.exists() {
            return Err(Error::AlreadyExists(format!(
                "Overlay disk already exists: {}",
                overlay_path.display()
            )));
        }

        // Convert paths to strings for command arguments
        let template_str = path_to_str(template_path)?;
        let overlay_str = path_to_str(overlay_path)?;

        // Ensure parent directory exists
        if let Some(parent) = overlay_path.parent() {
            if !parent.exists() {
                if let Ok(parent_str) = path_to_str(parent) {
                    self.run_privileged("mkdir", &["-p", parent_str]).ok();
                }
            }
        }

        // Check if we need elevated privileges (writing to system directories)
        let needs_privilege = overlay_path.starts_with("/var/lib")
            || overlay_path.starts_with("/usr")
            || overlay_path.starts_with("/etc");

        let output = if needs_privilege {
            self.run_privileged(
                "qemu-img",
                &[
                    "create",
                    "-f",
                    "qcow2",
                    "-F",
                    "qcow2",
                    "-b",
                    template_str,
                    overlay_str,
                ],
            )?
        } else {
            self.run_cmd(
                "qemu-img",
                &[
                    "create",
                    "-f",
                    "qcow2",
                    "-F",
                    "qcow2",
                    "-b",
                    template_str,
                    overlay_str,
                ],
            )?
        };

        if !output.success() {
            return Err(Error::libvirt(format!(
                "Failed to create overlay disk: {}",
                output.stderr
            )));
        }

        // Set proper permissions if we used privilege
        if needs_privilege {
            self.run_privileged("chmod", &["644", overlay_str]).ok();
        }

        Ok(())
    }

    /// Delete an overlay disk (uses pkexec for system directories)
    pub fn delete_overlay_disk(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }

        // Check if we need elevated privileges
        let needs_privilege =
            path.starts_with("/var/lib") || path.starts_with("/usr") || path.starts_with("/etc");

        let path_str = path_to_str(path)?;

        if needs_privilege {
            let output = self.run_privileged("rm", &["-f", path_str])?;
            if !output.success() && !output.stderr.contains("No such file") {
                return Err(Error::libvirt(format!(
                    "Failed to delete overlay: {}",
                    output.stderr
                )));
            }
        } else {
            fs::remove_file(path).ok();
        }
        Ok(())
    }

    /// Get the overlay disk path for a gateway VM
    pub fn gateway_overlay_path(&self, images_dir: &Path, role: &str) -> PathBuf {
        images_dir.join(format!("{}-gw.qcow2", role))
    }

    /// Get the overlay disk path for an app VM
    pub fn app_overlay_path(&self, images_dir: &Path, role: &str, number: u32) -> PathBuf {
        images_dir.join(format!("{}-app-{}-overlay.qcow2", role, number))
    }

    /// Get the overlay disk path for a disposable VM
    pub fn disposable_overlay_path(&self, cfg_root: &Path, role: &str) -> PathBuf {
        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let disp_dir = cfg_root.join(role).join("disposable");
        fs::create_dir_all(&disp_dir).ok();
        disp_dir.join(format!("disp-{}.qcow2", timestamp))
    }

    // ==================== VM Management ====================

    /// Check if a VM (domain) exists
    pub fn vm_exists(&self, name: &str) -> Result<bool> {
        let output = self.run_cmd("virsh", &["dominfo", name])?;
        Ok(output.success())
    }

    /// Get VM info
    pub fn get_vm_info(&self, name: &str) -> Result<Option<VmInfo>> {
        let output = self.run_cmd("virsh", &["dominfo", name])?;
        if !output.success() {
            return Ok(None);
        }

        let mut info = VmInfo {
            name: name.to_string(),
            state: VmState::Unknown,
            kind: VmKind::ProxyGateway,
            role: None,
        };

        for line in output.stdout.lines() {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() != 2 {
                continue;
            }
            let key = parts[0].trim().to_lowercase();
            let value = parts[1].trim();

            if key == "state" {
                info.state = VmState::from_virsh_state(value);
            }
        }

        // Determine role and kind from name
        if name.ends_with("-gw") {
            info.kind = VmKind::ProxyGateway;
            info.role = Some(name.strip_suffix("-gw").unwrap_or(name).to_string());
        } else if name.contains("-app-") {
            info.kind = VmKind::App;
            if let Some(role) = name.split("-app-").next() {
                info.role = Some(role.to_string());
            }
        } else if name.starts_with("disp-") {
            info.kind = VmKind::DisposableApp;
        }

        Ok(Some(info))
    }

    /// List all VMs matching a pattern
    pub fn list_vms(&self, pattern: Option<&str>) -> Result<Vec<VmInfo>> {
        let output = self.run_cmd("virsh", &["list", "--all", "--name"])?;
        if !output.success() {
            return Err(Error::libvirt(format!(
                "Failed to list VMs: {}",
                output.stderr
            )));
        }

        let mut vms = Vec::new();
        for line in output.stdout.lines() {
            let name = line.trim();
            if name.is_empty() {
                continue;
            }
            if let Some(pattern) = pattern {
                if !name.contains(pattern) {
                    continue;
                }
            }
            if let Some(info) = self.get_vm_info(name)? {
                vms.push(info);
            }
        }

        Ok(vms)
    }

    /// List VMs for a specific role
    pub fn list_role_vms(&self, role: &str) -> Result<Vec<VmInfo>> {
        self.list_vms(Some(role))
    }

    /// Build virt-install arguments for a gateway VM
    #[allow(clippy::too_many_arguments)]
    pub fn build_gateway_virt_install_args(
        &self,
        vm_name: &str,
        overlay_path: &Path,
        lan_net: &str,
        role_net: &str,
        role_dir: &Path,
        os_variant: &str,
        ram_mb: u32,
    ) -> Vec<String> {
        vec![
            "--name".to_string(),
            vm_name.to_string(),
            "--memory".to_string(),
            ram_mb.to_string(),
            "--vcpus".to_string(),
            "1".to_string(),
            "--import".to_string(),
            "--disk".to_string(),
            format!("path={},format=qcow2", overlay_path.display()),
            "--network".to_string(),
            format!("network={},model=virtio", lan_net),
            "--network".to_string(),
            format!("network={},model=virtio", role_net),
            "--filesystem".to_string(),
            format!(
                "source={},target=proxy,accessmode=mapped",
                role_dir.display()
            ),
            "--os-variant".to_string(),
            os_variant.to_string(),
            "--noautoconsole".to_string(),
        ]
    }

    /// Build virt-install arguments for an app VM
    pub fn build_app_virt_install_args(
        &self,
        vm_name: &str,
        overlay_path: &Path,
        role_net: &str,
        os_variant: &str,
        ram_mb: u32,
        share_dir: Option<&Path>,
    ) -> Vec<String> {
        let mut args = vec![
            "--name".to_string(),
            vm_name.to_string(),
            "--memory".to_string(),
            ram_mb.to_string(),
            "--vcpus".to_string(),
            "2".to_string(),
            "--import".to_string(),
            "--disk".to_string(),
            format!("path={},format=qcow2", overlay_path.display()),
            "--network".to_string(),
            format!("network={},model=virtio", role_net),
            "--os-variant".to_string(),
            os_variant.to_string(),
            "--noautoconsole".to_string(),
        ];

        if let Some(share) = share_dir {
            args.push("--filesystem".to_string());
            args.push(format!(
                "source={},target=shared,accessmode=mapped",
                share.display()
            ));
        }

        args
    }

    /// Build virt-install arguments for a disposable VM
    pub fn build_disposable_virt_install_args(
        &self,
        vm_name: &str,
        overlay_path: &Path,
        role_net: &str,
        os_variant: &str,
        ram_mb: u32,
    ) -> Vec<String> {
        vec![
            "--name".to_string(),
            vm_name.to_string(),
            "--memory".to_string(),
            ram_mb.to_string(),
            "--vcpus".to_string(),
            "2".to_string(),
            "--import".to_string(),
            "--transient".to_string(),
            "--disk".to_string(),
            format!("path={},format=qcow2", overlay_path.display()),
            "--network".to_string(),
            format!("network={},model=virtio", role_net),
            "--os-variant".to_string(),
            os_variant.to_string(),
            "--noautoconsole".to_string(),
        ]
    }

    /// Create a gateway VM
    #[allow(clippy::too_many_arguments)]
    pub fn create_gateway_vm(
        &self,
        vm_name: &str,
        overlay_path: &Path,
        lan_net: &str,
        role_net: &str,
        role_dir: &Path,
        os_variant: &str,
        ram_mb: u32,
    ) -> Result<()> {
        // Check VM doesn't already exist
        if self.vm_exists(vm_name)? {
            return Err(Error::AlreadyExists(format!(
                "VM '{}' already exists",
                vm_name
            )));
        }

        let args = self.build_gateway_virt_install_args(
            vm_name,
            overlay_path,
            lan_net,
            role_net,
            role_dir,
            os_variant,
            ram_mb,
        );

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let output = self.run_cmd("virt-install", &args_refs)?;

        if !output.success() {
            return Err(Error::libvirt(format!(
                "Failed to create VM '{}': {}",
                vm_name, output.stderr
            )));
        }

        Ok(())
    }

    /// Create an app VM
    pub fn create_app_vm(
        &self,
        vm_name: &str,
        overlay_path: &Path,
        role_net: &str,
        os_variant: &str,
        ram_mb: u32,
        share_dir: Option<&Path>,
    ) -> Result<()> {
        if self.vm_exists(vm_name)? {
            return Err(Error::AlreadyExists(format!(
                "VM '{}' already exists",
                vm_name
            )));
        }

        let args = self.build_app_virt_install_args(
            vm_name,
            overlay_path,
            role_net,
            os_variant,
            ram_mb,
            share_dir,
        );

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let output = self.run_cmd("virt-install", &args_refs)?;

        if !output.success() {
            return Err(Error::libvirt(format!(
                "Failed to create VM '{}': {}",
                vm_name, output.stderr
            )));
        }

        Ok(())
    }

    /// Create a disposable (transient) VM
    pub fn create_disposable_vm(
        &self,
        vm_name: &str,
        overlay_path: &Path,
        role_net: &str,
        os_variant: &str,
        ram_mb: u32,
    ) -> Result<()> {
        let args = self.build_disposable_virt_install_args(
            vm_name,
            overlay_path,
            role_net,
            os_variant,
            ram_mb,
        );

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let output = self.run_cmd("virt-install", &args_refs)?;

        if !output.success() {
            return Err(Error::libvirt(format!(
                "Failed to create disposable VM '{}': {}",
                vm_name, output.stderr
            )));
        }

        Ok(())
    }

    /// Start a VM
    pub fn start_vm(&self, name: &str) -> Result<()> {
        let output = self.run_cmd("virsh", &["start", name])?;
        if !output.success() {
            return Err(Error::libvirt(format!(
                "Failed to start VM '{}': {}",
                name, output.stderr
            )));
        }
        Ok(())
    }

    /// Stop a VM (graceful shutdown)
    pub fn stop_vm(&self, name: &str) -> Result<()> {
        let output = self.run_cmd("virsh", &["shutdown", name])?;
        if !output.success() {
            return Err(Error::libvirt(format!(
                "Failed to stop VM '{}': {}",
                name, output.stderr
            )));
        }
        Ok(())
    }

    /// Force stop a VM
    pub fn destroy_vm(&self, name: &str) -> Result<()> {
        let output = self.run_cmd("virsh", &["destroy", name])?;
        if !output.success() && !output.stderr.contains("not running") {
            return Err(Error::libvirt(format!(
                "Failed to destroy VM '{}': {}",
                name, output.stderr
            )));
        }
        Ok(())
    }

    /// Undefine (delete) a VM
    pub fn undefine_vm(&self, name: &str) -> Result<()> {
        // First try to destroy if running
        self.destroy_vm(name).ok();

        let output = self.run_cmd("virsh", &["undefine", name])?;
        if !output.success() && !output.stderr.contains("failed to get domain") {
            return Err(Error::libvirt(format!(
                "Failed to undefine VM '{}': {}",
                name, output.stderr
            )));
        }
        Ok(())
    }

    /// Full cleanup: destroy VM, undefine, delete overlay
    pub fn cleanup_vm(&self, name: &str, overlay_path: Option<&Path>) -> Result<()> {
        self.destroy_vm(name).ok();
        self.undefine_vm(name).ok();
        if let Some(path) = overlay_path {
            self.delete_overlay_disk(path).ok();
        }
        Ok(())
    }

    // ==================== Connectivity Testing ====================

    /// Get the disk image path for a VM by parsing its XML definition
    pub fn get_vm_disk_path(&self, vm_name: &str) -> Result<Option<PathBuf>> {
        let output = self.run_cmd("virsh", &["dumpxml", vm_name])?;
        if !output.success() {
            return Ok(None);
        }

        // Parse the XML to find the disk source
        // Look for: <source file='/path/to/disk.qcow2'/>
        for line in output.stdout.lines() {
            let line = line.trim();
            if line.contains("<source file=") {
                // Extract path from: <source file='/path/to/file.qcow2'/>
                if let Some(start) = line.find("file='") {
                    let path_start = start + 6;
                    if let Some(end) = line[path_start..].find('\'') {
                        let path_str = &line[path_start..path_start + end];
                        return Ok(Some(PathBuf::from(path_str)));
                    }
                }
                // Also try double quotes
                if let Some(start) = line.find("file=\"") {
                    let path_start = start + 6;
                    if let Some(end) = line[path_start..].find('"') {
                        let path_str = &line[path_start..path_start + end];
                        return Ok(Some(PathBuf::from(path_str)));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Get a map of disk paths to VM names for all VMs
    pub fn get_disk_to_vm_map(&self) -> Result<HashMap<PathBuf, Vec<String>>> {
        let mut map: HashMap<PathBuf, Vec<String>> = HashMap::new();

        // Get list of all VMs
        let output = self.run_cmd("virsh", &["list", "--all", "--name"])?;
        if !output.success() {
            return Ok(map);
        }

        for line in output.stdout.lines() {
            let vm_name = line.trim();
            if vm_name.is_empty() {
                continue;
            }

            if let Ok(Some(disk_path)) = self.get_vm_disk_path(vm_name) {
                map.entry(disk_path).or_default().push(vm_name.to_string());
            }
        }

        Ok(map)
    }

    /// Get all VMs that use a specific disk or its overlays (checks backing file chain)
    pub fn get_vms_using_image(&self, image_path: &Path) -> Result<Vec<String>> {
        let mut vms = Vec::new();
        let disk_map = self.get_disk_to_vm_map()?;

        // Direct match - VM uses this image directly
        if let Some(vm_list) = disk_map.get(image_path) {
            vms.extend(vm_list.clone());
        }

        // Check for overlays - VMs might use overlay disks backed by this image
        // For each VM disk, check if its backing file is our image
        for (disk_path, vm_names) in &disk_map {
            if let Ok(Some(backing)) = self.get_backing_file(disk_path) {
                if backing.as_path() == image_path {
                    vms.extend(vm_names.clone());
                }
            }
        }

        // Remove duplicates
        vms.sort();
        vms.dedup();
        Ok(vms)
    }

    /// Get the backing file for a qcow2 image
    pub fn get_backing_file(&self, disk_path: &Path) -> Result<Option<PathBuf>> {
        let path_str = path_to_str(disk_path)?;
        let output = self.run_cmd("qemu-img", &["info", path_str])?;
        if !output.success() {
            return Ok(None);
        }

        // Look for: backing file: /path/to/backing.qcow2
        for line in output.stdout.lines() {
            let line = line.trim();
            if line.starts_with("backing file:") {
                let path_str = line.strip_prefix("backing file:").unwrap_or("").trim();
                // Handle cases where there might be extra info after the path
                let path_str = path_str.split_whitespace().next().unwrap_or(path_str);
                if !path_str.is_empty() {
                    return Ok(Some(PathBuf::from(path_str)));
                }
            }
        }
        Ok(None)
    }

    /// Test TCP connectivity to a host:port
    pub fn test_tcp_connection(&self, host: &str, port: u16) -> Result<()> {
        let addr_str = format!("{}:{}", host, port);
        let addrs: Vec<SocketAddr> = addr_str
            .to_socket_addrs()
            .map_err(|e| Error::ConnectionTest {
                host: host.to_string(),
                port,
                reason: format!("DNS resolution failed: {}", e),
            })?
            .collect();

        if addrs.is_empty() {
            return Err(Error::ConnectionTest {
                host: host.to_string(),
                port,
                reason: "No addresses resolved".to_string(),
            });
        }

        let timeout = Duration::from_secs(self.connect_timeout_secs);
        for addr in addrs {
            match TcpStream::connect_timeout(&addr, timeout) {
                Ok(_) => return Ok(()),
                Err(_) => continue,
            }
        }

        Err(Error::ConnectionTest {
            host: host.to_string(),
            port,
            reason: "Connection timed out or refused".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_virt_install_args() {
        let adapter = LibvirtAdapter::new();
        let args = adapter.build_gateway_virt_install_args(
            "work-gw",
            Path::new("/var/lib/libvirt/images/work-gw.qcow2"),
            "lan-net",
            "work-inet",
            Path::new("/home/user/VMS/VM-Proxy-configs/work"),
            "debian12",
            512,
        );

        assert!(args.contains(&"--name".to_string()));
        assert!(args.contains(&"work-gw".to_string()));
        assert!(args.contains(&"--import".to_string()));
        assert!(args.iter().any(|a| a.contains("lan-net")));
        assert!(args.iter().any(|a| a.contains("work-inet")));
        assert!(args.iter().any(|a| a.contains("proxy,accessmode=mapped")));
    }

    #[test]
    fn test_app_virt_install_args() {
        let adapter = LibvirtAdapter::new();
        let args = adapter.build_app_virt_install_args(
            "work-app-1",
            Path::new("/var/lib/libvirt/images/work-app-1.qcow2"),
            "work-inet",
            "fedora40",
            2048,
            None,
        );

        assert!(args.contains(&"--name".to_string()));
        assert!(args.contains(&"work-app-1".to_string()));
        assert!(args.contains(&"2048".to_string()));
        assert!(args.iter().any(|a| a.contains("work-inet")));
        // Should not have lan-net
        assert!(!args.iter().any(|a| a.contains("lan-net")));
    }

    #[test]
    fn test_disposable_virt_install_args() {
        let adapter = LibvirtAdapter::new();
        let args = adapter.build_disposable_virt_install_args(
            "disp-work-20240101-120000",
            Path::new("/tmp/disp.qcow2"),
            "work-inet",
            "debian12",
            2048,
        );

        assert!(args.contains(&"--transient".to_string()));
        assert!(args.contains(&"--import".to_string()));
    }

    #[test]
    fn test_vm_state_parsing() {
        assert_eq!(VmState::from_virsh_state("running"), VmState::Running);
        assert_eq!(VmState::from_virsh_state("Running"), VmState::Running);
        assert_eq!(VmState::from_virsh_state("shut off"), VmState::ShutOff);
        assert_eq!(VmState::from_virsh_state("paused"), VmState::Paused);
        assert_eq!(VmState::from_virsh_state("unknown"), VmState::Unknown);
    }
}
