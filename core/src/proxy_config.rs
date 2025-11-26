//! Proxy configuration file and apply-proxy.sh script generation

use crate::{Result, ProxyConfig, GatewayMode};
use std::path::Path;
use std::fs;

/// Builder for generating proxy.conf and apply-proxy.sh files
#[derive(Debug)]
pub struct ProxyConfigBuilder;

impl ProxyConfigBuilder {
    /// Generate the proxy.conf file content
    pub fn generate_proxy_conf(config: &ProxyConfig) -> String {
        let mut lines = Vec::new();

        lines.push(format!("# Proxy config for role: {}", config.role));
        lines.push(format!("GATEWAY_MODE={}", config.gateway_mode.as_str()));
        lines.push(format!("CHAIN_STRATEGY={}", config.chain_strategy.as_str()));
        lines.push(format!("PROXY_COUNT={}", config.hops.len()));
        lines.push(String::new());

        // Proxy chain hops
        if config.gateway_mode == GatewayMode::ProxyChain && !config.hops.is_empty() {
            lines.push("# Proxy chain configuration".to_string());
            for hop in &config.hops {
                let idx = hop.index;
                lines.push(format!("PROXY_{}_TYPE={}", idx, hop.proxy_type.as_str()));
                lines.push(format!("PROXY_{}_HOST={}", idx, hop.host));
                lines.push(format!("PROXY_{}_PORT={}", idx, hop.port));
                lines.push(format!(
                    "PROXY_{}_USER={}",
                    idx,
                    hop.username.as_deref().unwrap_or("")
                ));
                lines.push(format!(
                    "PROXY_{}_PASS={}",
                    idx,
                    hop.password.as_deref().unwrap_or("")
                ));
                lines.push(format!(
                    "PROXY_{}_LABEL={}",
                    idx,
                    hop.label.as_deref().unwrap_or("")
                ));
            }

            // Backwards compatibility: first proxy fields
            if let Some(first) = config.hops.first() {
                lines.push(String::new());
                lines.push("# First proxy (for compatibility)".to_string());
                lines.push(format!("ACTIVE_PROTOCOL={}", first.proxy_type.as_str()));
                match first.proxy_type {
                    crate::ProxyType::Socks5 => {
                        lines.push(format!("SOCKS5_HOST={}", first.host));
                        lines.push(format!("SOCKS5_PORT={}", first.port));
                        lines.push(format!(
                            "SOCKS5_USER={}",
                            first.username.as_deref().unwrap_or("")
                        ));
                        lines.push(format!(
                            "SOCKS5_PASS={}",
                            first.password.as_deref().unwrap_or("")
                        ));
                        lines.push("HTTP_HOST=".to_string());
                        lines.push("HTTP_PORT=".to_string());
                        lines.push("HTTP_USER=".to_string());
                        lines.push("HTTP_PASS=".to_string());
                    }
                    crate::ProxyType::Http => {
                        lines.push("SOCKS5_HOST=".to_string());
                        lines.push("SOCKS5_PORT=".to_string());
                        lines.push("SOCKS5_USER=".to_string());
                        lines.push("SOCKS5_PASS=".to_string());
                        lines.push(format!("HTTP_HOST={}", first.host));
                        lines.push(format!("HTTP_PORT={}", first.port));
                        lines.push(format!(
                            "HTTP_USER={}",
                            first.username.as_deref().unwrap_or("")
                        ));
                        lines.push(format!(
                            "HTTP_PASS={}",
                            first.password.as_deref().unwrap_or("")
                        ));
                    }
                }
            }
        } else {
            // Empty compatibility fields
            lines.push("# First proxy (for compatibility)".to_string());
            lines.push("ACTIVE_PROTOCOL=".to_string());
            lines.push("SOCKS5_HOST=".to_string());
            lines.push("SOCKS5_PORT=".to_string());
            lines.push("SOCKS5_USER=".to_string());
            lines.push("SOCKS5_PASS=".to_string());
            lines.push("HTTP_HOST=".to_string());
            lines.push("HTTP_PORT=".to_string());
            lines.push("HTTP_USER=".to_string());
            lines.push("HTTP_PASS=".to_string());
        }

        lines.push(String::new());
        lines.push("# VPN / other modes".to_string());

        // WireGuard config
        if let Some(wg) = &config.wireguard {
            lines.push(format!("WG_CONFIG_PATH={}", wg.config_path));
            lines.push(format!("WG_INTERFACE_NAME={}", wg.interface_name));
            lines.push(format!("WG_ROUTE_ALL_TRAFFIC={}", wg.route_all_traffic));
        } else {
            lines.push("WG_CONFIG_PATH=".to_string());
            lines.push("WG_INTERFACE_NAME=".to_string());
            lines.push("WG_ROUTE_ALL_TRAFFIC=".to_string());
        }

        // OpenVPN config
        if let Some(ovpn) = &config.openvpn {
            lines.push(format!("OPENVPN_CONFIG_PATH={}", ovpn.config_path));
            lines.push(format!(
                "OPENVPN_AUTH_FILE={}",
                ovpn.auth_file.as_deref().unwrap_or("")
            ));
            lines.push(format!("OPENVPN_ROUTE_ALL_TRAFFIC={}", ovpn.route_all_traffic));
        } else {
            lines.push("OPENVPN_CONFIG_PATH=".to_string());
            lines.push("OPENVPN_AUTH_FILE=".to_string());
            lines.push("OPENVPN_ROUTE_ALL_TRAFFIC=".to_string());
        }

        lines.join("\n")
    }

