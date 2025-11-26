//! Proxy VM Wizard Core Library
//!
//! This crate provides the core functionality for managing proxy/gateway VMs,
//! app VMs, and disposable VMs using libvirt/QEMU.

pub mod model;
pub mod config;
pub mod libvirt;
pub mod proxy_config;
pub mod vpn_config;
pub mod error;

pub use error::{Error, Result};
pub use model::*;
pub use config::{GlobalConfig, RoleMeta, Template, TemplateRegistry};
pub use libvirt::LibvirtAdapter;
pub use proxy_config::ProxyConfigBuilder;
pub use vpn_config::{WireGuardParsedConfig, OpenVpnParsedConfig, list_wireguard_configs, list_openvpn_configs};

