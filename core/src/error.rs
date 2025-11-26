//! Error types for the proxy-vm-core crate

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Command execution failed: {cmd} - {message}")]
    Command { cmd: String, message: String },

    #[error("Command not found: {0}")]
    CommandNotFound(String),

    #[error("Libvirt error: {0}")]
    Libvirt(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Template error: {0}")]
    Template(String),

    #[error("Role error: {0}")]
    Role(String),

    #[error("VM error: {0}")]
    Vm(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] toml::ser::Error),

    #[error("Deserialization error: {0}")]
    Deserialization(#[from] toml::de::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Resource already exists: {0}")]
    AlreadyExists(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Connection test failed: {host}:{port} - {reason}")]
    ConnectionTest {
        host: String,
        port: u16,
        reason: String,
    },

    #[error("Authentication error: {0}")]
    Auth(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn auth(msg: impl Into<String>) -> Self {
        Error::Auth(msg.into())
    }

    pub fn config(msg: impl Into<String>) -> Self {
        Error::Config(msg.into())
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        Error::Validation(msg.into())
    }

    pub fn libvirt(msg: impl Into<String>) -> Self {
        Error::Libvirt(msg.into())
    }

    pub fn role(msg: impl Into<String>) -> Self {
        Error::Role(msg.into())
    }

    pub fn vm(msg: impl Into<String>) -> Self {
        Error::Vm(msg.into())
    }

    pub fn template(msg: impl Into<String>) -> Self {
        Error::Template(msg.into())
    }
}