    /// Generate the apply-proxy.sh script content
    pub fn generate_apply_proxy_script(role: &str) -> String {
        format!(
            r#"#!/usr/bin/env bash
set -euo pipefail

ROLE="{role}"
CONF="/proxy/proxy.conf"
OUT="/etc/proxychains.conf"

log() {{ echo "[apply-proxy][${{ROLE}}] $*"; }}

if [[ ! -f "$CONF" ]]; then
  log "Config file $CONF not found – nothing to do."
  exit 0
fi

# shellcheck disable=SC1090
. "$CONF" || {{
  log "Failed to source config from $CONF."
  exit 1
}}

MODE="${{GATEWAY_MODE:-}}"
if [[ "$MODE" = "PROXY_CHAIN" ]]; then
  COUNT="${{PROXY_COUNT:-0}}"
  if ! [[ "$COUNT" =~ ^[0-9]+$ ]] || [[ "$COUNT" -lt 1 ]]; then
    log "PROXY_CHAIN mode but PROXY_COUNT is invalid ('$COUNT')."
    exit 0
  fi

  STRAT="${{CHAIN_STRATEGY:-strict_chain}}"
  cat > "$OUT" <<EOC
# Auto-generated by apply-proxy.sh for role ${{ROLE}}
${{STRAT}}
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
    eval "T=\"\${{PROXY_${{i}}_TYPE:-}}\""
    eval "H=\"\${{PROXY_${{i}}_HOST:-}}\""
    eval "P=\"\${{PROXY_${{i}}_PORT:-}}\""
    eval "U=\"\${{PROXY_${{i}}_USER:-}}\""
    eval "PW=\"\${{PROXY_${{i}}_PASS:-}}\""

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
case "${{ACTIVE_PROTOCOL:-}}" in
  SOCKS5)
    if [[ -z "${{SOCKS5_HOST:-}}" || -z "${{SOCKS5_PORT:-}}" ]]; then
      log "SOCKS5 selected but SOCKS5_HOST or SOCKS5_PORT is empty."
      exit 0
    fi
    cat > "$OUT" <<EOC
# Auto-generated by apply-proxy.sh for role ${{ROLE}}
strict_chain
proxy_dns
tcp_read_time_out 15000
tcp_connect_time_out 8000

[ProxyList]
EOC
    if [[ -n "${{SOCKS5_USER:-}}" || -n "${{SOCKS5_PASS:-}}" ]]; then
      echo "socks5 ${{SOCKS5_HOST}} ${{SOCKS5_PORT}} ${{SOCKS5_USER:-}} ${{SOCKS5_PASS:-}}" >> "$OUT"
    else
      echo "socks5 ${{SOCKS5_HOST}} ${{SOCKS5_PORT}}" >> "$OUT"
    fi
    log "proxychains.conf updated for single SOCKS5."
    ;;
  HTTP)
    if [[ -z "${{HTTP_HOST:-}}" || -z "${{HTTP_PORT:-}}" ]]; then
      log "HTTP selected but HTTP_HOST or HTTP_PORT is empty."
      exit 0
    fi
    cat > "$OUT" <<EOC
# Auto-generated by apply-proxy.sh for role ${{ROLE}}
strict_chain
proxy_dns
tcp_read_time_out 15000
tcp_connect_time_out 8000

[ProxyList]
EOC
    if [[ -n "${{HTTP_USER:-}}" || -n "${{HTTP_PASS:-}}" ]]; then
      echo "http ${{HTTP_HOST}} ${{HTTP_PORT}} ${{HTTP_USER:-}} ${{HTTP_PASS:-}}" >> "$OUT"
    else
      echo "http ${{HTTP_HOST}} ${{HTTP_PORT}}" >> "$OUT"
    fi
    log "proxychains.conf updated for single HTTP."
    ;;
  *)
    log "GATEWAY_MODE='${{MODE}}' and ACTIVE_PROTOCOL='${{ACTIVE_PROTOCOL:-}}' – nothing to do in apply-proxy.sh yet."
    ;;
