//! VPN configuration file parsing for WireGuard and OpenVPN

use std::fs;
use std::path::Path;

/// Parsed information from a WireGuard config
#[derive(Debug, Clone, Default)]
pub struct WireGuardParsedConfig {
    pub interface_address: Option<String>,
    pub interface_dns: Option<String>,
    pub peers: Vec<WireGuardPeer>,
}

#[derive(Debug, Clone, Default)]
pub struct WireGuardPeer {
    pub endpoint: Option<String>,
    pub allowed_ips: Option<String>,
    pub name: Option<String>, // Extracted from comments or endpoint
}

impl WireGuardParsedConfig {
    /// Parse a WireGuard config file
    pub fn parse_file(path: &Path) -> Option<Self> {
        let content = fs::read_to_string(path).ok()?;
        Self::parse(&content)
    }

    /// Parse WireGuard config content
    pub fn parse(content: &str) -> Option<Self> {
        let mut config = WireGuardParsedConfig::default();
        let mut current_peer: Option<WireGuardPeer> = None;
        let mut last_comment = String::new();

        for line in content.lines() {
            let line = line.trim();

            // Track comments as potential peer names
            if line.starts_with('#') {
                last_comment = line.trim_start_matches('#').trim().to_string();
                continue;
            }

            if line.is_empty() {
                continue;
            }

            if line.to_lowercase() == "[interface]" {
                // Save any pending peer
                if let Some(peer) = current_peer.take() {
                    config.peers.push(peer);
                }
                continue;
            }

            if line.to_lowercase() == "[peer]" {
                // Save any pending peer
                if let Some(peer) = current_peer.take() {
                    config.peers.push(peer);
                }
                // Start new peer
                let mut peer = WireGuardPeer::default();
                if !last_comment.is_empty() {
                    peer.name = Some(last_comment.clone());
                }
                current_peer = Some(peer);
                last_comment.clear();
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_lowercase();
                let value = value.trim().to_string();

                match key.as_str() {
                    "address" => config.interface_address = Some(value),
                    "dns" => config.interface_dns = Some(value),
                    "endpoint" => {
                        if let Some(ref mut peer) = current_peer {
                            peer.endpoint = Some(value.clone());
                            // Extract server name from endpoint if no name set
                            if peer.name.is_none() {
                                if let Some(host) = value.split(':').next() {
                                    peer.name = Some(host.to_string());
                                }
                            }
                        }
                    }
                    "allowedips" => {
                        if let Some(ref mut peer) = current_peer {
                            peer.allowed_ips = Some(value);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Save last peer
        if let Some(peer) = current_peer {
            config.peers.push(peer);
        }

        Some(config)
    }

    /// Get a display name for this config
    pub fn display_name(&self) -> String {
        if let Some(peer) = self.peers.first() {
            if let Some(ref name) = peer.name {
                return name.clone();
            }
            if let Some(ref endpoint) = peer.endpoint {
                return endpoint.clone();
            }
        }
        "WireGuard Config".to_string()
    }
}

/// Parsed information from an OpenVPN config
#[derive(Debug, Clone, Default)]
pub struct OpenVpnParsedConfig {
    pub remotes: Vec<OpenVpnRemote>,
    pub protocol: Option<String>,
    pub dev_type: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct OpenVpnRemote {
    pub host: String,
    pub port: Option<u16>,
    pub protocol: Option<String>,
}

impl OpenVpnParsedConfig {
    /// Parse an OpenVPN config file
    pub fn parse_file(path: &Path) -> Option<Self> {
        let content = fs::read_to_string(path).ok()?;
        Self::parse(&content)
    }

    /// Parse OpenVPN config content
    pub fn parse(content: &str) -> Option<Self> {
        let mut config = OpenVpnParsedConfig::default();

        for line in content.lines() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0].to_lowercase().as_str() {
                "remote" => {
                    if parts.len() >= 2 {
                        let mut remote = OpenVpnRemote {
                            host: parts[1].to_string(),
                            port: None,
                            protocol: None,
                        };
                        if parts.len() >= 3 {
                            remote.port = parts[2].parse().ok();
                        }
                        if parts.len() >= 4 {
                            remote.protocol = Some(parts[3].to_string());
                        }
                        config.remotes.push(remote);
                    }
                }
                "proto" => {
                    if parts.len() >= 2 {
                        config.protocol = Some(parts[1].to_string());
                    }
                }
                "dev" => {
                    if parts.len() >= 2 {
                        config.dev_type = Some(parts[1].to_string());
                    }
                }
                _ => {}
            }
        }

        Some(config)
    }

    /// Get a display name for this config
    pub fn display_name(&self) -> String {
        if let Some(remote) = self.remotes.first() {
            let mut name = remote.host.clone();
            if let Some(port) = remote.port {
                name.push_str(&format!(":{}", port));
            }
            return name;
        }
        "OpenVPN Config".to_string()
    }
}

/// List all WireGuard config files in a directory
pub fn list_wireguard_configs(dir: &Path) -> Vec<(String, WireGuardParsedConfig)> {
    let mut configs = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "conf").unwrap_or(false) {
                if let Some(config) = WireGuardParsedConfig::parse_file(&path) {
                    if !config.peers.is_empty() {
                        let filename = path
                            .file_name()
                            .map(|f| f.to_string_lossy().to_string())
                            .unwrap_or_default();
                        configs.push((filename, config));
                    }
                }
            }
        }
    }

    configs
}

/// List all OpenVPN config files in a directory
pub fn list_openvpn_configs(dir: &Path) -> Vec<(String, OpenVpnParsedConfig)> {
    let mut configs = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase());
            if ext == Some("ovpn".to_string()) || ext == Some("conf".to_string()) {
                if let Some(config) = OpenVpnParsedConfig::parse_file(&path) {
                    if !config.remotes.is_empty() {
                        let filename = path
                            .file_name()
                            .map(|f| f.to_string_lossy().to_string())
                            .unwrap_or_default();
                        configs.push((filename, config));
                    }
                }
            }
        }
    }

    configs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wireguard_parse() {
        let content = r#"
[Interface]
PrivateKey = abc123
Address = 10.0.0.2/24
DNS = 1.1.1.1

# US Server
[Peer]
PublicKey = xyz789
Endpoint = us.example.com:51820
AllowedIPs = 0.0.0.0/0
"#;
        let config = WireGuardParsedConfig::parse(content).unwrap();
        assert_eq!(config.interface_address, Some("10.0.0.2/24".to_string()));
        assert_eq!(config.peers.len(), 1);
        assert_eq!(config.peers[0].name, Some("US Server".to_string()));
        assert_eq!(
            config.peers[0].endpoint,
            Some("us.example.com:51820".to_string())
        );
    }

    #[test]
    fn test_openvpn_parse() {
        let content = r#"
client
dev tun
proto udp
remote us.example.com 1194
remote eu.example.com 1194 tcp
"#;
        let config = OpenVpnParsedConfig::parse(content).unwrap();
        assert_eq!(config.remotes.len(), 2);
        assert_eq!(config.remotes[0].host, "us.example.com");
        assert_eq!(config.remotes[0].port, Some(1194));
        assert_eq!(config.remotes[1].protocol, Some("tcp".to_string()));
    }
}
