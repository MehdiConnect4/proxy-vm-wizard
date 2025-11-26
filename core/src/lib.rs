//! Proxy VM Wizard Core Library
//!
//! This crate provides the core functionality for managing proxy/gateway VMs,
//! app VMs, and disposable VMs using libvirt/QEMU.

pub mod auth;
pub mod config;
pub mod error;
pub mod libvirt;
pub mod model;
pub mod proxy_config;
pub mod vpn_config;

pub use auth::{AuthState, EncryptionManager};
pub use config::{GlobalConfig, RoleMeta, Template, TemplateRegistry};
pub use error::{Error, Result};
pub use libvirt::LibvirtAdapter;
pub use model::*;
pub use proxy_config::ProxyConfigBuilder;
pub use vpn_config::{
    list_openvpn_configs, list_wireguard_configs, OpenVpnParsedConfig, WireGuardParsedConfig,
};
