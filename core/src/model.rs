//! Domain model types for the Proxy VM Wizard

use serde::{Deserialize, Serialize};

/// Gateway mode for a proxy VM
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GatewayMode {
    #[default]
    ProxyChain,
    WireGuard,
    OpenVpn,
}

impl GatewayMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            GatewayMode::ProxyChain => "PROXY_CHAIN",
            GatewayMode::WireGuard => "WIREGUARD",
            GatewayMode::OpenVpn => "OPENVPN",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            GatewayMode::ProxyChain => "Proxy Chain (SOCKS5/HTTP)",
            GatewayMode::WireGuard => "WireGuard VPN",
            GatewayMode::OpenVpn => "OpenVPN",
        }
    }
}

/// Proxy type for a hop in the chain
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProxyType {
    #[default]
    Socks5,
    Http,
}

impl ProxyType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProxyType::Socks5 => "SOCKS5",
            ProxyType::Http => "HTTP",
        }
    }

    pub fn proxychains_name(&self) -> &'static str {
        match self {
            ProxyType::Socks5 => "socks5",
            ProxyType::Http => "http",
        }
    }
}

/// Chain strategy for proxychains
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChainStrategy {
    #[default]
    StrictChain,
    DynamicChain,
    RandomChain,
}

impl ChainStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChainStrategy::StrictChain => "strict_chain",
            ChainStrategy::DynamicChain => "dynamic_chain",
            ChainStrategy::RandomChain => "random_chain",
        }
    }
}

/// A single proxy hop in the chain
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProxyHop {
    pub index: u8,
    pub proxy_type: ProxyType,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub label: Option<String>,
}

impl ProxyHop {
    pub fn new(index: u8, proxy_type: ProxyType, host: String, port: u16) -> Self {
        Self {
            index,
            proxy_type,
            host,
            port,
            username: None,
            password: None,
            label: None,
        }
    }

    pub fn with_auth(mut self, username: String, password: String) -> Self {
        self.username = Some(username);
        self.password = Some(password);
        self
    }

    pub fn with_label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }

    /// Validate the proxy hop
    pub fn validate(&self) -> Result<(), String> {
        if self.host.is_empty() {
            return Err("Host cannot be empty".to_string());
        }
        if self.port == 0 {
            return Err("Port must be greater than 0".to_string());
        }
        Ok(())
    }
}

/// WireGuard configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WireGuardConfig {
    pub config_path: String,
    pub interface_name: String,
    pub route_all_traffic: bool,
}

/// OpenVPN configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenVpnConfig {
    pub config_path: String,
    pub auth_file: Option<String>,
    pub route_all_traffic: bool,
}

/// Complete proxy configuration for a role
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProxyConfig {
    pub role: String,
    pub gateway_mode: GatewayMode,
    pub chain_strategy: ChainStrategy,
    pub hops: Vec<ProxyHop>,
    pub wireguard: Option<WireGuardConfig>,
    pub openvpn: Option<OpenVpnConfig>,
}

impl ProxyConfig {
    pub fn new(role: String, gateway_mode: GatewayMode) -> Self {
        Self {
            role,
            gateway_mode,
            chain_strategy: ChainStrategy::StrictChain,
            hops: Vec::new(),
            wireguard: None,
            openvpn: None,
        }
    }

    pub fn add_hop(&mut self, hop: ProxyHop) {
        self.hops.push(hop);
    }

    /// Validate the proxy configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.role.is_empty() {
            return Err("Role name cannot be empty".to_string());
        }

        match self.gateway_mode {
            GatewayMode::ProxyChain => {
                if self.hops.is_empty() {
                    return Err("Proxy chain requires at least one hop".to_string());
                }
                if self.hops.len() > 8 {
                    return Err("Maximum 8 proxy hops allowed".to_string());
                }
                for hop in &self.hops {
                    hop.validate()?;
                }
            }
            GatewayMode::WireGuard => {
                if let Some(wg) = &self.wireguard {
                    if wg.config_path.is_empty() {
                        return Err("WireGuard config path cannot be empty".to_string());
                    }
                } else {
                    return Err("WireGuard mode requires WireGuard config".to_string());
                }
            }
            GatewayMode::OpenVpn => {
                if let Some(ovpn) = &self.openvpn {
                    if ovpn.config_path.is_empty() {
                        return Err("OpenVPN config path cannot be empty".to_string());
                    }
                } else {
                    return Err("OpenVPN mode requires OpenVPN config".to_string());
                }
            }
        }

        Ok(())
    }
}

/// Kind of VM
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VmKind {
    #[default]
    ProxyGateway,
    App,
    DisposableApp,
}

impl VmKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            VmKind::ProxyGateway => "Proxy/Gateway VM",
            VmKind::App => "App VM",
            VmKind::DisposableApp => "Disposable App VM",
        }
    }
}

/// Role kind for templates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RoleKind {
    ProxyGateway,
    App,
    DisposableApp,
    #[default]
    Generic,
}

impl RoleKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            RoleKind::ProxyGateway => "Proxy/Gateway",
            RoleKind::App => "App",
            RoleKind::DisposableApp => "Disposable App",
            RoleKind::Generic => "Generic",
        }
    }
}

/// VM state from libvirt
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VmState {
    Running,
    Paused,
    ShutOff,
    #[default]
    Unknown,
}

impl VmState {
    pub fn from_virsh_state(state: &str) -> Self {
        match state.trim().to_lowercase().as_str() {
            "running" => VmState::Running,
            "paused" => VmState::Paused,
            "shut off" | "shutoff" => VmState::ShutOff,
            _ => VmState::Unknown,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            VmState::Running => "Running",
            VmState::Paused => "Paused",
            VmState::ShutOff => "Shut Off",
            VmState::Unknown => "Unknown",
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self, VmState::Running)
    }
}

/// Information about a VM
#[derive(Debug, Clone, Default)]
pub struct VmInfo {
    pub name: String,
    pub state: VmState,
    pub kind: VmKind,
    pub role: Option<String>,
}

/// Network state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NetworkState {
    Active,
    Inactive,
    #[default]
    Unknown,
}

impl NetworkState {
    pub fn is_active(&self) -> bool {
        matches!(self, NetworkState::Active)
    }
}

/// Information about a libvirt network
#[derive(Debug, Clone, Default)]
pub struct NetworkInfo {
    pub name: String,
    pub state: NetworkState,
    pub autostart: bool,
}

/// Validates a role name according to allowed patterns
pub fn validate_role_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Role name cannot be empty".to_string());
    }

    let re = regex::Regex::new(r"^[a-z0-9_-]+$").unwrap();
    if !re.is_match(name) {
        return Err(
            "Role name must contain only lowercase letters, numbers, underscores, and hyphens"
                .to_string(),
        );
    }

    if name.len() > 32 {
        return Err("Role name must be 32 characters or less".to_string());
    }

    Ok(())
}

/// Normalize a role name to lowercase, no spaces
pub fn normalize_role_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}
