use serde::Deserialize;
use std::path::PathBuf;

/// 编译相关配置
#[derive(Deserialize)]
pub struct CompileConfig {
    /// 编译器
    pub compiler: PathBuf,
    /// 编译选项
    pub flag: Vec<String>,
}

/// CLI 配置
#[derive(Deserialize)]
pub struct Config {
    pub compile: CompileConfig,
}

impl Default for CompileConfig {
    fn default() -> Self {
        CompileConfig {
            compiler: PathBuf::from("g++"),
            flag: vec!["-fdiagnostics-color=always".to_owned()],
        }
    }
}
impl Default for Config {
    fn default() -> Self {
        Config {
            compile: CompileConfig::default(),
        }
    }
}
