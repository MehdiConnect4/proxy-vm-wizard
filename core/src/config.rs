//! Configuration management for global settings, templates, and roles

use crate::{auth, EncryptionManager, Error, GatewayMode, Result, RoleKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Current config version for migration support
pub const CONFIG_VERSION: u32 = 1;

/// Global configuration for the application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub version: u32,
    pub cfg: CfgSection,
    pub libvirt: LibvirtSection,
    pub defaults: DefaultsSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfgSection {
    /// Root directory for per-role configurations
    pub root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibvirtSection {
    /// Directory where qcow2 images are stored
    pub images_dir: PathBuf,
    /// Main LAN network for pfSense (gateway's first NIC)
    pub lan_net: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsSection {
    /// Default RAM for gateway VMs in MB
    pub gateway_ram_mb: u32,
    /// Default RAM for app VMs in MB
    pub app_ram_mb: u32,
    /// Default RAM for disposable VMs in MB
    pub disp_ram_mb: u32,
    /// Default OS variant for Debian templates
    pub debian_os_variant: String,
    /// Default OS variant for Fedora templates
    pub fedora_os_variant: String,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        Self {
            version: CONFIG_VERSION,
            cfg: CfgSection {
                root: home.join("VMS/VM-Proxy-configs"),
            },
            libvirt: LibvirtSection {
                images_dir: PathBuf::from("/var/lib/libvirt/images"),
                lan_net: "lan-net".to_string(),
            },
            defaults: DefaultsSection {
                gateway_ram_mb: 1024, // Minimum recommended for Debian
                app_ram_mb: 2048,
                disp_ram_mb: 2048,
                debian_os_variant: "debian12".to_string(),
                fedora_os_variant: "fedora40".to_string(),
            },
        }
    }
}

impl GlobalConfig {
    /// Get the default config file path
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("proxy-vm-wizard")
            .join("config.toml")
    }

    /// Load config from file, or create default if not exists
    pub fn load_or_default() -> Result<Self> {
        let path = Self::default_path();
        if path.exists() {
            Self::load(&path)
        } else {
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }

    /// Load config from a specific path
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;

        // Version migration would go here
        if config.version != CONFIG_VERSION {
            // For now, just use as-is; future versions would migrate
        }

        Ok(config)
    }

    /// Save config to the default path
    pub fn save(&self) -> Result<()> {
        let path = Self::default_path();
        self.save_to(&path)
    }

    /// Save config to a specific path
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Get the role directory for a given role name
    pub fn role_dir(&self, role: &str) -> PathBuf {
        self.cfg.root.join(role)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.libvirt.lan_net.is_empty() {
            return Err(Error::validation("LAN network name cannot be empty"));
        }
        if self.defaults.gateway_ram_mb < 128 {
            return Err(Error::validation("Gateway RAM must be at least 128 MB"));
        }
        if self.defaults.app_ram_mb < 256 {
            return Err(Error::validation("App RAM must be at least 256 MB"));
        }
        Ok(())
    }

    /// Load encrypted config from file
    pub fn load_encrypted(encryption: &EncryptionManager) -> Result<Self> {
        let path = Self::default_path();
        if !path.exists() {
            return Err(Error::NotFound("Config file not found".to_string()));
        }
        let content = encryption.decrypt_text_from_file(&path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save encrypted config to file
    pub fn save_encrypted(&self, encryption: &EncryptionManager) -> Result<()> {
        let path = Self::default_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        encryption.encrypt_text_to_file(&content, &path)?;
        Ok(())
    }

    /// Check if config file is encrypted
    pub fn is_encrypted() -> Result<bool> {
        let path = Self::default_path();
        auth::is_file_encrypted(&path)
    }
}

/// A qcow2 template for creating VMs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    /// Stable internal ID
    pub id: String,
    /// Human-readable label
    pub label: String,
    /// Absolute path to qcow2 file
    pub path: PathBuf,
    /// OS variant for virt-install (e.g., "debian12", "fedora40")
    pub os_variant: String,
    /// What kind of role this template is for
    pub role_kind: RoleKind,
    /// Default RAM in MB for VMs using this template
    pub default_ram_mb: u32,
    /// Optional notes about this template
    pub notes: Option<String>,
}

impl Template {
    /// Create a new template
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        path: PathBuf,
        os_variant: impl Into<String>,
        role_kind: RoleKind,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            path,
            os_variant: os_variant.into(),
            role_kind,
            default_ram_mb: 1024, // Minimum recommended for most OS
            notes: None,
        }
    }

    /// Check if the template file exists and is readable
    pub fn validate(&self) -> Result<()> {
        if !self.path.exists() {
            return Err(Error::template(format!(
                "Template file does not exist: {}",
                self.path.display()
            )));
        }
        if !self.path.is_file() {
            return Err(Error::template(format!(
                "Template path is not a file: {}",
                self.path.display()
            )));
        }
        // Try to read file metadata to check permissions
        fs::metadata(&self.path).map_err(|e| {
            Error::template(format!(
                "Cannot access template file {}: {}",
                self.path.display(),
                e
            ))
        })?;
        Ok(())
    }

    /// Check if template file exists (without error)
    pub fn exists(&self) -> bool {
        self.path.exists() && self.path.is_file()
    }
}