esac

exit 0
"#,
            role = role
        )
    }

    /// Write proxy.conf to a role directory
    pub fn write_proxy_conf(config: &ProxyConfig, role_dir: &Path) -> Result<()> {
        let content = Self::generate_proxy_conf(config);
        let path = role_dir.join("proxy.conf");
        fs::create_dir_all(role_dir)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// Write apply-proxy.sh to a role directory
    pub fn write_apply_proxy_script(role: &str, role_dir: &Path) -> Result<()> {
        let content = Self::generate_apply_proxy_script(role);
        let path = role_dir.join("apply-proxy.sh");
        fs::create_dir_all(role_dir)?;
        fs::write(&path, &content)?;

        // Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&path, perms)?;
        }

        Ok(())
    }

    /// Write both proxy.conf and apply-proxy.sh
    pub fn write_config_files(config: &ProxyConfig, role_dir: &Path) -> Result<()> {
        Self::write_proxy_conf(config, role_dir)?;
        Self::write_apply_proxy_script(&config.role, role_dir)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProxyType, ProxyHop, WireGuardConfig, OpenVpnConfig};
    use tempfile::tempdir;

    #[test]
    fn test_generate_proxy_conf_simple_chain() {
        let mut config = ProxyConfig::new("work".to_string(), GatewayMode::ProxyChain);
        config.add_hop(ProxyHop {
            index: 1,
            proxy_type: ProxyType::Socks5,
            host: "proxy1.example.com".to_string(),
            port: 1080,
            username: None,
            password: None,
            label: Some("Primary".to_string()),
        });

        let content = ProxyConfigBuilder::generate_proxy_conf(&config);
        assert!(content.contains("GATEWAY_MODE=PROXY_CHAIN"));
        assert!(content.contains("PROXY_COUNT=1"));
        assert!(content.contains("PROXY_1_TYPE=SOCKS5"));
        assert!(content.contains("PROXY_1_HOST=proxy1.example.com"));
        assert!(content.contains("PROXY_1_PORT=1080"));
        assert!(content.contains("SOCKS5_HOST=proxy1.example.com"));
    }

    #[test]
    fn test_generate_proxy_conf_multiple_hops() {
        let mut config = ProxyConfig::new("bank".to_string(), GatewayMode::ProxyChain);
        config.add_hop(ProxyHop {
            index: 1,
            proxy_type: ProxyType::Socks5,
            host: "proxy1.example.com".to_string(),
            port: 1080,
            username: Some("user1".to_string()),
            password: Some("pass1".to_string()),
            label: None,
        });
        config.add_hop(ProxyHop {
            index: 2,
            proxy_type: ProxyType::Http,
            host: "proxy2.example.com".to_string(),
            port: 8080,
            username: None,
            password: None,
            label: None,
        });

        let content = ProxyConfigBuilder::generate_proxy_conf(&config);
        assert!(content.contains("PROXY_COUNT=2"));
        assert!(content.contains("PROXY_1_TYPE=SOCKS5"));
        assert!(content.contains("PROXY_1_USER=user1"));
        assert!(content.contains("PROXY_2_TYPE=HTTP"));
        assert!(content.contains("PROXY_2_HOST=proxy2.example.com"));
    }

    #[test]
    fn test_generate_proxy_conf_wireguard() {
        let mut config = ProxyConfig::new("vpn".to_string(), GatewayMode::WireGuard);
        config.wireguard = Some(WireGuardConfig {
            config_path: "/proxy/wg_vpn.conf".to_string(),
            interface_name: "wg0".to_string(),
            route_all_traffic: true,
        });

        let content = ProxyConfigBuilder::generate_proxy_conf(&config);
        assert!(content.contains("GATEWAY_MODE=WIREGUARD"));
        assert!(content.contains("WG_CONFIG_PATH=/proxy/wg_vpn.conf"));
        assert!(content.contains("WG_INTERFACE_NAME=wg0"));
        assert!(content.contains("WG_ROUTE_ALL_TRAFFIC=true"));
    }

    #[test]
    fn test_generate_proxy_conf_openvpn() {
        let mut config = ProxyConfig::new("ovpn".to_string(), GatewayMode::OpenVpn);
        config.openvpn = Some(OpenVpnConfig {
            config_path: "/proxy/client.ovpn".to_string(),
            auth_file: Some("/proxy/auth.txt".to_string()),
            route_all_traffic: false,
        });

        let content = ProxyConfigBuilder::generate_proxy_conf(&config);
        assert!(content.contains("GATEWAY_MODE=OPENVPN"));
        assert!(content.contains("OPENVPN_CONFIG_PATH=/proxy/client.ovpn"));
        assert!(content.contains("OPENVPN_AUTH_FILE=/proxy/auth.txt"));
    }

    #[test]
    fn test_generate_apply_proxy_script() {
        let script = ProxyConfigBuilder::generate_apply_proxy_script("work");
        assert!(script.contains("ROLE=\"work\""));
        assert!(script.contains("CONF=\"/proxy/proxy.conf\""));
        assert!(script.contains("PROXY_CHAIN"));
        assert!(script.contains("proxychains.conf"));
    }

    #[test]
    fn test_write_config_files() {
        let dir = tempdir().unwrap();
        let role_dir = dir.path().join("work");

        let mut config = ProxyConfig::new("work".to_string(), GatewayMode::ProxyChain);
        config.add_hop(ProxyHop {
            index: 1,
            proxy_type: ProxyType::Socks5,
            host: "localhost".to_string(),
            port: 1080,
            username: None,
            password: None,
            label: None,
        });

        ProxyConfigBuilder::write_config_files(&config, &role_dir).unwrap();

        assert!(role_dir.join("proxy.conf").exists());
        assert!(role_dir.join("apply-proxy.sh").exists());

        let conf_content = fs::read_to_string(role_dir.join("proxy.conf")).unwrap();
        assert!(conf_content.contains("GATEWAY_MODE=PROXY_CHAIN"));

        let script_content = fs::read_to_string(role_dir.join("apply-proxy.sh")).unwrap();
        assert!(script_content.contains("ROLE=\"work\""));
    }
}

