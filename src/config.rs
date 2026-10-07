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

/// 给定当前目录和配置名称寻找项目目录
///
/// # 返回值
/// 当当前目录以及当前目录的父目录下都没有找到配置的时候返回 `None`，
/// 否则返回离当前目录最近的项目目录。
///
/// # 参数
/// - `dir`: 当前目录。
/// - `name`: 配置名称
pub fn discover_workspace(dir: &Path, name: &str) -> Option<PathBuf> {
    let mut cur = dir.to_owned();
    loop {
        if cur.join(name).is_file() {
            return Some(cur);
        }
        if !cur.pop() {
            return None;
        }
    }
}

pub async fn load_config<T: DeserializeOwned>(path: &Path) -> Result<T, ConfigError> {
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(ConfigError::IO)?;
    toml::from_str(&content).map_err(ConfigError::Parse)
}

#[cfg(test)]
#[path = "tests/test_config.rs"]
mod tests;