/// Registry of all templates
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemplateRegistry {
    pub version: u32,
    pub templates: HashMap<String, Template>,
}

impl TemplateRegistry {
    /// Get the default registry path
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("proxy-vm-wizard")
            .join("templates.toml")
    }

    /// Load registry from file, or create empty if not exists
    pub fn load_or_default() -> Result<Self> {
        let path = Self::default_path();
        if path.exists() {
            Self::load(&path)
        } else {
            let registry = Self::default();
            registry.save()?;
            Ok(registry)
        }
    }

    /// Load registry from a specific path
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let registry: Self = toml::from_str(&content)?;
        Ok(registry)
    }

    /// Save registry to the default path
    pub fn save(&self) -> Result<()> {
        let path = Self::default_path();
        self.save_to(&path)
    }

    /// Save registry to a specific path
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Add a template to the registry
    pub fn add(&mut self, template: Template) -> Result<()> {
        if self.templates.contains_key(&template.id) {
            return Err(Error::AlreadyExists(format!(
                "Template with ID '{}' already exists",
                template.id
            )));
        }
        self.templates.insert(template.id.clone(), template);
        Ok(())
    }

    /// Update an existing template
    pub fn update(&mut self, template: Template) -> Result<()> {
        if !self.templates.contains_key(&template.id) {
            return Err(Error::NotFound(format!(
                "Template with ID '{}' not found",
                template.id
            )));
        }
        self.templates.insert(template.id.clone(), template);
        Ok(())
    }

    /// Remove a template by ID
    pub fn remove(&mut self, id: &str) -> Result<()> {
        self.templates
            .remove(id)
            .ok_or_else(|| Error::NotFound(format!("Template with ID '{}' not found", id)))?;
        Ok(())
    }

    /// Get a template by ID
    pub fn get(&self, id: &str) -> Option<&Template> {
        self.templates.get(id)
    }

    /// Get templates by role kind
    pub fn get_by_role_kind(&self, kind: RoleKind) -> Vec<&Template> {
        self.templates
            .values()
            .filter(|t| t.role_kind == kind || t.role_kind == RoleKind::Generic)
            .collect()
    }

    /// Get all templates suitable for proxy gateways (Debian recommended)
    pub fn get_gateway_templates(&self) -> Vec<&Template> {
        self.templates
            .values()
            .filter(|t| t.role_kind == RoleKind::ProxyGateway || t.role_kind == RoleKind::Generic)
            .collect()
    }

    /// Get all templates suitable for app VMs
    pub fn get_app_templates(&self) -> Vec<&Template> {
        self.templates
            .values()
            .filter(|t| {
                t.role_kind == RoleKind::App
                    || t.role_kind == RoleKind::DisposableApp
                    || t.role_kind == RoleKind::Generic
            })
            .collect()
    }

    /// List all templates
    pub fn list(&self) -> Vec<&Template> {
        self.templates.values().collect()
    }

    /// Generate a unique ID for a new template
    pub fn generate_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }

    /// Load encrypted registry from file
    pub fn load_encrypted(encryption: &EncryptionManager) -> Result<Self> {
        let path = Self::default_path();
        if !path.exists() {
            return Err(Error::NotFound("Template registry not found".to_string()));
        }
        let content = encryption.decrypt_text_from_file(&path)?;
        let registry: Self = toml::from_str(&content)?;
        Ok(registry)
    }

    /// Save encrypted registry to file
    pub fn save_encrypted(&self, encryption: &EncryptionManager) -> Result<()> {
        let path = Self::default_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        encryption.encrypt_text_to_file(&content, &path)?;
        Ok(())
    }

    /// Check if registry file is encrypted
    pub fn is_encrypted() -> Result<bool> {
        let path = Self::default_path();
        auth::is_file_encrypted(&path)
    }
}

