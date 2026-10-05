use anyhow::Result;
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read configuration: {0}")]
    IO(#[from] std::io::Error),
    #[error("Failed to parse configuration: {0}")]
    Parse(toml::de::Error),
}

fn discover_configure(dir: &Path, name: &str) -> Option<PathBuf> {
    let mut cur = dir.to_owned();
    loop {
        let config = cur.join(name);
        if config.is_file() {
            break Some(config);
        }
        if let Some(p) = cur.parent() {
            cur = p.to_owned();
        } else {
            break None;
        }
    }
}

pub trait LoadConfig: DeserializeOwned {
    fn from_str(content: &str) -> Result<Self, ConfigError> {
        toml::from_str(content).map_err(ConfigError::Parse)
    }
    fn from_path(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path).map_err(ConfigError::IO)?;
        Self::from_str(&content)
    }
    fn from_dir(dir: &Path, name: &str) -> Result<Self, ConfigError> {
        let path = discover_configure(dir, name).ok_or(ConfigError::IO(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Configuration not found.",
        )))?;
        Self::from_path(&path)
    }
}

#[cfg(test)]
#[path = "tests/test_config.rs"]
mod tests;
