use std::{io, path::PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum VerbaError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("invalid provider base URL: {0}")]
    InvalidBaseUrl(String),
    #[error("config path has no parent directory: {0}")]
    MissingConfigParent(PathBuf),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("TOML parse error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),
    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("Secret Service error: {0}")]
    Secret(String),
    #[error("D-Bus error: {0}")]
    Dbus(String),
    #[error("systemd user service start failed: {0}")]
    SystemdStart(String),
}

pub type Result<T> = std::result::Result<T, VerbaError>;