/// Metadata for a role (stored in role directory)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleMeta {
    pub version: u32,
    pub role_name: String,
    /// Template ID for the gateway VM
    pub gw_template_id: Option<String>,
    /// Template ID for app VMs
    pub app_template_id: Option<String>,
    /// Template ID for disposable app VMs
    pub disp_template_id: Option<String>,
    /// LAN network override (defaults from global config)
    pub lan_net: Option<String>,
    /// RAM override for gateway
    pub gw_ram_mb: Option<u32>,
    /// RAM override for app VMs
    pub app_ram_mb: Option<u32>,
    /// vCPU override for gateway
    pub gw_vcpus: Option<u32>,
    /// Gateway mode configuration
    pub gateway_mode: GatewayMode,
    /// Count of app VMs created for this role
    pub app_vm_count: u32,
}

impl RoleMeta {
    pub fn new(role_name: String) -> Self {
        Self {
            version: CONFIG_VERSION,
            role_name,
            gw_template_id: None,
            app_template_id: None,
            disp_template_id: None,
            lan_net: None,
            gw_ram_mb: None,
            app_ram_mb: None,
            gw_vcpus: None,
            gateway_mode: GatewayMode::ProxyChain,
            app_vm_count: 0,
        }
    }

    /// Get the path for this role's metadata file
    pub fn path_for_role(cfg_root: &Path, role: &str) -> PathBuf {
        cfg_root.join(role).join("role-meta.toml")
    }

    /// Load role metadata from file
    pub fn load(cfg_root: &Path, role: &str) -> Result<Self> {
        let path = Self::path_for_role(cfg_root, role);
        if !path.exists() {
            return Err(Error::NotFound(format!(
                "Role metadata not found: {}",
                path.display()
            )));
        }
        let content = fs::read_to_string(&path)?;
        let meta: Self = toml::from_str(&content)?;
        Ok(meta)
    }

    /// Save role metadata to file
    pub fn save(&self, cfg_root: &Path) -> Result<()> {
        let role_dir = cfg_root.join(&self.role_name);
        fs::create_dir_all(&role_dir)?;

        let path = Self::path_for_role(cfg_root, &self.role_name);
        let content = toml::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// Get the next app VM number
    pub fn next_app_number(&mut self) -> u32 {
        self.app_vm_count += 1;
        self.app_vm_count
    }

    /// Get gateway VM name
    pub fn gw_vm_name(&self) -> String {
        format!("{}-gw", self.role_name)
    }

    /// Get app VM name for given number
    pub fn app_vm_name(&self, number: u32) -> String {
        format!("{}-app-{}", self.role_name, number)
    }

    /// Get role network name
    pub fn role_net_name(&self) -> String {
        format!("{}-inet", self.role_name)
    }
}

/// Discover existing roles from the config root directory
pub fn discover_roles(cfg_root: &Path) -> Result<Vec<String>> {
    if !cfg_root.exists() {
        return Ok(Vec::new());
    }

    let mut roles = Vec::new();
    for entry in fs::read_dir(cfg_root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            // Check if this looks like a role directory
            let meta_path = path.join("role-meta.toml");
            let conf_path = path.join("proxy.conf");
            if meta_path.exists() || conf_path.exists() {
                if let Some(name) = path.file_name() {
                    if let Some(name_str) = name.to_str() {
                        roles.push(name_str.to_string());
                    }
                }
            }
        }
    }
    roles.sort();
    Ok(roles)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_global_config_default() {
        let config = GlobalConfig::default();
        assert_eq!(config.version, CONFIG_VERSION);
        assert_eq!(config.libvirt.lan_net, "lan-net");
        assert_eq!(config.defaults.gateway_ram_mb, 1024); // Updated per virt-install recommendations
    }

    #[test]
    fn test_global_config_save_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let config = GlobalConfig::default();
        config.save_to(&path).unwrap();

        let loaded = GlobalConfig::load(&path).unwrap();
        assert_eq!(loaded.version, config.version);
        assert_eq!(loaded.libvirt.lan_net, config.libvirt.lan_net);
    }

    #[test]
    fn test_template_registry() {
        let mut registry = TemplateRegistry::default();

        let template = Template::new(
            "test-1",
            "Test Template",
            PathBuf::from("/tmp/test.qcow2"),
            "debian12",
            RoleKind::ProxyGateway,
        );

        registry.add(template.clone()).unwrap();
        assert!(registry.get("test-1").is_some());

        // Can't add duplicate
        assert!(registry.add(template).is_err());

        // Can remove
        registry.remove("test-1").unwrap();
        assert!(registry.get("test-1").is_none());
    }

    #[test]
    fn test_role_meta() {
        let dir = tempdir().unwrap();
        let cfg_root = dir.path();

        let mut meta = RoleMeta::new("work".to_string());
        meta.gw_template_id = Some("template-1".to_string());

        meta.save(cfg_root).unwrap();

        let loaded = RoleMeta::load(cfg_root, "work").unwrap();
        assert_eq!(loaded.role_name, "work");
        assert_eq!(loaded.gw_template_id, Some("template-1".to_string()));
    }
}
