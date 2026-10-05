use anyhow::Result;
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// 加载配置缩导致的错误
#[derive(Debug, Error)]
pub enum ConfigError {
    /// 读取配置所导致的 IO 错误
    #[error("Failed to read configuration: {0}")]
    IO(#[from] std::io::Error),
    /// 配置解析错误
    #[error("Failed to parse configuration: {0}")]
    Parse(toml::de::Error),
}

/// 给定当前目录和配置名称寻找配置
///
/// # 返回值
/// 当当前目录以及当前目录的父目录下都没有找到配置的时候返回 `None`，
/// 否则返回离当前目录最近的配置。
///
/// # 参数
/// - `dir`: 当前目录。
/// - `name`: 配置名称
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

/// 加载配置 Trait。
/// 用于给配置结构体实现解析方法。
/// 要求配置结构体实现 `DeserializeOwned` trait。
pub trait LoadConfig: DeserializeOwned {
    /// 从字符串加载配置
    ///
    /// # 参数
    /// - `content`: 配置内容
    fn from_str(content: &str) -> Result<Self, ConfigError> {
        toml::from_str(content).map_err(ConfigError::Parse)
    }
    /// 从文件路径加载配置。
    ///
    /// # 参数
    /// - `path`: 配置文件路径
    fn from_path(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path).map_err(ConfigError::IO)?;
        Self::from_str(&content)
    }
    /// 从当前目录向上寻找配置并且加载配置
    ///
    /// # 参数
    /// - `dir`: 当前目录
    /// - `name`: 配置名称
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
